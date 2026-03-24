use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

use rusqlite::{params, Connection, OptionalExtension};

use crate::{error::AppResult, models::LibrarySettings};

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

pub fn validate_suggestion(
    connection: &Connection,
    settings: &LibrarySettings,
    request: &ValidationRequest,
    reserved_targets: &HashSet<String>,
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
    if request.extension == ".ts4script" && request.guided_install && folder_depth > 1 {
        notes.push("guided_script_depth_requires_review".to_owned());
    }
    if request.extension == ".package" && folder_depth > 5 {
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
        if reserved_targets.contains(path) {
            notes.push("preview_path_collision_detected".to_owned());
        }

        if !request.allow_existing_target {
            let existing_owner: Option<i64> = connection
                .query_row(
                    "SELECT id FROM files WHERE path = ?1 AND id <> ?2",
                    params![path, request.file_id],
                    |row| row.get(0),
                )
                .optional()?;
            if existing_owner.is_some() {
                notes.push("existing_path_collision_detected".to_owned());
            }
        }
    }

    let review_required = notes.iter().any(|note| {
        matches!(
            note.as_str(),
            "unknown_kind_requires_review"
                | "low_confidence_requires_review"
                | "missing_target_root"
                | "preview_path_collision_detected"
                | "existing_path_collision_detected"
                | "guided_script_depth_requires_review"
        )
    });

    Ok(ValidationResult {
        final_relative_path: normalize_relative_path(&final_relative),
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
    use std::collections::HashSet;

    use tempfile::tempdir;

    use crate::{database, models::LibrarySettings};

    use super::{validate_suggestion, ValidationRequest};

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
            &HashSet::new(),
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

        let mut connection = rusqlite::Connection::open_in_memory().expect("in-memory db");
        database::initialize(&mut connection).expect("schema");
        connection
            .execute(
                "INSERT INTO files (path, filename, extension, kind, subtype, confidence, source_location)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                rusqlite::params![
                    mods_root.join("CAS/Hair/Simstrouble/Breezy.package").to_string_lossy(),
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
            &HashSet::new(),
        )
        .expect("validated");

        assert!(result.review_required);
        assert!(result
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
            &HashSet::new(),
        )
        .expect("validated");

        assert_eq!(result.final_relative_path, "MCCC/mc_cmd_center.ts4script");
        assert!(!result.review_required);
        assert!(!result
            .notes
            .contains(&"validator_flattened_script_depth".to_owned()));
    }
}
