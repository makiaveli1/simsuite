use crate::adapters::{
    detect_access_tier, AdapterError, CandidateSource, DiscoverInput, RemoteSnapshot,
    SnapshotEvidence, SourceAdapter,
};
use crate::error::AppResult;
use crate::models::SourceKind;
use crate::services::SharedRateLimiter;
use regex::Regex;
use reqwest::blocking::Client;
use scraper::{Html, Selector};
use sha2::{Digest, Sha256};

pub struct GenericPageAdapter {
    client: Client,
    rate_limiter: SharedRateLimiter,
}

impl GenericPageAdapter {
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

    pub fn compute_content_hash(html_content: &str) -> String {
        let patterns = [
            r"<script[^>]*>[\s\S]*?</script>",
            r"<style[^>]*>[\s\S]*?</style>",
            r"<nav[^>]*>[\s\S]*?</nav>",
            r"<footer[^>]*>[\s\S]*?</footer>",
            r"<header[^>]*>[\s\S]*?</header>",
            r"<aside[^>]*>[\s\S]*?</aside>",
        ];

        let mut content = html_content.to_string();
        for pattern in patterns {
            if let Ok(re) = Regex::new(pattern) {
                content = re.replace_all(&content, "").to_string();
            }
        }

        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    pub fn extract_title(html: &Html) -> Option<String> {
        Selector::parse("meta[property='og:title']")
            .ok()
            .and_then(|s| html.select(&s).next())
            .and_then(|el| el.value().attr("content"))
            .map(String::from)
            .or_else(|| {
                Selector::parse("title")
                    .ok()
                    .and_then(|s| html.select(&s).next())
                    .map(|el| el.text().collect::<String>().trim().to_string())
            })
            .or_else(|| {
                Selector::parse("h1")
                    .ok()
                    .and_then(|s| html.select(&s).next())
                    .map(|el| el.text().collect::<String>().trim().to_string())
            })
            .filter(|s| !s.is_empty())
    }
}

impl SourceAdapter for GenericPageAdapter {
    fn kind(&self) -> SourceKind {
        SourceKind::GenericPage
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
        let request = self.client.get(&binding.source_url);

        let response = request.send().map_err(AdapterError::Network)?;

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

        let title = Self::extract_title(&html);
        let content_hash = Self::compute_content_hash(&body);

        Ok(RemoteSnapshot {
            binding_id: binding.id.clone(),
            title,
            version_text: None,
            published_at: None,
            download_url: None,
            changelog_url: None,
            release_id: Some(content_hash),
            release_asset_names: vec![],
            image_hashes: vec![],
            etag,
            last_modified,
            evidence: SnapshotEvidence::default(),
            confidence: 0.50,
            raw: serde_json::json!({"source": "generic_page"}),
            access_tier: detect_access_tier(&binding.source_url),
            patron_free_version: None,
            file_fingerprints_json: None,
        })
    }
}

impl Default for GenericPageAdapter {
    fn default() -> Self {
        Self::new(SharedRateLimiter::default())
    }
}
