use crate::adapters::{
    AdapterError, CandidateSource, DiscoverInput, RemoteSnapshot, SnapshotEvidence, SourceAdapter,
};
use crate::error::AppResult;
use crate::models::SourceKind;
use crate::services::SharedRateLimiter;
use reqwest::blocking::Client;
use scraper::{Html, Selector};

pub struct StructuredPageAdapter {
    client: Client,
    rate_limiter: SharedRateLimiter,
}

impl StructuredPageAdapter {
    pub fn new(rate_limiter: SharedRateLimiter) -> Self {
        Self {
            client: Client::builder()
                .user_agent("SimSort/1.0")
                .build()
                .unwrap_or_default(),
            rate_limiter,
        }
    }

    fn check_rate_limit(&self, url: &str) -> AppResult<()> {
        if !self.rate_limiter.can_fetch(url) {
            return Err(AdapterError::RateLimited(url.to_string()).into());
        }
        Ok(())
    }

    fn record_request(&self, url: &str) {
        self.rate_limiter.record_fetch(url);
    }

    fn extract_from_tsr(&self, html: &Html) -> Option<StructuredData> {
        let title = self.extract_text(html, "h1.item-title, .item-name, [itemprop='name']");
        let version = self.extract_text(html, ".version, [itemprop='version'], .version-info");
        let download_url =
            self.extract_link(html, "a.download-btn, a[href*='download'], .download-link");
        let published = self.extract_text(html, "[itemprop='datePublished'], .publish-date, time");

        if title.is_some() {
            Some(StructuredData {
                title,
                version_text: version,
                download_url,
                published_at: published,
                release_id: None,
            })
        } else {
            None
        }
    }

    fn extract_text(&self, html: &Html, selector_str: &str) -> Option<String> {
        Selector::parse(selector_str)
            .ok()
            .and_then(|selector| html.select(&selector).next())
            .map(|el| el.text().collect::<String>().trim().to_string())
            .filter(|s| !s.is_empty())
    }

    fn extract_link(&self, html: &Html, selector_str: &str) -> Option<String> {
        Selector::parse(selector_str)
            .ok()
            .and_then(|selector| html.select(&selector).next())
            .and_then(|el| el.value().attr("href"))
            .map(String::from)
    }

    fn extract_version_from_text(&self, text: &str) -> Option<String> {
        let patterns = [
            r"v?(\d+\.\d+\.\d+(?:\.\d+)?)",
            r"version\s*(\d+\.\d+(?:\.\d+)?)",
            r"(\d+\.\d+\.\d+(?:\.\d+)?)\s*\(",
        ];

        for pattern in patterns {
            if let Ok(re) = regex::Regex::new(pattern) {
                if let Some(caps) = re.captures(text) {
                    return Some(caps.get(1).unwrap().as_str().to_string());
                }
            }
        }
        None
    }
}

struct StructuredData {
    title: Option<String>,
    version_text: Option<String>,
    download_url: Option<String>,
    published_at: Option<String>,
    release_id: Option<String>,
}

impl SourceAdapter for StructuredPageAdapter {
    fn kind(&self) -> SourceKind {
        SourceKind::StructuredPage
    }

    fn discover_candidates(&self, _input: &DiscoverInput) -> AppResult<Vec<CandidateSource>> {
        Ok(vec![])
    }

    fn refresh_snapshot(
        &self,
        binding: &crate::models::SourceBinding,
    ) -> AppResult<RemoteSnapshot> {
        let url = &binding.source_url;
        self.check_rate_limit(url)?;
        let response = self
            .client
            .get(&binding.source_url)
            .send()
            .map_err(AdapterError::Network)?;

        self.record_request(url);
        let etag = response
            .headers()
            .get("etag")
            .and_then(|v| v.to_str().ok())
            .map(String::from);

        let last_modified = response
            .headers()
            .get("last-modified")
            .and_then(|v| v.to_str().ok())
            .map(String::from);

        let body = response.text().map_err(AdapterError::Network)?;

        let html = Html::parse_document(&body);

        let data = self.extract_from_tsr(&html);

        let (title, version_text, download_url, published_at) = if let Some(d) = data {
            (d.title, d.version_text, d.download_url, d.published_at)
        } else {
            (
                self.extract_text(&html, "title, h1"),
                None,
                self.extract_link(&html, "a[href*='download'], a[href*='file']"),
                None,
            )
        };

        Ok(RemoteSnapshot {
            binding_id: binding.id.clone(),
            title,
            version_text,
            published_at,
            download_url,
            changelog_url: None,
            release_id: None,
            release_asset_names: vec![],
            image_hashes: vec![],
            etag,
            last_modified,
            evidence: SnapshotEvidence::default(),
            confidence: 0.70,
            raw: serde_json::json!({"source": "structured_page"}),
        })
    }
}

