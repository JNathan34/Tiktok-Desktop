use crate::settings::{DisplaySettings, Feed};
use tauri::{AppHandle, Manager, Url, Webview};

pub const ORIGIN: &str = "https://www.tiktok.com";
pub const CONTENT: &str = "tiktok-site";
pub const SHELL: &str = "shell";
pub const LIFECYCLE: &str = include_str!("playback.js");

pub fn display_script(settings: DisplaySettings) -> String {
    let json = serde_json::to_string(&settings).expect("display preferences are serializable");
    format!("({})({json})", include_str!("display.js"))
}

pub fn feed_url(feed: Feed) -> Url {
    url(match feed {
        Feed::Home => "/foryou",
        Feed::Following => "/following",
    })
}

pub fn url(path: &str) -> Url {
    Url::parse(ORIGIN)
        .expect("valid TikTok origin")
        .join(path)
        .expect("fixed TikTok path")
}

pub fn is_tiktok(url: &Url) -> bool {
    url.scheme() == "https"
        && url.port().is_none()
        && url.username().is_empty()
        && url.password().is_none()
        && matches!(
            url.host_str(),
            Some(
                "www.tiktok.com"
                    | "tiktok.com"
                    | "m.tiktok.com"
                    | "vm.tiktok.com"
                    | "vt.tiktok.com"
            )
        )
}

pub fn content(app: &AppHandle) -> Result<Webview, String> {
    app.get_webview(CONTENT)
        .ok_or_else(|| "TikTok view is unavailable.".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remote_navigation_does_not_cross_the_origin_allowlist() {
        for good in [
            "https://www.tiktok.com/foryou",
            "https://vm.tiktok.com/example/",
        ] {
            assert!(is_tiktok(&Url::parse(good).unwrap()), "{good}");
        }
        for bad in [
            "http://www.tiktok.com",
            "https://www.tiktok.com.evil.example",
            "https://www.tiktok.com@evil.example",
            "https://name@www.tiktok.com",
            "https://www.tiktok.com:8443",
            "file:///C:/Windows",
            "http://tauri.localhost",
            "https://tauri.localhost",
            "javascript:alert(1)",
            "https://evil.tiktok.com",
        ] {
            assert!(!is_tiktok(&Url::parse(bad).unwrap()), "{bad}");
        }
    }
}
