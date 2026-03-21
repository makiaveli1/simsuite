use std::time::Duration;

use reqwest::blocking::Client;
use serde::Deserialize;

use crate::adapters::{
    AdapterError, CandidateSource, DiscoverInput, FileInfo, RemoteSnapshot, SnapshotEvidence,
    SourceAdapter,
};
use crate::error::AppResult;
use crate::models::{AccessTier, SourceBinding, SourceKind};
use crate::services::SharedRateLimiter;

const BASE_URL: &str = "https://api.nexusmods.com";
const GAME_ID: &str = "1341";

pub struct NexusAdapter {
    client: Client,
    rate_limiter: SharedRateLimiter,
}

#[derive(Debug, Deserialize)]
struct NxModDetails {
    mod_id: i64,
    name: String,
    summary: Option<String>,
    description: Option<String>,
    url: String,
    updated: String,
    created: String,
}

#[derive(Debug, Deserialize)]
struct NxModFiles {
    mod_id: i64,
    files: Vec<NxFile>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct NxFile {
    file_id: i64,
    name: String,
    version: String,
    uploaded_at: String,
    file_name: String,
    download_url: Option<String>,
    size: i64,
}

#[derive(Debug, Deserialize)]
struct NxSearchResponse {
    mods: Vec<NxModInfo>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct NxModInfo {
    mod_id: i64,
    name: String,
    summary: Option<String>,
}

impl NexusAdapter {
    pub fn new(rate_limiter: SharedRateLimiter) -> Self {
        let client = Client::builder()
            .redirect(reqwest::redirect::Policy::limited(6))
            .timeout(Duration::from_secs(15))
            .user_agent("SimSuite/1.0")
            .build()
            .expect("Nexus client");
        NexusAdapter {
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

    fn search_mods(&self, query: &str) -> AppResult<Vec<NxModInfo>> {
        let url = format!("{}/v1/games/{}/mods/search.json", BASE_URL, GAME_ID);
        self.check_rate_limit(&url)?;
        let response = self
            .client
            .get(&url)
            .query(&[("name", query)])
            .header("Accept", "application/json")
            .send()?
            .error_for_status()
            .map_err(|e| AdapterError::Api(e.to_string()))?;

        self.record_request(&url);
        let body = response.text()?;
        let result: NxSearchResponse =
            serde_json::from_str(&body).map_err(|e| AdapterError::Parse(e.to_string()))?;
        Ok(result.mods)
    }

    fn get_mod_details(&self, mod_id: i64) -> AppResult<NxModDetails> {
        let url = format!("{}/v1/games/{}/mods/{}.json", BASE_URL, GAME_ID, mod_id);
        self.check_rate_limit(&url)?;
        let response = self
            .client
            .get(&url)
            .header("Accept", "application/json")
            .send()?
            .error_for_status()
            .map_err(|e| AdapterError::Api(e.to_string()))?;

        self.record_request(&url);
        let body = response.text()?;
        let result: NxModDetails =
            serde_json::from_str(&body).map_err(|e| AdapterError::Parse(e.to_string()))?;
        Ok(result)
    }

    fn get_mod_files(&self, mod_id: i64) -> AppResult<Vec<NxFile>> {
        let url = format!(
            "{}/v1/games/{}/mods/{}/files.json",
            BASE_URL, GAME_ID, mod_id
        );
        self.check_rate_limit(&url)?;
        let response = self
            .client
            .get(&url)
            .header("Accept", "application/json")
            .send()?
            .error_for_status()
            .map_err(|e| AdapterError::Api(e.to_string()))?;

        self.record_request(&url);
        #[derive(Deserialize)]
        struct NxFilesWrapper {
            files: Vec<NxFile>,
        }
        let body = response.text()?;
        let result: NxFilesWrapper =
            serde_json::from_str(&body).map_err(|e| AdapterError::Parse(e.to_string()))?;
        Ok(result.files)
    }

    fn calculate_confidence(
        &self,
        mod_name: &str,
        nx_mod: &NxModInfo,
        _files: &[FileInfo],
    ) -> (f64, Vec<String>) {
        let mut reasoning = Vec::new();
        let mut score: f64 = 0.5;

        let mod_lower = mod_name.to_lowercase();
        let nx_lower = nx_mod.name.to_lowercase();

        if nx_lower == mod_lower {
            score += 0.35;
            reasoning.push("Exact name match".to_string());
        } else if nx_lower.contains(&mod_lower) || mod_lower.contains(&nx_lower) {
            score += 0.15;
            reasoning.push("Partial name match".to_string());
        }

        if let Some(summary) = &nx_mod.summary {
            let summary_lower = summary.to_lowercase();
            if summary_lower.contains("sims 4") || summary_lower.contains("sims4") {
                score += 0.1;
                reasoning.push("Sims 4 related summary".to_string());
            }
        }

        let final_score = score.min(1.0);
        (final_score, reasoning)
    }
}

impl SourceAdapter for NexusAdapter {
    fn kind(&self) -> SourceKind {
        SourceKind::Nexus
    }

    fn discover_candidates(&self, input: &DiscoverInput) -> AppResult<Vec<CandidateSource>> {
        let search_results = self.search_mods(&input.normalized_name)?;

        let candidates: Vec<CandidateSource> = search_results
            .into_iter()
            .map(|m| {
                let (confidence, reasoning) =
                    self.calculate_confidence(&input.normalized_name, &m, &input.files);

                CandidateSource {
                    source_kind: SourceKind::Nexus,
                    source_url: format!("https://www.nexusmods.com/sims4/mods/{}", m.mod_id),
                    provider_mod_id: Some(m.mod_id.to_string()),
                    provider_file_id: None,
                    provider_repo: None,
                    confidence_score: confidence,
                    reasoning,
                    access_tier: AccessTier::Public,
                    patron_free_version: None,
                }
            })
            .collect();

        Ok(candidates)
    }

    fn refresh_snapshot(&self, binding: &SourceBinding) -> AppResult<RemoteSnapshot> {
        let mod_id: i64 = binding
            .provider_mod_id
            .as_ref()
            .and_then(|s| s.parse().ok())
            .ok_or_else(|| AdapterError::Api("Missing or invalid provider_mod_id".to_string()))?;

        let mod_details = self.get_mod_details(mod_id)?;
        let files = self.get_mod_files(mod_id)?;

        let latest_file = files.first();
        let release_id = latest_file.map(|f| f.file_id.to_string());
        let version_text = latest_file.map(|f| f.version.clone());
        let published_at = latest_file.map(|f| f.uploaded_at.clone());
        let download_url = latest_file.and_then(|f| f.download_url.clone());
        let release_asset_names: Vec<String> = files.iter().map(|f| f.file_name.clone()).collect();

        let mod_name = mod_details.name.clone();
        Ok(RemoteSnapshot {
            binding_id: binding.id.clone(),
            title: Some(mod_details.name),
            version_text,
            published_at,
            download_url,
            changelog_url: None,
            release_id,
            release_asset_names,
            image_hashes: Vec::new(),
            etag: None,
            last_modified: Some(mod_details.updated),
            evidence: SnapshotEvidence::default(),
            confidence: 0.85,
            raw: serde_json::json!({
                "mod_id": mod_id,
                "name": mod_name,
            }),
            access_tier: AccessTier::Public,
            patron_free_version: None,
            file_fingerprints_json: None,
        })
    }
}

impl Default for NexusAdapter {
    fn default() -> Self {
        Self::new(SharedRateLimiter::default())
    }
}
