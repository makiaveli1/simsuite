use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    path::{Path, PathBuf},
};

use rusqlite::{params, params_from_iter, types::Value, Connection, OptionalExtension};

use crate::{
    core::special_mod_versions::{
        self, build_signature, compare_versions, parse_version_parts, SignatureEntry,
    },
    error::AppResult,
    models::{
        FileInsights, InstalledVersionSummary, LibrarySettings, LibraryWatchListItem,
        LibraryWatchListResponse, LibraryWatchReviewItem, LibraryWatchReviewReason,
        LibraryWatchReviewResponse, LibraryWatchSetupItem, LibraryWatchSetupResponse,
        SpecialVersionStatus, VersionCompareStatus, VersionConfidence, VersionResolution,
        VersionSignal, WatchCapability, WatchListFilter, WatchResult, WatchSourceKind,
        WatchSourceOrigin, WatchStatus,
    },
    seed::{GuidedInstallProfileSeed, SeedPack},
};

const MAX_CANDIDATE_ROWS: usize = 96;
const MAX_SEARCH_TOKENS: usize = 4;
const MAX_SEARCH_FAMILY_HINTS: usize = 4;
const MAX_WATCH_LIST_LIMIT: usize = 48;
const MAX_WATCH_REVIEW_LIMIT: usize = 24;
const MAX_WATCH_SETUP_LIMIT: usize = 24;
const MAX_WATCH_SETUP_EXACT_LIMIT: usize = 8;
const WATCH_SETUP_SCAN_LIMIT: usize = 320;
const WATCH_SETUP_QUERY_LIMIT: usize = WATCH_SETUP_SCAN_LIMIT * 4;
const MATCH_SCORE_STRONG: f64 = 1.20;
const MATCH_SCORE_MEDIUM: f64 = 0.80;
const MATCH_SCORE_WEAK: f64 = 0.45;
const SIGNAL_CONFLICT_CONFIDENCE: f64 = 0.72;
const MIN_FAMILY_HINT_SEARCH_LEN: usize = 4;
const GENERIC_SOURCE_ORDER: [&str; 5] = [
    "payload",
    "embedded_name",
    "filename",
    "archive_path",
    "resource_summary",
];
const TOKEN_STOP_WORDS: [&str; 14] = [
    "package",
    "script",
    "scripts",
    "ts4script",
    "zip",
    "mod",
    "mods",
    "the",
    "and",
    "for",
    "with",
    "sims",
    "simsuite",
    "version",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompareDetailLevel {
    Queue,
    Full,
}

#[derive(Debug, Clone)]
struct SubjectFileRow {
    id: i64,
    filename: String,
    path: String,
    hash: Option<String>,
    size: i64,
    creator: Option<String>,
    source_location: String,
    insights: FileInsights,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum SubjectLocator {
    DownloadItem(i64),
    Folder(String),
    Family(String),
    FileStem(String),
}

#[derive(Debug, Clone, Default)]
struct SubjectVersion {
    value: Option<String>,
    confidence: VersionConfidence,
    evidence: Vec<String>,
    ambiguous: bool,
}

#[derive(Debug, Clone)]
struct VersionSubject {
    key: String,
    label: String,
    aggregate_signature: Option<String>,
    all_hashes_present: bool,
    creator_tokens: BTreeSet<String>,
    family_tokens: BTreeSet<String>,
    namespace_tokens: BTreeSet<String>,
    embedded_tokens: BTreeSet<String>,
    filename_tokens: BTreeSet<String>,
    version: SubjectVersion,
    files: Vec<SubjectFileRow>,
}

#[derive(Debug, Clone, Default)]
struct MatchBreakdown {
    score: f64,
    evidence: Vec<String>,
}

#[derive(Debug, Clone)]
struct WatchSourceRow {
    source_kind: WatchSourceKind,
    source_label: Option<String>,
    source_url: String,
}

#[derive(Debug, Clone)]
struct WatchResultRow {
    status: WatchStatus,
    latest_version: Option<String>,
    checked_at: Option<String>,
    confidence: VersionConfidence,
    note: Option<String>,
    evidence: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WatchSourceCapabilityKind {
    CanRefreshNow,
    SavedReferenceOnly,
    ProviderRequired,
}

#[derive(Debug, Clone)]
struct WatchSourceCapability {
    kind: WatchSourceCapabilityKind,
    note: String,
    provider_name: Option<String>,
}

pub fn resolve_download_item_version(
    connection: &Connection,
    settings: &crate::models::LibrarySettings,
    seed_pack: &SeedPack,
    item_id: i64,
    detail_level: CompareDetailLevel,
) -> AppResult<Option<VersionResolution>> {
    let incoming_rows = load_download_subject_rows(connection, item_id)?;
    if incoming_rows.is_empty() {
        return Ok(None);
    }

    let incoming_subject = build_subject(
        incoming_rows,
        SubjectLocator::DownloadItem(item_id),
        settings.mods_path.as_deref(),
    );
    let installed_rows =
        load_installed_candidate_rows(connection, &incoming_subject, detail_level)?;
    let resolution = resolve_against_installed_subjects(
        incoming_subject,
        installed_rows,
        settings.mods_path.as_deref(),
        None,
        seed_pack,
    );
    Ok(Some(resolution))
}

pub fn resolve_library_file_version(
    connection: &Connection,
    settings: &LibrarySettings,
    seed_pack: &SeedPack,
    file_id: i64,
) -> AppResult<(Option<InstalledVersionSummary>, Option<WatchResult>)> {
    let Some(file_row) = load_library_subject_row(connection, file_id)? else {
        return Ok((None, None));
    };

    let locator = subject_locator_for_row(&file_row, settings.mods_path.as_deref());
    let subject_rows = load_subject_rows_for_locator(
        connection,
        settings.mods_path.as_deref(),
        &locator,
        file_row.id,
    )?;
    let subject = build_subject(subject_rows, locator, settings.mods_path.as_deref());
    let installed_summary = Some(InstalledVersionSummary {
        subject_label: subject.label.clone(),
        subject_key: subject.key.clone(),
        version: subject.version.value.clone(),
        signature: subject.aggregate_signature.clone(),
        confidence: subject.version.confidence.clone(),
        evidence: subject.version.evidence.clone(),
    });
    let watch_result = resolve_watch_result(connection, seed_pack, &subject)?;

    Ok((installed_summary, watch_result))
}

pub fn list_library_watch_items(
    connection: &Connection,
    settings: &LibrarySettings,
    seed_pack: &SeedPack,
    filter: WatchListFilter,
    limit: usize,
) -> AppResult<LibraryWatchListResponse> {
    let mut seen_file_ids = HashSet::new();
    let mut candidate_file_ids = Vec::new();

    for file_id in load_saved_watch_anchor_file_ids(connection)? {
        if seen_file_ids.insert(file_id) {
            candidate_file_ids.push(file_id);
        }
    }

    for file_id in load_supported_special_watch_anchor_file_ids(connection, settings, seed_pack)? {
        if seen_file_ids.insert(file_id) {
            candidate_file_ids.push(file_id);
        }
    }

    let mut items = Vec::new();
    for file_id in candidate_file_ids {
        let Some(item) = build_library_watch_list_item(connection, settings, seed_pack, file_id)?
        else {
            continue;
        };

        if watch_result_matches_filter(&item.watch_result, filter) {
            items.push(item);
        }
    }

    items.sort_by(compare_library_watch_items);
    let total = items.len() as i64;
    items.truncate(limit.clamp(1, MAX_WATCH_LIST_LIMIT));

    Ok(LibraryWatchListResponse {
        filter,
        total,
        items,
    })
}

pub fn list_library_watch_setup_items(
    connection: &Connection,
    settings: &LibrarySettings,
    seed_pack: &SeedPack,
    limit: usize,
) -> AppResult<LibraryWatchSetupResponse> {
    let mut seen_subjects = HashSet::new();
    let mut items = Vec::new();

    for file_id in load_watch_setup_candidate_file_ids(connection)? {
        let Some(item) = build_library_watch_setup_item(
            connection,
            settings,
            seed_pack,
            file_id,
            &mut seen_subjects,
        )?
        else {
            continue;
        };
        items.push(item);
    }

    items.sort_by(compare_library_watch_setup_items);
    let total = items.len() as i64;
    let mut exact_page_items: Vec<_> = items
        .iter()
        .filter(|item| item.suggested_source_kind == WatchSourceKind::ExactPage)
        .cloned()
        .collect();
    let exact_page_total = exact_page_items.len() as i64;
    let exact_page_truncated = exact_page_items.len() > MAX_WATCH_SETUP_EXACT_LIMIT;
    exact_page_items.truncate(MAX_WATCH_SETUP_EXACT_LIMIT);
    let max_limit = limit.clamp(1, MAX_WATCH_SETUP_LIMIT);
    let truncated = items.len() > max_limit;
    items.truncate(max_limit);

    Ok(LibraryWatchSetupResponse {
        total,
        truncated,
        exact_page_total,
        exact_page_truncated,
        exact_page_items,
        items,
    })
}

pub fn list_library_watch_review_items(
    connection: &Connection,
    settings: &LibrarySettings,
    seed_pack: &SeedPack,
    limit: usize,
) -> AppResult<LibraryWatchReviewResponse> {
    let mut seen_file_ids = HashSet::new();
    let mut items = Vec::new();

    for file_id in load_saved_watch_anchor_file_ids(connection)? {
        if !seen_file_ids.insert(file_id) {
            continue;
        }

        let Some(item) = build_library_watch_review_item(connection, settings, seed_pack, file_id)?
        else {
            continue;
        };

        items.push(item);
    }

    items.sort_by(compare_library_watch_review_items);
    let total = items.len() as i64;
    let provider_needed_count = items
        .iter()
        .filter(|item| item.review_reason == LibraryWatchReviewReason::ProviderNeeded)
        .count() as i64;
    let reference_only_count = items
        .iter()
        .filter(|item| item.review_reason == LibraryWatchReviewReason::ReferenceOnly)
        .count() as i64;
    let unknown_result_count = items
        .iter()
        .filter(|item| item.review_reason == LibraryWatchReviewReason::UnknownResult)
        .count() as i64;
    items.truncate(limit.clamp(1, MAX_WATCH_REVIEW_LIMIT));

    Ok(LibraryWatchReviewResponse {
        total,
        provider_needed_count,
        reference_only_count,
        unknown_result_count,
        items,
    })
}

pub fn save_watch_source_for_library_file(
    connection: &Connection,
    settings: &LibrarySettings,
    seed_pack: &SeedPack,
    file_id: i64,
    source_kind: WatchSourceKind,
    source_label: Option<String>,
    source_url: &str,
) -> AppResult<Option<WatchResult>> {
    let Some(file_row) = load_library_subject_row(connection, file_id)? else {
        return Ok(None);
    };

    let locator = subject_locator_for_row(&file_row, settings.mods_path.as_deref());
    let subject_rows = load_subject_rows_for_locator(
        connection,
        settings.mods_path.as_deref(),
        &locator,
        file_row.id,
    )?;
    let subject = build_subject(subject_rows, locator, settings.mods_path.as_deref());

    if find_supported_profile(seed_pack, &subject).is_some() {
        return Err(
            "Supported special mods already use their own built-in official page here. Custom watch pages are not wired up yet."
                .into(),
        );
    }

    save_watch_source_for_subject(
        connection,
        &subject.key,
        Some(file_id),
        source_kind,
        source_label,
        source_url,
    )?;

    resolve_watch_result(connection, seed_pack, &subject)
}

pub fn clear_watch_source_for_library_file(
    connection: &Connection,
    settings: &LibrarySettings,
    seed_pack: &SeedPack,
    file_id: i64,
) -> AppResult<Option<WatchResult>> {
    let Some(file_row) = load_library_subject_row(connection, file_id)? else {
        return Ok(None);
    };

    let locator = subject_locator_for_row(&file_row, settings.mods_path.as_deref());
    let subject_rows = load_subject_rows_for_locator(
        connection,
        settings.mods_path.as_deref(),
        &locator,
        file_row.id,
    )?;
    let subject = build_subject(subject_rows, locator, settings.mods_path.as_deref());

    clear_watch_source_for_subject(connection, &subject.key)?;

    resolve_watch_result(connection, seed_pack, &subject)
}

pub fn refresh_watch_source_for_library_file(
    connection: &Connection,
    settings: &LibrarySettings,
    seed_pack: &SeedPack,
    file_id: i64,
) -> AppResult<Option<WatchResult>> {
    let Some(file_row) = load_library_subject_row(connection, file_id)? else {
        return Ok(None);
    };

    let locator = subject_locator_for_row(&file_row, settings.mods_path.as_deref());
    let subject_rows = load_subject_rows_for_locator(
        connection,
        settings.mods_path.as_deref(),
        &locator,
        file_row.id,
    )?;
    let subject = build_subject(subject_rows, locator, settings.mods_path.as_deref());

    if let Some(profile) = find_supported_profile(seed_pack, &subject) {
        let capability = special_profile_watch_capability(profile);
        if matches!(capability.kind, WatchSourceCapabilityKind::CanRefreshNow) {
            let _ = special_mod_versions::load_or_refresh_latest_info(connection, profile, true)?;
        }
        return resolve_watch_result(connection, seed_pack, &subject);
    }

    let Some(source) = load_watch_source_row(connection, &subject.key)? else {
        return resolve_watch_result(connection, seed_pack, &subject);
    };

    let capability = watch_source_capability(seed_pack, Some(&subject), &source);
    let result = match capability.kind {
        WatchSourceCapabilityKind::CanRefreshNow => {
            let latest = special_mod_versions::fetch_supported_watch_latest_from_url(
                &source.source_url,
                source.source_label.as_deref(),
            )?;
            build_generic_watch_result_row(&subject, latest, capability.note)
        }
        WatchSourceCapabilityKind::SavedReferenceOnly
        | WatchSourceCapabilityKind::ProviderRequired => WatchResultRow {
            status: WatchStatus::Unknown,
            latest_version: None,
            checked_at: Some(chrono::Utc::now().to_rfc3339()),
            confidence: VersionConfidence::Unknown,
            note: Some(capability.note),
            evidence: Vec::new(),
        },
    };
    save_watch_result_for_subject(connection, &subject.key, &result)?;

    resolve_watch_result(connection, seed_pack, &subject)
}

pub fn load_watch_counts(connection: &Connection, silent_special_mod_updates: bool) -> AppResult<(i64, i64, i64)> {
    let exact_generic = scalar(
        connection,
        "SELECT COUNT(*) FROM content_watch_results WHERE status = 'exact_update_available'",
    )?;
    let possible_generic = scalar(
        connection,
        "SELECT COUNT(*) FROM content_watch_results WHERE status = 'possible_update'",
    )?;
    let unknown_generic = scalar(
        connection,
        "SELECT COUNT(*) FROM content_watch_results WHERE status = 'unknown'",
    )?;

    let (exact_special, unknown_special) = if silent_special_mod_updates {
        (0, 0)
    } else {
        let exact_special = scalar(
            connection,
            "SELECT COUNT(*)
             FROM special_mod_family_state
             WHERE installed_version IS NOT NULL
               AND latest_version IS NOT NULL
               AND latest_status = 'known'
               AND latest_version <> installed_version",
        )?;
        let unknown_special = scalar(
            connection,
            "SELECT COUNT(*)
             FROM special_mod_family_state
             WHERE installed_version IS NOT NULL
               AND (latest_status = 'unknown' OR latest_status = '')",
        )?;
        (exact_special, unknown_special)
    };

    Ok((
        exact_generic + exact_special,
        possible_generic,
        unknown_generic + unknown_special,
    ))
}

pub fn list_auto_refreshable_watch_file_ids(
    connection: &Connection,
    seed_pack: &SeedPack,
) -> AppResult<Vec<i64>> {
    let mut statement = connection.prepare(
        "SELECT DISTINCT anchor_file_id, source_kind, source_label, source_url
         FROM content_watch_sources
         WHERE approved_by_user = 1
           AND anchor_file_id IS NOT NULL
         ORDER BY updated_at DESC",
    )?;

    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                WatchSourceRow {
                    source_kind: parse_watch_source_kind(&row.get::<_, String>(1)?),
                    source_label: row.get(2)?,
                    source_url: row.get(3)?,
                },
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(rows
        .into_iter()
        .filter(|(_, source)| {
            matches!(
                watch_source_capability(seed_pack, None, source).kind,
                WatchSourceCapabilityKind::CanRefreshNow
            )
        })
        .map(|(file_id, _)| file_id)
        .collect())
}

pub fn list_auto_refreshable_special_profile_keys(
    connection: &Connection,
    seed_pack: &SeedPack,
) -> AppResult<Vec<String>> {
    let mut statement = connection.prepare(
        "SELECT profile_key
         FROM special_mod_family_state
         WHERE install_state <> 'not_installed'
           AND (installed_version IS NOT NULL OR install_path IS NOT NULL)
         ORDER BY updated_at DESC",
    )?;

    let keys = statement
        .query_map([], |row| row.get::<_, String>(0))?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(keys
        .into_iter()
        .filter(|key| {
            seed_pack
                .install_catalog
                .guided_profiles
                .iter()
                .find(|profile| profile.key == *key)
                .is_some_and(|profile| {
                    matches!(
                        special_profile_watch_capability(profile).kind,
                        WatchSourceCapabilityKind::CanRefreshNow
                    )
                })
        })
        .collect())
}

fn load_saved_watch_anchor_file_ids(connection: &Connection) -> AppResult<Vec<i64>> {
    let mut statement = connection.prepare(
        "SELECT DISTINCT cws.anchor_file_id
         FROM content_watch_sources cws
         JOIN files f ON f.id = cws.anchor_file_id
         WHERE cws.approved_by_user = 1
           AND cws.anchor_file_id IS NOT NULL
           AND f.source_location <> 'downloads'
         ORDER BY cws.updated_at DESC",
    )?;

    let rows = statement
        .query_map([], |row| row.get::<_, i64>(0))?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(rows)
}

fn load_watch_setup_candidate_file_ids(connection: &Connection) -> AppResult<Vec<i64>> {
    let mut statement = connection.prepare(
        "SELECT
            f.id,
            COALESCE(f.confidence, 0),
            f.creator_id,
            f.insights
         FROM files f
         WHERE f.source_location <> 'downloads'
           AND f.kind NOT LIKE 'Tray%'
           AND LOWER(LTRIM(COALESCE(f.extension, ''), '.')) IN ('package', 'ts4script')
         ORDER BY
           CASE WHEN f.kind = 'ScriptMods' THEN 0 ELSE 1 END,
           CASE WHEN f.creator_id IS NOT NULL THEN 0 ELSE 1 END,
           CASE
             WHEN COALESCE(f.confidence, 0) >= 0.95 THEN 0
             WHEN COALESCE(f.confidence, 0) >= 0.8 THEN 1
             ELSE 2
           END,
           COALESCE(f.modified_at, '') DESC,
           f.filename COLLATE NOCASE
         LIMIT ?1",
    )?;

    let rows = statement
        .query_map(params![WATCH_SETUP_QUERY_LIMIT as i64], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, f64>(1)?,
                row.get::<_, Option<i64>>(2)?.is_some(),
                parse_file_insights(row.get(3)?),
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;

    let mut file_ids = Vec::new();
    for (file_id, confidence, has_creator, insights) in rows {
        if !file_has_watch_setup_seed_clues(has_creator, confidence, &insights) {
            continue;
        }

        file_ids.push(file_id);
        if file_ids.len() >= WATCH_SETUP_SCAN_LIMIT {
            break;
        }
    }

    Ok(file_ids)
}

fn file_has_watch_setup_seed_clues(
    has_creator: bool,
    confidence: f64,
    insights: &FileInsights,
) -> bool {
    has_creator
        || confidence >= SIGNAL_CONFLICT_CONFIDENCE
        || !insights.creator_hints.is_empty()
        || !insights.script_namespaces.is_empty()
        || !insights.embedded_names.is_empty()
        || !insights.family_hints.is_empty()
        || !insights.version_hints.is_empty()
        || insights
            .version_signals
            .iter()
            .any(|signal| signal.confidence >= SIGNAL_CONFLICT_CONFIDENCE)
}

fn load_supported_special_watch_anchor_file_ids(
    connection: &Connection,
    settings: &LibrarySettings,
    seed_pack: &SeedPack,
) -> AppResult<Vec<i64>> {
    let mut statement = connection.prepare(
        "SELECT profile_key, install_path
         FROM special_mod_family_state
         WHERE install_state <> 'not_installed'
           AND (installed_version IS NOT NULL OR install_path IS NOT NULL)
         ORDER BY updated_at DESC",
    )?;

    let rows = statement
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?))
        })?
        .collect::<Result<Vec<_>, _>>()?;

    let mut seen = HashSet::new();
    let mut seen_profiles = HashSet::new();
    let mut anchor_ids = Vec::new();
    for (profile_key, install_path) in rows {
        let Some(anchor_id) = find_watch_anchor_file_id_for_special_profile(
            connection,
            settings,
            seed_pack,
            &profile_key,
            install_path.as_deref(),
        )?
        else {
            continue;
        };

        if seen.insert(anchor_id) {
            anchor_ids.push(anchor_id);
        }
        seen_profiles.insert(profile_key);
    }

    let mut statement = connection.prepare(
        "SELECT id
         FROM files
         WHERE source_location <> 'downloads'
           AND (
                kind = 'ScriptMods'
                OR LOWER(LTRIM(COALESCE(extension, ''), '.')) IN ('ts4script', 'package')
           )
         ORDER BY filename COLLATE NOCASE",
    )?;
    let candidate_ids = statement
        .query_map([], |row| row.get::<_, i64>(0))?
        .collect::<Result<Vec<_>, _>>()?;

    let mut seen_locators = BTreeSet::new();
    for file_id in candidate_ids {
        let Some(file_row) = load_library_subject_row(connection, file_id)? else {
            continue;
        };

        let locator = subject_locator_for_row(&file_row, settings.mods_path.as_deref());
        if !seen_locators.insert(locator.clone()) {
            continue;
        }

        let subject_rows = load_subject_rows_for_locator(
            connection,
            settings.mods_path.as_deref(),
            &locator,
            file_id,
        )?;
        let subject = build_subject(subject_rows, locator, settings.mods_path.as_deref());
        let Some(profile) = find_supported_profile(seed_pack, &subject) else {
            continue;
        };

        if seen_profiles.insert(profile.key.clone()) && seen.insert(file_id) {
            anchor_ids.push(file_id);
        }
    }

    Ok(anchor_ids)
}

