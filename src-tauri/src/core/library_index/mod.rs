use rusqlite::{params, params_from_iter, types::Value, Connection, OptionalExtension};
use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    path::Path,
};

use crate::{
    core::{content_versions, duplicate_detector, scanner},
    database,
    error::AppResult,
    models::{
        CategoryOverrideInfo, CreatorLearningInfo, FileDetail, FileInsights, FolderTreeMetadata,
        FolderTreeNode, HomeOverview, LibraryFacets, LibraryFileRow, LibraryFolderFilesQuery,
        LibraryListResponse, LibraryPreviewDiagnostics, LibraryQuery, LibrarySettings,
        LibrarySortField, LibrarySummary, LibraryWatchFilter, ProblemSignal,
        ProblemSignalDestination, ProblemSignalProofLevel, ProblemSignalSeverity, WatchStatus,
    },
    seed::{SeedPack, TaxonomySeed},
};

const DEFAULT_FOLDER_QUERY_LIMIT: i64 = 500;
const MAX_FOLDER_QUERY_LIMIT: i64 = 1_000;

struct ProblemSignalInput<'a> {
    kind: &'a str,
    confidence: f64,
    source_location: &'a str,
    creator: Option<&'a str>,
    creator_hints: &'a [String],
    safety_notes: &'a [String],
    parser_warnings: &'a [String],
    review_reasons: &'a [String],
    has_review_queue: bool,
    has_duplicate: bool,
    watch_status: Option<WatchStatus>,
}

fn humanize_problem_code(code: &str) -> String {
    match code {
        "low_confidence_parse" => "Low classification confidence".to_owned(),
        "inspection_failed" => "Inspection failed during scan".to_owned(),
        "unsafe_script_depth" => {
            "Script file is nested deeper than the safe script depth".to_owned()
        }
        "tray_file_in_mods_root" => "Tray content is sitting in Mods".to_owned(),
        "no_category_detected" => "Category could not be confirmed".to_owned(),
        "conflicting_category_signals" => "Category clues conflict".to_owned(),
        "conflicting_creator_signals" => "Creator clues conflict".to_owned(),
        other => other.replace('_', " "),
    }
}

fn unique_problem_evidence(values: impl IntoIterator<Item = String>) -> Vec<String> {
    let mut seen = BTreeSet::new();
    values
        .into_iter()
        .filter(|value| seen.insert(value.clone()))
        .collect()
}

fn review_reason_evidence(input: &ProblemSignalInput<'_>) -> Vec<String> {
    if !input.review_reasons.is_empty() {
        return unique_problem_evidence(
            input
                .review_reasons
                .iter()
                .map(|reason| humanize_problem_code(reason)),
        );
    }

    let mut reasons = Vec::new();
    if input.confidence < 0.55 {
        reasons.push(humanize_problem_code("low_confidence_parse"));
    }
    reasons.extend(
        input
            .safety_notes
            .iter()
            .map(|reason| humanize_problem_code(reason)),
    );
    reasons.extend(
        input
            .parser_warnings
            .iter()
            .map(|reason| humanize_problem_code(reason)),
    );
    unique_problem_evidence(reasons)
}

fn build_problem_signals(input: &ProblemSignalInput<'_>) -> Vec<ProblemSignal> {
    let mut signals = Vec::new();
    let review_evidence = review_reason_evidence(input);
    let has_inspection_failure = input
        .review_reasons
        .iter()
        .any(|reason| reason == "inspection_failed")
        || input
            .parser_warnings
            .iter()
            .any(|reason| reason == "inspection_failed");

    if input.has_review_queue {
        signals.push(ProblemSignal {
            signal_type: if has_inspection_failure {
                "inspection_failed".to_owned()
            } else {
                "review_suggested".to_owned()
            },
            severity: ProblemSignalSeverity::Warning,
            proof_level: ProblemSignalProofLevel::Confirmed,
            short_label: if has_inspection_failure {
                "Could not inspect fully".to_owned()
            } else {
                "Review suggested".to_owned()
            },
            explanation: if has_inspection_failure {
                "SimSuite could not inspect this file cleanly during scan, so a manual review is suggested.".to_owned()
            } else if !input.safety_notes.is_empty() {
                "This file tripped scan rules that deserve a closer look.".to_owned()
            } else {
                "SimSuite found warning signals that are worth a closer look.".to_owned()
            },
            source: "review_queue".to_owned(),
            evidence: review_evidence.clone(),
            destination: Some(ProblemSignalDestination::Review),
            show_in_library: true,
            show_in_inspector: true,
            show_in_more_details: true,
            show_in_needs_review: true,
        });
    }

    if !input.safety_notes.is_empty() {
        signals.push(ProblemSignal {
            signal_type: "safety_note".to_owned(),
            severity: ProblemSignalSeverity::Warning,
            proof_level: ProblemSignalProofLevel::Confirmed,
            short_label: "Inspection warning".to_owned(),
            explanation: "This file matched a scan safety rule that deserves attention.".to_owned(),
            source: "safety_notes".to_owned(),
            evidence: unique_problem_evidence(
                input
                    .safety_notes
                    .iter()
                    .map(|note| humanize_problem_code(note)),
            ),
            destination: Some(ProblemSignalDestination::Review),
            show_in_library: !input.has_review_queue,
            show_in_inspector: true,
            show_in_more_details: true,
            show_in_needs_review: true,
        });
    }

    if !input.parser_warnings.is_empty() {
        signals.push(ProblemSignal {
            signal_type: if has_inspection_failure {
                "inspection_failed".to_owned()
            } else {
                "parser_warning".to_owned()
            },
            severity: ProblemSignalSeverity::Caution,
            proof_level: ProblemSignalProofLevel::Detected,
            short_label: if has_inspection_failure {
                "Could not inspect fully".to_owned()
            } else {
                "Metadata warning".to_owned()
            },
            explanation: if has_inspection_failure {
                "SimSuite could not finish inspecting this file cleanly during scan.".to_owned()
            } else {
                "SimSuite found metadata or classification clues that conflict or stayed incomplete.".to_owned()
            },
            source: "parser_warnings".to_owned(),
            evidence: unique_problem_evidence(
                input
                    .parser_warnings
                    .iter()
                    .map(|warning| humanize_problem_code(warning)),
            ),
            destination: Some(ProblemSignalDestination::Review),
            show_in_library: !input.has_review_queue && input.safety_notes.is_empty(),
            show_in_inspector: true,
            show_in_more_details: true,
            show_in_needs_review: true,
        });
    }

    if input.has_duplicate {
        signals.push(ProblemSignal {
            signal_type: "duplicate_candidate".to_owned(),
            severity: ProblemSignalSeverity::Caution,
            proof_level: ProblemSignalProofLevel::Detected,
            short_label: "Duplicate".to_owned(),
            explanation: "SimSuite found another file with the same file contents. Compare before changing either copy.".to_owned(),
            source: "duplicates".to_owned(),
            evidence: vec!["Same file contents".to_owned()],
            destination: Some(ProblemSignalDestination::Duplicates),
            show_in_library: false,
            show_in_inspector: true,
            show_in_more_details: true,
            show_in_needs_review: false,
        });
    }

    if input.source_location == "tray" {
        signals.push(ProblemSignal {
            signal_type: "stored_in_tray".to_owned(),
            severity: ProblemSignalSeverity::Info,
            proof_level: ProblemSignalProofLevel::Confirmed,
            short_label: "Stored in Tray".to_owned(),
            explanation: "This file is stored in Tray as library content, not as an active mod."
                .to_owned(),
            source: "source_location".to_owned(),
            evidence: vec!["source_location = tray".to_owned()],
            destination: None,
            show_in_library: true,
            show_in_inspector: true,
            show_in_more_details: true,
            show_in_needs_review: false,
        });
    }

    if input.source_location != "tray"
        && matches!(
            input
                .watch_status
                .clone()
                .unwrap_or(WatchStatus::NotWatched),
            WatchStatus::NotWatched
        )
    {
        signals.push(ProblemSignal {
            signal_type: "no_update_source".to_owned(),
            severity: ProblemSignalSeverity::Info,
            proof_level: ProblemSignalProofLevel::Confirmed,
            short_label: "No update source".to_owned(),
            explanation: "SimSuite is not tracking an update source for this file yet.".to_owned(),
            source: "watch_status".to_owned(),
            evidence: vec!["watch_status = not_watched".to_owned()],
            destination: Some(ProblemSignalDestination::Updates),
            show_in_library: false,
            show_in_inspector: true,
            show_in_more_details: true,
            show_in_needs_review: false,
        });
    }

    if input.confidence < 0.55 {
        signals.push(ProblemSignal {
            signal_type: "weak_metadata".to_owned(),
            severity: ProblemSignalSeverity::Caution,
            proof_level: ProblemSignalProofLevel::Detected,
            short_label: "Weak metadata".to_owned(),
            explanation: "SimSuite classified this file with low confidence, so the current metadata may need a manual check.".to_owned(),
            source: "confidence".to_owned(),
            evidence: vec![format!("confidence = {:.2}", input.confidence)],
            destination: Some(ProblemSignalDestination::Review),
            show_in_library: false,
            show_in_inspector: true,
            show_in_more_details: true,
            show_in_needs_review: false,
        });
    }

    if input.creator.is_none() {
        signals.push(ProblemSignal {
            signal_type: "missing_creator_metadata".to_owned(),
            severity: ProblemSignalSeverity::Caution,
            proof_level: ProblemSignalProofLevel::Confirmed,
            short_label: "Creator unclear".to_owned(),
            explanation: "SimSuite did not confirm a creator name for this file yet.".to_owned(),
            source: "creator_metadata".to_owned(),
            evidence: if input.creator_hints.is_empty() {
                vec!["No creator was confirmed".to_owned()]
            } else {
                vec![format!(
                    "{} unconfirmed creator hint{} found",
                    input.creator_hints.len(),
                    if input.creator_hints.len() == 1 {
                        ""
                    } else {
                        "s"
                    }
                )]
            },
            destination: None,
            show_in_library: false,
            show_in_inspector: true,
            show_in_more_details: true,
            show_in_needs_review: false,
        });
    }

    if input.kind == "ScriptMods" {
        signals.push(ProblemSignal {
            signal_type: "script_mod_caution".to_owned(),
            severity: ProblemSignalSeverity::Info,
            proof_level: ProblemSignalProofLevel::Confirmed,
            short_label: "Script mod caution".to_owned(),
            explanation: "This is a script mod, so check the mod notes before disabling, moving, or deleting it.".to_owned(),
            source: "file_kind".to_owned(),
            evidence: vec!["kind = ScriptMods".to_owned()],
            destination: None,
            show_in_library: false,
            show_in_inspector: true,
            show_in_more_details: true,
            show_in_needs_review: false,
        });
    }

    signals
}

fn primary_problem_signal(input: &ProblemSignalInput<'_>) -> Option<ProblemSignal> {
    build_problem_signals(input)
        .into_iter()
        .find(|signal| signal.show_in_library)
}

pub fn get_home_overview(
    connection: &Connection,
    settings: &LibrarySettings,
    seed_pack: &SeedPack,
) -> AppResult<HomeOverview> {
    let total_files = scalar(connection, "SELECT COUNT(*) FROM files")?;
    let mods_count = scalar(
        connection,
        "SELECT COUNT(*) FROM files WHERE kind NOT LIKE 'Tray%'",
    )?;
    let tray_count = scalar(
        connection,
        "SELECT COUNT(*) FROM files WHERE kind LIKE 'Tray%'",
    )?;
    let downloads_count = scalar(
        connection,
        "SELECT COUNT(*) FROM files WHERE source_location = 'downloads'",
    )?;
    let script_mods_count = scalar(
        connection,
        "SELECT COUNT(*) FROM files WHERE kind = 'ScriptMods'",
    )?;
    let creator_count = scalar(
        connection,
        "SELECT COUNT(DISTINCT creator_id) FROM files WHERE creator_id IS NOT NULL",
    )?;
    let bundles_count = scalar(connection, "SELECT COUNT(*) FROM bundles")?;
    let duplicates_count = scalar(connection, &exact_duplicate_pair_count_sql())?;
    let review_count = scalar(connection, "SELECT COUNT(*) FROM review_queue")?;
    let unsafe_count = scalar(
        connection,
        "SELECT COUNT(*) FROM files WHERE safety_notes <> '[]'",
    )?;
    let silent_special_mod_updates = {
        let val = connection
            .query_row(
                "SELECT value FROM app_settings WHERE key = 'silent_special_mod_updates'",
                [],
                |row| row.get::<_, String>(0),
            )
            .ok();
        val.as_deref() == Some("true")
    };
    let (exact_update_items, possible_update_items, check_failed_watch_items, unknown_watch_items) =
        content_versions::load_watch_counts(connection, silent_special_mod_updates)?;
    let watch_review_items =
        content_versions::list_library_watch_review_items(connection, settings, seed_pack, 1)?
            .total;
    let watch_setup_items =
        content_versions::list_library_watch_setup_items(connection, settings, seed_pack, 1)?.total;
    let last_scan_at = connection
        .query_row(
            "SELECT completed_at
             FROM scan_sessions
             WHERE completed_at IS NOT NULL
             ORDER BY started_at DESC
             LIMIT 1",
            [],
            |row| row.get::<_, String>(0),
        )
        .optional()?;
    let scan_needs_refresh = scanner::library_scan_needs_refresh(connection, seed_pack)?;

    Ok(HomeOverview {
        total_files,
        mods_count,
        tray_count,
        downloads_count,
        script_mods_count,
        creator_count,
        bundles_count,
        duplicates_count,
        review_count,
        unsafe_count,
        exact_update_items,
        possible_update_items,
        check_failed_watch_items,
        unknown_watch_items,
        watch_review_items,
        watch_setup_items,
        last_scan_at,
        scan_needs_refresh,
        read_only_mode: true,
    })
}

pub fn get_library_facets(
    connection: &Connection,
    taxonomy: &TaxonomySeed,
    kind_filter: Option<&str>,
) -> AppResult<LibraryFacets> {
    let creators = string_list(
        connection,
        "SELECT DISTINCT c.canonical_name
         FROM files f
         JOIN creators c ON f.creator_id = c.id
         WHERE f.source_location <> 'downloads'
         ORDER BY c.canonical_name COLLATE NOCASE",
    )?;
    let kinds = string_list(
        connection,
        "SELECT DISTINCT kind
         FROM files
         WHERE source_location <> 'downloads'
         ORDER BY kind COLLATE NOCASE",
    )?;
    // Subtypes are scoped to the selected kind when kind_filter is provided.
    // This makes the subtype chips in the UI truthful — they only show subtypes
    // that actually exist on files of the selected kind.
    let subtypes = if let Some(kind) = kind_filter {
        // Parameterized query — kind is bound as ?1
        let sql = "SELECT DISTINCT subtype
                   FROM files
                   WHERE source_location <> 'downloads'
                     AND kind = ?1
                     AND subtype IS NOT NULL
                     AND subtype <> ''
                   ORDER BY subtype COLLATE NOCASE";
        let mut stmt = connection.prepare(sql)?;
        let items = stmt
            .query_map([kind], |row| row.get(0))?
            .collect::<Result<Vec<String>, _>>()
            .map_err(crate::error::AppError::from)?;
        items
    } else {
        string_list(
            connection,
            "SELECT DISTINCT subtype
             FROM files
             WHERE source_location <> 'downloads'
               AND subtype IS NOT NULL
               AND subtype <> ''
             ORDER BY subtype COLLATE NOCASE",
        )?
    };
    let sources = string_list(
        connection,
        "SELECT DISTINCT source_location
         FROM files
         WHERE source_location <> 'downloads'
         ORDER BY source_location COLLATE NOCASE",
    )?;

    Ok(LibraryFacets {
        creators,
        kinds,
        subtypes,
        sources,
        taxonomy_kinds: taxonomy
            .kinds
            .iter()
            .chain(taxonomy.tray_kinds.iter())
            .cloned()
            .collect(),
    })
}

/// Returns summary counts for the Library strip, filtered to installed content only.
pub fn get_library_summary(connection: &Connection) -> AppResult<LibrarySummary> {
    let total = scalar(
        connection,
        "SELECT COUNT(*) FROM files WHERE source_location <> 'downloads'",
    )?;

    let tracked = scalar(
        connection,
        "SELECT COUNT(DISTINCT f.id)\
         FROM files f\
         JOIN content_watch_sources cws ON cws.anchor_file_id = f.id\
         WHERE f.source_location <> 'downloads'",
    )?;

    let not_tracked = scalar(
        connection,
        "SELECT COUNT(*)\
         FROM files f\
         LEFT JOIN content_watch_sources cws ON cws.anchor_file_id = f.id\
         WHERE f.source_location <> 'downloads' AND cws.subject_key IS NULL",
    )?;

    let has_updates = scalar(
        connection,
        "SELECT COUNT(DISTINCT f.id)\
         FROM files f\
         JOIN content_watch_sources cws ON cws.anchor_file_id = f.id\
         JOIN content_watch_results cwr ON cwr.subject_key = cws.subject_key\
         WHERE f.source_location <> 'downloads'\
         AND cwr.status IN ('exact_update_available', 'possible_update')",
    )?;

    let needs_review = scalar(
        connection,
        "SELECT COUNT(*)\
         FROM files f\
         WHERE f.source_location <> 'downloads'\
         AND (f.safety_notes <> '[]' OR f.parser_warnings <> '[]')",
    )?;

    let duplicates = scalar(connection, &exact_duplicate_file_count_sql())?;

    let disabled = scalar(
        connection,
        "SELECT COUNT(*) FROM files WHERE source_location = 'tray'",
    )?;

    Ok(LibrarySummary {
        total: total as i64,
        tracked: tracked as i64,
        not_tracked: not_tracked as i64,
        has_updates: has_updates as i64,
        needs_review: needs_review as i64,
        duplicates: duplicates as i64,
        disabled: disabled as i64,
    })
}

