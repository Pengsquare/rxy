use crate::categories::CATEGORIES;
use crate::config::Config;
use crate::feed::{fetch_multiple, Paper};
use crate::paper_faves::PaperFaves;
use crate::read_state::ReadState;
use crate::topic_filter::TopicFilter;
use ratatui::widgets::ListState;
use std::sync::mpsc::{self, Receiver};

#[derive(Debug, Clone, PartialEq)]
pub enum Panel
{
    Categories,
    Papers,
    Abstract,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PapersTab
{
    Feed,
    Saved,
}

pub struct App
{
    pub config: Config,
    pub read_state: ReadState,
    pub paper_faves: PaperFaves,
    pub topic_filter: Option<TopicFilter>,
    pub focus: Panel,
    pub papers_tab: PapersTab,
    pub cat_state: ListState,
    pub paper_state: ListState,
    pub saved_paper_state: ListState,
    pub papers: Vec<Paper>,
    pub hide_read: bool,
    pub loading: bool,
    pub status_msg: String,
    pub show_help: bool,
    pub show_all_categories: bool,
    pub abstract_scroll: u16,
    pub fav_picker: bool,
    pub fav_picker_state: ListState,
    feed_rx: Option<Receiver<(Vec<Paper>, Vec<String>)>>,
}

impl App
{
    pub fn new(topic_filter: Option<TopicFilter>) -> Self
    {
        let config = Config::load();
        let mut cat_state = ListState::default();
        cat_state.select(Some(0));
        // Start in "all" view when there are no favorites yet, so adding the
        // first one doesn't make the list suddenly collapse to a single entry.
        let show_all_categories = config.favorites.is_empty();

        App
        {
            config,
            read_state: ReadState::load(),
            paper_faves: PaperFaves::load(),
            topic_filter,
            focus: Panel::Categories,
            papers_tab: PapersTab::Feed,
            cat_state,
            paper_state: ListState::default(),
            saved_paper_state: ListState::default(),
            papers: Vec::new(),
            hide_read: true,
            loading: false,
            status_msg: String::from("Loading feed…"),
            show_help: false,
            show_all_categories,
            abstract_scroll: 0,
            fav_picker: false,
            fav_picker_state: ListState::default(),
            feed_rx: None,
        }
    }

    // ── Favorites picker ─────────────────────────────────────────────────────

    /// Non-favorite categories shown in the add-picker popup.
    pub fn fav_picker_items(&self) -> Vec<(&'static str, &'static str)>
    {
        CATEGORIES
            .iter()
            .filter(|(id, _)| !self.is_favorite(id))
            .copied()
            .collect()
    }

    fn open_fav_picker(&mut self)
    {
        self.fav_picker = true;
        let mut state = ListState::default();
        if !self.fav_picker_items().is_empty()
        {
            state.select(Some(0));
        }
        self.fav_picker_state = state;
    }

    fn fav_picker_confirm(&mut self)
    {
        let items = self.fav_picker_items();
        if let Some(idx) = self.fav_picker_state.selected()
        {
            if let Some(&(id, _)) = items.get(idx)
            {
                self.config.favorites.push(id.to_string());
                let _ = self.config.save();
                self.status_msg = format!("Added {} to favorites", id);
                // Keep picker open so the user can add more; items list shrinks by one.
                let new_len = self.fav_picker_items().len();
                if new_len == 0
                {
                    self.fav_picker = false;
                }
                else
                {
                    let clamped = idx.min(new_len - 1);
                    self.fav_picker_state.select(Some(clamped));
                }
            }
        }
    }

    fn remove_highlighted_favorite(&mut self)
    {
        let vis = self.visible_categories();
        if let Some((id, _)) = self.cat_state.selected().and_then(|i| vis.get(i))
        {
            let cat_id = id.to_string();
            if self.config.favorites.contains(&cat_id)
            {
                self.config.favorites.retain(|f| f != &cat_id);
                let _ = self.config.save();
                self.status_msg = format!("Removed {} from favorites", cat_id);
                // Clamp selection after item disappears (favorites-only mode shrinks)
                let new_len = self.visible_categories().len();
                match self.cat_state.selected()
                {
                    Some(sel) if sel >= new_len && new_len > 0 =>
                    {
                        self.cat_state.select(Some(new_len - 1));
                    }
                    _ if new_len == 0 =>
                    {
                        self.cat_state.select(None);
                    }
                    _ => {}
                }
            }
            else
            {
                self.status_msg = format!("{} is not a favorite", cat_id);
            }
        }
    }

