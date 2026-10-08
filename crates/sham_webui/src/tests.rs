use super::*;

#[test]
fn ui_port_is_8088() {
    assert_eq!(UI_PORT, 8088);
}
#[test]
fn serves_index() {
    let (ct, body) = serve_file("/").unwrap();
    assert!(ct.contains("text/html"));
    assert!(body.contains("Shambhala Ops"));
}
#[test]
fn serves_index_html_path() {
    assert!(serve_file("/index.html").is_some());
}
#[test]
fn unknown_404() {
    assert!(serve_file("/nope").is_none());
}
#[test]
fn min_touch_target_generous() {
    let px = min_touch_target_px(INDEX_HTML).unwrap();
    assert!(px >= 48, "px={}", px);
}
#[test]
fn no_external_cdn() {
    // offline-first: no http(s) script/link/src references except the API base
    for needle in ["src=\"http", "href=\"http", "cdn.", "googleapis", "unpkg"] {
        assert!(!INDEX_HTML.contains(needle), "found {}", needle);
    }
    assert!(INDEX_HTML.contains(API_BASE));
}
#[test]
fn has_offline_queue_js() {
    assert!(INDEX_HTML.contains("OFFLINE"));
    assert!(INDEX_HTML.contains("queue"));
}
#[test]
fn has_tired_mode() {
    assert!(INDEX_HTML.contains("tired"));
    assert!(INDEX_HTML.contains("88px"));
}
