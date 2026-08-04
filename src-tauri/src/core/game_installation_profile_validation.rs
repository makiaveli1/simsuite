use std::path::PathBuf;

use crate::{
    models::{
        GameInstallationCaseSensitivity, GameInstallationEnvironmentCompatibility,
        GameInstallationPathMetadataState, GameInstallationProfile,
        GameInstallationProfileStatus, GameInstallationProfileValidationReport,
        GameInstallationRoot, GameInstallationRootValidationReport,
        GameInstallationRootValidationState, GameOperatingEnvironment,
    },
    platform::{
        current_platform,
        path_semantics::{
            inspect_path_metadata, CaseSensitivity, PathMetadataState, RootIdentity,
        },
        PlatformId,
    },
};

use super::game_adapter::{
    validate_game_readiness, GameAdapterProfileEvidence, GameAdapterRootEvidence,
};

#[derive(Debug, Clone)]
struct ValidatedRoot {
    report: GameInstallationRootValidationReport,
    adapter_evidence: GameAdapterRootEvidence,
}

pub fn validate_game_installation_profile(
    profile: &GameInstallationProfile,
) -> GameInstallationProfileValidationReport {
    validate_game_installation_profile_for_platform(profile, current_platform())
}

pub fn game_installation_profile_environment_compatibility(
    profile: &GameInstallationProfile,
) -> GameInstallationEnvironmentCompatibility {
    environment_compatibility(profile.operating_environment, current_platform())
}

pub fn current_manual_sims4_environment() -> GameOperatingEnvironment {
    match current_platform() {
        PlatformId::Windows => GameOperatingEnvironment::NativeWindows,
        PlatformId::Macos => GameOperatingEnvironment::NativeMacos,
        PlatformId::Linux => GameOperatingEnvironment::Unknown,
    }
}

