use crate::models::SourceKind;
use std::collections::HashMap;
use std::time::Duration;

#[derive(Clone, Debug)]
pub struct UpdateScheduler {
    source_intervals: HashMap<SourceKind, Duration>,
    default_interval: Duration,
}

impl UpdateScheduler {
    /// Creates a new scheduler with default intervals for each source kind.
    pub fn new() -> Self {
        let mut intervals = HashMap::new();

        intervals.insert(SourceKind::CurseForge, Duration::from_secs(6 * 60 * 60));
        intervals.insert(SourceKind::GitHub, Duration::from_secs(6 * 60 * 60));
        intervals.insert(SourceKind::Nexus, Duration::from_secs(6 * 60 * 60));

        intervals.insert(SourceKind::Feed, Duration::from_secs(12 * 60 * 60));

        intervals.insert(
            SourceKind::StructuredPage,
            Duration::from_secs(24 * 60 * 60),
        );
        intervals.insert(SourceKind::GenericPage, Duration::from_secs(48 * 60 * 60));

        Self {
            source_intervals: intervals,
            default_interval: Duration::from_secs(24 * 60 * 60),
        }
    }

    /// Returns the check interval for a given source kind.
    pub fn get_interval(&self, kind: SourceKind) -> Duration {
        self.source_intervals
            .get(&kind)
            .copied()
            .unwrap_or(self.default_interval)
    }

    /// Returns true if a check is due based on the last check time.
    pub fn is_due(&self, last_check: Option<&str>, kind: SourceKind) -> bool {
        let interval = self.get_interval(kind);

        if let Some(last_str) = last_check {
            if let Ok(last_time) = chrono::DateTime::parse_from_rfc3339(last_str) {
                let now = chrono::Utc::now();
                let elapsed = now.signed_duration_since(last_time.with_timezone(&chrono::Utc));
                return elapsed.to_std().map(|d| d >= interval).unwrap_or(true);
            }
        }

        true
    }

    /// Returns the timestamp of the next check, or None if checks are not scheduled.
    #[allow(dead_code)]
    pub fn next_check_at(
        &self,
        last_check: Option<&str>,
        kind: SourceKind,
    ) -> Option<chrono::DateTime<chrono::Utc>> {
        let interval = self.get_interval(kind);

        if let Some(last_str) = last_check {
            if let Ok(last_time) = chrono::DateTime::parse_from_rfc3339(last_str) {
                return Some(
                    last_time.with_timezone(&chrono::Utc)
                        + chrono::Duration::from_std(interval).unwrap_or_default(),
                );
            }
        }

        Some(chrono::Utc::now())
    }
}

impl Default for UpdateScheduler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_curseforge_interval() {
        let scheduler = UpdateScheduler::new();
        assert_eq!(
            scheduler.get_interval(SourceKind::CurseForge),
            Duration::from_secs(6 * 60 * 60)
        );
    }

    #[test]
    fn test_generic_page_interval() {
        let scheduler = UpdateScheduler::new();
        assert_eq!(
            scheduler.get_interval(SourceKind::GenericPage),
            Duration::from_secs(48 * 60 * 60)
        );
    }

    #[test]
    fn test_is_due_never_checked() {
        let scheduler = UpdateScheduler::new();
        assert!(scheduler.is_due(None, SourceKind::CurseForge));
    }
}