    // ── Visibility helpers ────────────────────────────────────────────────────

    pub fn visible_categories(&self) -> Vec<(&'static str, &'static str)>
    {
        if self.show_all_categories || self.config.favorites.is_empty()
        {
            CATEGORIES.to_vec()
        }
        else
        {
            CATEGORIES
                .iter()
                .filter(|(id, _)| self.is_favorite(id))
                .copied()
                .collect()
        }
    }

    pub fn visible_paper_indices(&self) -> Vec<usize>
    {
        (0..self.papers.len())
            .filter(|&i| !self.hide_read || !self.read_state.is_read(&self.papers[i].link))
            .collect()
    }

    pub fn selected_paper(&self) -> Option<&Paper>
    {
        match self.papers_tab
        {
            PapersTab::Feed =>
            {
                let vis = self.visible_paper_indices();
                self.paper_state
                    .selected()
                    .and_then(|sel| vis.get(sel))
                    .and_then(|&idx| self.papers.get(idx))
            }
            PapersTab::Saved =>
            {
                self.saved_paper_state
                    .selected()
                    .and_then(|sel| self.paper_faves.papers().get(sel))
            }
        }
    }

    fn clamp_paper_selection(&mut self)
    {
        match self.papers_tab
        {
            PapersTab::Feed =>
            {
                let len = self.visible_paper_indices().len();
                match self.paper_state.selected()
                {
                    _ if len == 0 => self.paper_state.select(None),
                    Some(sel) if sel >= len => self.paper_state.select(Some(len - 1)),
                    _ => {}
                }
            }
            PapersTab::Saved =>
            {
                let len = self.paper_faves.papers().len();
                match self.saved_paper_state.selected()
                {
                    _ if len == 0 => self.saved_paper_state.select(None),
                    Some(sel) if sel >= len => self.saved_paper_state.select(Some(len - 1)),
                    _ => {}
                }
            }
        }
    }

    // ── Feed loading (background thread) ─────────────────────────────────────

    fn spawn_load(&mut self, cats: Vec<String>, force: bool)
    {
        let (tx, rx) = mpsc::channel();
        self.feed_rx = Some(rx);
        self.loading = true;
        let label = if cats.len() == 1
        {
            cats[0].clone()
        }
        else
        {
            format!("{} categories", cats.len())
        };
        self.status_msg = format!("Loading {}…", label);
        std::thread::spawn(move ||
        {
            let _ = tx.send(fetch_multiple(&cats, force));
        });
    }

    pub fn load_feed(&mut self)
    {
        let cats = if self.config.favorites.is_empty()
        {
            CATEGORIES
                .iter()
                .filter(|(id, _)| id.starts_with("cs."))
                .map(|(id, _)| id.to_string())
                .collect::<Vec<_>>()
        }
        else
        {
            self.config.favorites.clone()
        };
        self.spawn_load(cats, false);
    }

    pub fn load_feed_for_selected_category(&mut self)
    {
        let vis = self.visible_categories();
        if let Some((id, _)) = self.cat_state.selected().and_then(|i| vis.get(i))
        {
            self.spawn_load(vec![id.to_string()], false);
        }
    }

