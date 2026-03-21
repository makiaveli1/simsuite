use crate::error::{AppError, AppResult};
use crate::models::SourceKind;
use rusqlite::{params, Connection};
use std::collections::HashMap;
use url::Url;

#[derive(Debug)]
pub struct SourceLearning;

impl SourceLearning {
    /// Creates a new SourceLearning instance.
    pub fn new() -> Self {
        Self
    }

    /// Records a user confirmation of a source for a domain.
    pub fn record_confirmation(
        conn: &Connection,
        domain: &str,
        source_kind: SourceKind,
    ) -> AppResult<()> {
        if domain.is_empty() {
            return Err(AppError::Message("domain cannot be empty".into()));
        }

        tracing::debug!(
            "Recording confirmation for domain: {} with kind {:?}",
            domain,
            source_kind
        );

        let now = chrono::Utc::now().to_rfc3339();

        let source_kind_json = serde_json::to_string(&source_kind).map_err(|e| {
            tracing::error!("Failed to serialize source_kind: {}", e);
            AppError::Json(e)
        })?;

        conn.execute(
            "INSERT INTO source_learning (id, domain, source_kind, confirm_count, created_at, updated_at)
             VALUES (?1, ?2, ?3, 1, ?4, ?4)
             ON CONFLICT(domain) DO UPDATE SET
                confirm_count = confirm_count + 1,
                source_kind = excluded.source_kind,
                updated_at = excluded.updated_at",
            params![
                uuid::Uuid::new_v4().to_string(),
                domain,
                source_kind_json,
                now,
            ],
        )?;
        Ok(())
    }

    /// Records a user rejection of a source for a domain.
    pub fn record_rejection(conn: &Connection, domain: &str) -> AppResult<()> {
        if domain.is_empty() {
            return Err(AppError::Message("domain cannot be empty".into()));
        }

        tracing::debug!("Recording rejection for domain: {}", domain);

        let now = chrono::Utc::now().to_rfc3339();

        conn.execute(
            "INSERT INTO source_learning (id, domain, reject_count, created_at, updated_at)
             VALUES (?1, ?2, 1, ?3, ?3)
             ON CONFLICT(domain) DO UPDATE SET
                reject_count = reject_count + 1,
                updated_at = excluded.updated_at",
            params![uuid::Uuid::new_v4().to_string(), domain, now,],
        )?;
        Ok(())
    }

    /// Returns all learned domains where confirmations exceed rejections.
    #[allow(dead_code)]
    pub fn get_learned_domains(conn: &Connection) -> AppResult<HashMap<String, LearnedDomain>> {
        let mut stmt = conn.prepare(
            "SELECT domain, source_kind, confirm_count, reject_count 
             FROM source_learning 
             WHERE confirm_count > reject_count",
        )?;

        let rows = stmt.query_map([], |row| {
            let kind_str: String = row.get(1)?;
            let source_kind = serde_json::from_str(&kind_str).unwrap_or_else(|e| {
                tracing::warn!("Failed to parse source_kind JSON '{}': {}", kind_str, e);
                None
            });

            Ok(LearnedDomain {
                domain: row.get(0)?,
                source_kind,
                confirm_count: row.get(2)?,
                reject_count: row.get(3)?,
            })
        })?;

        let mut map = HashMap::new();
        for row in rows {
            if let Ok(domain) = row {
                map.insert(domain.domain.clone(), domain);
            }
        }
        tracing::debug!("Retrieved {} learned domains", map.len());
        Ok(map)
    }

    /// Extracts the domain from a URL string.
    pub fn extract_domain(url_str: &str) -> Option<String> {
        Url::parse(url_str)
            .ok()
            .and_then(|u| u.host_str().map(|s| s.to_string()))
    }

