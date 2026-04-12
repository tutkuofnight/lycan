use anyhow::{Context, Result};
use gtk::prelude::*;
use webkit2gtk::{SettingsExt, WebViewExt, WebViewExtManual};

use crate::pwa::{blocker, config};

const USER_AGENT: &str = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36";

fn is_nvidia() -> bool {
    std::fs::read_dir("/proc/driver/nvidia").is_ok()
        || std::path::Path::new("/dev/nvidiactl").exists()
}

pub fn open(app_id: &str) -> Result<()> {
    if is_nvidia() {
        std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
        std::env::set_var("WEBKIT_DISABLE_COMPOSITING_MODE", "1");
        std::env::set_var("LIBGL_ALWAYS_SOFTWARE", "1");
    }

    let app_config = config::load(app_id)
        .with_context(|| format!("PWA '{}' not found. Create it first with the TUI.", app_id))?;

    gtk::init().context("Failed to initialize GTK")?;

    let window = gtk::Window::new(gtk::WindowType::Toplevel);
    window.set_title(&app_config.name);
    window.set_default_size(1024, 768);

    let web_context = webkit2gtk::WebContext::default().unwrap();

    let content_manager = webkit2gtk::UserContentManager::new();
    blocker::apply(&content_manager);

    let webview: webkit2gtk::WebView =
        WebViewExtManual::new_with_context_and_user_content_manager(
            &web_context,
            &content_manager,
        );

    if let Some(settings) = WebViewExt::settings(&webview) {
        settings.set_user_agent(Some(USER_AGENT));
        settings.set_enable_javascript(true);
        settings.set_enable_developer_extras(true);
        settings.set_javascript_can_access_clipboard(true);
        settings.set_enable_webaudio(true);

        settings.set_enable_java(false);
        settings.set_enable_plugins(false);
        settings.set_enable_smooth_scrolling(true);
        settings.set_enable_page_cache(true);
        settings.set_enable_offline_web_application_cache(false);
        settings.set_enable_hyperlink_auditing(false);
    }

    webview.load_uri(&app_config.url);
    window.add(&webview);
    window.show_all();

    window.connect_delete_event(|_, _| {
        gtk::main_quit();
        glib::Propagation::Stop
    });

    gtk::main();
    Ok(())
}