    pub fn tick(&mut self)
    {
        use std::sync::mpsc::TryRecvError;

        let outcome = if let Some(rx) = &self.feed_rx
        {
            match rx.try_recv()
            {
                Ok(data) => Some(Ok(data)),
                Err(TryRecvError::Empty) => None,
                Err(TryRecvError::Disconnected) => Some(Err(())),
            }
        }
        else
        {
            None
        };

        match outcome
        {
            Some(Ok((mut papers, errors))) =>
            {
                self.feed_rx = None;
                self.loading = false;

                if let Some(filter) = &self.topic_filter
                {
                    for p in &mut papers
                    {
                        p.score = filter.score(p);
                    }
                    papers.retain(|p| p.score > 0);
                    papers.sort_by(|a, b| b.score.cmp(&a.score));
                }

                self.papers = papers;
                self.paper_state.select(None);

                let vis_len = self.visible_paper_indices().len();
                let total = self.papers.len();
                let hidden = total - vis_len;

                if vis_len == 0 && total == 0
                {
                    self.status_msg = if !errors.is_empty()
                    {
                        format!("Error: {}", errors.join("; "))
                    }
                    else
                    {
                        "No papers found. Press 'r' to retry.".to_string()
                    };
                }
                else
                {
                    if vis_len > 0
                    {
                        self.paper_state.select(Some(0));
                    }
                    let hide_note = if hidden > 0
                    {
                        format!(", {} read/hidden (h=show)", hidden)
                    }
                    else
                    {
                        String::new()
                    };
                    let err_note = if errors.is_empty()
                    {
                        String::new()
                    }
                    else
                    {
                        format!(" ({} errors)", errors.len())
                    };
                    self.status_msg =
                        format!("Loaded {} papers{}{}", total, hide_note, err_note);
                }
            }
            Some(Err(())) =>
            {
                self.feed_rx = None;
                self.loading = false;
                self.status_msg = "Feed loading thread failed.".to_string();
            }
            None => {}
        }
    }

    // ── Keybindings ───────────────────────────────────────────────────────────

