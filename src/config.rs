use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config
{
    pub favorites: Vec<String>,
}

impl Default for Config
{
    fn default() -> Self
    {
        Config
        {
            favorites: vec![],
        }
    }
}

fn config_path() -> PathBuf
{
    let mut path = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    path.push("rxy");
    path.push("config.toml");
    path
}

impl Config
{
    pub fn load() -> Self
    {
        let path = config_path();
        if path.exists()
        {
            match fs::read_to_string(&path)
            {
                Ok(contents) =>
                {
                    match toml::from_str(&contents)
                    {
                        Ok(config) => return config,
                        Err(_) => {}
                    }
                }
                Err(_) => {}
            }
        }
        let config = Config::default();
        let _ = config.save();
        config
    }

    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>>
    {
        let path = config_path();
        if let Some(parent) = path.parent()
        {
            fs::create_dir_all(parent)?;
        }
        let contents = toml::to_string_pretty(self)?;
        fs::write(&path, contents)?;
        Ok(())
    }
}
