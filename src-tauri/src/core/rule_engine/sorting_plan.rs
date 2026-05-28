use std::{collections::HashSet, path::Path};

use chrono::Utc;
use rusqlite::{params_from_iter, types::Value, Connection};

use crate::{
    error::AppResult,
    models::{
        ApplyPlanCategoryFolderMode, ApplyPlanCreatorFolderMode, ApplyPlanFolderConfig,
        ApplyPlanFolderConfigMode, GenerateSortingPreviewPlanRequest,
        GenerateSortingPreviewPlanScope, LibrarySettings, StagingPlan, StagingPlanActionKind,
        StagingPlanBucket, StagingPlanConfidenceLabel, StagingPlanCurrentRoot,
        StagingPlanEvidenceLevel, StagingPlanItem, StagingPlanSource, StagingPlanStatus,
    },
};

const DEFAULT_SORTING_PLAN_LIMIT: i64 = 100;
const MAX_SORTING_PLAN_LIMIT: i64 = 250;

#[derive(Debug, Clone)]
struct SortingCandidate {
    id: i64,
    filename: String,
    path: String,
    extension: String,
    kind: String,
    subtype: Option<String>,
    confidence: f64,
    source_location: String,
    relative_depth: i64,
    safety_notes: Vec<String>,
    parser_warnings: Vec<String>,
    creator: Option<String>,
    bundle_name: Option<String>,
    review_reasons: Vec<String>,
    has_exact_duplicate: bool,
}

pub fn generate_sorting_preview_plan(
    connection: &Connection,
    settings: &LibrarySettings,
    request: GenerateSortingPreviewPlanRequest,
) -> AppResult<StagingPlan> {
    let mut caveats = base_plan_caveats();
    let folder_config = request.folder_config.clone();
    if folder_config
        .as_ref()
        .is_some_and(|config| config.mode == ApplyPlanFolderConfigMode::Custom)
    {
        caveats.push(
            "Custom folder configuration influenced destination previews only; no folders or files were changed."
                .to_owned(),
        );
    }
    let candidates = match request.scope {
        GenerateSortingPreviewPlanScope::SelectedFiles { file_ids } => {
            let requested_count = file_ids.len();
            let ids = normalized_file_ids(file_ids);
            if ids.len() < requested_count {
                caveats.push(
                    "Some selected file IDs were skipped because the preview plan is bounded."
                        .to_owned(),
                );
            }
            load_selected_candidates(connection, &ids)?
        }
        GenerateSortingPreviewPlanScope::LibraryFolder {
            source_location,
            folder_path,
            recursive,
            limit,
        } => {
            let limit = bounded_limit(limit);
            caveats.push(format!(
                "Folder preview scope is capped at {} file{}.",
                limit,
                if limit == 1 { "" } else { "s" }
            ));
            load_folder_candidates(connection, &source_location, &folder_path, recursive, limit)?
        }
    };

    let items = candidates
        .iter()
        .map(|candidate| plan_item_for_candidate(settings, folder_config.as_ref(), candidate))
        .collect::<Vec<_>>();

    let item_count = items.len();
    let status = if item_count == 0 {
        StagingPlanStatus::Blocked
    } else {
        StagingPlanStatus::PreviewOnly
    };
    let summary = if item_count == 0 {
        "No Library files were available for a preview-only suggested plan.".to_owned()
    } else {
        format!(
            "{} Library file{} received preview-only organization suggestions. No files changed.",
            item_count,
            if item_count == 1 { "" } else { "s" }
        )
    };

    Ok(StagingPlan {
        id: "sorting-preview-plan-v1".to_owned(),
        created_at: Utc::now().to_rfc3339(),
        source: StagingPlanSource::Organize,
        status,
        title: "Suggested organization preview".to_owned(),
        summary,
        item_count,
        would_touch_files: false,
        caveats,
        items,
    })
}

fn base_plan_caveats() -> Vec<String> {
    vec![
        "No files changed. This generated plan is preview-only.".to_owned(),
        "Suggested destinations are review aids, not apply-ready file changes.".to_owned(),
        "Backup and restore support plus user confirmation are required before any future apply workflow.".to_owned(),
    ]
}

fn normalized_file_ids(file_ids: Vec<i64>) -> Vec<i64> {
    let mut seen = HashSet::new();
    let mut normalized = Vec::new();
    for id in file_ids {
        if id <= 0 || !seen.insert(id) {
            continue;
        }
        normalized.push(id);
        if normalized.len() >= MAX_SORTING_PLAN_LIMIT as usize {
            break;
        }
    }
    normalized
}

fn bounded_limit(limit: Option<i64>) -> i64 {
    limit
        .unwrap_or(DEFAULT_SORTING_PLAN_LIMIT)
        .max(0)
        .min(MAX_SORTING_PLAN_LIMIT)
}

fn load_selected_candidates(
    connection: &Connection,
    file_ids: &[i64],
) -> AppResult<Vec<SortingCandidate>> {
    if file_ids.is_empty() {
        return Ok(Vec::new());
    }

    let placeholders = std::iter::repeat("?")
        .take(file_ids.len())
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!(
        "{} WHERE f.source_location <> 'downloads' AND f.id IN ({}) ORDER BY f.filename COLLATE NOCASE, f.id ASC",
        candidate_select_sql(),
        placeholders
    );
    let params = file_ids
        .iter()
        .copied()
        .map(Value::Integer)
        .collect::<Vec<_>>();
    query_candidates(connection, &sql, params)
}

