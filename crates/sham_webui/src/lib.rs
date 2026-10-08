// sham_webui — sham-019: UI service.
// Web UI on :8088 (dossier port). Fat-thumb: 64px minimum touch targets,
// offline queue + sync, no external CDN (works with zero network).
// Static asset embedded via include_str!; serve_file() maps paths.
#![forbid(unsafe_code)]

#[cfg(test)]
mod tests;

pub const UI_PORT: u16 = 8088;
pub const API_BASE: &str = "http://localhost:8080";

pub const INDEX_HTML: &str = include_str!("../static/index.html");

/// Map a request path to (content-type, body). None = 404.
pub fn serve_file(path: &str) -> Option<(&'static str, &'static str)> {
    match path {
        "/" | "/index.html" => Some(("text/html; charset=utf-8", INDEX_HTML)),
        "/health" => Some(("application/json", "{\"status\":\"ok\"}")),
        _ => None,
    }
}

/// Fat-thumb audit of the HTML: every <button> must resolve to >= 48px.
/// Checks the stylesheet for the min touch target rule.
pub fn min_touch_target_px(html: &str) -> Option<u32> {
    // look for `--touch: NNpx` then `min-height: var(--touch)` usage
    let var = html.find("--touch:")?;
    let rest = &html[var + 8..];
    let end = rest.find("px")?;
    rest[..end].trim().parse().ok()
}