    pub fn on_key(&mut self, key: crossterm::event::KeyCode) -> bool
    {
        use crossterm::event::KeyCode;

        // Favorites picker modal intercepts all keys while open.
        if self.fav_picker
        {
            match key
            {
                KeyCode::Esc | KeyCode::Char('q') =>
                {
                    self.fav_picker = false;
                }
                KeyCode::Up | KeyCode::Char('k') =>
                {
                    let len = self.fav_picker_items().len();
                    if len > 0
                    {
                        let i = self.fav_picker_state.selected()
                            .map_or(0, |i| if i == 0 { len - 1 } else { i - 1 });
                        self.fav_picker_state.select(Some(i));
                    }
                }
                KeyCode::Down | KeyCode::Char('j') =>
                {
                    let len = self.fav_picker_items().len();
                    if len > 0
                    {
                        let i = self.fav_picker_state.selected()
                            .map_or(0, |i| (i + 1) % len);
                        self.fav_picker_state.select(Some(i));
                    }
                }
                KeyCode::Enter | KeyCode::Char(' ') =>
                {
                    self.fav_picker_confirm();
                }
                _ => {}
            }
            return false;
        }

        if self.show_help
        {
            self.show_help = false;
            return false;
        }

        match key
        {
            KeyCode::Char('q') | KeyCode::Esc =>
            {
                let _ = self.config.save();
                self.read_state.save();
                self.paper_faves.save();
                return true;
            }
            KeyCode::Char('?') =>
            {
                self.show_help = !self.show_help;
            }
            KeyCode::Tab =>
            {
                self.focus = match self.focus
                {
                    Panel::Categories => Panel::Papers,
                    Panel::Papers => Panel::Abstract,
                    Panel::Abstract => Panel::Categories,
                };
                self.abstract_scroll = 0;
            }
            KeyCode::Char('r') =>
            {
                let cats = if self.config.favorites.is_empty()
                {
                    CATEGORIES
                        .iter()
                        .filter(|(id, _)| id.starts_with("cs."))
                        .map(|(id, _)| id.to_string())
                        .collect::<Vec<_>>()
                }
                else
                {
                    self.config.favorites.clone()
                };
                self.spawn_load(cats, true);
            }
            KeyCode::Char('o') =>
            {
                if let Some(paper) = self.selected_paper()
                {
                    let link = paper.link.clone();
                    std::thread::spawn(move || { let _ = open::that(link); });
                    self.status_msg = format!("Opening: {}", paper.link);
                }
            }
            KeyCode::Char('p') =>
            {
                if let Some(paper) = self.selected_paper()
                {
                    let pdf = paper.pdf_link.clone();
                    if pdf.is_empty()
                    {
                        self.status_msg = "No PDF link available.".to_string();
                    }
                    else
                    {
                        let msg = pdf.clone();
                        std::thread::spawn(move || { let _ = open::that(pdf); });
                        self.status_msg = format!("Opening PDF: {}", msg);
                    }
                }
            }
            KeyCode::Char('h') =>
            {
                self.toggle_hide_read();
            }
            KeyCode::Char('s') =>
            {
                if let Some(paper) = self.selected_paper()
                {
                    let paper = paper.clone();
                    let link = paper.link.clone();
                    self.paper_faves.toggle(&paper);
                    self.paper_faves.save();
                    let saved = self.paper_faves.is_saved(&link);
                    self.status_msg = if saved
                    {
                        format!("Saved: {}", paper.title)
                    }
                    else
                    {
                        format!("Removed from saved: {}", paper.title)
                    };
                    // If we unsaved the current paper while in Saved tab, clamp selection.
                    if self.papers_tab == PapersTab::Saved && !saved
                    {
                        self.clamp_paper_selection();
                    }
                }
            }
            KeyCode::Char('S') =>
            {
                self.papers_tab = match self.papers_tab
                {
                    PapersTab::Feed => PapersTab::Saved,
                    PapersTab::Saved => PapersTab::Feed,
                };
                // Initialise selection when first entering Saved tab.
                if self.papers_tab == PapersTab::Saved
                    && self.saved_paper_state.selected().is_none()
                    && !self.paper_faves.papers().is_empty()
                {
                    self.saved_paper_state.select(Some(0));
                }
                self.abstract_scroll = 0;
            }
            KeyCode::Char('x') =>
            {
                if let Some(paper) = self.selected_paper()
                {
                    let link = paper.link.clone();
                    self.read_state.toggle(&link);
                    self.read_state.save();
                    self.clamp_paper_selection();
                    self.status_msg = if self.read_state.is_read(&link)
                    {
                        "Marked as read".to_string()
                    }
                    else
                    {
                        "Marked as unread".to_string()
                    };
                }
            }
            KeyCode::Char('f') =>
            {
                if self.focus == Panel::Categories
                {
                    self.toggle_favorite();
                }
            }
            KeyCode::Char('+') | KeyCode::Char('=') =>
            {
                if self.focus == Panel::Categories
                {
                    self.open_fav_picker();
                }
            }
            KeyCode::Char('-') =>
            {
                if self.focus == Panel::Categories
                {
                    self.remove_highlighted_favorite();
                }
            }
            KeyCode::Char('a') =>
            {
                if self.focus == Panel::Categories
                {
                    self.toggle_category_view();
                }
            }
            KeyCode::Up | KeyCode::Char('k') => self.navigate_up(),
            KeyCode::Down | KeyCode::Char('j') => self.navigate_down(),
            KeyCode::Enter | KeyCode::Char(' ') =>
            {
                if self.loading
                {
                    return false;
                }
                match self.focus
                {
                    Panel::Categories =>
                    {
                        self.load_feed_for_selected_category();
                        self.focus = Panel::Papers;
                    }
                    Panel::Papers =>
                    {
                        self.focus = Panel::Abstract;
                        self.abstract_scroll = 0;
                    }
                    Panel::Abstract =>
                    {
                        self.focus = Panel::Papers;
                        self.clamp_paper_selection();
                    }
                }
            }
            _ => {}
        }
        false
    }

    // ── Toggle helpers ────────────────────────────────────────────────────────

    fn toggle_hide_read(&mut self)
    {
        let current_link = self.selected_paper().map(|p| p.link.clone());

        self.hide_read = !self.hide_read;

        let vis = self.visible_paper_indices();
        if let Some(link) = current_link
        {
            if let Some(new_sel) = vis.iter().position(|&idx| self.papers[idx].link == link)
            {
                self.paper_state.select(Some(new_sel));
                return;
            }
        }
        self.clamp_paper_selection();

        let hidden = self.papers.len() - vis.len();
        self.status_msg = if self.hide_read
        {
            format!("Hiding {} read papers  (h=show all)", hidden)
        }
        else
        {
            format!("Showing all {} papers  (h=hide read)", self.papers.len())
        };
    }