fn load_folder_candidates(
    connection: &Connection,
    source_location: &str,
    folder_path: &str,
    recursive: bool,
    limit: i64,
) -> AppResult<Vec<SortingCandidate>> {
    let source = source_location.trim().to_ascii_lowercase();
    if !matches!(source.as_str(), "mods" | "tray") {
        return Ok(Vec::new());
    }

    let segments = normalized_folder_segments(&source, folder_path);
    let mut filters = String::from(" WHERE f.source_location = ?");
    let mut params = vec![Value::Text(source.clone())];

    if segments.is_empty() {
        if !recursive {
            filters.push_str(" AND f.relative_depth = 0");
        }
    } else {
        if recursive {
            filters.push_str(" AND f.relative_depth >= ?");
        } else {
            filters.push_str(" AND f.relative_depth = ?");
        }
        params.push(Value::Integer(segments.len() as i64));

        let root = match source.as_str() {
            "mods" => settings_path_from_db(connection, "mods")?,
            "tray" => settings_path_from_db(connection, "tray")?,
            _ => None,
        };
        let Some(root) = root else {
            return Ok(Vec::new());
        };
        filters.push_str(" AND LOWER(REPLACE(f.path, '\\', '/')) LIKE ? ESCAPE '~'");
        params.push(Value::Text(folder_like_pattern(&root, &segments)));
    }

    params.push(Value::Integer(limit));
    let sql = format!(
        "{}{} ORDER BY f.filename COLLATE NOCASE, f.id ASC LIMIT ?",
        candidate_select_sql(),
        filters
    );
    query_candidates(connection, &sql, params)
}

fn settings_path_from_db(
    connection: &Connection,
    source_location: &str,
) -> AppResult<Option<String>> {
    let settings = crate::database::get_library_settings(connection)?;
    Ok(match source_location {
        "mods" => settings.mods_path,
        "tray" => settings.tray_path,
        _ => None,
    })
}

fn candidate_select_sql() -> String {
    format!(
        "SELECT
            f.id,
            f.filename,
            f.path,
            f.extension,
            f.kind,
            f.subtype,
            f.confidence,
            f.source_location,
            f.relative_depth,
            f.safety_notes,
            f.parser_warnings,
            c.canonical_name,
            b.bundle_name,
            COALESCE(rq.review_reasons, '') AS review_reasons,
            {} AS has_exact_duplicate
         FROM files f
         LEFT JOIN creators c ON f.creator_id = c.id
         LEFT JOIN bundles b ON f.bundle_id = b.id
         LEFT JOIN (
            SELECT file_id, GROUP_CONCAT(reason, '||') AS review_reasons
            FROM review_queue
            GROUP BY file_id
         ) rq ON rq.file_id = f.id",
        exact_duplicate_exists_sql()
    )
}

