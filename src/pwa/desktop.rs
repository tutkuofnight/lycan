use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;

use super::config::AppConfig;

fn desktop_file_path(app_id: &str) -> Result<PathBuf> {
    let dir =
        dirs::data_dir().context("Could not determine data directory")?
        .join("applications");
    fs::create_dir_all(&dir)?;
    Ok(dir.join(format!("lycan-{}.desktop", app_id)))
}

fn find_lycan_binary() -> String {
    std::env::current_exe()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| "lycan".to_string())
}

pub fn create(config: &AppConfig) -> Result<()> {
    let icon = super::config::icon_path(&config.id)?;
    let lycan_bin = find_lycan_binary();

    let contents = format!(
        "[Desktop Entry]\n\
         Name={name}\n\
         Exec={exec} open {id}\n\
         Icon={icon}\n\
         Type=Application\n\
         Categories=Network;WebApp;\n\
         StartupWMClass=lycan-{id}\n",
        name = config.name,
        exec = lycan_bin,
        id = config.id,
        icon = icon.display(),
    );

    let path = desktop_file_path(&config.id)?;
    fs::write(&path, contents).context("Failed to write .desktop file")?;
    Ok(())
}

pub fn remove(app_id: &str) -> Result<()> {
    let path = desktop_file_path(app_id)?;
    if path.exists() {
        fs::remove_file(&path).context("Failed to remove .desktop file")?;
    }
    Ok(())
}