impl Default for StructuredPageAdapter {
    fn default() -> Self {
        Self::new(SharedRateLimiter::default())
    }
}

#[cfg(test)]
mod tests {
    use crate::adapters::structured_page::StructuredPageAdapter;
    use crate::adapters::SourceAdapter;
    use crate::models::{SourceBinding, SourceKind};
    use scraper::Html;

    const TSR_ITEM_PAGE_HTML: &str = r#"<!DOCTYPE html>
<html>
<head><title>Amazing CC Item - TSR</title></head>
<body>
    <h1 class="item-title">Amazing CC Item v1.2.3</h1>
    <div class="version">Version 1.2.3</div>
    <a class="download-btn" href="https://example.com/download/item123">Download</a>
    <span class="publish-date">2024-01-15</span>
</body>
</html>"#;

    #[test]
    fn test_parse_tsr_item_page() {
        let adapter = StructuredPageAdapter::new();
        let html = Html::parse_document(TSR_ITEM_PAGE_HTML);

        let title = adapter.extract_text(&html, "h1.item-title");
        assert_eq!(title.as_deref(), Some("Amazing CC Item v1.2.3"));

        let version = adapter.extract_text(&html, ".version");
        assert_eq!(version.as_deref(), Some("Version 1.2.3"));

        let download_url = adapter.extract_link(&html, "a.download-btn");
        assert_eq!(
            download_url.as_deref(),
            Some("https://example.com/download/item123")
        );
    }

    #[test]
    fn test_extract_version_from_text() {
        let adapter = StructuredPageAdapter::new();

        assert_eq!(
            adapter.extract_version_from_text("v1.2.3").as_deref(),
            Some("1.2.3")
        );
        assert_eq!(
            adapter
                .extract_version_from_text("Version 2.0.1")
                .as_deref(),
            Some("2.0.1")
        );
        assert_eq!(
            adapter
                .extract_version_from_text("Release 1.0.0 (Beta)")
                .as_deref(),
            Some("1.0.0")
        );
        assert_eq!(
            adapter
                .extract_version_from_text("No version here")
                .as_deref(),
            None
        );
    }

    #[test]
    fn test_extract_download_link() {
        let adapter = StructuredPageAdapter::new();
        let html = Html::parse_document(
            r#"<html><body>
            <a href="https://example.com/files/mod.zip">Download Mod</a>
            <a href="https://other.com/file.exe">Other</a>
        </body></html>"#,
        );

        let link = adapter.extract_link(&html, "a[href*='download'], a[href*='file']");
        assert!(link.is_some());
    }

    #[test]
    fn test_structured_adapter_kind() {
        let adapter = StructuredPageAdapter::new();
        assert_eq!(adapter.kind(), SourceKind::StructuredPage);
    }

    #[test]
    #[ignore] // Requires actual HTTP server
    fn test_refresh_snapshot_extracts_fields() {
        let adapter = StructuredPageAdapter::new();

        let binding = SourceBinding {
            id: "test-binding-struct".to_string(),
            local_mod_id: "test-mod-1".to_string(),
            source_kind: SourceKind::StructuredPage,
            source_url: "https://example.com/item.html".to_string(),
            provider_mod_id: None,
            provider_file_id: None,
            provider_repo: None,
            bind_method: "manual".to_string(),
            is_primary: true,
            created_at: "2025-01-01T00:00:00Z".to_string(),
            updated_at: "2025-01-01T00:00:00Z".to_string(),
        };

        let result = adapter.refresh_snapshot(&binding);
        assert!(result.is_ok(), "Should succeed: {:?}", result.err());

        let snapshot = result.expect("Should have result");
        assert_eq!(snapshot.binding_id, "test-binding-struct");
        assert!(
            snapshot.title.is_some()
                || snapshot.download_url.is_some()
                || snapshot.version_text.is_some()
        );
    }
}