fn validate_game_installation_profile_for_platform(
    profile: &GameInstallationProfile,
    platform: PlatformId,
) -> GameInstallationProfileValidationReport {
    let current_environment = native_environment(platform);
    let environment_compatibility =
        environment_compatibility(profile.operating_environment, platform);
    let mut profile_review_notes = Vec::new();

    match environment_compatibility {
        GameInstallationEnvironmentCompatibility::Matches => {}
        GameInstallationEnvironmentCompatibility::Mismatch => profile_review_notes.push(
            "This profile targets a different native operating system. Root probing was skipped so SimSuite does not interpret foreign path syntax with the current host rules."
                .to_owned(),
        ),
        GameInstallationEnvironmentCompatibility::RequiresAdapter => profile_review_notes.push(
            "This profile uses Wine, Proton, or Lutris. Environment-aware path validation is pending the platform adapter and was not guessed."
                .to_owned(),
        ),
        GameInstallationEnvironmentCompatibility::Unknown => profile_review_notes.push(
            "The profile operating environment is unknown, so filesystem validation was not guessed."
                .to_owned(),
        ),
    }

    let validated_roots = profile
        .roots
        .iter()
        .map(|root| {
            if environment_compatibility == GameInstallationEnvironmentCompatibility::Matches {
                validate_root(root, platform)
            } else {
                skipped_root_report(root)
            }
        })
        .collect::<Vec<_>>();
    let roots = validated_roots
        .iter()
        .map(|root| root.report.clone())
        .collect::<Vec<_>>();
    let adapter_roots = validated_roots
        .iter()
        .map(|root| root.adapter_evidence.clone())
        .collect::<Vec<_>>();

    let generic_root_state = derive_generic_root_state(&roots);
    if roots.is_empty() {
        profile_review_notes.push(
            "The profile has no configured roots, so no filesystem readiness can be proven."
                .to_owned(),
        );
    }

    let player_confirmed = profile.confirmation_state
        == crate::models::GameInstallationConfirmationState::Confirmed;
    if !player_confirmed {
        profile_review_notes.push(
            "The profile is not currently confirmed by the player and requires review."
                .to_owned(),
        );
    }

    let game_readiness = if environment_compatibility
        == GameInstallationEnvironmentCompatibility::Matches
    {
        validate_game_readiness(GameAdapterProfileEvidence {
            profile,
            roots: &adapter_roots,
        })
    } else {
        None
    };
    let game_specific_validation_pending = game_readiness
        .as_ref()
        .map(|report| !report.complete)
        .unwrap_or(true);

    if game_readiness.is_none()
        && environment_compatibility == GameInstallationEnvironmentCompatibility::Matches
    {
        profile_review_notes.push(format!(
            "No game adapter is registered for '{}', so game-specific readiness was not guessed.",
            profile.game_id
        ));
    }

    let mut blockers = roots
        .iter()
        .flat_map(|root| {
            root.blockers
                .iter()
                .map(move |message| format!("{}: {message}", root.root_id))
        })
        .collect::<Vec<_>>();
    if let Some(game_readiness) = &game_readiness {
        blockers.extend(
            game_readiness
                .blockers
                .iter()
                .map(|message| format!("{}: {message}", game_readiness.adapter_id)),
        );
    }

    let mut aggregated_review_notes = profile_review_notes;
    aggregated_review_notes.extend(roots.iter().flat_map(|root| {
        root.review_notes
            .iter()
            .map(move |message| format!("{}: {message}", root.root_id))
    }));
    if let Some(game_readiness) = &game_readiness {
        aggregated_review_notes.extend(
            game_readiness
                .review_notes
                .iter()
                .map(|message| format!("{}: {message}", game_readiness.adapter_id)),
        );
    }

    let game_state = game_readiness.as_ref().map(|report| report.state);
    let state = if generic_root_state == GameInstallationProfileStatus::Unavailable
        || game_state == Some(GameInstallationProfileStatus::Unavailable)
    {
        GameInstallationProfileStatus::Unavailable
    } else if generic_root_state == GameInstallationProfileStatus::Valid
        && game_state == Some(GameInstallationProfileStatus::Valid)
        && player_confirmed
    {
        GameInstallationProfileStatus::Valid
    } else {
        GameInstallationProfileStatus::NeedsReview
    };

    GameInstallationProfileValidationReport {
        profile_id: profile.profile_id.clone(),
        profile_name: profile.profile_name.clone(),
        game_id: profile.game_id.clone(),
        stored_environment: profile.operating_environment,
        current_environment,
        environment_compatibility,
        state,
        generic_root_state,
        game_specific_validation_pending,
        game_readiness,
        read_only: true,
        roots,
        blockers,
        review_notes: aggregated_review_notes,
    }
}

