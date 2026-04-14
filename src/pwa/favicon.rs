use anyhow::{Context, Result};
use image::ImageFormat;

use crate::fs_private;
use scraper::{Html, Selector};
use url::Url;

pub fn fetch_and_save(site_url: &str, save_path: &std::path::Path) -> Result<()> {
    let base_url = Url::parse(site_url).context("Invalid URL")?;

    let icon_url = find_best_icon(&base_url).unwrap_or_else(|| {
        let mut fallback = base_url.clone();
        fallback.set_path("/favicon.ico");
        fallback.set_query(None);
        fallback.set_fragment(None);
        fallback.to_string()
    });

    let resp = ureq::get(&icon_url).call().context("Failed to download favicon")?;
    let bytes = resp
        .into_body()
        .read_to_vec()
        .context("Failed to read favicon bytes")?;

    if let Some(parent) = save_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    match image::load_from_memory(&bytes) {
        Ok(img) => {
            let resized = img.resize(128, 128, image::imageops::FilterType::Lanczos3);
            resized.save_with_format(save_path, ImageFormat::Png)?;
        }
        Err(_) => {
            std::fs::write(save_path, &bytes)?;
        }
    }

    let _ = fs_private::file_owner_only(save_path);

    Ok(())
}

fn find_best_icon(base_url: &Url) -> Option<String> {
    let resp = ureq::get(base_url.as_str()).call().ok()?;
    let html = resp.into_body().read_to_string().ok()?;

    let document = Html::parse_document(&html);

    let selectors = [
        "link[rel='apple-touch-icon']",
        "link[rel='icon'][sizes]",
        "link[rel='icon']",
        "link[rel='shortcut icon']",
    ];

    for sel_str in &selectors {
        if let Ok(selector) = Selector::parse(sel_str) {
            let mut best: Option<(i32, String)> = None;

            for element in document.select(&selector) {
                if let Some(href) = element.value().attr("href") {
                    let size = element
                        .value()
                        .attr("sizes")
                        .and_then(parse_largest_size)
                        .unwrap_or(0);

                    let absolute = resolve_url(base_url, href);

                    match &best {
                        Some((best_size, _)) if size > *best_size => {
                            best = Some((size, absolute));
                        }
                        None => {
                            best = Some((size, absolute));
                        }
                        _ => {}
                    }
                }
            }

            if let Some((_, url)) = best {
                return Some(url);
            }
        }
    }

    None
}

fn parse_largest_size(sizes: &str) -> Option<i32> {
    sizes
        .split_whitespace()
        .filter_map(|s| {
            let lower = s.to_lowercase();
            let parts: Vec<&str> = lower.split('x').collect();
            if parts.len() == 2 {
                parts[0].parse::<i32>().ok()
            } else {
                None
            }
        })
        .max()
}

fn resolve_url(base: &Url, href: &str) -> String {
    match Url::parse(href) {
        Ok(absolute) => absolute.to_string(),
        Err(_) => base.join(href).map(|u| u.to_string()).unwrap_or_else(|_| href.to_string()),
    }
}
