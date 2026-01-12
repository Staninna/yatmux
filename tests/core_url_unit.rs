use yatmux::core::url::{UrlManager, UrlSpan, detect_urls};

#[test]
fn test_detect_urls_https() {
    let urls = detect_urls("Check out https://example.com for more info");
    assert_eq!(urls.len(), 1);
    assert_eq!(urls[0].url, "https://example.com");
    assert_eq!(urls[0].start_col, 10);
}

#[test]
fn test_detect_urls_http() {
    let urls = detect_urls("Visit http://test.org/path?query=1");
    assert_eq!(urls.len(), 1);
    assert_eq!(urls[0].url, "http://test.org/path?query=1");
}

#[test]
fn test_detect_urls_www() {
    let urls = detect_urls("Go to www.example.com/page");
    assert_eq!(urls.len(), 1);
    assert_eq!(urls[0].url, "www.example.com/page");
    assert_eq!(urls[0].full_url(), "https://www.example.com/page");
}

#[test]
fn test_detect_urls_multiple() {
    let urls = detect_urls("See https://a.com and https://b.com");
    assert_eq!(urls.len(), 2);
    assert_eq!(urls[0].url, "https://a.com");
    assert_eq!(urls[1].url, "https://b.com");
}

#[test]
fn test_detect_urls_trailing_punctuation() {
    let urls = detect_urls("Visit https://example.com.");
    assert_eq!(urls.len(), 1);
    assert_eq!(urls[0].url, "https://example.com");
}

#[test]
fn test_detect_urls_none() {
    let urls = detect_urls("No URLs here, just text");
    assert!(urls.is_empty());
}

#[test]
fn test_url_span_contains() {
    let span = UrlSpan::new(5, 20, "https://test.com".to_string());
    assert!(!span.contains(4));
    assert!(span.contains(5));
    assert!(span.contains(19));
    assert!(!span.contains(20));
}

#[test]
fn test_url_manager_hover() {
    let mut mgr = UrlManager::new();
    mgr.set_dimensions(24);
    mgr.update_row(0, "Visit https://example.com today");

    assert!(!mgr.is_hovered(0, 6));
    mgr.update_hover(0, 10);
    assert!(mgr.is_hovered(0, 10));
    assert!(mgr.hovered_url().is_some());

    mgr.clear_hover();
    assert!(mgr.hovered_url().is_none());
}
