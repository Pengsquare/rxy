use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

pub struct ReadState
{
    ids: HashSet<String>,
}

fn path() -> PathBuf
{
    let mut p = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    p.push("rxy");
    p.push("read.txt");
    p
}

impl ReadState
{
    pub fn load() -> Self
    {
        let ids = fs::read_to_string(path())
            .unwrap_or_default()
            .lines()
            .filter(|l| !l.is_empty())
            .map(String::from)
            .collect();
        ReadState { ids }
    }

    pub fn save(&self)
    {
        let p = path();
        if let Some(parent) = p.parent()
        {
            let _ = fs::create_dir_all(parent);
        }
        let mut v: Vec<&String> = self.ids.iter().collect();
        v.sort();
        let _ = fs::write(p, v.iter().map(|s| s.as_str()).collect::<Vec<_>>().join("\n"));
    }

    pub fn toggle(&mut self, link: &str)
    {
        if !self.ids.remove(link)
        {
            self.ids.insert(link.to_string());
        }
    }

    pub fn is_read(&self, link: &str) -> bool
    {
        self.ids.contains(link)
    }

    pub fn count(&self) -> usize
    {
        self.ids.len()
    }
}
