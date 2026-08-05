use std::{
    collections::HashSet,
    io::ErrorKind,
    path::{Path, PathBuf},
};

use rusqlite::{params, Connection};

use crate::{
    core::sims4_adapter::{
        SIMS4_MAX_PACKAGE_FOLDER_DEPTH, SIMS4_MAX_SCRIPT_FOLDER_DEPTH,
    },
    error::AppResult,
    models::LibrarySettings,
    platform::path_semantics::{comparison_key, probe_case_sensitivity, CaseSensitivity},
};

#[derive(Debug, Clone)]
pub struct ValidationRequest {
    pub file_id: i64,
    pub filename: String,
    pub extension: String,
    pub kind: String,
    pub subtype: Option<String>,
    pub creator: Option<String>,
    pub bundle_name: Option<String>,
    pub source_location: String,
    pub confidence: f64,
    pub suggested_relative_path: String,
    pub guided_install: bool,
    pub allow_existing_target: bool,
}

#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub final_relative_path: String,
    pub final_absolute_path: Option<String>,
    pub notes: Vec<String>,
    pub corrected: bool,
    pub review_required: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TargetPathState {
    Missing,
    Occupied,
    Unreadable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DestinationRootRole {
    Mods,
    Tray,
}

impl DestinationRootRole {
    fn for_kind(kind: &str) -> Self {
        if kind.starts_with("Tray") {
            Self::Tray
        } else {
            Self::Mods
        }
    }
}

#[derive(Debug, Clone)]
struct DestinationRootEvidence {
    role: DestinationRootRole,
    configured_root_key: Option<String>,
    canonical_root: Option<PathBuf>,
    case_sensitivity: CaseSensitivity,
}

impl DestinationRootEvidence {
    fn from_configured(role: DestinationRootRole, configured: Option<&str>) -> Self {
        let configured = configured
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(PathBuf::from);
        let configured_root_key = configured.as_deref().map(conservative_root_key);
        let canonical_root = configured
            .as_deref()
            .filter(|path| path.is_absolute())
            .and_then(|path| std::fs::canonicalize(path).ok())
            .filter(|path| {
                std::fs::metadata(path)
                    .map(|metadata| metadata.is_dir())
                    .unwrap_or(false)
            });
        let case_sensitivity = canonical_root
            .as_deref()
            .and_then(|root| probe_case_sensitivity(root).ok())
            .unwrap_or(CaseSensitivity::Unknown);

        Self {
            role,
            configured_root_key,
            canonical_root,
            case_sensitivity,
        }
    }
}

#[derive(Debug, Clone)]
struct DestinationPathKeys {
    sensitive: String,
    folded: String,
}

#[derive(Debug, Clone)]
struct DestinationRootReservations {
    evidence: DestinationRootEvidence,
    sensitive_paths: HashSet<String>,
    folded_paths: HashSet<String>,
}

impl DestinationRootReservations {
    fn new(evidence: DestinationRootEvidence) -> Self {
        Self {
            evidence,
            sensitive_paths: HashSet::new(),
            folded_paths: HashSet::new(),
        }
    }

    fn reserve(&mut self, keys: DestinationPathKeys) {
        self.sensitive_paths.insert(keys.sensitive);
        self.folded_paths.insert(keys.folded);
    }

    fn collides_with(
        &self,
        candidate_root: &DestinationRootEvidence,
        keys: &DestinationPathKeys,
    ) -> bool {
        if !destination_roots_may_match(&self.evidence, candidate_root) {
            return false;
        }
        if self.evidence.case_sensitivity == CaseSensitivity::Sensitive
            && candidate_root.case_sensitivity == CaseSensitivity::Sensitive
        {
            self.sensitive_paths.contains(&keys.sensitive)
        } else {
            self.folded_paths.contains(&keys.folded)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DestinationReservationStatus {
    Available,
    Collision,
    IdentityUnavailable,
}

#[derive(Debug, Clone)]
pub struct DestinationReservations {
    mods_root: DestinationRootReservations,
    tray_root: DestinationRootReservations,
}

impl Default for DestinationReservations {
    fn default() -> Self {
        Self::new(&LibrarySettings::default())
    }
}

impl DestinationReservations {
    pub fn new(settings: &LibrarySettings) -> Self {
        Self {
            mods_root: DestinationRootReservations::new(
                DestinationRootEvidence::from_configured(
                    DestinationRootRole::Mods,
                    settings.mods_path.as_deref(),
                ),
            ),
            tray_root: DestinationRootReservations::new(
                DestinationRootEvidence::from_configured(
                    DestinationRootRole::Tray,
                    settings.tray_path.as_deref(),
                ),
            ),
        }
    }

    pub fn reserve(&mut self, kind: &str, relative_path: &str) -> bool {
        let Some(keys) = destination_path_keys(relative_path) else {
            return false;
        };
        match DestinationRootRole::for_kind(kind) {
            DestinationRootRole::Mods => self.mods_root.reserve(keys),
            DestinationRootRole::Tray => self.tray_root.reserve(keys),
        }
        true
    }

    fn status_for(&self, kind: &str, relative_path: &str) -> DestinationReservationStatus {
        let Some(keys) = destination_path_keys(relative_path) else {
            return DestinationReservationStatus::IdentityUnavailable;
        };
        let role = DestinationRootRole::for_kind(kind);
        let candidate_root = match role {
            DestinationRootRole::Mods => &self.mods_root.evidence,
            DestinationRootRole::Tray => &self.tray_root.evidence,
        };
        if self.mods_root.collides_with(candidate_root, &keys)
            || self.tray_root.collides_with(candidate_root, &keys)
        {
            DestinationReservationStatus::Collision
        } else {
            DestinationReservationStatus::Available
        }
    }
}

fn destination_path_keys(relative_path: &str) -> Option<DestinationPathKeys> {
    let relative_path = Path::new(relative_path);
    Some(DestinationPathKeys {
        sensitive: comparison_key(relative_path, CaseSensitivity::Sensitive)
            .ok()?
            .storage_key_v1()?,
        folded: comparison_key(relative_path, CaseSensitivity::Insensitive)
            .ok()?
            .storage_key_v1()?,
    })
}

fn conservative_root_key(path: &Path) -> String {
    let normalized = path.to_string_lossy().replace('\\', "/");
    let trimmed = normalized.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        "/".to_owned()
    } else {
        trimmed.to_lowercase()
    }
}

fn destination_roots_may_match(
    left: &DestinationRootEvidence,
    right: &DestinationRootEvidence,
) -> bool {
    if left.role == right.role {
        return true;
    }
    if let (Some(left_root), Some(right_root)) = (&left.canonical_root, &right.canonical_root) {
        return left_root == right_root;
    }
    matches!(
        (&left.configured_root_key, &right.configured_root_key),
        (Some(left_key), Some(right_key)) if left_key == right_key
    )
}

fn target_path_state_from_error_kind(kind: ErrorKind) -> TargetPathState {
    if kind == ErrorKind::NotFound {
        TargetPathState::Missing
    } else {
        TargetPathState::Unreadable
    }
}

fn target_path_state(path: &Path) -> TargetPathState {
    match std::fs::symlink_metadata(path) {
        Ok(_) => TargetPathState::Occupied,
        Err(error) => target_path_state_from_error_kind(error.kind()),
    }
}

pub fn validate_suggestion(
    connection: &Connection,
    settings: &LibrarySettings,
    request: &ValidationRequest,
    reservations: &DestinationReservations,
) -> AppResult<ValidationResult> {
    let mut final_relative = PathBuf::from(&request.suggested_relative_path);
    let mut notes = Vec::new();

    if request.kind.starts_with("Tray") {
        let bundle_folder = match request.kind.as_str() {
            "TrayHousehold" => "Households",
            "TrayLot" => "Lots",
            "TrayRoom" => "Rooms",
            _ => "Tray",
        };
        let bundle_name = sanitize_component(
            request
                .bundle_name
                .as_deref()
                .unwrap_or_else(|| file_stem(&request.filename)),
            "Bundle",
        );
        final_relative = PathBuf::from(bundle_folder)
            .join(bundle_name)
            .join(&request.filename);
        if final_relative != PathBuf::from(&request.suggested_relative_path) {
            notes.push("validator_routed_tray_content_to_tray_root".to_owned());
        }
    }

    if request.extension == ".ts4script" && !request.guided_install {
        let creator = request
            .creator
            .as_deref()
            .map(|value| sanitize_component(value, "Unknown"))
            .filter(|value| !value.is_empty());
        final_relative = match creator {
            Some(creator) => PathBuf::from("ScriptMods")
                .join(creator)
                .join(&request.filename),
            None => PathBuf::from("ScriptMods").join(&request.filename),
        };
        if final_relative != PathBuf::from(&request.suggested_relative_path) {
            notes.push("validator_flattened_script_depth".to_owned());
        }
    }

    let folder_depth = final_relative
        .parent()
        .map(|parent| parent.components().count())
        .unwrap_or(0);
    if request.extension == ".ts4script"
        && request.guided_install
        && folder_depth > SIMS4_MAX_SCRIPT_FOLDER_DEPTH
    {
        notes.push("guided_script_depth_requires_review".to_owned());
    }
    if request.extension == ".package" && folder_depth > SIMS4_MAX_PACKAGE_FOLDER_DEPTH {
        final_relative = package_fallback_path(request);
        notes.push("validator_limited_package_depth".to_owned());
    }

    if request.kind == "Unknown" {
        notes.push("unknown_kind_requires_review".to_owned());
    }

    if request.confidence < 0.55 {
        notes.push("low_confidence_requires_review".to_owned());
    }

    if request.kind.starts_with("Tray") && request.source_location == "mods" {
        notes.push("tray_file_will_be_relocated_from_mods".to_owned());
    }

    let target_root = if request.kind.starts_with("Tray") {
        settings.tray_path.as_deref()
    } else {
        settings.mods_path.as_deref()
    };

    let final_relative_path = normalize_relative_path(&final_relative);
    let final_absolute_path = target_root.map(|root| {
        Path::new(root)
            .join(&final_relative)
            .to_string_lossy()
            .to_string()
    });

    if final_absolute_path.is_none() {
        notes.push("missing_target_root".to_owned());
    }

    if let Some(path) = &final_absolute_path {
        match reservations.status_for(&request.kind, &final_relative_path) {
            DestinationReservationStatus::Collision => {
                notes.push("preview_path_collision_detected".to_owned());
            }
            DestinationReservationStatus::IdentityUnavailable => {
                notes.push("preview_path_identity_unavailable".to_owned());
            }
            DestinationReservationStatus::Available => {}
        }

        match target_path_state(Path::new(path)) {
            TargetPathState::Unreadable => {
                notes.push("existing_path_collision_detected".to_owned());
            }
            TargetPathState::Occupied if !request.allow_existing_target => {
                let current_file_owns_target: bool = connection.query_row(
                    "SELECT EXISTS(SELECT 1 FROM files WHERE id = ?1 AND path = ?2)",
                    params![request.file_id, path],
                    |row| row.get(0),
                )?;
                let another_file_owns_target: bool = connection.query_row(
                    "SELECT EXISTS(SELECT 1 FROM files WHERE path = ?1 AND id <> ?2)",
                    params![path, request.file_id],
                    |row| row.get(0),
                )?;

                if !current_file_owns_target || another_file_owns_target {
                    notes.push("existing_path_collision_detected".to_owned());
                }
            }
            TargetPathState::Missing | TargetPathState::Occupied => {}
        }
    }

    let review_required = notes.iter().any(|note| {
        matches!(
            note.as_str(),
            "unknown_kind_requires_review"
                | "low_confidence_requires_review"
                | "missing_target_root"
                | "preview_path_collision_detected"
                | "preview_path_identity_unavailable"
                | "existing_path_collision_detected"
                | "guided_script_depth_requires_review"
        )
    });

    Ok(ValidationResult {
        final_relative_path,
        final_absolute_path,
        corrected: final_relative != PathBuf::from(&request.suggested_relative_path),
        review_required,
        notes,
    })
}

fn package_fallback_path(request: &ValidationRequest) -> PathBuf {
    let mut path = PathBuf::new();
    path.push(sanitize_component(&request.kind, "Mods"));
    if let Some(subtype) = request.subtype.as_deref() {
        path.push(sanitize_component(subtype, "Misc"));
    }
    if let Some(creator) = request.creator.as_deref() {
        path.push(sanitize_component(creator, "Unknown"));
    }
    path.join(&request.filename)
}

fn sanitize_component(value: &str, fallback: &str) -> String {
    let cleaned = value
        .chars()
        .map(|character| {
            if matches!(
                character,
                '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*'
            ) {
                '_'
            } else {
                character
            }
        })
        .collect::<String>()
        .trim()
        .replace('.', "_");

    if cleaned.is_empty() {
        fallback.to_owned()
    } else {
        cleaned
    }
}

fn file_stem(filename: &str) -> &str {
    filename
        .rsplit_once('.')
        .map(|(stem, _)| stem)
        .unwrap_or(filename)
}

fn normalize_relative_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use std::{io::ErrorKind, path::PathBuf};

    use tempfile::tempdir;

    use crate::{database, models::LibrarySettings};

    use super::{
        target_path_state_from_error_kind, validate_suggestion, CaseSensitivity,
        DestinationReservationStatus, DestinationReservations, DestinationRootEvidence,
        DestinationRootReservations, DestinationRootRole, TargetPathState, ValidationRequest,
    };

    #[test]
    fn validator_treats_unreadable_destination_metadata_as_occupied_risk() {
        assert_eq!(
            target_path_state_from_error_kind(ErrorKind::NotFound),
            TargetPathState::Missing
        );
        assert_eq!(
            target_path_state_from_error_kind(ErrorKind::PermissionDenied),
            TargetPathState::Unreadable
        );
        assert_eq!(
            target_path_state_from_error_kind(ErrorKind::InvalidData),
            TargetPathState::Unreadable
        );
    }

    #[test]
    fn validator_flattens_script_mod_destinations() {
        let mut connection = rusqlite::Connection::open_in_memory().expect("in-memory db");
        database::initialize(&mut connection).expect("schema");

        let result = validate_suggestion(
            &connection,
            &LibrarySettings {
                mods_path: Some("C:/Mods".to_owned()),
                tray_path: Some("C:/Tray".to_owned()),
                downloads_path: None,
                ..Default::default()
            },
            &ValidationRequest {
                file_id: 1,
                filename: "mc_cmd_center.ts4script".to_owned(),
                extension: ".ts4script".to_owned(),
                kind: "ScriptMods".to_owned(),
                subtype: None,
                creator: Some("Deaderpool".to_owned()),
                bundle_name: None,
                source_location: "mods".to_owned(),
                confidence: 0.95,
                suggested_relative_path: "Gameplay/Deaderpool/MCCC/mc_cmd_center.ts4script"
                    .to_owned(),
                guided_install: false,
                allow_existing_target: false,
            },
            &DestinationReservations::default(),
        )
        .expect("validated");

        assert_eq!(
            result.final_relative_path,
            "ScriptMods/Deaderpool/mc_cmd_center.ts4script"
        );
        assert!(result
            .notes
            .contains(&"validator_flattened_script_depth".to_owned()));
    }

    #[test]
    fn validator_marks_collisions_for_review() {
        let temp = tempdir().expect("tempdir");
        let mods_root = temp.path().join("Mods");
        std::fs::create_dir_all(&mods_root).expect("mods");
        let existing_path = mods_root.join("CAS/Hair/Simstrouble/Breezy.package");
        std::fs::create_dir_all(existing_path.parent().expect("existing parent"))
            .expect("existing parent directory");
        std::fs::write(&existing_path, b"occupied").expect("existing target");

        let mut connection = rusqlite::Connection::open_in_memory().expect("in-memory db");
        database::initialize(&mut connection).expect("schema");
        connection
            .execute(
                "INSERT INTO files (path, filename, extension, kind, subtype, confidence, source_location)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                rusqlite::params![
                    existing_path.to_string_lossy(),
                    "Breezy.package",
                    ".package",
                    "CAS",
                    "Hair",
                    0.8_f64,
                    "mods"
                ],
            )
            .expect("seed file");

        let result = validate_suggestion(
            &connection,
            &LibrarySettings {
                mods_path: Some(mods_root.to_string_lossy().to_string()),
                tray_path: Some("C:/Tray".to_owned()),
                downloads_path: None,
                ..Default::default()
            },
            &ValidationRequest {
                file_id: 2,
                filename: "Breezy.package".to_owned(),
                extension: ".package".to_owned(),
                kind: "CAS".to_owned(),
                subtype: Some("Hair".to_owned()),
                creator: Some("Simstrouble".to_owned()),
                bundle_name: None,
                source_location: "mods".to_owned(),
                confidence: 0.8,
                suggested_relative_path: "CAS/Hair/Simstrouble/Breezy.package".to_owned(),
                guided_install: false,
                allow_existing_target: false,
            },
            &DestinationReservations::default(),
        )
        .expect("validated");

        assert!(result.review_required);
        assert!(result
            .notes
            .contains(&"existing_path_collision_detected".to_owned()));
    }

    #[test]
    fn validator_distinguishes_unindexed_targets_from_the_current_file() {
        let temp = tempdir().expect("tempdir");
        let mods_root = temp.path().join("Mods");
        let target_path = mods_root.join("CAS/Hair/Simstrouble/Breezy.package");
        std::fs::create_dir_all(target_path.parent().expect("target parent"))
            .expect("target parent directory");
        std::fs::write(&target_path, b"occupied").expect("target file");

        let mut connection = rusqlite::Connection::open_in_memory().expect("in-memory db");
        database::initialize(&mut connection).expect("schema");
        let request = ValidationRequest {
            file_id: 99,
            filename: "Breezy.package".to_owned(),
            extension: ".package".to_owned(),
            kind: "CAS".to_owned(),
            subtype: Some("Hair".to_owned()),
            creator: Some("Simstrouble".to_owned()),
            bundle_name: None,
            source_location: "mods".to_owned(),
            confidence: 0.8,
            suggested_relative_path: "CAS/Hair/Simstrouble/Breezy.package".to_owned(),
            guided_install: false,
            allow_existing_target: false,
        };
        let settings = LibrarySettings {
            mods_path: Some(mods_root.to_string_lossy().to_string()),
            tray_path: Some(temp.path().join("Tray").to_string_lossy().to_string()),
            downloads_path: None,
            ..Default::default()
        };

        let unindexed_result = validate_suggestion(
            &connection,
            &settings,
            &request,
            &DestinationReservations::new(&settings),
        )
        .expect("unindexed target validation");
        assert!(unindexed_result.review_required);
        assert!(unindexed_result
            .notes
            .contains(&"existing_path_collision_detected".to_owned()));

        connection
            .execute(
                "INSERT INTO files (id, path, filename, extension, kind, subtype, confidence, source_location)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                rusqlite::params![
                    request.file_id,
                    target_path.to_string_lossy(),
                    request.filename,
                    request.extension,
                    request.kind,
                    request.subtype,
                    request.confidence,
                    request.source_location
                ],
            )
            .expect("current file row");

        let current_file_result = validate_suggestion(
            &connection,
            &settings,
            &request,
            &DestinationReservations::new(&settings),
        )
        .expect("current file validation");
        assert!(!current_file_result.review_required);
        assert!(!current_file_result
            .notes
            .contains(&"existing_path_collision_detected".to_owned()));
    }

    #[test]
    fn validator_ignores_stale_index_rows_when_target_file_is_missing() {
        let temp = tempdir().expect("tempdir");
        let mods_root = temp.path().join("Mods");
        std::fs::create_dir_all(&mods_root).expect("mods");
        let stale_path = mods_root.join("MCCC/mc_cmd_center.ts4script");

        let mut connection = rusqlite::Connection::open_in_memory().expect("in-memory db");
        database::initialize(&mut connection).expect("schema");
        connection
            .execute(
                "INSERT INTO files (path, filename, extension, kind, subtype, confidence, source_location)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                rusqlite::params![
                    stale_path.to_string_lossy(),
                    "mc_cmd_center.ts4script",
                    ".ts4script",
                    "Script Mods",
                    "Utility",
                    0.97_f64,
                    "mods"
                ],
            )
            .expect("stale index row");

        let result = validate_suggestion(
            &connection,
            &LibrarySettings {
                mods_path: Some(mods_root.to_string_lossy().to_string()),
                tray_path: Some(temp.path().join("Tray").to_string_lossy().to_string()),
                downloads_path: None,
                ..Default::default()
            },
            &ValidationRequest {
                file_id: 99,
                filename: "mc_cmd_center.ts4script".to_owned(),
                extension: ".ts4script".to_owned(),
                kind: "ScriptMods".to_owned(),
                subtype: Some("Utility".to_owned()),
                creator: Some("Deaderpool".to_owned()),
                bundle_name: None,
                source_location: "downloads".to_owned(),
                confidence: 0.97,
                suggested_relative_path: "MCCC/mc_cmd_center.ts4script".to_owned(),
                guided_install: true,
                allow_existing_target: false,
            },
            &DestinationReservations::default(),
        )
        .expect("validated");

        assert!(!stale_path.exists());
        assert!(!result.review_required);
        assert!(!result
            .notes
            .contains(&"existing_path_collision_detected".to_owned()));
    }

    #[test]
    fn validator_keeps_guided_script_paths_when_depth_is_safe() {
        let mut connection = rusqlite::Connection::open_in_memory().expect("in-memory db");
        database::initialize(&mut connection).expect("schema");

        let result = validate_suggestion(
            &connection,
            &LibrarySettings {
                mods_path: Some("C:/Mods".to_owned()),
                tray_path: Some("C:/Tray".to_owned()),
                downloads_path: None,
                ..Default::default()
            },
            &ValidationRequest {
                file_id: 7,
                filename: "mc_cmd_center.ts4script".to_owned(),
                extension: ".ts4script".to_owned(),
                kind: "ScriptMods".to_owned(),
                subtype: Some("Utility".to_owned()),
                creator: Some("Deaderpool".to_owned()),
                bundle_name: None,
                source_location: "downloads".to_owned(),
                confidence: 0.97,
                suggested_relative_path: "MCCC/mc_cmd_center.ts4script".to_owned(),
                guided_install: true,
                allow_existing_target: true,
            },
            &DestinationReservations::default(),
        )
        .expect("validated");

        assert_eq!(result.final_relative_path, "MCCC/mc_cmd_center.ts4script");
        assert!(!result.review_required);
        assert!(!result
            .notes
            .contains(&"validator_flattened_script_depth".to_owned()));
    }

    fn test_root(
        role: DestinationRootRole,
        canonical_root: &str,
        case_sensitivity: CaseSensitivity,
    ) -> DestinationRootEvidence {
        DestinationRootEvidence {
            role,
            configured_root_key: Some(canonical_root.to_lowercase()),
            canonical_root: Some(PathBuf::from(canonical_root)),
            case_sensitivity,
        }
    }

    fn test_reservations(
        mods_root: &str,
        mods_case: CaseSensitivity,
        tray_root: &str,
        tray_case: CaseSensitivity,
    ) -> DestinationReservations {
        DestinationReservations {
            mods_root: DestinationRootReservations::new(test_root(
                DestinationRootRole::Mods,
                mods_root,
                mods_case,
            )),
            tray_root: DestinationRootReservations::new(test_root(
                DestinationRootRole::Tray,
                tray_root,
                tray_case,
            )),
        }
    }

    #[test]
    fn destination_reservations_preserve_case_distinct_paths_on_sensitive_roots() {
        let mut reservations = test_reservations(
            "/fixture/Mods",
            CaseSensitivity::Sensitive,
            "/fixture/Tray",
            CaseSensitivity::Sensitive,
        );
        assert!(reservations.reserve("CAS", "Creator/Mod.package"));
        assert_eq!(
            reservations.status_for("CAS", "creator/mod.package"),
            DestinationReservationStatus::Available
        );
    }

    #[test]
    fn destination_reservations_block_case_aliases_on_insensitive_roots() {
        let mut reservations = test_reservations(
            "C:/Fixture/Mods",
            CaseSensitivity::Insensitive,
            "C:/Fixture/Tray",
            CaseSensitivity::Insensitive,
        );
        assert!(reservations.reserve("CAS", "Creator/Mod.package"));
        assert_eq!(
            reservations.status_for("CAS", "creator/mod.package"),
            DestinationReservationStatus::Collision
        );
    }

    #[test]
    fn destination_reservations_fail_closed_for_unknown_case_policy() {
        let mut reservations = test_reservations(
            "/fixture/Mods",
            CaseSensitivity::Unknown,
            "/fixture/Tray",
            CaseSensitivity::Unknown,
        );
        assert!(reservations.reserve("CAS", "Creator/Mod.package"));
        assert_eq!(
            reservations.status_for("CAS", "creator/mod.package"),
            DestinationReservationStatus::Collision
        );
    }

    #[test]
    fn destination_reservations_keep_distinct_mods_and_tray_roots_separate() {
        let mut reservations = test_reservations(
            "/fixture/Mods",
            CaseSensitivity::Sensitive,
            "/fixture/Tray",
            CaseSensitivity::Sensitive,
        );
        assert!(reservations.reserve("CAS", "Shared/item.package"));
        assert_eq!(
            reservations.status_for("TrayLot", "Shared/item.package"),
            DestinationReservationStatus::Available
        );
    }

    #[test]
    fn destination_reservations_detect_shared_physical_root_across_roles() {
        let mut reservations = test_reservations(
            "/fixture/Shared",
            CaseSensitivity::Insensitive,
            "/fixture/Shared",
            CaseSensitivity::Insensitive,
        );
        assert!(reservations.reserve("CAS", "Shared/item.package"));
        assert_eq!(
            reservations.status_for("TrayLot", "shared/ITEM.package"),
            DestinationReservationStatus::Collision
        );
    }

    #[test]
    fn destination_reservations_reject_unkeyable_relative_paths() {
        let reservations = test_reservations(
            "/fixture/Mods",
            CaseSensitivity::Sensitive,
            "/fixture/Tray",
            CaseSensitivity::Sensitive,
        );
        assert_eq!(
            reservations.status_for("CAS", "../escape.package"),
            DestinationReservationStatus::IdentityUnavailable
        );
    }

    #[test]
    fn validator_keeps_missing_destination_roots_blocked() {
        let mut connection = rusqlite::Connection::open_in_memory().expect("in-memory db");
        database::initialize(&mut connection).expect("schema");
        let settings = LibrarySettings::default();
        let result = validate_suggestion(
            &connection,
            &settings,
            &ValidationRequest {
                file_id: 1,
                filename: "missing-root.package".to_owned(),
                extension: ".package".to_owned(),
                kind: "CAS".to_owned(),
                subtype: None,
                creator: None,
                bundle_name: None,
                source_location: "downloads".to_owned(),
                confidence: 0.9,
                suggested_relative_path: "CAS/missing-root.package".to_owned(),
                guided_install: false,
                allow_existing_target: false,
            },
            &DestinationReservations::new(&settings),
        )
        .expect("validated");

        assert!(result.review_required);
        assert!(result
            .notes
            .contains(&"missing_target_root".to_owned()));
        assert!(!result
            .notes
            .contains(&"preview_path_identity_unavailable".to_owned()));
    }
}