fn query_candidates(
    connection: &Connection,
    sql: &str,
    params: Vec<Value>,
) -> AppResult<Vec<SortingCandidate>> {
    let mut statement = connection.prepare(sql)?;
    let candidates = statement
        .query_map(params_from_iter(params.iter()), |row| {
            Ok(SortingCandidate {
                id: row.get(0)?,
                filename: row.get(1)?,
                path: row.get(2)?,
                extension: normalize_extension(row.get::<_, String>(3)?),
                kind: row.get(4)?,
                subtype: row.get(5)?,
                confidence: row.get(6)?,
                source_location: row.get(7)?,
                relative_depth: row.get(8)?,
                safety_notes: parse_string_array(row.get::<_, String>(9)?),
                parser_warnings: parse_string_array(row.get::<_, String>(10)?),
                creator: row.get(11)?,
                bundle_name: row.get(12)?,
                review_reasons: split_review_reasons(row.get::<_, String>(13)?),
                has_exact_duplicate: row.get::<_, i64>(14)? != 0,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(candidates)
}

fn exact_duplicate_exists_sql() -> &'static str {
    "EXISTS (
        SELECT 1
        FROM duplicates d
        JOIN files other ON other.id = CASE
            WHEN d.file_id_a = f.id THEN d.file_id_b
            ELSE d.file_id_a
        END
        WHERE d.duplicate_type = 'exact'
          AND (d.file_id_a = f.id OR d.file_id_b = f.id)
          AND other.id <> f.id
          AND LOWER(TRIM(COALESCE(other.path, ''))) <> LOWER(TRIM(COALESCE(f.path, '')))
          AND (
            (
              TRIM(COALESCE(f.hash, '')) <> ''
              AND TRIM(COALESCE(other.hash, '')) <> ''
              AND LOWER(TRIM(f.hash)) = LOWER(TRIM(other.hash))
            )
            OR
            (
              TRIM(COALESCE(f.content_fingerprint, '')) <> ''
              AND TRIM(COALESCE(other.content_fingerprint, '')) <> ''
              AND LOWER(TRIM(f.content_fingerprint)) = LOWER(TRIM(other.content_fingerprint))
              AND LOWER(TRIM(COALESCE(f.content_fingerprint_kind, ''))) = LOWER(TRIM(COALESCE(other.content_fingerprint_kind, '')))
              AND LOWER(TRIM(COALESCE(f.content_fingerprint_kind, ''))) IN ('package', 'script')
              AND LOWER(TRIM(COALESCE(f.content_fingerprint_status, ''))) = 'available'
              AND LOWER(TRIM(COALESCE(other.content_fingerprint_status, ''))) = 'available'
              AND TRIM(COALESCE(f.content_fingerprint_version, '')) <> ''
              AND TRIM(COALESCE(other.content_fingerprint_version, '')) <> ''
              AND LOWER(TRIM(f.content_fingerprint_version)) = LOWER(TRIM(other.content_fingerprint_version))
            )
          )
    )"
}

fn plan_item_for_candidate(
    settings: &LibrarySettings,
    folder_config: Option<&ApplyPlanFolderConfig>,
    candidate: &SortingCandidate,
) -> StagingPlanItem {
    let mut signals = vec![
        format!("source:{}", candidate.source_location),
        format!("extension:{}", candidate.extension),
        format!("kind:{}", candidate.kind),
    ];
    if let Some(subtype) = candidate
        .subtype
        .as_ref()
        .filter(|value| !value.trim().is_empty())
    {
        signals.push(format!("subtype:{subtype}"));
    }
    if candidate
        .creator
        .as_ref()
        .is_some_and(|value| !value.trim().is_empty())
    {
        signals.push("creator_metadata_present".to_owned());
    }
    if candidate
        .bundle_name
        .as_ref()
        .is_some_and(|value| !value.trim().is_empty())
    {
        signals.push("same_pack_grouping_hint".to_owned());
    }

    let current_root = current_root_for_source(&candidate.source_location);

    if candidate.has_exact_duplicate {
        return build_review_item(
            candidate,
            "This file has exact duplicate proof. SimSuite routes it to review instead of suggesting a move.",
            StagingPlanEvidenceLevel::Deterministic,
            StagingPlanConfidenceLabel::Deterministic,
            signals,
            vec!["exact_duplicate_review_required".to_owned()],
            vec![
                "Duplicate proof is review information, not cleanup or delete instruction."
                    .to_owned(),
                "No files changed. This plan is preview-only.".to_owned(),
            ],
            current_root,
        );
    }

    if !candidate.parser_warnings.is_empty() {
        signals.push("parser_warnings".to_owned());
        return build_review_item(
            candidate,
            "Parser warnings are present, so SimSuite keeps this item in manual review.",
            StagingPlanEvidenceLevel::EvidenceBacked,
            StagingPlanConfidenceLabel::EvidenceBacked,
            signals,
            candidate.parser_warnings.clone(),
            vec![
                "Parser warnings block strong organization suggestions.".to_owned(),
                "No files changed. This plan is preview-only.".to_owned(),
            ],
            current_root,
        );
    }

    if !candidate.safety_notes.is_empty() {
        signals.push("inspection_or_placement_warning".to_owned());
        return build_review_item(
            candidate,
            "Inspection or placement warnings are present, so SimSuite keeps this item in manual review.",
            StagingPlanEvidenceLevel::EvidenceBacked,
            StagingPlanConfidenceLabel::EvidenceBacked,
            signals,
            candidate.safety_notes.clone(),
            vec![
                "Warning signals block strong organization suggestions.".to_owned(),
                "No files changed. This plan is preview-only.".to_owned(),
            ],
            current_root,
        );
    }

    if !candidate.review_reasons.is_empty() {
        signals.push("review_queue".to_owned());
        return build_review_item(
            candidate,
            "This file is already in a review queue, so SimSuite does not suggest moving it.",
            StagingPlanEvidenceLevel::EvidenceBacked,
            StagingPlanConfidenceLabel::EvidenceBacked,
            signals,
            candidate.review_reasons.clone(),
            vec![
                "Review queue items stay review-first.".to_owned(),
                "No files changed. This plan is preview-only.".to_owned(),
            ],
            current_root,
        );
    }

    if source_is_unsupported(&candidate.source_location) {
        return leave_in_place_item(
            candidate,
            "SimSuite does not have enough supported source-root evidence to suggest a destination.",
            signals,
            vec!["unsupported_source_root".to_owned()],
            current_root,
        );
    }

    if candidate.extension == ".ts4script" || candidate.kind == "ScriptMods" {
        signals.push("script_file".to_owned());
        if candidate.relative_depth > 1 {
            return build_review_item(
                candidate,
                "Script placement can be sensitive, so nested script files stay in manual review.",
                StagingPlanEvidenceLevel::EvidenceBacked,
                StagingPlanConfidenceLabel::EvidenceBacked,
                signals,
                vec!["script_placement_review_required".to_owned()],
                vec![
                    "Script files should be reviewed before any future move.".to_owned(),
                    "No files changed. This plan is preview-only.".to_owned(),
                ],
                current_root,
            );
        }
        return suggest_bucket_item(
            settings,
            folder_config,
            candidate,
            StagingPlanBucket::ScriptMods,
            "This is a script file, so SimSuite can preview a Script Mods destination with caveats.",
            StagingPlanEvidenceLevel::Deterministic,
            StagingPlanConfidenceLabel::Deterministic,
            signals,
            vec![
                "Script placement can be sensitive; review before applying any future move."
                    .to_owned(),
                "No files changed. This plan is preview-only.".to_owned(),
            ],
            current_root,
        );
    }

    if is_tray_content(candidate) {
        signals.push("tray_content".to_owned());
        return StagingPlanItem {
            id: format!("sorting-file-{}", candidate.id),
            file_id: Some(candidate.id),
            file_name: candidate.filename.clone(),
            current_path: Some(candidate.path.clone()),
            suggested_destination_path: None,
            action_kind: StagingPlanActionKind::SuggestReview,
            evidence_level: StagingPlanEvidenceLevel::Deterministic,
            reason: "This is Tray content, so SimSuite keeps it in a Tray review plan.".to_owned(),
            caveats: vec![
                "Tray sets can span multiple files; review the group before any future action."
                    .to_owned(),
                "No files changed. This plan is preview-only.".to_owned(),
            ],
            source_signals: signals,
            blocked_reasons: vec!["tray_group_review_required".to_owned()],
            bucket: StagingPlanBucket::Tray,
            confidence_label: StagingPlanConfidenceLabel::Deterministic,
            current_root,
            would_touch_files: false,
        };
    }

    if let Some(bucket) = strong_bucket_for_candidate(candidate) {
        if current_parent_matches_bucket(settings, folder_config, candidate, &bucket) {
            let mut signals = signals;
            signals.push("current_folder_matches_bucket".to_owned());
            return StagingPlanItem {
                id: format!("sorting-file-{}", candidate.id),
                file_id: Some(candidate.id),
                file_name: candidate.filename.clone(),
                current_path: Some(candidate.path.clone()),
                suggested_destination_path: None,
                action_kind: StagingPlanActionKind::LeaveInPlace,
                evidence_level: StagingPlanEvidenceLevel::EvidenceBacked,
                reason: "The current folder already matches the strongest category signal, so SimSuite suggests leaving this file in place.".to_owned(),
                caveats: vec![
                    "Leave in place is still preview-only.".to_owned(),
                    "No files changed.".to_owned(),
                ],
                source_signals: signals,
                blocked_reasons: vec!["no_clear_benefit_to_move".to_owned()],
                bucket,
                confidence_label: StagingPlanConfidenceLabel::EvidenceBacked,
                current_root,
                would_touch_files: false,
            };
        }

        let reason = bucket_reason(&bucket);
        return suggest_bucket_item(
            settings,
            folder_config,
            candidate,
            bucket,
            reason,
            StagingPlanEvidenceLevel::EvidenceBacked,
            StagingPlanConfidenceLabel::EvidenceBacked,
            signals,
            vec![
                "Review before applying any future move.".to_owned(),
                "No files changed. This plan is preview-only.".to_owned(),
            ],
            current_root,
        );
    }

    if filename_has_sorting_clue(&candidate.filename) {
        signals.push("filename_clue".to_owned());
        return build_review_item(
            candidate,
            "Filename clues are weak evidence, so SimSuite routes this item to review instead of a move suggestion.",
            StagingPlanEvidenceLevel::Heuristic,
            StagingPlanConfidenceLabel::Heuristic,
            signals,
            vec!["filename_only_clue".to_owned()],
            vec![
                "Filename-only clues never create a strong move suggestion.".to_owned(),
                "No files changed. This plan is preview-only.".to_owned(),
            ],
            current_root,
        );
    }

    leave_in_place_item(
        candidate,
        "SimSuite has limited information here, so this file should stay in place for review.",
        signals,
        vec!["weak_or_unknown_metadata".to_owned()],
        current_root,
    )
}

fn build_review_item(
    candidate: &SortingCandidate,
    reason: &str,
    evidence_level: StagingPlanEvidenceLevel,
    confidence_label: StagingPlanConfidenceLabel,
    source_signals: Vec<String>,
    blocked_reasons: Vec<String>,
    caveats: Vec<String>,
    current_root: StagingPlanCurrentRoot,
) -> StagingPlanItem {
    StagingPlanItem {
        id: format!("sorting-file-{}", candidate.id),
        file_id: Some(candidate.id),
        file_name: candidate.filename.clone(),
        current_path: Some(candidate.path.clone()),
        suggested_destination_path: None,
        action_kind: StagingPlanActionKind::SuggestReview,
        evidence_level,
        reason: reason.to_owned(),
        caveats,
        source_signals,
        blocked_reasons,
        bucket: StagingPlanBucket::NeedsReview,
        confidence_label,
        current_root,
        would_touch_files: false,
    }
}

fn leave_in_place_item(
    candidate: &SortingCandidate,
    reason: &str,
    source_signals: Vec<String>,
    blocked_reasons: Vec<String>,
    current_root: StagingPlanCurrentRoot,
) -> StagingPlanItem {
    StagingPlanItem {
        id: format!("sorting-file-{}", candidate.id),
        file_id: Some(candidate.id),
        file_name: candidate.filename.clone(),
        current_path: Some(candidate.path.clone()),
        suggested_destination_path: None,
        action_kind: StagingPlanActionKind::LeaveInPlace,
        evidence_level: StagingPlanEvidenceLevel::ReviewOnly,
        reason: reason.to_owned(),
        caveats: vec![
            "Leave in place is a preview-only suggestion.".to_owned(),
            "No files changed.".to_owned(),
        ],
        source_signals,
        blocked_reasons,
        bucket: StagingPlanBucket::UnknownLeaveInPlace,
        confidence_label: StagingPlanConfidenceLabel::ReviewOnly,
        current_root,
        would_touch_files: false,
    }
}

fn suggest_bucket_item(
    settings: &LibrarySettings,
    folder_config: Option<&ApplyPlanFolderConfig>,
    candidate: &SortingCandidate,
    bucket: StagingPlanBucket,
    reason: &str,
    evidence_level: StagingPlanEvidenceLevel,
    confidence_label: StagingPlanConfidenceLabel,
    source_signals: Vec<String>,
    mut caveats: Vec<String>,
    current_root: StagingPlanCurrentRoot,
) -> StagingPlanItem {
    let target_root = target_root_for_bucket(settings, &bucket);
    let (suggested_destination_path, custom_folder_applied) = target_root
        .map(|root| {
            build_suggested_destination(
                root,
                &bucket,
                &candidate.filename,
                folder_config,
                candidate,
            )
        })
        .map(|(path, applied)| (Some(path), applied))
        .unwrap_or((None, false));
    let mut source_signals = source_signals;
    if custom_folder_applied {
        source_signals.push("custom_folder_config_applied".to_owned());
    }
    let (action_kind, bucket, blocked_reasons) = if suggested_destination_path.is_some() {
        (StagingPlanActionKind::SuggestMove, bucket, Vec::new())
    } else {
        caveats.push(
            "A configured target folder is needed before SimSuite can preview a destination path."
                .to_owned(),
        );
        (
            StagingPlanActionKind::SuggestReview,
            StagingPlanBucket::NeedsReview,
            vec!["missing_configured_target_root".to_owned()],
        )
    };

    StagingPlanItem {
        id: format!("sorting-file-{}", candidate.id),
        file_id: Some(candidate.id),
        file_name: candidate.filename.clone(),
        current_path: Some(candidate.path.clone()),
        suggested_destination_path,
        action_kind,
        evidence_level,
        reason: reason.to_owned(),
        caveats,
        source_signals,
        blocked_reasons,
        bucket,
        confidence_label,
        current_root,
        would_touch_files: false,
    }
}

fn strong_bucket_for_candidate(candidate: &SortingCandidate) -> Option<StagingPlanBucket> {
    if candidate.confidence < 0.7 {
        return None;
    }

    match candidate.kind.as_str() {
        "CAS" => Some(StagingPlanBucket::Cas),
        "BuildBuy" => Some(StagingPlanBucket::BuildBuy),
        "Gameplay" => Some(StagingPlanBucket::Gameplay),
        "PresetsAndSliders" => Some(StagingPlanBucket::PresetsSliders),
        "OverridesAndDefaults" => Some(StagingPlanBucket::OverridesDefaults),
        _ => None,
    }
}

fn bucket_reason(bucket: &StagingPlanBucket) -> &'static str {
    match bucket {
        StagingPlanBucket::Cas => {
            "Package metadata suggests CAS content, so SimSuite can preview a CAS destination with caveats."
        }
        StagingPlanBucket::BuildBuy => {
            "Package metadata suggests Build/Buy content, so SimSuite can preview a Build/Buy grouping."
        }
        StagingPlanBucket::Gameplay => {
            "SimSuite found gameplay-style metadata, so this can be previewed as a suggested destination."
        }
        StagingPlanBucket::PresetsSliders => {
            "Subtype metadata points toward presets or sliders, so this can be reviewed there."
        }
        StagingPlanBucket::OverridesDefaults => {
            "SimSuite has an override/defaults clue, so this should be reviewed before any move."
        }
        _ => "SimSuite can preview a suggested destination with caveats.",
    }
}

fn target_root_for_bucket<'a>(
    settings: &'a LibrarySettings,
    bucket: &StagingPlanBucket,
) -> Option<&'a str> {
    match bucket {
        StagingPlanBucket::Tray => settings.tray_path.as_deref(),
        StagingPlanBucket::NeedsReview | StagingPlanBucket::UnknownLeaveInPlace => None,
        _ => settings.mods_path.as_deref(),
    }
    .filter(|value| !value.trim().is_empty())
}

