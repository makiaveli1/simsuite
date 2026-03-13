use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    path::{Path, PathBuf},
};

use rusqlite::{params, params_from_iter, types::Value, Connection, OptionalExtension};

use crate::{
    core::special_mod_versions::{build_signature, parse_version_parts, SignatureEntry},
    error::AppResult,
    models::{
        FileInsights, InstalledVersionSummary, VersionCompareStatus, VersionConfidence,
        VersionResolution, VersionSignal, WatchResult, WatchSourceKind, WatchStatus,
    },
    seed::{GuidedInstallProfileSeed, SeedPack},
};

const MAX_CANDIDATE_ROWS: usize = 96;
const MAX_SEARCH_TOKENS: usize = 4;
const MATCH_SCORE_STRONG: f64 = 1.20;
const MATCH_SCORE_MEDIUM: f64 = 0.80;
const MATCH_SCORE_WEAK: f64 = 0.45;
const SIGNAL_CONFLICT_CONFIDENCE: f64 = 0.72;
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
    settings: &crate::models::LibrarySettings,
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

pub fn load_watch_counts(connection: &Connection) -> AppResult<(i64, i64, i64)> {
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

    Ok((
        exact_generic + exact_special,
        possible_generic,
        unknown_generic + unknown_special,
    ))
}

fn scalar(connection: &Connection, sql: &str) -> AppResult<i64> {
    Ok(connection.query_row(sql, [], |row| row.get(0))?)
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
             WHERE f.id = ?1",
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
            .filter_map(|file| file.creator.clone())
            .filter(|value| !value.trim().is_empty())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        for row in load_installed_rows_by_creators(connection, &creators)? {
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
        if incoming_identity_score >= MATCH_SCORE_WEAK {
            resolution.status = VersionCompareStatus::NotInstalled;
            resolution.confidence = if incoming_identity_score >= MATCH_SCORE_MEDIUM {
                VersionConfidence::Medium
            } else {
                VersionConfidence::Weak
            };
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
    if subject.version.value.is_some() {
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
                source_label: Some(profile.display_name.clone()),
                source_url,
                latest_version,
                checked_at,
                confidence: confidence_from_signal(confidence),
                note: latest_note,
                evidence,
            }));
        }
    }

    let source = load_watch_source_row(connection, &subject.key)?;
    let result = load_watch_result_row(connection, &subject.key)?;
    match (source, result) {
        (Some(source), Some(result)) => Ok(Some(WatchResult {
            status: result.status,
            source_kind: Some(source.source_kind),
            source_label: source.source_label,
            source_url: Some(source.source_url),
            latest_version: result.latest_version,
            checked_at: result.checked_at,
            confidence: result.confidence,
            note: result.note,
            evidence: result.evidence,
        })),
        _ => Ok(Some(WatchResult {
            status: WatchStatus::NotWatched,
            source_kind: None,
            source_label: None,
            source_url: None,
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
