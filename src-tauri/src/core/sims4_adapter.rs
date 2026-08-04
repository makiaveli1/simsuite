use std::path::Path;

use crate::models::{
    GameInstallationAdapterReadinessReport, GameInstallationEvidenceStrength,
    GameInstallationProfileStatus, GameInstallationReadinessEvidence,
    GameInstallationRootValidationState,
};

use super::game_adapter::{
    GameAdapter, GameAdapterProfileEvidence, GameAdapterRootEvidence,
};

pub const SIMS4_MOD_EXTENSIONS: &[&str] = &[".package", ".ts4script"];
pub const SIMS4_TRAY_EXTENSIONS: &[&str] = &[
    ".trayitem",
    ".blueprint",
    ".bpi",
    ".householdbinary",
    ".hhi",
    ".sgi",
    ".room",
    ".rmi",
];
pub const SIMS4_MAX_SCRIPT_FOLDER_DEPTH: usize = 1;
pub const SIMS4_MAX_PACKAGE_FOLDER_DEPTH: usize = 5;

pub fn supports_mod_extension(extension: &str) -> bool {
    SIMS4_MOD_EXTENSIONS.contains(&extension)
}

pub fn supports_tray_extension(extension: &str) -> bool {
    SIMS4_TRAY_EXTENSIONS.contains(&extension)
}

#[derive(Debug, Clone, Copy)]
pub struct Sims4Adapter;

impl GameAdapter for Sims4Adapter {
    fn adapter_id(&self) -> &'static str {
        "sims4_v1"
    }

    fn game_id(&self) -> &'static str {
        "sims4"
    }

    fn supported_mod_extensions(&self) -> &'static [&'static str] {
        SIMS4_MOD_EXTENSIONS
    }

    fn supported_tray_extensions(&self) -> &'static [&'static str] {
        SIMS4_TRAY_EXTENSIONS
    }

    fn max_script_folder_depth(&self) -> usize {
        SIMS4_MAX_SCRIPT_FOLDER_DEPTH
    }

    fn max_package_folder_depth(&self) -> usize {
        SIMS4_MAX_PACKAGE_FOLDER_DEPTH
    }

    fn validate_readiness(
        &self,
        profile_evidence: GameAdapterProfileEvidence<'_>,
    ) -> GameInstallationAdapterReadinessReport {
        let mut evidence = vec![GameInstallationReadinessEvidence {
            code: "sims4_adapter_contract".to_owned(),
            strength: GameInstallationEvidenceStrength::Confirmed,
            summary: "The Sims 4 adapter owns Mods, Tray, supported-content, and placement-depth rules for this report."
                .to_owned(),
            root_ids: Vec::new(),
        }];
        let mut blockers = Vec::new();
        let mut review_notes = Vec::new();

        let mods = root_by_id(profile_evidence.roots, "mods");
        let tray = root_by_id(profile_evidence.roots, "tray");
        let user_data = root_by_id(profile_evidence.roots, "user_data");

        record_required_root_state("mods", mods, &mut blockers, &mut review_notes);
        record_required_root_state("tray", tray, &mut blockers, &mut review_notes);

        if let Some(user_data) = user_data {
            record_configured_root_state("user_data", user_data, &mut blockers, &mut review_notes);
        }

        if let (Some(mods), Some(tray)) = (mods, tray) {
            evaluate_root_coherence(
                mods,
                tray,
                user_data,
                &mut evidence,
                &mut blockers,
                &mut review_notes,
            );
        }

        for root in profile_evidence.roots {
            if matches!(root.root_id.as_str(), "user_data" | "mods" | "tray")
                && root.symlink_observed == Some(true)
            {
                review_notes.push(format!(
                    "Root '{}' uses a symlink or alias. Its canonical target is known, but the profile should remain under review until the player confirms that mapping.",
                    root.root_id
                ));
            }
        }

        let state = if !blockers.is_empty() {
            GameInstallationProfileStatus::Unavailable
        } else if !review_notes.is_empty() {
            GameInstallationProfileStatus::NeedsReview
        } else {
            GameInstallationProfileStatus::Valid
        };

        GameInstallationAdapterReadinessReport {
            adapter_id: self.adapter_id().to_owned(),
            complete: true,
            state,
            supported_mod_extensions: self
                .supported_mod_extensions()
                .iter()
                .map(|value| (*value).to_owned())
                .collect(),
            supported_tray_extensions: self
                .supported_tray_extensions()
                .iter()
                .map(|value| (*value).to_owned())
                .collect(),
            max_script_folder_depth: self.max_script_folder_depth(),
            max_package_folder_depth: self.max_package_folder_depth(),
            evidence,
            blockers,
            review_notes,
        }
    }
}