fn build_suggested_destination(
    root: &str,
    bucket: &StagingPlanBucket,
    filename: &str,
    folder_config: Option<&ApplyPlanFolderConfig>,
    candidate: &SortingCandidate,
) -> (String, bool) {
    let (segments, custom_folder_applied) =
        destination_folder_segments(bucket, folder_config, candidate);
    let mut destination = Path::new(root).to_path_buf();
    for segment in segments {
        destination = destination.join(segment);
    }
    destination = destination.join(filename);
    (
        destination.to_string_lossy().to_string(),
        custom_folder_applied,
    )
}

fn destination_folder_segments(
    bucket: &StagingPlanBucket,
    folder_config: Option<&ApplyPlanFolderConfig>,
    candidate: &SortingCandidate,
) -> (Vec<String>, bool) {
    let default_segment = default_bucket_folder_name(bucket).to_owned();
    let Some(config) = folder_config else {
        return (vec![default_segment], false);
    };

    if config.mode != ApplyPlanFolderConfigMode::Custom {
        return (vec![default_segment], false);
    }

    let mut applied_custom = false;
    let bucket_key = bucket_config_key(bucket);
    let mut segments = config
        .bucket_folders
        .get(bucket_key)
        .map(|value| safe_config_path_segments(value))
        .filter(|segments| !segments.is_empty())
        .unwrap_or_else(|| vec![default_segment.clone()]);

    if segments != vec![default_segment.clone()] {
        applied_custom = true;
    }

    if config.category_folder_mode == ApplyPlanCategoryFolderMode::BucketAndCategory {
        if let Some(subtype) = candidate
            .subtype
            .as_ref()
            .map(|value| sanitize_config_segment(value))
            .filter(|value| !value.is_empty())
        {
            segments.push(subtype);
            applied_custom = true;
        }
    }

    if config.creator_folder_mode == ApplyPlanCreatorFolderMode::WhenAvailable {
        if let Some(creator) = candidate
            .creator
            .as_ref()
            .map(|value| sanitize_config_segment(value))
            .filter(|value| !value.is_empty())
        {
            segments.push(creator);
            applied_custom = true;
        }
    }

    if let Some(max_depth) = config.max_depth.filter(|value| *value > 0) {
        segments.truncate(max_depth as usize);
    }

    if segments.is_empty() {
        (vec![default_segment], applied_custom)
    } else {
        (segments, applied_custom)
    }
}

