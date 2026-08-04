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

#[derive(Debug, Clone)]
struct NativeCandidateSource {
    source_code: String,
    suggested_name: String,
    documents_path: PathBuf,
    source_evidence: String,
    source_priority: usize,
}

#[derive(Debug, Clone)]
struct WindowsOneDriveSource {
    source_code: &'static str,
    suggested_name: &'static str,
    root_path: PathBuf,
    source_priority: usize,
}

pub fn detect_game_installation_candidates() -> GameInstallationCandidateDetectionResult {
    let platform = current_platform();
    let windows_one_drive_sources = if platform == PlatformId::Windows {
        collect_windows_one_drive_sources()
    } else {
        Vec::new()
    };

    detect_game_installation_candidates_for_platform(
        platform,
        dirs::document_dir(),
        dirs::home_dir(),
        dirs::download_dir(),
        windows_one_drive_sources,
    )
}

fn detect_game_installation_candidates_for_platform(
    platform: PlatformId,
    documents_dir: Option<PathBuf>,
    home_dir: Option<PathBuf>,
    downloads_dir: Option<PathBuf>,
    windows_one_drive_sources: Vec<WindowsOneDriveSource>,
) -> GameInstallationCandidateDetectionResult {
    let current_environment = match platform {
        PlatformId::Windows => GameOperatingEnvironment::NativeWindows,
        PlatformId::Macos => GameOperatingEnvironment::NativeMacos,
        PlatformId::Linux => GameOperatingEnvironment::NativeLinux,
    };
    let (supported, candidates, review_notes) = match platform {
        PlatformId::Macos => {
            let (candidates, review_notes) =
                detect_macos_sims4_candidates(documents_dir, home_dir, downloads_dir);
            (true, candidates, review_notes)
        }
        PlatformId::Windows => {
            let (candidates, review_notes) = detect_windows_sims4_candidates(
                documents_dir,
                home_dir,
                downloads_dir,
                windows_one_drive_sources,
            );
            (true, candidates, review_notes)
        }
        PlatformId::Linux => (
            false,
            Vec::new(),
            vec![
                "Automatic Sims 4 folder suggestions are not available for native Linux yet. Use the guarded manual folder chooser. Wine, Proton, and Lutris setups need a separate environment adapter before SimSuite can identify them safely."
                    .to_owned(),
            ],
        ),
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
        sources.push(NativeCandidateSource {
            source_code: "macos_documents_directory".to_owned(),
            suggested_name: "Sims 4 in macOS Documents".to_owned(),
            documents_path: documents_dir,
            source_evidence: "The macOS Documents location resolved successfully.".to_owned(),
            source_priority: 0,
        });
    }
    if let Some(home_dir) = home_dir {
        sources.push(NativeCandidateSource {
            source_code: "macos_home_documents_fallback".to_owned(),
            suggested_name: "Sims 4 in home Documents".to_owned(),
            documents_path: home_dir.join("Documents"),
            source_evidence: "The standard home Documents fallback exists on this Mac."
                .to_owned(),
            source_priority: 10,
        });
    }

    detect_native_sims4_candidates(
        sources,
        downloads_dir,
        GameOperatingEnvironment::NativeMacos,
        "macOS",
    )
}

fn detect_windows_sims4_candidates(
    documents_dir: Option<PathBuf>,
    home_dir: Option<PathBuf>,
    downloads_dir: Option<PathBuf>,
    windows_one_drive_sources: Vec<WindowsOneDriveSource>,
) -> (Vec<GameInstallationCandidate>, Vec<String>) {
    let mut sources = Vec::new();
    if let Some(documents_dir) = documents_dir {
        sources.push(NativeCandidateSource {
            source_code: "windows_configured_documents".to_owned(),
            suggested_name: "Sims 4 in configured Windows Documents".to_owned(),
            documents_path: documents_dir,
            source_evidence:
                "Windows reported this as the configured Documents location, including any operating-system folder redirection."
                    .to_owned(),
            source_priority: 0,
        });
    }

    for source in windows_one_drive_sources {
        if !source.root_path.is_absolute() {
            continue;
        }
        sources.push(NativeCandidateSource {
            source_code: source.source_code.to_owned(),
            suggested_name: source.suggested_name.to_owned(),
            documents_path: source.root_path.join("Documents"),
            source_evidence:
                "Windows exposed this OneDrive root through an explicit OneDrive environment setting."
                    .to_owned(),
            source_priority: source.source_priority,
        });
    }

    if let Some(home_dir) = home_dir {
        sources.push(NativeCandidateSource {
            source_code: "windows_home_documents_fallback".to_owned(),
            suggested_name: "Sims 4 in home Documents".to_owned(),
            documents_path: home_dir.join("Documents"),
            source_evidence:
                "The standard Windows home Documents fallback exists for this user."
                    .to_owned(),
            source_priority: 20,
        });
    }

    detect_native_sims4_candidates(
        sources,
        downloads_dir,
        GameOperatingEnvironment::NativeWindows,
        "Windows",
    )
}