    /// Adjusts a score based on learned domain history.
    /// Boosts confirmed domains and penalizes rejected ones.
    #[allow(dead_code)]
    pub fn boost_for_learned_domain(
        score: f64,
        domain: &str,
        learned: &HashMap<String, LearnedDomain>,
    ) -> f64 {
        if let Some(info) = learned.get(domain) {
            if info.confirm_count > info.reject_count {
                tracing::debug!("Boosting score for confirmed domain: {}", domain);
                return (score + 10.0).min(100.0);
            } else if info.reject_count > info.confirm_count {
                tracing::debug!("Penalizing score for rejected domain: {}", domain);
                return (score - 20.0).max(0.0);
            }
        }
        score
    }
}

impl Default for SourceLearning {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct LearnedDomain {
    pub domain: String,
    pub source_kind: Option<SourceKind>,
    pub confirm_count: i64,
    pub reject_count: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_domain() {
        assert_eq!(
            SourceLearning::extract_domain("https://api.curseforge.com/v1/mods"),
            Some("api.curseforge.com".to_string())
        );
    }

    #[test]
    fn test_extract_domain_with_www() {
        assert_eq!(
            SourceLearning::extract_domain("https://www.nexusmods.com/mods/1"),
            Some("www.nexusmods.com".to_string())
        );
    }

    #[test]
    fn test_extract_domain_invalid() {
        assert_eq!(SourceLearning::extract_domain("not a url"), None);
    }

    #[test]
    fn test_boost_confirmed_domain() {
        let mut learned = HashMap::new();
        learned.insert(
            "curseforge.com".to_string(),
            LearnedDomain {
                domain: "curseforge.com".to_string(),
                source_kind: Some(SourceKind::CurseForge),
                confirm_count: 5,
                reject_count: 1,
            },
        );

        let boosted = SourceLearning::boost_for_learned_domain(50.0, "curseforge.com", &learned);
        assert_eq!(boosted, 60.0);
    }

    #[test]
    fn test_penalize_rejected_domain() {
        let mut learned = HashMap::new();
        learned.insert(
            "fake-site.com".to_string(),
            LearnedDomain {
                domain: "fake-site.com".to_string(),
                source_kind: None,
                confirm_count: 1,
                reject_count: 5,
            },
        );

        let penalized = SourceLearning::boost_for_learned_domain(50.0, "fake-site.com", &learned);
        assert_eq!(penalized, 30.0);
    }

    #[test]
    fn test_boost_preserves_score_for_unknown_domain() {
        let learned = HashMap::new();
        let score = SourceLearning::boost_for_learned_domain(50.0, "unknown.com", &learned);
        assert_eq!(score, 50.0);
    }

    #[test]
    fn test_boost_caps_at_100() {
        let mut learned = HashMap::new();
        learned.insert(
            "trusted.com".to_string(),
            LearnedDomain {
                domain: "trusted.com".to_string(),
                source_kind: Some(SourceKind::CurseForge),
                confirm_count: 10,
                reject_count: 0,
            },
        );

        let boosted = SourceLearning::boost_for_learned_domain(95.0, "trusted.com", &learned);
        assert_eq!(boosted, 100.0);
    }

    #[test]
    fn test_penalty_floors_at_0() {
        let mut learned = HashMap::new();
        learned.insert(
            "banned.com".to_string(),
            LearnedDomain {
                domain: "banned.com".to_string(),
                source_kind: None,
                confirm_count: 0,
                reject_count: 10,
            },
        );

        let penalized = SourceLearning::boost_for_learned_domain(15.0, "banned.com", &learned);
        assert_eq!(penalized, 0.0);
    }

    #[test]
    fn test_balanced_domain_no_change() {
        let mut learned = HashMap::new();
        learned.insert(
            "neutral.com".to_string(),
            LearnedDomain {
                domain: "neutral.com".to_string(),
                source_kind: Some(SourceKind::GenericPage),
                confirm_count: 5,
                reject_count: 5,
            },
        );

        let score = SourceLearning::boost_for_learned_domain(50.0, "neutral.com", &learned);
        assert_eq!(score, 50.0);
    }
}
