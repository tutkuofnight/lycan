//! Restrict on-disk Lycan data to the owning user (Unix `chmod` 0700 / 0600).
//! Other local accounts cannot read WebKit profiles or `config.json` without root.

use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

#[cfg(unix)]
pub fn dir_owner_only(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let meta = fs::metadata(path).with_context(|| format!("stat {}", path.display()))?;
    if !meta.is_dir() {
        return Ok(());
    }
    let mut perms = meta.permissions();
    perms.set_mode(0o700);
    fs::set_permissions(path, perms).with_context(|| format!("chmod 700 {}", path.display()))?;
    Ok(())
}

#[cfg(unix)]
pub fn file_owner_only(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let meta = fs::metadata(path).with_context(|| format!("stat {}", path.display()))?;
    if !meta.is_file() {
        return Ok(());
    }
    let mut perms = meta.permissions();
    perms.set_mode(0o600);
    fs::set_permissions(path, perms).with_context(|| format!("chmod 600 {}", path.display()))?;
    Ok(())
}

#[cfg(not(unix))]
pub fn dir_owner_only(_path: &Path) -> Result<()> {
    Ok(())
}

#[cfg(not(unix))]
pub fn file_owner_only(_path: &Path) -> Result<()> {
    Ok(())
}