fn root_by_id<'a>(
    roots: &'a [GameAdapterRootEvidence],
    root_id: &str,
) -> Option<&'a GameAdapterRootEvidence> {
    roots.iter().find(|root| root.root_id == root_id)
}

fn record_required_root_state(
    root_id: &str,
    root: Option<&GameAdapterRootEvidence>,
    blockers: &mut Vec<String>,
    review_notes: &mut Vec<String>,
) {
    let Some(root) = root else {
        blockers.push(format!(
            "The Sims 4 profile is missing the required '{root_id}' root."
        ));
        return;
    };

    match root.state {
        GameInstallationRootValidationState::Unavailable => blockers.push(format!(
            "The required '{root_id}' root is unavailable according to generic filesystem validation."
        )),
        GameInstallationRootValidationState::NeedsReview
        | GameInstallationRootValidationState::Unvalidated => review_notes.push(format!(
            "The required '{root_id}' root still needs generic filesystem review."
        )),
        GameInstallationRootValidationState::Valid => {}
    }
}

fn record_configured_root_state(
    root_id: &str,
    root: &GameAdapterRootEvidence,
    blockers: &mut Vec<String>,
    review_notes: &mut Vec<String>,
) {
    match root.state {
        GameInstallationRootValidationState::Unavailable if root.required => blockers.push(format!(
            "The configured required '{root_id}' root is unavailable."
        )),
        GameInstallationRootValidationState::Unavailable
        | GameInstallationRootValidationState::NeedsReview
        | GameInstallationRootValidationState::Unvalidated => review_notes.push(format!(
            "The configured '{root_id}' root needs generic filesystem review."
        )),
        GameInstallationRootValidationState::Valid => {}
    }
}

fn evaluate_root_coherence(
    mods: &GameAdapterRootEvidence,
    tray: &GameAdapterRootEvidence,
    user_data: Option<&GameAdapterRootEvidence>,
    evidence: &mut Vec<GameInstallationReadinessEvidence>,
    blockers: &mut Vec<String>,
    review_notes: &mut Vec<String>,
) {
    let (Some(mods_path), Some(tray_path)) =
        (mods.canonical_path.as_deref(), tray.canonical_path.as_deref())
    else {
        if mods.state != GameInstallationRootValidationState::Unavailable
            && tray.state != GameInstallationRootValidationState::Unavailable
        {
            review_notes.push(
                "Canonical Mods and Tray identities are both required before Sims 4 root coherence can be proven."
                    .to_owned(),
            );
        }
        evidence.push(GameInstallationReadinessEvidence {
            code: "sims4_root_coherence_unknown".to_owned(),
            strength: GameInstallationEvidenceStrength::Unknown,
            summary: "Mods and Tray root coherence could not be proven from canonical path evidence."
                .to_owned(),
            root_ids: vec!["mods".to_owned(), "tray".to_owned()],
        });
        return;
    };

    if mods_path == tray_path {
        blockers.push(
            "Mods and Tray resolve to the same directory, but Sims 4 requires separate root roles."
                .to_owned(),
        );
        evidence.push(GameInstallationReadinessEvidence {
            code: "sims4_root_roles_overlap".to_owned(),
            strength: GameInstallationEvidenceStrength::Confirmed,
            summary: "Mods and Tray resolve to one canonical directory.".to_owned(),
            root_ids: vec!["mods".to_owned(), "tray".to_owned()],
        });
        return;
    }

    let Some(mods_parent) = mods_path.parent() else {
        blockers.push("The Mods root has no usable parent directory.".to_owned());
        return;
    };
    let Some(tray_parent) = tray_path.parent() else {
        blockers.push("The Tray root has no usable parent directory.".to_owned());
        return;
    };

    if let Some(user_data) = user_data {
        evaluate_with_explicit_user_data(
            mods_path,
            tray_path,
            mods_parent,
            tray_parent,
            user_data,
            evidence,
            blockers,
            review_notes,
        );
    } else if mods_parent == tray_parent {
        if mods_parent.parent().is_none() {
            review_notes.push(
                "Mods and Tray share only a filesystem root. That parent is too broad to prove one specific Sims 4 user-data tree."
                    .to_owned(),
            );
            evidence.push(GameInstallationReadinessEvidence {
                code: "sims4_user_data_parent_too_broad".to_owned(),
                strength: GameInstallationEvidenceStrength::Possible,
                summary: "Mods and Tray share a filesystem-root parent, which is insufficient coherence evidence."
                    .to_owned(),
                root_ids: vec!["mods".to_owned(), "tray".to_owned()],
            });
        } else {
            evidence.push(GameInstallationReadinessEvidence {
                code: "sims4_user_data_inferred".to_owned(),
                strength: GameInstallationEvidenceStrength::StronglySupported,
                summary: "Mods and Tray are separate canonical directories with the same direct parent, strongly supporting one Sims 4 user-data tree."
                    .to_owned(),
                root_ids: vec!["mods".to_owned(), "tray".to_owned()],
            });
        }
    } else {
        blockers.push(
            "Mods and Tray belong to different canonical parent directories, so one coherent Sims 4 user-data tree cannot be proven."
                .to_owned(),
        );
        evidence.push(GameInstallationReadinessEvidence {
            code: "sims4_user_data_conflict".to_owned(),
            strength: GameInstallationEvidenceStrength::Confirmed,
            summary: "Mods and Tray resolve under different canonical parents.".to_owned(),
            root_ids: vec!["mods".to_owned(), "tray".to_owned()],
        });
    }
}

