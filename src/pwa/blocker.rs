use webkit2gtk::{
    UserContentInjectedFrames, UserContentManager, UserContentManagerExt as _, UserScript,
    UserScriptInjectionTime,
};

const BLOCKED_DOMAINS: &[&str] = &[
    "doubleclick.net",
    "googlesyndication.com",
    "googleadservices.com",
    "google-analytics.com",
    "googletagmanager.com",
    "googletagservices.com",
    "adservice.google.com",
    "pagead2.googlesyndication.com",
    "facebook.net/tr",
    "connect.facebook.net",
    "analytics.facebook.com",
    "ads-twitter.com",
    "static.ads-twitter.com",
    "ads.linkedin.com",
    "snap.licdn.com",
    "adnxs.com",
    "adsrvr.org",
    "amazon-adsystem.com",
    "criteo.com",
    "criteo.net",
    "outbrain.com",
    "taboola.com",
    "scorecardresearch.com",
    "quantserve.com",
    "hotjar.com",
    "fullstory.com",
    "mixpanel.com",
    "segment.io",
    "segment.com",
    "sentry-cdn.com",
    "bat.bing.com",
    "clarity.ms",
];

pub fn apply(content_manager: &UserContentManager) {
    let domain_checks: String = BLOCKED_DOMAINS
        .iter()
        .map(|d| format!("url.includes('{}')", d))
        .collect::<Vec<_>>()
        .join(" || ");

    let script_source = format!(
        r#"
(function() {{
    const isBlocked = (url) => {domain_checks};

    // Block fetch requests to ad/tracker domains
    const origFetch = window.fetch;
    window.fetch = function(input, init) {{
        const url = (typeof input === 'string') ? input : input.url;
        if (isBlocked(url)) return Promise.reject(new Error('blocked'));
        return origFetch.apply(this, arguments);
    }};

    // Block XMLHttpRequest to ad/tracker domains
    const origOpen = XMLHttpRequest.prototype.open;
    XMLHttpRequest.prototype.open = function(method, url) {{
        if (isBlocked(url)) {{
            this._blocked = true;
            return;
        }}
        return origOpen.apply(this, arguments);
    }};
    const origSend = XMLHttpRequest.prototype.send;
    XMLHttpRequest.prototype.send = function() {{
        if (this._blocked) return;
        return origSend.apply(this, arguments);
    }};

    // Block beacon requests
    const origBeacon = navigator.sendBeacon;
    if (origBeacon) {{
        navigator.sendBeacon = function(url) {{
            if (isBlocked(url)) return true;
            return origBeacon.apply(this, arguments);
        }};
    }}
}})();
"#
    );

    let script = UserScript::new(
        &script_source,
        UserContentInjectedFrames::AllFrames,
        UserScriptInjectionTime::Start,
        &[],
        &[],
    );

    content_manager.add_script(&script);
}
