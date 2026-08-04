use std::{
    collections::HashSet,
    fs,
    io::ErrorKind,
    path::{Path, PathBuf},
};

use crate::{
    models::{
        GameInstallationCandidate, GameInstallationCandidateConfidence,
        GameInstallationCandidateDetectionResult, GameInstallationCandidateRoot,
        GameOperatingEnvironment,
    },
    platform::{current_platform, PlatformId},
};

pub fn detect_game_installation_candidates() -> GameInstallationCandidateDetectionResult {
    detect_game_installation_candidates_for_platform(
        current_platform(),
        dirs::document_dir(),
        dirs::home_dir(),
        dirs::download_dir(),
    )
}

fn detect_game_installation_candidates_for_platform(
    platform: PlatformId,
    documents_dir: Option<PathBuf>,
    home_dir: Option<PathBuf>,
    downloads_dir: Option<PathBuf>,
) -> GameInstallationCandidateDetectionResult {
    let current_environment = match platform {
        PlatformId::Windows => GameOperatingEnvironment::NativeWindows,
        PlatformId::Macos => GameOperatingEnvironment::NativeMacos,
        PlatformId::Linux => GameOperatingEnvironment::NativeLinux,
    };
    let supported = platform == PlatformId::Macos;
    let (candidates, review_notes) = if supported {
        detect_macos_sims4_candidates(documents_dir, home_dir, downloads_dir)
    } else {
        (
            Vec::new(),
            vec![
                "Automatic Sims 4 folder suggestions are not available on this operating system yet. Use the guarded manual folder chooser."
                    .to_owned(),
            ],
        )
    };

    GameInstallationCandidateDetectionResult {
        current_environment,
        supported,
        candidates,
        read_only: true,
        review_notes,
    }
}

fn detect_macos_sims4_candidates(
    documents_dir: Option<PathBuf>,
    home_dir: Option<PathBuf>,
    downloads_dir: Option<PathBuf>,
) -> (Vec<GameInstallationCandidate>, Vec<String>) {
    let mut sources = Vec::new();
    if let Some(documents_dir) = documents_dir {
        sources.push((
            "macos_documents_directory",
            "Sims 4 in macOS Documents",
            documents_dir.join("Electronic Arts").join("The Sims 4"),
            0_usize,
        ));
    }
    if let Some(home_dir) = home_dir {
        sources.push((
            "macos_home_documents_fallback",
            "Sims 4 in home Documents",
            home_dir
                .join("Documents")
                .join("Electronic Arts")
                .join("The Sims 4"),
            1_usize,
        ));
    }

    let downloads_dir = downloads_dir.filter(|path| path.is_dir());
    let mut seen_roots = HashSet::new();
    let mut candidates = Vec::new();
    let mut review_notes = Vec::new();
    for (source_code, suggested_name, user_data_path, source_priority) in sources {
        match fs::metadata(&user_data_path) {
            Ok(metadata) if metadata.is_dir() => {}
            Ok(_) => continue,
            Err(error) if error.kind() == ErrorKind::NotFound => continue,
            Err(error) => {
                review_notes.push(format!(
                    "SimSuite could not inspect '{}' at '{}': {}",
                    suggested_name,
                    path_to_string(&user_data_path),
                    error
                ));
                continue;
            }
        }

        let identity = canonical_or_original(&user_data_path);
        if !seen_roots.insert(identity) {
            continue;
        }

        candidates.push((
            build_macos_candidate(
                source_code,
                suggested_name,
                user_data_path,
                downloads_dir.as_deref(),
            ),
            source_priority,
        ));
    }

    candidates.sort_by_key(|(candidate, source_priority)| {
        let confidence_priority = match candidate.confidence {
            GameInstallationCandidateConfidence::Strong => 0_usize,
            GameInstallationCandidateConfidence::Possible => 1_usize,
        };
        (confidence_priority, *source_priority)
    });
    let candidates = candidates
        .into_iter()
        .enumerate()
        .map(|(index, (mut candidate, _))| {
            candidate.rank = index + 1;
            candidate
        })
        .collect();
    (candidates, review_notes)
}