fn default_bucket_folder_name(bucket: &StagingPlanBucket) -> &'static str {
    match bucket {
        StagingPlanBucket::ScriptMods => "Script Mods",
        StagingPlanBucket::Cas => "CAS",
        StagingPlanBucket::BuildBuy => "BuildBuy",
        StagingPlanBucket::Gameplay => "Gameplay",
        StagingPlanBucket::PresetsSliders => "PresetsAndSliders",
        StagingPlanBucket::OverridesDefaults => "OverridesAndDefaults",
        StagingPlanBucket::Tray => "Tray",
        StagingPlanBucket::NeedsReview => "Needs Review",
        StagingPlanBucket::UnknownLeaveInPlace => "Unknown",
    }
}

fn bucket_config_key(bucket: &StagingPlanBucket) -> &'static str {
    match bucket {
        StagingPlanBucket::ScriptMods => "script_mods",
        StagingPlanBucket::Cas => "cas",
        StagingPlanBucket::BuildBuy => "build_buy",
        StagingPlanBucket::Gameplay => "gameplay",
        StagingPlanBucket::PresetsSliders => "presets_sliders",
        StagingPlanBucket::OverridesDefaults => "overrides_defaults",
        StagingPlanBucket::Tray => "tray",
        StagingPlanBucket::NeedsReview => "needs_review",
        StagingPlanBucket::UnknownLeaveInPlace => "unknown_leave_in_place",
    }
}