fn validate_root(
    root: &GameInstallationRoot,
    platform: PlatformId,
) -> ValidatedRoot {
    let path = PathBuf::from(&root.configured_path);
    if !path.is_absolute() {
        return failed_root_report(
            root,
            Some(false),
            None,
            None,
            GameInstallationPathMetadataState::NotChecked,
            None,
            GameInstallationCaseSensitivity::NotChecked,
            None,
            "The configured root must be an absolute path.".to_owned(),
        );
    }

    let metadata_state = inspect_path_metadata(&path);
    match metadata_state {
        PathMetadataState::Missing => failed_root_report(
            root,
            Some(true),
            Some(false),
            Some(false),
            GameInstallationPathMetadataState::Missing,
            None,
            GameInstallationCaseSensitivity::NotChecked,
            Some(false),
            "The configured root does not exist.".to_owned(),
        ),
        PathMetadataState::File => failed_root_report(
            root,
            Some(true),
            Some(true),
            Some(true),
            GameInstallationPathMetadataState::File,
            None,
            GameInstallationCaseSensitivity::NotChecked,
            Some(false),
            "The configured root points to a file instead of a directory.".to_owned(),
        ),
        PathMetadataState::Other => failed_root_report(
            root,
            Some(true),
            Some(true),
            Some(true),
            GameInstallationPathMetadataState::Other,
            None,
            GameInstallationCaseSensitivity::NotChecked,
            Some(false),
            "The configured root is not a regular directory.".to_owned(),
        ),
        PathMetadataState::Unreadable(kind) => failed_root_report(
            root,
            Some(true),
            None,
            Some(false),
            GameInstallationPathMetadataState::Unreadable,
            None,
            GameInstallationCaseSensitivity::NotChecked,
            None,
            format!("Root metadata could not be read safely ({kind:?})."),
        ),
        PathMetadataState::Directory | PathMetadataState::Symlink => {
            let symlink_observed = metadata_state == PathMetadataState::Symlink;
            match RootIdentity::probe_existing_root(&root.root_id, platform, &path) {
                Ok(identity) => {
                    let case_sensitivity =
                        map_case_sensitivity(identity.capabilities.case_sensitivity);
                    let mut state = GameInstallationRootValidationState::Valid;
                    let blockers = Vec::new();
                    let mut review_notes = Vec::new();

                    if symlink_observed {
                        state = GameInstallationRootValidationState::NeedsReview;
                        review_notes.push(
                            "The configured root is a symlink or alias. Its canonical target was resolved, but relationship checks are pending the game adapter."
                                .to_owned(),
                        );
                    }
                    if case_sensitivity == GameInstallationCaseSensitivity::Unknown {
                        state = GameInstallationRootValidationState::NeedsReview;
                        review_notes.push(
                            "Root-local case sensitivity could not be proven without guessing."
                                .to_owned(),
                        );
                    }

                    let canonical_path = identity.canonical_root;
                    let report = GameInstallationRootValidationReport {
                        root_id: root.root_id.clone(),
                        root_role: root.root_role.clone(),
                        configured_path: root.configured_path.clone(),
                        required: root.required,
                        state,
                        absolute_path: Some(true),
                        exists: Some(true),
                        metadata_readable: Some(true),
                        metadata_state: if symlink_observed {
                            GameInstallationPathMetadataState::Symlink
                        } else {
                            GameInstallationPathMetadataState::Directory
                        },
                        canonical_path_display: Some(canonical_path.display().to_string()),
                        case_sensitivity,
                        symlink_observed: Some(symlink_observed),
                        blockers,
                        review_notes,
                    };
                    ValidatedRoot {
                        adapter_evidence: GameAdapterRootEvidence {
                            root_id: root.root_id.clone(),
                            required: root.required,
                            state,
                            canonical_path: Some(canonical_path),
                            symlink_observed: Some(symlink_observed),
                        },
                        report,
                    }
                }
                Err(error) => failed_root_report(
                    root,
                    Some(true),
                    Some(true),
                    Some(true),
                    if symlink_observed {
                        GameInstallationPathMetadataState::Symlink
                    } else {
                        GameInstallationPathMetadataState::Directory
                    },
                    None,
                    GameInstallationCaseSensitivity::NotChecked,
                    Some(symlink_observed),
                    format!("Canonical root identity could not be proven: {error}"),
                ),
            }
        }
    }
}

fn skipped_root_report(root: &GameInstallationRoot) -> ValidatedRoot {
    let state = GameInstallationRootValidationState::NeedsReview;
    let report = GameInstallationRootValidationReport {
        root_id: root.root_id.clone(),
        root_role: root.root_role.clone(),
        configured_path: root.configured_path.clone(),
        required: root.required,
        state,
        absolute_path: None,
        exists: None,
        metadata_readable: None,
        metadata_state: GameInstallationPathMetadataState::NotChecked,
        canonical_path_display: None,
        case_sensitivity: GameInstallationCaseSensitivity::NotChecked,
        symlink_observed: None,
        blockers: Vec::new(),
        review_notes: vec![
            "Root probing was skipped because the stored operating environment cannot yet be interpreted safely on this host."
                .to_owned(),
        ],
    };
    ValidatedRoot {
        adapter_evidence: GameAdapterRootEvidence {
            root_id: root.root_id.clone(),
            required: root.required,
            state,
            canonical_path: None,
            symlink_observed: None,
        },
        report,
    }
}

