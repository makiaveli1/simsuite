use crate::adapters::{
    AdapterError, CandidateSource, DiscoverInput, RemoteSnapshot, SnapshotEvidence, SourceAdapter,
};
use crate::error::AppResult;
use crate::models::{AccessTier, SourceBinding, SourceKind};
use crate::services::SharedRateLimiter;
use reqwest::blocking::Client;
use std::time::Duration;

const EA_BROKEN_MODS_INDEX: &str = 
    "https://forums.thesims.com/en_US/discussions/the-sims-4-mods-and-custom-content-en/broken-and-updated-sims-4-mods-and-cc";

const FALLBACK_URLS: &[(&str, &str)] = &[
    ("1.122", "https://forums.thesims.com/en_US/discussions/the-sims-4-mods-and-custom-content-en/broken-and-updated-sims-4-mods-and-cc-patch-1-122-march-17-2026"),
    ("1.121", "https://forums.thesims.com/en_US/discussions/the-sims-4-mods-and-custom-content-en/broken-and-updated-sims-4-mods-and-cc-patch-1-121"),
];

pub struct EABrokenModsAdapter {
    client: Client,
    rate_limiter: SharedRateLimiter,
}

impl EABrokenModsAdapter {
    pub fn new(rate_limiter: SharedRateLimiter) -> Self {
        let client = Client::builder()
            .redirect(reqwest::redirect::Policy::limited(6))
            .timeout(Duration::from_secs(30))
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
            .build()
            .expect("EA forums client");
        EABrokenModsAdapter {
            client,
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

    pub fn fetch_index_page(&self) -> AppResult<String> {
        self.check_rate_limit(EA_BROKEN_MODS_INDEX)?;

        let response = self
            .client
            .get(EA_BROKEN_MODS_INDEX)
            .header("Accept", "text/html")
            .send()?
            .error_for_status()
            .map_err(|e| AdapterError::Network(e))?;

        self.record_request(EA_BROKEN_MODS_INDEX);

        let body = response.text()?;
        if body.len() < 1000 {
            return self.fetch_with_fallback();
        }

        Ok(body)
    }

    fn fetch_with_fallback(&self) -> AppResult<String> {
        for (_, url) in FALLBACK_URLS {
            self.check_rate_limit(url)?;

            match self.client.get(*url).send() {
                Ok(response) => {
                    if response.status().is_success() {
                        self.record_request(url);
                        if let Ok(body) = response.text() {
                            if body.len() > 1000 {
                                return Ok(body);
                            }
                        }
                    }
                }
                Err(_) => continue,
            }
        }

        Err(AdapterError::Api("All EA forum fetch attempts failed".to_string()).into())
    }

    pub fn parse_thread_links(&self, html: &str) -> Vec<(String, String)> {
        let pattern = regex::Regex::new(
            r#"href="([^"]*broken-and-updated-sims-4-mods-and-cc-patch-(\d+)-(\d+)-(\d+)[^"]*)"#,
        )
        .unwrap();

        let mut threads = Vec::new();
        for cap in pattern.captures_iter(html) {
            let url = cap.get(1).unwrap().as_str().to_string();
            let major = cap.get(2).unwrap().as_str();
            let minor = cap.get(3).unwrap().as_str();
            let patch = cap.get(4).unwrap().as_str();

            if url.contains("/patch-") {
                let patch_version = format!("{}.{}.{}", major, minor, patch);
                threads.push((patch_version, url));
            }
        }

        threads
    }

    pub fn parse_mod_names_from_thread(&self, html: &str) -> Vec<String> {
        let mut mod_names = Vec::new();

        let bold_pattern = regex::Regex::new(r"<strong>([^<]{3,100})</strong>").unwrap();
        for cap in bold_pattern.captures_iter(html) {
            let name = cap.get(1).unwrap().as_str().trim().to_string();
            if self.is_likely_mod_name(&name) {
                mod_names.push(name);
            }
        }

        let td_pattern = regex::Regex::new(r"<td[^>]*>([^<]{3,100})</td>").unwrap();
        for cap in td_pattern.captures_iter(html) {
            let name = cap.get(1).unwrap().as_str().trim().to_string();
            if self.is_likely_mod_name(&name) {
                mod_names.push(name);
            }
        }

        mod_names
    }

    fn is_likely_mod_name(&self, name: &str) -> bool {
        let lower = name.to_lowercase();

        if lower.contains("broken") && lower.contains("patch") {
            return false;
        }
        if lower.contains("click here") {
            return false;
        }
        if lower.contains("read more") {
            return false;
        }
        if regex::Regex::new(r"^\d+\.\d+$").unwrap().is_match(name) {
            return false;
        }

        name.len() >= 3 && name.len() <= 100
    }
}

impl SourceAdapter for EABrokenModsAdapter {
    fn kind(&self) -> SourceKind {
        SourceKind::EaBrokenMods
    }

    fn discover_candidates(&self, _input: &DiscoverInput) -> AppResult<Vec<CandidateSource>> {
        Ok(vec![])
    }

    fn refresh_snapshot(&self, _binding: &SourceBinding) -> AppResult<RemoteSnapshot> {
        let html = self.fetch_index_page()?;

        let mut all_mods = Vec::new();

        let threads = self.parse_thread_links(&html);
        for (_, url) in &threads {
            if url != EA_BROKEN_MODS_INDEX {
                if let Ok(thread_html) = self.client.get(url).send() {
                    if let Ok(body) = thread_html.text() {
                        all_mods.extend(self.parse_mod_names_from_thread(&body));
                    }
                }
            }
        }

        all_mods.extend(self.parse_mod_names_from_thread(&html));

        all_mods.sort();
        all_mods.dedup();

        let mod_count = all_mods.len();

        Ok(RemoteSnapshot {
            binding_id: _binding.id.clone(),
            title: Some(format!("EA Broken Mods List ({} mods)", mod_count)),
            version_text: None,
            published_at: None,
            download_url: None,
            changelog_url: Some(EA_BROKEN_MODS_INDEX.to_string()),
            release_id: None,
            release_asset_names: all_mods,
            image_hashes: Vec::new(),
            etag: None,
            last_modified: None,
            evidence: SnapshotEvidence::default(),
            confidence: 0.95,
            access_tier: AccessTier::Public,
            patron_free_version: None,
            raw: serde_json::json!({ "mod_count": mod_count }),
        })
    }
}

impl Default for EABrokenModsAdapter {
    fn default() -> Self {
        Self::new(SharedRateLimiter::default())
    }
}