fn find_watch_anchor_file_id_for_special_profile(
    connection: &Connection,
    settings: &LibrarySettings,
    seed_pack: &SeedPack,
    profile_key: &str,
    install_path: Option<&str>,
) -> AppResult<Option<i64>> {
    if let Some(install_path) = install_path {
        let direct_match = connection
            .query_row(
                "SELECT id
                 FROM files
                 WHERE source_location <> 'downloads'
                   AND (path = ?1 OR path LIKE ?2)
                 ORDER BY CASE WHEN path = ?1 THEN 0 ELSE 1 END,
                          relative_depth ASC,
                          filename COLLATE NOCASE
                 LIMIT 1",
                params![install_path, format!("{install_path}%")],
                |row| row.get::<_, i64>(0),
            )
            .optional()?;

        if direct_match.is_some() {
            return Ok(direct_match);
        }
    }

    let mut statement = connection.prepare(
        "SELECT id
         FROM files
         WHERE source_location <> 'downloads'
           AND (
                kind = 'ScriptMods'
                OR LOWER(LTRIM(COALESCE(extension, ''), '.')) IN ('ts4script', 'package')
           )
         ORDER BY filename COLLATE NOCASE",
    )?;
    let candidate_ids = statement
        .query_map([], |row| row.get::<_, i64>(0))?
        .collect::<Result<Vec<_>, _>>()?;

    let mut seen_locators = BTreeSet::new();
    for file_id in candidate_ids {
        let Some(file_row) = load_library_subject_row(connection, file_id)? else {
            continue;
        };

        let locator = subject_locator_for_row(&file_row, settings.mods_path.as_deref());
        if !seen_locators.insert(locator.clone()) {
            continue;
        }

        let subject_rows = load_subject_rows_for_locator(
            connection,
            settings.mods_path.as_deref(),
            &locator,
            file_id,
        )?;
        let subject = build_subject(subject_rows, locator, settings.mods_path.as_deref());
        if find_supported_profile(seed_pack, &subject)
            .is_some_and(|profile| profile.key == profile_key)
        {
            return Ok(Some(file_id));
        }
    }

    Ok(None)
}

fn build_library_watch_setup_item(
    connection: &Connection,
    settings: &LibrarySettings,
    seed_pack: &SeedPack,
    file_id: i64,
    seen_subjects: &mut HashSet<String>,
) -> AppResult<Option<LibraryWatchSetupItem>> {
    let Some(file_row) = load_library_subject_row(connection, file_id)? else {
        return Ok(None);
    };

    let locator = subject_locator_for_row(&file_row, settings.mods_path.as_deref());
    let subject_rows = load_subject_rows_for_locator(
        connection,
        settings.mods_path.as_deref(),
        &locator,
        file_id,
    )?;
    let subject = build_subject(subject_rows, locator, settings.mods_path.as_deref());

    if !seen_subjects.insert(subject.key.clone()) {
        return Ok(None);
    }

    if find_supported_profile(seed_pack, &subject).is_some() {
        return Ok(None);
    }

    let watch_result = resolve_watch_result(connection, seed_pack, &subject)?;
    if watch_result
        .as_ref()
        .is_some_and(|result| result.source_origin != WatchSourceOrigin::None)
    {
        return Ok(None);
    }

    let Some((suggested_source_kind, setup_hint)) = watch_setup_suggestion_for_subject(&subject)
    else {
        return Ok(None);
    };

    Ok(Some(LibraryWatchSetupItem {
        file_id,
        filename: file_row.filename,
        creator: file_row.creator,
        subject_label: subject.label,
        installed_version: subject.version.value,
        suggested_source_kind,
        setup_hint,
    }))
}

fn build_library_watch_list_item(
    connection: &Connection,
    settings: &LibrarySettings,
    seed_pack: &SeedPack,
    file_id: i64,
) -> AppResult<Option<LibraryWatchListItem>> {
    let Some(file_row) = load_library_subject_row(connection, file_id)? else {
        return Ok(None);
    };

    let locator = subject_locator_for_row(&file_row, settings.mods_path.as_deref());
    let subject_rows = load_subject_rows_for_locator(
        connection,
        settings.mods_path.as_deref(),
        &locator,
        file_id,
    )?;
    let subject = build_subject(subject_rows, locator, settings.mods_path.as_deref());
    let Some(watch_result) = resolve_watch_result(connection, seed_pack, &subject)? else {
        return Ok(None);
    };

    if watch_result.source_origin == WatchSourceOrigin::None {
        return Ok(None);
    }

    Ok(Some(LibraryWatchListItem {
        file_id,
        filename: file_row.filename,
        creator: file_row.creator,
        subject_label: subject.label,
        installed_version: subject.version.value,
        watch_result,
    }))
}

fn build_library_watch_review_item(
    connection: &Connection,
    settings: &LibrarySettings,
    seed_pack: &SeedPack,
    file_id: i64,
) -> AppResult<Option<LibraryWatchReviewItem>> {
    let Some(list_item) = build_library_watch_list_item(connection, settings, seed_pack, file_id)?
    else {
        return Ok(None);
    };

    let Some((review_reason, review_hint)) = watch_review_reason_and_hint(&list_item.watch_result)
    else {
        return Ok(None);
    };

    Ok(Some(LibraryWatchReviewItem {
        file_id: list_item.file_id,
        filename: list_item.filename,
        creator: list_item.creator,
        subject_label: list_item.subject_label,
        installed_version: list_item.installed_version,
        watch_result: list_item.watch_result,
        review_reason,
        review_hint,
    }))
}

fn watch_setup_suggestion_for_subject(
    subject: &VersionSubject,
) -> Option<(WatchSourceKind, String)> {
    let has_creator = !subject.creator_tokens.is_empty();
    let has_trusted_version = subject.version.value.is_some()
        && matches!(
            subject.version.confidence,
            VersionConfidence::Exact | VersionConfidence::Strong | VersionConfidence::Medium
        );
    let has_script = subject
        .files
        .iter()
        .any(|file| file.filename.to_lowercase().ends_with(".ts4script"))
        || !subject.namespace_tokens.is_empty();
    let has_family = !subject.family_tokens.is_empty();
    let has_name_clue =
        has_family || !subject.embedded_tokens.is_empty() || subject.filename_tokens.len() >= 2;

    if has_creator && has_trusted_version && has_name_clue {
        return Some((
            WatchSourceKind::ExactPage,
            "Has creator and version clues, so an exact mod page should work well here.".to_owned(),
        ));
    }

    if has_script && has_creator && has_name_clue {
        return Some((
            WatchSourceKind::ExactPage,
            "Has script and creator clues, so an exact mod page is the safest next step."
                .to_owned(),
        ));
    }

    if has_script && has_trusted_version && has_family {
        return Some((
            WatchSourceKind::ExactPage,
            "Has script and version clues, so an exact mod page should be the cleanest fit."
                .to_owned(),
        ));
    }

    if has_creator && (has_family || has_script || has_trusted_version) {
        return Some((
            WatchSourceKind::CreatorPage,
            "Has strong creator clues, so a creator page is a reasonable reminder if no exact page is handy."
                .to_owned(),
        ));
    }

    if has_trusted_version && has_family && has_name_clue {
        return Some((
            WatchSourceKind::ExactPage,
            "Has version and family clues, so an exact mod page is worth setting up here."
                .to_owned(),
        ));
    }

    None
}

fn watch_review_reason_and_hint(
    watch_result: &WatchResult,
) -> Option<(LibraryWatchReviewReason, String)> {
    if watch_result.source_origin != WatchSourceOrigin::SavedByUser {
        return None;
    }

    if watch_result.capability == WatchCapability::ProviderRequired {
        let provider_label = watch_result
            .provider_name
            .clone()
            .unwrap_or_else(|| "A provider".to_owned());
        return Some((
            LibraryWatchReviewReason::ProviderNeeded,
            format!(
                "{provider_label} support is still needed before SimSuite can check this saved page automatically."
            ),
        ));
    }

    if watch_result.capability == WatchCapability::SavedReferenceOnly {
        let hint = match watch_result.source_kind {
            Some(WatchSourceKind::CreatorPage) => {
                "This creator page is saved as a reminder only. Keep it if it helps, or replace it with an exact mod page."
                    .to_owned()
            }
            Some(WatchSourceKind::ExactPage) => {
                "This saved page is still reference-only. Review whether it should stay a reminder or be replaced with a safer exact page."
                    .to_owned()
            }
            None => "This saved watch source still needs a manual decision.".to_owned(),
        };

        return Some((LibraryWatchReviewReason::ReferenceOnly, hint));
    }

    if watch_result.status == WatchStatus::Unknown {
        return Some((
            LibraryWatchReviewReason::UnknownResult,
            "SimSuite checked this source, but the result is still unclear. Review the page or replace the link."
                .to_owned(),
        ));
    }

    None
}

