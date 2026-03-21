use std::time::Duration;

use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};

use crate::adapters::{
    AdapterError, CandidateSource, DiscoverInput, FileInfo, RemoteSnapshot, SnapshotEvidence,
    SourceAdapter,
};
use crate::error::AppResult;
use crate::models::{SourceBinding, SourceKind};

const BASE_URL: &str = "https://api.curseforge.com";
const GAME_ID: i64 = 432;

pub struct CurseForgeAdapter {
    client: Client,
    api_key: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CfSearchResponse {
    data: Vec<CfMod>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CfMod {
    id: i64,
    name: String,
    slug: String,
    links: Option<CfLinks>,
    categories: Option<Vec<CfCategory>>,
    latest_files: Option<Vec<CfFile>>,
}

#[derive(Debug, Deserialize)]
struct CfLinks {
    website_url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CfCategory {
    id: i64,
    name: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CfFile {
    id: i64,
    file_name: String,
    display_name: Option<String>,
    version: Option<String>,
    release_date: Option<String>,
    download_url: Option<String>,
    files: Option<Vec<CfFileDetail>>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CfFileDetail {
    id: i64,
    filename: String,
    file_size: i64,
}

#[derive(Debug, Serialize)]
struct CfSearchRequest {
    game_id: i64,
    search_filter: String,
    #[serde(rename = "sortField")]
    sort_field: i32,
    #[serde(rename = "sortOrder")]
    sort_order: String,
}

impl CurseForgeAdapter {
    pub fn new(api_key: Option<String>) -> Self {
        let client = Client::builder()
            .redirect(reqwest::redirect::Policy::limited(6))
            .timeout(Duration::from_secs(15))
            .user_agent("SimSuite/1.0")
            .build()
            .expect("CurseForge client");
        CurseForgeAdapter { client, api_key }
    }

    fn add_auth_headers(
        &self,
        request: reqwest::blocking::RequestBuilder,
    ) -> reqwest::blocking::RequestBuilder {
        if let Some(key) = &self.api_key {
            request.header("x-api-key", key)
        } else {
            request
        }
    }

    fn search_mods(&self, query: &str) -> AppResult<Vec<CfMod>> {
        let url = format!("{}/v1/mods/search", BASE_URL);
        let response = self
            .add_auth_headers(self.client.get(&url))
            .query(&[
                ("gameId", GAME_ID.to_string()),
                ("searchFilter", query.to_string()),
                ("sortField", 1.to_string()),
                ("sortOrder", "desc".to_string()),
            ])
            .send()?
            .error_for_status()
            .map_err(|e| AdapterError::Api(e.to_string()))?;

        let body = response.text()?;
        let result: CfSearchResponse =
            serde_json::from_str(&body).map_err(|e| AdapterError::Parse(e.to_string()))?;
        Ok(result.data)
    }

    fn get_mod(&self, mod_id: i64) -> AppResult<CfMod> {
        let url = format!("{}/v1/mods/{}", BASE_URL, mod_id);
        let response = self
            .add_auth_headers(self.client.get(&url))
            .send()?
            .error_for_status()
            .map_err(|e| AdapterError::Api(e.to_string()))?;

        #[derive(Deserialize)]
        struct CfModResponse {
            data: CfMod,
        }
        let body = response.text()?;
        let result: CfModResponse =
            serde_json::from_str(&body).map_err(|e| AdapterError::Parse(e.to_string()))?;
        Ok(result.data)
    }

    fn get_latest_files(&self, mod_id: i64) -> AppResult<Vec<CfFile>> {
        let url = format!("{}/v1/mods/{}/files", BASE_URL, mod_id);
        let response = self
            .add_auth_headers(self.client.get(&url))
            .query(&[("pageSize", "5")])
            .send()?
            .error_for_status()
            .map_err(|e| AdapterError::Api(e.to_string()))?;

        #[derive(Deserialize)]
        struct CfFilesResponse {
            data: Vec<CfFile>,
        }
        let body = response.text()?;
        let result: CfFilesResponse =
            serde_json::from_str(&body).map_err(|e| AdapterError::Parse(e.to_string()))?;
        Ok(result.data)
    }

    fn calculate_confidence(
        &self,
        mod_name: &str,
        cf_mod: &CfMod,
        _files: &[FileInfo],
    ) -> (f64, Vec<String>) {
        let mut reasoning = Vec::new();
        let mut score: f64 = 0.5;

        let mod_lower = mod_name.to_lowercase();
        let cf_lower = cf_mod.name.to_lowercase();

        if cf_lower == mod_lower {
            score += 0.4;
            reasoning.push("Exact name match".to_string());
        } else if cf_lower.contains(&mod_lower) || mod_lower.contains(&cf_lower) {
            score += 0.2;
            reasoning.push("Partial name match".to_string());
        }

        if let Some(links) = &cf_mod.links {
            if let Some(url) = &links.website_url {
                reasoning.push(format!("Has website: {}", url));
            }
        }

        if cf_mod
            .latest_files
            .as_ref()
            .map(|f| !f.is_empty())
            .unwrap_or(false)
        {
            score += 0.1;
            reasoning.push("Has recent files".to_string());
        }

        let final_score = score.min(1.0);
        (final_score, reasoning)
    }
}

impl SourceAdapter for CurseForgeAdapter {
    fn kind(&self) -> SourceKind {
        SourceKind::CurseForge
    }

    fn discover_candidates(&self, input: &DiscoverInput) -> AppResult<Vec<CandidateSource>> {
        let search_results = self.search_mods(&input.normalized_name)?;

        let candidates: Vec<CandidateSource> = search_results
            .into_iter()
            .filter(|m| {
                m.latest_files
                    .as_ref()
                    .map(|f| !f.is_empty())
                    .unwrap_or(false)
            })
            .map(|m| {
                let (confidence, reasoning) =
                    self.calculate_confidence(&input.normalized_name, &m, &input.files);
                let website_url = m
                    .links
                    .as_ref()
                    .and_then(|l| l.website_url.clone())
                    .unwrap_or_default();

                CandidateSource {
                    source_kind: SourceKind::CurseForge,
                    source_url: website_url,
                    provider_mod_id: Some(m.id.to_string()),
                    provider_file_id: None,
                    provider_repo: None,
                    confidence_score: confidence,
                    reasoning,
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

        let mod_info = self.get_mod(mod_id)?;
        let latest_files = self.get_latest_files(mod_id)?;

        let latest_file = latest_files.first();
        let release_id = latest_file.map(|f| f.id.to_string());
        let version_text = latest_file.and_then(|f| f.version.clone());
        let published_at = latest_file.and_then(|f| f.release_date.clone());
        let download_url = latest_file.and_then(|f| f.download_url.clone());
        let release_asset_names: Vec<String> = latest_file
            .and_then(|f| f.files.clone())
            .map(|files| files.iter().map(|f| f.filename.clone()).collect())
            .unwrap_or_default();

        let title = Some(mod_info.name);
        let website_url = mod_info.links.and_then(|l| l.website_url);

        Ok(RemoteSnapshot {
            binding_id: binding.id.clone(),
            title,
            version_text,
            published_at,
            download_url,
            changelog_url: None,
            release_id,
            release_asset_names,
            image_hashes: Vec::new(),
            etag: None,
            last_modified: None,
            evidence: SnapshotEvidence::default(),
            confidence: 0.9,
            raw: serde_json::json!({
                "mod_id": mod_id,
                "slug": mod_info.slug,
                "website_url": website_url,
            }),
        })
    }
}

impl Default for CurseForgeAdapter {
    fn default() -> Self {
        Self::new(None)
    }
}