#[allow(clippy::too_many_arguments)]
fn evaluate_with_explicit_user_data(
    mods_path: &Path,
    tray_path: &Path,
    mods_parent: &Path,
    tray_parent: &Path,
    user_data: &GameAdapterRootEvidence,
    evidence: &mut Vec<GameInstallationReadinessEvidence>,
    blockers: &mut Vec<String>,
    review_notes: &mut Vec<String>,
) {
    let Some(user_data_path) = user_data.canonical_path.as_deref() else {
        if user_data.state != GameInstallationRootValidationState::Unavailable {
            review_notes.push(
                "The explicit user-data root has no canonical identity, so its relationship to Mods and Tray cannot be proven."
                    .to_owned(),
            );
        }
        evidence.push(GameInstallationReadinessEvidence {
            code: "sims4_explicit_user_data_unknown".to_owned(),
            strength: GameInstallationEvidenceStrength::Unknown,
            summary: "The explicit user-data root could not be compared canonically."
                .to_owned(),
            root_ids: vec![
                "user_data".to_owned(),
                "mods".to_owned(),
                "tray".to_owned(),
            ],
        });
        return;
    };

    if user_data_path == mods_path || user_data_path == tray_path {
        blockers.push(
            "The explicit user-data root overlaps a required Mods or Tray root instead of containing separate child roles."
                .to_owned(),
        );
        evidence.push(GameInstallationReadinessEvidence {
            code: "sims4_user_data_role_overlap".to_owned(),
            strength: GameInstallationEvidenceStrength::Confirmed,
            summary: "The explicit user-data root resolves to the same canonical directory as Mods or Tray."
                .to_owned(),
            root_ids: vec![
                "user_data".to_owned(),
                "mods".to_owned(),
                "tray".to_owned(),
            ],
        });
        return;
    }

    if mods_parent == user_data_path && tray_parent == user_data_path {
        evidence.push(GameInstallationReadinessEvidence {
            code: "sims4_user_data_confirmed".to_owned(),
            strength: GameInstallationEvidenceStrength::Confirmed,
            summary: "Mods and Tray are separate direct children of the explicit canonical user-data root."
                .to_owned(),
            root_ids: vec![
                "user_data".to_owned(),
                "mods".to_owned(),
                "tray".to_owned(),
            ],
        });
        return;
    }

    let mods_inside = mods_path.starts_with(user_data_path);
    let tray_inside = tray_path.starts_with(user_data_path);
    if mods_inside && tray_inside {
        review_notes.push(
            "Mods and Tray are inside the explicit user-data root but are not both direct children. This custom layout needs player review."
                .to_owned(),
        );
        evidence.push(GameInstallationReadinessEvidence {
            code: "sims4_user_data_nested".to_owned(),
            strength: GameInstallationEvidenceStrength::Possible,
            summary: "Both roots are contained by the explicit user-data root, but the expected direct-child relationship is not present."
                .to_owned(),
            root_ids: vec![
                "user_data".to_owned(),
                "mods".to_owned(),
                "tray".to_owned(),
            ],
        });
    } else {
        blockers.push(
            "At least one required Sims 4 root resolves outside the explicit canonical user-data root."
                .to_owned(),
        );
        evidence.push(GameInstallationReadinessEvidence {
            code: "sims4_user_data_escape".to_owned(),
            strength: GameInstallationEvidenceStrength::Confirmed,
            summary: "Mods or Tray resolves outside the explicit user-data root."
                .to_owned(),
            root_ids: vec![
                "user_data".to_owned(),
                "mods".to_owned(),
                "tray".to_owned(),
            ],
        });
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::{
        core::game_adapter::GameAdapterProfileEvidence,
        models::{
            GameInstallationConfirmationState, GameInstallationDetectionMethod,
            GameInstallationProfile, GameInstallationRoot, GameOperatingEnvironment,
        },
    };

    fn profile() -> GameInstallationProfile {
        GameInstallationProfile {
            profile_id: "sims4-test".to_owned(),
            profile_name: "Sims 4 test".to_owned(),
            game_id: "sims4".to_owned(),
            operating_environment: GameOperatingEnvironment::NativeMacos,
            status: GameInstallationProfileStatus::NeedsReview,
            detection_method: GameInstallationDetectionMethod::Manual,
            detection_evidence_json: "{}".to_owned(),
            confirmation_state: GameInstallationConfirmationState::Confirmed,
            confirmed_at: None,
            last_validated_at: None,
            created_at: "2026-08-04T00:00:00Z".to_owned(),
            updated_at: "2026-08-04T00:00:00Z".to_owned(),
            roots: Vec::<GameInstallationRoot>::new(),
        }
    }

    fn root(root_id: &str, path: &str) -> GameAdapterRootEvidence {
        GameAdapterRootEvidence {
            root_id: root_id.to_owned(),
            required: matches!(root_id, "mods" | "tray"),
            state: GameInstallationRootValidationState::Valid,
            canonical_path: Some(PathBuf::from(path)),
            symlink_observed: Some(false),
        }
    }

    fn report(roots: &[GameAdapterRootEvidence]) -> GameInstallationAdapterReadinessReport {
        let profile = profile();
        Sims4Adapter.validate_readiness(GameAdapterProfileEvidence {
            profile: &profile,
            roots,
        })
    }

    #[test]
    fn adapter_contract_reuses_current_supported_extensions_and_depth_rules() {
        assert!(supports_mod_extension(".package"));
        assert!(supports_mod_extension(".ts4script"));
        assert!(!supports_mod_extension(".trayitem"));
        for extension in [
            ".trayitem",
            ".blueprint",
            ".bpi",
            ".householdbinary",
            ".hhi",
            ".sgi",
            ".room",
            ".rmi",
        ] {
            assert!(supports_tray_extension(extension));
        }
        assert_eq!(SIMS4_MAX_SCRIPT_FOLDER_DEPTH, 1);
        assert_eq!(SIMS4_MAX_PACKAGE_FOLDER_DEPTH, 5);
    }

    #[test]
    fn explicit_user_data_with_direct_mods_and_tray_is_valid() {
        let roots = vec![
            root("user_data", "/Players/Test/The Sims 4"),
            root("mods", "/Players/Test/The Sims 4/Custom Mods"),
            root("tray", "/Players/Test/The Sims 4/Custom Tray"),
        ];

        let report = report(&roots);

        assert_eq!(report.state, GameInstallationProfileStatus::Valid);
        assert!(report.blockers.is_empty());
        assert!(report.review_notes.is_empty());
        assert!(report.evidence.iter().any(|item| {
            item.code == "sims4_user_data_confirmed"
                && item.strength == GameInstallationEvidenceStrength::Confirmed
        }));
    }

    #[test]
    fn same_parent_without_explicit_user_data_is_strongly_supported() {
        let roots = vec![
            root("mods", "/Players/Test/The Sims 4/Mods"),
            root("tray", "/Players/Test/The Sims 4/Tray"),
        ];

        let report = report(&roots);

        assert_eq!(report.state, GameInstallationProfileStatus::Valid);
        assert!(report.evidence.iter().any(|item| {
            item.code == "sims4_user_data_inferred"
                && item.strength == GameInstallationEvidenceStrength::StronglySupported
        }));
    }

    #[test]
    fn custom_root_names_are_accepted_when_stable_roles_are_coherent() {
        let roots = vec![
            root("mods", "/Players/Test/Profile/Installed Content"),
            root("tray", "/Players/Test/Profile/Saved Builds"),
        ];

        assert_eq!(report(&roots).state, GameInstallationProfileStatus::Valid);
    }

    #[test]
    fn roots_from_different_user_data_trees_are_unavailable() {
        let roots = vec![
            root("mods", "/Players/Test/Profile A/Mods"),
            root("tray", "/Players/Test/Profile B/Tray"),
        ];

        let report = report(&roots);

        assert_eq!(report.state, GameInstallationProfileStatus::Unavailable);
        assert!(report
            .evidence
            .iter()
            .any(|item| item.code == "sims4_user_data_conflict"));
    }

    #[test]
    fn missing_required_tray_root_is_unavailable() {
        let roots = vec![root("mods", "/Players/Test/The Sims 4/Mods")];

        let report = report(&roots);

        assert_eq!(report.state, GameInstallationProfileStatus::Unavailable);
        assert!(report.blockers.join(" ").contains("tray"));
    }

    #[test]
    fn overlapping_mods_and_tray_roles_are_unavailable() {
        let roots = vec![
            root("mods", "/Players/Test/The Sims 4/Content"),
            root("tray", "/Players/Test/The Sims 4/Content"),
        ];

        let report = report(&roots);

        assert_eq!(report.state, GameInstallationProfileStatus::Unavailable);
        assert!(report
            .evidence
            .iter()
            .any(|item| item.code == "sims4_root_roles_overlap"));
    }

    #[test]
    fn explicit_user_data_cannot_overlap_mods_or_tray() {
        let roots = vec![
            root("user_data", "/Players/Test/The Sims 4/Mods"),
            root("mods", "/Players/Test/The Sims 4/Mods"),
            root("tray", "/Players/Test/The Sims 4/Mods/Tray"),
        ];

        let report = report(&roots);

        assert_eq!(report.state, GameInstallationProfileStatus::Unavailable);
        assert!(report
            .evidence
            .iter()
            .any(|item| item.code == "sims4_user_data_role_overlap"));
    }

    #[test]
    fn nested_roots_inside_explicit_user_data_need_review() {
        let roots = vec![
            root("user_data", "/Players/Test/The Sims 4"),
            root("mods", "/Players/Test/The Sims 4/Custom/Mods"),
            root("tray", "/Players/Test/The Sims 4/Custom/Tray"),
        ];

        let report = report(&roots);

        assert_eq!(report.state, GameInstallationProfileStatus::NeedsReview);
        assert!(report
            .evidence
            .iter()
            .any(|item| item.code == "sims4_user_data_nested"));
    }

    #[test]
    fn generic_root_review_keeps_adapter_in_review() {
        let mut mods = root("mods", "/Players/Test/The Sims 4/Mods");
        mods.state = GameInstallationRootValidationState::NeedsReview;
        let roots = vec![mods, root("tray", "/Players/Test/The Sims 4/Tray")];

        let report = report(&roots);

        assert_eq!(report.state, GameInstallationProfileStatus::NeedsReview);
        assert!(report.review_notes.join(" ").contains("mods"));
    }

    #[test]
    fn filesystem_root_parent_is_too_broad_for_valid_readiness() {
        let (mods_path, tray_path) = if cfg!(windows) {
            (r"C:\Mods", r"C:\Tray")
        } else {
            ("/Mods", "/Tray")
        };
        let roots = vec![root("mods", mods_path), root("tray", tray_path)];

        let report = report(&roots);

        assert_eq!(report.state, GameInstallationProfileStatus::NeedsReview);
        assert!(report
            .evidence
            .iter()
            .any(|item| item.code == "sims4_user_data_parent_too_broad"));
    }

    #[test]
    fn unrelated_downloads_alias_does_not_reduce_game_readiness() {
        let mut downloads = root("downloads", "/Players/Test/Downloads");
        downloads.symlink_observed = Some(true);
        let roots = vec![
            root("mods", "/Players/Test/The Sims 4/Mods"),
            root("tray", "/Players/Test/The Sims 4/Tray"),
            downloads,
        ];

        let report = report(&roots);

        assert_eq!(report.state, GameInstallationProfileStatus::Valid);
        assert!(!report.review_notes.join(" ").contains("downloads"));
    }

    #[test]
    fn production_adapter_source_contains_no_platform_or_filesystem_probes() {
        let source = include_str!("sims4_adapter.rs");
        let production_source = source.split("#[cfg(test)]").next().expect("production source");

        for forbidden in [
            "std::fs",
            "canonicalize(",
            "symlink_metadata(",
            "metadata(",
            "current_platform",
            "PlatformId",
            "PathMetadataState",
        ] {
            assert!(
                !production_source.contains(forbidden),
                "Sims4Adapter must consume platform evidence rather than call {forbidden}"
            );
        }
    }
}