fn build_macos_candidate(
    source_code: &str,
    suggested_name: &str,
    user_data_path: PathBuf,
    downloads_dir: Option<&Path>,
) -> GameInstallationCandidate {
    let mods_path = user_data_path.join("Mods");
    let tray_path = user_data_path.join("Tray");
    let mods_exists = mods_path.is_dir();
    let tray_exists = tray_path.is_dir();
    let confidence = if mods_exists && tray_exists {
        GameInstallationCandidateConfidence::Strong
    } else {
        GameInstallationCandidateConfidence::Possible
    };

    let mut evidence = vec![
        "The macOS Documents location resolved successfully.".to_owned(),
        "An existing Sims 4 user-data directory was found.".to_owned(),
    ];
    let mut warnings = Vec::new();
    if mods_exists {
        evidence.push("An existing Mods directory was found inside this setup.".to_owned());
    } else {
        warnings.push(
            "The expected Mods directory was not found. This suggestion cannot be confirmed until a valid Mods root is chosen."
                .to_owned(),
        );
    }
    if tray_exists {
        evidence.push("An existing Tray directory was found inside this setup.".to_owned());
    } else {
        warnings.push(
            "The expected Tray directory was not found. This suggestion cannot be confirmed until a valid Tray root is chosen."
                .to_owned(),
        );
    }

    if fs::symlink_metadata(&user_data_path)
        .map(|metadata| metadata.file_type().is_symlink())
        .unwrap_or(false)
    {
        warnings.push(
            "The user-data path is a symbolic link or alias. SimSuite will require its normal live validation before confirmation."
                .to_owned(),
        );
    }

    let mut suggested_roots = vec![
        GameInstallationCandidateRoot {
            root_id: "user_data".to_owned(),
            root_role: "game_user_data".to_owned(),
            configured_path: path_to_string(&user_data_path),
            required: false,
            exists: true,
        },
        GameInstallationCandidateRoot {
            root_id: "mods".to_owned(),
            root_role: "installed_mods".to_owned(),
            configured_path: path_to_string(&mods_path),
            required: true,
            exists: mods_exists,
        },
        GameInstallationCandidateRoot {
            root_id: "tray".to_owned(),
            root_role: "installed_tray".to_owned(),
            configured_path: path_to_string(&tray_path),
            required: true,
            exists: tray_exists,
        },
    ];
    if let Some(downloads_dir) = downloads_dir {
        suggested_roots.push(GameInstallationCandidateRoot {
            root_id: "downloads".to_owned(),
            root_role: "intake_downloads".to_owned(),
            configured_path: path_to_string(downloads_dir),
            required: false,
            exists: true,
        });
        evidence.push("An existing macOS Downloads directory is available as optional intake.".to_owned());
    }

    GameInstallationCandidate {
        candidate_id: source_code.to_owned(),
        rank: 0,
        game_id: "sims4".to_owned(),
        operating_environment: GameOperatingEnvironment::NativeMacos,
        suggested_name: suggested_name.to_owned(),
        suggested_roots,
        confidence,
        detection_evidence: evidence,
        warnings,
        read_only: true,
    }
}

