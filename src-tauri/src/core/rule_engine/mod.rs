use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

use rusqlite::{params, Connection, OptionalExtension};

use crate::{
    core::validator::{validate_suggestion, ValidationRequest},
    error::AppResult,
    models::{
        LibrarySettings, OrganizationPreview, PreviewSuggestion, ReviewQueueItem, RulePreset,
    },
};

#[derive(Debug, Clone)]
struct PreviewCandidate {
    id: i64,
    filename: String,
    path: String,
    extension: String,
    kind: String,
    subtype: Option<String>,
    confidence: f64,
    source_location: String,
    creator: Option<String>,
    bundle_name: Option<String>,
    creator_locked_by_user: bool,
    creator_preferred_path: Option<String>,
}

pub fn list_rule_presets(connection: &Connection) -> AppResult<Vec<RulePreset>> {
    let mut statement = connection.prepare(
        "SELECT rule_name, rule_template, rule_priority
         FROM rules
         WHERE enabled = 1
         ORDER BY rule_priority ASC",
    )?;

    let presets = statement
        .query_map([], |row| {
            let name: String = row.get(0)?;
            Ok(RulePreset {
                template: row.get(1)?,
                priority: row.get(2)?,
                description: preset_description(&name).to_owned(),
                name,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(presets)
}

pub fn build_preview(
    connection: &Connection,
    settings: &LibrarySettings,
    preset_name: Option<String>,
    limit: i64,
) -> AppResult<OrganizationPreview> {
    let preset_name = normalize_preset_name(preset_name);
    let candidates = load_candidates(connection, None, limit)?;
    let detected_structure = detect_structure_label(connection, settings)?;
    let suggestions = suggest_for_candidates(connection, settings, &preset_name, candidates)?;

    Ok(OrganizationPreview {
        preset_name,
        detected_structure,
        total_considered: suggestions.len() as i64,
        corrected_count: suggestions.iter().filter(|item| item.corrected).count() as i64,
        review_count: suggestions
            .iter()
            .filter(|item| item.review_required)
            .count() as i64,
        suggestions,
    })
}

pub fn load_review_queue(
    connection: &Connection,
    settings: &LibrarySettings,
    preset_name: Option<String>,
    limit: i64,
) -> AppResult<Vec<ReviewQueueItem>> {
    let preset_name = normalize_preset_name(preset_name);

    let mut statement = connection.prepare(
        "SELECT
            rq.id,
            rq.file_id,
            rq.reason,
            rq.confidence,
            f.filename,
            f.path,
            f.extension,
            f.kind,
            f.subtype,
            f.source_location,
            c.canonical_name,
            b.bundle_name,
            COALESCE(c.locked_by_user, 0),
            c.preferred_path,
            f.safety_notes
         FROM review_queue rq
         JOIN files f ON rq.file_id = f.id
         LEFT JOIN creators c ON f.creator_id = c.id
         LEFT JOIN bundles b ON f.bundle_id = b.id
         ORDER BY rq.created_at DESC
         LIMIT ?1",
    )?;

    let rows = statement
        .query_map(params![limit], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                PreviewCandidate {
                    id: row.get(1)?,
                    filename: row.get(4)?,
                    path: row.get(5)?,
                    extension: row.get(6)?,
                    kind: row.get(7)?,
                    subtype: row.get(8)?,
                    confidence: row.get(3)?,
                    source_location: row.get(9)?,
                    creator: row.get(10)?,
                    bundle_name: row.get(11)?,
                    creator_locked_by_user: row.get::<_, i64>(12)? != 0,
                    creator_preferred_path: row.get(13)?,
                },
                row.get::<_, String>(2)?,
                parse_string_array(row.get::<_, String>(14)?),
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;

    let candidates = rows
        .iter()
        .map(|(_, candidate, _, _)| candidate.clone())
        .collect();
    let suggestions = suggest_for_candidates(connection, settings, &preset_name, candidates)?;

    let mut mapped = Vec::new();
    for (review_id, candidate, reason, safety_notes) in rows {
        let suggestion = suggestions
            .iter()
            .find(|item| item.file_id == candidate.id)
            .cloned();

        mapped.push(ReviewQueueItem {
            id: review_id,
            file_id: candidate.id,
            filename: candidate.filename,
            path: candidate.path,
            reason,
            confidence: candidate.confidence,
            kind: candidate.kind,
            subtype: candidate.subtype,
            creator: candidate.creator,
            suggested_path: suggestion.and_then(|item| item.final_absolute_path),
            safety_notes,
            source_location: candidate.source_location,
        });
    }

    Ok(mapped)
}

fn suggest_for_candidates(
    connection: &Connection,
    settings: &LibrarySettings,
    preset_name: &str,
    candidates: Vec<PreviewCandidate>,
) -> AppResult<Vec<PreviewSuggestion>> {
    let mut reserved_targets = HashSet::new();
    let mut suggestions = Vec::new();

    for candidate in candidates {
        let rule_relative = preset_relative_path(preset_name, &candidate);
        let validator = validate_suggestion(
            connection,
            settings,
            &ValidationRequest {
                file_id: candidate.id,
                filename: candidate.filename.clone(),
                extension: candidate.extension.clone(),
                kind: candidate.kind.clone(),
                subtype: candidate.subtype.clone(),
                creator: candidate.creator.clone(),
                bundle_name: candidate.bundle_name.clone(),
                source_location: candidate.source_location.clone(),
                confidence: candidate.confidence,
                suggested_relative_path: rule_relative.clone(),
            },
            &reserved_targets,
        )?;

        if let Some(path) = &validator.final_absolute_path {
            reserved_targets.insert(path.clone());
        }

        let suggested_absolute_path = target_root(settings, &candidate.kind).map(|root| {
            Path::new(root)
                .join(&rule_relative)
                .to_string_lossy()
                .to_string()
        });

        suggestions.push(PreviewSuggestion {
            file_id: candidate.id,
            filename: candidate.filename,
            current_path: candidate.path,
            suggested_relative_path: rule_relative,
            suggested_absolute_path,
            final_relative_path: validator.final_relative_path,
            final_absolute_path: validator.final_absolute_path,
            rule_label: preset_name.to_owned(),
            validator_notes: validator.notes,
            review_required: validator.review_required,
            corrected: validator.corrected,
            confidence: candidate.confidence,
            kind: candidate.kind,
            creator: candidate.creator,
            source_location: candidate.source_location,
            bundle_name: candidate.bundle_name,
        });
    }

    Ok(suggestions)
}

fn load_candidates(
    connection: &Connection,
    file_ids: Option<&[i64]>,
    limit: i64,
) -> AppResult<Vec<PreviewCandidate>> {
    if let Some(file_ids) = file_ids {
        let mut results = Vec::new();
        for file_id in file_ids {
            if let Some(candidate) = load_candidate(connection, *file_id)? {
                results.push(candidate);
            }
        }
        return Ok(results);
    }

    let mut statement = connection.prepare(
        "SELECT
            f.id,
            f.filename,
            f.path,
            f.extension,
            f.kind,
            f.subtype,
            f.confidence,
            f.source_location,
            c.canonical_name,
            b.bundle_name,
            COALESCE(c.locked_by_user, 0),
            c.preferred_path
         FROM files f
         LEFT JOIN creators c ON f.creator_id = c.id
         LEFT JOIN bundles b ON f.bundle_id = b.id
         ORDER BY f.modified_at DESC, f.filename COLLATE NOCASE
         LIMIT ?1",
    )?;

    let candidates = statement
        .query_map(params![limit], |row| {
            Ok(PreviewCandidate {
                id: row.get(0)?,
                filename: row.get(1)?,
                path: row.get(2)?,
                extension: row.get(3)?,
                kind: row.get(4)?,
                subtype: row.get(5)?,
                confidence: row.get(6)?,
                source_location: row.get(7)?,
                creator: row.get(8)?,
                bundle_name: row.get(9)?,
                creator_locked_by_user: row.get::<_, i64>(10)? != 0,
                creator_preferred_path: row.get(11)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()
        .map_err(crate::error::AppError::from)?;

    Ok(candidates)
}

fn load_candidate(connection: &Connection, file_id: i64) -> AppResult<Option<PreviewCandidate>> {
    connection
        .query_row(
            "SELECT
                f.id,
                f.filename,
                f.path,
                f.extension,
                f.kind,
                f.subtype,
                f.confidence,
                f.source_location,
                c.canonical_name,
                b.bundle_name,
                COALESCE(c.locked_by_user, 0),
                c.preferred_path
             FROM files f
             LEFT JOIN creators c ON f.creator_id = c.id
             LEFT JOIN bundles b ON f.bundle_id = b.id
             WHERE f.id = ?1",
            params![file_id],
            |row| {
                Ok(PreviewCandidate {
                    id: row.get(0)?,
                    filename: row.get(1)?,
                    path: row.get(2)?,
                    extension: row.get(3)?,
                    kind: row.get(4)?,
                    subtype: row.get(5)?,
                    confidence: row.get(6)?,
                    source_location: row.get(7)?,
                    creator: row.get(8)?,
                    bundle_name: row.get(9)?,
                    creator_locked_by_user: row.get::<_, i64>(10)? != 0,
                    creator_preferred_path: row.get(11)?,
                })
            },
        )
        .optional()
        .map_err(Into::into)
}

fn detect_structure_label(
    connection: &Connection,
    settings: &LibrarySettings,
) -> AppResult<String> {
    let Some(mods_root) = settings.mods_path.as_deref() else {
        return Ok("Mods path not configured yet".to_owned());
    };

    let mut statement = connection.prepare(
        "SELECT path, kind, subtype
         FROM files
         WHERE source_location = 'mods'
         ORDER BY modified_at DESC
         LIMIT 120",
    )?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;

    let mut kind_led = 0;
    let mut subtype_led = 0;
    for (path, kind, subtype) in rows {
        if let Ok(relative) = Path::new(&path).strip_prefix(mods_root) {
            let segments = relative
                .parent()
                .map(|parent| {
                    parent
                        .components()
                        .map(|item| item.as_os_str().to_string_lossy().to_string())
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();

            if let Some(first) = segments.first() {
                if normalize_component(first) == normalize_component(&kind) {
                    kind_led += 1;
                }
            }

            if let Some(second) = segments.get(1) {
                if let Some(subtype) = &subtype {
                    if normalize_component(second) == normalize_component(subtype) {
                        subtype_led += 1;
                    }
                }
            }
        }
    }

    if kind_led > 24 && subtype_led > 12 {
        Ok("Detected a category-first structure with subtype folders".to_owned())
    } else if kind_led > 20 {
        Ok("Detected a category-led structure that can support mirror mode later".to_owned())
    } else {
        Ok("Current library looks mixed; using preset-driven preview mode".to_owned())
    }
}

fn preset_relative_path(preset_name: &str, candidate: &PreviewCandidate) -> String {
    if candidate.creator_locked_by_user {
        if let Some(preferred_path) = candidate
            .creator_preferred_path
            .as_deref()
            .map(|value| value.trim())
            .filter(|value| !value.is_empty())
        {
            return normalize_relative_path(PathBuf::from(preferred_path).join(&candidate.filename));
        }
    }

    let filename = candidate.filename.clone();
    let creator = sanitize_component(candidate.creator.as_deref().unwrap_or("Unknown"), "Unknown");
    let subtype = sanitize_component(candidate.subtype.as_deref().unwrap_or("Misc"), "Misc");
    let kind = sanitize_component(&candidate.kind, "Mods");
    let mod_name = sanitize_component(file_stem(&candidate.filename), "Item");
    let tray_bundle = sanitize_component(
        candidate
            .bundle_name
            .as_deref()
            .unwrap_or(file_stem(&candidate.filename)),
        "Bundle",
    );

    match preset_name {
        "Creator First" => normalize_relative_path(
            PathBuf::from("Creators")
                .join(creator)
                .join(&kind)
                .join(&subtype)
                .join(filename),
        ),
        "Hybrid" => normalize_relative_path(
            PathBuf::from(&kind)
                .join(creator)
                .join(&subtype)
                .join(filename),
        ),
        "Minimal Safe" => minimal_safe_path(candidate, &creator, &subtype, &tray_bundle),
        _ => category_first_path(candidate, &creator, &subtype, &mod_name, &tray_bundle),
    }
}

fn category_first_path(
    candidate: &PreviewCandidate,
    creator: &str,
    subtype: &str,
    mod_name: &str,
    tray_bundle: &str,
) -> String {
    let filename = &candidate.filename;

    let path = match candidate.kind.as_str() {
        "CAS" => PathBuf::from("CAS")
            .join(subtype)
            .join(creator)
            .join(filename),
        "BuildBuy" => PathBuf::from("BuildBuy").join(creator).join(filename),
        "Gameplay" => PathBuf::from("Gameplay")
            .join(creator)
            .join(mod_name)
            .join(filename),
        "ScriptMods" => PathBuf::from("Gameplay")
            .join(creator)
            .join(mod_name)
            .join(filename),
        "OverridesAndDefaults" => PathBuf::from("Overrides").join(creator).join(filename),
        "PosesAndAnimation" => PathBuf::from("Poses").join(creator).join(filename),
        "PresetsAndSliders" => PathBuf::from("Presets").join(subtype).join(filename),
        "TrayHousehold" | "TrayLot" | "TrayRoom" | "TrayItem" => PathBuf::from("ImportedTray")
            .join(tray_bundle)
            .join(filename),
        _ => PathBuf::from("Unknown").join(filename),
    };

    normalize_relative_path(path)
}

fn minimal_safe_path(
    candidate: &PreviewCandidate,
    creator: &str,
    subtype: &str,
    tray_bundle: &str,
) -> String {
    let filename = &candidate.filename;
    let path = match candidate.kind.as_str() {
        "ScriptMods" => PathBuf::from("ScriptMods").join(creator).join(filename),
        "CAS" => PathBuf::from("CAS").join(subtype).join(filename),
        "BuildBuy" => PathBuf::from("BuildBuy").join(filename),
        "Gameplay" => PathBuf::from("Gameplay").join(creator).join(filename),
        "OverridesAndDefaults" => PathBuf::from("Overrides").join(filename),
        "PosesAndAnimation" => PathBuf::from("Poses").join(filename),
        "PresetsAndSliders" => PathBuf::from("Presets").join(filename),
        "TrayHousehold" | "TrayLot" | "TrayRoom" | "TrayItem" => {
            PathBuf::from("Tray").join(tray_bundle).join(filename)
        }
        _ => PathBuf::from("Review").join(filename),
    };

    normalize_relative_path(path)
}

fn target_root<'a>(settings: &'a LibrarySettings, kind: &str) -> Option<&'a str> {
    if kind.starts_with("Tray") {
        settings.tray_path.as_deref()
    } else {
        settings.mods_path.as_deref()
    }
}

fn normalize_preset_name(preset_name: Option<String>) -> String {
    preset_name
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "Category First".to_owned())
}

fn preset_description(name: &str) -> &'static str {
    match name {
        "Creator First" => "Creator folders lead, then kind and subtype.",
        "Hybrid" => "Balances content categories with creator grouping.",
        "Minimal Safe" => "Uses conservative folders that minimize risky depth.",
        _ => "Uses category and subtype first, then groups by creator.",
    }
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

fn parse_string_array(value: String) -> Vec<String> {
    serde_json::from_str(&value).unwrap_or_default()
}

fn normalize_component(value: &str) -> String {
    value
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn normalize_relative_path(path: PathBuf) -> String {
    path.to_string_lossy().replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use crate::{database, models::LibrarySettings};

    use super::{build_preview, list_rule_presets};

    #[test]
    fn lists_seeded_rule_presets() {
        let mut connection = rusqlite::Connection::open_in_memory().expect("in-memory db");
        database::initialize(&mut connection).expect("schema");
        database::seed_database(
            &mut connection,
            &crate::seed::load_seed_pack().expect("seed"),
        )
        .expect("seed db");

        let presets = list_rule_presets(&connection).expect("presets");
        assert!(presets.iter().any(|preset| preset.name == "Category First"));
        assert!(presets.iter().any(|preset| preset.name == "Minimal Safe"));
    }

    #[test]
    fn preview_builds_safe_paths_for_scripts() {
        let mut connection = rusqlite::Connection::open_in_memory().expect("in-memory db");
        database::initialize(&mut connection).expect("schema");
        database::seed_database(
            &mut connection,
            &crate::seed::load_seed_pack().expect("seed"),
        )
        .expect("seed db");
        let creator_id = connection
            .query_row(
                "SELECT id FROM creators WHERE canonical_name = ?1",
                rusqlite::params!["Deaderpool"],
                |row| row.get::<_, i64>(0),
            )
            .expect("creator");
        connection
            .execute(
                "INSERT INTO files (
                    path, filename, extension, kind, confidence, source_location, creator_id
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                rusqlite::params![
                    "C:/Mods/deep/mc_cmd_center.ts4script",
                    "mc_cmd_center.ts4script",
                    ".ts4script",
                    "ScriptMods",
                    0.95_f64,
                    "mods",
                    creator_id
                ],
            )
            .expect("file");

        let preview = build_preview(
            &connection,
            &LibrarySettings {
                mods_path: Some("C:/Mods".to_owned()),
                tray_path: Some("C:/Tray".to_owned()),
            },
            Some("Category First".to_owned()),
            20,
        )
        .expect("preview");

        let script = preview
            .suggestions
            .iter()
            .find(|item| item.filename == "mc_cmd_center.ts4script")
            .expect("script suggestion");
        assert_eq!(
            script.final_relative_path,
            "ScriptMods/Deaderpool/mc_cmd_center.ts4script"
        );
    }

    #[test]
    fn preview_prefers_locked_creator_path() {
        let mut connection = rusqlite::Connection::open_in_memory().expect("in-memory db");
        database::initialize(&mut connection).expect("schema");
        database::seed_database(
            &mut connection,
            &crate::seed::load_seed_pack().expect("seed"),
        )
        .expect("seed db");
        connection
            .execute(
                "INSERT INTO creators (canonical_name, notes, locked_by_user, preferred_path)
                 VALUES (?1, ?2, 1, ?3)",
                rusqlite::params!["CustomMaker", "User-learned", "Creators/CustomMaker"],
            )
            .expect("creator");
        let creator_id = connection.last_insert_rowid();
        connection
            .execute(
                "INSERT INTO files (
                    path, filename, extension, kind, confidence, source_location, creator_id
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                rusqlite::params![
                    "C:/Mods/Loose/item.package",
                    "item.package",
                    ".package",
                    "CAS",
                    0.97_f64,
                    "mods",
                    creator_id
                ],
            )
            .expect("file");

        let preview = build_preview(
            &connection,
            &LibrarySettings {
                mods_path: Some("C:/Mods".to_owned()),
                tray_path: Some("C:/Tray".to_owned()),
            },
            Some("Category First".to_owned()),
            20,
        )
        .expect("preview");

        let item = preview
            .suggestions
            .iter()
            .find(|suggestion| suggestion.filename == "item.package")
            .expect("item suggestion");
        assert_eq!(item.final_relative_path, "Creators/CustomMaker/item.package");
    }
}
