use quick_xml::events::Event;
use quick_xml::Reader;
use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const CACHE_TTL_SECS: u64 = 3_600; // 1 hour

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Paper
{
    pub title: String,
    pub link: String,
    pub pdf_link: String,
    pub authors: Vec<String>,
    pub abstract_text: String,
    pub categories: Vec<String>,
    pub pub_date: String,
    /// Relevance score assigned by a topic filter (0 when no filter is active).
    #[serde(default)]
    pub score: i32,
}

#[derive(Serialize, Deserialize)]
struct CachedFeed
{
    fetched_at: u64,
    papers: Vec<Paper>,
}

fn cache_path(cats: &[String]) -> Option<std::path::PathBuf>
{
    let mut sorted = cats.to_vec();
    sorted.sort();
    let key = sorted.join("_");
    dirs::cache_dir().map(|d| d.join("rxy").join(format!("{}.json", key)))
}

fn load_cache(cats: &[String]) -> Option<Vec<Paper>>
{
    let path = cache_path(cats)?;
    let data = std::fs::read(&path).ok()?;
    let cached: CachedFeed = serde_json::from_slice(&data).ok()?;
    let now = SystemTime::now().duration_since(UNIX_EPOCH).ok()?.as_secs();
    if now.saturating_sub(cached.fetched_at) < CACHE_TTL_SECS
    {
        Some(cached.papers)
    }
    else
    {
        None
    }
}