fn canonical_or_original(path: &Path) -> PathBuf {
    fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

fn path_to_string(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

#[cfg(test)]
mod tests {
    use std::{fs, path::Path};

    use tempfile::TempDir;

    use super::*;

    fn create_directory(path: &Path) {
        fs::create_dir_all(path).expect("create test directory");
    }

    #[test]
    fn complete_macos_layout_returns_one_strong_ranked_candidate() {
        let temp = TempDir::new().expect("temp dir");
        let documents = temp.path().join("DocumentsResolved");
        let user_data = documents.join("Electronic Arts").join("The Sims 4");
        create_directory(&user_data.join("Mods"));
        create_directory(&user_data.join("Tray"));
        let downloads = temp.path().join("Downloads");
        create_directory(&downloads);

        let result = detect_game_installation_candidates_for_platform(
            PlatformId::Macos,
            Some(documents),
            None,
            Some(downloads),
        );

        assert!(result.supported);
        assert!(result.read_only);
        assert_eq!(result.current_environment, GameOperatingEnvironment::NativeMacos);
        assert_eq!(result.candidates.len(), 1);
        let candidate = &result.candidates[0];
        assert_eq!(candidate.rank, 1);
        assert_eq!(candidate.confidence, GameInstallationCandidateConfidence::Strong);
        assert!(candidate.read_only);
        assert_eq!(candidate.operating_environment, GameOperatingEnvironment::NativeMacos);
        assert_eq!(candidate.suggested_roots.len(), 4);
        assert!(candidate.warnings.is_empty());
        assert!(candidate
            .suggested_roots
            .iter()
            .filter(|root| root.required)
            .all(|root| root.exists));
    }

    #[test]
    fn incomplete_macos_layout_stays_possible_and_reports_missing_required_root() {
        let temp = TempDir::new().expect("temp dir");
        let documents = temp.path().join("DocumentsResolved");
        let user_data = documents.join("Electronic Arts").join("The Sims 4");
        create_directory(&user_data.join("Mods"));

        let result = detect_game_installation_candidates_for_platform(
            PlatformId::Macos,
            Some(documents),
            None,
            None,
        );

        assert!(result.supported);
        assert_eq!(result.candidates.len(), 1);
        let candidate = &result.candidates[0];
        assert_eq!(candidate.confidence, GameInstallationCandidateConfidence::Possible);
        assert!(candidate.warnings.iter().any(|warning| warning.contains("Tray")));
        let tray = candidate
            .suggested_roots
            .iter()
            .find(|root| root.root_id == "tray")
            .expect("tray root");
        assert!(!tray.exists);
    }

    #[test]
    fn equivalent_documents_sources_are_deduplicated() {
        let temp = TempDir::new().expect("temp dir");
        let home = temp.path();
        let documents = home.join("Documents");
        let user_data = documents.join("Electronic Arts").join("The Sims 4");
        create_directory(&user_data.join("Mods"));
        create_directory(&user_data.join("Tray"));

        let result = detect_game_installation_candidates_for_platform(
            PlatformId::Macos,
            Some(documents),
            Some(home.to_path_buf()),
            None,
        );

        assert_eq!(result.candidates.len(), 1);
        assert_eq!(
            result.candidates[0].candidate_id,
            "macos_documents_directory"
        );
    }

    #[test]
    fn non_macos_hosts_return_no_native_macos_candidates() {
        let temp = TempDir::new().expect("temp dir");
        let documents = temp.path().join("Documents");
        create_directory(
            &documents
                .join("Electronic Arts")
                .join("The Sims 4")
                .join("Mods"),
        );

        for platform in [PlatformId::Windows, PlatformId::Linux] {
            let result = detect_game_installation_candidates_for_platform(
                platform,
                Some(documents.clone()),
                None,
                None,
            );
            assert!(!result.supported);
            assert!(result.read_only);
            assert!(result.candidates.is_empty());
            assert_eq!(result.review_notes.len(), 1);
            let expected_environment = match platform {
                PlatformId::Windows => GameOperatingEnvironment::NativeWindows,
                PlatformId::Linux => GameOperatingEnvironment::NativeLinux,
                PlatformId::Macos => unreachable!(),
            };
            assert_eq!(result.current_environment, expected_environment);
        }
    }

    #[test]
    fn production_candidate_detection_contains_no_file_mutation_calls() {
        let source = include_str!("game_installation_candidate_detection.rs");
        let production_source = source
            .split("#[cfg(test)]")
            .next()
            .expect("production source");
        for forbidden in [
            "fs::write",
            "fs::copy",
            "fs::rename",
            "fs::remove",
            "fs::create_dir",
            "File::create",
        ] {
            assert!(
                !production_source.contains(forbidden),
                "candidate detection must not call {forbidden}"
            );
        }
    }
}
