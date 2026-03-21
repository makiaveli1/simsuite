use crate::adapters::{AdapterRegistry, CandidateSource, DiscoverInput, FileInfo};
use crate::error::{AppError, AppResult};
use crate::models::{AppBehaviorSettings, LocalFile, LocalMod};
use crate::services::candidate_scorer::{self, MatchSignals};
use crate::services::SharedRateLimiter;
use rusqlite::{params, Connection};

#[derive(Debug)]
pub struct CandidateDiscovery {
    registry: AdapterRegistry,
}

impl CandidateDiscovery {
    pub fn new(settings: &AppBehaviorSettings, rate_limiter: SharedRateLimiter) -> Self {
        Self {
            registry: AdapterRegistry::new(settings, rate_limiter),
        }
    }

    pub fn discover_for_mod(
        &self,
        local_mod: &LocalMod,
        files: &[LocalFile],
    ) -> AppResult<Vec<CandidateSource>> {
        if local_mod.id.is_empty() {
            return Err(AppError::Message("local_mod.id cannot be empty".into()));
        }

        tracing::debug!(
            "Discovering candidates for mod: {} ({} files)",
            local_mod.display_name,
            files.len()
        );

        let input = DiscoverInput {
            local_mod_id: local_mod.id.clone(),
            display_name: local_mod.display_name.clone(),
            normalized_name: local_mod.normalized_name.clone(),
            creator_name: local_mod.creator_name.clone(),
            category: local_mod.category.clone(),
            files: files
                .iter()
                .map(|f| FileInfo {
                    file_name: f.file_name.clone(),
                    sha256: f.sha256.clone(),
                    size: f.file_size,
                })
                .collect(),
        };

        let result = self.registry.discover_all(&input);
        tracing::debug!(
            "Discovered {} candidates for mod {}",
            result.as_ref().map(|c| c.len()).unwrap_or(0),
            local_mod.display_name
        );
        result
    }

    /// Builds match signals by comparing a local mod and its files against a candidate source.
    pub fn build_signals(
        local_mod: &LocalMod,
        candidate: &CandidateSource,
        files: &[LocalFile],
    ) -> MatchSignals {
        let mut signals = MatchSignals::default();

        if candidate.source_url.contains(&local_mod.normalized_name) {
            signals.exact_title_match = true;
        }
        signals.fuzzy_title_score =
            candidate_scorer::string_similarity(&local_mod.normalized_name, &candidate.source_url);

        if let Some(creator) = &local_mod.creator_name {
            if candidate
                .source_url
                .to_lowercase()
                .contains(&creator.to_lowercase())
            {
                signals.exact_creator_match = true;
            }
            signals.fuzzy_creator_score =
                candidate_scorer::string_similarity(creator, &candidate.source_url);
        }

        if !files.is_empty() {
            let first_file = &files[0].file_name;
            signals.file_name_similarity =
                candidate_scorer::string_similarity(first_file, &candidate.source_url);
        }

        if let Some(category) = &local_mod.category {
            if candidate
                .source_url
                .to_lowercase()
                .contains(&category.to_lowercase())
            {
                signals.category_match = true;
            }
        }

        signals.user_confirmed_source = candidate.confidence_score >= 50.0;

        signals
    }

    /// Scores and ranks candidates for a local mod.
    /// Returns candidates sorted by score in descending order.
    pub fn score_candidates(
        local_mod: &LocalMod,
        candidates: Vec<CandidateSource>,
        files: &[LocalFile],
    ) -> Vec<ScoredCandidate> {
        let mut scored: Vec<ScoredCandidate> = candidates
            .into_iter()
            .map(|c| {
                let signals = Self::build_signals(local_mod, &c, files);
                let score = candidate_scorer::score_match(&signals);
                let level = candidate_scorer::confidence_level(score).to_string();
                ScoredCandidate {
                    candidate: c,
                    score,
                    level,
                    signals,
                }
            })
            .collect();

        scored.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        if tracing::enabled!(tracing::Level::DEBUG) {
            let total = scored.len();
            tracing::debug!(
                "Scored {} candidates for mod {}",
                total,
                local_mod.display_name
            );
            for (i, s) in scored.iter().take(3).enumerate() {
                tracing::trace!(
                    "  [{}/{}] score={:.1} level={}",
                    i + 1,
                    total,
                    s.score,
                    s.level
                );
            }
        }

        scored
    }

    pub fn should_auto_bind(score: f64) -> bool {
        candidate_scorer::should_auto_bind(score)
    }

    /// Stores candidate sources in the database for a local mod.
    pub fn store_candidates(
        conn: &Connection,
        local_mod_id: &str,
        candidates: &[CandidateSource],
    ) -> AppResult<()> {
        if local_mod_id.is_empty() {
            return Err(AppError::Message("local_mod_id cannot be empty".into()));
        }

        tracing::info!(
            "Storing {} candidates for mod {}",
            candidates.len(),
            local_mod_id
        );

        for candidate in candidates {
            let id = uuid::Uuid::new_v4().to_string();
            let now = chrono::Utc::now().to_rfc3339();

            let source_kind_json = serde_json::to_string(&candidate.source_kind).map_err(|e| {
                tracing::error!("Failed to serialize source_kind: {}", e);
                AppError::Json(e)
            })?;
            let reasoning_json = serde_json::to_string(&candidate.reasoning).map_err(|e| {
                tracing::error!("Failed to serialize reasoning: {}", e);
                AppError::Json(e)
            })?;

            conn.execute(
                "INSERT INTO candidate_sources (
                    id, local_mod_id, source_kind, source_url, provider_mod_id,
                    provider_file_id, provider_repo, confidence_score, reasoning_json,
                    status, created_at, updated_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                params![
                    id,
                    local_mod_id,
                    source_kind_json,
                    candidate.source_url,
                    candidate.provider_mod_id,
                    candidate.provider_file_id,
                    candidate.provider_repo,
                    candidate.confidence_score,
                    reasoning_json,
                    "suggested",
                    now,
                    now,
                ],
            )?;
        }

        Ok(())
    }
}

