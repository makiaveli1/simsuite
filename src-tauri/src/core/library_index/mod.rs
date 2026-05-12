use rusqlite::{params, params_from_iter, types::Value, Connection, OptionalExtension};
use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    path::Path,
};

use crate::{
    core::{content_versions, scanner},
    database,
    error::AppResult,
    models::{
        CategoryOverrideInfo, CreatorLearningInfo, FileDetail, FileInsights, FolderTreeMetadata,
        FolderTreeNode, HomeOverview, LibraryFacets, LibraryFileRow, LibraryFolderFilesQuery,
        LibraryListResponse, LibraryQuery, LibrarySettings, LibrarySortField, LibrarySummary,
        LibraryWatchFilter, ProblemSignal, ProblemSignalDestination, ProblemSignalProofLevel,
        ProblemSignalSeverity, WatchStatus,
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

fn build_folder_scope_filter(
    connection: &Connection,
    source: &str,
    target_segments: &[String],
    recursive: bool,
) -> AppResult<FolderScopeFilter> {
    let child_segments = target_segments.iter().skip(1).cloned().collect::<Vec<_>>();
    let child_depth = child_segments.len() as i64;
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
}

#[derive(Debug)]
struct FolderMetadataRow {
    source_location: String,
    relative_path: String,
    name: String,
    depth: i64,
    disk_path: String,
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
    let folder_rows = load_library_folder_rows(connection, query)?;
    let (filters, params) = build_filters(query);
    let sql = format!(
        "SELECT f.path, f.source_location, f.relative_depth\n\
         FROM files f\n\
         LEFT JOIN creators c ON f.creator_id = c.id\n\
         LEFT JOIN content_watch_sources cws ON cws.anchor_file_id = f.id\n\
         LEFT JOIN content_watch_results cwr ON cwr.subject_key = cws.subject_key\n\
         WHERE f.source_location <> 'downloads'\n\
        {filters}\n\
         ORDER BY f.source_location COLLATE NOCASE, f.path COLLATE NOCASE",
        filters = filters
    );

    let mut statement = connection.prepare(&sql)?;
    let rows = statement
        .query_map(params_from_iter(params.iter()), |row| {
            Ok(FolderFileRow {
                path: row.get(0)?,
                source_location: row.get(1)?,
                relative_depth: row.get(2)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(build_folder_metadata_from_rows(folder_rows, rows))
}

fn load_library_folder_rows(
    connection: &Connection,
    query: &LibraryQuery,
) -> AppResult<Vec<FolderMetadataRow>> {
    let sources = folder_sources_for_query(query);
    if sources.is_empty() {
        return Ok(Vec::new());
    }

    let placeholders = std::iter::repeat("?")
        .take(sources.len())
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!(
        "SELECT source_location, relative_path, name, depth, full_path\n\
         FROM library_folders\n\
         WHERE source_location IN ({placeholders})\n\
         ORDER BY source_location COLLATE NOCASE, normalized_relative_path COLLATE NOCASE"
    );
    let params = sources.into_iter().map(Value::Text).collect::<Vec<Value>>();

    let mut statement = connection.prepare(&sql)?;
    let rows = statement
        .query_map(params_from_iter(params.iter()), |row| {
            Ok(FolderMetadataRow {
                source_location: row.get(0)?,
                relative_path: row.get(1)?,
                name: row.get(2)?,
                depth: row.get(3)?,
                disk_path: row.get(4)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(rows)
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
    file_rows: Vec<FolderFileRow>,
) -> FolderTreeMetadata {
    let mut nodes: BTreeMap<String, FolderNodeAccumulator> = BTreeMap::new();

    for row in folder_rows {
        add_folder_metadata_row(&mut nodes, row);
    }

    for row in file_rows {
        let Some(segments) =
            folder_segments_for_file(&row.path, &row.source_location, row.relative_depth)
        else {
            continue;
        };

        for index in 0..segments.len() {
            let path = segments[..=index].join("/");
            let source_location = row.source_location.to_ascii_lowercase();
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
            entry.total_file_count += 1;
            if index == segments.len() - 1 {
                entry.direct_file_count += 1;
            }

            if index > 0 {
                let parent_path = segments[..index].join("/");
                if let Some(parent) = nodes.get_mut(&parent_path) {
                    parent.children.insert(path.clone());
                }
            }
        }
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

            // Phase 5an: resolve thumbnails on-demand if they were deferred during scan.
            // During scan, thumbnails are skipped (THUMBNAIL_DEFERRED=true) to avoid
            // 3× DBPF re-parse per file. Here we do the deferred thumbnail work.
            use crate::core::file_inspector::resolve_package_thumbnails_deferred;
            let (embedded_thumb, cached_thumb) =
                resolve_package_thumbnails_deferred(Path::new(&detail.path));
            detail.insights.thumbnail_preview =
                detail.insights.thumbnail_preview.or(embedded_thumb);
            detail.insights.cached_thumbnail_preview =
                detail.insights.cached_thumbnail_preview.or(cached_thumb);

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
            "ORDER BY\
                 CASE WHEN f.modified_at IS NULL THEN 1 ELSE 0 END ASC,\
                 f.modified_at DESC,\
                 f.filename COLLATE NOCASE ASC",
        ),
        LibrarySortField::HasUpdatesFirst => {
            // Sort by update priority: update leads first, then failed or unclear
            // checks, then quieter reminder/current/not-watched states.
            String::from(
                "ORDER BY\
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
    format!(
        "{pair}.file_id_a <> {pair}.file_id_b
         AND TRIM(COALESCE({left}.hash, '')) <> ''
         AND TRIM(COALESCE({right}.hash, '')) <> ''
         AND LOWER(TRIM({left}.hash)) = LOWER(TRIM({right}.hash))
         AND TRIM(COALESCE({left}.path, '')) <> ''
         AND TRIM(COALESCE({right}.path, '')) <> ''
         AND LOWER(REPLACE(TRIM({left}.path), '/', '\\')) <> LOWER(REPLACE(TRIM({right}.path), '/', '\\'))",
        left = left_alias,
        right = right_alias,
        pair = pair_alias
    )
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
        models::{LibraryFolderFilesQuery, LibraryQuery, LibrarySettings, LibraryWatchFilter},
        seed::load_seed_pack,
    };
    use std::time::Instant;

    use super::{
        get_file_detail, get_folder_tree_metadata, get_library_facets, list_library_files,
        list_library_folder_files, MAX_FOLDER_QUERY_LIMIT,
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
    fn folder_file_listing_uses_root_scoped_sql_and_avoids_path_tail_false_positive() {
        let (connection, _settings, _seed_pack) = setup_library_env();

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
