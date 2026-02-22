use crate::feed::Paper;
use std::fs;
use std::path::PathBuf;

fn path() -> PathBuf
{
    let mut p = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    p.push("rxy");
    p.push("saved.json");
    p
}

pub struct PaperFaves
{
    papers: Vec<Paper>,
}

impl PaperFaves
{
    pub fn load() -> Self
    {
        let papers = fs::read_to_string(path())
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        PaperFaves { papers }
    }

    pub fn save(&self)
    {
        let p = path();
        if let Some(parent) = p.parent()
        {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_string_pretty(&self.papers)
        {
            let _ = fs::write(p, json);
        }
    }

    pub fn toggle(&mut self, paper: &Paper)
    {
        if let Some(pos) = self.papers.iter().position(|p| p.link == paper.link)
        {
            self.papers.remove(pos);
        }
        else
        {
            self.papers.push(paper.clone());
        }
    }

    pub fn is_saved(&self, link: &str) -> bool
    {
        self.papers.iter().any(|p| p.link == link)
    }

    pub fn papers(&self) -> &[Paper]
    {
        &self.papers
    }
}
