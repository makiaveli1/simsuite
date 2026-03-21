pub mod curseforge;
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
use crate::models::{AppBehaviorSettings, SourceBinding, SourceKind, UpdateStatus};
use crate::services::SharedRateLimiter;
use serde::{Deserialize, Serialize};

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
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SnapshotEvidence {
    pub version_changed: bool,
    pub download_changed: bool,
    pub title_changed: bool,
    pub asset_list_changed: bool,
    pub feed_guid_changed: bool,
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