fn save_cache(cats: &[String], papers: &[Paper])
{
    let Some(path) = cache_path(cats) else { return };
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let cached = CachedFeed { fetched_at: now, papers: papers.to_vec() };
    if let Some(parent) = path.parent()
    {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(data) = serde_json::to_vec(&cached)
    {
        let _ = std::fs::write(&path, data);
    }
}

/// Fetch recent papers for one or more categories via the arXiv export API.
/// Uses a single OR-combined request so we don't hammer the endpoint.
/// When `force` is false the result is served from a 1-hour disk cache when available.
pub fn fetch_multiple(cats: &[String], force: bool) -> (Vec<Paper>, Vec<String>)
{
    if cats.is_empty()
    {
        return (Vec::new(), Vec::new());
    }

    if !force
    {
        if let Some(papers) = load_cache(cats)
        {
            return (papers, Vec::new());
        }
    }

    let search_query = cats
        .iter()
        .map(|c| format!("cat:{}", c))
        .collect::<Vec<_>>()
        .join("+OR+");

    let url = format!(
        "https://export.arxiv.org/api/query\
         ?search_query={}&sortBy=submittedDate&sortOrder=descending&max_results=100",
        search_query
    );

    match fetch_url(&url)
    {
        Ok(papers) =>
        {
            save_cache(cats, &papers);
            (papers, Vec::new())
        }
        Err(e) => (Vec::new(), vec![e]),
    }
}

fn fetch_url(url: &str) -> Result<Vec<Paper>, String>
{
    let client = reqwest::blocking::Client::builder()
        .user_agent("rxy/0.1")
        .build()
        .map_err(|e| format!("Client build error: {}", e))?;

    let mut backoff = Duration::from_secs(3);
    for attempt in 0..3_u32
    {
        let resp = client
            .get(url)
            .send()
            .map_err(|e| format!("HTTP error: {}", e))?;

        if resp.status().as_u16() == 429
        {
            if attempt < 2
            {
                let wait = resp
                    .headers()
                    .get("retry-after")
                    .and_then(|v| v.to_str().ok())
                    .and_then(|s| s.parse::<u64>().ok())
                    .map(Duration::from_secs)
                    .unwrap_or(backoff);
                std::thread::sleep(wait);
                backoff *= 2;
                continue;
            }
            return Err(
                "Rate limited (429). Wait a moment and press 'r' to retry.".to_string(),
            );
        }

        if !resp.status().is_success()
        {
            return Err(format!("HTTP {}", resp.status()));
        }

        let body = resp.text().map_err(|e| format!("Read error: {}", e))?;
        return parse_atom(&body);
    }

    Err("Rate limited after retries. Wait a moment and press 'r' to retry.".to_string())
}

// ── Atom parser ──────────────────────────────────────────────────────────────

fn local_name(qname: quick_xml::name::QName<'_>) -> String
{
    String::from_utf8_lossy(qname.local_name().as_ref())
        .to_lowercase()
        .to_string()
}

/// Strip trailing version suffix: `.../abs/2502.12345v3` → `.../abs/2502.12345`
fn strip_version(url: &str) -> String
{
    if let Some(v_pos) = url.rfind('v')
    {
        let after = &url[v_pos + 1..];
        if !after.is_empty() && after.chars().all(|c| c.is_ascii_digit())
        {
            return url[..v_pos].to_string();
        }
    }
    url.to_string()
}

fn parse_atom(xml: &str) -> Result<Vec<Paper>, String>
{
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut papers: Vec<Paper> = Vec::new();
    let mut current: Option<PaperBuilder> = None;
    let mut stack: Vec<String> = Vec::new();
    let mut buf = Vec::new();

    loop
    {
        match reader.read_event_into(&mut buf)
        {
            Ok(Event::Start(ref e)) =>
            {
                let tag = local_name(e.name());
                if tag == "entry"
                {
                    current = Some(PaperBuilder::default());
                    stack.clear();
                }
                else if current.is_some()
                {
                    if tag == "link"
                    {
                        if let Some(ref mut b) = current
                        {
                            apply_link(e, b);
                        }
                    }
                    stack.push(tag);
                }
            }

            Ok(Event::Empty(ref e)) =>
            {
                let tag = local_name(e.name());
                if let Some(ref mut b) = current
                {
                    match tag.as_str()
                    {
                        "link" => apply_link(e, b),
                        "primary_category" | "category" =>
                        {
                            for attr in e.attributes().flatten()
                            {
                                if String::from_utf8_lossy(attr.key.as_ref()).to_lowercase()
                                    == "term"
                                {
                                    let v = String::from_utf8_lossy(&attr.value).to_string();
                                    if !b.categories.contains(&v)
                                    {
                                        b.categories.push(v);
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }

            Ok(Event::Text(ref e)) =>
            {
                if let Some(ref mut b) = current
                {
                    let text = e.unescape().unwrap_or_default().trim().to_string();
                    if text.is_empty()
                    {
                        buf.clear();
                        continue;
                    }
                    let cur = stack.last().map(String::as_str).unwrap_or("");
                    let par = stack
                        .len()
                        .checked_sub(2)
                        .and_then(|i| stack.get(i))
                        .map(String::as_str)
                        .unwrap_or("");

                    match cur
                    {
                        "title" =>
                        {
                            if b.title.is_empty()
                            {
                                b.title = text;
                            }
                        }
                        "summary" =>
                        {
                            if b.abstract_text.is_empty()
                            {
                                b.abstract_text =
                                    text.split_whitespace().collect::<Vec<_>>().join(" ");
                            }
                        }
                        "id" =>
                        {
                            if b.link.is_empty() && text.contains("arxiv.org/abs/")
                            {
                                b.link = strip_version(&text);
                            }
                        }
                        "name" if par == "author" =>
                        {
                            b.authors.push(text);
                        }
                        "published" =>
                        {
                            if b.pub_date.is_empty()
                            {
                                b.pub_date = text.chars().take(10).collect();
                            }
                        }
                        _ => {}
                    }
                }
            }

            Ok(Event::End(ref e)) =>
            {
                let tag = local_name(e.name());
                if tag == "entry"
                {
                    if let Some(b) = current.take()
                    {
                        if let Some(p) = b.build()
                        {
                            papers.push(p);
                        }
                    }
                    stack.clear();
                }
                else if current.is_some()
                {
                    stack.pop();
                }
            }

            Ok(Event::Eof) => break,
            Err(e) => return Err(format!("XML parse error: {}", e)),
            _ => {}
        }
        buf.clear();
    }

    Ok(papers)
}

/// Extract the HTML abstract-page link from a `<link>` element's attributes.
fn apply_link(e: &quick_xml::events::BytesStart<'_>, b: &mut PaperBuilder)
{
    let mut href = String::new();
    let mut rel = String::new();
    let mut title = String::new();

    for attr in e.attributes().flatten()
    {
        let key = String::from_utf8_lossy(attr.key.as_ref()).to_lowercase();
        let val = String::from_utf8_lossy(&attr.value).to_string();
        match key.as_str()
        {
            "href" => href = val,
            "rel" => rel = val,
            "title" => title = val,
            _ => {}
        }
    }

    if href.contains("arxiv.org/abs/")
        && (rel == "alternate" || rel.is_empty())
        && title != "pdf"
        && b.link.is_empty()
    {
        b.link = strip_version(&href);
    }

    if title == "pdf" && !href.is_empty() && b.pdf_link.is_empty()
    {
        b.pdf_link = strip_version(&href);
    }
}

// ── Builder ───────────────────────────────────────────────────────────────────

#[derive(Default)]
struct PaperBuilder
{
    title: String,
    link: String,
    pdf_link: String,
    authors: Vec<String>,
    abstract_text: String,
    categories: Vec<String>,
    pub_date: String,
    score: i32,
}

impl PaperBuilder
{
    fn build(self) -> Option<Paper>
    {
        if self.title.is_empty() && self.link.is_empty()
        {
            return None;
        }
        Some(Paper
        {
            title: self.title,
            link: self.link,
            pdf_link: self.pdf_link,
            authors: self.authors,
            abstract_text: self.abstract_text,
            categories: self.categories,
            pub_date: self.pub_date,
            score: self.score,
        })
    }
}