fn safe_config_path_segments(value: &str) -> Vec<String> {
    value
        .replace('\\', "/")
        .split('/')
        .map(sanitize_config_segment)
        .filter(|segment| !segment.is_empty())
        .collect()
}

fn sanitize_config_segment(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() || matches!(trimmed, "." | "..") || trimmed.contains(':') {
        return String::new();
    }

    trimmed
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
        .trim_matches('.')
        .trim()
        .to_owned()
}

fn current_parent_matches_bucket(
    settings: &LibrarySettings,
    folder_config: Option<&ApplyPlanFolderConfig>,
    candidate: &SortingCandidate,
    bucket: &StagingPlanBucket,
) -> bool {
    let root = match candidate.source_location.as_str() {
        "mods" => settings.mods_path.as_deref(),
        "tray" => settings.tray_path.as_deref(),
        _ => None,
    };
    let Some(root) = root else {
        return false;
    };
    let Ok(relative) = Path::new(&candidate.path).strip_prefix(root) else {
        return false;
    };
    let Some(parent) = relative.parent() else {
        return false;
    };
    let first = parent
        .components()
        .next()
        .map(|component| normalize_component(&component.as_os_str().to_string_lossy()));
    let expected = destination_folder_segments(bucket, folder_config, candidate)
        .0
        .first()
        .map(|segment| normalize_component(segment));
    first == expected
}

fn is_tray_content(candidate: &SortingCandidate) -> bool {
    candidate.source_location == "tray"
        || candidate.kind.starts_with("Tray")
        || matches!(
            candidate.extension.as_str(),
            ".trayitem" | ".blueprint" | ".bpi" | ".hhi" | ".householdbinary" | ".sgi" | ".rmi"
        )
}

fn source_is_unsupported(source_location: &str) -> bool {
    !matches!(source_location, "mods" | "tray")
}

fn filename_has_sorting_clue(filename: &str) -> bool {
    let normalized = filename.to_ascii_lowercase();
    [
        "cas", "hair", "build", "buy", "gameplay", "slider", "preset", "override", "default",
    ]
    .iter()
    .any(|token| normalized.contains(token))
}

fn current_root_for_source(source_location: &str) -> StagingPlanCurrentRoot {
    match source_location {
        "mods" => StagingPlanCurrentRoot::Mods,
        "tray" => StagingPlanCurrentRoot::Tray,
        "downloads" => StagingPlanCurrentRoot::Downloads,
        "inbox" => StagingPlanCurrentRoot::Inbox,
        _ => StagingPlanCurrentRoot::Unknown,
    }
}

fn normalize_extension(value: String) -> String {
    let trimmed = value.trim().to_ascii_lowercase();
    if trimmed.starts_with('.') {
        trimmed
    } else if trimmed.is_empty() {
        trimmed
    } else {
        format!(".{trimmed}")
    }
}

fn parse_string_array(json: String) -> Vec<String> {
    serde_json::from_str::<Vec<String>>(&json)
        .unwrap_or_default()
        .into_iter()
        .filter(|value| !value.trim().is_empty())
        .collect()
}