fn watch_result_matches_filter(watch_result: &WatchResult, filter: WatchListFilter) -> bool {
    match filter {
        WatchListFilter::Attention => matches!(
            watch_result.status,
            WatchStatus::ExactUpdateAvailable | WatchStatus::PossibleUpdate | WatchStatus::Unknown
        ),
        WatchListFilter::ExactUpdates => watch_result.status == WatchStatus::ExactUpdateAvailable,
        WatchListFilter::PossibleUpdates => watch_result.status == WatchStatus::PossibleUpdate,
        WatchListFilter::Unclear => watch_result.status == WatchStatus::Unknown,
        WatchListFilter::All => true,
    }
}

fn compare_library_watch_items(
    left: &LibraryWatchListItem,
    right: &LibraryWatchListItem,
) -> std::cmp::Ordering {
    watch_status_priority(&left.watch_result.status)
        .cmp(&watch_status_priority(&right.watch_result.status))
        .then_with(|| {
            left.subject_label
                .to_lowercase()
                .cmp(&right.subject_label.to_lowercase())
        })
        .then_with(|| {
            left.filename
                .to_lowercase()
                .cmp(&right.filename.to_lowercase())
        })
}

fn compare_library_watch_review_items(
    left: &LibraryWatchReviewItem,
    right: &LibraryWatchReviewItem,
) -> std::cmp::Ordering {
    watch_review_priority(&left.review_reason)
        .cmp(&watch_review_priority(&right.review_reason))
        .then_with(|| {
            left.subject_label
                .to_lowercase()
                .cmp(&right.subject_label.to_lowercase())
        })
        .then_with(|| {
            left.filename
                .to_lowercase()
                .cmp(&right.filename.to_lowercase())
        })
}

fn compare_library_watch_setup_items(
    left: &LibraryWatchSetupItem,
    right: &LibraryWatchSetupItem,
) -> std::cmp::Ordering {
    watch_setup_priority(right)
        .cmp(&watch_setup_priority(left))
        .then_with(|| {
            left.subject_label
                .to_lowercase()
                .cmp(&right.subject_label.to_lowercase())
        })
        .then_with(|| {
            left.filename
                .to_lowercase()
                .cmp(&right.filename.to_lowercase())
        })
}

fn watch_setup_priority(item: &LibraryWatchSetupItem) -> i32 {
    let mut priority = 0;

    if item.suggested_source_kind == WatchSourceKind::ExactPage {
        priority += 2;
    }
    if item
        .installed_version
        .as_deref()
        .is_some_and(|value| !value.trim().is_empty())
    {
        priority += 2;
    }
    if item
        .creator
        .as_deref()
        .is_some_and(|value| !value.trim().is_empty())
    {
        priority += 1;
    }

    priority
}

fn watch_review_priority(reason: &LibraryWatchReviewReason) -> i32 {
    match reason {
        LibraryWatchReviewReason::ProviderNeeded => 0,
        LibraryWatchReviewReason::ReferenceOnly => 1,
        LibraryWatchReviewReason::UnknownResult => 2,
    }
}

fn watch_status_priority(status: &WatchStatus) -> usize {
    match status {
        WatchStatus::ExactUpdateAvailable => 0,
        WatchStatus::PossibleUpdate => 1,
        WatchStatus::Unknown => 2,
        WatchStatus::NotWatched => 3,
        WatchStatus::Current => 4,
    }
}

fn save_watch_result_for_subject(
    connection: &Connection,
    subject_key: &str,
    result: &WatchResultRow,
) -> AppResult<()> {
    connection.execute(
        "INSERT INTO content_watch_results (
            subject_key,
            status,
            latest_version,
            checked_at,
            confidence,
            note,
            evidence,
            updated_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, CURRENT_TIMESTAMP)
         ON CONFLICT(subject_key) DO UPDATE SET
            status = excluded.status,
            latest_version = excluded.latest_version,
            checked_at = excluded.checked_at,
            confidence = excluded.confidence,
            note = excluded.note,
            evidence = excluded.evidence,
            updated_at = CURRENT_TIMESTAMP",
        params![
            subject_key,
            watch_status_label(&result.status),
            result.latest_version,
            result.checked_at,
            version_confidence_label(&result.confidence),
            result.note,
            serde_json::to_string(&result.evidence)?,
        ],
    )?;
    Ok(())
}

fn scalar(connection: &Connection, sql: &str) -> AppResult<i64> {
    Ok(connection.query_row(sql, [], |row| row.get(0))?)
}

pub fn save_watch_source_for_subject(
    connection: &Connection,
    subject_key: &str,
    anchor_file_id: Option<i64>,
    source_kind: WatchSourceKind,
    source_label: Option<String>,
    source_url: &str,
) -> AppResult<()> {
    connection.execute(
        "INSERT INTO content_watch_sources (
            subject_key,
            anchor_file_id,
            source_kind,
            source_label,
            source_url,
            approved_by_user,
            updated_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, 1, CURRENT_TIMESTAMP)
        ON CONFLICT(subject_key) DO UPDATE SET
            anchor_file_id = excluded.anchor_file_id,
            source_kind = excluded.source_kind,
            source_label = excluded.source_label,
            source_url = excluded.source_url,
            approved_by_user = excluded.approved_by_user,
            updated_at = CURRENT_TIMESTAMP",
        params![
            subject_key,
            anchor_file_id,
            match source_kind {
                WatchSourceKind::ExactPage => "exact_page",
                WatchSourceKind::CreatorPage => "creator_page",
            },
            source_label,
            source_url,
        ],
    )?;
    Ok(())
}

pub fn clear_watch_source_for_subject(connection: &Connection, subject_key: &str) -> AppResult<()> {
    connection.execute(
        "DELETE FROM content_watch_sources WHERE subject_key = ?1",
        params![subject_key],
    )?;
    Ok(())
}

fn parse_file_insights(value: String) -> FileInsights {
    serde_json::from_str(&value).unwrap_or_default()
}