fn collect_windows_one_drive_sources() -> Vec<WindowsOneDriveSource> {
    [
        (
            "OneDrive",
            "windows_onedrive",
            "Sims 4 in OneDrive Documents",
            5_usize,
        ),
        (
            "OneDriveConsumer",
            "windows_onedrive_consumer",
            "Sims 4 in personal OneDrive Documents",
            6_usize,
        ),
        (
            "OneDriveCommercial",
            "windows_onedrive_commercial",
            "Sims 4 in work or school OneDrive Documents",
            7_usize,
        ),
    ]
    .into_iter()
    .filter_map(|(environment_variable, source_code, suggested_name, source_priority)| {
        let value = std::env::var_os(environment_variable)?;
        if value.is_empty() {
            return None;
        }
        Some(WindowsOneDriveSource {
            source_code,
            suggested_name,
            root_path: PathBuf::from(value),
            source_priority,
        })
    })
    .collect()
}

fn detect_native_sims4_candidates(
    sources: Vec<NativeCandidateSource>,
    downloads_dir: Option<PathBuf>,
    operating_environment: GameOperatingEnvironment,
    host_label: &str,
) -> (Vec<GameInstallationCandidate>, Vec<String>) {
    let downloads_dir = downloads_dir.filter(|path| path.is_dir());
    let mut seen_roots = HashSet::new();
    let mut candidates = Vec::new();
    let mut review_notes = Vec::new();

    for source in sources {
        let user_data_path = source
            .documents_path
            .join("Electronic Arts")
            .join("The Sims 4");
        match fs::metadata(&user_data_path) {
            Ok(metadata) if metadata.is_dir() => {}
            Ok(_) => continue,
            Err(error) if error.kind() == ErrorKind::NotFound => continue,
            Err(error) => {
                review_notes.push(format!(
                    "SimSuite could not inspect '{}' at '{}': {}",
                    source.suggested_name,
                    path_to_string(&user_data_path),
                    error
                ));
                continue;
            }
        }

        let identity = candidate_identity(&user_data_path, operating_environment);
        if !seen_roots.insert(identity) {
            continue;
        }

        let source_priority = source.source_priority;
        candidates.push((
            build_native_candidate(
                source,
                user_data_path,
                downloads_dir.as_deref(),
                operating_environment,
                host_label,
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

fn build_native_candidate(
    source: NativeCandidateSource,
    user_data_path: PathBuf,
    downloads_dir: Option<&Path>,
    operating_environment: GameOperatingEnvironment,
    host_label: &str,
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
        source.source_evidence,
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

    if path_is_redirect_or_symlink(&user_data_path) {
        warnings.push(
            "The user-data path is a symbolic link or filesystem redirect. SimSuite will require its normal live validation before confirmation."
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
        evidence.push(format!(
            "An existing {host_label} Downloads directory is available as optional intake."
        ));
    }

    GameInstallationCandidate {
        candidate_id: source.source_code,
        rank: 0,
        game_id: "sims4".to_owned(),
        operating_environment,
        suggested_name: source.suggested_name,
        suggested_roots,
        confidence,
        detection_evidence: evidence,
        warnings,
        read_only: true,
    }
}

fn path_is_redirect_or_symlink(path: &Path) -> bool {
    let Ok(metadata) = fs::symlink_metadata(path) else {
        return false;
    };
    if metadata.file_type().is_symlink() {
        return true;
    }

    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;

        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0400;
        return metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0;
    }

    #[cfg(not(windows))]
    {
        false
    }
}

fn candidate_identity(path: &Path, operating_environment: GameOperatingEnvironment) -> String {
    let canonical_path = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let identity = path_to_string(&canonical_path).replace('\\', "/");
    if operating_environment == GameOperatingEnvironment::NativeWindows {
        identity.to_lowercase()
    } else {
        identity
    }
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

    fn windows_one_drive_source(
        source_code: &'static str,
        root_path: PathBuf,
        source_priority: usize,
    ) -> WindowsOneDriveSource {
        WindowsOneDriveSource {
            source_code,
            suggested_name: "Sims 4 in OneDrive Documents",
            root_path,
            source_priority,
        }
    }

    #[cfg(windows)]
    fn create_windows_junction(link: &Path, target: &Path) {
        use std::process::Command;

        let status = Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(link)
            .arg(target)
            .status()
            .expect("launch mklink");
        assert!(status.success(), "create Windows junction");
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
            Vec::new(),
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
            Vec::new(),
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
    fn equivalent_macos_documents_sources_are_deduplicated() {
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
            Vec::new(),
        );

        assert_eq!(result.candidates.len(), 1);
        assert_eq!(
            result.candidates[0].candidate_id,
            "macos_documents_directory"
        );
    }

    #[test]
    fn configured_windows_documents_returns_a_strong_candidate() {
        let temp = TempDir::new().expect("temp dir");
        let redirected_documents = temp.path().join("Redirected Documents");
        let user_data = redirected_documents
            .join("Electronic Arts")
            .join("The Sims 4");
        create_directory(&user_data.join("Mods"));
        create_directory(&user_data.join("Tray"));

        let result = detect_game_installation_candidates_for_platform(
            PlatformId::Windows,
            Some(redirected_documents),
            None,
            None,
            Vec::new(),
        );

        assert!(result.supported);
        assert_eq!(result.current_environment, GameOperatingEnvironment::NativeWindows);
        assert_eq!(result.candidates.len(), 1);
        let candidate = &result.candidates[0];
        assert_eq!(candidate.rank, 1);
        assert_eq!(candidate.candidate_id, "windows_configured_documents");
        assert_eq!(candidate.confidence, GameInstallationCandidateConfidence::Strong);
        assert!(candidate
            .detection_evidence
            .iter()
            .any(|item| item.contains("folder redirection")));
    }

    #[test]
    fn explicit_onedrive_root_can_supply_a_windows_candidate() {
        let temp = TempDir::new().expect("temp dir");
        let one_drive_root = temp.path().join("OneDrive - Example");
        let user_data = one_drive_root
            .join("Documents")
            .join("Electronic Arts")
            .join("The Sims 4");
        create_directory(&user_data.join("Mods"));
        create_directory(&user_data.join("Tray"));

        let result = detect_game_installation_candidates_for_platform(
            PlatformId::Windows,
            None,
            None,
            None,
            vec![windows_one_drive_source(
                "windows_onedrive_commercial",
                one_drive_root,
                7,
            )],
        );

        assert_eq!(result.candidates.len(), 1);
        let candidate = &result.candidates[0];
        assert_eq!(candidate.candidate_id, "windows_onedrive_commercial");
        assert!(candidate
            .detection_evidence
            .iter()
            .any(|item| item.contains("explicit OneDrive")));
    }

    #[test]
    fn configured_documents_and_onedrive_sources_are_deduplicated() {
        let temp = TempDir::new().expect("temp dir");
        let one_drive_root = temp.path().join("OneDrive");
        let documents = one_drive_root.join("Documents");
        let user_data = documents.join("Electronic Arts").join("The Sims 4");
        create_directory(&user_data.join("Mods"));
        create_directory(&user_data.join("Tray"));

        let result = detect_game_installation_candidates_for_platform(
            PlatformId::Windows,
            Some(documents),
            None,
            None,
            vec![windows_one_drive_source(
                "windows_onedrive",
                one_drive_root,
                5,
            )],
        );

        assert_eq!(result.candidates.len(), 1);
        assert_eq!(
            result.candidates[0].candidate_id,
            "windows_configured_documents"
        );
    }

    #[test]
    fn windows_strong_candidates_rank_before_possible_candidates() {
        let temp = TempDir::new().expect("temp dir");
        let configured_documents = temp.path().join("Configured Documents");
        let configured_user_data = configured_documents
            .join("Electronic Arts")
            .join("The Sims 4");
        create_directory(&configured_user_data.join("Mods"));

        let one_drive_root = temp.path().join("OneDrive");
        let one_drive_user_data = one_drive_root
            .join("Documents")
            .join("Electronic Arts")
            .join("The Sims 4");
        create_directory(&one_drive_user_data.join("Mods"));
        create_directory(&one_drive_user_data.join("Tray"));

        let result = detect_game_installation_candidates_for_platform(
            PlatformId::Windows,
            Some(configured_documents),
            None,
            None,
            vec![windows_one_drive_source(
                "windows_onedrive",
                one_drive_root,
                5,
            )],
        );

        assert_eq!(result.candidates.len(), 2);
        assert_eq!(result.candidates[0].candidate_id, "windows_onedrive");
        assert_eq!(result.candidates[0].confidence, GameInstallationCandidateConfidence::Strong);
        assert_eq!(result.candidates[1].candidate_id, "windows_configured_documents");
        assert_eq!(result.candidates[1].confidence, GameInstallationCandidateConfidence::Possible);
    }

    #[test]
    fn native_linux_remains_manual_only() {
        let temp = TempDir::new().expect("temp dir");
        let documents = temp.path().join("Documents");
        create_directory(
            &documents
                .join("Electronic Arts")
                .join("The Sims 4")
                .join("Mods"),
        );

        let result = detect_game_installation_candidates_for_platform(
            PlatformId::Linux,
            Some(documents),
            None,
            None,
            Vec::new(),
        );

        assert!(!result.supported);
        assert!(result.read_only);
        assert!(result.candidates.is_empty());
        assert_eq!(result.current_environment, GameOperatingEnvironment::NativeLinux);
        assert!(result.review_notes[0].contains("Wine, Proton, and Lutris"));
    }

    #[test]
    fn windows_candidate_identity_is_case_and_separator_insensitive() {
        let upper = candidate_identity(
            Path::new("C:\\Users\\Player\\Documents\\Electronic Arts\\The Sims 4"),
            GameOperatingEnvironment::NativeWindows,
        );
        let lower = candidate_identity(
            Path::new("c:/users/player/documents/electronic arts/the sims 4"),
            GameOperatingEnvironment::NativeWindows,
        );

        assert_eq!(upper, lower);
    }

    #[test]
    fn relative_onedrive_roots_are_ignored() {
        let result = detect_game_installation_candidates_for_platform(
            PlatformId::Windows,
            None,
            None,
            None,
            vec![windows_one_drive_source(
                "windows_onedrive",
                PathBuf::from("relative-onedrive"),
                5,
            )],
        );

        assert!(result.supported);
        assert!(result.candidates.is_empty());
    }

    #[cfg(windows)]
    #[test]
    fn windows_junction_aliases_deduplicate_and_report_redirect() {
        let temp = TempDir::new().expect("temp dir");
        let real_documents = temp.path().join("Real Documents");
        let real_user_data = real_documents
            .join("Electronic Arts")
            .join("The Sims 4");
        create_directory(&real_user_data.join("Mods"));
        create_directory(&real_user_data.join("Tray"));

        let redirected_documents = temp.path().join("Redirected Documents");
        let redirected_ea = redirected_documents.join("Electronic Arts");
        create_directory(&redirected_ea);
        create_windows_junction(&redirected_ea.join("The Sims 4"), &real_user_data);

        let (candidates, review_notes) = detect_native_sims4_candidates(
            vec![
                NativeCandidateSource {
                    source_code: "windows_redirected_documents".to_owned(),
                    suggested_name: "Sims 4 in redirected Documents".to_owned(),
                    documents_path: redirected_documents,
                    source_evidence: "Controlled Windows junction fixture.".to_owned(),
                    source_priority: 0,
                },
                NativeCandidateSource {
                    source_code: "windows_real_documents".to_owned(),
                    suggested_name: "Sims 4 in real Documents".to_owned(),
                    documents_path: real_documents,
                    source_evidence: "Controlled Windows target fixture.".to_owned(),
                    source_priority: 1,
                },
            ],
            None,
            GameOperatingEnvironment::NativeWindows,
            "Windows",
        );

        assert!(review_notes.is_empty());
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].candidate_id, "windows_redirected_documents");
        assert!(candidates[0]
            .warnings
            .iter()
            .any(|warning| warning.contains("filesystem redirect")));
    }

    #[cfg(windows)]
    #[test]
    #[ignore = "native Windows host evidence; run through the dedicated proof wrapper"]
    fn windows_native_candidate_probe() {
        let documents_dir = dirs::document_dir();
        let home_dir = dirs::home_dir();
        let downloads_dir = dirs::download_dir();
        for path in [&documents_dir, &home_dir, &downloads_dir]
            .into_iter()
            .flatten()
        {
            assert!(path.is_absolute(), "Windows known folders must be absolute");
        }

        let result = detect_game_installation_candidates();
        assert_eq!(
            result.current_environment,
            GameOperatingEnvironment::NativeWindows
        );
        assert!(result.supported);
        assert!(result.read_only);
        assert!(result.candidates.iter().all(|candidate| candidate.read_only));

        let one_drive_environment = ["OneDrive", "OneDriveConsumer", "OneDriveCommercial"]
            .into_iter()
            .map(|name| {
                (
                    name,
                    std::env::var_os(name)
                        .map(PathBuf::from)
                        .map(|path| path_to_string(&path)),
                )
            })
            .collect::<std::collections::BTreeMap<_, _>>();
        let receipt = serde_json::json!({
            "schemaVersion": "1.0",
            "proof": "native_windows_game_profile_candidates",
            "configuredDocuments": documents_dir.map(|path| path_to_string(&path)),
            "homeDirectory": home_dir.map(|path| path_to_string(&path)),
            "downloadsDirectory": downloads_dir.map(|path| path_to_string(&path)),
            "oneDriveEnvironment": one_drive_environment,
            "detectorResult": result,
        });
        println!(
            "SIMSUITE_WINDOWS_GAME_PROFILE_PROOF_JSON={}",
            serde_json::to_string(&receipt).expect("serialize proof receipt")
        );
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