pub fn get_library_preview_diagnostics(
    connection: &Connection,
) -> AppResult<LibraryPreviewDiagnostics> {
    let mut diagnostics = LibraryPreviewDiagnostics {
        deferred_extraction_enabled: crate::core::file_inspector::THUMBNAIL_DEFERRED,
        ..Default::default()
    };

    let mut statement = connection.prepare(
        "SELECT source_location, extension, insights
         FROM files
         WHERE source_location <> 'downloads'",
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

    for (source_location, extension, insights_json) in rows {
        let insights = parse_insights(insights_json);
        let has_embedded = has_preview_payload(&insights.thumbnail_preview);
        let has_cached = has_preview_payload(&insights.cached_thumbnail_preview);
        let has_preview = has_embedded || has_cached;
        let source = source_location.trim().to_ascii_lowercase();
        let extension = extension.trim().to_ascii_lowercase();
        let is_package = extension == ".package" && source != "tray";
        let is_script = extension == ".ts4script";
        let is_tray = source == "tray";

        diagnostics.total_rows += 1;
        if has_preview {
            diagnostics.rows_with_preview += 1;
        } else {
            diagnostics.rows_without_preview += 1;
        }
        if has_embedded {
            diagnostics.embedded_preview_rows += 1;
        }
        if has_cached {
            diagnostics.cached_preview_rows += 1;
        }

        if is_package {
            diagnostics.package_rows += 1;
            if has_preview {
                diagnostics.package_rows_with_preview += 1;
            } else {
                diagnostics.package_rows_deferred_or_missing += 1;
            }
            continue;
        }

        diagnostics.unsupported_rows += 1;
        if !has_preview {
            diagnostics.unsupported_without_preview += 1;
        }
        if is_script {
            diagnostics.script_rows += 1;
        } else if is_tray {
            diagnostics.tray_rows += 1;
        } else {
            diagnostics.other_unsupported_rows += 1;
        }
    }

    Ok(diagnostics)
}

pub fn list_library_files(
    connection: &Connection,
    query: LibraryQuery,
) -> AppResult<LibraryListResponse> {
    list_library_files_scoped(connection, query, "", Vec::new())
}

fn list_library_files_scoped(
    connection: &Connection,
    query: LibraryQuery,
    extra_filters: &str,
    extra_params: Vec<Value>,
) -> AppResult<LibraryListResponse> {
    let include_previews = query.include_previews.unwrap_or(true);
    let compact_paged_rows = query.limit.is_some();
    let (filters, params) = build_filters(&query);
    let scoped_filters = format!("{filters}{extra_filters}");
    let mut scoped_params = params.clone();
    scoped_params.extend(extra_params);
    let order_by = build_order_by(query.sort_by);
    let exact_duplicate_exists = exact_duplicate_exists_sql("f.id");

    let total_sql = format!(
        "SELECT COUNT(*)\n\
         FROM files f\n\
         LEFT JOIN creators c ON f.creator_id = c.id\n\
         LEFT JOIN bundles b ON f.bundle_id = b.id\n\
         LEFT JOIN content_watch_sources cws ON cws.anchor_file_id = f.id\n\
         LEFT JOIN content_watch_results cwr ON cwr.subject_key = cws.subject_key\n\
         WHERE f.source_location <> 'downloads'\n\
        {scoped_filters}",
        scoped_filters = scoped_filters
    );

    let total =
        connection.query_row(&total_sql, params_from_iter(scoped_params.iter()), |row| {
            row.get(0)
        })?;
    let peer_counts = load_relationship_peer_counts(connection, &scoped_filters, &scoped_params)?;

    // Pagination: only apply LIMIT/OFFSET when query.limit is explicitly set.
    // When limit is None (tree-mode), return all filtered rows without LIMIT/OFFSET.
    let rows_sql = if query.limit.is_some() {
        let limit = query.limit.unwrap_or(100);
        let offset = query.offset.unwrap_or(0);
        let mut row_params = scoped_params.clone();
        row_params.push(Value::Integer(limit));
        row_params.push(Value::Integer(offset));
        format!(
            "SELECT\n\
             f.id,\n\
             f.filename,\n\
             f.path,\n\
             f.extension,\n\
             f.kind,\n\
             f.subtype,\n\
             f.confidence,\n\
             f.source_location,\n\
             f.size,\n\
             f.modified_at,\n\
             c.canonical_name,\n\
             b.bundle_name,\n\
             b.bundle_type,\n\
             b.file_count,\n\
             f.relative_depth,\n\
             f.safety_notes,\n\
             f.parser_warnings,\n\
             f.insights,\n\
             cwr.status,\n\
             {exact_duplicate_exists} AS has_duplicate,
\
             EXISTS (\n\
               SELECT 1 FROM review_queue rq\n\
               WHERE rq.file_id = f.id\n\
             ) AS has_review_queue,
\
             0 AS same_pack_peer_count
\
             FROM files f\n\
             LEFT JOIN creators c ON f.creator_id = c.id\n\
             LEFT JOIN bundles b ON f.bundle_id = b.id\n\
             LEFT JOIN content_watch_sources cws ON cws.anchor_file_id = f.id\n\
             LEFT JOIN content_watch_results cwr ON cwr.subject_key = cws.subject_key\n\
             WHERE f.source_location <> 'downloads'\n\
            {scoped_filters}\n\
             {order_by}\n\
             LIMIT ? OFFSET ?",
            scoped_filters = scoped_filters,
            order_by = order_by,
            exact_duplicate_exists = exact_duplicate_exists
        )
    } else {
        format!(
            "SELECT\n\
             f.id,\n\
             f.filename,\n\
             f.path,\n\
             f.extension,\n\
             f.kind,\n\
             f.subtype,\n\
             f.confidence,\n\
             f.source_location,\n\
             f.size,\n\
             f.modified_at,\n\
             c.canonical_name,\n\
             b.bundle_name,\n\
             b.bundle_type,\n\
             b.file_count,\n\
             f.relative_depth,\n\
             f.safety_notes,\n\
             f.parser_warnings,\n\
             f.insights,\n\
             cwr.status,\n\
             {exact_duplicate_exists} AS has_duplicate,
\
             EXISTS (\n\
               SELECT 1 FROM review_queue rq\n\
               WHERE rq.file_id = f.id\n\
             ) AS has_review_queue,
\
             0 AS same_pack_peer_count
\
             FROM files f\n\
             LEFT JOIN creators c ON f.creator_id = c.id\n\
             LEFT JOIN bundles b ON f.bundle_id = b.id\n\
             LEFT JOIN content_watch_sources cws ON cws.anchor_file_id = f.id\n\
             LEFT JOIN content_watch_results cwr ON cwr.subject_key = cws.subject_key\n\
             WHERE f.source_location <> 'downloads'\n\
            {scoped_filters}\n\
             {order_by}",
            scoped_filters = scoped_filters,
            order_by = order_by,
            exact_duplicate_exists = exact_duplicate_exists
        )
    };

    let row_params = if query.limit.is_some() {
        let limit = query.limit.unwrap_or(100);
        let offset = query.offset.unwrap_or(0);
        let mut p = scoped_params.clone();
        p.push(Value::Integer(limit));
        p.push(Value::Integer(offset));
        p
    } else {
        scoped_params.clone()
    };

    let mut statement = connection.prepare(&rows_sql)?;
    let items = statement
        .query_map(params_from_iter(row_params.iter()), |row| {
            let id = row.get::<_, i64>(0)?;
            let (same_folder_peer_count, same_pack_peer_count) =
                peer_counts.get(&id).copied().unwrap_or_default();
            let watch_status_str: Option<String> = row.get(18)?;
            let watch_status = watch_status_str
                .map(|s| match s.as_str() {
                    "current" => WatchStatus::Current,
                    "exact_update_available" => WatchStatus::ExactUpdateAvailable,
                    "possible_update" => WatchStatus::PossibleUpdate,
                    "check_failed" => WatchStatus::CheckFailed,
                    "reminder_only" => WatchStatus::ReminderOnly,
                    "unknown" => WatchStatus::Unknown,
                    _ => WatchStatus::NotWatched,
                })
                .unwrap_or_default();
            let kind: String = row.get(4)?;
            let confidence: f64 = row.get(6)?;
            let source_location: String = row.get(7)?;
            let creator: Option<String> = row.get(10)?;
            let safety_notes = parse_string_array(row.get::<_, String>(15)?);
            let parser_warnings = parse_string_array(row.get::<_, String>(16)?);
            let insights = compact_library_row_insights(
                parse_insights(row.get::<_, Option<String>>(17)?),
                include_previews,
                compact_paged_rows,
            );
            let has_duplicate = row.get::<_, i64>(19)? != 0;
            let has_review_queue = row.get::<_, i64>(20)? != 0;
            let primary_problem_signal = primary_problem_signal(&ProblemSignalInput {
                kind: &kind,
                confidence,
                source_location: &source_location,
                creator: creator.as_deref(),
                creator_hints: &insights.creator_hints,
                safety_notes: &safety_notes,
                parser_warnings: &parser_warnings,
                review_reasons: &[],
                has_review_queue,
                has_duplicate,
                watch_status: Some(watch_status.clone()),
            });
            Ok(LibraryFileRow {
                id,
                filename: row.get(1)?,
                path: row.get(2)?,
                extension: row.get(3)?,
                kind,
                subtype: row.get(5)?,
                confidence,
                source_location,
                size: row.get(8)?,
                modified_at: row.get(9)?,
                creator,
                bundle_name: row.get(11)?,
                bundle_type: row.get(12)?,
                grouped_file_count: row.get(13)?,
                relative_depth: row.get(14)?,
                safety_notes,
                parser_warnings,
                insights,
                watch_status,
                has_duplicate,
                installed_version: None,
                same_folder_peer_count,
                same_pack_peer_count,
                primary_problem_signal,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(LibraryListResponse { total, items })
}

pub fn list_library_folder_files(
    connection: &Connection,
    query: LibraryFolderFilesQuery,
) -> AppResult<LibraryListResponse> {
    let target_segments = normalize_virtual_folder_path(&query.folder_path);
    let Some(root) = target_segments.first() else {
        return Ok(LibraryListResponse {
            total: 0,
            items: Vec::new(),
        });
    };
    let Some(source) = source_location_for_folder_root(root) else {
        return Ok(LibraryListResponse {
            total: 0,
            items: Vec::new(),
        });
    };

    let mut filters = query.filters;
    if filters
        .source
        .as_ref()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .is_some_and(|filter_source| !filter_source.eq_ignore_ascii_case(&source))
    {
        return Ok(empty_library_list_response());
    }
    filters.source = None;
    filters.limit = Some(bounded_folder_query_limit(query.limit));
    filters.offset = Some(query.offset.unwrap_or(0).max(0));
    filters.include_previews = query.include_previews.or(filters.include_previews);

    let folder_scope =
        build_folder_scope_filter(connection, &source, &target_segments, query.recursive)?;
    list_library_files_scoped(connection, filters, &folder_scope.sql, folder_scope.params)
}

fn empty_library_list_response() -> LibraryListResponse {
    LibraryListResponse {
        total: 0,
        items: Vec::new(),
    }
}

fn bounded_folder_query_limit(limit: Option<i64>) -> i64 {
    limit
        .unwrap_or(DEFAULT_FOLDER_QUERY_LIMIT)
        .max(0)
        .min(MAX_FOLDER_QUERY_LIMIT)
}

#[derive(Debug)]
struct FolderScopeFilter {
    sql: String,
    params: Vec<Value>,
}

#[derive(Debug)]
struct FolderIdentityScope {
    installation_profile_id: String,
    installation_root_id: String,
    profile_relative_path: String,
    profile_relative_path_key: Option<String>,
}

#[derive(Debug)]
enum FolderIdentityDecision {
    Current(FolderIdentityScope),
    Legacy,
    Blocked,
}

fn build_folder_scope_filter(
    connection: &Connection,
    source: &str,
    target_segments: &[String],
    recursive: bool,
) -> AppResult<FolderScopeFilter> {
    let child_segments = target_segments.iter().skip(1).cloned().collect::<Vec<_>>();
    let child_depth = child_segments.len() as i64;

    match load_folder_identity_decision(connection, source, &child_segments)? {
        FolderIdentityDecision::Current(identity) => {
            return Ok(build_identity_folder_scope_filter(
                source,
                child_depth,
                recursive,
                identity,
            ));
        }
        FolderIdentityDecision::Blocked => {
            return Ok(FolderScopeFilter {
                sql: " AND 1 = 0".to_owned(),
                params: Vec::new(),
            });
        }
        FolderIdentityDecision::Legacy => {}
    }

    let mut sql = String::from(" AND f.source_location = ?");
    let mut params = vec![Value::Text(source.to_owned())];

    if child_segments.is_empty() {
        if !recursive {
            sql.push_str(" AND f.relative_depth = 0");
        }
        return Ok(FolderScopeFilter { sql, params });
    }

    if recursive {
        sql.push_str(" AND f.relative_depth >= ?");
    } else {
        sql.push_str(" AND f.relative_depth = ?");
    }
    params.push(Value::Integer(child_depth));

    sql.push_str(" AND LOWER(REPLACE(f.path, '\\', '/')) LIKE ? ESCAPE '~'");
    params.push(Value::Text(folder_path_like_pattern(
        folder_root_disk_path(connection, source)?.as_deref(),
        &child_segments,
    )));

    Ok(FolderScopeFilter { sql, params })
}

fn load_folder_identity_decision(
    connection: &Connection,
    source: &str,
    child_segments: &[String],
) -> AppResult<FolderIdentityDecision> {
    let profile_relative_path = child_segments.join("/");
    let normalized_relative_path = normalize_path_for_query(&profile_relative_path);
    let row = connection
        .query_row(
            "SELECT installation_profile_id, installation_root_id,
                    profile_relative_path, profile_relative_path_key
             FROM library_folders
             WHERE source_location = ?1
               AND (
                   profile_relative_path COLLATE BINARY = ?2
                   OR (
                       profile_relative_path IS NULL
                       AND normalized_relative_path = ?3 COLLATE NOCASE
                   )
               )
             ORDER BY CASE WHEN profile_relative_path IS NOT NULL THEN 0 ELSE 1 END
             LIMIT 1",
            params![source, profile_relative_path, normalized_relative_path],
            |row| {
                Ok((
                    row.get::<_, Option<String>>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, Option<String>>(3)?,
                ))
            },
        )
        .optional()?;

    let Some((profile_id, root_id, relative_path, relative_path_key)) = row else {
        return Ok(FolderIdentityDecision::Legacy);
    };

    match (profile_id, root_id, relative_path, relative_path_key) {
        (None, None, None, None) => Ok(FolderIdentityDecision::Legacy),
        (Some(profile_id), Some(root_id), Some(relative_path), relative_path_key)
            if !profile_id.trim().is_empty() && !root_id.trim().is_empty() =>
        {
            let Some(active_profile) = database::get_active_game_installation_profile(connection)?
            else {
                return Ok(FolderIdentityDecision::Blocked);
            };
            if active_profile.profile_id != profile_id {
                return Ok(FolderIdentityDecision::Blocked);
            }

            Ok(FolderIdentityDecision::Current(FolderIdentityScope {
                installation_profile_id: profile_id,
                installation_root_id: root_id,
                profile_relative_path: relative_path,
                profile_relative_path_key: relative_path_key.filter(|key| !key.trim().is_empty()),
            }))
        }
        _ => Ok(FolderIdentityDecision::Blocked),
    }
}

fn build_identity_folder_scope_filter(
    source: &str,
    child_depth: i64,
    recursive: bool,
    identity: FolderIdentityScope,
) -> FolderScopeFilter {
    let mut sql = String::from(
        " AND f.source_location = ? AND f.installation_profile_id = ? AND f.installation_root_id = ?",
    );
    let mut params = vec![
        Value::Text(source.to_owned()),
        Value::Text(identity.installation_profile_id),
        Value::Text(identity.installation_root_id),
    ];

    if recursive {
        if child_depth > 0 {
            sql.push_str(" AND f.relative_depth >= ?");
            params.push(Value::Integer(child_depth));
        }
    } else {
        sql.push_str(" AND f.relative_depth = ?");
        params.push(Value::Integer(child_depth));
    }

    if identity.profile_relative_path.is_empty() {
        return FolderScopeFilter { sql, params };
    }

    if let Some(relative_path_key) = identity.profile_relative_path_key {
        sql.push_str(
            " AND f.profile_relative_path_key IS NOT NULL AND substr(f.profile_relative_path_key, 1, length(?)) = ? COLLATE BINARY AND substr(f.profile_relative_path_key, length(?) + 1, 1) = '|'",
        );
        params.push(Value::Text(relative_path_key.clone()));
        params.push(Value::Text(relative_path_key.clone()));
        params.push(Value::Text(relative_path_key));
    } else {
        sql.push_str(
            " AND f.profile_relative_path IS NOT NULL AND substr(f.profile_relative_path, 1, length(?)) = ? COLLATE BINARY AND substr(f.profile_relative_path, length(?) + 1, 1) = '/'",
        );
        params.push(Value::Text(identity.profile_relative_path.clone()));
        params.push(Value::Text(identity.profile_relative_path.clone()));
        params.push(Value::Text(identity.profile_relative_path));
    }

    FolderScopeFilter { sql, params }
}

fn folder_root_disk_path(connection: &Connection, source: &str) -> AppResult<Option<String>> {
    let settings = database::get_library_settings(connection)?;
    let root = match source {
        "mods" => settings.mods_path,
        "tray" => settings.tray_path,
        _ => None,
    };
    Ok(root)
}

fn folder_path_like_pattern(root_path: Option<&str>, child_segments: &[String]) -> String {
    let child_path = child_segments
        .iter()
        .map(|segment| escape_like(&normalize_path_for_query(segment)))
        .collect::<Vec<_>>()
        .join("/");

    if let Some(root_path) = root_path
        .map(normalize_path_for_query)
        .filter(|value| !value.is_empty())
    {
        return format!(
            "{}/{}%",
            escape_like(&root_path),
            folder_child_prefix(&child_path)
        );
    }

    format!("%/{}%", folder_child_prefix(&child_path))
}

fn folder_child_prefix(child_path: &str) -> String {
    if child_path.ends_with('/') {
        child_path.to_owned()
    } else {
        format!("{child_path}/")
    }
}

fn normalize_path_for_query(path: &str) -> String {
    path.trim()
        .replace('\\', "/")
        .trim_matches('/')
        .to_ascii_lowercase()
}

fn escape_like(value: &str) -> String {
    value
        .replace('~', "~~")
        .replace('%', "~%")
        .replace('_', "~_")
}

#[derive(Debug)]
struct RelationshipPeerRow {
    id: i64,
    path: String,
    source_location: String,
    relative_depth: i64,
    bundle_id: Option<i64>,
}

fn load_relationship_peer_counts(
    connection: &Connection,
    filters: &str,
    params: &[Value],
) -> AppResult<HashMap<i64, (i64, i64)>> {
    let sql = format!(
        "SELECT DISTINCT f.id, f.path, f.source_location, f.relative_depth, f.bundle_id\n\
         FROM files f\n\
         LEFT JOIN creators c ON f.creator_id = c.id\n\
         LEFT JOIN content_watch_sources cws ON cws.anchor_file_id = f.id\n\
         LEFT JOIN content_watch_results cwr ON cwr.subject_key = cws.subject_key\n\
         WHERE f.source_location <> 'downloads'\n\
        {filters}",
        filters = filters
    );
    let mut statement = connection.prepare(&sql)?;
    let rows = statement
        .query_map(params_from_iter(params.iter()), |row| {
            Ok(RelationshipPeerRow {
                id: row.get(0)?,
                path: row.get(1)?,
                source_location: row.get(2)?,
                relative_depth: row.get(3)?,
                bundle_id: row.get(4)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    let mut folder_groups: HashMap<(String, String), Vec<i64>> = HashMap::new();
    let mut bundle_groups: HashMap<i64, Vec<i64>> = HashMap::new();

    for row in rows {
        if let Some(folder_segments) =
            folder_segments_for_file(&row.path, &row.source_location, row.relative_depth)
        {
            folder_groups
                .entry((
                    row.source_location.to_ascii_lowercase(),
                    folder_segments.join("/"),
                ))
                .or_default()
                .push(row.id);
        }

        if let Some(bundle_id) = row.bundle_id {
            bundle_groups.entry(bundle_id).or_default().push(row.id);
        }
    }

    let mut counts = HashMap::new();
    for ids in folder_groups.values() {
        let peer_count = ids.len().saturating_sub(1) as i64;
        for id in ids {
            counts.entry(*id).or_insert((0, 0)).0 = peer_count;
        }
    }
    for ids in bundle_groups.values() {
        let peer_count = ids.len().saturating_sub(1) as i64;
        for id in ids {
            counts.entry(*id).or_insert((0, 0)).1 = peer_count;
        }
    }

    Ok(counts)
}

#[derive(Debug)]
struct FolderFileRow {
    path: String,
    source_location: String,
    relative_depth: i64,
    installation_profile_id: Option<String>,
    installation_root_id: Option<String>,
    profile_relative_path: Option<String>,
}

#[derive(Debug)]
struct FolderDirectCountRow {
    source_location: String,
    installation_profile_id: Option<String>,
    installation_root_id: Option<String>,
    profile_parent_relative_path: Option<String>,
    profile_parent_relative_path_key: Option<String>,
    direct_file_count: i64,
}

#[derive(Debug)]
struct FolderTreeFileSelection {
    direct_count_rows: Vec<FolderDirectCountRow>,
    fallback_file_rows: Vec<FolderFileRow>,
}

#[derive(Debug)]
struct FolderMetadataRow {
    source_location: String,
    relative_path: String,
    name: String,
    depth: i64,
    disk_path: String,
    installation_profile_id: Option<String>,
    installation_root_id: Option<String>,
    profile_relative_path: Option<String>,
    profile_relative_path_key: Option<String>,
}

#[derive(Debug, Clone)]
enum FolderTreeSourceMode {
    Current {
        installation_profile_id: String,
        installation_root_ids: BTreeSet<String>,
    },
    Legacy,
    Blocked,
}

#[derive(Debug)]
struct FolderTreeRowSelection {
    folder_rows: Vec<FolderMetadataRow>,
    source_modes: HashMap<String, FolderTreeSourceMode>,
}

#[derive(Debug, Clone)]
struct FolderNodeAccumulator {
    path: String,
    name: String,
    depth: i64,
    source_location: String,
    disk_path: Option<String>,
    direct_file_count: i64,
    total_file_count: i64,
    children: BTreeSet<String>,
}

pub fn get_folder_tree_metadata(
    connection: &Connection,
    query: &LibraryQuery,
) -> AppResult<FolderTreeMetadata> {
    let folder_selection = load_library_folder_rows(connection, query)?;
    let file_selection = load_folder_tree_file_counts(connection, query)?;

    Ok(build_folder_metadata_from_rows(
        folder_selection.folder_rows,
        file_selection.direct_count_rows,
        file_selection.fallback_file_rows,
        &folder_selection.source_modes,
    ))
}

fn load_folder_tree_file_counts(
    connection: &Connection,
    query: &LibraryQuery,
) -> AppResult<FolderTreeFileSelection> {
    let (filters, params) = build_filters(query);
    let grouped_sql = format!(
        "SELECT f.source_location, f.installation_profile_id,\n\
                f.installation_root_id, f.profile_parent_relative_path,\n\
                f.profile_parent_relative_path_key, COUNT(*)\n\
         FROM files f\n\
         LEFT JOIN creators c ON f.creator_id = c.id\n\
         LEFT JOIN content_watch_sources cws ON cws.anchor_file_id = f.id\n\
         LEFT JOIN content_watch_results cwr ON cwr.subject_key = cws.subject_key\n\
         WHERE f.source_location <> 'downloads'\n\
           AND f.profile_parent_relative_path IS NOT NULL\n\
        {filters}\n\
         GROUP BY f.source_location, f.installation_profile_id,\n\
                  f.installation_root_id, f.profile_parent_relative_path,\n\
                  f.profile_parent_relative_path_key\n\
         ORDER BY f.source_location COLLATE NOCASE,\n\
                  f.profile_parent_relative_path COLLATE NOCASE",
        filters = filters
    );
    let direct_count_rows = {
        let mut statement = connection.prepare(&grouped_sql)?;
        let rows = statement
            .query_map(params_from_iter(params.iter()), |row| {
                Ok(FolderDirectCountRow {
                    source_location: row.get(0)?,
                    installation_profile_id: row.get(1)?,
                    installation_root_id: row.get(2)?,
                    profile_parent_relative_path: row.get(3)?,
                    profile_parent_relative_path_key: row.get(4)?,
                    direct_file_count: row.get(5)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        rows
    };

    let fallback_sql = format!(
        "SELECT f.path, f.source_location, f.relative_depth,\n\
                f.installation_profile_id, f.installation_root_id,\n\
                f.profile_relative_path\n\
         FROM files f\n\
         LEFT JOIN creators c ON f.creator_id = c.id\n\
         LEFT JOIN content_watch_sources cws ON cws.anchor_file_id = f.id\n\
         LEFT JOIN content_watch_results cwr ON cwr.subject_key = cws.subject_key\n\
         WHERE f.source_location <> 'downloads'\n\
           AND f.profile_parent_relative_path IS NULL\n\
        {filters}\n\
         ORDER BY f.source_location COLLATE NOCASE, f.path COLLATE NOCASE",
        filters = filters
    );
    let fallback_file_rows = {
        let mut statement = connection.prepare(&fallback_sql)?;
        let rows = statement
            .query_map(params_from_iter(params.iter()), |row| {
                Ok(FolderFileRow {
                    path: row.get(0)?,
                    source_location: row.get(1)?,
                    relative_depth: row.get(2)?,
                    installation_profile_id: row.get(3)?,
                    installation_root_id: row.get(4)?,
                    profile_relative_path: row.get(5)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        rows
    };

    Ok(FolderTreeFileSelection {
        direct_count_rows,
        fallback_file_rows,
    })
}

fn load_library_folder_rows(
    connection: &Connection,
    query: &LibraryQuery,
) -> AppResult<FolderTreeRowSelection> {
    let sources = folder_sources_for_query(query);
    if sources.is_empty() {
        return Ok(FolderTreeRowSelection {
            folder_rows: Vec::new(),
            source_modes: HashMap::new(),
        });
    }

    let placeholders = std::iter::repeat("?")
        .take(sources.len())
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!(
        "SELECT source_location, relative_path, name, depth, full_path,\n\
                installation_profile_id, installation_root_id,\n\
                profile_relative_path, profile_relative_path_key\n\
         FROM library_folders\n\
         WHERE source_location IN ({placeholders})\n\
         ORDER BY source_location COLLATE NOCASE, normalized_relative_path COLLATE NOCASE"
    );
    let params = sources
        .iter()
        .cloned()
        .map(Value::Text)
        .collect::<Vec<Value>>();

    let mut statement = connection.prepare(&sql)?;
    let rows = statement
        .query_map(params_from_iter(params.iter()), |row| {
            Ok(FolderMetadataRow {
                source_location: row.get(0)?,
                relative_path: row.get(1)?,
                name: row.get(2)?,
                depth: row.get(3)?,
                disk_path: row.get(4)?,
                installation_profile_id: row.get(5)?,
                installation_root_id: row.get(6)?,
                profile_relative_path: row.get(7)?,
                profile_relative_path_key: row.get(8)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    let active_profile_id = database::get_active_game_installation_profile(connection)?
        .map(|profile| profile.profile_id);
    let mut rows_by_source: HashMap<String, Vec<FolderMetadataRow>> = HashMap::new();
    for row in rows {
        rows_by_source
            .entry(row.source_location.to_ascii_lowercase())
            .or_default()
            .push(row);
    }

    let mut folder_rows = Vec::new();
    let mut source_modes = HashMap::new();
    for source in sources {
        let rows = rows_by_source.remove(&source).unwrap_or_default();
        let (mode, selected_rows) =
            select_folder_tree_source_rows(active_profile_id.as_deref(), rows);
        folder_rows.extend(selected_rows);
        source_modes.insert(source, mode);
    }

    Ok(FolderTreeRowSelection {
        folder_rows,
        source_modes,
    })
}

fn select_folder_tree_source_rows(
    active_profile_id: Option<&str>,
    rows: Vec<FolderMetadataRow>,
) -> (FolderTreeSourceMode, Vec<FolderMetadataRow>) {
    let mut current_rows = Vec::new();
    let mut legacy_rows = Vec::new();
    let mut has_owned_identity = false;
    let mut has_malformed_identity = false;

    for mut row in rows {
        let identity_is_empty = row.installation_profile_id.is_none()
            && row.installation_root_id.is_none()
            && row.profile_relative_path.is_none()
            && row.profile_relative_path_key.is_none();
        if identity_is_empty {
            legacy_rows.push(row);
            continue;
        }

        has_owned_identity = true;
        let (Some(profile_id), Some(root_id), Some(relative_path)) = (
            row.installation_profile_id.as_deref(),
            row.installation_root_id.as_deref(),
            row.profile_relative_path.as_deref(),
        ) else {
            has_malformed_identity = true;
            continue;
        };
        let comparison_key_is_valid = row
            .profile_relative_path_key
            .as_deref()
            .map(|key| !key.trim().is_empty() && !relative_path.is_empty())
            .unwrap_or(true);
        if profile_id.trim().is_empty()
            || root_id.trim().is_empty()
            || !comparison_key_is_valid
            || !is_safe_profile_relative_path(relative_path, true)
        {
            has_malformed_identity = true;
            continue;
        }

        if active_profile_id == Some(profile_id) {
            row.relative_path = relative_path.to_owned();
            row.depth = profile_relative_folder_depth(relative_path);
            current_rows.push(row);
        }
    }

    if has_malformed_identity {
        return (FolderTreeSourceMode::Blocked, Vec::new());
    }

    if !current_rows.is_empty() {
        let installation_profile_id = active_profile_id.unwrap_or_default().to_owned();
        let installation_root_ids = current_rows
            .iter()
            .filter_map(|row| row.installation_root_id.clone())
            .collect::<BTreeSet<_>>();
        let root_row_count = current_rows
            .iter()
            .filter(|row| row.profile_relative_path.as_deref() == Some(""))
            .count();
        if installation_root_ids.len() != 1 || root_row_count != 1 {
            return (FolderTreeSourceMode::Blocked, Vec::new());
        }
        return (
            FolderTreeSourceMode::Current {
                installation_profile_id,
                installation_root_ids,
            },
            current_rows,
        );
    }

    if has_owned_identity {
        (FolderTreeSourceMode::Blocked, Vec::new())
    } else {
        (FolderTreeSourceMode::Legacy, legacy_rows)
    }
}

fn profile_relative_folder_depth(relative_path: &str) -> i64 {
    relative_path
        .replace('\\', "/")
        .split('/')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .count() as i64
}

fn is_safe_profile_relative_path(relative_path: &str, allow_empty: bool) -> bool {
    let normalized = relative_path.replace('\\', "/");
    if normalized.is_empty() {
        return allow_empty;
    }
    if normalized.starts_with('/') {
        return false;
    }

    let parts = normalized.split('/').map(str::trim).collect::<Vec<_>>();
    if parts
        .iter()
        .any(|part| part.is_empty() || *part == "." || *part == "..")
    {
        return false;
    }
    !parts[0].contains(':')
}

fn folder_sources_for_query(query: &LibraryQuery) -> Vec<String> {
    match query.source.as_deref().map(str::trim) {
        Some(source) if source.eq_ignore_ascii_case("mods") => vec!["mods".to_owned()],
        Some(source) if source.eq_ignore_ascii_case("tray") => vec!["tray".to_owned()],
        Some(source) if !source.is_empty() => Vec::new(),
        _ => vec!["mods".to_owned(), "tray".to_owned()],
    }
}

fn build_folder_metadata_from_rows(
    folder_rows: Vec<FolderMetadataRow>,
    direct_count_rows: Vec<FolderDirectCountRow>,
    fallback_file_rows: Vec<FolderFileRow>,
    source_modes: &HashMap<String, FolderTreeSourceMode>,
) -> FolderTreeMetadata {
    let mut nodes: BTreeMap<String, FolderNodeAccumulator> = BTreeMap::new();

    for row in folder_rows {
        add_folder_metadata_row(&mut nodes, row);
    }

    for row in direct_count_rows {
        if row.direct_file_count <= 0 {
            continue;
        }
        let Some(segments) = folder_segments_for_direct_count(&row, source_modes) else {
            continue;
        };
        add_file_segments(
            &mut nodes,
            &row.source_location,
            segments,
            row.direct_file_count,
        );
    }

    for row in fallback_file_rows {
        let Some(segments) = folder_segments_for_tree_file(&row, source_modes) else {
            continue;
        };
        add_file_segments(&mut nodes, &row.source_location, segments, 1);
    }

    let mut roots = nodes
        .values()
        .filter(|node| node.depth == 0)
        .map(|node| build_folder_node(&node.path, &nodes))
        .collect::<Vec<_>>();
    roots.sort_by(compare_folder_nodes);

    let total_folders = roots
        .iter()
        .map(|root| 1 + count_folder_descendants(&root.children))
        .sum();

    FolderTreeMetadata {
        total_folders,
        roots,
    }
}

fn add_file_segments(
    nodes: &mut BTreeMap<String, FolderNodeAccumulator>,
    source_location: &str,
    segments: Vec<String>,
    file_count: i64,
) {
    if segments.is_empty() || file_count <= 0 {
        return;
    }

    for index in 0..segments.len() {
        let path = segments[..=index].join("/");
        let source_location = source_location.to_ascii_lowercase();
        let entry = nodes
            .entry(path.clone())
            .or_insert_with(|| FolderNodeAccumulator {
                path: path.clone(),
                name: segments[index].clone(),
                depth: index as i64,
                source_location,
                disk_path: None,
                direct_file_count: 0,
                total_file_count: 0,
                children: BTreeSet::new(),
            });
        entry.total_file_count += file_count;
        if index == segments.len() - 1 {
            entry.direct_file_count += file_count;
        }

        if index > 0 {
            let parent_path = segments[..index].join("/");
            if let Some(parent) = nodes.get_mut(&parent_path) {
                parent.children.insert(path.clone());
            }
        }
    }
}

fn add_folder_metadata_row(
    nodes: &mut BTreeMap<String, FolderNodeAccumulator>,
    row: FolderMetadataRow,
) {
    let Some(mut segments) =
        folder_segments_for_folder_row(&row.source_location, &row.relative_path)
    else {
        return;
    };
    if segments.is_empty() {
        return;
    }

    if row.depth == 0 {
        segments[0] = row.name.clone();
    } else if let Some(last) = segments.last_mut() {
        *last = row.name.clone();
    }

    for index in 0..segments.len() {
        let path = segments[..=index].join("/");
        let source_location = row.source_location.to_ascii_lowercase();
        let is_row_node = index == segments.len() - 1;
        let entry = nodes
            .entry(path.clone())
            .or_insert_with(|| FolderNodeAccumulator {
                path: path.clone(),
                name: segments[index].clone(),
                depth: index as i64,
                source_location,
                disk_path: None,
                direct_file_count: 0,
                total_file_count: 0,
                children: BTreeSet::new(),
            });
        if is_row_node {
            entry.name = segments[index].clone();
            entry.depth = index as i64;
            entry.disk_path = Some(row.disk_path.clone());
        }

        if index > 0 {
            let parent_path = segments[..index].join("/");
            if let Some(parent) = nodes.get_mut(&parent_path) {
                parent.children.insert(path.clone());
            }
        }
    }
}

fn build_folder_node(
    path: &str,
    nodes: &BTreeMap<String, FolderNodeAccumulator>,
) -> FolderTreeNode {
    let node = nodes.get(path).expect("folder node exists");
    let mut children = node
        .children
        .iter()
        .map(|child_path| build_folder_node(child_path, nodes))
        .collect::<Vec<_>>();
    children.sort_by(compare_folder_nodes);

    FolderTreeNode {
        path: node.path.clone(),
        name: node.name.clone(),
        depth: node.depth,
        source_location: node.source_location.clone(),
        disk_path: node.disk_path.clone(),
        direct_file_count: node.direct_file_count,
        child_folder_count: children.len() as i64,
        total_file_count: node.total_file_count,
        children,
    }
}

fn count_folder_descendants(nodes: &[FolderTreeNode]) -> i64 {
    nodes
        .iter()
        .map(|node| 1 + count_folder_descendants(&node.children))
        .sum()
}

fn compare_folder_nodes(left: &FolderTreeNode, right: &FolderTreeNode) -> std::cmp::Ordering {
    folder_sort_rank(&left.name)
        .cmp(&folder_sort_rank(&right.name))
        .then_with(|| {
            left.name
                .to_ascii_lowercase()
                .cmp(&right.name.to_ascii_lowercase())
        })
}

fn folder_sort_rank(name: &str) -> u8 {
    match name {
        "Mods" => 0,
        "Tray" => 1,
        _ => 2,
    }
}

fn normalize_virtual_folder_path(path: &str) -> Vec<String> {
    path.replace('\\', "/")
        .split('/')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn source_location_for_folder_root(root: &str) -> Option<String> {
    if root.eq_ignore_ascii_case("mods") {
        return Some("mods".to_owned());
    }
    if root.eq_ignore_ascii_case("tray") {
        return Some("tray".to_owned());
    }
    None
}

fn folder_segments_for_direct_count(
    row: &FolderDirectCountRow,
    source_modes: &HashMap<String, FolderTreeSourceMode>,
) -> Option<Vec<String>> {
    let source_location = row.source_location.to_ascii_lowercase();
    let FolderTreeSourceMode::Current {
        installation_profile_id,
        installation_root_ids,
    } = source_modes.get(&source_location)?
    else {
        return None;
    };
    let (Some(profile_id), Some(root_id), Some(parent_relative_path)) = (
        row.installation_profile_id.as_deref(),
        row.installation_root_id.as_deref(),
        row.profile_parent_relative_path.as_deref(),
    ) else {
        return None;
    };
    if profile_id != installation_profile_id
        || !installation_root_ids.contains(root_id)
        || !is_safe_profile_relative_path(parent_relative_path, true)
    {
        return None;
    }

    match row.profile_parent_relative_path_key.as_deref() {
        Some(key) if key.trim().is_empty() || parent_relative_path.is_empty() => return None,
        _ => {}
    }

    folder_segments_for_folder_row(&source_location, parent_relative_path)
}

fn folder_segments_for_tree_file(
    row: &FolderFileRow,
    source_modes: &HashMap<String, FolderTreeSourceMode>,
) -> Option<Vec<String>> {
    let source_location = row.source_location.to_ascii_lowercase();
    let mode = source_modes.get(&source_location)?;
    match mode {
        FolderTreeSourceMode::Current {
            installation_profile_id,
            installation_root_ids,
        } => {
            let (Some(profile_id), Some(root_id), Some(relative_path)) = (
                row.installation_profile_id.as_deref(),
                row.installation_root_id.as_deref(),
                row.profile_relative_path.as_deref(),
            ) else {
                return None;
            };
            if profile_id != installation_profile_id
                || !installation_root_ids.contains(root_id)
                || !is_safe_profile_relative_path(relative_path, false)
            {
                return None;
            }
            folder_segments_for_profile_file(relative_path, &source_location)
        }
        FolderTreeSourceMode::Legacy => {
            if row.installation_profile_id.is_some()
                || row.installation_root_id.is_some()
                || row.profile_relative_path.is_some()
            {
                return None;
            }
            folder_segments_for_file(&row.path, &source_location, row.relative_depth)
        }
        FolderTreeSourceMode::Blocked => None,
    }
}

fn folder_segments_for_profile_file(
    profile_relative_path: &str,
    source_location: &str,
) -> Option<Vec<String>> {
    if !is_safe_profile_relative_path(profile_relative_path, false) {
        return None;
    }
    let mut components = profile_relative_path
        .replace('\\', "/")
        .split('/')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    components.pop()?;

    let mut segments = Vec::with_capacity(components.len() + 1);
    segments.push(folder_root_name(source_location)?);
    segments.extend(components);
    Some(segments)
}

fn folder_segments_for_file(
    path: &str,
    source_location: &str,
    relative_depth: i64,
) -> Option<Vec<String>> {
    let root = folder_root_name(source_location)?;
    let normalized = path.replace('\\', "/");
    let components = normalized
        .split('/')
        .filter(|part| !part.is_empty())
        .map(|part| part.trim().to_owned())
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    let parent_components = components
        .len()
        .checked_sub(1)
        .map(|last| components[..last].to_vec())
        .unwrap_or_default();
    let folder_depth = relative_depth.max(0) as usize;

    let child_segments = if folder_depth > 0 && parent_components.len() >= folder_depth {
        parent_components[parent_components.len() - folder_depth..].to_vec()
    } else {
        Vec::new()
    };

    let mut segments = Vec::with_capacity(child_segments.len() + 1);
    segments.push(root);
    segments.extend(child_segments);
    Some(segments)
}

fn folder_segments_for_folder_row(
    source_location: &str,
    relative_path: &str,
) -> Option<Vec<String>> {
    let mut segments = vec![folder_root_name(source_location)?];
    segments.extend(
        relative_path
            .replace('\\', "/")
            .split('/')
            .map(str::trim)
            .filter(|part| !part.is_empty())
            .map(ToOwned::to_owned),
    );
    Some(segments)
}

fn folder_root_name(source_location: &str) -> Option<String> {
    let trimmed = source_location.trim();
    if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("downloads") {
        return None;
    }

    if trimmed.eq_ignore_ascii_case("mods") {
        return Some("Mods".to_owned());
    }
    if trimmed.eq_ignore_ascii_case("tray") {
        return Some("Tray".to_owned());
    }

    let mut chars = trimmed.chars();
    let Some(first) = chars.next() else {
        return None;
    };
    Some(format!(
        "{}{}",
        first.to_ascii_uppercase(),
        chars.as_str().to_ascii_lowercase()
    ))
}

pub fn get_file_detail(
    connection: &Connection,
    settings: &LibrarySettings,
    seed_pack: &SeedPack,
    file_id: i64,
) -> AppResult<Option<FileDetail>> {
    let detail = connection
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
                f.size,
                f.modified_at,
                c.canonical_name,
                b.bundle_name,
                b.bundle_type,
                b.file_count,
                f.relative_depth,
                f.safety_notes,
                f.hash,
                f.created_at,
                f.parser_warnings,
                f.insights,
                c.id,
                COALESCE(c.locked_by_user, 0),
                c.preferred_path,
                uco.kind,
                uco.subtype
             FROM files f
             LEFT JOIN creators c ON f.creator_id = c.id
             LEFT JOIN bundles b ON f.bundle_id = b.id
             LEFT JOIN user_category_overrides uco ON uco.match_path = f.path
             WHERE f.id = ?1
               AND f.source_location <> 'downloads'",
            params![file_id],
            |row| {
                Ok(FileDetail {
                    id: row.get(0)?,
                    filename: row.get(1)?,
                    path: row.get(2)?,
                    extension: row.get(3)?,
                    kind: row.get(4)?,
                    subtype: row.get(5)?,
                    confidence: row.get(6)?,
                    source_location: row.get(7)?,
                    size: row.get(8)?,
                    modified_at: row.get(9)?,
                    creator: row.get(10)?,
                    bundle_name: row.get(11)?,
                    bundle_type: row.get(12)?,
                    grouped_file_count: row.get(13)?,
                    relative_depth: row.get(14)?,
                    safety_notes: parse_string_array(row.get::<_, String>(15)?),
                    hash: row.get(16)?,
                    created_at: row.get(17)?,
                    parser_warnings: parse_string_array(row.get::<_, String>(18)?),
                    insights: parse_insights(Some(row.get::<_, String>(19)?)),
                    installed_version_summary: None,
                    watch_result: None,
                    creator_learning: CreatorLearningInfo {
                        locked_by_user: row.get::<_, i64>(21)? != 0,
                        preferred_path: row.get(22)?,
                        learned_aliases: Vec::new(),
                    },
                    category_override: {
                        let kind: Option<String> = row.get(23)?;
                        CategoryOverrideInfo {
                            saved_by_user: kind.is_some(),
                            kind,
                            subtype: row.get(24)?,
                        }
                    },
                    duplicates_count: 0,
                    duplicate_types: Vec::new(),
                    installed_version: None,
                    problem_signals: Vec::new(),
                })
            },
        )
        .optional()?;

    match detail {
        Some(mut detail) => {
            if let Some(creator_name) = detail.creator.as_deref() {
                detail.creator_learning.learned_aliases =
                    list_creator_aliases(connection, creator_name)?;
            }
            let (installed_version_summary, watch_result) =
                content_versions::resolve_library_file_version(
                    connection, settings, seed_pack, file_id,
                )?;
            detail.installed_version_summary = installed_version_summary;
            detail.watch_result = watch_result;

            // Load duplicate info for this file.
            let duplicates_count: i64 = connection.query_row(
                &format!(
                    "SELECT COUNT(*)
                     FROM duplicates d
                     JOIN files da ON d.file_id_a = da.id
                     JOIN files db ON d.file_id_b = db.id
                     WHERE {}",
                    exact_duplicate_for_file_sql("?1")
                ),
                params![file_id],
                |row| row.get(0),
            )?;
            detail.duplicates_count = duplicates_count.max(0) as usize;
            detail.duplicate_types = if detail.duplicates_count > 0 {
                vec!["exact".to_owned()]
            } else {
                Vec::new()
            };

            let review_reasons: Vec<String> = connection
                .prepare(
                    "SELECT DISTINCT reason FROM review_queue
                     WHERE file_id = ?1
                     ORDER BY reason",
                )?
                .query_map(params![file_id], |row| row.get(0))?
                .collect::<Result<Vec<String>, _>>()?;
            detail.problem_signals = build_problem_signals(&ProblemSignalInput {
                kind: &detail.kind,
                confidence: detail.confidence,
                source_location: &detail.source_location,
                creator: detail.creator.as_deref(),
                creator_hints: &detail.insights.creator_hints,
                safety_notes: &detail.safety_notes,
                parser_warnings: &detail.parser_warnings,
                review_reasons: &review_reasons,
                has_review_queue: !review_reasons.is_empty(),
                has_duplicate: detail.duplicates_count > 0,
                watch_status: detail
                    .watch_result
                    .as_ref()
                    .map(|result| result.status.clone()),
            });

            // Resolve thumbnails only for selected package details. This keeps
            // expensive DBPF/cache reads out of ordinary list, grid, and folder
            // browsing while allowing a found preview to be reused later.
            if detail.extension.eq_ignore_ascii_case(".package") {
                use crate::core::file_inspector::resolve_package_thumbnails_deferred;
                let (embedded_thumb, cached_thumb) =
                    resolve_package_thumbnails_deferred(Path::new(&detail.path));
                if merge_preview_results_into_insights(
                    &mut detail.insights,
                    embedded_thumb,
                    cached_thumb,
                ) {
                    persist_file_insights(connection, detail.id, &detail.insights)?;
                }
            }

            Ok(Some(detail))
        }
        None => Ok(None),
    }
}

pub fn build_filters(query: &LibraryQuery) -> (String, Vec<Value>) {
    let mut sql = String::new();
    let mut params = Vec::new();

    if let Some(search) = query
        .search
        .as_ref()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
    {
        sql.push_str(" AND (f.filename LIKE ? OR f.path LIKE ? OR COALESCE(c.canonical_name, '') LIKE ? OR COALESCE(f.subtype, '') LIKE ?)");
        let pattern = format!("%{search}%");
        params.push(Value::Text(pattern.clone()));
        params.push(Value::Text(pattern.clone()));
        params.push(Value::Text(pattern.clone()));
        params.push(Value::Text(pattern));
    }

    if let Some(kind) = query.kind.as_ref().filter(|value| !value.is_empty()) {
        sql.push_str(" AND f.kind = ?");
        params.push(Value::Text(kind.clone()));
    }

    if let Some(subtype) = query.subtype.as_ref().filter(|value| !value.is_empty()) {
        sql.push_str(" AND f.subtype = ?");
        params.push(Value::Text(subtype.clone()));
    }

    if let Some(creator) = query.creator.as_ref().filter(|value| !value.is_empty()) {
        sql.push_str(" AND c.canonical_name = ?");
        params.push(Value::Text(creator.clone()));
    }

    if let Some(source) = query.source.as_ref().filter(|value| !value.is_empty()) {
        sql.push_str(" AND f.source_location = ?");
        params.push(Value::Text(source.clone()));
    }

    if let Some(min_confidence) = query.min_confidence {
        sql.push_str(" AND f.confidence >= ?");
        params.push(Value::Real(min_confidence));
    }

    // Apply watch-state quick filter. Relies on cws/cwr JOIN being present.
    match query.watch_filter.unwrap_or_default() {
        LibraryWatchFilter::HasUpdates => {
            sql.push_str(" AND cwr.status IN ('exact_update_available', 'possible_update')");
        }
        LibraryWatchFilter::NeedsAttention => {
            // Includes both safety notes (genuine concerns) and parser warnings
            // (uncertain metadata) — items the user should manually review.
            sql.push_str(" AND (f.safety_notes <> '[]' OR f.parser_warnings <> '[]')");
        }
        LibraryWatchFilter::NotTracked => {
            sql.push_str(" AND cws.subject_key IS NULL");
        }
        LibraryWatchFilter::Duplicates => {
            sql.push_str(" AND ");
            sql.push_str(&exact_duplicate_exists_sql("f.id"));
        }
        LibraryWatchFilter::All => {}
    }

    (sql, params)
}

fn build_order_by(sort_by: Option<LibrarySortField>) -> String {
    match sort_by.unwrap_or_default() {
        LibrarySortField::Name => String::from("ORDER BY f.filename COLLATE NOCASE"),
        LibrarySortField::Creator => String::from(
            "ORDER BY c.canonical_name COLLATE NOCASE ASC, f.filename COLLATE NOCASE ASC",
        ),
        LibrarySortField::RecentlyModified => String::from(
            "ORDER BY \
                 CASE WHEN f.modified_at IS NULL THEN 1 ELSE 0 END ASC,\
                 f.modified_at DESC,\
                 f.filename COLLATE NOCASE ASC",
        ),
        LibrarySortField::HasUpdatesFirst => {
            // Sort by update priority: update leads first, then failed or unclear
            // checks, then quieter reminder/current/not-watched states.
            String::from(
                "ORDER BY \
                 CASE cwr.status\
                 WHEN 'exact_update_available' THEN 1\
                 WHEN 'possible_update' THEN 2\
                 WHEN 'check_failed' THEN 3\
                 WHEN 'unknown' THEN 4\
                 WHEN 'reminder_only' THEN 5\
                 WHEN 'current' THEN 6\
                 ELSE 7\
                 END ASC,\
                 f.filename COLLATE NOCASE ASC",
            )
        }
    }
}

fn scalar(connection: &Connection, sql: &str) -> AppResult<i64> {
    connection
        .query_row(sql, [], |row| row.get(0))
        .map_err(Into::into)
}

fn exact_duplicate_pair_count_sql() -> String {
    format!(
        "SELECT COUNT(*)
         FROM duplicates d
         JOIN files da ON d.file_id_a = da.id
         JOIN files db ON d.file_id_b = db.id
         WHERE {}",
        exact_duplicate_pair_sql("da", "db", "d")
    )
}

fn exact_duplicate_file_count_sql() -> String {
    format!(
        "SELECT COUNT(DISTINCT file_id)
         FROM (
           SELECT d.file_id_a AS file_id
           FROM duplicates d
           JOIN files da ON d.file_id_a = da.id
           JOIN files db ON d.file_id_b = db.id
           WHERE {proof}
           UNION
           SELECT d.file_id_b AS file_id
           FROM duplicates d
           JOIN files da ON d.file_id_a = da.id
           JOIN files db ON d.file_id_b = db.id
           WHERE {proof}
         )",
        proof = exact_duplicate_pair_sql("da", "db", "d")
    )
}

fn exact_duplicate_exists_sql(file_expr: &str) -> String {
    format!(
        "EXISTS (
           SELECT 1
           FROM duplicates d
           JOIN files da ON d.file_id_a = da.id
           JOIN files db ON d.file_id_b = db.id
           WHERE {}
         )",
        exact_duplicate_for_file_sql(file_expr)
    )
}

fn exact_duplicate_for_file_sql(file_expr: &str) -> String {
    format!(
        "{} AND (d.file_id_a = {file_expr} OR d.file_id_b = {file_expr})",
        exact_duplicate_pair_sql("da", "db", "d"),
        file_expr = file_expr
    )
}

fn exact_duplicate_pair_sql(left_alias: &str, right_alias: &str, pair_alias: &str) -> String {
    duplicate_detector::exact_duplicate_proof_sql(left_alias, right_alias, pair_alias)
}

fn string_list(connection: &Connection, sql: &str) -> AppResult<Vec<String>> {
    let mut statement = connection.prepare(sql)?;
    let items = statement
        .query_map([], |row| row.get(0))?
        .collect::<Result<Vec<String>, _>>()
        .map_err(crate::error::AppError::from)?;
    Ok(items)
}

fn parse_string_array(value: String) -> Vec<String> {
    serde_json::from_str(&value).unwrap_or_default()
}

fn parse_insights(value: Option<String>) -> FileInsights {
    match value {
        Some(v) => serde_json::from_str(&v).unwrap_or_default(),
        None => FileInsights::default(),
    }
}

fn has_preview_payload(value: &Option<String>) -> bool {
    value
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .is_some()
}

fn normalize_preview_payload(value: Option<String>) -> Option<String> {
    value.filter(|payload| !payload.trim().is_empty())
}

fn merge_preview_results_into_insights(
    insights: &mut FileInsights,
    embedded_thumb: Option<String>,
    cached_thumb: Option<String>,
) -> bool {
    let mut changed = false;

    if !has_preview_payload(&insights.thumbnail_preview) {
        if let Some(value) = normalize_preview_payload(embedded_thumb) {
            insights.thumbnail_preview = Some(value);
            changed = true;
        }
    }

    if !has_preview_payload(&insights.cached_thumbnail_preview) {
        if let Some(value) = normalize_preview_payload(cached_thumb) {
            insights.cached_thumbnail_preview = Some(value);
            changed = true;
        }
    }

    changed
}

fn persist_file_insights(
    connection: &Connection,
    file_id: i64,
    insights: &FileInsights,
) -> AppResult<()> {
    let insights_json = serde_json::to_string(insights)?;
    connection.execute(
        "UPDATE files SET insights = ?1 WHERE id = ?2",
        params![insights_json, file_id],
    )?;
    Ok(())
}

fn compact_library_row_insights(
    mut insights: FileInsights,
    include_previews: bool,
    compact: bool,
) -> FileInsights {
    if !include_previews {
        insights.thumbnail_preview = None;
        insights.cached_thumbnail_preview = None;
    }

    if compact {
        insights.resource_summary.truncate(1);
        insights.script_namespaces.truncate(3);
        insights.embedded_names.truncate(4);
        insights.creator_hints.truncate(2);
        insights.version_hints.truncate(1);
        insights.version_signals.truncate(1);
        insights.family_hints.truncate(4);
    }

    insights
}

fn list_creator_aliases(connection: &Connection, canonical_name: &str) -> AppResult<Vec<String>> {
    let mut statement = connection.prepare(
        "SELECT u.alias_name
         FROM user_creator_aliases u
         JOIN creators c ON c.id = u.creator_id
         WHERE c.canonical_name = ?1 COLLATE NOCASE
         ORDER BY u.updated_at DESC, u.alias_name COLLATE NOCASE",
    )?;

    let aliases = statement
        .query_map(params![canonical_name], |row| row.get(0))?
        .collect::<Result<Vec<String>, _>>()
        .map_err(crate::error::AppError::from)?;

    Ok(aliases)
}

#[cfg(test)]
mod tests {
    use rusqlite::params;

    use crate::{
        database,
        models::{
            LibraryFolderFilesQuery, LibraryQuery, LibrarySettings, LibrarySortField,
            LibraryWatchFilter,
        },
        seed::load_seed_pack,
    };
    use std::time::Instant;

    use super::{
        get_file_detail, get_folder_tree_metadata, get_library_facets,
        get_library_preview_diagnostics, list_library_files, list_library_folder_files,
        load_folder_tree_file_counts, merge_preview_results_into_insights,
        persist_file_insights, select_folder_tree_source_rows, FolderMetadataRow,
        FolderTreeSourceMode, MAX_FOLDER_QUERY_LIMIT,
    };

    fn setup_library_env() -> (rusqlite::Connection, LibrarySettings, crate::seed::SeedPack) {
        let mut connection = rusqlite::Connection::open_in_memory().expect("in-memory db");
        database::initialize(&mut connection).expect("schema");
        let seed_pack = load_seed_pack().expect("seed");
        database::seed_database(&mut connection, &seed_pack).expect("seed db");

        let settings = LibrarySettings {
            mods_path: Some("C:/Mods".to_owned()),
            tray_path: Some("C:/Tray".to_owned()),
            downloads_path: Some("C:/Downloads".to_owned()),
            ..Default::default()
        };
        database::save_library_paths(&mut connection, &settings).expect("save library paths");

        connection
            .execute(
                "INSERT INTO creators (canonical_name, notes) VALUES (?1, ?2)",
                params!["TestCreator", "fixture"],
            )
            .expect("creator");
        let creator_id = connection.last_insert_rowid();
        let insights_json =
            serde_json::to_string(&crate::models::FileInsights::default()).expect("insights json");

        connection
            .execute(
                "INSERT INTO files (
                    path,
                    filename,
                    extension,
                    hash,
                    creator_id,
                    kind,
                    subtype,
                    confidence,
                    source_location,
                    parser_warnings,
                    insights
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                params![
                    "C:/Mods/TestCreator/installed.package",
                    "installed.package",
                    ".package",
                    "installed-hash",
                    creator_id,
                    "Gameplay",
                    "Utility",
                    0.91_f64,
                    "mods",
                    "[]",
                    insights_json,
                ],
            )
            .expect("installed file");
        connection
            .execute(
                "INSERT INTO files (
                    path,
                    filename,
                    extension,
                    hash,
                    creator_id,
                    kind,
                    subtype,
                    confidence,
                    source_location,
                    parser_warnings,
                    insights
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                params![
                    "C:/Downloads/incoming.package",
                    "incoming.package",
                    ".package",
                    "incoming-hash",
                    creator_id,
                    "Gameplay",
                    "Utility",
                    0.88_f64,
                    "downloads",
                    "[]",
                    serde_json::to_string(&crate::models::FileInsights::default())
                        .expect("insights json"),
                ],
            )
            .expect("download file");

        (connection, settings, seed_pack)
    }

    fn default_insights_json() -> String {
        serde_json::to_string(&crate::models::FileInsights::default()).expect("insights json")
    }

    fn insert_library_file_row(
        connection: &rusqlite::Connection,
        path: &str,
        filename: &str,
        source_location: &str,
        relative_depth: i64,
        kind: &str,
        confidence: f64,
        insights_json: Option<String>,
    ) -> i64 {
        connection
            .execute(
                "INSERT INTO files (
                    path,
                    filename,
                    extension,
                    kind,
                    confidence,
                    source_location,
                    relative_depth,
                    parser_warnings,
                    insights
                 ) VALUES (?1, ?2, '.package', ?3, ?4, ?5, ?6, '[]', ?7)",
                params![
                    path,
                    filename,
                    kind,
                    confidence,
                    source_location,
                    relative_depth,
                    insights_json.unwrap_or_else(default_insights_json),
                ],
            )
            .expect("insert library file row");
        connection.last_insert_rowid()
    }

    fn insert_library_folder_row(
        connection: &rusqlite::Connection,
        source_location: &str,
        relative_path: &str,
        normalized_relative_path: &str,
        parent_normalized_relative_path: Option<&str>,
        name: &str,
        depth: i64,
        full_path: &str,
    ) -> i64 {
        connection
            .execute(
                "INSERT INTO library_folders (
                    source_location,
                    relative_path,
                    normalized_relative_path,
                    parent_normalized_relative_path,
                    name,
                    depth,
                    full_path
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    source_location,
                    relative_path,
                    normalized_relative_path,
                    parent_normalized_relative_path,
                    name,
                    depth,
                    full_path,
                ],
            )
            .expect("insert library folder row");
        connection.last_insert_rowid()
    }

    fn active_profile_root_identity(
        connection: &rusqlite::Connection,
        source_location: &str,
    ) -> (String, String) {
        let (root_id, root_role, configured_path) = match source_location {
            "mods" => ("mods", "installed_mods", "C:/Mods"),
            "tray" => ("tray", "tray_library", "C:/Tray"),
            other => panic!("unsupported source fixture: {other}"),
        };
        let profile_id = "library-folder-identity-profile";
        let timestamp = "2026-08-05T00:00:00Z";

        if database::get_active_game_installation_profile(connection)
            .expect("active profile lookup")
            .is_none()
        {
            connection
                .execute(
                    "INSERT INTO game_installation_profiles (
                        profile_id, profile_name, game_id, operating_environment,
                        status, detection_method, detection_evidence_json,
                        confirmation_state, confirmed_at, last_validated_at,
                        created_at, updated_at
                     ) VALUES (?1, ?2, 'sims4', 'unknown', 'valid', 'manual', '{}',
                               'confirmed', ?3, ?3, ?3, ?3)",
                    params![profile_id, "Library folder identity fixture", timestamp],
                )
                .expect("insert active profile fixture");
            connection
                .execute(
                    "INSERT INTO game_installation_roots (
                        profile_id, root_id, root_role, configured_path,
                        required, validation_state, filesystem_capabilities_json,
                        last_validated_at, created_at, updated_at
                     ) VALUES (?1, ?2, ?3, ?4, 1, 'valid', '{}', ?5, ?5, ?5)",
                    params![profile_id, root_id, root_role, configured_path, timestamp],
                )
                .expect("insert active profile root fixture");
            connection
                .execute(
                    "INSERT INTO app_settings (key, value, source, updated_at)
                     VALUES ('active_game_installation_profile_id', ?1, 'user', ?2)
                     ON CONFLICT(key) DO UPDATE SET
                        value = excluded.value,
                        source = excluded.source,
                        updated_at = excluded.updated_at",
                    params![profile_id, timestamp],
                )
                .expect("activate profile fixture");
        }

        let profile = database::get_active_game_installation_profile(connection)
            .expect("active profile lookup")
            .expect("active profile fixture");
        let root_id = profile
            .roots
            .iter()
            .find(|root| root.root_role == root_role)
            .expect("active profile root")
            .root_id
            .clone();
        (profile.profile_id, root_id)
    }

    fn assign_library_folder_identity(
        connection: &rusqlite::Connection,
        folder_id: i64,
        profile_id: &str,
        root_id: &str,
        relative_path: &str,
        relative_path_key: Option<&str>,
    ) {
        connection
            .execute(
                "UPDATE library_folders
                 SET installation_profile_id = ?1,
                     installation_root_id = ?2,
                     profile_relative_path = ?3,
                     profile_relative_path_key = ?4
                 WHERE id = ?5",
                params![
                    profile_id,
                    root_id,
                    relative_path,
                    relative_path_key,
                    folder_id,
                ],
            )
            .expect("assign folder identity");
    }

    fn insert_identity_library_file_row(
        connection: &rusqlite::Connection,
        path: &str,
        filename: &str,
        source_location: &str,
        relative_depth: i64,
        profile_id: &str,
        root_id: &str,
        relative_path: &str,
        relative_path_key: Option<&str>,
    ) -> i64 {
        connection
            .execute(
                "INSERT INTO files (
                    path,
                    filename,
                    extension,
                    kind,
                    confidence,
                    source_location,
                    relative_depth,
                    parser_warnings,
                    insights,
                    installation_profile_id,
                    installation_root_id,
                    profile_relative_path,
                    profile_relative_path_key
                 ) VALUES (?1, ?2, '.package', 'Gameplay', 0.8, ?3, ?4, '[]', ?5, ?6, ?7, ?8, ?9)",
                params![
                    path,
                    filename,
                    source_location,
                    relative_depth,
                    default_insights_json(),
                    profile_id,
                    root_id,
                    relative_path,
                    relative_path_key,
                ],
            )
            .expect("insert identity-owned library file row");
        connection.last_insert_rowid()
    }

    fn assign_file_parent_identity(
        connection: &rusqlite::Connection,
        file_id: i64,
        parent_relative_path: &str,
        parent_relative_path_key: Option<&str>,
    ) {
        connection
            .execute(
                "UPDATE files
                 SET profile_parent_relative_path = ?1,
                     profile_parent_relative_path_key = ?2
                 WHERE id = ?3",
                params![parent_relative_path, parent_relative_path_key, file_id],
            )
            .expect("assign file parent identity");
    }

    fn sensitive_path_key(components: &[&str]) -> String {
        let mut key = String::from("v1");
        for component in components {
            key.push_str(&format!("|n{}:{component}", component.chars().count()));
        }
        key
    }

    #[test]
    fn preview_diagnostics_count_available_deferred_and_unsupported_rows() {
        let (connection, _settings, _seed_pack) = setup_library_env();

        let embedded_json = serde_json::to_string(&crate::models::FileInsights {
            thumbnail_preview: Some("embedded-preview".to_owned()),
            ..Default::default()
        })
        .expect("embedded insights");
        let cached_json = serde_json::to_string(&crate::models::FileInsights {
            cached_thumbnail_preview: Some("cached-preview".to_owned()),
            ..Default::default()
        })
        .expect("cached insights");
        let empty_json = default_insights_json();

        connection
            .execute(
                "INSERT INTO files (
                    path,
                    filename,
                    extension,
                    kind,
                    confidence,
                    source_location,
                    relative_depth,
                    parser_warnings,
                    insights
                 ) VALUES
                   ('C:/Mods/embedded.package', 'embedded.package', '.package', 'CAS', 0.9, 'mods', 0, '[]', ?1),
                   ('C:/Mods/cached.package', 'cached.package', '.package', 'CAS', 0.9, 'mods', 0, '[]', ?2),
                   ('C:/Mods/script.ts4script', 'script.ts4script', '.ts4script', 'ScriptMods', 0.9, 'mods', 0, '[]', ?3),
                   ('C:/Tray/household.trayitem', 'household.trayitem', '.trayitem', 'TrayItem', 0.9, 'tray', 0, '[]', ?3)",
                params![embedded_json, cached_json, empty_json],
            )
            .expect("insert preview diagnostic rows");

        let diagnostics =
            get_library_preview_diagnostics(&connection).expect("preview diagnostics");

        assert_eq!(diagnostics.total_rows, 5);
        assert_eq!(diagnostics.rows_with_preview, 2);
        assert_eq!(diagnostics.rows_without_preview, 3);
        assert_eq!(diagnostics.embedded_preview_rows, 1);
        assert_eq!(diagnostics.cached_preview_rows, 1);
        assert_eq!(diagnostics.package_rows, 3);
        assert_eq!(diagnostics.package_rows_with_preview, 2);
        assert_eq!(diagnostics.package_rows_deferred_or_missing, 1);
        assert_eq!(diagnostics.script_rows, 1);
        assert_eq!(diagnostics.tray_rows, 1);
        assert_eq!(diagnostics.unsupported_rows, 2);
        assert_eq!(diagnostics.unsupported_without_preview, 2);
        assert_eq!(diagnostics.failed_extraction_rows, 0);
        assert_eq!(diagnostics.stale_preview_rows, 0);
        assert!(!diagnostics.failure_state_tracked);
        assert!(!diagnostics.stale_state_tracked);
        assert!(diagnostics.deferred_extraction_enabled);
    }

    #[test]
    fn selected_detail_preview_merge_preserves_existing_preview_payloads() {
        let mut insights = crate::models::FileInsights::default();

        assert!(merge_preview_results_into_insights(
            &mut insights,
            Some("embedded-preview".to_owned()),
            Some("cached-preview".to_owned()),
        ));
        assert_eq!(
            insights.thumbnail_preview.as_deref(),
            Some("embedded-preview")
        );
        assert_eq!(
            insights.cached_thumbnail_preview.as_deref(),
            Some("cached-preview")
        );

        assert!(!merge_preview_results_into_insights(
            &mut insights,
            Some("new-embedded".to_owned()),
            Some("new-cached".to_owned()),
        ));
        assert_eq!(
            insights.thumbnail_preview.as_deref(),
            Some("embedded-preview")
        );
        assert_eq!(
            insights.cached_thumbnail_preview.as_deref(),
            Some("cached-preview")
        );
    }

    #[test]
    fn selected_detail_preview_persistence_updates_indexed_insights() {
        let (connection, _settings, _seed_pack) = setup_library_env();
        let insights = crate::models::FileInsights {
            thumbnail_preview: Some("detail-preview".to_owned()),
            ..Default::default()
        };

        persist_file_insights(&connection, 1, &insights).expect("persist preview insights");

        let stored: String = connection
            .query_row("SELECT insights FROM files WHERE id = 1", [], |row| {
                row.get(0)
            })
            .expect("stored insights");
        let parsed: crate::models::FileInsights =
            serde_json::from_str(&stored).expect("parse stored insights");
        assert_eq!(parsed.thumbnail_preview.as_deref(), Some("detail-preview"));
    }

    #[test]
    fn library_queries_focus_on_installed_content() {
        let (connection, settings, seed_pack) = setup_library_env();

        let listing =
            list_library_files(&connection, LibraryQuery::default()).expect("library listing");
        assert_eq!(listing.total, 1);
        assert_eq!(listing.items.len(), 1);
        assert_eq!(listing.items[0].filename, "installed.package");
        assert_eq!(listing.items[0].source_location, "mods");

        let facets = get_library_facets(&connection, &seed_pack.taxonomy, None).expect("facets");
        assert_eq!(facets.sources, vec!["mods".to_owned()]);
        assert_eq!(facets.creators, vec!["TestCreator".to_owned()]);

        let download_detail =
            get_file_detail(&connection, &settings, &seed_pack, 2).expect("download detail lookup");
        assert!(download_detail.is_none());
    }

    #[test]
    fn library_listing_supports_paged_queries() {
        let (connection, _settings, _seed_pack) = setup_library_env();

        let listing = list_library_files(
            &connection,
            LibraryQuery {
                limit: Some(1),
                offset: Some(0),
                ..Default::default()
            },
        )
        .expect("paged library listing");

        assert_eq!(listing.total, 1);
        assert_eq!(listing.items.len(), 1);
        assert_eq!(listing.items[0].filename, "installed.package");
    }

    #[test]
    fn file_detail_reports_exact_duplicate_pair_count_only() {
        let (connection, settings, seed_pack) = setup_library_env();

        for filename in ["installed-copy-a.package", "installed-copy-b.package"] {
            connection
                .execute(
                    "INSERT INTO files (
                        path,
                        filename,
                        extension,
                        hash,
                        kind,
                        confidence,
                        source_location,
                        parser_warnings,
                        insights
                     ) VALUES (?1, ?2, '.package', 'installed-hash', 'Gameplay', 0.8, 'mods', '[]', ?3)",
                    params![
                        format!("C:/Mods/TestCreator/{filename}"),
                        filename,
                        serde_json::to_string(&crate::models::FileInsights::default())
                            .expect("insights json"),
                    ],
                )
                .expect("insert duplicate peer");
        }

        connection
            .execute(
                "INSERT INTO duplicates (file_id_a, file_id_b, duplicate_type, detection_method)
                 VALUES
                   (1, 3, 'exact', 'sha256'),
                   (1, 4, 'exact', 'sha256'),
                   (1, 2, 'filename', 'filename_match')",
                [],
            )
            .expect("insert duplicate pairs");

        let detail = get_file_detail(&connection, &settings, &seed_pack, 1)
            .expect("detail")
            .expect("installed detail");

        assert_eq!(detail.duplicates_count, 2);
        assert_eq!(detail.duplicate_types, vec!["exact".to_owned()]);
    }

    #[test]
    fn duplicate_library_filter_only_returns_exact_content_duplicates() {
        let (connection, _settings, _seed_pack) = setup_library_env();

        connection
            .execute(
                "INSERT INTO duplicates (file_id_a, file_id_b, duplicate_type, detection_method)
                 VALUES
                   (1, 2, 'filename', 'filename_match'),
                   (1, 2, 'exact', 'sha256'),
                   (1, 1, 'exact', 'sha256')",
                [],
            )
            .expect("insert weak and malformed comparisons");

        let name_match_listing = list_library_files(
            &connection,
            LibraryQuery {
                watch_filter: Some(LibraryWatchFilter::Duplicates),
                ..Default::default()
            },
        )
        .expect("name match listing");
        assert_eq!(name_match_listing.total, 0);

        connection
            .execute("DELETE FROM duplicates", [])
            .expect("clear malformed rows");
        connection
            .execute("UPDATE files SET hash = 'installed-hash' WHERE id = 2", [])
            .expect("align exact duplicate hash");
        connection
            .execute(
                "INSERT INTO duplicates (file_id_a, file_id_b, duplicate_type, detection_method)
                 VALUES (1, 2, 'exact', 'sha256')",
                [],
            )
            .expect("insert exact duplicate");

        let exact_listing = list_library_files(
            &connection,
            LibraryQuery {
                watch_filter: Some(LibraryWatchFilter::Duplicates),
                ..Default::default()
            },
        )
        .expect("exact listing");
        assert_eq!(exact_listing.total, 1);
        assert!(exact_listing.items[0].has_duplicate);
    }

    #[test]
    fn duplicate_library_counts_include_exact_package_fingerprints() {
        let (connection, settings, seed_pack) = setup_library_env();

        connection
            .execute(
                "UPDATE files
                 SET content_fingerprint_kind = 'package',
                     content_fingerprint = 'same-package-fingerprint',
                     content_fingerprint_version = 'v1',
                     content_fingerprint_status = 'available'
                 WHERE id IN (1, 2)",
                [],
            )
            .expect("set package fingerprints");
        connection
            .execute(
                "INSERT INTO duplicates (file_id_a, file_id_b, duplicate_type, detection_method)
                 VALUES (1, 2, 'exact', 'package_fingerprint_v1')",
                [],
            )
            .expect("insert package fingerprint duplicate");

        let detail = get_file_detail(&connection, &settings, &seed_pack, 1)
            .expect("detail")
            .expect("installed detail");
        assert_eq!(detail.duplicates_count, 1);

        let listing = list_library_files(
            &connection,
            LibraryQuery {
                watch_filter: Some(LibraryWatchFilter::Duplicates),
                ..Default::default()
            },
        )
        .expect("duplicate listing");
        assert_eq!(listing.total, 1);
        assert!(listing.items[0].has_duplicate);
    }

    #[test]
    fn folder_file_listing_returns_paged_direct_folder_contents() {
        let (connection, _settings, _seed_pack) = setup_library_env();
        let insights_json =
            serde_json::to_string(&crate::models::FileInsights::default()).expect("insights json");

        connection
            .execute(
                "UPDATE files SET relative_depth = 1 WHERE filename = 'installed.package'",
                [],
            )
            .expect("align fixture depth");
        connection
            .execute(
                "INSERT INTO files (
                    path,
                    filename,
                    extension,
                    kind,
                    confidence,
                    source_location,
                    relative_depth,
                    parser_warnings,
                    insights
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    "C:/Mods/TestCreator/second.package",
                    "second.package",
                    ".package",
                    "Gameplay",
                    0.82_f64,
                    "mods",
                    1_i64,
                    "[]",
                    insights_json,
                ],
            )
            .expect("second direct file");
        connection
            .execute(
                "INSERT INTO files (
                    path,
                    filename,
                    extension,
                    kind,
                    confidence,
                    source_location,
                    relative_depth,
                    parser_warnings,
                    insights
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    "C:/Mods/TestCreator/Nested/deep.package",
                    "deep.package",
                    ".package",
                    "Gameplay",
                    0.8_f64,
                    "mods",
                    2_i64,
                    "[]",
                    serde_json::to_string(&crate::models::FileInsights::default())
                        .expect("insights json"),
                ],
            )
            .expect("nested file");

        let listing = list_library_folder_files(
            &connection,
            LibraryFolderFilesQuery {
                folder_path: "Mods/TestCreator".to_owned(),
                recursive: false,
                limit: Some(1),
                offset: Some(1),
                include_previews: Some(false),
                ..Default::default()
            },
        )
        .expect("folder listing");

        assert_eq!(listing.total, 2);
        assert_eq!(listing.items.len(), 1);
        assert_eq!(listing.items[0].filename, "second.package");
        assert!(listing
            .items
            .iter()
            .all(|item| item.filename != "deep.package"));
        assert!(listing
            .items
            .iter()
            .all(|item| item.insights.thumbnail_preview.is_none()));
    }

    #[test]
    fn folder_file_listing_respects_library_filters() {
        let (connection, _settings, _seed_pack) = setup_library_env();

        connection
            .execute(
                "UPDATE files SET relative_depth = 1 WHERE filename = 'installed.package'",
                [],
            )
            .expect("align fixture depth");

        let listing = list_library_folder_files(
            &connection,
            LibraryFolderFilesQuery {
                folder_path: "Mods/TestCreator".to_owned(),
                recursive: false,
                filters: LibraryQuery {
                    kind: Some("BuildBuy".to_owned()),
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .expect("folder listing");

        assert_eq!(listing.total, 0);
        assert!(listing.items.is_empty());
    }

    #[test]
    fn folder_file_listing_uses_current_profile_identity_instead_of_absolute_path() {
        let (connection, _settings, _seed_pack) = setup_library_env();
        let (profile_id, root_id) = active_profile_root_identity(&connection, "mods");
        let folder_id = insert_library_folder_row(
            &connection,
            "mods",
            "PortableFolder",
            "portablefolder",
            Some(""),
            "PortableFolder",
            1,
            "C:/Mods/PortableFolder",
        );
        let folder_key = sensitive_path_key(&["PortableFolder"]);
        assign_library_folder_identity(
            &connection,
            folder_id,
            &profile_id,
            &root_id,
            "PortableFolder",
            Some(&folder_key),
        );

        let correct_key = sensitive_path_key(&["PortableFolder", "correct.package"]);
        insert_identity_library_file_row(
            &connection,
            "Z:/Relocated/PortableFolder/correct.package",
            "correct.package",
            "mods",
            1,
            &profile_id,
            &root_id,
            "PortableFolder/correct.package",
            Some(&correct_key),
        );
        let wrong_key = sensitive_path_key(&["PortableFolder", "wrong.package"]);
        insert_identity_library_file_row(
            &connection,
            "C:/Mods/PortableFolder/wrong.package",
            "wrong.package",
            "mods",
            1,
            "foreign-profile",
            &root_id,
            "PortableFolder/wrong.package",
            Some(&wrong_key),
        );

        let listing = list_library_folder_files(
            &connection,
            LibraryFolderFilesQuery {
                folder_path: "Mods/PortableFolder".to_owned(),
                recursive: false,
                ..Default::default()
            },
        )
        .expect("identity folder listing");

        assert_eq!(listing.total, 1);
        assert_eq!(listing.items[0].filename, "correct.package");
    }

    #[test]
    fn folder_file_listing_preserves_case_distinct_identity_keys() {
        let (connection, _settings, _seed_pack) = setup_library_env();
        let (profile_id, root_id) = active_profile_root_identity(&connection, "mods");
        let folder_id = insert_library_folder_row(
            &connection,
            "mods",
            "Creator",
            "creator",
            Some(""),
            "Creator",
            1,
            "C:/Mods/Creator",
        );
        let folder_key = sensitive_path_key(&["Creator"]);
        assign_library_folder_identity(
            &connection,
            folder_id,
            &profile_id,
            &root_id,
            "Creator",
            Some(&folder_key),
        );

        let matching_key = sensitive_path_key(&["Creator", "inside.package"]);
        insert_identity_library_file_row(
            &connection,
            "C:/Mods/Creator/inside.package",
            "inside.package",
            "mods",
            1,
            &profile_id,
            &root_id,
            "Creator/inside.package",
            Some(&matching_key),
        );
        let alias_key = sensitive_path_key(&["creator", "alias.package"]);
        insert_identity_library_file_row(
            &connection,
            "C:/Mods/creator/alias.package",
            "alias.package",
            "mods",
            1,
            &profile_id,
            &root_id,
            "creator/alias.package",
            Some(&alias_key),
        );

        let listing = list_library_folder_files(
            &connection,
            LibraryFolderFilesQuery {
                folder_path: "Mods/Creator".to_owned(),
                recursive: false,
                ..Default::default()
            },
        )
        .expect("case-sensitive folder listing");

        assert_eq!(listing.total, 1);
        assert_eq!(listing.items[0].filename, "inside.package");
    }

    #[test]
    fn folder_file_listing_uses_exact_observed_path_when_key_is_unknown() {
        let (connection, _settings, _seed_pack) = setup_library_env();
        let (profile_id, root_id) = active_profile_root_identity(&connection, "mods");
        let folder_id = insert_library_folder_row(
            &connection,
            "mods",
            "CaseFolder",
            "casefolder",
            Some(""),
            "CaseFolder",
            1,
            "C:/Mods/CaseFolder",
        );
        assign_library_folder_identity(
            &connection,
            folder_id,
            &profile_id,
            &root_id,
            "CaseFolder",
            None,
        );

        insert_identity_library_file_row(
            &connection,
            "C:/Mods/CaseFolder/exact.package",
            "exact.package",
            "mods",
            1,
            &profile_id,
            &root_id,
            "CaseFolder/exact.package",
            None,
        );
        insert_identity_library_file_row(
            &connection,
            "C:/Mods/casefolder/alias.package",
            "alias.package",
            "mods",
            1,
            &profile_id,
            &root_id,
            "casefolder/alias.package",
            None,
        );

        let listing = list_library_folder_files(
            &connection,
            LibraryFolderFilesQuery {
                folder_path: "Mods/CaseFolder".to_owned(),
                recursive: false,
                ..Default::default()
            },
        )
        .expect("unknown-key folder listing");

        assert_eq!(listing.total, 1);
        assert_eq!(listing.items[0].filename, "exact.package");
    }

    #[test]
    fn folder_file_listing_blocks_identity_from_a_stale_profile() {
        let (connection, _settings, _seed_pack) = setup_library_env();
        let (_profile_id, root_id) = active_profile_root_identity(&connection, "mods");
        let folder_id = insert_library_folder_row(
            &connection,
            "mods",
            "StaleFolder",
            "stalefolder",
            Some(""),
            "StaleFolder",
            1,
            "C:/Mods/StaleFolder",
        );
        let folder_key = sensitive_path_key(&["StaleFolder"]);
        assign_library_folder_identity(
            &connection,
            folder_id,
            "foreign-profile",
            &root_id,
            "StaleFolder",
            Some(&folder_key),
        );
        insert_library_file_row(
            &connection,
            "C:/Mods/StaleFolder/legacy.package",
            "legacy.package",
            "mods",
            1,
            "Gameplay",
            0.8,
            None,
        );

        let listing = list_library_folder_files(
            &connection,
            LibraryFolderFilesQuery {
                folder_path: "Mods/StaleFolder".to_owned(),
                recursive: false,
                ..Default::default()
            },
        )
        .expect("stale-profile folder listing");

        assert_eq!(listing.total, 0);
        assert!(listing.items.is_empty());
    }

    #[test]
    fn folder_file_listing_uses_root_scoped_sql_and_avoids_path_tail_false_positive() {
        let (connection, _settings, _seed_pack) = setup_library_env();
        insert_library_folder_row(
            &connection,
            "mods",
            "TestCreator",
            "testcreator",
            Some(""),
            "TestCreator",
            1,
            "C:/Mods/TestCreator",
        );

        insert_library_file_row(
            &connection,
            "C:/Mods/TestCreator/real.package",
            "real.package",
            "mods",
            1,
            "Gameplay",
            0.88,
            None,
        );
        insert_library_file_row(
            &connection,
            "C:/Archive/TestCreator/archive-copy.package",
            "archive-copy.package",
            "mods",
            1,
            "Gameplay",
            0.82,
            None,
        );

        let listing = list_library_folder_files(
            &connection,
            LibraryFolderFilesQuery {
                folder_path: "Mods/TestCreator".to_owned(),
                recursive: false,
                limit: Some(10),
                include_previews: Some(false),
                ..Default::default()
            },
        )
        .expect("folder listing");

        assert!(listing
            .items
            .iter()
            .any(|item| item.filename == "real.package"));
        assert!(listing
            .items
            .iter()
            .all(|item| item.filename != "archive-copy.package"));
    }

    #[test]
    fn folder_file_listing_supports_recursive_search_sort_preview_and_paging_under_load() {
        let (connection, _settings, _seed_pack) = setup_library_env();
        let started_at = Instant::now();

        for index in 0..1_000 {
            insert_library_file_row(
                &connection,
                &format!("C:/Mods/HugeFolder/huge_{index:04}.package"),
                &format!("huge_{index:04}.package"),
                "mods",
                1,
                if index % 3 == 0 { "CAS" } else { "Gameplay" },
                0.5,
                None,
            );
        }
        for index in 0..75 {
            insert_library_file_row(
                &connection,
                &format!("C:/Mods/HugeFolder/Nested/nested_{index:04}.package"),
                &format!("nested_{index:04}.package"),
                "mods",
                2,
                "Gameplay",
                0.7,
                None,
            );
        }
        let preview_json = serde_json::to_string(&crate::models::FileInsights {
            thumbnail_preview: Some("preview-bytes".to_owned()),
            ..Default::default()
        })
        .expect("preview insights");
        insert_library_file_row(
            &connection,
            "C:/Mods/HugeFolder/special_preview.package",
            "special_preview.package",
            "mods",
            1,
            "Gameplay",
            0.9,
            Some(preview_json),
        );

        let direct_page = list_library_folder_files(
            &connection,
            LibraryFolderFilesQuery {
                folder_path: "Mods/HugeFolder".to_owned(),
                recursive: false,
                filters: LibraryQuery {
                    sort_by: Some(crate::models::LibrarySortField::Name),
                    ..Default::default()
                },
                limit: Some(25),
                offset: Some(50),
                include_previews: Some(false),
            },
        )
        .expect("direct folder page");

        assert_eq!(direct_page.total, 1_001);
        assert_eq!(direct_page.items.len(), 25);
        assert!(direct_page
            .items
            .iter()
            .all(|item| item.insights.thumbnail_preview.is_none()));

        let recursive = list_library_folder_files(
            &connection,
            LibraryFolderFilesQuery {
                folder_path: "Mods/HugeFolder".to_owned(),
                recursive: true,
                limit: Some(2_000),
                include_previews: Some(false),
                ..Default::default()
            },
        )
        .expect("recursive folder page");
        assert_eq!(recursive.total, 1_076);
        assert_eq!(recursive.items.len(), MAX_FOLDER_QUERY_LIMIT as usize);

        let search_with_preview = list_library_folder_files(
            &connection,
            LibraryFolderFilesQuery {
                folder_path: "Mods/HugeFolder".to_owned(),
                recursive: true,
                filters: LibraryQuery {
                    search: Some("special_preview".to_owned()),
                    ..Default::default()
                },
                limit: Some(10),
                include_previews: Some(true),
                ..Default::default()
            },
        )
        .expect("search with preview");
        assert_eq!(search_with_preview.total, 1);
        assert_eq!(
            search_with_preview.items[0]
                .insights
                .thumbnail_preview
                .as_deref(),
            Some("preview-bytes")
        );

        eprintln!(
            "library_folder_stress rows=1076 elapsed_ms={}",
            started_at.elapsed().as_millis()
        );
    }

    #[test]
    #[ignore = "large synthetic stress harness; run pnpm run test:library:stress"]
    fn large_library_backend_stress_queries_remain_bounded_and_truthful() {
        let (mut connection, settings, seed_pack) = setup_library_env();
        let started_at = Instant::now();
        let mut timings = Vec::<(&'static str, u128)>::new();

        connection
            .execute("DELETE FROM duplicates", [])
            .expect("clear duplicates");
        connection
            .execute("DELETE FROM files", [])
            .expect("clear files");
        connection
            .execute("DELETE FROM library_folders", [])
            .expect("clear folders");
        connection
            .execute("DELETE FROM bundles", [])
            .expect("clear bundles");

        connection
            .execute(
                "INSERT INTO creators (canonical_name, notes) VALUES ('StressCreator', 'stress fixture')",
                [],
            )
            .expect("insert stress creator");
        let creator_id = connection.last_insert_rowid();

        let mut bundle_ids = Vec::new();
        for index in 0..25 {
            connection
                .execute(
                    "INSERT INTO bundles (bundle_name, bundle_type, file_count, confidence)
                     VALUES (?1, 'package_set', 60, 0.91)",
                    params![format!("Stress Pack {index:02}")],
                )
                .expect("insert stress bundle");
            bundle_ids.push(connection.last_insert_rowid());
        }
        let relationship_bundle_id = bundle_ids[0];
        let empty_insights = default_insights_json();
        let preview_insights = serde_json::to_string(&crate::models::FileInsights {
            thumbnail_preview: Some("stress-preview".to_owned()),
            ..Default::default()
        })
        .expect("preview insights");

        {
            let transaction = connection.transaction().expect("stress transaction");
            {
                let mut folder_statement = transaction
                    .prepare(
                        "INSERT INTO library_folders (
                            source_location,
                            relative_path,
                            normalized_relative_path,
                            parent_normalized_relative_path,
                            name,
                            depth,
                            full_path
                         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    )
                    .expect("prepare folder insert");
                for (source, relative, normalized, parent, name, depth, full_path) in [
                    ("mods", "", "", None, "Mods", 0, "C:/Mods"),
                    (
                        "mods",
                        "HugeDirect",
                        "hugedirect",
                        Some(""),
                        "HugeDirect",
                        1,
                        "C:/Mods/HugeDirect",
                    ),
                    ("mods", "Deep", "deep", Some(""), "Deep", 1, "C:/Mods/Deep"),
                    (
                        "mods",
                        "Deep/Level 1",
                        "deep/level 1",
                        Some("deep"),
                        "Level 1",
                        2,
                        "C:/Mods/Deep/Level 1",
                    ),
                    (
                        "mods",
                        "Deep/Level 1/Level 2",
                        "deep/level 1/level 2",
                        Some("deep/level 1"),
                        "Level 2",
                        3,
                        "C:/Mods/Deep/Level 1/Level 2",
                    ),
                    (
                        "mods",
                        "Relationships",
                        "relationships",
                        Some(""),
                        "Relationships",
                        1,
                        "C:/Mods/Relationships",
                    ),
                    (
                        "mods",
                        "Mixed Missing",
                        "mixed missing",
                        Some(""),
                        "Mixed Missing",
                        1,
                        "C:/Mods/Mixed Missing",
                    ),
                    (
                        "mods",
                        "Empty Stress",
                        "empty stress",
                        Some(""),
                        "Empty Stress",
                        1,
                        "C:/Mods/Empty Stress",
                    ),
                    ("tray", "", "", None, "Tray", 0, "C:/Tray"),
                    (
                        "tray",
                        "Saved Lots",
                        "saved lots",
                        Some(""),
                        "Saved Lots",
                        1,
                        "C:/Tray/Saved Lots",
                    ),
                ] {
                    folder_statement
                        .execute(params![
                            source, relative, normalized, parent, name, depth, full_path
                        ])
                        .expect("insert stress folder");
                }
                for index in 0..150 {
                    folder_statement
                        .execute(params![
                            "mods",
                            format!("Empty Stress/Empty {index:03}"),
                            format!("empty stress/empty {index:03}"),
                            Some("empty stress"),
                            format!("Empty {index:03}"),
                            2_i64,
                            format!("C:/Mods/Empty Stress/Empty {index:03}"),
                        ])
                        .expect("insert empty stress folder");
                }
            }

            {
                let mut file_statement = transaction
                    .prepare(
                        "INSERT INTO files (
                            path,
                            filename,
                            extension,
                            hash,
                            size,
                            creator_id,
                            bundle_id,
                            kind,
                            subtype,
                            confidence,
                            source_location,
                            relative_depth,
                            safety_notes,
                            parser_warnings,
                            insights
                         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, '[]', '[]', ?13)",
                    )
                    .expect("prepare file insert");

                for index in 0..5_000 {
                    let filename = if index == 123 {
                        "stress_target_0123.package".to_owned()
                    } else {
                        format!("huge_direct_{index:05}.package")
                    };
                    let long_segment = if index % 997 == 0 {
                        "_with_a_very_long_creator_style_filename_segment_that_should_not_change_query_shape"
                    } else {
                        ""
                    };
                    file_statement
                        .execute(params![
                            format!("C:/Mods/HugeDirect/{filename}{long_segment}"),
                            filename,
                            ".package",
                            format!("stress-huge-hash-{index:05}"),
                            1_024_i64 + index as i64,
                            Some(creator_id),
                            Some(bundle_ids[index % bundle_ids.len()]),
                            if index % 4 == 0 { "CAS" } else { "Gameplay" },
                            if index % 9 == 0 {
                                Some("Hair")
                            } else {
                                Some("Utility")
                            },
                            0.86_f64,
                            "mods",
                            1_i64,
                            if index % 1_000 == 0 {
                                &preview_insights
                            } else {
                                &empty_insights
                            },
                        ])
                        .expect("insert huge direct file");
                }

                for index in 0..2_000 {
                    file_statement
                        .execute(params![
                            format!("C:/Mods/Deep/Level 1/Level 2/deep_nested_{index:05}.package"),
                            format!("deep_nested_{index:05}.package"),
                            ".package",
                            format!("stress-deep-hash-{index:05}"),
                            2_048_i64 + index as i64,
                            Some(creator_id),
                            Some(bundle_ids[index % bundle_ids.len()]),
                            "Gameplay",
                            Some("Nested"),
                            0.81_f64,
                            "mods",
                            3_i64,
                            &empty_insights,
                        ])
                        .expect("insert deep file");
                }

                for index in 0..1_500 {
                    file_statement
                        .execute(params![
                            format!("C:/Mods/Relationships/relationship_pack_{index:05}.package"),
                            format!("relationship_pack_{index:05}.package"),
                            ".package",
                            format!("stress-relationship-hash-{index:05}"),
                            3_072_i64 + index as i64,
                            Some(creator_id),
                            Some(relationship_bundle_id),
                            "Gameplay",
                            Some("PackMember"),
                            0.9_f64,
                            "mods",
                            1_i64,
                            &empty_insights,
                        ])
                        .expect("insert relationship file");
                }

                for index in 0..1_000 {
                    file_statement
                        .execute(params![
                            format!("C:/Tray/Saved Lots/tray_item_{index:05}.trayitem"),
                            format!("tray_item_{index:05}.trayitem"),
                            ".trayitem",
                            format!("stress-tray-hash-{index:05}"),
                            4_096_i64 + index as i64,
                            Option::<i64>::None,
                            Option::<i64>::None,
                            "TrayItem",
                            Option::<String>::None,
                            0.78_f64,
                            "tray",
                            1_i64,
                            &empty_insights,
                        ])
                        .expect("insert tray file");
                }

                for index in 0..500 {
                    file_statement
                        .execute(params![
                            format!(
                                "C:/Mods/Mixed Missing/Unknown Creator Folder With A Very Long Name {index:05}/unknown_{index:05}.ts4script"
                            ),
                            format!("unknown_{index:05}.ts4script"),
                            ".ts4script",
                            format!("stress-unknown-hash-{index:05}"),
                            512_i64 + index as i64,
                            Option::<i64>::None,
                            Option::<i64>::None,
                            "Unknown",
                            Option::<String>::None,
                            0.41_f64,
                            "mods",
                            1_i64,
                            &empty_insights,
                        ])
                        .expect("insert missing metadata file");
                }
            }

            transaction.commit().expect("commit stress fixture");
        }

        let op_started = Instant::now();
        let first_page = list_library_files(
            &connection,
            LibraryQuery {
                limit: Some(50),
                offset: Some(0),
                include_previews: Some(false),
                sort_by: Some(LibrarySortField::Name),
                ..Default::default()
            },
        )
        .expect("large library first page");
        timings.push(("list_first_page", op_started.elapsed().as_millis()));
        assert_eq!(first_page.total, 10_000);
        assert_eq!(first_page.items.len(), 50);
        assert!(first_page
            .items
            .iter()
            .all(|item| item.insights.thumbnail_preview.is_none()));

        let op_started = Instant::now();
        let search = list_library_files(
            &connection,
            LibraryQuery {
                search: Some("stress_target".to_owned()),
                limit: Some(10),
                include_previews: Some(false),
                ..Default::default()
            },
        )
        .expect("large library search");
        timings.push(("list_search", op_started.elapsed().as_millis()));
        assert_eq!(search.total, 1);
        assert_eq!(search.items[0].filename, "stress_target_0123.package");

        let op_started = Instant::now();
        let filtered = list_library_files(
            &connection,
            LibraryQuery {
                kind: Some("CAS".to_owned()),
                limit: Some(75),
                include_previews: Some(false),
                ..Default::default()
            },
        )
        .expect("large library filter");
        timings.push(("list_filter", op_started.elapsed().as_millis()));
        assert!(filtered.total > 1_000);
        assert_eq!(filtered.items.len(), 75);

        let op_started = Instant::now();
        let sorted = list_library_files(
            &connection,
            LibraryQuery {
                sort_by: Some(LibrarySortField::RecentlyModified),
                limit: Some(100),
                include_previews: Some(false),
                ..Default::default()
            },
        )
        .expect("large library sort");
        timings.push(("list_sort", op_started.elapsed().as_millis()));
        assert_eq!(sorted.total, 10_000);
        assert_eq!(sorted.items.len(), 100);

        let op_started = Instant::now();
        let metadata = get_folder_tree_metadata(&connection, &LibraryQuery::default())
            .expect("large folder tree");
        timings.push(("folder_tree_metadata", op_started.elapsed().as_millis()));
        assert!(metadata.total_folders >= 160);
        let mods = metadata
            .roots
            .iter()
            .find(|node| node.name == "Mods")
            .expect("mods root");
        let huge_direct = mods
            .children
            .iter()
            .find(|node| node.name == "HugeDirect")
            .expect("huge direct folder");
        assert_eq!(huge_direct.direct_file_count, 5_000);
        assert_eq!(huge_direct.total_file_count, 5_000);
        let empty_stress = mods
            .children
            .iter()
            .find(|node| node.name == "Empty Stress")
            .expect("empty stress folder");
        assert_eq!(empty_stress.total_file_count, 0);
        assert_eq!(empty_stress.child_folder_count, 150);

        let op_started = Instant::now();
        let direct_folder = list_library_folder_files(
            &connection,
            LibraryFolderFilesQuery {
                folder_path: "Mods/HugeDirect".to_owned(),
                recursive: false,
                limit: Some(75),
                offset: Some(100),
                include_previews: Some(false),
                ..Default::default()
            },
        )
        .expect("large direct folder");
        timings.push(("folder_direct_page", op_started.elapsed().as_millis()));
        assert_eq!(direct_folder.total, 5_000);
        assert_eq!(direct_folder.items.len(), 75);
        assert!(direct_folder
            .items
            .iter()
            .all(|item| item.insights.thumbnail_preview.is_none()));

        let op_started = Instant::now();
        let recursive_folder = list_library_folder_files(
            &connection,
            LibraryFolderFilesQuery {
                folder_path: "Mods/Deep".to_owned(),
                recursive: true,
                limit: Some(80),
                include_previews: Some(false),
                ..Default::default()
            },
        )
        .expect("large recursive folder");
        timings.push(("folder_recursive_page", op_started.elapsed().as_millis()));
        assert_eq!(recursive_folder.total, 2_000);
        assert_eq!(recursive_folder.items.len(), 80);

        let op_started = Instant::now();
        let empty_folder = list_library_folder_files(
            &connection,
            LibraryFolderFilesQuery {
                folder_path: "Mods/Empty Stress/Empty 042".to_owned(),
                recursive: false,
                limit: Some(25),
                include_previews: Some(false),
                ..Default::default()
            },
        )
        .expect("empty folder listing");
        timings.push(("empty_folder_listing", op_started.elapsed().as_millis()));
        assert_eq!(empty_folder.total, 0);
        assert!(empty_folder.items.is_empty());

        let op_started = Instant::now();
        let relationship_page = list_library_files(
            &connection,
            LibraryQuery {
                search: Some("relationship_pack".to_owned()),
                limit: Some(20),
                include_previews: Some(false),
                ..Default::default()
            },
        )
        .expect("relationship-heavy page");
        timings.push(("relationship_peer_counts", op_started.elapsed().as_millis()));
        assert_eq!(relationship_page.total, 1_500);
        assert_eq!(relationship_page.items.len(), 20);
        assert!(relationship_page
            .items
            .iter()
            .all(|item| item.same_folder_peer_count == 1_499));
        assert!(relationship_page
            .items
            .iter()
            .all(|item| item.same_pack_peer_count == 1_499));

        let detail_id: i64 = connection
            .query_row(
                "SELECT id FROM files WHERE filename = 'stress_target_0123.package'",
                [],
                |row| row.get(0),
            )
            .expect("stress detail id");
        let op_started = Instant::now();
        let detail = get_file_detail(&connection, &settings, &seed_pack, detail_id)
            .expect("stress detail")
            .expect("detail exists");
        timings.push(("file_detail", op_started.elapsed().as_millis()));
        assert_eq!(detail.filename, "stress_target_0123.package");

        let op_started = Instant::now();
        let duplicate_overview =
            crate::core::duplicate_detector::get_duplicate_overview(&connection)
                .expect("duplicate overview");
        timings.push(("duplicate_overview", op_started.elapsed().as_millis()));
        assert_eq!(duplicate_overview.exact_pairs, 0);

        let op_started = Instant::now();
        let preview_diagnostics =
            get_library_preview_diagnostics(&connection).expect("preview diagnostics");
        timings.push(("preview_diagnostics", op_started.elapsed().as_millis()));
        assert_eq!(preview_diagnostics.total_rows, 10_000);
        assert_eq!(preview_diagnostics.rows_with_preview, 5);
        assert_eq!(preview_diagnostics.package_rows, 8_500);
        assert_eq!(preview_diagnostics.script_rows, 500);
        assert_eq!(preview_diagnostics.tray_rows, 1_000);

        eprintln!(
            "library_large_backend_stress rows=10000 total_elapsed_ms={} timings={:?}",
            started_at.elapsed().as_millis(),
            timings
        );
    }

    #[test]
    fn folder_file_listing_handles_root_direct_files_and_source_filters() {
        let (connection, _settings, _seed_pack) = setup_library_env();
        insert_library_file_row(
            &connection,
            "C:/Mods/root_only.package",
            "root_only.package",
            "mods",
            0,
            "Gameplay",
            0.84,
            None,
        );
        insert_library_file_row(
            &connection,
            "C:/Tray/root_household.trayitem",
            "root_household.trayitem",
            "tray",
            0,
            "TrayHousehold",
            0.91,
            None,
        );

        let mods_root = list_library_folder_files(
            &connection,
            LibraryFolderFilesQuery {
                folder_path: "Mods".to_owned(),
                recursive: false,
                limit: Some(50),
                include_previews: Some(false),
                ..Default::default()
            },
        )
        .expect("mods root files");
        assert!(mods_root
            .items
            .iter()
            .any(|item| item.filename == "root_only.package"));
        assert!(mods_root
            .items
            .iter()
            .all(|item| item.source_location == "mods"));

        let tray_root = list_library_folder_files(
            &connection,
            LibraryFolderFilesQuery {
                folder_path: "Tray".to_owned(),
                recursive: false,
                filters: LibraryQuery {
                    source: Some("tray".to_owned()),
                    ..Default::default()
                },
                limit: Some(50),
                include_previews: Some(false),
                ..Default::default()
            },
        )
        .expect("tray root files");
        assert_eq!(tray_root.total, 1);
        assert_eq!(tray_root.items[0].filename, "root_household.trayitem");

        let mismatched_source = list_library_folder_files(
            &connection,
            LibraryFolderFilesQuery {
                folder_path: "Tray".to_owned(),
                recursive: false,
                filters: LibraryQuery {
                    source: Some("mods".to_owned()),
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .expect("mismatched source");
        assert_eq!(mismatched_source.total, 0);
        assert!(mismatched_source.items.is_empty());
    }

    #[test]
    fn folder_tree_metadata_uses_current_profile_identity_for_relocated_paths_and_counts() {
        let (connection, _settings, _seed_pack) = setup_library_env();
        connection
            .execute(
                "DELETE FROM files WHERE source_location IN ('mods', 'tray')",
                [],
            )
            .expect("clear fixture files");
        connection
            .execute("DELETE FROM library_folders", [])
            .expect("clear fixture folders");

        let (profile_id, root_id) = active_profile_root_identity(&connection, "mods");
        let root_folder_id =
            insert_library_folder_row(&connection, "mods", "", "", None, "Mods", 0, "C:/Mods");
        assign_library_folder_identity(
            &connection,
            root_folder_id,
            &profile_id,
            &root_id,
            "",
            None,
        );
        let child_folder_id = insert_library_folder_row(
            &connection,
            "mods",
            "PortableFolder",
            "portablefolder",
            Some(""),
            "PortableFolder",
            1,
            "C:/Mods/PortableFolder",
        );
        let child_folder_key = sensitive_path_key(&["PortableFolder"]);
        assign_library_folder_identity(
            &connection,
            child_folder_id,
            &profile_id,
            &root_id,
            "PortableFolder",
            Some(&child_folder_key),
        );

        let root_file_key = sensitive_path_key(&["root.package"]);
        insert_identity_library_file_row(
            &connection,
            "Z:/Relocated/RootAlias/root.package",
            "root.package",
            "mods",
            0,
            &profile_id,
            &root_id,
            "root.package",
            Some(&root_file_key),
        );
        let child_file_key = sensitive_path_key(&["PortableFolder", "current.package"]);
        insert_identity_library_file_row(
            &connection,
            "Z:/Misleading/AbsoluteTail/NotPortable/current.package",
            "current.package",
            "mods",
            1,
            &profile_id,
            &root_id,
            "PortableFolder/current.package",
            Some(&child_file_key),
        );
        let foreign_file_key = sensitive_path_key(&["PortableFolder", "foreign.package"]);
        insert_identity_library_file_row(
            &connection,
            "C:/Mods/PortableFolder/foreign.package",
            "foreign.package",
            "mods",
            1,
            "foreign-profile",
            &root_id,
            "PortableFolder/foreign.package",
            Some(&foreign_file_key),
        );

        let file_selection = load_folder_tree_file_counts(
            &connection,
            &LibraryQuery {
                source: Some("mods".to_owned()),
                ..Default::default()
            },
        )
        .expect("pre-v13 fallback selection");
        assert!(file_selection.direct_count_rows.is_empty());
        assert_eq!(file_selection.fallback_file_rows.len(), 3);

        let metadata = get_folder_tree_metadata(
            &connection,
            &LibraryQuery {
                source: Some("mods".to_owned()),
                ..Default::default()
            },
        )
        .expect("profile-aware folder metadata");

        let mods = metadata
            .roots
            .iter()
            .find(|node| node.name == "Mods")
            .expect("mods root");
        assert_eq!(mods.direct_file_count, 1);
        assert_eq!(mods.total_file_count, 2);
        assert_eq!(mods.child_folder_count, 1);

        let portable = mods
            .children
            .iter()
            .find(|node| node.name == "PortableFolder")
            .expect("portable folder");
        assert_eq!(portable.path, "Mods/PortableFolder");
        assert_eq!(portable.disk_path.as_deref(), Some("C:/Mods/PortableFolder"));
        assert_eq!(portable.direct_file_count, 1);
        assert_eq!(portable.total_file_count, 1);
        assert!(mods.children.iter().all(|node| node.name != "NotPortable"));
    }

    #[test]
    fn folder_tree_counts_group_by_parent_identity_and_preserve_case_distinct_siblings() {
        let (connection, _settings, _seed_pack) = setup_library_env();
        connection
            .execute(
                "DELETE FROM files WHERE source_location IN ('mods', 'tray')",
                [],
            )
            .expect("clear fixture files");
        connection
            .execute("DELETE FROM library_folders", [])
            .expect("clear fixture folders");

        let (profile_id, root_id) = active_profile_root_identity(&connection, "mods");
        let root_folder_id =
            insert_library_folder_row(&connection, "mods", "", "", None, "Mods", 0, "C:/Mods");
        assign_library_folder_identity(
            &connection,
            root_folder_id,
            &profile_id,
            &root_id,
            "",
            None,
        );

        let creator_upper_id = insert_library_folder_row(
            &connection,
            "mods",
            "Creator",
            "creator",
            Some(""),
            "Creator",
            1,
            "C:/Mods/Creator",
        );
        let creator_upper_key = sensitive_path_key(&["Creator"]);
        assign_library_folder_identity(
            &connection,
            creator_upper_id,
            &profile_id,
            &root_id,
            "Creator",
            Some(&creator_upper_key),
        );
        let creator_lower_id = insert_library_folder_row(
            &connection,
            "mods",
            "creator",
            "creator",
            Some(""),
            "creator",
            1,
            "C:/Mods/creator",
        );
        let creator_lower_key = sensitive_path_key(&["creator"]);
        assign_library_folder_identity(
            &connection,
            creator_lower_id,
            &profile_id,
            &root_id,
            "creator",
            Some(&creator_lower_key),
        );

        let root_file_key = sensitive_path_key(&["root_keep.package"]);
        let root_file_id = insert_identity_library_file_row(
            &connection,
            "Z:/Relocated/root_keep.package",
            "root_keep.package",
            "mods",
            0,
            &profile_id,
            &root_id,
            "root_keep.package",
            Some(&root_file_key),
        );
        assign_file_parent_identity(&connection, root_file_id, "", None);

        for index in 0..120 {
            let filename = format!("keep_{index:03}.package");
            let relative_path = format!("Creator/{filename}");
            let file_key = sensitive_path_key(&["Creator", &filename]);
            let file_id = insert_identity_library_file_row(
                &connection,
                &format!("Z:/Relocated/Creator/{filename}"),
                &filename,
                "mods",
                1,
                &profile_id,
                &root_id,
                &relative_path,
                Some(&file_key),
            );
            assign_file_parent_identity(
                &connection,
                file_id,
                "Creator",
                Some(&creator_upper_key),
            );
        }

        for index in 0..80 {
            let filename = format!("skip_{index:03}.package");
            let relative_path = format!("creator/{filename}");
            let file_key = sensitive_path_key(&["creator", &filename]);
            let file_id = insert_identity_library_file_row(
                &connection,
                &format!("Z:/Relocated/creator/{filename}"),
                &filename,
                "mods",
                1,
                &profile_id,
                &root_id,
                &relative_path,
                Some(&file_key),
            );
            assign_file_parent_identity(
                &connection,
                file_id,
                "creator",
                Some(&creator_lower_key),
            );
        }

        let full_query = LibraryQuery {
            source: Some("mods".to_owned()),
            ..Default::default()
        };
        let file_selection =
            load_folder_tree_file_counts(&connection, &full_query).expect("grouped count rows");
        assert_eq!(file_selection.direct_count_rows.len(), 3);
        assert!(file_selection.fallback_file_rows.is_empty());
        assert_eq!(
            file_selection
                .direct_count_rows
                .iter()
                .map(|row| row.direct_file_count)
                .sum::<i64>(),
            201
        );

        let metadata =
            get_folder_tree_metadata(&connection, &full_query).expect("grouped folder metadata");
        let mods = metadata
            .roots
            .iter()
            .find(|node| node.name == "Mods")
            .expect("mods root");
        assert_eq!(mods.direct_file_count, 1);
        assert_eq!(mods.total_file_count, 201);
        assert_eq!(mods.child_folder_count, 2);
        let creator_upper = mods
            .children
            .iter()
            .find(|node| node.name == "Creator")
            .expect("upper-case creator folder");
        let creator_lower = mods
            .children
            .iter()
            .find(|node| node.name == "creator")
            .expect("lower-case creator folder");
        assert_eq!(creator_upper.path, "Mods/Creator");
        assert_eq!(creator_upper.direct_file_count, 120);
        assert_eq!(creator_upper.total_file_count, 120);
        assert_eq!(creator_lower.path, "Mods/creator");
        assert_eq!(creator_lower.direct_file_count, 80);
        assert_eq!(creator_lower.total_file_count, 80);

        let filtered_query = LibraryQuery {
            source: Some("mods".to_owned()),
            search: Some("keep".to_owned()),
            ..Default::default()
        };
        let filtered_selection = load_folder_tree_file_counts(&connection, &filtered_query)
            .expect("filtered grouped count rows");
        assert_eq!(filtered_selection.direct_count_rows.len(), 2);
        assert!(filtered_selection.fallback_file_rows.is_empty());
        assert_eq!(
            filtered_selection
                .direct_count_rows
                .iter()
                .map(|row| row.direct_file_count)
                .sum::<i64>(),
            121
        );

        let filtered = get_folder_tree_metadata(&connection, &filtered_query)
            .expect("filtered grouped folder metadata");
        let filtered_mods = filtered
            .roots
            .iter()
            .find(|node| node.name == "Mods")
            .expect("filtered mods root");
        assert_eq!(filtered_mods.direct_file_count, 1);
        assert_eq!(filtered_mods.total_file_count, 121);
        assert_eq!(
            filtered_mods
                .children
                .iter()
                .find(|node| node.name == "Creator")
                .expect("filtered upper creator")
                .direct_file_count,
            120
        );
        assert_eq!(
            filtered_mods
                .children
                .iter()
                .find(|node| node.name == "creator")
                .expect("filtered lower creator remains visible")
                .direct_file_count,
            0
        );
    }

    #[test]
    fn folder_tree_metadata_blocks_a_source_owned_by_a_stale_profile() {
        let (connection, _settings, _seed_pack) = setup_library_env();
        connection
            .execute(
                "DELETE FROM files WHERE source_location IN ('mods', 'tray')",
                [],
            )
            .expect("clear fixture files");
        connection
            .execute("DELETE FROM library_folders", [])
            .expect("clear fixture folders");

        let (_profile_id, root_id) = active_profile_root_identity(&connection, "mods");
        let root_folder_id =
            insert_library_folder_row(&connection, "mods", "", "", None, "Mods", 0, "C:/Mods");
        assign_library_folder_identity(
            &connection,
            root_folder_id,
            "foreign-profile",
            &root_id,
            "",
            None,
        );
        insert_library_file_row(
            &connection,
            "C:/Mods/legacy-leak.package",
            "legacy-leak.package",
            "mods",
            0,
            "Gameplay",
            0.8,
            None,
        );

        let metadata = get_folder_tree_metadata(
            &connection,
            &LibraryQuery {
                source: Some("mods".to_owned()),
                ..Default::default()
            },
        )
        .expect("stale-profile folder metadata");

        assert_eq!(metadata.total_folders, 0);
        assert!(metadata.roots.is_empty());
    }

    #[test]
    fn folder_tree_source_selection_blocks_incomplete_and_unsafe_identity() {
        let incomplete = FolderMetadataRow {
            source_location: "mods".to_owned(),
            relative_path: "".to_owned(),
            name: "Mods".to_owned(),
            depth: 0,
            disk_path: "C:/Mods".to_owned(),
            installation_profile_id: Some("active-profile".to_owned()),
            installation_root_id: None,
            profile_relative_path: Some("".to_owned()),
            profile_relative_path_key: None,
        };
        let (incomplete_mode, incomplete_rows) =
            select_folder_tree_source_rows(Some("active-profile"), vec![incomplete]);
        assert!(matches!(incomplete_mode, FolderTreeSourceMode::Blocked));
        assert!(incomplete_rows.is_empty());

        let unsafe_row = FolderMetadataRow {
            source_location: "mods".to_owned(),
            relative_path: "escape".to_owned(),
            name: "Escape".to_owned(),
            depth: 1,
            disk_path: "C:/Mods/Escape".to_owned(),
            installation_profile_id: Some("active-profile".to_owned()),
            installation_root_id: Some("mods-root".to_owned()),
            profile_relative_path: Some("../Escape".to_owned()),
            profile_relative_path_key: None,
        };
        let (unsafe_mode, unsafe_rows) =
            select_folder_tree_source_rows(Some("active-profile"), vec![unsafe_row]);
        assert!(matches!(unsafe_mode, FolderTreeSourceMode::Blocked));
        assert!(unsafe_rows.is_empty());

        let child_without_root = FolderMetadataRow {
            source_location: "mods".to_owned(),
            relative_path: "Creator".to_owned(),
            name: "Creator".to_owned(),
            depth: 1,
            disk_path: "C:/Mods/Creator".to_owned(),
            installation_profile_id: Some("active-profile".to_owned()),
            installation_root_id: Some("mods-root".to_owned()),
            profile_relative_path: Some("Creator".to_owned()),
            profile_relative_path_key: None,
        };
        let (missing_root_mode, missing_root_rows) =
            select_folder_tree_source_rows(Some("active-profile"), vec![child_without_root]);
        assert!(matches!(missing_root_mode, FolderTreeSourceMode::Blocked));
        assert!(missing_root_rows.is_empty());

        let current_root = FolderMetadataRow {
            source_location: "mods".to_owned(),
            relative_path: "".to_owned(),
            name: "Mods".to_owned(),
            depth: 0,
            disk_path: "C:/Mods".to_owned(),
            installation_profile_id: Some("active-profile".to_owned()),
            installation_root_id: Some("mods-root".to_owned()),
            profile_relative_path: Some("".to_owned()),
            profile_relative_path_key: None,
        };
        let child_from_second_root = FolderMetadataRow {
            source_location: "mods".to_owned(),
            relative_path: "Other".to_owned(),
            name: "Other".to_owned(),
            depth: 1,
            disk_path: "D:/Mods/Other".to_owned(),
            installation_profile_id: Some("active-profile".to_owned()),
            installation_root_id: Some("second-mods-root".to_owned()),
            profile_relative_path: Some("Other".to_owned()),
            profile_relative_path_key: None,
        };
        let (mixed_root_mode, mixed_root_rows) = select_folder_tree_source_rows(
            Some("active-profile"),
            vec![current_root, child_from_second_root],
        );
        assert!(matches!(mixed_root_mode, FolderTreeSourceMode::Blocked));
        assert!(mixed_root_rows.is_empty());
    }

    #[test]
    fn folder_tree_metadata_builds_nested_counts() {
        let (connection, _settings, _seed_pack) = setup_library_env();
        let insights_json =
            serde_json::to_string(&crate::models::FileInsights::default()).expect("insights json");

        connection
            .execute(
                "UPDATE files SET relative_depth = 1 WHERE filename = 'installed.package'",
                [],
            )
            .expect("align fixture depth");
        connection
            .execute(
                "INSERT INTO files (
                    path,
                    filename,
                    extension,
                    kind,
                    confidence,
                    source_location,
                    relative_depth,
                    parser_warnings,
                    insights
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    "C:/Mods/root.package",
                    "root.package",
                    ".package",
                    "Gameplay",
                    0.8_f64,
                    "mods",
                    0_i64,
                    "[]",
                    insights_json,
                ],
            )
            .expect("root file");
        connection
            .execute(
                "INSERT INTO files (
                    path,
                    filename,
                    extension,
                    kind,
                    confidence,
                    source_location,
                    relative_depth,
                    parser_warnings,
                    insights
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    "C:/Mods/TestCreator/Nested/deep.package",
                    "deep.package",
                    ".package",
                    "Gameplay",
                    0.8_f64,
                    "mods",
                    2_i64,
                    "[]",
                    serde_json::to_string(&crate::models::FileInsights::default())
                        .expect("insights json"),
                ],
            )
            .expect("nested file");
        connection
            .execute(
                "INSERT INTO files (
                    path,
                    filename,
                    extension,
                    kind,
                    confidence,
                    source_location,
                    relative_depth,
                    parser_warnings,
                    insights
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    "C:/Tray/household.trayitem",
                    "household.trayitem",
                    ".trayitem",
                    "TrayHousehold",
                    0.9_f64,
                    "tray",
                    0_i64,
                    "[]",
                    serde_json::to_string(&crate::models::FileInsights::default())
                        .expect("insights json"),
                ],
            )
            .expect("tray file");

        let metadata = get_folder_tree_metadata(&connection, &LibraryQuery::default())
            .expect("folder metadata");

        let mods = metadata
            .roots
            .iter()
            .find(|node| node.name == "Mods")
            .expect("mods root");
        assert_eq!(mods.direct_file_count, 1);
        assert_eq!(mods.total_file_count, 3);
        assert_eq!(mods.child_folder_count, 1);

        let test_creator = mods
            .children
            .iter()
            .find(|node| node.name == "TestCreator")
            .expect("creator folder");
        assert_eq!(test_creator.path, "Mods/TestCreator");
        assert_eq!(test_creator.direct_file_count, 1);
        assert_eq!(test_creator.total_file_count, 2);
        assert_eq!(test_creator.child_folder_count, 1);

        let nested = test_creator
            .children
            .iter()
            .find(|node| node.name == "Nested")
            .expect("nested folder");
        assert_eq!(nested.direct_file_count, 1);
        assert_eq!(nested.total_file_count, 1);

        let tray = metadata
            .roots
            .iter()
            .find(|node| node.name == "Tray")
            .expect("tray root");
        assert_eq!(tray.direct_file_count, 1);
        assert_eq!(tray.total_file_count, 1);
    }

    #[test]
    fn folder_tree_metadata_includes_true_empty_folder_rows() {
        let (connection, _settings, _seed_pack) = setup_library_env();
        connection
            .execute(
                "DELETE FROM files WHERE source_location IN ('mods', 'tray')",
                [],
            )
            .expect("clear fixture files");

        insert_library_folder_row(&connection, "mods", "", "", None, "Mods", 0, "C:/Mods");
        insert_library_folder_row(
            &connection,
            "mods",
            "Empty",
            "empty",
            Some(""),
            "Empty",
            1,
            "C:/Mods/Empty",
        );
        insert_library_folder_row(
            &connection,
            "mods",
            "Empty/Nested",
            "empty/nested",
            Some("empty"),
            "Nested",
            2,
            "C:/Mods/Empty/Nested",
        );
        insert_library_folder_row(&connection, "tray", "", "", None, "Tray", 0, "C:/Tray");
        insert_library_folder_row(
            &connection,
            "tray",
            "Saved Rooms",
            "saved rooms",
            Some(""),
            "Saved Rooms",
            1,
            "C:/Tray/Saved Rooms",
        );

        let metadata = get_folder_tree_metadata(&connection, &LibraryQuery::default())
            .expect("folder metadata");

        let mods = metadata
            .roots
            .iter()
            .find(|node| node.name == "Mods")
            .expect("mods root");
        assert_eq!(mods.disk_path.as_deref(), Some("C:/Mods"));
        assert_eq!(mods.direct_file_count, 0);
        assert_eq!(mods.total_file_count, 0);
        assert_eq!(mods.child_folder_count, 1);

        let empty = mods
            .children
            .iter()
            .find(|node| node.name == "Empty")
            .expect("empty folder");
        assert_eq!(empty.path, "Mods/Empty");
        assert_eq!(empty.disk_path.as_deref(), Some("C:/Mods/Empty"));
        assert_eq!(empty.direct_file_count, 0);
        assert_eq!(empty.total_file_count, 0);
        assert_eq!(empty.child_folder_count, 1);

        let nested = empty
            .children
            .iter()
            .find(|node| node.name == "Nested")
            .expect("nested empty folder");
        assert_eq!(nested.path, "Mods/Empty/Nested");
        assert_eq!(nested.disk_path.as_deref(), Some("C:/Mods/Empty/Nested"));
        assert_eq!(nested.direct_file_count, 0);
        assert_eq!(nested.total_file_count, 0);

        let tray = metadata
            .roots
            .iter()
            .find(|node| node.name == "Tray")
            .expect("tray root");
        let saved_rooms = tray
            .children
            .iter()
            .find(|node| node.name == "Saved Rooms")
            .expect("empty tray folder");
        assert_eq!(
            saved_rooms.disk_path.as_deref(),
            Some("C:/Tray/Saved Rooms")
        );
        assert_eq!(saved_rooms.total_file_count, 0);

        let empty_listing = list_library_folder_files(
            &connection,
            LibraryFolderFilesQuery {
                folder_path: "Mods/Empty".to_owned(),
                recursive: false,
                limit: Some(25),
                include_previews: Some(false),
                ..Default::default()
            },
        )
        .expect("empty folder direct listing");
        assert_eq!(empty_listing.total, 0);
        assert!(empty_listing.items.is_empty());

        let recursive_listing = list_library_folder_files(
            &connection,
            LibraryFolderFilesQuery {
                folder_path: "Mods/Empty".to_owned(),
                recursive: true,
                limit: Some(25),
                include_previews: Some(false),
                ..Default::default()
            },
        )
        .expect("empty folder recursive listing");
        assert_eq!(recursive_listing.total, 0);
        assert!(recursive_listing.items.is_empty());
    }

    #[test]
    fn relationship_peer_counts_use_real_parent_folder_and_real_bundle() {
        let (connection, _settings, _seed_pack) = setup_library_env();
        let insights_json =
            serde_json::to_string(&crate::models::FileInsights::default()).expect("insights json");

        connection
            .execute(
                "UPDATE files SET relative_depth = 1 WHERE filename = 'installed.package'",
                [],
            )
            .expect("align fixture depth");
        connection
            .execute(
                "INSERT INTO files (
                    path,
                    filename,
                    extension,
                    kind,
                    confidence,
                    source_location,
                    relative_depth,
                    parser_warnings,
                    insights
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    "C:/Mods/TestCreator/sibling.package",
                    "sibling.package",
                    ".package",
                    "Gameplay",
                    0.8_f64,
                    "mods",
                    1_i64,
                    "[]",
                    insights_json,
                ],
            )
            .expect("same folder file");
        connection
            .execute(
                "INSERT INTO files (
                    path,
                    filename,
                    extension,
                    kind,
                    confidence,
                    source_location,
                    relative_depth,
                    parser_warnings,
                    insights
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    "C:/Mods/Other/other.package",
                    "other.package",
                    ".package",
                    "Gameplay",
                    0.8_f64,
                    "mods",
                    1_i64,
                    "[]",
                    serde_json::to_string(&crate::models::FileInsights::default())
                        .expect("insights json"),
                ],
            )
            .expect("different folder file");

        let listing =
            list_library_files(&connection, LibraryQuery::default()).expect("library listing");
        let installed = listing
            .items
            .iter()
            .find(|item| item.filename == "installed.package")
            .expect("installed row");
        let other = listing
            .items
            .iter()
            .find(|item| item.filename == "other.package")
            .expect("other row");

        assert_eq!(installed.same_folder_peer_count, 1);
        assert_eq!(other.same_folder_peer_count, 0);
        assert_eq!(installed.same_pack_peer_count, 0);
        assert_eq!(other.same_pack_peer_count, 0);
    }

    #[test]
    fn facets_and_listing_respect_kind_scoped_subtypes() {
        let (connection, _settings, seed_pack) = setup_library_env();

        connection
            .execute(
                "INSERT INTO files (
                    path,
                    filename,
                    extension,
                    kind,
                    subtype,
                    confidence,
                    source_location,
                    parser_warnings,
                    insights
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    "C:/Mods/BuildBuy/chair.package",
                    "chair.package",
                    ".package",
                    "BuildBuy",
                    "Seating",
                    0.82_f64,
                    "mods",
                    "[]",
                    serde_json::to_string(&crate::models::FileInsights::default())
                        .expect("insights json"),
                ],
            )
            .expect("buildbuy file");

        let facets =
            get_library_facets(&connection, &seed_pack.taxonomy, Some("BuildBuy")).expect("facets");
        assert_eq!(facets.subtypes, vec!["Seating".to_owned()]);

        let listing = list_library_files(
            &connection,
            LibraryQuery {
                kind: Some("BuildBuy".to_owned()),
                subtype: Some("Seating".to_owned()),
                ..Default::default()
            },
        )
        .expect("filtered listing");
        assert_eq!(listing.total, 1);
        assert_eq!(listing.items[0].kind, "BuildBuy");
        assert_eq!(listing.items[0].subtype.as_deref(), Some("Seating"));
    }
}