fn parse_string_array(value: Option<String>) -> Vec<String> {
    value
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

fn load_download_subject_rows(
    connection: &Connection,
    item_id: i64,
) -> AppResult<Vec<SubjectFileRow>> {
    let mut statement = connection.prepare(
        "SELECT
            f.id,
            f.filename,
            f.path,
            f.hash,
            f.size,
            c.canonical_name,
            f.source_location,
            f.insights
         FROM files f
         LEFT JOIN creators c ON c.id = f.creator_id
         WHERE f.download_item_id = ?1
           AND f.source_location = 'downloads'
         ORDER BY f.filename COLLATE NOCASE",
    )?;

    let rows = statement
        .query_map(params![item_id], |row| {
            Ok(SubjectFileRow {
                id: row.get(0)?,
                filename: row.get(1)?,
                path: row.get(2)?,
                hash: row.get(3)?,
                size: row.get(4)?,
                creator: row.get(5)?,
                source_location: row.get(6)?,
                insights: parse_file_insights(row.get(7)?),
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(rows)
}

fn load_library_subject_row(
    connection: &Connection,
    file_id: i64,
) -> AppResult<Option<SubjectFileRow>> {
    connection
        .query_row(
            "SELECT
                f.id,
                f.filename,
                f.path,
                f.hash,
                f.size,
                c.canonical_name,
                f.source_location,
                f.insights
             FROM files f
             LEFT JOIN creators c ON c.id = f.creator_id
             WHERE f.id = ?1
               AND f.source_location <> 'downloads'",
            params![file_id],
            |row| {
                Ok(SubjectFileRow {
                    id: row.get(0)?,
                    filename: row.get(1)?,
                    path: row.get(2)?,
                    hash: row.get(3)?,
                    size: row.get(4)?,
                    creator: row.get(5)?,
                    source_location: row.get(6)?,
                    insights: parse_file_insights(row.get(7)?),
                })
            },
        )
        .optional()
        .map_err(Into::into)
}

fn load_subject_rows_for_locator(
    connection: &Connection,
    mods_path: Option<&str>,
    locator: &SubjectLocator,
    fallback_file_id: i64,
) -> AppResult<Vec<SubjectFileRow>> {
    let rows = match locator {
        SubjectLocator::DownloadItem(item_id) => load_download_subject_rows(connection, *item_id),
        SubjectLocator::Folder(folder) => load_subject_rows_by_sql(
            connection,
            "SELECT
                f.id,
                f.filename,
                f.path,
                f.hash,
                f.size,
                c.canonical_name,
                f.source_location,
                f.insights
             FROM files f
             LEFT JOIN creators c ON c.id = f.creator_id
             WHERE f.source_location <> 'downloads'
               AND f.path LIKE ?1
             ORDER BY f.filename COLLATE NOCASE",
            vec![Value::Text(format!("{folder}\\%"))],
        ),
        SubjectLocator::Family(family_key) => {
            load_subject_rows_by_family_hint(connection, family_key)
        }
        SubjectLocator::FileStem(stem) => load_subject_rows_by_sql(
            connection,
            "SELECT
                f.id,
                f.filename,
                f.path,
                f.hash,
                f.size,
                c.canonical_name,
                f.source_location,
                f.insights
             FROM files f
             LEFT JOIN creators c ON c.id = f.creator_id
             WHERE f.source_location <> 'downloads'
               AND LOWER(f.filename) LIKE ?1
             ORDER BY f.filename COLLATE NOCASE",
            vec![Value::Text(format!("{stem}%"))],
        ),
    }?;

    if !rows.is_empty() {
        return Ok(rows);
    }

    let _ = mods_path;
    load_subject_rows_by_file_id(connection, fallback_file_id)
}

fn load_subject_rows_by_sql(
    connection: &Connection,
    sql: &str,
    values: Vec<Value>,
) -> AppResult<Vec<SubjectFileRow>> {
    let mut statement = connection.prepare(sql)?;
    let rows = statement
        .query_map(params_from_iter(values.iter()), |row| {
            Ok(SubjectFileRow {
                id: row.get(0)?,
                filename: row.get(1)?,
                path: row.get(2)?,
                hash: row.get(3)?,
                size: row.get(4)?,
                creator: row.get(5)?,
                source_location: row.get(6)?,
                insights: parse_file_insights(row.get(7)?),
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

fn load_subject_rows_by_file_id(
    connection: &Connection,
    file_id: i64,
) -> AppResult<Vec<SubjectFileRow>> {
    let Some(row) = load_library_subject_row(connection, file_id)? else {
        return Ok(Vec::new());
    };
    Ok(vec![row])
}

fn load_subject_rows_by_family_hint(
    connection: &Connection,
    family_key: &str,
) -> AppResult<Vec<SubjectFileRow>> {
    let mut statement = connection.prepare(
        "SELECT
            f.id,
            f.filename,
            f.path,
            f.hash,
            f.size,
            c.canonical_name,
            f.source_location,
            f.insights
         FROM files f
         LEFT JOIN creators c ON c.id = f.creator_id
         WHERE f.source_location <> 'downloads'
           AND LOWER(f.insights) LIKE ?1
         ORDER BY f.filename COLLATE NOCASE
         LIMIT ?2",
    )?;
    let needle = format!("%{}%", family_key.replace('\"', ""));
    let rows = statement
        .query_map(params![needle, MAX_CANDIDATE_ROWS as i64], |row| {
            Ok(SubjectFileRow {
                id: row.get(0)?,
                filename: row.get(1)?,
                path: row.get(2)?,
                hash: row.get(3)?,
                size: row.get(4)?,
                creator: row.get(5)?,
                source_location: row.get(6)?,
                insights: parse_file_insights(row.get(7)?),
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(rows
        .into_iter()
        .filter(|row| {
            row.insights
                .family_hints
                .iter()
                .any(|hint| normalize_token(hint) == *family_key)
        })
        .collect())
}

fn load_installed_rows_by_family_hints(
    connection: &Connection,
    family_hints: &[String],
) -> AppResult<Vec<SubjectFileRow>> {
    if family_hints.is_empty() {
        return Ok(Vec::new());
    }

    let mut rows_by_id = HashMap::<i64, SubjectFileRow>::new();
    for family_hint in family_hints {
        for row in load_subject_rows_by_family_hint(connection, family_hint)? {
            rows_by_id.entry(row.id).or_insert(row);
        }
    }

    Ok(rows_by_id.into_values().collect())
}

fn collect_candidate_family_hints(subject: &VersionSubject) -> Vec<String> {
    let mut family_hints = subject
        .files
        .iter()
        .flat_map(|file| file.insights.family_hints.iter())
        .map(|hint| normalize_token(hint))
        .filter(|hint| hint.len() >= MIN_FAMILY_HINT_SEARCH_LEN)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    family_hints.sort_by(|left, right| right.len().cmp(&left.len()).then_with(|| left.cmp(right)));
    family_hints.truncate(MAX_SEARCH_FAMILY_HINTS);
    family_hints
}

fn load_installed_candidate_rows(
    connection: &Connection,
    incoming_subject: &VersionSubject,
    detail_level: CompareDetailLevel,
) -> AppResult<Vec<SubjectFileRow>> {
    let mut rows_by_id = HashMap::<i64, SubjectFileRow>::new();
    let exact_hashes = incoming_subject
        .files
        .iter()
        .filter_map(|file| file.hash.clone())
        .filter(|hash| !hash.trim().is_empty())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let filenames = incoming_subject
        .files
        .iter()
        .map(|file| file.filename.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();

    for row in load_installed_rows_by_hashes(connection, &exact_hashes)? {
        rows_by_id.insert(row.id, row);
    }
    for row in load_installed_rows_by_filenames(connection, &filenames)? {
        rows_by_id.entry(row.id).or_insert(row);
    }

    if detail_level == CompareDetailLevel::Full {
        let creators = incoming_subject
            .files
            .iter()
            .flat_map(|file| {
                file.creator
                    .iter()
                    .cloned()
                    .chain(file.insights.creator_hints.iter().cloned())
                    .collect::<Vec<_>>()
            })
            .filter(|value| !value.trim().is_empty())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        for row in load_installed_rows_by_creators(connection, &creators)? {
            rows_by_id.entry(row.id).or_insert(row);
        }

        let family_hints = collect_candidate_family_hints(incoming_subject);
        for row in load_installed_rows_by_family_hints(connection, &family_hints)? {
            rows_by_id.entry(row.id).or_insert(row);
        }

        let tokens = incoming_subject
            .filename_tokens
            .iter()
            .filter(|token| token.len() >= 4)
            .take(MAX_SEARCH_TOKENS)
            .cloned()
            .collect::<Vec<_>>();
        for row in load_installed_rows_by_filename_tokens(connection, &tokens)? {
            rows_by_id.entry(row.id).or_insert(row);
        }
    }

    Ok(rows_by_id.into_values().collect())
}

fn load_installed_rows_by_hashes(
    connection: &Connection,
    hashes: &[String],
) -> AppResult<Vec<SubjectFileRow>> {
    if hashes.is_empty() {
        return Ok(Vec::new());
    }

    let placeholders = (0..hashes.len())
        .map(|index| format!("?{}", index + 1))
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!(
        "SELECT
            f.id,
            f.filename,
            f.path,
            f.hash,
            f.size,
            c.canonical_name,
            f.source_location,
            f.insights
         FROM files f
         LEFT JOIN creators c ON c.id = f.creator_id
         WHERE f.source_location <> 'downloads'
           AND f.hash IN ({placeholders})
         LIMIT ?{}",
        hashes.len() + 1
    );
    let mut values = hashes.iter().cloned().map(Value::Text).collect::<Vec<_>>();
    values.push(Value::Integer(MAX_CANDIDATE_ROWS as i64));
    load_subject_rows_by_sql(connection, &sql, values)
}

fn load_installed_rows_by_filenames(
    connection: &Connection,
    filenames: &[String],
) -> AppResult<Vec<SubjectFileRow>> {
    if filenames.is_empty() {
        return Ok(Vec::new());
    }

    let placeholders = (0..filenames.len())
        .map(|index| format!("?{}", index + 1))
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!(
        "SELECT
            f.id,
            f.filename,
            f.path,
            f.hash,
            f.size,
            c.canonical_name,
            f.source_location,
            f.insights
         FROM files f
         LEFT JOIN creators c ON c.id = f.creator_id
         WHERE f.source_location <> 'downloads'
           AND f.filename IN ({placeholders})
         LIMIT ?{}",
        filenames.len() + 1
    );
    let mut values = filenames
        .iter()
        .cloned()
        .map(Value::Text)
        .collect::<Vec<_>>();
    values.push(Value::Integer(MAX_CANDIDATE_ROWS as i64));
    load_subject_rows_by_sql(connection, &sql, values)
}

fn load_installed_rows_by_creators(
    connection: &Connection,
    creators: &[String],
) -> AppResult<Vec<SubjectFileRow>> {
    if creators.is_empty() {
        return Ok(Vec::new());
    }

    let placeholders = (0..creators.len())
        .map(|index| format!("?{}", index + 1))
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!(
        "SELECT
            f.id,
            f.filename,
            f.path,
            f.hash,
            f.size,
            c.canonical_name,
            f.source_location,
            f.insights
         FROM files f
         LEFT JOIN creators c ON c.id = f.creator_id
         WHERE f.source_location <> 'downloads'
           AND c.canonical_name IN ({placeholders})
         LIMIT ?{}",
        creators.len() + 1
    );
    let mut values = creators
        .iter()
        .cloned()
        .map(Value::Text)
        .collect::<Vec<_>>();
    values.push(Value::Integer(MAX_CANDIDATE_ROWS as i64));
    load_subject_rows_by_sql(connection, &sql, values)
}

fn load_installed_rows_by_filename_tokens(
    connection: &Connection,
    tokens: &[String],
) -> AppResult<Vec<SubjectFileRow>> {
    if tokens.is_empty() {
        return Ok(Vec::new());
    }

    let conditions = (0..tokens.len())
        .map(|index| format!("LOWER(f.filename) LIKE ?{}", index + 1))
        .collect::<Vec<_>>()
        .join(" OR ");
    let sql = format!(
        "SELECT
            f.id,
            f.filename,
            f.path,
            f.hash,
            f.size,
            c.canonical_name,
            f.source_location,
            f.insights
         FROM files f
         LEFT JOIN creators c ON c.id = f.creator_id
         WHERE f.source_location <> 'downloads'
           AND ({conditions})
         LIMIT ?{}",
        tokens.len() + 1
    );
    let mut values = tokens
        .iter()
        .map(|token| Value::Text(format!("%{token}%")))
        .collect::<Vec<_>>();
    values.push(Value::Integer(MAX_CANDIDATE_ROWS as i64));
    load_subject_rows_by_sql(connection, &sql, values)
}

fn build_subject(
    rows: Vec<SubjectFileRow>,
    locator: SubjectLocator,
    mods_path: Option<&str>,
) -> VersionSubject {
    let label = subject_label(&rows, &locator);
    let key = subject_key(&rows, &locator, mods_path);
    let aggregate_signature = build_subject_signature(&rows);
    let all_hashes_present = rows
        .iter()
        .all(|file| file.hash.as_deref().is_some_and(|value| !value.is_empty()));

    let creator_tokens = rows
        .iter()
        .flat_map(|file| {
            file.creator
                .iter()
                .flat_map(|value| collect_tokens(value))
                .chain(
                    file.insights
                        .creator_hints
                        .iter()
                        .flat_map(|value| collect_tokens(value)),
                )
                .collect::<Vec<_>>()
        })
        .collect();
    let family_tokens = rows
        .iter()
        .flat_map(|file| {
            file.insights
                .family_hints
                .iter()
                .flat_map(|value| collect_tokens(value))
        })
        .collect();
    let namespace_tokens = rows
        .iter()
        .flat_map(|file| {
            file.insights
                .script_namespaces
                .iter()
                .flat_map(|value| collect_tokens(value))
        })
        .collect();
    let embedded_tokens = rows
        .iter()
        .flat_map(|file| {
            file.insights
                .embedded_names
                .iter()
                .flat_map(|value| collect_tokens(value))
        })
        .collect();
    let filename_tokens = rows
        .iter()
        .flat_map(|file| collect_tokens(&file.filename))
        .collect();
    let version = subject_version_from_rows(&rows);

    VersionSubject {
        key,
        label,
        aggregate_signature,
        all_hashes_present,
        creator_tokens,
        family_tokens,
        namespace_tokens,
        embedded_tokens,
        filename_tokens,
        version,
        files: rows,
    }
}

fn build_subject_signature(rows: &[SubjectFileRow]) -> Option<String> {
    let entries = rows
        .iter()
        .map(|file| SignatureEntry {
            filename: file.filename.clone(),
            size: file.size,
            hash: file.hash.clone(),
        })
        .collect::<Vec<_>>();
    build_signature(&entries)
}

fn subject_locator_for_row(row: &SubjectFileRow, mods_path: Option<&str>) -> SubjectLocator {
    if row.source_location == "downloads" {
        return SubjectLocator::FileStem(normalize_stem(&row.filename));
    }

    let file_path = Path::new(&row.path);
    if let Some(root) = mods_path.map(PathBuf::from) {
        if let Ok(relative) = file_path.strip_prefix(&root) {
            if let Some(parent) = relative.parent() {
                let parent_str = parent.to_string_lossy().trim().to_owned();
                if !parent_str.is_empty() && parent_str != "." {
                    return SubjectLocator::Folder(root.join(parent).to_string_lossy().to_string());
                }
            }
        }
    }

    if let Some(family) = row
        .insights
        .family_hints
        .iter()
        .map(|hint| normalize_token(hint))
        .find(|hint| !hint.is_empty())
    {
        return SubjectLocator::Family(family);
    }

    SubjectLocator::FileStem(normalize_stem(&row.filename))
}

fn subject_label(rows: &[SubjectFileRow], locator: &SubjectLocator) -> String {
    if let Some(label) = rows
        .iter()
        .flat_map(|file| file.insights.family_hints.iter())
        .find(|value| !value.trim().is_empty())
    {
        return label.trim().to_owned();
    }

    match locator {
        SubjectLocator::DownloadItem(_) => rows
            .first()
            .map(|file| prettify_stem(&file.filename))
            .unwrap_or_else(|| "Download".to_owned()),
        SubjectLocator::Folder(folder) => Path::new(folder)
            .file_name()
            .and_then(|value| value.to_str())
            .map(|value| value.to_owned())
            .unwrap_or_else(|| "Folder".to_owned()),
        SubjectLocator::Family(value) => prettify_token(value),
        SubjectLocator::FileStem(value) => prettify_token(value),
    }
}

fn subject_key(
    rows: &[SubjectFileRow],
    locator: &SubjectLocator,
    mods_path: Option<&str>,
) -> String {
    match locator {
        SubjectLocator::DownloadItem(item_id) => format!("download-item:{item_id}"),
        SubjectLocator::Folder(folder) => {
            if let Some(root) = mods_path.map(PathBuf::from) {
                let folder_path = PathBuf::from(folder);
                if let Ok(relative) = folder_path.strip_prefix(root) {
                    return format!("folder:{}", normalize_token(&relative.to_string_lossy()));
                }
            }
            format!("folder:{}", normalize_token(folder))
        }
        SubjectLocator::Family(family) => format!("family:{family}"),
        SubjectLocator::FileStem(stem) => {
            if let Some(first) = rows.first() {
                format!("file:{}:{}", stem, normalize_token(&first.path))
            } else {
                format!("file:{stem}")
            }
        }
    }
}

fn subject_version_from_rows(rows: &[SubjectFileRow]) -> SubjectVersion {
    let mut signals = rows
        .iter()
        .flat_map(|file| {
            file.insights
                .version_signals
                .iter()
                .cloned()
                .map(move |signal| {
                    (
                        signal,
                        file.filename.clone(),
                        file.insights.version_hints.clone(),
                    )
                })
        })
        .collect::<Vec<_>>();
    signals.sort_by(|left, right| compare_version_signals(&left.0, &right.0));

    let Some((top_signal, top_filename, _)) = signals.first().cloned() else {
        return SubjectVersion::default();
    };

    let conflicting = signals.iter().skip(1).find(|(signal, _, _)| {
        signal.normalized_value != top_signal.normalized_value
            && signal.confidence >= SIGNAL_CONFLICT_CONFIDENCE
            && top_signal.confidence >= SIGNAL_CONFLICT_CONFIDENCE
            && source_priority(&signal.source_kind) <= source_priority(&top_signal.source_kind) + 1
    });

    let mut evidence = Vec::new();
    evidence.push(format!(
        "{} hinted {} from {}.",
        top_filename,
        top_signal.normalized_value,
        human_source_kind(&top_signal.source_kind)
    ));

    if let Some((signal, filename, _)) = conflicting {
        evidence.push(format!(
            "{} also hinted {}, so SimSuite is staying cautious.",
            filename, signal.normalized_value
        ));
        return SubjectVersion {
            value: None,
            confidence: VersionConfidence::Unknown,
            evidence,
            ambiguous: true,
        };
    }

    if signals
        .iter()
        .skip(1)
        .any(|(signal, _, _)| signal.normalized_value == top_signal.normalized_value)
    {
        evidence.push(format!(
            "Other local clues also pointed to {}.",
            top_signal.normalized_value
        ));
    }

    SubjectVersion {
        value: Some(top_signal.normalized_value),
        confidence: confidence_from_signal(top_signal.confidence),
        evidence,
        ambiguous: false,
    }
}

fn compare_version_signals(left: &VersionSignal, right: &VersionSignal) -> std::cmp::Ordering {
    source_priority(&left.source_kind)
        .cmp(&source_priority(&right.source_kind))
        .then_with(|| right.confidence.total_cmp(&left.confidence))
        .then_with(|| {
            version_part_length(&right.normalized_value)
                .cmp(&version_part_length(&left.normalized_value))
        })
}

fn version_part_length(value: &str) -> usize {
    parse_version_parts(value)
        .map(|parts| parts.len())
        .unwrap_or_default()
}

fn source_priority(source_kind: &str) -> usize {
    GENERIC_SOURCE_ORDER
        .iter()
        .position(|candidate| *candidate == source_kind)
        .unwrap_or(GENERIC_SOURCE_ORDER.len())
}

fn confidence_from_signal(value: f64) -> VersionConfidence {
    if value >= 0.90 {
        VersionConfidence::Strong
    } else if value >= 0.72 {
        VersionConfidence::Medium
    } else if value >= 0.48 {
        VersionConfidence::Weak
    } else {
        VersionConfidence::Unknown
    }
}

fn human_source_kind(source_kind: &str) -> &'static str {
    match source_kind {
        "payload" => "inside the file",
        "embedded_name" => "an embedded name",
        "filename" => "the file name",
        "archive_path" => "the archive path",
        "resource_summary" => "a resource summary",
        _ => "local clues",
    }
}

fn resolve_against_installed_subjects(
    incoming_subject: VersionSubject,
    installed_rows: Vec<SubjectFileRow>,
    mods_path: Option<&str>,
    forced_locator: Option<SubjectLocator>,
    seed_pack: &SeedPack,
) -> VersionResolution {
    let mut grouped = BTreeMap::<SubjectLocator, Vec<SubjectFileRow>>::new();
    for row in installed_rows {
        let locator = forced_locator
            .clone()
            .unwrap_or_else(|| subject_locator_for_row(&row, mods_path));
        grouped.entry(locator).or_default().push(row);
    }

    let installed_subjects = grouped
        .into_iter()
        .map(|(locator, rows)| build_subject(rows, locator, mods_path))
        .collect::<Vec<_>>();

    let mut best_match: Option<(VersionSubject, MatchBreakdown)> = None;
    for subject in installed_subjects {
        let breakdown = score_subject_match(&incoming_subject, &subject);
        let should_replace = best_match
            .as_ref()
            .map(|(_, current)| breakdown.score > current.score)
            .unwrap_or(true);
        if should_replace {
            best_match = Some((subject, breakdown));
        }
    }

    build_resolution(incoming_subject, best_match, seed_pack)
}

fn build_resolution(
    incoming_subject: VersionSubject,
    best_match: Option<(VersionSubject, MatchBreakdown)>,
    seed_pack: &SeedPack,
) -> VersionResolution {
    let mut resolution = VersionResolution {
        subject_label: Some(incoming_subject.label.clone()),
        incoming_version: incoming_subject.version.value.clone(),
        incoming_signature: incoming_subject.aggregate_signature.clone(),
        incoming_evidence: incoming_subject.version.evidence.clone(),
        ..VersionResolution::default()
    };

    let incoming_identity_score = incoming_identity_score(&incoming_subject);

    let Some((installed_subject, breakdown)) = best_match else {
        if incoming_identity_score >= MATCH_SCORE_MEDIUM {
            resolution.status = VersionCompareStatus::NotInstalled;
            resolution.confidence = confidence_from_match_score(incoming_identity_score);
            resolution.evidence = vec![
                "SimSuite could not find an installed copy that matched this download.".to_owned(),
            ];
        } else {
            resolution.status = VersionCompareStatus::Unknown;
            resolution.confidence = VersionConfidence::Unknown;
            resolution.evidence = vec![
                "SimSuite could not match this download to an installed mod confidently yet."
                    .to_owned(),
            ];
        }
        return resolution;
    };

    resolution.matched_subject_label = Some(installed_subject.label.clone());
    resolution.matched_subject_key = Some(installed_subject.key.clone());
    resolution.installed_version = installed_subject.version.value.clone();
    resolution.installed_signature = installed_subject.aggregate_signature.clone();
    resolution.installed_evidence = installed_subject.version.evidence.clone();
    resolution.match_score = breakdown.score;

    if installed_subject.all_hashes_present
        && incoming_subject.all_hashes_present
        && incoming_subject.aggregate_signature.is_some()
        && incoming_subject.aggregate_signature == installed_subject.aggregate_signature
    {
        resolution.status = VersionCompareStatus::SameVersion;
        resolution.confidence = VersionConfidence::Exact;
        resolution.evidence = breakdown.evidence;
        resolution.evidence.push(
            "The installed copy and the download have the same local file fingerprint.".to_owned(),
        );
        return resolution;
    }

    let match_confidence = confidence_from_match_score(breakdown.score);
    if match_confidence == VersionConfidence::Weak || match_confidence == VersionConfidence::Unknown
    {
        resolution.status = VersionCompareStatus::Unknown;
        resolution.confidence = VersionConfidence::Unknown;
        resolution.evidence = breakdown.evidence;
        resolution
            .evidence
            .push("SimSuite found a possible installed match, but the local clues were too weak to trust.".to_owned());
        return resolution;
    }

    if incoming_subject.version.ambiguous || installed_subject.version.ambiguous {
        resolution.status = VersionCompareStatus::Unknown;
        resolution.confidence = VersionConfidence::Unknown;
        resolution.evidence = breakdown.evidence;
        resolution.evidence.push(
            "The version clues on one side disagreed, so SimSuite stayed cautious.".to_owned(),
        );
        return resolution;
    }

    resolution.evidence = breakdown.evidence;

    match (
        incoming_subject.version.value.as_deref(),
        installed_subject.version.value.as_deref(),
    ) {
        (Some(incoming), Some(installed)) => {
            if incoming == installed {
                if signatures_disagree(
                    incoming_subject.aggregate_signature.as_deref(),
                    incoming_subject.all_hashes_present,
                    installed_subject.aggregate_signature.as_deref(),
                    installed_subject.all_hashes_present,
                ) {
                    resolution.status = VersionCompareStatus::Unknown;
                    resolution.confidence = VersionConfidence::Unknown;
                    resolution.evidence.push(
                        "The version labels match, but the local file fingerprints do not."
                            .to_owned(),
                    );
                } else {
                    resolution.status = VersionCompareStatus::SameVersion;
                    resolution.confidence = combine_confidence(
                        match_confidence,
                        installed_subject.version.confidence.clone(),
                    );
                    resolution
                        .evidence
                        .push(format!("Both local copies point to version {incoming}."));
                }
            } else if let (Some(incoming_parts), Some(installed_parts)) = (
                parse_version_parts(incoming),
                parse_version_parts(installed),
            ) {
                match incoming_parts.cmp(&installed_parts) {
                    std::cmp::Ordering::Greater => {
                        resolution.status = VersionCompareStatus::IncomingNewer;
                        resolution.confidence = combine_confidence(
                            match_confidence,
                            incoming_subject.version.confidence.clone(),
                        );
                        resolution.evidence.push(format!(
                            "The download points to {incoming}, and the installed copy points to {installed}."
                        ));
                    }
                    std::cmp::Ordering::Equal => {
                        resolution.status = VersionCompareStatus::SameVersion;
                        resolution.confidence = combine_confidence(
                            match_confidence,
                            incoming_subject.version.confidence.clone(),
                        );
                        resolution.evidence.push(format!(
                            "Both local copies normalize to version {incoming}."
                        ));
                    }
                    std::cmp::Ordering::Less => {
                        resolution.status = VersionCompareStatus::IncomingOlder;
                        resolution.confidence = combine_confidence(
                            match_confidence,
                            installed_subject.version.confidence.clone(),
                        );
                        resolution.evidence.push(format!(
                            "The download points to {incoming}, but the installed copy points to {installed}."
                        ));
                    }
                }
            } else {
                resolution.status = VersionCompareStatus::Unknown;
                resolution.confidence = VersionConfidence::Unknown;
                resolution.evidence.push(
                    "SimSuite found both versions, but could not compare their numbering safely."
                        .to_owned(),
                );
            }
        }
        _ => {
            resolution.status = VersionCompareStatus::Unknown;
            resolution.confidence = VersionConfidence::Unknown;
            resolution
                .evidence
                .push("SimSuite found the installed copy, but could not read a clear version on both sides.".to_owned());
        }
    }

    if let Some(profile) = find_supported_profile(seed_pack, &installed_subject) {
        if resolution.subject_label.is_none() {
            resolution.subject_label = Some(profile.display_name.clone());
        }
    }

    resolution
}

fn score_subject_match(incoming: &VersionSubject, installed: &VersionSubject) -> MatchBreakdown {
    let mut breakdown = MatchBreakdown::default();

    let shared_hashes = shared_hash_count(
        &incoming
            .files
            .iter()
            .filter_map(|file| file.hash.as_deref())
            .map(str::to_owned)
            .collect::<HashSet<_>>(),
        &installed
            .files
            .iter()
            .filter_map(|file| file.hash.as_deref())
            .map(str::to_owned)
            .collect::<HashSet<_>>(),
    );
    if shared_hashes > 0 {
        breakdown.score += 1.0;
        breakdown
            .evidence
            .push(format!("{} file hash match(es) lined up.", shared_hashes));
    }

    breakdown.score += overlap_score(
        &incoming.creator_tokens,
        &installed.creator_tokens,
        0.36,
        "creator",
        &mut breakdown.evidence,
    );
    breakdown.score += overlap_score(
        &incoming.family_tokens,
        &installed.family_tokens,
        0.62,
        "family",
        &mut breakdown.evidence,
    );
    breakdown.score += overlap_score(
        &incoming.namespace_tokens,
        &installed.namespace_tokens,
        0.42,
        "script namespace",
        &mut breakdown.evidence,
    );
    breakdown.score += overlap_score(
        &incoming.embedded_tokens,
        &installed.embedded_tokens,
        0.24,
        "embedded name",
        &mut breakdown.evidence,
    );
    breakdown.score += overlap_score(
        &incoming.filename_tokens,
        &installed.filename_tokens,
        0.34,
        "file name",
        &mut breakdown.evidence,
    );

    if incoming.label.eq_ignore_ascii_case(&installed.label) {
        breakdown.score += 0.18;
        breakdown
            .evidence
            .push("The local labels also line up.".to_owned());
    }

    breakdown
}

fn shared_hash_count(left: &HashSet<String>, right: &HashSet<String>) -> usize {
    left.intersection(right).count()
}

fn overlap_score(
    left: &BTreeSet<String>,
    right: &BTreeSet<String>,
    weight: f64,
    label: &str,
    evidence: &mut Vec<String>,
) -> f64 {
    let overlap = left
        .intersection(right)
        .take(3)
        .cloned()
        .collect::<Vec<_>>();
    if overlap.is_empty() {
        return 0.0;
    }

    evidence.push(format!(
        "Shared {} clue{}: {}.",
        label,
        if overlap.len() == 1 { "" } else { "s" },
        overlap.join(", ")
    ));
    (overlap.len() as f64).min(2.0) * weight / 2.0
}

fn incoming_identity_score(subject: &VersionSubject) -> f64 {
    let mut score = 0.0;
    if !subject.creator_tokens.is_empty() {
        score += 0.25;
    }
    if !subject.family_tokens.is_empty() {
        score += 0.40;
    }
    if !subject.namespace_tokens.is_empty() {
        score += 0.28;
    }
    if !subject.filename_tokens.is_empty() {
        score += 0.24;
    }
    if subject.version.value.is_some()
        && matches!(
            subject.version.confidence,
            VersionConfidence::Exact | VersionConfidence::Strong | VersionConfidence::Medium
        )
    {
        score += 0.20;
    }
    score
}

fn confidence_from_match_score(score: f64) -> VersionConfidence {
    if score >= 2.0 {
        VersionConfidence::Exact
    } else if score >= MATCH_SCORE_STRONG {
        VersionConfidence::Strong
    } else if score >= MATCH_SCORE_MEDIUM {
        VersionConfidence::Medium
    } else if score >= MATCH_SCORE_WEAK {
        VersionConfidence::Weak
    } else {
        VersionConfidence::Unknown
    }
}

fn combine_confidence(left: VersionConfidence, right: VersionConfidence) -> VersionConfidence {
    use VersionConfidence::{Exact, Medium, Strong, Unknown, Weak};

    match (left, right) {
        (Strong, Strong) => Strong,
        (Exact, Exact) => Exact,
        (Exact, Strong) | (Strong, Exact) => Strong,
        (Unknown, _) | (_, Unknown) => Unknown,
        (Weak, _) | (_, Weak) => Weak,
        (Medium, _) | (_, Medium) => Medium,
    }
}

fn signatures_disagree(
    left: Option<&str>,
    left_ready: bool,
    right: Option<&str>,
    right_ready: bool,
) -> bool {
    left_ready && right_ready && left.is_some() && right.is_some() && left != right
}

fn resolve_watch_result(
    connection: &Connection,
    seed_pack: &SeedPack,
    subject: &VersionSubject,
) -> AppResult<Option<WatchResult>> {
    if let Some(profile) = find_supported_profile(seed_pack, subject) {
        let capability = special_profile_watch_capability(profile);
        let special = connection
            .query_row(
                "SELECT
                    latest_source_url,
                    latest_version,
                    latest_checked_at,
                    latest_status,
                    latest_note,
                    latest_confidence
                 FROM special_mod_family_state
                 WHERE profile_key = ?1",
                params![profile.key.as_str()],
                |row| {
                    Ok((
                        row.get::<_, Option<String>>(0)?,
                        row.get::<_, Option<String>>(1)?,
                        row.get::<_, Option<String>>(2)?,
                        row.get::<_, Option<String>>(3)?,
                        row.get::<_, Option<String>>(4)?,
                        row.get::<_, Option<f64>>(5)?.unwrap_or_default(),
                    ))
                },
            )
            .optional()?;

        if let Some((
            source_url,
            latest_version,
            checked_at,
            latest_status,
            latest_note,
            confidence,
        )) = special
        {
            let installed_version = subject.version.value.clone();
            let status = if latest_status.as_deref() == Some("known") {
                match (installed_version.as_deref(), latest_version.as_deref()) {
                    (Some(installed), Some(latest)) if installed == latest => WatchStatus::Current,
                    (Some(_), Some(_)) => WatchStatus::ExactUpdateAvailable,
                    _ => WatchStatus::Unknown,
                }
            } else {
                WatchStatus::Unknown
            };

            let mut evidence = Vec::new();
            if let Some(version) = latest_version.as_deref() {
                evidence.push(format!("Official helper check last saw version {version}."));
            }
            if let Some(note) = latest_note.as_deref() {
                evidence.push(note.to_owned());
            }

            return Ok(Some(WatchResult {
                status,
                source_kind: Some(WatchSourceKind::ExactPage),
                source_origin: WatchSourceOrigin::BuiltInSpecial,
                source_label: Some(profile.display_name.clone()),
                source_url: source_url.or_else(|| {
                    profile
                        .latest_check_url
                        .clone()
                        .or_else(|| Some(profile.official_source_url.clone()))
                }),
                capability: map_watch_capability_kind(capability.kind),
                can_refresh_now: matches!(
                    capability.kind,
                    WatchSourceCapabilityKind::CanRefreshNow
                ),
                provider_name: capability.provider_name.clone(),
                latest_version,
                checked_at,
                confidence: confidence_from_signal(confidence),
                note: latest_note,
                evidence,
            }));
        }

        return Ok(Some(WatchResult {
            status: WatchStatus::NotWatched,
            source_kind: Some(WatchSourceKind::ExactPage),
            source_origin: WatchSourceOrigin::BuiltInSpecial,
            source_label: Some(profile.display_name.clone()),
            source_url: profile
                .latest_check_url
                .clone()
                .or_else(|| Some(profile.official_source_url.clone())),
            capability: map_watch_capability_kind(capability.kind),
            can_refresh_now: matches!(capability.kind, WatchSourceCapabilityKind::CanRefreshNow),
            provider_name: capability.provider_name.clone(),
            latest_version: None,
            checked_at: None,
            confidence: VersionConfidence::Unknown,
            note: Some(capability.note),
            evidence: Vec::new(),
        }));
    }

    let source = load_watch_source_row(connection, &subject.key)?;
    let result = load_watch_result_row(connection, &subject.key)?;
    match (source, result) {
        (Some(source), Some(result)) => {
            let capability = watch_source_capability(seed_pack, Some(subject), &source);
            Ok(Some(WatchResult {
                status: result.status,
                source_kind: Some(source.source_kind),
                source_origin: WatchSourceOrigin::SavedByUser,
                source_label: source.source_label,
                source_url: Some(source.source_url),
                capability: map_watch_capability_kind(capability.kind),
                can_refresh_now: matches!(
                    capability.kind,
                    WatchSourceCapabilityKind::CanRefreshNow
                ),
                provider_name: capability.provider_name.clone(),
                latest_version: result.latest_version,
                checked_at: result.checked_at,
                confidence: result.confidence,
                note: result.note,
                evidence: result.evidence,
            }))
        }
        (Some(source), None) => {
            let capability = watch_source_capability(seed_pack, Some(subject), &source);
            Ok(Some(WatchResult {
                status: WatchStatus::NotWatched,
                source_kind: Some(source.source_kind),
                source_origin: WatchSourceOrigin::SavedByUser,
                source_label: source.source_label,
                source_url: Some(source.source_url),
                capability: map_watch_capability_kind(capability.kind),
                can_refresh_now: matches!(
                    capability.kind,
                    WatchSourceCapabilityKind::CanRefreshNow
                ),
                provider_name: capability.provider_name.clone(),
                latest_version: None,
                checked_at: None,
                confidence: VersionConfidence::Unknown,
                note: Some(capability.note),
                evidence: Vec::new(),
            }))
        }
        (None, Some(result)) => Ok(Some(WatchResult {
            status: result.status,
            source_kind: None,
            source_origin: WatchSourceOrigin::None,
            source_label: None,
            source_url: None,
            capability: WatchCapability::SavedReferenceOnly,
            can_refresh_now: false,
            provider_name: None,
            latest_version: result.latest_version,
            checked_at: result.checked_at,
            confidence: result.confidence,
            note: result.note,
            evidence: result.evidence,
        })),
        (None, None) => Ok(Some(WatchResult {
            status: WatchStatus::NotWatched,
            source_kind: None,
            source_origin: WatchSourceOrigin::None,
            source_label: None,
            source_url: None,
            capability: WatchCapability::SavedReferenceOnly,
            can_refresh_now: false,
            provider_name: None,
            latest_version: None,
            checked_at: None,
            confidence: VersionConfidence::Unknown,
            note: Some(
                "No approved watch source is saved for this installed content yet.".to_owned(),
            ),
            evidence: Vec::new(),
        })),
    }
}

fn special_profile_watch_capability(profile: &GuidedInstallProfileSeed) -> WatchSourceCapability {
    match profile.latest_check_strategy.as_deref().unwrap_or("manual") {
        "mccc_downloads_page" | "xml_injector_page" | "github_releases" => WatchSourceCapability {
            kind: WatchSourceCapabilityKind::CanRefreshNow,
            note: "SimSuite can check this official page now when you press Check now.".to_owned(),
            provider_name: None,
        },
        "protected_page" => {
            if url_host_matches(
                profile
                    .latest_check_url
                    .as_deref()
                    .or(Some(profile.official_source_url.as_str())),
                "curseforge.com",
            ) {
                WatchSourceCapability {
                    kind: WatchSourceCapabilityKind::ProviderRequired,
                    note:
                        "This page is saved, but CurseForge checks need a future approved API path."
                            .to_owned(),
                    provider_name: Some("CurseForge".to_owned()),
                }
            } else {
                WatchSourceCapability {
                    kind: WatchSourceCapabilityKind::SavedReferenceOnly,
                    note:
                        "This page is saved, but this site blocks safe automatic checks right now."
                            .to_owned(),
                    provider_name: None,
                }
            }
        }
        _ => WatchSourceCapability {
            kind: WatchSourceCapabilityKind::SavedReferenceOnly,
            note:
                "This page is saved as a reference, but SimSuite cannot check it automatically yet."
                    .to_owned(),
            provider_name: None,
        },
    }
}

fn watch_source_capability(
    seed_pack: &SeedPack,
    subject: Option<&VersionSubject>,
    source: &WatchSourceRow,
) -> WatchSourceCapability {
    if let (Some(subject), WatchSourceKind::ExactPage) = (subject, source.source_kind.clone()) {
        if let Some(profile) = find_supported_profile(seed_pack, subject) {
            if watch_url_matches_profile(&source.source_url, profile) {
                return special_profile_watch_capability(profile);
            }
        }
    }

    match source.source_kind {
        WatchSourceKind::CreatorPage => WatchSourceCapability {
            kind: WatchSourceCapabilityKind::SavedReferenceOnly,
            note: "Creator pages are saved as reminders for now. Automatic creator-page checks are not built yet."
                .to_owned(),
            provider_name: None,
        },
        WatchSourceKind::ExactPage => {
            if is_supported_generic_github_release_url(&source.source_url) {
                return WatchSourceCapability {
                    kind: WatchSourceCapabilityKind::CanRefreshNow,
                    note: "SimSuite can check this GitHub releases page now when you press Check now."
                        .to_owned(),
                    provider_name: None,
                };
            }

            if url_host_matches(Some(source.source_url.as_str()), "curseforge.com") {
                return WatchSourceCapability {
                    kind: WatchSourceCapabilityKind::ProviderRequired,
                    note: "This page is saved, but CurseForge checks need a future approved API path."
                        .to_owned(),
                    provider_name: Some("CurseForge".to_owned()),
                };
            }

            if url_host_matches(Some(source.source_url.as_str()), "lot51.cc") {
                return WatchSourceCapability {
                    kind: WatchSourceCapabilityKind::SavedReferenceOnly,
                    note: "This page is saved, but this site blocks safe automatic checks right now."
                        .to_owned(),
                    provider_name: None,
                };
            }

            WatchSourceCapability {
                kind: WatchSourceCapabilityKind::SavedReferenceOnly,
                note: "This page is saved as a reference, but SimSuite cannot check it automatically yet."
                    .to_owned(),
                provider_name: None,
            }
        }
    }
}

fn map_watch_capability_kind(kind: WatchSourceCapabilityKind) -> WatchCapability {
    match kind {
        WatchSourceCapabilityKind::CanRefreshNow => WatchCapability::CanRefreshNow,
        WatchSourceCapabilityKind::SavedReferenceOnly => WatchCapability::SavedReferenceOnly,
        WatchSourceCapabilityKind::ProviderRequired => WatchCapability::ProviderRequired,
    }
}

fn build_generic_watch_result_row(
    subject: &VersionSubject,
    latest: crate::models::SpecialOfficialLatestInfo,
    fallback_note: String,
) -> WatchResultRow {
    let mut evidence = Vec::new();
    if let Some(version) = latest.latest_version.as_deref() {
        evidence.push(format!("Saved watch page last saw version {version}."));
    }
    if let Some(note) = latest.note.as_deref() {
        evidence.push(note.to_owned());
    }

    let mut note = latest.note.clone().or(Some(fallback_note));
    let status = if latest.status == "known" {
        match compare_versions(
            subject.version.value.is_some(),
            subject.version.value.as_deref(),
            None,
            latest.latest_version.as_deref(),
            None,
        ) {
            SpecialVersionStatus::SameVersion => WatchStatus::Current,
            SpecialVersionStatus::IncomingNewer => WatchStatus::ExactUpdateAvailable,
            SpecialVersionStatus::IncomingOlder => {
                note = Some(
                    "The saved page looks older than what is installed, so SimSuite is staying cautious."
                        .to_owned(),
                );
                WatchStatus::Unknown
            }
            _ => WatchStatus::Unknown,
        }
    } else {
        WatchStatus::Unknown
    };

    WatchResultRow {
        status,
        latest_version: latest.latest_version,
        checked_at: latest
            .checked_at
            .or_else(|| Some(chrono::Utc::now().to_rfc3339())),
        confidence: confidence_from_signal(latest.confidence),
        note,
        evidence,
    }
}

fn watch_url_matches_profile(source_url: &str, profile: &GuidedInstallProfileSeed) -> bool {
    normalized_url(source_url) == normalized_url(&profile.official_source_url)
        || profile
            .latest_check_url
            .as_deref()
            .is_some_and(|value| normalized_url(source_url) == normalized_url(value))
}

fn normalized_url(value: &str) -> String {
    value.trim().trim_end_matches('/').to_ascii_lowercase()
}

fn url_host_matches(value: Option<&str>, expected_host: &str) -> bool {
    value
        .and_then(|raw| reqwest::Url::parse(raw).ok())
        .and_then(|url| url.host_str().map(str::to_ascii_lowercase))
        .is_some_and(|host| host == expected_host || host == format!("www.{expected_host}"))
}

fn is_supported_generic_github_release_url(value: &str) -> bool {
    let Ok(url) = reqwest::Url::parse(value) else {
        return false;
    };
    let host = url.host_str().unwrap_or_default().to_ascii_lowercase();
    if host != "github.com" && host != "www.github.com" {
        return false;
    }
    let path = url.path().to_ascii_lowercase();
    path.contains("/releases")
}

fn load_watch_source_row(
    connection: &Connection,
    subject_key: &str,
) -> AppResult<Option<WatchSourceRow>> {
    connection
        .query_row(
            "SELECT source_kind, source_label, source_url
             FROM content_watch_sources
             WHERE subject_key = ?1",
            params![subject_key],
            |row| {
                Ok(WatchSourceRow {
                    source_kind: parse_watch_source_kind(&row.get::<_, String>(0)?),
                    source_label: row.get(1)?,
                    source_url: row.get(2)?,
                })
            },
        )
        .optional()
        .map_err(Into::into)
}

fn load_watch_result_row(
    connection: &Connection,
    subject_key: &str,
) -> AppResult<Option<WatchResultRow>> {
    connection
        .query_row(
            "SELECT status, latest_version, checked_at, confidence, note, evidence
             FROM content_watch_results
             WHERE subject_key = ?1",
            params![subject_key],
            |row| {
                Ok(WatchResultRow {
                    status: parse_watch_status(&row.get::<_, String>(0)?),
                    latest_version: row.get(1)?,
                    checked_at: row.get(2)?,
                    confidence: parse_version_confidence(&row.get::<_, String>(3)?),
                    note: row.get(4)?,
                    evidence: parse_string_array(row.get(5)?),
                })
            },
        )
        .optional()
        .map_err(Into::into)
}

fn find_supported_profile<'a>(
    seed_pack: &'a SeedPack,
    subject: &VersionSubject,
) -> Option<&'a GuidedInstallProfileSeed> {
    seed_pack
        .install_catalog
        .guided_profiles
        .iter()
        .map(|profile| {
            let mut score = 0.0;
            let subject_tokens = subject
                .family_tokens
                .iter()
                .chain(subject.filename_tokens.iter())
                .cloned()
                .collect::<BTreeSet<_>>();

            if profile
                .required_name_clues
                .iter()
                .map(|value| normalize_token(value))
                .any(|token| subject_tokens.contains(&token))
            {
                score += 0.9;
            }
            score += profile
                .name_clues
                .iter()
                .map(|value| normalize_token(value))
                .filter(|token| subject_tokens.contains(token))
                .count() as f64
                * 0.18;
            score += profile
                .script_prefixes
                .iter()
                .map(|value| normalize_token(value))
                .filter(|token| {
                    subject
                        .filename_tokens
                        .iter()
                        .any(|value| value.starts_with(token))
                })
                .count() as f64
                * 0.14;
            score += profile
                .sample_filenames
                .iter()
                .map(|value| normalize_stem(value))
                .filter(|token| subject.filename_tokens.contains(token))
                .count() as f64
                * 0.22;
            (profile, score)
        })
        .filter(|(_, score)| *score >= 0.9)
        .max_by(|left, right| left.1.total_cmp(&right.1))
        .map(|(profile, _)| profile)
}

fn parse_watch_status(value: &str) -> WatchStatus {
    match value {
        "current" => WatchStatus::Current,
        "exact_update_available" => WatchStatus::ExactUpdateAvailable,
        "possible_update" => WatchStatus::PossibleUpdate,
        "unknown" => WatchStatus::Unknown,
        _ => WatchStatus::NotWatched,
    }
}

fn watch_status_label(value: &WatchStatus) -> &'static str {
    match value {
        WatchStatus::NotWatched => "not_watched",
        WatchStatus::Current => "current",
        WatchStatus::ExactUpdateAvailable => "exact_update_available",
        WatchStatus::PossibleUpdate => "possible_update",
        WatchStatus::Unknown => "unknown",
    }
}

fn parse_watch_source_kind(value: &str) -> WatchSourceKind {
    match value {
        "creator_page" => WatchSourceKind::CreatorPage,
        _ => WatchSourceKind::ExactPage,
    }
}

fn parse_version_confidence(value: &str) -> VersionConfidence {
    match value {
        "exact" => VersionConfidence::Exact,
        "strong" => VersionConfidence::Strong,
        "medium" => VersionConfidence::Medium,
        "weak" => VersionConfidence::Weak,
        _ => VersionConfidence::Unknown,
    }
}

fn version_confidence_label(value: &VersionConfidence) -> &'static str {
    match value {
        VersionConfidence::Exact => "exact",
        VersionConfidence::Strong => "strong",
        VersionConfidence::Medium => "medium",
        VersionConfidence::Weak => "weak",
        VersionConfidence::Unknown => "unknown",
    }
}

fn normalize_stem(value: &str) -> String {
    let stem = value
        .rsplit_once('.')
        .map(|(left, _)| left)
        .unwrap_or(value);
    normalize_token(stem)
}

fn normalize_token(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                ' '
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn collect_tokens(value: &str) -> BTreeSet<String> {
    normalize_token(value)
        .split_whitespace()
        .filter(|token| token.len() >= 3 && !TOKEN_STOP_WORDS.contains(token))
        .map(|token| token.to_owned())
        .collect()
}

fn prettify_token(value: &str) -> String {
    value
        .split_whitespace()
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => format!("{}{}", first.to_ascii_uppercase(), chars.as_str()),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn prettify_stem(value: &str) -> String {
    prettify_token(&normalize_stem(value))
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::{
        clear_watch_source_for_library_file, list_auto_refreshable_watch_file_ids,
        list_library_watch_items, list_library_watch_review_items, list_library_watch_setup_items,
        refresh_watch_source_for_library_file, resolve_download_item_version,
        save_watch_source_for_library_file, CompareDetailLevel,
    };
    use crate::{
        database,
        models::{
            FileInsights, LibrarySettings, LibraryWatchReviewReason, VersionCompareStatus,
            VersionConfidence, VersionSignal, WatchCapability, WatchListFilter, WatchSourceKind,
            WatchSourceOrigin, WatchStatus,
        },
        seed::load_seed_pack,
    };
    use rusqlite::{params, Connection};

    fn setup_watch_env() -> (Connection, crate::seed::SeedPack, LibrarySettings, i64) {
        let mut connection = Connection::open_in_memory().expect("in-memory db");
        database::initialize(&mut connection).expect("schema");
        let seed_pack = load_seed_pack().expect("seed pack");
        database::seed_database(&mut connection, &seed_pack).expect("seed db");

        let settings = LibrarySettings {
            mods_path: Some("C:/Users/Test/Documents/Electronic Arts/The Sims 4/Mods".to_owned()),
            tray_path: None,
            downloads_path: Some("C:/Users/Test/Downloads".to_owned()),
            ..Default::default()
        };
        let insights = FileInsights {
            creator_hints: vec!["TestCreator".to_owned()],
            version_hints: vec!["1.0".to_owned()],
            version_signals: vec![VersionSignal {
                raw_value: "1.0".to_owned(),
                normalized_value: "1.0".to_owned(),
                source_kind: "filename".to_owned(),
                source_path: None,
                matched_by: Some("filename pattern".to_owned()),
                confidence: 0.84,
            }],
            ..FileInsights::default()
        };

        connection
            .execute(
                "INSERT INTO files (
                    path,
                    filename,
                    extension,
                    kind,
                    confidence,
                    source_location,
                    parser_warnings,
                    insights
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    "C:/Users/Test/Documents/Electronic Arts/The Sims 4/Mods/TestCreator/watch_test_mod_v1.0.package",
                    "watch_test_mod_v1.0.package",
                    ".package",
                    "Gameplay",
                    0.94_f64,
                    "mods",
                    "[]",
                    serde_json::to_string(&insights).expect("insights json"),
                ],
            )
            .expect("insert file");
        let file_id = connection.last_insert_rowid();

        (connection, seed_pack, settings, file_id)
    }

    fn insert_compare_download_item(connection: &Connection, item_id: i64, display_name: &str) {
        connection
            .execute(
                "INSERT INTO download_items (
                    id, source_path, display_name, source_kind, status
                 ) VALUES (?1, ?2, ?3, 'file', 'ready')",
                params![
                    item_id,
                    format!("C:/Users/Test/Downloads/{display_name}"),
                    display_name
                ],
            )
            .expect("insert download item");
    }

    fn insert_compare_file(
        connection: &Connection,
        path: &str,
        filename: &str,
        source_location: &str,
        creator_id: Option<i64>,
        download_item_id: Option<i64>,
        insights: &FileInsights,
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
                    creator_id,
                    download_item_id,
                    parser_warnings,
                    insights
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    path,
                    filename,
                    ".package",
                    "Gameplay",
                    0.86_f64,
                    source_location,
                    creator_id,
                    download_item_id,
                    "[]",
                    serde_json::to_string(insights).expect("insights json"),
                ],
            )
            .expect("insert compare file");

        connection.last_insert_rowid()
    }

    #[test]
    fn supported_library_profile_exposes_check_now_when_source_is_safe() {
        let mut connection = Connection::open_in_memory().expect("in-memory db");
        database::initialize(&mut connection).expect("schema");
        let seed_pack = load_seed_pack().expect("seed pack");
        database::seed_database(&mut connection, &seed_pack).expect("seed db");

        let settings = LibrarySettings {
            mods_path: Some("C:/Users/Test/Documents/Electronic Arts/The Sims 4/Mods".to_owned()),
            tray_path: None,
            downloads_path: Some("C:/Users/Test/Downloads".to_owned()),
            ..Default::default()
        };
        let insights = FileInsights {
            family_hints: vec!["s4cl".to_owned()],
            version_hints: vec!["2.9.0".to_owned()],
            version_signals: vec![VersionSignal {
                raw_value: "2.9.0".to_owned(),
                normalized_value: "2.9.0".to_owned(),
                source_kind: "filename".to_owned(),
                source_path: None,
                matched_by: Some("fixture".to_owned()),
                confidence: 0.88,
            }],
            ..FileInsights::default()
        };

        connection
            .execute(
                "INSERT INTO files (
                    path,
                    filename,
                    extension,
                    kind,
                    confidence,
                    source_location,
                    parser_warnings,
                    insights
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    "C:/Users/Test/Documents/Electronic Arts/The Sims 4/Mods/S4CL/S4CL.ts4script",
                    "S4CL.ts4script",
                    ".ts4script",
                    "ScriptMods",
                    0.97_f64,
                    "mods",
                    "[]",
                    serde_json::to_string(&insights).expect("insights json"),
                ],
            )
            .expect("insert file");
        let file_id = connection.last_insert_rowid();

        let (_, watch_result) =
            super::resolve_library_file_version(&connection, &settings, &seed_pack, file_id)
                .expect("resolve version");
        let watch_result = watch_result.expect("watch result");

        assert_eq!(watch_result.source_kind, Some(WatchSourceKind::ExactPage));
        assert_eq!(
            watch_result.source_origin,
            WatchSourceOrigin::BuiltInSpecial
        );
        assert_eq!(watch_result.capability, WatchCapability::CanRefreshNow);
        assert!(watch_result.can_refresh_now);
        assert!(watch_result
            .note
            .as_deref()
            .is_some_and(|note| note.contains("Check now")));
    }

    #[test]
    fn supported_library_profile_can_match_from_filename_without_family_hint() {
        let mut connection = Connection::open_in_memory().expect("in-memory db");
        database::initialize(&mut connection).expect("schema");
        let seed_pack = load_seed_pack().expect("seed pack");
        database::seed_database(&mut connection, &seed_pack).expect("seed db");

        let settings = LibrarySettings {
            mods_path: Some("C:/Users/Test/Documents/Electronic Arts/The Sims 4/Mods".to_owned()),
            tray_path: None,
            downloads_path: Some("C:/Users/Test/Downloads".to_owned()),
            ..Default::default()
        };
        let insights = FileInsights {
            family_hints: vec![
                "version".to_owned(),
                "version txt".to_owned(),
                "version.txt".to_owned(),
                "versiontxt".to_owned(),
            ],
            version_hints: vec!["2.9.0".to_owned()],
            version_signals: vec![VersionSignal {
                raw_value: "2.9.0".to_owned(),
                normalized_value: "2.9.0".to_owned(),
                source_kind: "filename".to_owned(),
                source_path: None,
                matched_by: Some("fixture".to_owned()),
                confidence: 0.88,
            }],
            script_namespaces: vec!["version.txt".to_owned()],
            ..FileInsights::default()
        };

        connection
            .execute(
                "INSERT INTO files (
                    path,
                    filename,
                    extension,
                    kind,
                    confidence,
                    source_location,
                    parser_warnings,
                    insights
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    "C:/Users/Test/Documents/Electronic Arts/The Sims 4/Mods/S4CL/S4CL.ts4script",
                    "S4CL.ts4script",
                    ".ts4script",
                    "ScriptMods",
                    0.97_f64,
                    "mods",
                    "[]",
                    serde_json::to_string(&insights).expect("insights json"),
                ],
            )
            .expect("insert file");
        let file_id = connection.last_insert_rowid();

        let (_, watch_result) =
            super::resolve_library_file_version(&connection, &settings, &seed_pack, file_id)
                .expect("resolve version");
        let watch_result = watch_result.expect("watch result");

        assert_eq!(watch_result.source_kind, Some(WatchSourceKind::ExactPage));
        assert_eq!(
            watch_result.source_origin,
            WatchSourceOrigin::BuiltInSpecial
        );
        assert_eq!(watch_result.capability, WatchCapability::CanRefreshNow);
        assert!(watch_result.can_refresh_now);
    }

    #[test]
    fn saving_generic_exact_watch_source_marks_reference_only_state() {
        let (connection, seed_pack, settings, file_id) = setup_watch_env();

        let watch = save_watch_source_for_library_file(
            &connection,
            &settings,
            &seed_pack,
            file_id,
            WatchSourceKind::ExactPage,
            Some("Test Watch".to_owned()),
            "https://example.com/mod-page",
        )
        .expect("save watch")
        .expect("watch result");

        assert_eq!(watch.status, WatchStatus::NotWatched);
        assert_eq!(watch.source_kind, Some(WatchSourceKind::ExactPage));
        assert_eq!(watch.source_origin, WatchSourceOrigin::SavedByUser);
        assert_eq!(watch.capability, WatchCapability::SavedReferenceOnly);
        assert_eq!(watch.provider_name, None);
        assert_eq!(watch.source_label.as_deref(), Some("Test Watch"));
        assert_eq!(
            watch.source_url.as_deref(),
            Some("https://example.com/mod-page")
        );
        assert!(!watch.can_refresh_now);
        assert!(watch
            .note
            .as_deref()
            .is_some_and(|note| note.contains("saved as a reference")));
    }

    #[test]
    fn saving_github_release_watch_source_allows_check_now() {
        let (connection, seed_pack, settings, file_id) = setup_watch_env();

        let watch = save_watch_source_for_library_file(
            &connection,
            &settings,
            &seed_pack,
            file_id,
            WatchSourceKind::ExactPage,
            Some("GitHub Release".to_owned()),
            "https://github.com/example/mod/releases",
        )
        .expect("save watch")
        .expect("watch result");

        assert_eq!(watch.status, WatchStatus::NotWatched);
        assert_eq!(watch.source_kind, Some(WatchSourceKind::ExactPage));
        assert_eq!(watch.source_origin, WatchSourceOrigin::SavedByUser);
        assert_eq!(watch.capability, WatchCapability::CanRefreshNow);
        assert!(watch.can_refresh_now);
        assert!(watch
            .note
            .as_deref()
            .is_some_and(|note| note.contains("Check now")));

        let refreshable =
            list_auto_refreshable_watch_file_ids(&connection, &seed_pack).expect("targets");
        assert_eq!(refreshable, vec![file_id]);
    }

    #[test]
    fn watch_list_returns_saved_watch_rows() {
        let (connection, seed_pack, settings, file_id) = setup_watch_env();

        save_watch_source_for_library_file(
            &connection,
            &settings,
            &seed_pack,
            file_id,
            WatchSourceKind::ExactPage,
            Some("GitHub Release".to_owned()),
            "https://github.com/example/mod/releases",
        )
        .expect("save watch");

        let response =
            list_library_watch_items(&connection, &settings, &seed_pack, WatchListFilter::All, 12)
                .expect("watch list");

        assert_eq!(response.total, 1);
        assert_eq!(response.items[0].file_id, file_id);
        assert_eq!(
            response.items[0].watch_result.source_origin,
            WatchSourceOrigin::SavedByUser
        );
    }

    #[test]
    fn watch_setup_list_returns_unwatched_candidates() {
        let (connection, seed_pack, settings, file_id) = setup_watch_env();

        let response = list_library_watch_setup_items(&connection, &settings, &seed_pack, 6)
            .expect("watch setup list");

        assert_eq!(response.total, 1);
        assert!(!response.truncated);
        assert_eq!(response.exact_page_total, 1);
        assert!(!response.exact_page_truncated);
        assert_eq!(response.exact_page_items.len(), 1);
        assert_eq!(response.items[0].file_id, file_id);
        assert_eq!(
            response.items[0].suggested_source_kind,
            WatchSourceKind::ExactPage
        );
        assert_eq!(response.exact_page_items[0].file_id, file_id);
    }

    #[test]
    fn watch_setup_candidate_scan_skips_default_empty_insights() {
        let (connection, _seed_pack, _settings, file_id) = setup_watch_env();

        connection
            .execute(
                "INSERT INTO files (
                    path,
                    filename,
                    extension,
                    kind,
                    confidence,
                    source_location,
                    parser_warnings,
                    insights
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    "C:/Users/Test/Documents/Electronic Arts/The Sims 4/Mods/Misc/plain_item.package",
                    "plain_item.package",
                    ".package",
                    "Gameplay",
                    0.22_f64,
                    "mods",
                    "[]",
                    serde_json::to_string(&FileInsights::default()).expect("insights json"),
                ],
            )
            .expect("insert default insight file");

        let candidate_ids =
            super::load_watch_setup_candidate_file_ids(&connection).expect("candidate file ids");

        assert_eq!(candidate_ids, vec![file_id]);
    }

    #[test]
    fn watch_setup_list_skips_weak_version_only_candidates() {
        let mut connection = Connection::open_in_memory().expect("in-memory db");
        database::initialize(&mut connection).expect("schema");
        let seed_pack = load_seed_pack().expect("seed pack");
        database::seed_database(&mut connection, &seed_pack).expect("seed db");

        let settings = LibrarySettings {
            mods_path: Some("C:/Users/Test/Documents/Electronic Arts/The Sims 4/Mods".to_owned()),
            tray_path: None,
            downloads_path: Some("C:/Users/Test/Downloads".to_owned()),
            ..Default::default()
        };
        let insights = FileInsights {
            version_hints: vec!["1.0".to_owned()],
            version_signals: vec![VersionSignal {
                raw_value: "1.0".to_owned(),
                normalized_value: "1.0".to_owned(),
                source_kind: "filename".to_owned(),
                source_path: None,
                matched_by: Some("filename pattern".to_owned()),
                confidence: 0.51,
            }],
            ..FileInsights::default()
        };

        connection
            .execute(
                "INSERT INTO files (
                    path,
                    filename,
                    extension,
                    kind,
                    confidence,
                    source_location,
                    parser_warnings,
                    insights
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    "C:/Users/Test/Documents/Electronic Arts/The Sims 4/Mods/Misc/weak_guess_v1.package",
                    "weak_guess_v1.package",
                    ".package",
                    "Gameplay",
                    0.41_f64,
                    "mods",
                    "[]",
                    serde_json::to_string(&insights).expect("insights json"),
                ],
            )
            .expect("insert file");

        let response = list_library_watch_setup_items(&connection, &settings, &seed_pack, 6)
            .expect("watch setup list");

        assert_eq!(response.total, 0);
        assert!(response.items.is_empty());
    }

    #[test]
    fn creator_and_version_only_download_stays_unknown_without_installed_match() {
        let mut connection = Connection::open_in_memory().expect("in-memory db");
        database::initialize(&mut connection).expect("schema");
        let seed_pack = load_seed_pack().expect("seed pack");
        database::seed_database(&mut connection, &seed_pack).expect("seed db");

        let settings = LibrarySettings {
            mods_path: Some("C:/Users/Test/Documents/Electronic Arts/The Sims 4/Mods".to_owned()),
            tray_path: None,
            downloads_path: Some("C:/Users/Test/Downloads".to_owned()),
            ..Default::default()
        };

        insert_compare_download_item(&connection, 1, "creator_version_only.package");
        let insights = FileInsights {
            creator_hints: vec!["TestCreator".to_owned()],
            version_hints: vec!["1.0".to_owned()],
            version_signals: vec![VersionSignal {
                raw_value: "1.0".to_owned(),
                normalized_value: "1.0".to_owned(),
                source_kind: "filename".to_owned(),
                source_path: None,
                matched_by: Some("fixture".to_owned()),
                confidence: 0.88,
            }],
            ..FileInsights::default()
        };

        insert_compare_file(
            &connection,
            "C:/Users/Test/Downloads/creator_version_only.package",
            "creator_version_only.package",
            "downloads",
            None,
            Some(1),
            &insights,
        );

        let resolution = resolve_download_item_version(
            &connection,
            &settings,
            &seed_pack,
            1,
            CompareDetailLevel::Full,
        )
        .expect("resolution")
        .expect("version resolution");

        assert_eq!(resolution.status, VersionCompareStatus::Unknown);
        assert_eq!(resolution.confidence, VersionConfidence::Unknown);
    }

    #[test]
    fn creator_family_and_version_download_can_still_report_not_installed() {
        let mut connection = Connection::open_in_memory().expect("in-memory db");
        database::initialize(&mut connection).expect("schema");
        let seed_pack = load_seed_pack().expect("seed pack");
        database::seed_database(&mut connection, &seed_pack).expect("seed db");

        let settings = LibrarySettings {
            mods_path: Some("C:/Users/Test/Documents/Electronic Arts/The Sims 4/Mods".to_owned()),
            tray_path: None,
            downloads_path: Some("C:/Users/Test/Downloads".to_owned()),
            ..Default::default()
        };

        insert_compare_download_item(&connection, 1, "identified_family.package");
        let insights = FileInsights {
            creator_hints: vec!["TestCreator".to_owned()],
            family_hints: vec!["alpha suite".to_owned()],
            version_hints: vec!["2.3".to_owned()],
            version_signals: vec![VersionSignal {
                raw_value: "2.3".to_owned(),
                normalized_value: "2.3".to_owned(),
                source_kind: "filename".to_owned(),
                source_path: None,
                matched_by: Some("fixture".to_owned()),
                confidence: 0.88,
            }],
            ..FileInsights::default()
        };

        insert_compare_file(
            &connection,
            "C:/Users/Test/Downloads/identified_family.package",
            "identified_family.package",
            "downloads",
            None,
            Some(1),
            &insights,
        );

        let resolution = resolve_download_item_version(
            &connection,
            &settings,
            &seed_pack,
            1,
            CompareDetailLevel::Full,
        )
        .expect("resolution")
        .expect("version resolution");

        assert_eq!(resolution.status, VersionCompareStatus::NotInstalled);
        assert_eq!(resolution.confidence, VersionConfidence::Medium);
    }

    #[test]
    fn candidate_family_hints_skip_short_values_and_prefer_stronger_clues() {
        let subject = super::VersionSubject {
            key: "download-item:1".to_owned(),
            label: "Alpha Suite".to_owned(),
            aggregate_signature: None,
            all_hashes_present: false,
            creator_tokens: BTreeSet::new(),
            family_tokens: BTreeSet::new(),
            namespace_tokens: BTreeSet::new(),
            embedded_tokens: BTreeSet::new(),
            filename_tokens: BTreeSet::new(),
            version: super::SubjectVersion::default(),
            files: vec![super::SubjectFileRow {
                id: 1,
                filename: "mystery.package".to_owned(),
                path: "C:/Users/Test/Downloads/mystery.package".to_owned(),
                hash: None,
                size: 42,
                creator: None,
                source_location: "downloads".to_owned(),
                insights: FileInsights {
                    family_hints: vec![
                        "abc".to_owned(),
                        "alpha suite".to_owned(),
                        "alphasuite".to_owned(),
                        "beta".to_owned(),
                    ],
                    ..FileInsights::default()
                },
            }],
        };

        assert_eq!(
            super::collect_candidate_family_hints(&subject),
            vec![
                "alpha suite".to_owned(),
                "alphasuite".to_owned(),
                "beta".to_owned(),
            ]
        );
    }

    #[test]
    fn full_compare_uses_family_hints_to_find_installed_candidates() {
        let mut connection = Connection::open_in_memory().expect("in-memory db");
        database::initialize(&mut connection).expect("schema");
        let seed_pack = load_seed_pack().expect("seed pack");
        database::seed_database(&mut connection, &seed_pack).expect("seed db");

        let settings = LibrarySettings {
            mods_path: Some("C:/Users/Test/Documents/Electronic Arts/The Sims 4/Mods".to_owned()),
            tray_path: None,
            downloads_path: Some("C:/Users/Test/Downloads".to_owned()),
            ..Default::default()
        };

        let installed_insights = FileInsights {
            family_hints: vec!["alpha suite".to_owned()],
            version_hints: vec!["2.3".to_owned()],
            version_signals: vec![VersionSignal {
                raw_value: "2.3".to_owned(),
                normalized_value: "2.3".to_owned(),
                source_kind: "filename".to_owned(),
                source_path: None,
                matched_by: Some("fixture".to_owned()),
                confidence: 0.88,
            }],
            ..FileInsights::default()
        };
        insert_compare_file(
            &connection,
            "C:/Users/Test/Documents/Electronic Arts/The Sims 4/Mods/FamilyOnly/hidden_core.package",
            "hidden_core.package",
            "mods",
            None,
            None,
            &installed_insights,
        );

        insert_compare_download_item(&connection, 1, "mystery_bundle.package");
        let incoming_insights = FileInsights {
            family_hints: vec!["alpha suite".to_owned()],
            version_hints: vec!["2.3".to_owned()],
            version_signals: vec![VersionSignal {
                raw_value: "2.3".to_owned(),
                normalized_value: "2.3".to_owned(),
                source_kind: "filename".to_owned(),
                source_path: None,
                matched_by: Some("fixture".to_owned()),
                confidence: 0.88,
            }],
            ..FileInsights::default()
        };
        insert_compare_file(
            &connection,
            "C:/Users/Test/Downloads/mystery_bundle.package",
            "mystery_bundle.package",
            "downloads",
            None,
            Some(1),
            &incoming_insights,
        );

        let resolution = resolve_download_item_version(
            &connection,
            &settings,
            &seed_pack,
            1,
            CompareDetailLevel::Full,
        )
        .expect("resolution")
        .expect("version resolution");

        assert_eq!(resolution.status, VersionCompareStatus::SameVersion);
        assert_eq!(
            resolution.matched_subject_label.as_deref(),
            Some("alpha suite")
        );
        assert_eq!(resolution.installed_version.as_deref(), Some("2.3"));
    }

    #[test]
    fn full_compare_uses_creator_hints_to_find_installed_candidates() {
        let mut connection = Connection::open_in_memory().expect("in-memory db");
        database::initialize(&mut connection).expect("schema");
        let seed_pack = load_seed_pack().expect("seed pack");
        database::seed_database(&mut connection, &seed_pack).expect("seed db");

        let settings = LibrarySettings {
            mods_path: Some("C:/Users/Test/Documents/Electronic Arts/The Sims 4/Mods".to_owned()),
            tray_path: None,
            downloads_path: Some("C:/Users/Test/Downloads".to_owned()),
            ..Default::default()
        };

        connection
            .execute(
                "INSERT INTO creators (canonical_name, notes, created_by_user)
                 VALUES (?1, ?2, 1)",
                params!["HintMaker", "Test creator"],
            )
            .expect("insert creator");
        let creator_id = connection.last_insert_rowid();

        let installed_insights = FileInsights {
            family_hints: vec!["alpha suite".to_owned()],
            version_hints: vec!["2.3".to_owned()],
            version_signals: vec![VersionSignal {
                raw_value: "2.3".to_owned(),
                normalized_value: "2.3".to_owned(),
                source_kind: "filename".to_owned(),
                source_path: None,
                matched_by: Some("fixture".to_owned()),
                confidence: 0.88,
            }],
            ..FileInsights::default()
        };
        insert_compare_file(
            &connection,
            "C:/Users/Test/Documents/Electronic Arts/The Sims 4/Mods/HintMaker/installed_alpha_core.package",
            "installed_alpha_core.package",
            "mods",
            Some(creator_id),
            None,
            &installed_insights,
        );

        insert_compare_download_item(&connection, 1, "fresh_bundle.package");
        let incoming_insights = FileInsights {
            creator_hints: vec!["HintMaker".to_owned()],
            family_hints: vec!["alpha suite".to_owned()],
            version_hints: vec!["2.3".to_owned()],
            version_signals: vec![VersionSignal {
                raw_value: "2.3".to_owned(),
                normalized_value: "2.3".to_owned(),
                source_kind: "filename".to_owned(),
                source_path: None,
                matched_by: Some("fixture".to_owned()),
                confidence: 0.88,
            }],
            ..FileInsights::default()
        };
        insert_compare_file(
            &connection,
            "C:/Users/Test/Downloads/fresh_bundle.package",
            "fresh_bundle.package",
            "downloads",
            None,
            Some(1),
            &incoming_insights,
        );

        let resolution = resolve_download_item_version(
            &connection,
            &settings,
            &seed_pack,
            1,
            CompareDetailLevel::Full,
        )
        .expect("resolution")
        .expect("version resolution");

        assert_eq!(resolution.status, VersionCompareStatus::SameVersion);
        assert_eq!(
            resolution.matched_subject_label.as_deref(),
            Some("alpha suite")
        );
        assert_eq!(resolution.installed_version.as_deref(), Some("2.3"));
    }

    #[test]
    fn watch_setup_list_skips_tracked_items() {
        let (connection, seed_pack, settings, file_id) = setup_watch_env();

        save_watch_source_for_library_file(
            &connection,
            &settings,
            &seed_pack,
            file_id,
            WatchSourceKind::ExactPage,
            Some("GitHub Release".to_owned()),
            "https://github.com/example/mod/releases",
        )
        .expect("save watch");

        let response = list_library_watch_setup_items(&connection, &settings, &seed_pack, 6)
            .expect("watch setup list");

        assert_eq!(response.total, 0);
        assert!(response.items.is_empty());
    }

    #[test]
    fn saving_creator_page_marks_reminder_only_state() {
        let (connection, seed_pack, settings, file_id) = setup_watch_env();

        let watch = save_watch_source_for_library_file(
            &connection,
            &settings,
            &seed_pack,
            file_id,
            WatchSourceKind::CreatorPage,
            Some("Creator".to_owned()),
            "https://example.com/creator-page",
        )
        .expect("save watch")
        .expect("watch result");

        assert_eq!(watch.status, WatchStatus::NotWatched);
        assert_eq!(watch.source_kind, Some(WatchSourceKind::CreatorPage));
        assert_eq!(watch.source_origin, WatchSourceOrigin::SavedByUser);
        assert_eq!(watch.capability, WatchCapability::SavedReferenceOnly);
        assert!(!watch.can_refresh_now);
        assert!(watch
            .note
            .as_deref()
            .is_some_and(|note| note.contains("reminders")));

        let refreshable =
            list_auto_refreshable_watch_file_ids(&connection, &seed_pack).expect("targets");
        assert!(!refreshable.contains(&file_id));

        let review = list_library_watch_review_items(&connection, &settings, &seed_pack, 8)
            .expect("watch review list");
        assert_eq!(review.total, 1);
        assert_eq!(review.reference_only_count, 1);
        assert_eq!(
            review.items[0].review_reason,
            LibraryWatchReviewReason::ReferenceOnly
        );
    }

    #[test]
    fn saving_curseforge_exact_page_marks_provider_required_state() {
        let (connection, seed_pack, settings, file_id) = setup_watch_env();

        let watch = save_watch_source_for_library_file(
            &connection,
            &settings,
            &seed_pack,
            file_id,
            WatchSourceKind::ExactPage,
            Some("CurseForge page".to_owned()),
            "https://www.curseforge.com/sims4/mods/example-mod",
        )
        .expect("save watch")
        .expect("watch result");

        assert_eq!(watch.status, WatchStatus::NotWatched);
        assert_eq!(watch.source_kind, Some(WatchSourceKind::ExactPage));
        assert_eq!(watch.source_origin, WatchSourceOrigin::SavedByUser);
        assert_eq!(watch.capability, WatchCapability::ProviderRequired);
        assert_eq!(watch.provider_name.as_deref(), Some("CurseForge"));
        assert!(!watch.can_refresh_now);
        assert!(watch
            .note
            .as_deref()
            .is_some_and(|note| note.contains("approved API path")));

        let review = list_library_watch_review_items(&connection, &settings, &seed_pack, 8)
            .expect("watch review list");
        assert_eq!(review.total, 1);
        assert_eq!(review.provider_needed_count, 1);
        assert_eq!(review.items[0].file_id, file_id);
        assert_eq!(
            review.items[0].review_reason,
            LibraryWatchReviewReason::ProviderNeeded
        );
    }

    #[test]
    fn refreshing_creator_page_watch_stays_cautious_without_network_guessing() {
        let (connection, seed_pack, settings, file_id) = setup_watch_env();

        save_watch_source_for_library_file(
            &connection,
            &settings,
            &seed_pack,
            file_id,
            WatchSourceKind::CreatorPage,
            Some("Creator".to_owned()),
            "https://example.com/creator-page",
        )
        .expect("save watch");

        let watch =
            refresh_watch_source_for_library_file(&connection, &settings, &seed_pack, file_id)
                .expect("refresh watch")
                .expect("watch result");

        assert_eq!(watch.status, WatchStatus::Unknown);
        assert_eq!(watch.source_kind, Some(WatchSourceKind::CreatorPage));
        assert_eq!(watch.source_origin, WatchSourceOrigin::SavedByUser);
        assert_eq!(watch.capability, WatchCapability::SavedReferenceOnly);
        assert!(!watch.can_refresh_now);
        assert!(watch.checked_at.is_some());
        assert!(watch
            .note
            .as_deref()
            .is_some_and(|note| note.contains("creator-page checks are not built yet")));
    }

    #[test]
    fn attention_watch_list_keeps_unknown_watch_rows() {
        let (connection, seed_pack, settings, file_id) = setup_watch_env();

        save_watch_source_for_library_file(
            &connection,
            &settings,
            &seed_pack,
            file_id,
            WatchSourceKind::CreatorPage,
            Some("Creator".to_owned()),
            "https://example.com/creator-page",
        )
        .expect("save watch");

        refresh_watch_source_for_library_file(&connection, &settings, &seed_pack, file_id)
            .expect("refresh watch");

        let response = list_library_watch_items(
            &connection,
            &settings,
            &seed_pack,
            WatchListFilter::Attention,
            12,
        )
        .expect("watch list");

        assert_eq!(response.total, 1);
        assert_eq!(response.items[0].file_id, file_id);
        assert_eq!(response.items[0].watch_result.status, WatchStatus::Unknown);
    }

    #[test]
    fn clearing_watch_source_for_library_file_returns_not_watched_state() {
        let (connection, seed_pack, settings, file_id) = setup_watch_env();

        save_watch_source_for_library_file(
            &connection,
            &settings,
            &seed_pack,
            file_id,
            WatchSourceKind::CreatorPage,
            Some("Test Creator".to_owned()),
            "https://example.com/creator-page",
        )
        .expect("save watch");

        let watch =
            clear_watch_source_for_library_file(&connection, &settings, &seed_pack, file_id)
                .expect("clear watch")
                .expect("watch result");

        assert_eq!(watch.status, WatchStatus::NotWatched);
        assert!(watch.source_kind.is_none());
        assert_eq!(watch.source_origin, WatchSourceOrigin::None);
        assert_eq!(watch.capability, WatchCapability::SavedReferenceOnly);
        assert!(watch.source_url.is_none());
        assert!(watch
            .note
            .as_deref()
            .is_some_and(|note| note.contains("No approved watch source")));
    }

    #[test]
    fn saving_watch_source_for_download_file_is_rejected() {
        let (connection, seed_pack, settings, _file_id) = setup_watch_env();
        connection
            .execute(
                "INSERT INTO files (
                    path,
                    filename,
                    extension,
                    kind,
                    confidence,
                    source_location,
                    parser_warnings,
                    insights
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    "C:/Users/Test/Downloads/watch_test_mod_v1.0.package",
                    "watch_test_mod_v1.0.package",
                    ".package",
                    "Gameplay",
                    0.74_f64,
                    "downloads",
                    "[]",
                    serde_json::to_string(&FileInsights::default()).expect("insights json"),
                ],
            )
            .expect("insert download file");
        let download_file_id = connection.last_insert_rowid();

        let watch = save_watch_source_for_library_file(
            &connection,
            &settings,
            &seed_pack,
            download_file_id,
            WatchSourceKind::ExactPage,
            Some("Download row".to_owned()),
            "https://example.com/download-row",
        )
        .expect("save watch");

        assert!(watch.is_none());
    }

    #[test]
    fn saving_custom_watch_source_for_supported_special_mod_is_rejected() {
        let mut connection = Connection::open_in_memory().expect("in-memory db");
        database::initialize(&mut connection).expect("schema");
        let seed_pack = load_seed_pack().expect("seed pack");
        database::seed_database(&mut connection, &seed_pack).expect("seed db");

        let settings = LibrarySettings {
            mods_path: Some("C:/Users/Test/Documents/Electronic Arts/The Sims 4/Mods".to_owned()),
            tray_path: None,
            downloads_path: Some("C:/Users/Test/Downloads".to_owned()),
            ..Default::default()
        };
        let insights = FileInsights {
            family_hints: vec!["s4cl".to_owned()],
            version_hints: vec!["2.9.0".to_owned()],
            version_signals: vec![VersionSignal {
                raw_value: "2.9.0".to_owned(),
                normalized_value: "2.9.0".to_owned(),
                source_kind: "filename".to_owned(),
                source_path: None,
                matched_by: Some("fixture".to_owned()),
                confidence: 0.88,
            }],
            ..FileInsights::default()
        };

        connection
            .execute(
                "INSERT INTO files (
                    path,
                    filename,
                    extension,
                    kind,
                    confidence,
                    source_location,
                    parser_warnings,
                    insights
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    "C:/Users/Test/Documents/Electronic Arts/The Sims 4/Mods/S4CL/S4CL.ts4script",
                    "S4CL.ts4script",
                    ".ts4script",
                    "ScriptMods",
                    0.97_f64,
                    "mods",
                    "[]",
                    serde_json::to_string(&insights).expect("insights json"),
                ],
            )
            .expect("insert file");
        let file_id = connection.last_insert_rowid();

        let error = save_watch_source_for_library_file(
            &connection,
            &settings,
            &seed_pack,
            file_id,
            WatchSourceKind::ExactPage,
            Some("Custom page".to_owned()),
            "https://example.com/custom-page",
        )
        .expect_err("custom save should be rejected");

        assert!(error.to_string().contains("built-in official page"));
    }

    #[test]
    fn watch_list_returns_built_in_special_rows() {
        let mut connection = Connection::open_in_memory().expect("in-memory db");
        database::initialize(&mut connection).expect("schema");
        let seed_pack = load_seed_pack().expect("seed pack");
        database::seed_database(&mut connection, &seed_pack).expect("seed db");

        let settings = LibrarySettings {
            mods_path: Some("C:/Users/Test/Documents/Electronic Arts/The Sims 4/Mods".to_owned()),
            tray_path: None,
            downloads_path: Some("C:/Users/Test/Downloads".to_owned()),
            ..Default::default()
        };
        let insights = FileInsights {
            family_hints: vec!["s4cl".to_owned()],
            version_hints: vec!["2.9.0".to_owned()],
            version_signals: vec![VersionSignal {
                raw_value: "2.9.0".to_owned(),
                normalized_value: "2.9.0".to_owned(),
                source_kind: "filename".to_owned(),
                source_path: None,
                matched_by: Some("fixture".to_owned()),
                confidence: 0.88,
            }],
            ..FileInsights::default()
        };

        connection
            .execute(
                "INSERT INTO files (
                    path,
                    filename,
                    extension,
                    kind,
                    confidence,
                    source_location,
                    parser_warnings,
                    insights
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    "C:/Users/Test/Documents/Electronic Arts/The Sims 4/Mods/S4CL/S4CL.ts4script",
                    "S4CL.ts4script",
                    ".ts4script",
                    "ScriptMods",
                    0.97_f64,
                    "mods",
                    "[]",
                    serde_json::to_string(&insights).expect("insights json"),
                ],
            )
            .expect("insert file");
        let file_id = connection.last_insert_rowid();

        let response =
            list_library_watch_items(&connection, &settings, &seed_pack, WatchListFilter::All, 12)
                .expect("watch list");

        assert_eq!(response.total, 1);
        assert_eq!(response.items[0].file_id, file_id);
        assert_eq!(
            response.items[0].watch_result.source_origin,
            WatchSourceOrigin::BuiltInSpecial
        );
    }
}