fn split_review_reasons(value: String) -> Vec<String> {
    value
        .split("||")
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn normalized_folder_segments(source_location: &str, folder_path: &str) -> Vec<String> {
    folder_path
        .replace('\\', "/")
        .split('/')
        .map(str::trim)
        .filter(|segment| !segment.is_empty())
        .filter(|segment| {
            let normalized = segment.to_ascii_lowercase();
            !((source_location == "mods" && normalized == "mods")
                || (source_location == "tray" && normalized == "tray"))
        })
        .map(ToOwned::to_owned)
        .collect()
}

fn folder_like_pattern(root: &str, segments: &[String]) -> String {
    let normalized_root = normalize_path_for_query(root);
    let child_path = segments
        .iter()
        .map(|segment| escape_like(&normalize_path_for_query(segment)))
        .collect::<Vec<_>>()
        .join("/");
    format!(
        "{}/{}%",
        escape_like(&normalized_root),
        folder_child_prefix(&child_path)
    )
}

fn folder_child_prefix(child_path: &str) -> String {
    if child_path.ends_with('/') {
        child_path.to_owned()
    } else {
        format!("{child_path}/")
    }
}

fn normalize_path_for_query(value: &str) -> String {
    value
        .replace('\\', "/")
        .trim_end_matches('/')
        .to_ascii_lowercase()
}

fn escape_like(value: &str) -> String {
    value
        .replace('~', "~~")
        .replace('%', "~%")
        .replace('_', "~_")
}

fn normalize_component(value: &str) -> String {
    value
        .to_ascii_lowercase()
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database;
    use rusqlite::{params, Connection};
    use tempfile::tempdir;

    fn setup_connection() -> (Connection, LibrarySettings) {
        let temp = tempdir().expect("temp dir");
        let mods = temp.path().join("Mods");
        let tray = temp.path().join("Tray");
        std::fs::create_dir_all(&mods).expect("mods dir");
        std::fs::create_dir_all(&tray).expect("tray dir");

        let mut connection = Connection::open_in_memory().expect("memory db");
        database::initialize(&mut connection).expect("schema");
        let settings = LibrarySettings {
            mods_path: Some(mods.to_string_lossy().to_string()),
            tray_path: Some(tray.to_string_lossy().to_string()),
            downloads_path: None,
            download_reject_folder: None,
        };
        database::save_library_paths(&mut connection, &settings).expect("settings");
        (connection, settings)
    }

    fn insert_file(
        connection: &Connection,
        settings: &LibrarySettings,
        id: i64,
        relative_path: &str,
        extension: &str,
        kind: &str,
        subtype: Option<&str>,
        confidence: f64,
        source_location: &str,
        parser_warnings: &[&str],
        safety_notes: &[&str],
    ) {
        let root = match source_location {
            "tray" => settings.tray_path.as_ref().expect("tray root"),
            _ => settings.mods_path.as_ref().expect("mods root"),
        };
        let path = Path::new(root).join(relative_path);
        let relative_depth = Path::new(relative_path)
            .parent()
            .map(|parent| parent.components().count() as i64)
            .unwrap_or(0);
        let filename = path
            .file_name()
            .expect("filename")
            .to_string_lossy()
            .to_string();
        connection
            .execute(
                "INSERT INTO files (
                    id, path, filename, extension, hash, size, modified_at, kind, subtype,
                    confidence, source_location, relative_depth, safety_notes, parser_warnings,
                    insights
                ) VALUES (?1, ?2, ?3, ?4, ?5, 1, '2026-05-13T00:00:00Z', ?6, ?7, ?8, ?9, ?10, ?11, ?12, '{}')",
                params![
                    id,
                    path.to_string_lossy().to_string(),
                    filename,
                    extension,
                    format!("hash-{id}"),
                    kind,
                    subtype,
                    confidence,
                    source_location,
                    relative_depth,
                    serde_json::to_string(parser_warnings).expect("warnings"),
                    serde_json::to_string(safety_notes).expect("notes"),
                ],
            )
            .expect("insert file");
    }

    fn selected_request(file_ids: Vec<i64>) -> GenerateSortingPreviewPlanRequest {
        GenerateSortingPreviewPlanRequest {
            scope: GenerateSortingPreviewPlanScope::SelectedFiles { file_ids },
            folder_config: None,
            context_trail: Vec::new(),
        }
    }

    fn first_item(plan: &StagingPlan) -> &StagingPlanItem {
        plan.items.first().expect("first plan item")
    }

    #[test]
    fn selected_file_ids_produce_preview_only_plan() {
        let (connection, settings) = setup_connection();
        insert_file(
            &connection,
            &settings,
            1,
            "Loose/Hair.package",
            ".package",
            "CAS",
            Some("Hair"),
            0.86,
            "mods",
            &[],
            &[],
        );

        let plan = generate_sorting_preview_plan(&connection, &settings, selected_request(vec![1]))
            .expect("plan");

        assert_eq!(plan.status, StagingPlanStatus::PreviewOnly);
        assert_eq!(plan.item_count, 1);
        assert!(!plan.would_touch_files);
        let item = first_item(&plan);
        assert_eq!(item.bucket, StagingPlanBucket::Cas);
        assert_eq!(item.action_kind, StagingPlanActionKind::SuggestMove);
        assert_eq!(
            item.evidence_level,
            StagingPlanEvidenceLevel::EvidenceBacked
        );
        assert!(!item.would_touch_files);
        assert!(item.suggested_destination_path.is_some());
    }

    #[test]
    fn exact_duplicates_route_to_review_not_move() {
        let (connection, settings) = setup_connection();
        insert_file(
            &connection,
            &settings,
            1,
            "A.package",
            ".package",
            "CAS",
            None,
            0.9,
            "mods",
            &[],
            &[],
        );
        insert_file(
            &connection,
            &settings,
            2,
            "B.package",
            ".package",
            "CAS",
            None,
            0.9,
            "mods",
            &[],
            &[],
        );
        connection
            .execute("UPDATE files SET hash = 'same-hash' WHERE id IN (1, 2)", [])
            .expect("same hash");
        connection
            .execute(
                "INSERT INTO duplicates (file_id_a, file_id_b, duplicate_type, detection_method) VALUES (1, 2, 'exact', 'sha256')",
                [],
            )
            .expect("duplicate");

        let plan = generate_sorting_preview_plan(&connection, &settings, selected_request(vec![1]))
            .expect("plan");
        let item = first_item(&plan);

        assert_eq!(item.action_kind, StagingPlanActionKind::SuggestReview);
        assert_eq!(item.bucket, StagingPlanBucket::NeedsReview);
        assert_eq!(item.evidence_level, StagingPlanEvidenceLevel::Deterministic);
        assert!(item
            .blocked_reasons
            .contains(&"exact_duplicate_review_required".to_owned()));
        assert!(item.suggested_destination_path.is_none());
    }

    #[test]
    fn parser_warnings_route_to_review() {
        let (connection, settings) = setup_connection();
        insert_file(
            &connection,
            &settings,
            3,
            "Warned.package",
            ".package",
            "CAS",
            None,
            0.9,
            "mods",
            &["low_confidence_parse"],
            &[],
        );

        let plan = generate_sorting_preview_plan(&connection, &settings, selected_request(vec![3]))
            .expect("plan");
        let item = first_item(&plan);

        assert_eq!(item.action_kind, StagingPlanActionKind::SuggestReview);
        assert_eq!(item.bucket, StagingPlanBucket::NeedsReview);
        assert!(item
            .blocked_reasons
            .contains(&"low_confidence_parse".to_owned()));
    }

    #[test]
    fn inspection_warnings_route_to_review() {
        let (connection, settings) = setup_connection();
        insert_file(
            &connection,
            &settings,
            4,
            "DeepScript.ts4script",
            ".ts4script",
            "ScriptMods",
            None,
            0.9,
            "mods",
            &[],
            &["script_too_deep"],
        );

        let plan = generate_sorting_preview_plan(&connection, &settings, selected_request(vec![4]))
            .expect("plan");
        let item = first_item(&plan);

        assert_eq!(item.action_kind, StagingPlanActionKind::SuggestReview);
        assert_eq!(item.bucket, StagingPlanBucket::NeedsReview);
        assert!(item.blocked_reasons.contains(&"script_too_deep".to_owned()));
    }

    #[test]
    fn weak_filename_only_clue_does_not_create_strong_move() {
        let (connection, settings) = setup_connection();
        insert_file(
            &connection,
            &settings,
            5,
            "maybe_hair.package",
            ".package",
            "Unknown",
            None,
            0.31,
            "mods",
            &[],
            &[],
        );

        let plan = generate_sorting_preview_plan(&connection, &settings, selected_request(vec![5]))
            .expect("plan");
        let item = first_item(&plan);

        assert_eq!(item.action_kind, StagingPlanActionKind::SuggestReview);
        assert_eq!(item.evidence_level, StagingPlanEvidenceLevel::Heuristic);
        assert_eq!(item.bucket, StagingPlanBucket::NeedsReview);
        assert!(item.suggested_destination_path.is_none());
    }

    #[test]
    fn script_file_can_preview_script_mods_bucket_with_caveat() {
        let (connection, settings) = setup_connection();
        insert_file(
            &connection,
            &settings,
            6,
            "Script.ts4script",
            ".ts4script",
            "ScriptMods",
            None,
            0.9,
            "mods",
            &[],
            &[],
        );

        let plan = generate_sorting_preview_plan(&connection, &settings, selected_request(vec![6]))
            .expect("plan");
        let item = first_item(&plan);

        assert_eq!(item.bucket, StagingPlanBucket::ScriptMods);
        assert_eq!(item.action_kind, StagingPlanActionKind::SuggestMove);
        assert!(item
            .caveats
            .iter()
            .any(|value| value.contains("Script placement")));
    }

    #[test]
    fn strong_build_buy_metadata_can_preview_build_buy_bucket() {
        let (connection, settings) = setup_connection();
        insert_file(
            &connection,
            &settings,
            7,
            "Chair.package",
            ".package",
            "BuildBuy",
            Some("Chair"),
            0.88,
            "mods",
            &[],
            &[],
        );

        let plan = generate_sorting_preview_plan(&connection, &settings, selected_request(vec![7]))
            .expect("plan");
        let item = first_item(&plan);

        assert_eq!(item.bucket, StagingPlanBucket::BuildBuy);
        assert_eq!(item.action_kind, StagingPlanActionKind::SuggestMove);
    }

    #[test]
    fn tray_source_routes_to_tray_review_plan() {
        let (connection, settings) = setup_connection();
        insert_file(
            &connection,
            &settings,
            8,
            "Family.trayitem",
            ".trayitem",
            "TrayHousehold",
            None,
            0.9,
            "tray",
            &[],
            &[],
        );

        let plan = generate_sorting_preview_plan(&connection, &settings, selected_request(vec![8]))
            .expect("plan");
        let item = first_item(&plan);

        assert_eq!(item.bucket, StagingPlanBucket::Tray);
        assert_eq!(item.action_kind, StagingPlanActionKind::SuggestReview);
        assert!(item.suggested_destination_path.is_none());
    }

    #[test]
    fn missing_target_root_blocks_destination_path() {
        let (connection, mut settings) = setup_connection();
        let insert_settings = settings.clone();
        insert_file(
            &connection,
            &insert_settings,
            9,
            "Loose/Hair.package",
            ".package",
            "CAS",
            Some("Hair"),
            0.86,
            "mods",
            &[],
            &[],
        );
        settings.mods_path = None;

        let plan = generate_sorting_preview_plan(&connection, &settings, selected_request(vec![9]))
            .expect("plan");
        let item = first_item(&plan);

        assert_eq!(item.action_kind, StagingPlanActionKind::SuggestReview);
        assert_eq!(item.bucket, StagingPlanBucket::NeedsReview);
        assert!(item
            .blocked_reasons
            .contains(&"missing_configured_target_root".to_owned()));
        assert!(item.suggested_destination_path.is_none());
    }

    #[test]
    fn folder_scope_is_bounded_by_limit() {
        let (connection, settings) = setup_connection();
        for id in 10..20 {
            insert_file(
                &connection,
                &settings,
                id,
                &format!("Batch/File{id}.package"),
                ".package",
                "CAS",
                None,
                0.82,
                "mods",
                &[],
                &[],
            );
        }

        let plan = generate_sorting_preview_plan(
            &connection,
            &settings,
            GenerateSortingPreviewPlanRequest {
                scope: GenerateSortingPreviewPlanScope::LibraryFolder {
                    source_location: "mods".to_owned(),
                    folder_path: "Mods/Batch".to_owned(),
                    recursive: false,
                    limit: Some(3),
                },
                folder_config: None,
                context_trail: Vec::new(),
            },
        )
        .expect("plan");

        assert_eq!(plan.item_count, 3);
        assert!(plan.items.iter().all(|item| !item.would_touch_files));
    }

    #[test]
    fn empty_scope_returns_blocked_plan() {
        let (connection, settings) = setup_connection();

        let plan = generate_sorting_preview_plan(&connection, &settings, selected_request(vec![]))
            .expect("plan");

        assert_eq!(plan.status, StagingPlanStatus::Blocked);
        assert_eq!(plan.item_count, 0);
        assert!(plan.items.is_empty());
        assert!(!plan.would_touch_files);
    }

    #[test]
    fn generator_source_does_not_call_file_changing_paths() {
        let source = include_str!("sorting_plan.rs");
        let implementation_source = source.split("#[cfg(test)]").next().unwrap_or(source);
        for forbidden in [
            concat!("apply_", "preview_organization"),
            concat!("apply_", "preview_moves"),
            concat!("apply_", "preview_moves_for_files"),
            concat!("commit_", "staging_area"),
            concat!("commit_", "all_staging_areas"),
            concat!("cleanup_", "staging_areas"),
            concat!("move_", "single_file"),
            concat!("std::fs::", "rename"),
            concat!("std::fs::", "remove"),
        ] {
            assert!(
                !implementation_source.contains(forbidden),
                "sorting preview generator must not call {forbidden}"
            );
        }
    }
}
