use crate::adapters::{RemoteSnapshot, SnapshotEvidence, UpdateDecision};
use crate::core::special_mod_versions::normalize_version_with_confidence;
use crate::models::{LocalMod, UpdateStatus};

/// Detects updates by comparing a current remote snapshot against a previous one.
///
/// Returns an `UpdateDecision` with the appropriate status, confidence score,
/// and a human-readable summary of what changed.
pub fn detect_update(
    _local_mod: &LocalMod,
    previous: Option<&RemoteSnapshot>,
    current: &RemoteSnapshot,
) -> UpdateDecision {
    if previous.is_none() {
        tracing::debug!("First time checking this source");
        return UpdateDecision {
            status: UpdateStatus::SourceActivity,
            confidence: current.confidence,
            summary: Some("First time checking this source".into()),
        };
    }

    let prev = previous.unwrap();

    if current.release_id.is_some()
        && prev.release_id.is_some()
        && current.release_id != prev.release_id
    {
        tracing::info!("Confirmed update: release ID changed");
        return UpdateDecision {
            status: UpdateStatus::ConfirmedUpdate,
            confidence: 0.98,
            summary: Some("New release detected".into()),
        };
    }

    if current.version_text.is_some()
        && prev.version_text.is_some()
        && current.version_text != prev.version_text
    {
        let (_, version_confidence) =
            normalize_version_with_confidence(current.version_text.as_ref().unwrap());

        if version_confidence < 0.70 {
            tracing::debug!(
                "Ambiguous version format changed from {:?} to {:?}",
                prev.version_text,
                current.version_text
            );
            return UpdateDecision {
                status: UpdateStatus::SourceActivity,
                confidence: 0.55,
                summary: Some("Version changed but format is ambiguous".into()),
            };
        }

        tracing::info!(
            "Confirmed update: version changed from {:?} to {:?}",
            prev.version_text,
            current.version_text
        );
        return UpdateDecision {
            status: UpdateStatus::ConfirmedUpdate,
            confidence: 0.92,
            summary: Some("Version changed".into()),
        };
    }

    if current.download_url.is_some()
        && prev.download_url.is_some()
        && current.download_url != prev.download_url
    {
        tracing::debug!("Probable update: download URL changed");
        return UpdateDecision {
            status: UpdateStatus::ProbableUpdate,
            confidence: 0.78,
            summary: Some("Download target changed".into()),
        };
    }

    if current.evidence.asset_list_changed || current.evidence.feed_guid_changed {
        tracing::debug!("Probable update: source assets changed");
        return UpdateDecision {
            status: UpdateStatus::ProbableUpdate,
            confidence: 0.72,
            summary: Some("Source assets changed".into()),
        };
    }

    if current.evidence.title_changed {
        tracing::debug!("Source activity: page title changed");
        return UpdateDecision {
            status: UpdateStatus::SourceActivity,
            confidence: 0.55,
            summary: Some("Page title changed".into()),
        };
    }

    tracing::debug!("No meaningful change detected");
    UpdateDecision {
        status: UpdateStatus::UpToDate,
        confidence: 0.99,
        summary: Some("No meaningful change".into()),
    }
}

/// Compares two snapshots and returns evidence of what changed between them.
pub fn compare_snapshots(prev: &RemoteSnapshot, curr: &RemoteSnapshot) -> SnapshotEvidence {
    SnapshotEvidence {
        version_changed: prev.version_text != curr.version_text,
        download_changed: prev.download_url != curr.download_url,
        title_changed: prev.title != curr.title,
        asset_list_changed: prev.release_asset_names != curr.release_asset_names,
        feed_guid_changed: false,
    }
}