#[allow(clippy::too_many_arguments)]
fn failed_root_report(
    root: &GameInstallationRoot,
    absolute_path: Option<bool>,
    exists: Option<bool>,
    metadata_readable: Option<bool>,
    metadata_state: GameInstallationPathMetadataState,
    canonical_path_display: Option<String>,
    case_sensitivity: GameInstallationCaseSensitivity,
    symlink_observed: Option<bool>,
    message: String,
) -> ValidatedRoot {
    let state = if root.required {
        GameInstallationRootValidationState::Unavailable
    } else {
        GameInstallationRootValidationState::NeedsReview
    };
    let (blockers, review_notes) = if root.required {
        (vec![message], Vec::new())
    } else {
        (Vec::new(), vec![message])
    };

    let report = GameInstallationRootValidationReport {
        root_id: root.root_id.clone(),
        root_role: root.root_role.clone(),
        configured_path: root.configured_path.clone(),
        required: root.required,
        state,
        absolute_path,
        exists,
        metadata_readable,
        metadata_state,
        canonical_path_display,
        case_sensitivity,
        symlink_observed,
        blockers,
        review_notes,
    };
    ValidatedRoot {
        adapter_evidence: GameAdapterRootEvidence {
            root_id: root.root_id.clone(),
            required: root.required,
            state,
            canonical_path: None,
            symlink_observed,
        },
        report,
    }
}

fn derive_generic_root_state(
    roots: &[GameInstallationRootValidationReport],
) -> GameInstallationProfileStatus {
    if roots.is_empty() {
        return GameInstallationProfileStatus::NeedsReview;
    }
    if roots.iter().any(|root| {
        root.required && root.state == GameInstallationRootValidationState::Unavailable
    }) {
        return GameInstallationProfileStatus::Unavailable;
    }
    if roots
        .iter()
        .any(|root| root.state != GameInstallationRootValidationState::Valid)
    {
        return GameInstallationProfileStatus::NeedsReview;
    }
    GameInstallationProfileStatus::Valid
}

fn environment_compatibility(
    environment: GameOperatingEnvironment,
    platform: PlatformId,
) -> GameInstallationEnvironmentCompatibility {
    match environment {
        GameOperatingEnvironment::NativeWindows if platform == PlatformId::Windows => {
            GameInstallationEnvironmentCompatibility::Matches
        }
        GameOperatingEnvironment::NativeMacos if platform == PlatformId::Macos => {
            GameInstallationEnvironmentCompatibility::Matches
        }
        GameOperatingEnvironment::NativeLinux if platform == PlatformId::Linux => {
            GameInstallationEnvironmentCompatibility::Matches
        }
        GameOperatingEnvironment::NativeWindows
        | GameOperatingEnvironment::NativeMacos
        | GameOperatingEnvironment::NativeLinux => {
            GameInstallationEnvironmentCompatibility::Mismatch
        }
        GameOperatingEnvironment::Wine
        | GameOperatingEnvironment::Proton
        | GameOperatingEnvironment::Lutris => {
            GameInstallationEnvironmentCompatibility::RequiresAdapter
        }
        GameOperatingEnvironment::Unknown => GameInstallationEnvironmentCompatibility::Unknown,
    }
}

fn native_environment(platform: PlatformId) -> GameOperatingEnvironment {
    match platform {
        PlatformId::Windows => GameOperatingEnvironment::NativeWindows,
        PlatformId::Macos => GameOperatingEnvironment::NativeMacos,
        PlatformId::Linux => GameOperatingEnvironment::NativeLinux,
    }
}

