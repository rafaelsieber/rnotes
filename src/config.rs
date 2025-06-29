use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub root_directory: PathBuf,
    pub editor: String,
    pub git_enabled: bool,
    pub git_repository: Option<String>,
    pub git_username: Option<String>,
    pub git_email: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        let home_dir = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        let root_directory = home_dir.join("rnotes");
        
        Self {
            root_directory,
            editor: "vim".to_string(),
            git_enabled: false,
            git_repository: None,
            git_username: None,
            git_email: None,
        }
    }
}

impl Config {
    pub fn load_or_create() -> Result<Self> {
        let config_path = Self::config_file_path()?;
        
        if config_path.exists() {
            let content = fs::read_to_string(&config_path)?;
            let config: Config = serde_json::from_str(&content)?;
            
            // Ensure the root directory exists
            if !config.root_directory.exists() {
                fs::create_dir_all(&config.root_directory)?;
            }
            
            Ok(config)
        } else {
            let config = Config::default();
            
            // Create the root directory
            if !config.root_directory.exists() {
                fs::create_dir_all(&config.root_directory)?;
            }
            
            // Create config directory if it doesn't exist
            if let Some(parent) = config_path.parent() {
                fs::create_dir_all(parent)?;
            }
            
            config.save()?;
            Ok(config)
        }
    }
    
    pub fn save(&self) -> Result<()> {
        let config_path = Self::config_file_path()?;
        let content = serde_json::to_string_pretty(self)?;
        fs::write(config_path, content)?;
        Ok(())
    }
    
    fn config_file_path() -> Result<PathBuf> {
        let config_dir = dirs::config_dir()
            .ok_or_else(|| anyhow::anyhow!("Unable to find config directory"))?;
        Ok(config_dir.join("rnotes").join("config.json"))
    }
}
