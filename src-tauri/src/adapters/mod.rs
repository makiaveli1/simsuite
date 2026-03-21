pub mod curseforge;
pub mod ea_broken_mods;
pub mod errors;
pub mod feed;
pub mod generic_page;
pub mod github;
pub mod nexus;
pub mod structured_page;

#[cfg(test)]
mod curseforge_test;
#[cfg(test)]
mod generic_page_test;

pub use errors::AdapterError;
pub use feed::FeedAdapter;

use crate::error::AppResult;
use crate::models::{AccessTier, AppBehaviorSettings, SourceBinding, SourceKind, UpdateStatus};
use crate::services::SharedRateLimiter;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use url::Url;

const BLOCKED_HEADERS: &[&str] = &[
    "authorization",
    "proxy-authorization",
    "cookie",
    "host",
    "content-type",
    "content-length",
    "transfer-encoding",
    "connection",
    "date",
    "upgrade",
    "sec-",
    "proxy-",
];

fn is_header_allowed(name: &str) -> bool {
    let lower = name.to_lowercase();
    !BLOCKED_HEADERS
        .iter()
        .any(|blocked| lower.starts_with(blocked) || lower == *blocked)
}

pub fn apply_custom_headers(
    request: reqwest::blocking::RequestBuilder,
    binding: &SourceBinding,
) -> reqwest::blocking::RequestBuilder {
    if let Some(headers_json) = &binding.custom_headers_json {
        if let Ok(headers) = serde_json::from_str::<HashMap<String, String>>(headers_json) {
            let mut req = request;
            for (key, value) in headers {
                if is_header_allowed(&key) {
                    req = req.header(&key, &value);
                } else {
                    tracing::warn!("Blocked sensitive header: {}", key);
                }
            }
            return req;
        }
    }
    request
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct DiscoverInput {
    pub local_mod_id: String,
    pub display_name: String,
    pub normalized_name: String,
    pub creator_name: Option<String>,
    pub category: Option<String>,
    pub files: Vec<FileInfo>,
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct FileInfo {
    pub file_name: String,
    pub sha256: Option<String>,
    pub size: i64,
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct CandidateSource {
    pub source_kind: SourceKind,
    pub source_url: String,
    pub provider_mod_id: Option<String>,
    pub provider_file_id: Option<String>,
    pub provider_repo: Option<String>,
    pub confidence_score: f64,
    pub reasoning: Vec<String>,
    pub access_tier: AccessTier,
    pub patron_free_version: Option<String>,
}

#[derive(Debug)]
pub struct RemoteSnapshot {
    pub binding_id: String,
    pub title: Option<String>,
    pub version_text: Option<String>,
    pub published_at: Option<String>,
    pub download_url: Option<String>,
    pub changelog_url: Option<String>,
    pub release_id: Option<String>,
    pub release_asset_names: Vec<String>,
    pub image_hashes: Vec<String>,
    pub etag: Option<String>,
    pub last_modified: Option<String>,
    pub evidence: SnapshotEvidence,
    pub confidence: f64,
    pub raw: serde_json::Value,
    pub access_tier: AccessTier,
    pub patron_free_version: Option<String>,
    pub file_fingerprints_json: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SnapshotEvidence {
    pub version_changed: bool,
    pub download_changed: bool,
    pub title_changed: bool,
    pub asset_list_changed: bool,
    pub feed_guid_changed: bool,
}

const PATRON_DOMAINS: &[(&str, &[&str])] = &[
    ("patreon.com", &["posts", "community"]),
    ("ko-fi.com", &[]),
    ("kofi.com", &[]),
    ("buymeacoffee.com", &[]),
    ("gumroad.com", &[]),
    ("boosty.to", &[]),
    ("liberapay.com", &[]),
];

pub fn detect_access_tier(url: &str) -> AccessTier {
    let parsed = match Url::parse(url) {
        Ok(p) => p,
        Err(_) => return AccessTier::Public,
    };

    let host = match parsed.host_str() {
        Some(h) => h.to_lowercase(),
        None => return AccessTier::Public,
    };

    let path = parsed.path();

    for (domain, early_access_paths) in PATRON_DOMAINS {
        if host == *domain || host.ends_with(&format!(".{}", domain)) {
            if early_access_paths.is_empty() {
                return AccessTier::PatronOnly;
            }
            if early_access_paths.iter().any(|p| path.contains(p)) {
                return AccessTier::EarlyAccess;
            }
            return AccessTier::PatronOnly;
        }
    }

    AccessTier::Public
}

impl Default for RemoteSnapshot {
    fn default() -> Self {
        Self {
            binding_id: String::new(),
            title: None,
            version_text: None,
            published_at: None,
            download_url: None,
            changelog_url: None,
            release_id: None,
            release_asset_names: Vec::new(),
            image_hashes: Vec::new(),
            etag: None,
            last_modified: None,
            evidence: SnapshotEvidence::default(),
            confidence: 0.0,
            raw: serde_json::Value::Null,
            access_tier: AccessTier::Public,
            patron_free_version: None,
            file_fingerprints_json: None,
        }
    }
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct UpdateDecision {
    pub status: UpdateStatus,
    pub confidence: f64,
    pub summary: Option<String>,
}

pub trait SourceAdapter: Send + Sync {
    fn kind(&self) -> SourceKind;
    fn discover_candidates(&self, input: &DiscoverInput) -> AppResult<Vec<CandidateSource>>;
    fn refresh_snapshot(&self, binding: &SourceBinding) -> AppResult<RemoteSnapshot>;
}

pub struct AdapterRegistry {
    adapters: Vec<Box<dyn SourceAdapter>>,
}

impl std::fmt::Debug for AdapterRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AdapterRegistry")
            .field("adapter_count", &self.adapters.len())
            .finish()
    }
}

impl AdapterRegistry {
    pub fn new(settings: &AppBehaviorSettings, rate_limiter: SharedRateLimiter) -> Self {
        let adapters: Vec<Box<dyn SourceAdapter>> = vec![
            Box::new(curseforge::CurseForgeAdapter::new(
                settings.curseforge_api_key.clone(),
                rate_limiter.clone(),
            )),
            Box::new(feed::FeedAdapter::new(rate_limiter.clone())),
            Box::new(generic_page::GenericPageAdapter::new(rate_limiter.clone())),
            Box::new(github::GitHubAdapter::new(rate_limiter.clone())),
            Box::new(nexus::NexusAdapter::new(rate_limiter.clone())),
            Box::new(structured_page::StructuredPageAdapter::new(
                rate_limiter.clone(),
            )),
            Box::new(ea_broken_mods::EABrokenModsAdapter::new(
                rate_limiter.clone(),
            )),
        ];
        AdapterRegistry { adapters }
    }

    pub fn for_kind(&self, kind: SourceKind) -> Option<&dyn SourceAdapter> {
        self.adapters
            .iter()
            .find(|a| a.kind() == kind)
            .map(|a| a.as_ref())
    }

    pub fn discover_all(&self, input: &DiscoverInput) -> AppResult<Vec<CandidateSource>> {
        let mut results = Vec::new();
        for adapter in &self.adapters {
            if let Ok(candidates) = adapter.discover_candidates(input) {
                results.extend(candidates);
            }
        }
        results.sort_by(|a, b| {
            b.confidence_score
                .partial_cmp(&a.confidence_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        Ok(results)
    }
}

impl Default for AdapterRegistry {
    fn default() -> Self {
        Self::new(
            &AppBehaviorSettings {
                keep_running_in_background: false,
                automatic_watch_checks: false,
                watch_check_interval_hours: 12,
                last_watch_check_at: None,
                last_watch_check_error: None,
                curseforge_api_key: None,
                github_api_token: None,
            },
            SharedRateLimiter::default(),
        )
    }
}
