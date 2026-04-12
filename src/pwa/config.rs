use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use slug::slugify;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub id: String,
    pub name: String,
    pub url: String,
    pub created_at: DateTime<Utc>,
}

impl AppConfig {
    pub fn new(name: &str, url: &str) -> Self {
        Self {
            id: slugify(name),
            name: name.to_string(),
            url: url.to_string(),
            created_at: Utc::now(),
        }
    }
}

pub fn data_dir() -> Result<PathBuf> {
    let base = dirs::data_dir().context("Could not determine XDG data directory")?;
    Ok(base.join("lycan").join("apps"))
}

pub fn app_dir(app_id: &str) -> Result<PathBuf> {
    Ok(data_dir()?.join(app_id))
}

pub fn icon_path(app_id: &str) -> Result<PathBuf> {
    Ok(app_dir(app_id)?.join("icon.png"))
}

pub fn save(config: &AppConfig) -> Result<()> {
    let dir = app_dir(&config.id)?;
    fs::create_dir_all(&dir).context("Failed to create app directory")?;
    let path = dir.join("config.json");
    let json = serde_json::to_string_pretty(config)?;
    fs::write(&path, json).context("Failed to write config")?;
    Ok(())
}

pub fn load(app_id: &str) -> Result<AppConfig> {
    let path = app_dir(app_id)?.join("config.json");
    let data = fs::read_to_string(&path).context("Failed to read config")?;
    let config: AppConfig = serde_json::from_str(&data)?;
    Ok(config)
}

pub fn list_apps() -> Result<Vec<AppConfig>> {
    let dir = data_dir()?;
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut apps = Vec::new();
    for entry in fs::read_dir(&dir)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            let config_path = entry.path().join("config.json");
            if config_path.exists() {
                if let Ok(data) = fs::read_to_string(&config_path) {
                    if let Ok(config) = serde_json::from_str::<AppConfig>(&data) {
                        apps.push(config);
                    }
                }
            }
        }
    }
    apps.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(apps)
}

pub fn update(app_id: &str, new_name: &str, new_url: &str) -> Result<AppConfig> {
    let mut config = load(app_id)?;
    let url_changed = config.url != new_url;
    config.name = new_name.to_string();
    config.url = new_url.to_string();
    save(&config)?;

    super::desktop::create(&config)?;

    if url_changed {
        let icon = icon_path(app_id)?;
        let _ = super::favicon::fetch_and_save(new_url, &icon);
    }

    Ok(config)
}

pub fn delete(app_id: &str) -> Result<()> {
    let dir = app_dir(app_id)?;
    if dir.exists() {
        fs::remove_dir_all(&dir).context("Failed to remove app directory")?;
    }
    Ok(())
}
