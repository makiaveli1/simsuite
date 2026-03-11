use std::{
    collections::{BTreeMap, HashSet},
    path::{Path, PathBuf},
};

use rusqlite::{params, Connection, OptionalExtension};

use crate::{
    core::validator::{validate_suggestion, ValidationRequest},
    error::AppResult,
    models::{
        LibrarySettings, OrganizationPreview, PreviewIssueSummary, PreviewSuggestion,
        ReviewQueueItem, RulePreset,
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

#[derive(Debug, Clone)]
struct StructureProfile {
    label: String,
    recommended_preset: String,
    recommended_reason: String,
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
    sample_limit: i64,
) -> AppResult<OrganizationPreview> {
    let preset_name = normalize_preset_name(preset_name);
    let candidates = load_candidates(connection, None, None)?;
    let mut preview =
        build_preview_from_candidates(connection, settings, &preset_name, candidates)?;

    if sample_limit > 0 && preview.suggestions.len() > sample_limit as usize {
        preview.suggestions.truncate(sample_limit as usize);
    }

    Ok(preview)
}

pub fn build_preview_full(
    connection: &Connection,
    settings: &LibrarySettings,
    preset_name: Option<String>,
) -> AppResult<OrganizationPreview> {
    let preset_name = normalize_preset_name(preset_name);
    let candidates = load_candidates(connection, None, None)?;
    build_preview_from_candidates(connection, settings, &preset_name, candidates)
}

pub fn build_preview_for_files(
    connection: &Connection,
    settings: &LibrarySettings,
    preset_name: Option<String>,
    file_ids: &[i64],
) -> AppResult<OrganizationPreview> {
    let preset_name = normalize_preset_name(preset_name);
    let candidates = load_candidates(connection, Some(file_ids), None)?;
    build_preview_from_candidates(connection, settings, &preset_name, candidates)
}

fn build_preview_from_candidates(
    connection: &Connection,
    settings: &LibrarySettings,
    preset_name: &str,
    candidates: Vec<PreviewCandidate>,
) -> AppResult<OrganizationPreview> {
    let structure_profile = detect_structure_profile(connection, settings)?;
    let mut suggestions = suggest_for_candidates(connection, settings, preset_name, candidates)?;
    sort_suggestions(&mut suggestions);
    let safe_count = suggestions
        .iter()
        .filter(|item| {
            !item.review_required && item.final_absolute_path != Some(item.current_path.clone())
        })
        .count() as i64;
    let aligned_count = suggestions
        .iter()
        .filter(|item| item.final_absolute_path == Some(item.current_path.clone()))
        .count() as i64;
    let issue_summary = summarize_preview_issues(&suggestions);

    Ok(OrganizationPreview {
        preset_name: preset_name.to_owned(),
        detected_structure: structure_profile.label,
        total_considered: suggestions.len() as i64,
        safe_count,
        aligned_count,
        corrected_count: suggestions.iter().filter(|item| item.corrected).count() as i64,
        review_count: suggestions
            .iter()
            .filter(|item| item.review_required)
            .count() as i64,
        recommended_preset: structure_profile.recommended_preset,
        recommended_reason: structure_profile.recommended_reason,
        issue_summary,
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
        let rule_relative = preset_relative_path(preset_name, settings, &candidate);
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
                guided_install: false,
                allow_existing_target: false,
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
    limit: Option<i64>,
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

    let base_query = "SELECT
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
         ORDER BY f.modified_at DESC, f.filename COLLATE NOCASE";

    let candidates = if let Some(limit) = limit {
        let mut statement = connection.prepare(&format!("{base_query}\nLIMIT ?1"))?;
        let rows = statement.query_map(params![limit], |row| {
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
        })?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(crate::error::AppError::from)?
    } else {
        let mut statement = connection.prepare(base_query)?;
        let rows = statement.query_map([], |row| {
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
        })?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(crate::error::AppError::from)?
    };

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

fn detect_structure_profile(
    connection: &Connection,
    settings: &LibrarySettings,
) -> AppResult<StructureProfile> {
    let Some(mods_root) = settings.mods_path.as_deref() else {
        return Ok(StructureProfile {
            label: "Mods path not configured yet".to_owned(),
            recommended_preset: "Minimal Safe".to_owned(),
            recommended_reason: "Set the Mods folder first, then start with the safest preset."
                .to_owned(),
        });
    };

    let mut statement = connection.prepare(
        "SELECT path, kind, subtype, c.canonical_name
         FROM files
         LEFT JOIN creators c ON files.creator_id = c.id
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
                row.get::<_, Option<String>>(3)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;

    let mut kind_led = 0;
    let mut subtype_led = 0;
    let mut creator_led = 0;
    let mut kind_under_creator = 0;
    let mut inspected = 0;

    for (path, kind, subtype, creator) in rows {
        if let Ok(relative) = Path::new(&path).strip_prefix(mods_root) {
            inspected += 1;
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

            if let Some(first) = segments.first() {
                if let Some(creator) = &creator {
                    if normalize_component(first) == normalize_component(creator) {
                        creator_led += 1;
                    }
                }
            }

            if let Some(second) = segments.get(1) {
                if normalize_component(second) == normalize_component(&kind) {
                    kind_under_creator += 1;
                }
            }
        }
    }

    if inspected < 10 {
        return Ok(StructureProfile {
            label: "Not enough organized files yet to read the current folder style".to_owned(),
            recommended_preset: "Minimal Safe".to_owned(),
            recommended_reason:
                "Start with the safest layout first, then switch once the library settles."
                    .to_owned(),
        });
    }

    if kind_led > 24 && subtype_led > 12 {
        return Ok(StructureProfile {
            label: "Mostly category folders with stable subtype branches".to_owned(),
            recommended_preset: "Mirror Mode".to_owned(),
            recommended_reason:
                "Your library already has a readable category shape, so Mirror Mode will keep it and only fix unsafe placements."
                    .to_owned(),
        });
    }

    if creator_led > 20 && kind_under_creator > 10 {
        return Ok(StructureProfile {
            label: "Mostly creator folders with content grouped underneath".to_owned(),
            recommended_preset: "Creator First".to_owned(),
            recommended_reason:
                "Your current folders already lean creator-first, so this preset will stay familiar while still validating each target."
                    .to_owned(),
        });
    }

    if kind_led > 18 {
        return Ok(StructureProfile {
            label: "Some category grouping is already present, but the tree is still mixed".to_owned(),
            recommended_preset: "Category First".to_owned(),
            recommended_reason:
                "Category First will clean toward a clear type-based layout without being as conservative as Minimal Safe."
                    .to_owned(),
        });
    }

    Ok(StructureProfile {
        label: "Current library looks mixed, so a conservative pass is safest".to_owned(),
        recommended_preset: "Minimal Safe".to_owned(),
        recommended_reason:
            "Minimal Safe keeps folder depth shallow and is the easiest first cleanup when the current shape is inconsistent."
                .to_owned(),
    })
}

fn preset_relative_path(
    preset_name: &str,
    settings: &LibrarySettings,
    candidate: &PreviewCandidate,
) -> String {
    if candidate.creator_locked_by_user {
        if let Some(preferred_path) = candidate
            .creator_preferred_path
            .as_deref()
            .map(|value| value.trim())
            .filter(|value| !value.is_empty())
        {
            return normalize_relative_path(
                PathBuf::from(preferred_path).join(&candidate.filename),
            );
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
        "Mirror Mode" => mirror_mode_path(
            settings,
            candidate,
            &creator,
            &subtype,
            &mod_name,
            &tray_bundle,
        ),
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

fn mirror_mode_path(
    settings: &LibrarySettings,
    candidate: &PreviewCandidate,
    creator: &str,
    subtype: &str,
    mod_name: &str,
    tray_bundle: &str,
) -> String {
    if let Some(relative) = current_relative_path(settings, candidate) {
        return normalize_relative_path(relative);
    }

    category_first_path(candidate, creator, subtype, mod_name, tray_bundle)
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
        "Mirror Mode" => "Keeps the current safe structure and only corrects risky paths.",
        "Creator First" => "Creator folders lead, then kind and subtype.",
        "Hybrid" => "Balances content categories with creator grouping.",
        "Minimal Safe" => "Uses conservative folders that minimize risky depth.",
        _ => "Uses category and subtype first, then groups by creator.",
    }
}

fn current_relative_path(
    settings: &LibrarySettings,
    candidate: &PreviewCandidate,
) -> Option<PathBuf> {
    let current_root = match candidate.source_location.as_str() {
        "tray" => settings.tray_path.as_deref(),
        "mods" => settings.mods_path.as_deref(),
        _ => None,
    }?;

    Path::new(&candidate.path)
        .strip_prefix(current_root)
        .ok()
        .map(Path::to_path_buf)
}

fn sort_suggestions(suggestions: &mut [PreviewSuggestion]) {
    suggestions.sort_by(|left, right| {
        preview_state_rank(left)
            .cmp(&preview_state_rank(right))
            .then_with(|| right.confidence.total_cmp(&left.confidence))
            .then_with(|| {
                left.filename
                    .to_lowercase()
                    .cmp(&right.filename.to_lowercase())
            })
    });
}

fn preview_state_rank(suggestion: &PreviewSuggestion) -> u8 {
    if !suggestion.review_required
        && suggestion.final_absolute_path != Some(suggestion.current_path.clone())
    {
        0
    } else if suggestion.review_required {
        1
    } else {
        2
    }
}

fn summarize_preview_issues(suggestions: &[PreviewSuggestion]) -> Vec<PreviewIssueSummary> {
    let mut counts = BTreeMap::<String, i64>::new();

    for suggestion in suggestions {
        let mut seen = HashSet::new();
        for note in &suggestion.validator_notes {
            if seen.insert(note) {
                *counts.entry(note.clone()).or_default() += 1;
            }
        }
    }

    let priority = [
        "low_confidence_requires_review",
        "unknown_kind_requires_review",
        "existing_path_collision_detected",
        "preview_path_collision_detected",
        "tray_file_will_be_relocated_from_mods",
        "validator_routed_tray_content_to_tray_root",
        "validator_flattened_script_depth",
        "validator_limited_package_depth",
        "missing_target_root",
    ];

    let mut summary = Vec::new();
    for code in priority {
        if let Some(count) = counts.remove(code) {
            let (label, tone) = preview_issue_label(code);
            summary.push(PreviewIssueSummary {
                code: code.to_owned(),
                label: label.to_owned(),
                count,
                tone: tone.to_owned(),
            });
        }
    }

    for (code, count) in counts {
        let (label, tone) = preview_issue_label(&code);
        summary.push(PreviewIssueSummary {
            code,
            label: label.to_owned(),
            count,
            tone: tone.to_owned(),
        });
    }

    summary
}

fn preview_issue_label(code: &str) -> (&'static str, &'static str) {
    match code {
        "low_confidence_requires_review" => ("Name or type still looks uncertain", "review"),
        "unknown_kind_requires_review" => ("Some files still have an unknown type", "review"),
        "existing_path_collision_detected" => {
            ("Another file already uses that destination", "review")
        }
        "preview_path_collision_detected" => {
            ("Two files in this pass want the same slot", "review")
        }
        "tray_file_will_be_relocated_from_mods" => ("Tray files were found inside Mods", "warn"),
        "validator_routed_tray_content_to_tray_root" => {
            ("Tray files were rerouted back to Tray", "warn")
        }
        "validator_flattened_script_depth" => {
            ("Script mods were flattened to a safe depth", "warn")
        }
        "validator_limited_package_depth" => ("Deep folder paths were shortened", "warn"),
        "missing_target_root" => ("A required root folder is missing", "review"),
        _ => ("Validator raised an extra check", "neutral"),
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
                downloads_path: None,
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
        assert_eq!(preview.safe_count, 1);
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
                downloads_path: None,
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
        assert_eq!(
            item.final_relative_path,
            "Creators/CustomMaker/item.package"
        );
    }

    #[test]
    fn mirror_mode_preserves_existing_relative_path_when_safe() {
        let mut connection = rusqlite::Connection::open_in_memory().expect("in-memory db");
        database::initialize(&mut connection).expect("schema");
        database::seed_database(
            &mut connection,
            &crate::seed::load_seed_pack().expect("seed"),
        )
        .expect("seed db");
        connection
            .execute(
                "INSERT INTO files (
                    path, filename, extension, kind, subtype, confidence, source_location
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                rusqlite::params![
                    "C:/Mods/CAS/Hair/Artist/Breezy.package",
                    "Breezy.package",
                    ".package",
                    "CAS",
                    "Hair",
                    0.91_f64,
                    "mods"
                ],
            )
            .expect("file");

        let preview = build_preview(
            &connection,
            &LibrarySettings {
                mods_path: Some("C:/Mods".to_owned()),
                tray_path: Some("C:/Tray".to_owned()),
                downloads_path: None,
            },
            Some("Mirror Mode".to_owned()),
            20,
        )
        .expect("preview");

        let item = preview
            .suggestions
            .iter()
            .find(|suggestion| suggestion.filename == "Breezy.package")
            .expect("item suggestion");
        assert_eq!(item.final_relative_path, "CAS/Hair/Artist/Breezy.package");
    }

    #[test]
    fn preview_counts_full_library_but_limits_returned_rows() {
        let mut connection = rusqlite::Connection::open_in_memory().expect("in-memory db");
        database::initialize(&mut connection).expect("schema");
        database::seed_database(
            &mut connection,
            &crate::seed::load_seed_pack().expect("seed"),
        )
        .expect("seed db");

        for index in 0..3 {
            connection
                .execute(
                    "INSERT INTO files (
                        path, filename, extension, kind, subtype, confidence, source_location
                     ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    rusqlite::params![
                        format!("C:/Mods/Loose/item_{index}.package"),
                        format!("item_{index}.package"),
                        ".package",
                        "CAS",
                        "Hair",
                        0.97_f64,
                        "mods"
                    ],
                )
                .expect("file");
        }

        let preview = build_preview(
            &connection,
            &LibrarySettings {
                mods_path: Some("C:/Mods".to_owned()),
                tray_path: Some("C:/Tray".to_owned()),
                downloads_path: None,
            },
            Some("Category First".to_owned()),
            2,
        )
        .expect("preview");

        assert_eq!(preview.total_considered, 3);
        assert_eq!(preview.safe_count, 3);
        assert_eq!(preview.suggestions.len(), 2);
    }
}
