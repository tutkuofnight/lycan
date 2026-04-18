use std::fs;

use anyhow::{Context, Result};
use gtk::prelude::*;
use webkit2gtk::{
    HardwareAccelerationPolicy, MemoryPressureSettings, SettingsExt, WebViewExt,
    WebsiteDataManager,
};
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

        // Scroll/animation jank mitigations (perf plan phase 1):
        // - Smooth scrolling enables WebKit's thread-decoupled async scroll path with
        //   momentum, which is what the kinetic feel of modern web apps assumes.
        // - Back/forward navigation gestures are useless in a single-page PWA window
        //   and still cost input-event bookkeeping.
        // - Hyperlink auditing (<a ping>) fires extra beacons we don't need.
        settings.set_enable_smooth_scrolling(true);
        settings.set_enable_back_forward_navigation_gestures(false);
        settings.set_enable_hyperlink_auditing(false);

        #[cfg(debug_assertions)]
        settings.set_enable_write_console_messages_to_stdout(true);
        #[cfg(not(debug_assertions))]
        settings.set_enable_write_console_messages_to_stdout(false);
    }
}

/// Looser memory-pressure thresholds trade a little RAM for smoother animation: fewer
/// mid-scroll GC pauses in heavy web apps (YouTube Music, Discord, etc). On low-RAM
/// systems (<4 GiB) we fall back to tight defaults so WebKit doesn't push us into swap.
fn configure_memory_pressure() {
    let mut mps = MemoryPressureSettings::new();
    let total_kb = read_meminfo_total_kb().unwrap_or(0);
    if total_kb > 0 && total_kb < 4 * 1024 * 1024 {
        mps.set_conservative_threshold(0.50);
        mps.set_strict_threshold(0.75);
        mps.set_kill_threshold(0.90);
        mps.set_poll_interval(30.0);
    } else {
        mps.set_conservative_threshold(0.75);
        mps.set_strict_threshold(0.90);
        mps.set_kill_threshold(0.95);
        mps.set_poll_interval(60.0);
    }
    WebsiteDataManager::set_memory_pressure_settings(&mut mps);
}

fn read_meminfo_total_kb() -> Option<u64> {
    let s = fs::read_to_string("/proc/meminfo").ok()?;
    for line in s.lines() {
        if let Some(rest) = line.strip_prefix("MemTotal:") {
            return rest.split_whitespace().next()?.parse().ok();
        }
    }
    None
}

fn is_x11_session() -> bool {
    match std::env::var("XDG_SESSION_TYPE").ok().as_deref() {
        Some("x11") => true,
        Some("wayland") => false,
        _ => std::env::var_os("WAYLAND_DISPLAY").is_none()
            && std::env::var_os("DISPLAY").is_some(),
    }
}

/// Wayland is always composited. On X11, ask GDK (which reads `_NET_WM_CM_S0`).
/// Must be called after `gtk::init()`.
fn detect_compositor() -> Option<bool> {
    if !is_x11_session() {
        return Some(true);
    }
    gtk::gdk::Screen::default().map(|s| s.is_composited())
}