    fn toggle_category_view(&mut self)
    {
        let current_id = self
            .visible_categories()
            .get(self.cat_state.selected().unwrap_or(0))
            .map(|(id, _)| id.to_string());

        self.show_all_categories = !self.show_all_categories;

        let vis = self.visible_categories();
        let new_idx = current_id
            .and_then(|id| vis.iter().position(|(v, _)| *v == id.as_str()))
            .unwrap_or(0);

        if vis.is_empty()
        {
            self.cat_state.select(None);
        }
        else
        {
            self.cat_state.select(Some(new_idx));
        }
    }

    pub fn toggle_favorite(&mut self)
    {
        let vis = self.visible_categories();
        if let Some((id, _)) = self.cat_state.selected().and_then(|i| vis.get(i))
        {
            let cat_id = id.to_string();
            if self.config.favorites.contains(&cat_id)
            {
                self.config.favorites.retain(|f| f != &cat_id);
                self.status_msg = format!("Removed {} from favorites", cat_id);
            }
            else
            {
                self.config.favorites.push(cat_id.clone());
                self.status_msg = format!("Added {} to favorites", cat_id);
            }
            let _ = self.config.save();
        }
    }

    // ── Navigation ────────────────────────────────────────────────────────────

    fn navigate_up(&mut self)
    {
        match self.focus
        {
            Panel::Categories =>
            {
                let len = self.visible_categories().len();
                if len == 0 { return; }
                let i = self.cat_state.selected()
                    .map_or(0, |i| if i == 0 { len - 1 } else { i - 1 });
                self.cat_state.select(Some(i));
            }
            Panel::Papers =>
            {
                match self.papers_tab
                {
                    PapersTab::Feed =>
                    {
                        let len = self.visible_paper_indices().len();
                        if len == 0 { return; }
                        let i = self.paper_state.selected()
                            .map_or(0, |i| if i == 0 { len - 1 } else { i - 1 });
                        self.paper_state.select(Some(i));
                    }
                    PapersTab::Saved =>
                    {
                        let len = self.paper_faves.papers().len();
                        if len == 0 { return; }
                        let i = self.saved_paper_state.selected()
                            .map_or(0, |i| if i == 0 { len - 1 } else { i - 1 });
                        self.saved_paper_state.select(Some(i));
                    }
                }
                self.abstract_scroll = 0;
            }
            Panel::Abstract =>
            {
                self.abstract_scroll = self.abstract_scroll.saturating_sub(1);
            }
        }
    }

    fn navigate_down(&mut self)
    {
        match self.focus
        {
            Panel::Categories =>
            {
                let len = self.visible_categories().len();
                if len == 0 { return; }
                let i = self.cat_state.selected().map_or(0, |i| (i + 1) % len);
                self.cat_state.select(Some(i));
            }
            Panel::Papers =>
            {
                match self.papers_tab
                {
                    PapersTab::Feed =>
                    {
                        let len = self.visible_paper_indices().len();
                        if len == 0 { return; }
                        let i = self.paper_state.selected().map_or(0, |i| (i + 1) % len);
                        self.paper_state.select(Some(i));
                    }
                    PapersTab::Saved =>
                    {
                        let len = self.paper_faves.papers().len();
                        if len == 0 { return; }
                        let i = self.saved_paper_state.selected().map_or(0, |i| (i + 1) % len);
                        self.saved_paper_state.select(Some(i));
                    }
                }
                self.abstract_scroll = 0;
            }
            Panel::Abstract =>
            {
                self.abstract_scroll += 1;
            }
        }
    }

    pub fn is_favorite(&self, cat_id: &str) -> bool
    {
        self.config.favorites.contains(&cat_id.to_string())
    }
}
