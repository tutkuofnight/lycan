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

pub fn script() -> String {
    let domain_list: String = BLOCKED_DOMAINS
        .iter()
        .map(|d| format!("'{d}'"))
        .collect::<Vec<_>>()
        .join(",");

    format!(
        r#"
(function() {{
    const _bd = new Set([{domain_list}]);
    const isBlocked = (raw) => {{
        try {{
            const h = new URL(raw, location.href).hostname;
            for (const d of _bd) {{ if (h === d || h.endsWith('.' + d)) return true; }}
        }} catch(_) {{}}
        return false;
    }};

    const _f = window.fetch;
    window.fetch = function(input, init) {{
        const u = (typeof input === 'string') ? input : (input && input.url) || '';
        if (isBlocked(u)) return Promise.resolve(new Response('', {{status: 200, statusText: 'OK'}}));
        return _f.apply(this, arguments);
    }};

    const _xo = XMLHttpRequest.prototype.open;
    const _xs = XMLHttpRequest.prototype.send;
    XMLHttpRequest.prototype.open = function(method, url) {{
        this._blockedUrl = isBlocked(String(url || ''));
        if (!this._blockedUrl) return _xo.apply(this, arguments);
    }};
    XMLHttpRequest.prototype.send = function() {{
        if (this._blockedUrl) {{
            Object.defineProperty(this, 'status', {{value: 200, writable: false}});
            Object.defineProperty(this, 'readyState', {{value: 4, writable: false}});
            Object.defineProperty(this, 'responseText', {{value: '', writable: false}});
            Object.defineProperty(this, 'response', {{value: '', writable: false}});
            if (typeof this.onreadystatechange === 'function') {{
                try {{ this.onreadystatechange(new Event('readystatechange')); }} catch(_) {{}}
            }}
            if (typeof this.onload === 'function') {{
                try {{ this.onload(new Event('load')); }} catch(_) {{}}
            }}
            return;
        }}
        return _xs.apply(this, arguments);
    }};

    if (navigator.sendBeacon) {{
        const _sb = navigator.sendBeacon.bind(navigator);
        navigator.sendBeacon = function(url) {{
            if (isBlocked(String(url))) return true;
            return _sb.apply(this, arguments);
        }};
    }}
}})();
"#
    )
}
