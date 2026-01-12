//! URL detection for the terminal.
//!
//! This module provides URL detection in terminal text and tracking
//! of URL spans for highlighting and click handling.

use regex::Regex;
use std::sync::LazyLock;

/// Regex pattern for URL detection.
/// Matches http://, https://, and common URL patterns.
static URL_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?i)(https?://[^\s<>\[\](){}"']+|www\.[^\s<>\[\](){}"']+\.[a-z]{2,}[^\s<>\[\](){}"']*)"#,
    )
    .expect("URL regex should compile")
});

/// A detected URL span within a row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UrlSpan {
    /// Starting column (inclusive).
    pub start_col: usize,
    /// Ending column (exclusive).
    pub end_col: usize,
    /// The URL text.
    pub url: String,
}

impl UrlSpan {
    /// Creates a new URL span.
    pub fn new(start_col: usize, end_col: usize, url: String) -> Self {
        UrlSpan {
            start_col,
            end_col,
            url,
        }
    }

    /// Checks if the given column is within this URL span.
    pub fn contains(&self, col: usize) -> bool {
        col >= self.start_col && col < self.end_col
    }

    /// Returns the URL, adding https:// prefix if needed.
    pub fn full_url(&self) -> String {
        if self.url.starts_with("http://") || self.url.starts_with("https://") {
            self.url.clone()
        } else {
            format!("https://{}", self.url)
        }
    }
}

/// Detects URLs in a line of text.
pub fn detect_urls(text: &str) -> Vec<UrlSpan> {
    URL_REGEX
        .find_iter(text)
        .map(|m| {
            let url = m.as_str().to_string();
            // Trim trailing punctuation that's likely not part of the URL
            let trimmed = url.trim_end_matches(|c| matches!(c, '.' | ',' | ';' | ':' | '!' | '?'));
            let trimmed_len = url.len() - trimmed.len();
            UrlSpan::new(m.start(), m.end() - trimmed_len, trimmed.to_string())
        })
        .collect()
}

/// Manages URL detection and hover state.
#[derive(Default)]
pub struct UrlManager {
    /// Detected URLs per row (row index -> URL spans).
    urls: Vec<Vec<UrlSpan>>,
    /// Currently hovered URL (row, URL span).
    hovered: Option<(usize, UrlSpan)>,
    view_rows: usize,
}

impl UrlManager {
    pub fn new() -> Self {
        UrlManager::default()
    }

    /// Updates the view dimensions.
    pub fn set_dimensions(&mut self, rows: usize) {
        if self.view_rows != rows {
            self.view_rows = rows;
            self.urls.clear();
            self.urls.resize(rows, Vec::new());
            self.hovered = None;
        }
    }

    /// Updates URLs for a specific row.
    /// Combines regex-detected URLs with OSC 8 hyperlinks (hyperlinks take priority).
    pub fn update_row(&mut self, row: usize, text: &str) {
        self.update_row_with_hyperlinks(row, text, &[]);
    }

    /// Updates URLs for a specific row, including OSC 8 hyperlinks.
    /// OSC 8 hyperlinks take priority over regex-detected URLs.
    pub fn update_row_with_hyperlinks(
        &mut self,
        row: usize,
        text: &str,
        hyperlinks: &[Option<String>],
    ) {
        if row >= self.urls.len() {
            return;
        }

        // First, collect OSC 8 hyperlink spans
        let mut osc8_spans: Vec<UrlSpan> = Vec::new();
        let mut i = 0;
        while i < hyperlinks.len() {
            if let Some(ref url) = hyperlinks[i] {
                let start = i;
                // Find the end of this hyperlink span
                while i < hyperlinks.len() && hyperlinks[i].as_ref() == Some(url) {
                    i += 1;
                }
                osc8_spans.push(UrlSpan::new(start, i, url.clone()));
            } else {
                i += 1;
            }
        }

        // Get regex-detected URLs
        let regex_spans = detect_urls(text);

        // Merge: OSC 8 hyperlinks take priority
        // Filter out regex spans that overlap with OSC 8 spans
        let mut merged: Vec<UrlSpan> = osc8_spans.clone();
        for regex_span in regex_spans {
            let overlaps = osc8_spans.iter().any(|osc| {
                !(regex_span.end_col <= osc.start_col || regex_span.start_col >= osc.end_col)
            });
            if !overlaps {
                merged.push(regex_span);
            }
        }

        // Sort by start column
        merged.sort_by_key(|s| s.start_col);

        self.urls[row] = merged;
    }

    /// Updates the hover state based on cursor position.
    pub fn update_hover(&mut self, row: usize, col: usize) {
        self.hovered = None;
        if row < self.urls.len() {
            for span in &self.urls[row] {
                if span.contains(col) {
                    self.hovered = Some((row, span.clone()));
                    break;
                }
            }
        }
    }

    /// Clears the hover state.
    pub fn clear_hover(&mut self) {
        self.hovered = None;
    }

    /// Returns the currently hovered URL if any.
    pub fn hovered_url(&self) -> Option<&UrlSpan> {
        self.hovered.as_ref().map(|(_, span)| span)
    }

    /// Checks if a cell is part of a URL.
    pub fn is_url(&self, row: usize, col: usize) -> bool {
        if row < self.urls.len() {
            self.urls[row].iter().any(|span| span.contains(col))
        } else {
            false
        }
    }

    /// Checks if a cell is part of the hovered URL.
    pub fn is_hovered(&self, row: usize, col: usize) -> bool {
        if let Some((hover_row, ref span)) = self.hovered {
            hover_row == row && span.contains(col)
        } else {
            false
        }
    }

    /// Returns the URL at the given position if any.
    pub fn url_at(&self, row: usize, col: usize) -> Option<&UrlSpan> {
        if row < self.urls.len() {
            self.urls[row].iter().find(|span| span.contains(col))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
