//! Persistent WebKit tuning for this machine (`~/.local/share/lycan/webkit-tuning.json`).
//! Generated on first `lycan` run from DRM PCI vendors and session type.
//! Run `lycan detect-system` to recompute after a GPU change or switching X11/Wayland.
//!
//! `LYCAN_WEBKIT_*` env vars still override the file for one-off debugging.

use std::fs;
use std::path::PathBuf;
use std::sync::OnceLock;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct WebKitTuningProfile {
    pub version: u32,
    /// Maps to `WEBKIT_DISABLE_DMABUF_RENDERER=1` when true (recommended on NVIDIA).
    pub disable_dmabuf: bool,
    /// Maps to `WEBKIT_DISABLE_COMPOSITING_MODE=1` when true (trade-off: can fix some
    /// Wayland+NVIDIA glitches; may reduce smoothness elsewhere).
    pub disable_compositing: bool,
}

impl Default for WebKitTuningProfile {
    fn default() -> Self {
        Self {
            version: 1,
            disable_dmabuf: false,
            disable_compositing: false,
        }
    }
}

static PROFILE: OnceLock<WebKitTuningProfile> = OnceLock::new();

/// Warm cache and create `webkit-tuning.json` if missing. Call from `main` before GTK.
pub fn init() {
    let _ = profile();
}

pub fn profile() -> &'static WebKitTuningProfile {
    PROFILE.get_or_init(load_or_create)
}

fn tuning_path() -> Option<PathBuf> {
    let base = dirs::data_dir()?.join("lycan");
    Some(base.join("webkit-tuning.json"))
}

/// NVIDIA kernel module / device nodes (works when DRM vendor scan is ambiguous).
fn nvidia_driver_loaded() -> bool {
    fs::read_dir("/proc/driver/nvidia").is_ok() || std::path::Path::new("/dev/nvidiactl").exists()
}

/// Any DRM `cardN` device reporting PCI vendor NVIDIA (0x10de).
fn nvidia_on_drm_bus() -> bool {
    let drm = std::path::Path::new("/sys/class/drm");
    let Ok(entries) = fs::read_dir(drm) else {
        return false;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if !name.starts_with("card") || name.contains('-') {
            continue;
        }
        let idx = &name[4..];
        if idx.is_empty() || !idx.chars().all(|c| c.is_ascii_digit()) {
            continue;
        }
        let vendor_path = entry.path().join("device").join("vendor");
        let Ok(v) = fs::read_to_string(&vendor_path) else {
            continue;
        };
        if v.trim().eq_ignore_ascii_case("0x10de") {
            return true;
        }
    }
    false
}

pub fn nvidia_detected() -> bool {
    nvidia_on_drm_bus() || nvidia_driver_loaded()
}

fn compute_recommended() -> WebKitTuningProfile {
    let nvidia = nvidia_detected();
    let wayland = std::env::var_os("WAYLAND_DISPLAY").is_some();
    WebKitTuningProfile {
        version: 1,
        disable_dmabuf: nvidia,
        // Wayland + proprietary NVIDIA stack is a common source of WebKit compositor bugs;
        // disabling compositing mode sometimes stabilizes resizing / overlays.
        disable_compositing: nvidia && wayland,
    }
}

fn load_or_create() -> WebKitTuningProfile {
    let Some(path) = tuning_path() else {
        return compute_recommended();
    };

    if let Ok(bytes) = fs::read(&path) {
        if let Ok(parsed) = serde_json::from_slice::<WebKitTuningProfile>(&bytes) {
            if parsed.version >= 1 {
                return parsed;
            }
        }
    }

    let recommended = compute_recommended();
    if let Some(parent) = path.parent() {
        if fs::create_dir_all(parent).is_ok() {
            if let Ok(json) = serde_json::to_string_pretty(&recommended) {
                let _ = fs::write(&path, json);
            }
        }
    }

    recommended
}

/// Recompute tuning from the current machine and overwrite `webkit-tuning.json`.
pub fn regenerate_profile() -> Result<()> {
    let path = tuning_path().context("Could not resolve data directory for lycan")?;
    let parent = path.parent().context("webkit-tuning path has no parent")?;
    fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    let p = compute_recommended();
    let json = serde_json::to_string_pretty(&p).context("serialize webkit profile")?;
    fs::write(&path, json).with_context(|| format!("write {}", path.display()))?;
    println!("WebKit tuning profile updated: {}", path.display());
    println!("  disable_dmabuf: {}", p.disable_dmabuf);
    println!("  disable_compositing: {}", p.disable_compositing);
    Ok(())
}