fn map_case_sensitivity(value: CaseSensitivity) -> GameInstallationCaseSensitivity {
    match value {
        CaseSensitivity::Sensitive => GameInstallationCaseSensitivity::Sensitive,
        CaseSensitivity::Insensitive => GameInstallationCaseSensitivity::Insensitive,
        CaseSensitivity::Unknown => GameInstallationCaseSensitivity::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::*;
    use crate::models::{
        GameInstallationConfirmationState, GameInstallationDetectionMethod,
    };

    fn profile(
        environment: GameOperatingEnvironment,
        roots: Vec<GameInstallationRoot>,
    ) -> GameInstallationProfile {
        GameInstallationProfile {
            profile_id: "profile-test".to_owned(),
            profile_name: "Test profile".to_owned(),
            game_id: "sims4".to_owned(),
            operating_environment: environment,
            status: GameInstallationProfileStatus::NeedsReview,
            detection_method: GameInstallationDetectionMethod::Manual,
            detection_evidence_json: "{}".to_owned(),
            confirmation_state: GameInstallationConfirmationState::Confirmed,
            confirmed_at: Some("2026-08-04T00:00:00Z".to_owned()),
            last_validated_at: None,
            created_at: "2026-08-04T00:00:00Z".to_owned(),
            updated_at: "2026-08-04T00:00:00Z".to_owned(),
            roots,
        }
    }

    fn root(root_id: &str, path: PathBuf, required: bool) -> GameInstallationRoot {
        GameInstallationRoot {
            profile_id: "profile-test".to_owned(),
            root_id: root_id.to_owned(),
            root_role: format!("{root_id}_role"),
            configured_path: path.display().to_string(),
            required,
            validation_state: GameInstallationRootValidationState::Unvalidated,
            filesystem_capabilities_json: "{}".to_owned(),
            last_validated_at: None,
            created_at: "2026-08-04T00:00:00Z".to_owned(),
            updated_at: "2026-08-04T00:00:00Z".to_owned(),
        }
    }

    fn known_case_root(parent: &std::path::Path, name: &str) -> PathBuf {
        let root = parent.join(name);
        fs::create_dir(&root).expect("root directory");
        fs::write(root.join("CaseProbe.package"), b"probe").expect("case probe file");
        root
    }

    #[test]
    fn native_coherent_roots_can_be_fully_validated_without_persistence() {
        let directory = tempdir().expect("tempdir");
        let mods = known_case_root(directory.path(), "Mods");
        let tray = known_case_root(directory.path(), "Tray");
        let profile = profile(
            native_environment(current_platform()),
            vec![root("mods", mods, true), root("tray", tray, true)],
        );

        let report = validate_game_installation_profile(&profile);

        assert_eq!(report.environment_compatibility, GameInstallationEnvironmentCompatibility::Matches);
        assert_eq!(report.generic_root_state, GameInstallationProfileStatus::Valid);
        assert_eq!(report.state, GameInstallationProfileStatus::Valid);
        assert!(!report.game_specific_validation_pending);
        let game_readiness = report.game_readiness.as_ref().expect("Sims 4 readiness");
        assert_eq!(game_readiness.adapter_id, "sims4_v1");
        assert_eq!(game_readiness.state, GameInstallationProfileStatus::Valid);
        assert!(game_readiness.evidence.iter().any(|item| {
            item.code == "sims4_user_data_inferred"
                && item.strength
                    == crate::models::GameInstallationEvidenceStrength::StronglySupported
        }));
        assert!(report.read_only);
        assert!(report.roots.iter().all(|root| {
            root.state == GameInstallationRootValidationState::Valid
                && root.canonical_path_display.is_some()
                && matches!(
                    root.case_sensitivity,
                    GameInstallationCaseSensitivity::Sensitive
                        | GameInstallationCaseSensitivity::Insensitive
                )
        }));
    }

    #[test]
    fn missing_required_root_makes_generic_profile_unavailable() {
        let directory = tempdir().expect("tempdir");
        let missing = directory.path().join("MissingMods");
        let profile = profile(
            native_environment(current_platform()),
            vec![root("mods", missing, true)],
        );

        let report = validate_game_installation_profile(&profile);

        assert_eq!(report.generic_root_state, GameInstallationProfileStatus::Unavailable);
        assert_eq!(report.state, GameInstallationProfileStatus::Unavailable);
        assert_eq!(report.roots[0].metadata_state, GameInstallationPathMetadataState::Missing);
        assert!(!report.roots[0].blockers.is_empty());
    }

    #[test]
    fn relative_required_root_fails_closed_before_filesystem_lookup() {
        let profile = profile(
            native_environment(current_platform()),
            vec![root("mods", PathBuf::from("relative/Mods"), true)],
        );

        let report = validate_game_installation_profile(&profile);

        assert_eq!(report.generic_root_state, GameInstallationProfileStatus::Unavailable);
        assert_eq!(report.roots[0].absolute_path, Some(false));
        assert_eq!(report.roots[0].metadata_state, GameInstallationPathMetadataState::NotChecked);
    }

    #[test]
    fn empty_root_keeps_unknown_case_policy_in_review() {
        let directory = tempdir().expect("tempdir");
        let empty_root = directory.path().join("EmptyMods");
        fs::create_dir(&empty_root).expect("empty root");
        let profile = profile(
            native_environment(current_platform()),
            vec![root("mods", empty_root, true)],
        );

        let report = validate_game_installation_profile(&profile);

        assert_eq!(report.generic_root_state, GameInstallationProfileStatus::NeedsReview);
        assert_eq!(report.roots[0].state, GameInstallationRootValidationState::NeedsReview);
        assert_eq!(report.roots[0].case_sensitivity, GameInstallationCaseSensitivity::Unknown);
    }

    #[test]
    fn foreign_native_profile_is_not_interpreted_with_host_path_rules() {
        let foreign_environment = match current_platform() {
            PlatformId::Windows => GameOperatingEnvironment::NativeMacos,
            PlatformId::Macos | PlatformId::Linux => GameOperatingEnvironment::NativeWindows,
        };
        let profile = profile(
            foreign_environment,
            vec![root("mods", PathBuf::from("foreign-native-path"), true)],
        );

        let report = validate_game_installation_profile(&profile);

        assert_eq!(report.environment_compatibility, GameInstallationEnvironmentCompatibility::Mismatch);
        assert_eq!(report.generic_root_state, GameInstallationProfileStatus::NeedsReview);
        assert_eq!(report.roots[0].absolute_path, None);
        assert_eq!(report.roots[0].metadata_state, GameInstallationPathMetadataState::NotChecked);
        assert!(report.roots[0].blockers.is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn symlink_root_reports_canonical_target_and_stays_in_review() {
        use std::os::unix::fs::symlink;

        let directory = tempdir().expect("tempdir");
        let target = known_case_root(directory.path(), "RealMods");
        let alias = directory.path().join("ModsAlias");
        symlink(&target, &alias).expect("root symlink");
        let profile = profile(
            native_environment(current_platform()),
            vec![root("mods", alias, true)],
        );

        let report = validate_game_installation_profile(&profile);

        assert_eq!(report.roots[0].metadata_state, GameInstallationPathMetadataState::Symlink);
        assert_eq!(report.roots[0].symlink_observed, Some(true));
        assert_eq!(report.roots[0].state, GameInstallationRootValidationState::NeedsReview);
        let expected_target = target
            .canonicalize()
            .expect("canonical target")
            .display()
            .to_string();
        assert_eq!(
            report.roots[0].canonical_path_display.as_deref(),
            Some(expected_target.as_str())
        );
    }

    #[test]
    fn manual_sims4_environment_does_not_guess_native_linux() {
        let expected = match current_platform() {
            PlatformId::Windows => GameOperatingEnvironment::NativeWindows,
            PlatformId::Macos => GameOperatingEnvironment::NativeMacos,
            PlatformId::Linux => GameOperatingEnvironment::Unknown,
        };

        assert_eq!(current_manual_sims4_environment(), expected);
    }

    #[test]
    fn production_validation_source_contains_no_file_mutation_calls() {
        let source = include_str!("game_installation_profile_validation.rs");
        let production_source = source.split("#[cfg(test)]").next().expect("production source");

        for forbidden in [
            "fs::write(",
            "fs::create_dir",
            "fs::remove",
            "fs::rename",
            "File::create",
            "OpenOptions",
        ] {
            assert!(
                !production_source.contains(forbidden),
                "read-only profile validation must not call {forbidden}"
            );
        }
    }
}
