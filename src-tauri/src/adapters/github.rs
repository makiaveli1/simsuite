use std::time::Duration;

use reqwest::blocking::Client;
use serde::Deserialize;

use crate::adapters::{
    AdapterError, CandidateSource, DiscoverInput, FileInfo, RemoteSnapshot, SnapshotEvidence,
    SourceAdapter,
};
use crate::error::AppResult;
use crate::models::{SourceBinding, SourceKind};

const BASE_URL: &str = "https://api.github.com";

pub struct GitHubAdapter {
    client: Client,
}

#[derive(Debug, Deserialize)]
struct GhSearchResponse {
    total_count: i64,
    items: Vec<GhRepo>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GhRepo {
    id: i64,
    full_name: String,
    name: String,
    owner: GhOwner,
    description: Option<String>,
    html_url: String,
    releases_url: Option<String>,
    topics: Option<Vec<String>>,
    stargazers_count: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct GhOwner {
    login: String,
}

#[derive(Debug, Deserialize)]
struct GhReleasesResponse {
    items: Vec<GhRelease>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GhRelease {
    id: i64,
    tag_name: String,
    name: Option<String>,
    body: Option<String>,
    html_url: String,
    published_at: Option<String>,
    assets: Vec<GhAsset>,
    draft: bool,
    prerelease: bool,
}

#[derive(Debug, Deserialize)]
struct GhAsset {
    id: i64,
    name: String,
    size: i64,
    browser_download_url: Option<String>,
}

impl GitHubAdapter {
    pub fn new() -> Self {
        let client = Client::builder()
            .redirect(reqwest::redirect::Policy::limited(6))
            .timeout(Duration::from_secs(15))
            .user_agent("SimSuite/1.0")
            .build()
            .expect("GitHub client");
        GitHubAdapter { client }
    }

    fn search_repos(&self, query: &str) -> AppResult<Vec<GhRepo>> {
        let url = format!("{}/search/repositories", BASE_URL);
        let response = self
            .client
            .get(&url)
            .query(&[("q", query), ("sort", "stars"), ("per_page", "10")])
            .header("Accept", "application/vnd.github.v3+json")
            .send()?
            .error_for_status()
            .map_err(|e| AdapterError::Api(e.to_string()))?;

        let body = response.text()?;
        let result: GhSearchResponse =
            serde_json::from_str(&body).map_err(|e| AdapterError::Parse(e.to_string()))?;
        Ok(result.items)
    }

    fn get_latest_release(&self, owner: &str, repo: &str) -> AppResult<Option<GhRelease>> {
        let url = format!("{}/repos/{}/{}/releases", BASE_URL, owner, repo);
        let response = self
            .client
            .get(&url)
            .header("Accept", "application/vnd.github.v3+json")
            .send()?
            .error_for_status()
            .map_err(|e| AdapterError::Api(e.to_string()))?;

        #[derive(Deserialize)]
        struct GhReleasesWrapper {
            items: Vec<GhRelease>,
        }

        let body = response.text()?;
        let result: Vec<GhRelease> =
            serde_json::from_str(&body).map_err(|e| AdapterError::Parse(e.to_string()))?;
        Ok(result.into_iter().find(|r| !r.draft && !r.prerelease))
    }

    fn parse_owner_repo(&self, url: &str) -> Option<(String, String)> {
        let normalized = url.trim_end_matches('/');

        if normalized.contains("/releases") {
            let parts: Vec<&str> = normalized.split('/').collect();
            if parts.len() >= 2 {
                let owner = parts.get(parts.len() - 4).copied()?;
                let repo = parts.get(parts.len() - 3).copied()?;
                return Some((owner.to_string(), repo.to_string()));
            }
        }

        let re = regex::Regex::new(r"github\.com/([^/]+)/([^/]+)").ok()?;
        let caps = re.captures(normalized)?;
        let owner = caps.get(1)?.as_str();
        let repo = caps.get(2)?.as_str();
        Some((owner.to_string(), repo.to_string()))
    }

    fn calculate_confidence(
        &self,
        mod_name: &str,
        repo: &GhRepo,
        _files: &[FileInfo],
    ) -> (f64, Vec<String>) {
        let mut reasoning = Vec::new();
        let mut score: f64 = 0.4;

        let mod_lower = mod_name.to_lowercase();
        let repo_lower = repo.name.to_lowercase();
        let full_lower = repo.full_name.to_lowercase();

        if repo_lower == mod_lower || full_lower == mod_lower {
            score += 0.4;
            reasoning.push("Exact repository name match".to_string());
        } else if repo_lower.contains(&mod_lower) || mod_lower.contains(&repo_lower) {
            score += 0.2;
            reasoning.push("Partial name match".to_string());
        }

        if let Some(desc) = &repo.description {
            let desc_lower = desc.to_lowercase();
            if desc_lower.contains("sims 4") || desc_lower.contains("sims4") {
                score += 0.1;
                reasoning.push("Sims 4 related description".to_string());
            }
        }

        if let Some(topics) = &repo.topics {
            let topics_str = topics.join(" ");
            if topics_str.contains("sims") || topics_str.contains("mod") {
                score += 0.1;
                reasoning.push("Relevant topics".to_string());
            }
        }

        if let Some(stars) = repo.stargazers_count {
            if stars > 100 {
                reasoning.push(format!("Popular repo ({} stars)", stars));
            }
        }

        let final_score = score.min(1.0);
        (final_score, reasoning)
    }
}

impl SourceAdapter for GitHubAdapter {
    fn kind(&self) -> SourceKind {
        SourceKind::GitHub
    }

    fn discover_candidates(&self, input: &DiscoverInput) -> AppResult<Vec<CandidateSource>> {
        let query = format!("{} sims4 mod", input.normalized_name);
        let search_results = self.search_repos(&query)?;

        let candidates: Vec<CandidateSource> = search_results
            .into_iter()
            .map(|repo| {
                let (confidence, reasoning) =
                    self.calculate_confidence(&input.normalized_name, &repo, &input.files);

                CandidateSource {
                    source_kind: SourceKind::GitHub,
                    source_url: repo.html_url,
                    provider_mod_id: Some(repo.id.to_string()),
                    provider_file_id: None,
                    provider_repo: Some(repo.full_name),
                    confidence_score: confidence,
                    reasoning,
                }
            })
            .collect();

        Ok(candidates)
    }

    fn refresh_snapshot(&self, binding: &SourceBinding) -> AppResult<RemoteSnapshot> {
        let (owner, repo) = self
            .parse_owner_repo(&binding.source_url)
            .ok_or_else(|| AdapterError::Api("Could not parse GitHub URL".to_string()))?;

        let release = self
            .get_latest_release(&owner, &repo)?
            .ok_or_else(|| AdapterError::Api("No release found".to_string()))?;

        let release_id = Some(release.id.to_string());
        let version_text = Some(release.tag_name.clone());
        let published_at = release.published_at;
        let download_url = release
            .assets
            .first()
            .and_then(|a| a.browser_download_url.clone());
        let changelog_url = Some(release.html_url);
        let release_asset_names: Vec<String> =
            release.assets.iter().map(|a| a.name.clone()).collect();

        Ok(RemoteSnapshot {
            binding_id: binding.id.clone(),
            title: release.name.or_else(|| Some(format!("{}/{}", owner, repo))),
            version_text,
            published_at,
            download_url,
            changelog_url,
            release_id,
            release_asset_names,
            image_hashes: Vec::new(),
            etag: None,
            last_modified: None,
            evidence: SnapshotEvidence::default(),
            confidence: 0.95,
            raw: serde_json::json!({
                "owner": owner,
                "repo": repo,
                "release_id": release.id,
            }),
        })
    }
}

impl Default for GitHubAdapter {
    fn default() -> Self {
        Self::new()
    }
}
