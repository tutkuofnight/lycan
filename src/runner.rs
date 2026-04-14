use std::fs;

use anyhow::{Context, Result};
use gtk::prelude::*;
use webkit2gtk::{HardwareAccelerationPolicy, SettingsExt, WebViewExt};
use wry::{WebContext, WebView, WebViewBuilderExtUnix, WebViewExtUnix};

use crate::fs_private;
use crate::pwa::{blocker, config};
use crate::webkit_tuning;

const USER_AGENT: &str = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/133.0.0.0 Safari/537.36";

/// WebKitGTK tuning (perf plan §1): GPU path, DNS prefetch, caches, media.
///
/// **JIT / YARR:** There are no `webkit_settings_set_enable_jit` / `…_yarr…` symbols in current
/// `webkit2gtk-sys`, no matching methods on `Settings` in `webkit2gtk` 2.0.x, and the upstream
/// [`WebKitSettings`](https://webkitgtk.org/reference/webkitgtk/stable/class.Settings.html) API docs
/// list no JIT toggles — execution tiers live inside JavaScriptCore and are not exposed per-webview
/// on the GTK port (unlike old drafts some guides still mention).
///
/// Not done here (process model / portability): shared `WebContext` across PWAs, prewarm pool,
/// suspend APIs; forced `GDK_BACKEND`, Mesa overrides, or disabling WebKit sandbox (breaks Wayland,
/// some drivers, or security).
fn tune_webkit_view(webview: &WebView) {
    let wk = webview.webview();
    if let Some(settings) = WebViewExt::settings(&wk) {
        settings.set_enable_javascript(true);
        settings.set_enable_javascript_markup(true);
        settings.set_javascript_can_access_clipboard(true);

        settings.set_enable_dns_prefetching(true);

        settings.set_enable_webgl(true);
        #[allow(deprecated)]
        settings.set_enable_accelerated_2d_canvas(true);
        settings.set_hardware_acceleration_policy(HardwareAccelerationPolicy::Always);

        settings.set_enable_page_cache(true);

        settings.set_enable_media_stream(true);
        settings.set_enable_webaudio(true);

        #[cfg(not(debug_assertions))]
        settings.set_enable_write_console_messages_to_stdout(false);
    }
}

/// Applies `~/.local/share/lycan/webkit-tuning.json` (created on first `lycan` run) unless overridden:
///
/// - If `WEBKIT_DISABLE_DMABUF_RENDERER` / `WEBKIT_DISABLE_COMPOSITING_MODE` are already set, they are left alone.
/// - `LYCAN_WEBKIT_DISABLE_DMABUF=1|0` or `LYCAN_WEBKIT_DISABLE_COMPOSITING=1|0` override the file for that process.
/// - Delete `webkit-tuning.json` to regenerate after a GPU or session-type change.
fn apply_webkit_environment() {
    let p = webkit_tuning::profile();

    if std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none() {
        let disable_dmabuf = match env_truthy("LYCAN_WEBKIT_DISABLE_DMABUF") {
            Some(v) => v,
            None => p.disable_dmabuf,
        };
        if disable_dmabuf {
            unsafe {
                std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
            }
        }
    }

    if std::env::var_os("WEBKIT_DISABLE_COMPOSITING_MODE").is_none() {
        let disable_comp = match env_truthy("LYCAN_WEBKIT_DISABLE_COMPOSITING") {
            Some(v) => v,
            None => p.disable_compositing,
        };
        if disable_comp {
            unsafe {
                std::env::set_var("WEBKIT_DISABLE_COMPOSITING_MODE", "1");
            }
        }
    }
}

fn env_truthy(name: &str) -> Option<bool> {
    let v = std::env::var(name).ok()?;
    match v.to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Some(true),
        "0" | "false" | "no" | "off" => Some(false),
        _ => None,
    }
}

pub fn open(app_id: &str) -> Result<()> {
    apply_webkit_environment();

    let app_config = config::load(app_id)
        .with_context(|| format!("PWA '{}' not found. Create it first with the TUI.", app_id))?;

    gtk::init().context("Failed to initialize GTK")?;

    let window = gtk::Window::new(gtk::WindowType::Toplevel);
    window.set_title(&app_config.name);
    window.set_default_size(1024, 768);

    let app_root = config::app_dir(&app_config.id)?;
    if app_root.exists() {
        let _ = fs_private::dir_owner_only(&app_root);
    }

    let data_dir = config::webview_data_dir(&app_config.id)?;
    fs::create_dir_all(&data_dir).with_context(|| format!("create {}", data_dir.display()))?;
    fs_private::dir_owner_only(&data_dir).with_context(|| format!("chmod 700 {}", data_dir.display()))?;

    // Keep `WebContext` alive for the whole GTK loop — dropping it early breaks the WebView.
    let mut web_context = WebContext::new(Some(data_dir));

    let _webview = wry::WebViewBuilder::new_with_web_context(&mut web_context)
        .with_url(&app_config.url)
        .with_user_agent(USER_AGENT)
        .with_initialization_script_for_main_only(&blocker::script(), false)
        .with_devtools(cfg!(debug_assertions))
        .with_clipboard(true)
        .build_gtk(&window)
        .context("Failed to create WebView")?;

    tune_webkit_view(&_webview);

    window.show_all();

    window.connect_delete_event(|_, _| {
        gtk::main_quit();
        glib::Propagation::Stop
    });

    gtk::main();
    Ok(())
}