/// Applies `~/.local/share/lycan/webkit-tuning.json` (created on first `lycan` run) unless overridden:
///
/// - If `WEBKIT_DISABLE_DMABUF_RENDERER` / `WEBKIT_DISABLE_COMPOSITING_MODE` are already set, they are left alone.
/// - `LYCAN_WEBKIT_DISABLE_DMABUF=1|0` or `LYCAN_WEBKIT_DISABLE_COMPOSITING=1|0` override the file for that process.
/// - On X11 without a running compositor, WebKit compositing is disabled to avoid
///   double-buffering against the X server (a common scroll-jank cause).
/// - Delete `webkit-tuning.json` to regenerate after a GPU or session-type change.
fn apply_webkit_environment(has_compositor: Option<bool>) {
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
            None => {
                let no_x_compositor = is_x11_session() && has_compositor == Some(false);
                p.disable_compositing || no_x_compositor
            }
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
    let app_config = config::load(app_id)
        .with_context(|| format!("PWA '{}' not found. Create it first with the TUI.", app_id))?;

    // Wayland compositors (Mutter/GNOME Shell) match windows to .desktop files by comparing
    // the window's app_id to the desktop file basename. GTK seeds the Wayland app_id from
    // `g_prgname`, which defaults to argv[0] (`lycan`) — so without this every PWA window
    // would collide on the same app_id and miss its per-app icon. Set it to the desktop
    // file stem (see `pwa::desktop::desktop_file_path`) before `gtk::init()`.
    glib::set_prgname(Some(format!("lycan-{}", app_config.id).as_str()));

    gtk::init().context("Failed to initialize GTK")?;

    // Compositor detection requires GDK, and WebKit env vars must be set before the first
    // web process spawns (i.e. before `WebContext::new`), so this order is load-bearing.
    let has_compositor = detect_compositor();
    apply_webkit_environment(has_compositor);
    configure_memory_pressure();

    let window = gtk::Window::new(gtk::WindowType::Toplevel);
    window.set_title(&app_config.name);
    window.set_default_size(1024, 768);

    // Fallback for X11 and compositors that read the GtkWindow icon directly.
    let icon_path = config::icon_path(&app_config.id)?;
    if icon_path.exists() {
        let _ = window.set_icon_from_file(&icon_path);
    }

    let app_root = config::app_dir(&app_config.id)?;
    if app_root.exists() {
        let _ = fs_private::dir_owner_only(&app_root);
    }

    let data_dir = config::webview_data_dir(&app_config.id)?;
    fs::create_dir_all(&data_dir).with_context(|| format!("create {}", data_dir.display()))?;
    fs_private::dir_owner_only(&data_dir).with_context(|| format!("chmod 700 {}", data_dir.display()))?;

    // Keep `WebContext` alive for the whole GTK loop — dropping it early breaks the WebView.
    let mut web_context = WebContext::new(Some(data_dir));

    let init_script = build_init_script();

    let _webview = wry::WebViewBuilder::new_with_web_context(&mut web_context)
        .with_url(&app_config.url)
        .with_user_agent(USER_AGENT)
        .with_initialization_script_for_main_only(&init_script, false)
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

fn build_init_script() -> String {
    let mut script = blocker::script();
    if !GL_PROBE_SCRIPT.is_empty() {
        script.push('\n');
        script.push_str(GL_PROBE_SCRIPT);
    }
    script
}

/// Debug-only GL renderer probe. Logs the active WebGL renderer to stdout so we can
/// catch silent `llvmpipe` (software) fallbacks — the usual cause of WebGL-heavy PWAs
/// feeling sluggish when the driver stack isn't actually on GPU.
#[cfg(debug_assertions)]
const GL_PROBE_SCRIPT: &str = r#"
document.addEventListener('DOMContentLoaded', () => {
  try {
    const c = document.createElement('canvas');
    const g = c.getContext('webgl2') || c.getContext('webgl');
    if (!g) { console.log('[lycan] WebGL: unavailable'); return; }
    const ext = g.getExtension('WEBGL_debug_renderer_info');
    const r = ext ? g.getParameter(ext.UNMASKED_RENDERER_WEBGL) : '(unmasked info hidden)';
    const v = ext ? g.getParameter(ext.UNMASKED_VENDOR_WEBGL) : '';
    console.log('[lycan] GL:', v, '/', r);
    const rl = String(r).toLowerCase();
    if (rl.includes('llvmpipe') || rl.includes('swrast') || rl.includes('software')) {
      console.warn('[lycan] Software rasterizer active — hardware acceleration is NOT on.');
    }
  } catch (e) { console.log('[lycan] GL probe failed:', String(e)); }
}, { once: true });
"#;

#[cfg(not(debug_assertions))]
const GL_PROBE_SCRIPT: &str = "";