impl Default for CandidateDiscovery {
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

#[derive(Debug)]
pub struct ScoredCandidate {
    pub candidate: CandidateSource,
    pub score: f64,
    pub level: String,
    pub signals: MatchSignals,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::SourceKind;

    fn create_test_local_mod() -> LocalMod {
        LocalMod {
            id: "mod-1".to_string(),
            display_name: "Amazing Kinky MOD".to_string(),
            normalized_name: "amazing_kinky_mod".to_string(),
            creator_name: Some("Kinky".to_string()),
            category: Some("clothing".to_string()),
            local_root_path: "/path/to/mod".to_string(),
            tracking_mode: crate::models::TrackingMode::DetectedOnly,
            source_confidence: 0.0,
            confirmed_source_id: None,
            current_status: crate::models::UpdateStatus::Untracked,
            last_checked_at: None,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
        }
    }

    fn create_test_local_files() -> Vec<LocalFile> {
        vec![LocalFile {
            id: "file-1".to_string(),
            local_mod_id: "mod-1".to_string(),
            file_path: "/path/to/mod/amazing_kinky_mod.zip".to_string(),
            file_name: "amazing_kinky_mod.zip".to_string(),
            file_ext: ".zip".to_string(),
            file_size: 1024,
            sha256: Some("abc123".to_string()),
            modified_at: Some("2024-01-01T00:00:00Z".to_string()),
        }]
    }

    fn create_test_candidates() -> Vec<CandidateSource> {
        vec![
            CandidateSource {
                source_kind: SourceKind::CurseForge,
                source_url: "https://www.curseforge.com/sims4/amazing-kinky-mod".to_string(),
                provider_mod_id: Some("12345".to_string()),
                provider_file_id: Some("67890".to_string()),
                provider_repo: None,
                confidence_score: 30.0,
                reasoning: vec!["Title match".to_string()],
            },
            CandidateSource {
                source_kind: SourceKind::Nexus,
                source_url: "https://www.nexusmods.com/sims4/mods/amazing_kinky_mod".to_string(),
                provider_mod_id: Some("111".to_string()),
                provider_file_id: Some("222".to_string()),
                provider_repo: None,
                confidence_score: 40.0,
                reasoning: vec!["Exact title match".to_string()],
            },
            CandidateSource {
                source_kind: SourceKind::GitHub,
                source_url: "https://github.com/kinky/amazing-kinky-mod".to_string(),
                provider_mod_id: None,
                provider_file_id: None,
                provider_repo: Some("kinky/amazing-kinky-mod".to_string()),
                confidence_score: 20.0,
                reasoning: vec!["Creator match".to_string()],
            },
        ]
    }

    #[test]
    fn test_build_signals_from_local_mod() {
        let local_mod = create_test_local_mod();
        let files = create_test_local_files();
        let candidate = CandidateSource {
            source_kind: SourceKind::CurseForge,
            source_url: "https://www.curseforge.com/sims4/clothing/amazing_kinky_mod".to_string(),
            provider_mod_id: None,
            provider_file_id: None,
            provider_repo: None,
            confidence_score: 80.0,
            reasoning: vec![],
        };

        let signals = CandidateDiscovery::build_signals(&local_mod, &candidate, &files);

        assert!(signals.fuzzy_title_score >= 0.0);
        assert!(signals.exact_title_match);
        assert!(signals.exact_creator_match);
        assert!(signals.category_match);
        assert!(signals.file_name_similarity >= 0.0);
        assert!(signals.user_confirmed_source);
    }

    #[test]
    fn test_score_candidates() {
        let local_mod = create_test_local_mod();
        let files = create_test_local_files();
        let candidates = create_test_candidates();

        let scored = CandidateDiscovery::score_candidates(&local_mod, candidates, &files);

        assert_eq!(scored.len(), 3);
        assert!(scored[0].score >= scored[1].score);
        assert!(scored[1].score >= scored[2].score);
        assert!(scored[0].score >= 0.0);
        assert!(!scored[0].level.is_empty());
    }

    #[test]
    fn test_auto_bind_threshold() {
        assert!(!CandidateDiscovery::should_auto_bind(90.0));
        assert!(!CandidateDiscovery::should_auto_bind(95.0));
        assert!(!CandidateDiscovery::should_auto_bind(100.0));
        assert!(!CandidateDiscovery::should_auto_bind(50.0));
    }

    #[test]
    fn test_discover_calls_all_adapters() {
        let discovery = CandidateDiscovery::default();
        let local_mod = create_test_local_mod();
        let files = create_test_local_files();

        let result = discovery.discover_for_mod(&local_mod, &files);
        assert!(result.is_ok());
    }
}
