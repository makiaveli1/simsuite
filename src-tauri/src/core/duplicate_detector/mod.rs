use std::{
    collections::{HashMap, HashSet},
    path::Path,
    sync::OnceLock,
};

use regex::Regex;
use rusqlite::{params, Connection, OptionalExtension};

use crate::{
    error::AppResult,
    models::{DuplicateOverview, DuplicatePair},
};

const MAX_REVIEW_PAIRS_PER_GROUP: usize = 2_000;

pub fn rebuild_duplicates(connection: &mut Connection) -> AppResult<usize> {
    let transaction = connection.transaction()?;
    transaction.execute("DELETE FROM duplicates", [])?;

    transaction.execute(
        "INSERT INTO duplicates (file_id_a, file_id_b, duplicate_type, detection_method, created_at)
         SELECT a.id, b.id, 'exact', 'sha256', CURRENT_TIMESTAMP
         FROM files a
         JOIN files b ON LOWER(TRIM(a.hash)) = LOWER(TRIM(b.hash)) AND a.id < b.id
         WHERE TRIM(COALESCE(a.hash, '')) <> ''
           AND TRIM(COALESCE(b.hash, '')) <> ''
           AND TRIM(COALESCE(a.path, '')) <> ''
           AND TRIM(COALESCE(b.path, '')) <> ''
           AND LOWER(REPLACE(TRIM(a.path), '/', '\\')) <> LOWER(REPLACE(TRIM(b.path), '/', '\\'))",
        params![],
    )?;

    insert_filename_duplicates(&transaction)?;

    insert_version_duplicates(&transaction)?;

    let count: i64 =
        transaction.query_row("SELECT COUNT(*) FROM duplicates", [], |row| row.get(0))?;
    transaction.commit()?;
    Ok(count as usize)
}

pub fn get_duplicate_overview(connection: &Connection) -> AppResult<DuplicateOverview> {
    let exact_proof = exact_file_proof_sql("a", "b", "d");
    Ok(DuplicateOverview {
        total_pairs: scalar(
            connection,
            "SELECT COUNT(*)
             FROM duplicates d
             JOIN files a ON d.file_id_a = a.id
             JOIN files b ON d.file_id_b = b.id",
        )?,
        exact_pairs: scalar(
            connection,
            &format!(
                "SELECT COUNT(*)
                 FROM duplicates d
                 JOIN files a ON d.file_id_a = a.id
                 JOIN files b ON d.file_id_b = b.id
                 WHERE {exact_proof}"
            ),
        )?,
        filename_pairs: scalar(
            connection,
            &format!(
                "SELECT COUNT(*)
                 FROM duplicates d
                 JOIN files a ON d.file_id_a = a.id
                 JOIN files b ON d.file_id_b = b.id
                 WHERE d.duplicate_type = 'filename'
                   AND NOT ({exact_proof})"
            ),
        )?,
        version_pairs: scalar(
            connection,
            &format!(
                "SELECT COUNT(*)
                 FROM duplicates d
                 JOIN files a ON d.file_id_a = a.id
                 JOIN files b ON d.file_id_b = b.id
                 WHERE d.duplicate_type = 'version'
                   AND NOT ({exact_proof})"
            ),
        )?,
    })
}

pub fn list_duplicate_pairs(
    connection: &Connection,
    duplicate_type: Option<String>,
    limit: i64,
) -> AppResult<Vec<DuplicatePair>> {
    let limit = limit.max(1);
    let exact_proof = exact_file_proof_sql("a", "b", "d");
    let items = if let Some(duplicate_type) = duplicate_type.filter(|value| !value.is_empty()) {
        let duplicate_type = duplicate_type.trim().to_ascii_lowercase();
        let mut statement = connection.prepare(&format!(
            "SELECT
                d.id,
                d.duplicate_type,
                d.detection_method,
                a.id,
                a.filename,
                a.path,
                ca.canonical_name,
                a.hash,
                a.modified_at,
                a.size,
                b.id,
                b.filename,
                b.path,
                cb.canonical_name,
                b.hash,
                b.modified_at,
                b.size
             FROM duplicates d
             JOIN files a ON d.file_id_a = a.id
             JOIN files b ON d.file_id_b = b.id
             LEFT JOIN creators ca ON a.creator_id = ca.id
             LEFT JOIN creators cb ON b.creator_id = cb.id
             WHERE (
                (?1 = 'exact' AND ({exact_proof}))
                OR (?1 <> 'exact' AND d.duplicate_type = ?1 AND NOT ({exact_proof}))
             )
             ORDER BY d.duplicate_type, a.filename COLLATE NOCASE, b.filename COLLATE NOCASE
             LIMIT ?2",
        ))?;

        let rows = statement
            .query_map(params![duplicate_type, limit], map_duplicate_pair)?
            .collect::<Result<Vec<_>, _>>()?;
        rows
    } else {
        let mut statement = connection.prepare(&format!(
            "SELECT
                d.id,
                d.duplicate_type,
                d.detection_method,
                a.id,
                a.filename,
                a.path,
                ca.canonical_name,
                a.hash,
                a.modified_at,
                a.size,
                b.id,
                b.filename,
                b.path,
                cb.canonical_name,
                b.hash,
                b.modified_at,
                b.size
             FROM duplicates d
             JOIN files a ON d.file_id_a = a.id
             JOIN files b ON d.file_id_b = b.id
             LEFT JOIN creators ca ON a.creator_id = ca.id
             LEFT JOIN creators cb ON b.creator_id = cb.id
             ORDER BY
                CASE
                    WHEN ({exact_proof}) THEN 0
                    WHEN d.duplicate_type = 'version' THEN 1
                    ELSE 2
                END,
                a.filename COLLATE NOCASE,
                b.filename COLLATE NOCASE
             LIMIT ?1",
        ))?;

        let rows = statement
            .query_map(params![limit], map_duplicate_pair)?
            .collect::<Result<Vec<_>, _>>()?;
        rows
    };

    Ok(items)
}

fn map_duplicate_pair(row: &rusqlite::Row<'_>) -> rusqlite::Result<DuplicatePair> {
    let duplicate_type: String = row.get(1)?;
    let detection_method: String = row.get(2)?;
    let primary_file_id: i64 = row.get(3)?;
    let primary_filename: String = row.get(4)?;
    let primary_path: String = row.get(5)?;
    let primary_creator: Option<String> = row.get(6)?;
    let primary_hash: Option<String> = row.get(7)?;
    let primary_size: i64 = row.get(9)?;
    let secondary_file_id: i64 = row.get(10)?;
    let secondary_filename: String = row.get(11)?;
    let secondary_path: String = row.get(12)?;
    let secondary_creator: Option<String> = row.get(13)?;
    let secondary_hash: Option<String> = row.get(14)?;
    let secondary_size: i64 = row.get(16)?;
    let intelligence = classify_duplicate_pair(
        &duplicate_type,
        &detection_method,
        primary_file_id,
        &primary_filename,
        &primary_path,
        primary_creator.as_deref(),
        primary_hash.as_deref(),
        primary_size,
        secondary_file_id,
        &secondary_filename,
        &secondary_path,
        secondary_creator.as_deref(),
        secondary_hash.as_deref(),
        secondary_size,
    );

    Ok(DuplicatePair {
        id: row.get(0)?,
        duplicate_type,
        detection_method,
        is_duplicate: intelligence.is_duplicate,
        comparison_kind: intelligence.comparison_kind,
        classification: intelligence.classification,
        classification_label: intelligence.classification_label,
        confidence_label: intelligence.confidence_label,
        evidence: intelligence.evidence,
        cautions: intelligence.cautions,
        primary_file_id,
        primary_filename,
        primary_path,
        primary_creator,
        primary_hash,
        primary_modified_at: row.get(8)?,
        primary_size,
        secondary_file_id,
        secondary_filename,
        secondary_path,
        secondary_creator,
        secondary_hash,
        secondary_modified_at: row.get(15)?,
        secondary_size,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DuplicateIntelligence {
    is_duplicate: bool,
    comparison_kind: String,
    classification: String,
    classification_label: String,
    confidence_label: String,
    evidence: Vec<String>,
    cautions: Vec<String>,
}

#[allow(clippy::too_many_arguments)]
fn classify_duplicate_pair(
    duplicate_type: &str,
    detection_method: &str,
    primary_file_id: i64,
    primary_filename: &str,
    primary_path: &str,
    primary_creator: Option<&str>,
    primary_hash: Option<&str>,
    primary_size: i64,
    secondary_file_id: i64,
    secondary_filename: &str,
    secondary_path: &str,
    secondary_creator: Option<&str>,
    secondary_hash: Option<&str>,
    secondary_size: i64,
) -> DuplicateIntelligence {
    let same_hash = hashes_match(primary_hash, secondary_hash);
    let exact_file_proof = exact_file_proof(
        primary_file_id,
        primary_path,
        primary_hash,
        secondary_file_id,
        secondary_path,
        secondary_hash,
    );
    let malformed_pair =
        primary_file_id <= 0 || secondary_file_id <= 0 || primary_file_id == secondary_file_id;
    let missing_path = path_missing(primary_path, secondary_path);
    let same_canonical_path = same_canonical_path(primary_path, secondary_path);
    let same_filename = primary_filename.eq_ignore_ascii_case(secondary_filename);
    let primary_versions = version_tokens_from_filename(primary_filename);
    let secondary_versions = version_tokens_from_filename(secondary_filename);
    let has_version_clue = !primary_versions.is_empty() || !secondary_versions.is_empty();
    let version_differs = !primary_versions.is_empty()
        && !secondary_versions.is_empty()
        && primary_versions != secondary_versions;

    let (is_duplicate, comparison_kind, classification, classification_label, confidence_label) =
        if exact_file_proof {
            (
                true,
                "exact_file",
                "duplicate",
                "Duplicate",
                "Same file contents",
            )
        } else if malformed_pair || missing_path || same_canonical_path {
            (
                false,
                "unknown",
                "unknown",
                "Manual review needed",
                "Limited evidence",
            )
        } else if duplicate_type.eq_ignore_ascii_case("version")
            || (has_version_clue && version_differs && !same_hash)
        {
            (
                false,
                "version_review",
                "version_review",
                "Version review",
                "Version clue",
            )
        } else if duplicate_type.eq_ignore_ascii_case("filename") || same_filename {
            (
                false,
                "name_match_review",
                "name_match_review",
                "Name match",
                "Name match",
            )
        } else {
            (
                false,
                "unknown",
                "unknown",
                "Manual review needed",
                "Limited evidence",
            )
        };

    let mut evidence = Vec::new();
    if exact_file_proof {
        evidence.push("Same file contents".to_owned());
    } else if same_hash {
        evidence.push("Matching hash needs manual review".to_owned());
    } else if normalized_hash(primary_hash).is_some() && normalized_hash(secondary_hash).is_some() {
        evidence.push("Content hash differs".to_owned());
    } else {
        evidence.push("Hash missing".to_owned());
    }

    if malformed_pair {
        evidence.push("Duplicate row is incomplete".to_owned());
    }
    if missing_path {
        evidence.push("Path metadata is incomplete".to_owned());
    }
    if same_canonical_path {
        evidence.push("Duplicate row points to the same path".to_owned());
    }

    if same_filename {
        evidence.push("Same filename".to_owned());
    } else if canonical_version_key(primary_filename).is_some()
        && canonical_version_key(primary_filename) == canonical_version_key(secondary_filename)
    {
        evidence.push("Similar filename".to_owned());
    }

    if has_version_clue {
        evidence.push("Version clue found".to_owned());
    }
    if version_differs {
        evidence.push("Version differs".to_owned());
    }

    if primary_size == secondary_size {
        evidence.push("Size matches".to_owned());
    } else {
        evidence.push("Size differs".to_owned());
    }

    match (
        normalize_creator(primary_creator),
        normalize_creator(secondary_creator),
    ) {
        (Some(left), Some(right)) if left == right => evidence.push("Creator matches".to_owned()),
        (Some(_), Some(_)) => evidence.push("Creator differs".to_owned()),
        _ => evidence.push("Creator unknown".to_owned()),
    }

    evidence.push(format!("Detection method: {detection_method}"));

    let mut cautions = vec!["Compare before changing anything".to_owned()];
    if !is_duplicate {
        cautions.push("This is not duplicate proof".to_owned());
    }
    if classification == "version_review" {
        cautions.push("This may be another release of the same mod".to_owned());
    }
    if classification == "name_match_review" {
        cautions.push("Same filename is not same-content proof".to_owned());
    }
    if primary_creator.is_none() || secondary_creator.is_none() {
        cautions.push("Creator metadata is incomplete".to_owned());
    }
    if malformed_pair || missing_path || same_canonical_path {
        cautions.push("SimSuite has limited information here".to_owned());
    }

    DuplicateIntelligence {
        is_duplicate,
        comparison_kind: comparison_kind.to_owned(),
        classification: classification.to_owned(),
        classification_label: classification_label.to_owned(),
        confidence_label: confidence_label.to_owned(),
        evidence: unique_strings(evidence),
        cautions: unique_strings(cautions),
    }
}

fn hashes_match(primary_hash: Option<&str>, secondary_hash: Option<&str>) -> bool {
    normalized_hash(primary_hash).is_some()
        && normalized_hash(primary_hash) == normalized_hash(secondary_hash)
}

fn exact_file_proof(
    primary_file_id: i64,
    primary_path: &str,
    primary_hash: Option<&str>,
    secondary_file_id: i64,
    secondary_path: &str,
    secondary_hash: Option<&str>,
) -> bool {
    primary_file_id > 0
        && secondary_file_id > 0
        && primary_file_id != secondary_file_id
        && !path_missing(primary_path, secondary_path)
        && !same_canonical_path(primary_path, secondary_path)
        && hashes_match(primary_hash, secondary_hash)
}

fn normalized_hash(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_ascii_lowercase())
}

fn same_canonical_path(primary_path: &str, secondary_path: &str) -> bool {
    let primary = canonical_path_key(primary_path);
    let secondary = canonical_path_key(secondary_path);
    !primary.is_empty() && !secondary.is_empty() && primary == secondary
}

fn path_missing(primary_path: &str, secondary_path: &str) -> bool {
    canonical_path_key(primary_path).is_empty() || canonical_path_key(secondary_path).is_empty()
}

fn canonical_path_key(path: &str) -> String {
    path.trim().replace('/', "\\").to_ascii_lowercase()
}

fn exact_file_proof_sql(left_alias: &str, right_alias: &str, pair_alias: &str) -> String {
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

fn normalize_creator(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_lowercase())
}

fn unique_strings(values: Vec<String>) -> Vec<String> {
    let mut out = Vec::new();
    for value in values {
        if !out.contains(&value) {
            out.push(value);
        }
    }
    out
}

fn insert_version_duplicates(connection: &Connection) -> AppResult<()> {
    let mut statement = connection.prepare(
        "SELECT id, filename, extension
         FROM files
         ORDER BY filename COLLATE NOCASE",
    )?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;

    let existing_pairs = load_existing_pairs(connection)?;
    let mut grouped = HashMap::<String, Vec<(i64, String)>>::new();

    for (file_id, filename, extension) in rows {
        if let Some(version_key) = canonical_version_key(&filename) {
            grouped
                .entry(format!("{}|{}", version_key, extension.to_lowercase()))
                .or_default()
                .push((file_id, filename));
        }
    }

    let mut insert_statement = connection.prepare(
        "INSERT INTO duplicates (file_id_a, file_id_b, duplicate_type, detection_method, created_at)
         VALUES (?1, ?2, 'version', 'version_token_strip', CURRENT_TIMESTAMP)",
    )?;

    for files in grouped.values().filter(|items| items.len() > 1) {
        let mut inserted_for_group = 0_usize;
        'pairs: for left in 0..files.len() {
            for right in (left + 1)..files.len() {
                let (left_id, left_name) = &files[left];
                let (right_id, right_name) = &files[right];
                if left_name.eq_ignore_ascii_case(right_name) {
                    continue;
                }

                let pair_key = ordered_pair(*left_id, *right_id);
                if existing_pairs.contains(&pair_key) {
                    continue;
                }

                insert_statement.execute(params![pair_key.0, pair_key.1])?;
                inserted_for_group += 1;
                if inserted_for_group >= MAX_REVIEW_PAIRS_PER_GROUP {
                    break 'pairs;
                }
            }
        }
    }

    Ok(())
}

fn insert_filename_duplicates(connection: &Connection) -> AppResult<()> {
    let mut statement = connection.prepare(
        "SELECT id, filename, COALESCE(hash, '')
         FROM files
         ORDER BY filename COLLATE NOCASE, id",
    )?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;

    let mut grouped = HashMap::<String, Vec<(i64, String)>>::new();
    for (file_id, filename, hash) in rows {
        grouped
            .entry(filename.to_ascii_lowercase())
            .or_default()
            .push((file_id, hash));
    }

    let mut insert_statement = connection.prepare(
        "INSERT INTO duplicates (file_id_a, file_id_b, duplicate_type, detection_method, created_at)
         VALUES (?1, ?2, 'filename', 'filename_match', CURRENT_TIMESTAMP)",
    )?;

    for files in grouped.values().filter(|items| items.len() > 1) {
        let mut inserted_for_group = 0_usize;
        'pairs: for left in 0..files.len() {
            for right in (left + 1)..files.len() {
                let (left_id, left_hash) = &files[left];
                let (right_id, right_hash) = &files[right];
                let left_normalized = normalized_hash(Some(left_hash));
                let right_normalized = normalized_hash(Some(right_hash));
                if left_normalized == right_normalized {
                    continue;
                }

                let pair_key = ordered_pair(*left_id, *right_id);
                insert_statement.execute(params![pair_key.0, pair_key.1])?;
                inserted_for_group += 1;
                if inserted_for_group >= MAX_REVIEW_PAIRS_PER_GROUP {
                    break 'pairs;
                }
            }
        }
    }

    Ok(())
}

fn load_existing_pairs(connection: &Connection) -> AppResult<HashSet<(i64, i64)>> {
    let mut statement = connection.prepare("SELECT file_id_a, file_id_b FROM duplicates")?;
    let rows = statement
        .query_map([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)))?
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .map(|(left, right)| ordered_pair(left, right))
        .collect::<HashSet<_>>();
    Ok(rows)
}

fn ordered_pair(a: i64, b: i64) -> (i64, i64) {
    if a < b {
        (a, b)
    } else {
        (b, a)
    }
}

fn canonical_version_key(filename: &str) -> Option<String> {
    let stem = Path::new(filename)
        .file_stem()?
        .to_string_lossy()
        .to_lowercase();
    let tokens = stem
        .split(|character: char| !character.is_ascii_alphanumeric())
        .filter(|token| !token.is_empty())
        .collect::<Vec<_>>();
    if tokens.is_empty() {
        return None;
    }

    let mut normalized = Vec::new();
    let mut removed_version = false;
    let mut skip_numeric_after_keyword = false;

    for token in tokens {
        if matches!(token, "version" | "ver" | "update" | "updated" | "hotfix") {
            removed_version = true;
            skip_numeric_after_keyword = true;
            continue;
        }

        if skip_numeric_after_keyword && is_version_number(token) {
            removed_version = true;
            skip_numeric_after_keyword = false;
            continue;
        }
        skip_numeric_after_keyword = false;

        if is_prefixed_version_token(token) {
            removed_version = true;
            continue;
        }

        normalized.push(token.to_owned());
    }

    if removed_version && !normalized.is_empty() {
        Some(normalized.join(" "))
    } else {
        None
    }
}

fn version_tokens_from_filename(filename: &str) -> Vec<String> {
    static VERSION_TOKEN_RE: OnceLock<Regex> = OnceLock::new();
    static DATE_TOKEN_RE: OnceLock<Regex> = OnceLock::new();

    let stem = Path::new(filename)
        .file_stem()
        .map(|value| value.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    if stem.is_empty() {
        return Vec::new();
    }

    let version_re = VERSION_TOKEN_RE.get_or_init(|| {
        Regex::new(
            r"(?i)(?:^|[^a-z0-9])(?:v|ver|version|update|updated|hotfix|build)[\s_.-]*([0-9]+(?:[\s_.-][0-9]+){0,3}[a-z]?)",
        )
        .expect("valid version token regex")
    });
    let date_re = DATE_TOKEN_RE.get_or_init(|| {
        Regex::new(r"(?i)(?:^|[^0-9])((?:19|20)[0-9]{2}[\s_.-][0-9]{1,2}(?:[\s_.-][0-9]{1,2})?)(?:[^0-9]|$)")
            .expect("valid date token regex")
    });

    let mut tokens = Vec::new();
    for capture in version_re.captures_iter(&stem) {
        if let Some(value) = capture.get(1) {
            tokens.push(normalize_version_token(value.as_str()));
        }
    }
    for capture in date_re.captures_iter(&stem) {
        if let Some(value) = capture.get(1) {
            tokens.push(normalize_version_token(value.as_str()));
        }
    }

    unique_strings(tokens)
}

fn normalize_version_token(value: &str) -> String {
    value
        .trim()
        .trim_matches(|character: char| !character.is_ascii_alphanumeric())
        .replace([' ', '_', '-'], ".")
        .to_lowercase()
}

fn is_prefixed_version_token(token: &str) -> bool {
    let Some(stripped) = token.strip_prefix('v') else {
        return false;
    };

    !stripped.is_empty() && is_version_number(stripped)
}

fn is_version_number(token: &str) -> bool {
    token
        .chars()
        .all(|character| character.is_ascii_digit() || character == '.')
}

fn scalar(connection: &Connection, sql: &str) -> AppResult<i64> {
    connection
        .query_row(sql, [], |row| row.get(0))
        .optional()?
        .ok_or_else(|| crate::error::AppError::Message("Missing scalar result".to_owned()))
}

#[cfg(test)]
mod tests {
    use rusqlite::params;
    use std::time::Instant;

    use crate::database;

    use super::*;

    fn insert_file(connection: &Connection, filename: &str, hash: Option<&str>, size: i64) -> i64 {
        insert_file_with_creator(connection, filename, hash, size, None)
    }

    fn insert_file_with_creator(
        connection: &Connection,
        filename: &str,
        hash: Option<&str>,
        size: i64,
        creator: Option<&str>,
    ) -> i64 {
        let creator_id = creator.map(|name| {
            connection
                .execute(
                    "INSERT OR IGNORE INTO creators (canonical_name, notes) VALUES (?1, 'test')",
                    params![name],
                )
                .expect("insert creator");
            connection
                .query_row(
                    "SELECT id FROM creators WHERE canonical_name = ?1",
                    params![name],
                    |row| row.get::<_, i64>(0),
                )
                .expect("creator id")
        });

        connection
            .execute(
                "INSERT INTO files (
                    path, filename, extension, hash, size, creator_id, kind, confidence, source_location,
                    relative_depth, safety_notes, parser_warnings
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'CAS', 0.8, 'mods', 0, '[]', '[]')",
                params![
                    format!(r"C:\Mods\{}\{filename}", hash.unwrap_or("no-hash")),
                    filename,
                    Path::new(filename)
                        .extension()
                        .map(|value| format!(".{}", value.to_string_lossy().to_lowercase()))
                        .unwrap_or_default(),
                    hash,
                    size,
                    creator_id,
                ],
            )
            .expect("insert file");

        connection.last_insert_rowid()
    }

    fn insert_duplicate_row(
        connection: &Connection,
        left_id: i64,
        right_id: i64,
        duplicate_type: &str,
    ) {
        connection
            .execute(
                "INSERT INTO duplicates (file_id_a, file_id_b, duplicate_type, detection_method)
                 VALUES (?1, ?2, ?3, 'test_fixture')",
                params![left_id, right_id, duplicate_type],
            )
            .expect("insert duplicate row");
    }

    #[test]
    fn rebuild_duplicates_detects_exact_filename_and_version_pairs() {
        let mut connection = Connection::open_in_memory().expect("db");
        database::initialize(&mut connection).expect("schema");

        insert_file(&connection, "set.package", Some("same"), 10);
        insert_file(&connection, "set_copy.package", Some("same"), 10);
        insert_file(&connection, "hair.package", Some("aaa"), 10);
        insert_file(&connection, "hair.package", Some("bbb"), 10);
        insert_file(&connection, "mod_v1.package", Some("111"), 10);
        insert_file(&connection, "mod_v2.package", Some("222"), 10);

        let count = rebuild_duplicates(&mut connection).expect("rebuild");
        let overview = get_duplicate_overview(&connection).expect("overview");

        assert_eq!(count, 3);
        assert_eq!(overview.exact_pairs, 1);
        assert_eq!(overview.filename_pairs, 1);
        assert_eq!(overview.version_pairs, 1);
    }

    #[test]
    fn filename_duplicates_match_case_insensitively_and_exact_pairs_stay_separate() {
        let mut connection = Connection::open_in_memory().expect("db");
        database::initialize(&mut connection).expect("schema");

        insert_file(&connection, "Hair.package", Some("aaa"), 10);
        insert_file(&connection, "hair.package", Some("bbb"), 10);
        insert_file(&connection, "hair-copy.package", Some("bbb"), 10);

        rebuild_duplicates(&mut connection).expect("rebuild");
        let overview = get_duplicate_overview(&connection).expect("overview");

        assert_eq!(overview.exact_pairs, 1);
        assert_eq!(overview.filename_pairs, 1);
    }

    #[test]
    fn canonical_version_key_only_matches_version_marked_files() {
        assert_eq!(
            canonical_version_key("mod_v2.package").as_deref(),
            Some("mod")
        );
        assert_eq!(
            canonical_version_key("mod_version_3.package").as_deref(),
            Some("mod")
        );
        assert_eq!(canonical_version_key("chair_set.package"), None);
    }

    #[test]
    fn exact_hash_pair_returns_duplicate_classification_with_evidence() {
        let mut connection = Connection::open_in_memory().expect("db");
        database::initialize(&mut connection).expect("schema");

        insert_file_with_creator(
            &connection,
            "set.package",
            Some("same"),
            10,
            Some("Creator"),
        );
        insert_file_with_creator(
            &connection,
            "set_copy.package",
            Some("same"),
            10,
            Some("Creator"),
        );

        rebuild_duplicates(&mut connection).expect("rebuild");
        let pairs = list_duplicate_pairs(&connection, Some("exact".to_owned()), 10).expect("pairs");

        assert_eq!(pairs.len(), 1);
        assert!(pairs[0].is_duplicate);
        assert_eq!(pairs[0].comparison_kind, "exact_file");
        assert_eq!(pairs[0].classification, "duplicate");
        assert_eq!(pairs[0].classification_label, "Duplicate");
        assert!(pairs[0].evidence.contains(&"Same file contents".to_owned()));
        assert!(pairs[0].evidence.contains(&"Creator matches".to_owned()));
        assert!(!pairs[0]
            .cautions
            .contains(&"This is not duplicate proof".to_owned()));
    }

    #[test]
    fn exact_filter_skips_malformed_exact_rows() {
        let mut connection = Connection::open_in_memory().expect("db");
        database::initialize(&mut connection).expect("schema");

        let empty_left = insert_file(&connection, "empty-a.package", Some("   "), 10);
        let empty_right = insert_file(&connection, "empty-b.package", Some(""), 10);
        let mismatch_left = insert_file(&connection, "mismatch-a.package", Some("aaa"), 10);
        let mismatch_right = insert_file(&connection, "mismatch-b.package", Some("bbb"), 10);
        let self_pair = insert_file(&connection, "self.package", Some("self"), 10);
        let same_path_left = insert_file(&connection, "same-path-a.package", Some("samepath"), 10);
        let same_path_right = insert_file(&connection, "same-path-b.package", Some("samepath"), 10);
        let empty_path_left =
            insert_file(&connection, "empty-path-a.package", Some("emptypath"), 10);
        let empty_path_right =
            insert_file(&connection, "empty-path-b.package", Some("emptypath"), 10);
        connection
            .execute(
                "UPDATE files SET path = ?1 WHERE id = ?2",
                params![r"c:\mods\samepath\same-path-a.package", same_path_right],
            )
            .expect("update same canonical path");
        connection
            .execute(
                "UPDATE files SET path = '' WHERE id = ?1",
                params![empty_path_left],
            )
            .expect("clear path");

        insert_duplicate_row(&connection, empty_left, empty_right, "exact");
        insert_duplicate_row(&connection, mismatch_left, mismatch_right, "exact");
        insert_duplicate_row(&connection, self_pair, self_pair, "exact");
        insert_duplicate_row(&connection, same_path_left, same_path_right, "exact");
        insert_duplicate_row(&connection, empty_path_left, empty_path_right, "exact");

        let exact_pairs =
            list_duplicate_pairs(&connection, Some("exact".to_owned()), 10).expect("pairs");
        assert!(exact_pairs.is_empty());

        let all_pairs = list_duplicate_pairs(&connection, None, 10).expect("all pairs");
        assert_eq!(all_pairs.len(), 5);
        assert!(all_pairs.iter().all(|pair| !pair.is_duplicate));
        assert!(all_pairs
            .iter()
            .all(|pair| pair.comparison_kind == "unknown"));
        assert!(all_pairs.iter().any(|pair| pair
            .evidence
            .contains(&"Path metadata is incomplete".to_owned())));
    }

    #[test]
    fn stale_filename_row_with_matching_hash_is_still_exact_proof() {
        let mut connection = Connection::open_in_memory().expect("db");
        database::initialize(&mut connection).expect("schema");

        let left = insert_file(&connection, "fixture.package", Some("ABC123"), 10);
        let right = insert_file(&connection, "fixture.package", Some(" abc123 "), 10);
        insert_duplicate_row(&connection, left, right, "filename");

        let exact_pairs =
            list_duplicate_pairs(&connection, Some("exact".to_owned()), 10).expect("pairs");
        assert_eq!(exact_pairs.len(), 1);
        assert!(exact_pairs[0].is_duplicate);
        assert_eq!(exact_pairs[0].comparison_kind, "exact_file");
        assert_eq!(exact_pairs[0].classification_label, "Duplicate");

        let name_pairs =
            list_duplicate_pairs(&connection, Some("filename".to_owned()), 10).expect("pairs");
        assert!(name_pairs.is_empty());
    }

    #[test]
    fn filename_match_is_name_review_not_duplicate_proof() {
        let mut connection = Connection::open_in_memory().expect("db");
        database::initialize(&mut connection).expect("schema");

        insert_file_with_creator(&connection, "hair.package", Some("aaa"), 10, Some("A"));
        insert_file_with_creator(&connection, "hair.package", Some("bbb"), 12, Some("B"));

        rebuild_duplicates(&mut connection).expect("rebuild");
        let pairs =
            list_duplicate_pairs(&connection, Some("filename".to_owned()), 10).expect("pairs");

        assert_eq!(pairs.len(), 1);
        assert!(!pairs[0].is_duplicate);
        assert_eq!(pairs[0].comparison_kind, "name_match_review");
        assert_eq!(pairs[0].classification, "name_match_review");
        assert_eq!(pairs[0].classification_label, "Name match");
        assert!(pairs[0].evidence.contains(&"Same filename".to_owned()));
        assert!(pairs[0]
            .evidence
            .contains(&"Content hash differs".to_owned()));
        assert!(pairs[0].evidence.contains(&"Creator differs".to_owned()));
        assert!(pairs[0]
            .cautions
            .contains(&"This is not duplicate proof".to_owned()));
        assert!(pairs[0]
            .cautions
            .contains(&"Same filename is not same-content proof".to_owned()));
    }

    #[test]
    fn version_token_match_is_version_review_not_duplicate() {
        let mut connection = Connection::open_in_memory().expect("db");
        database::initialize(&mut connection).expect("schema");

        insert_file_with_creator(
            &connection,
            "mod_v1.package",
            Some("111"),
            10,
            Some("Creator"),
        );
        insert_file_with_creator(
            &connection,
            "mod_v2.package",
            Some("222"),
            12,
            Some("Creator"),
        );

        rebuild_duplicates(&mut connection).expect("rebuild");
        let pairs =
            list_duplicate_pairs(&connection, Some("version".to_owned()), 10).expect("pairs");

        assert_eq!(pairs.len(), 1);
        assert!(!pairs[0].is_duplicate);
        assert_eq!(pairs[0].comparison_kind, "version_review");
        assert_eq!(pairs[0].classification, "version_review");
        assert_eq!(pairs[0].classification_label, "Version review");
        assert!(pairs[0].evidence.contains(&"Version clue found".to_owned()));
        assert!(pairs[0].evidence.contains(&"Version differs".to_owned()));
        assert!(pairs[0]
            .cautions
            .contains(&"This may be another release of the same mod".to_owned()));
    }

    #[test]
    fn same_folder_or_same_pack_alone_does_not_create_duplicate_pair() {
        let mut connection = Connection::open_in_memory().expect("db");
        database::initialize(&mut connection).expect("schema");

        let first = insert_file(&connection, "chair.package", Some("111"), 10);
        let second = insert_file(&connection, "table.package", Some("222"), 12);
        connection
            .execute(
                "UPDATE files SET path = ?1 WHERE id = ?2",
                params![r"C:\Mods\Pack\chair.package", first],
            )
            .expect("update first path");
        connection
            .execute(
                "UPDATE files SET path = ?1 WHERE id = ?2",
                params![r"C:\Mods\Pack\table.package", second],
            )
            .expect("update second path");
        connection
            .execute(
                "INSERT INTO bundles (bundle_name, bundle_type, file_count, confidence)
                 VALUES ('Pack', 'package_set', 2, 0.9)",
                [],
            )
            .expect("insert bundle");
        let bundle_id = connection.last_insert_rowid();
        connection
            .execute(
                "UPDATE files SET bundle_id = ?1 WHERE id IN (?2, ?3)",
                params![bundle_id, first, second],
            )
            .expect("update bundle");

        rebuild_duplicates(&mut connection).expect("rebuild");
        let overview = get_duplicate_overview(&connection).expect("overview");

        assert_eq!(overview.total_pairs, 0);
    }

    #[test]
    fn missing_creator_keeps_filename_match_at_manual_review_strength() {
        let mut connection = Connection::open_in_memory().expect("db");
        database::initialize(&mut connection).expect("schema");

        insert_file(&connection, "preset.package", Some("111"), 10);
        insert_file(&connection, "preset.package", Some("222"), 10);

        rebuild_duplicates(&mut connection).expect("rebuild");
        let pairs =
            list_duplicate_pairs(&connection, Some("filename".to_owned()), 10).expect("pairs");

        assert_eq!(pairs.len(), 1);
        assert!(!pairs[0].is_duplicate);
        assert_eq!(pairs[0].classification, "name_match_review");
        assert!(pairs[0].evidence.contains(&"Creator unknown".to_owned()));
        assert!(pairs[0]
            .cautions
            .contains(&"Creator metadata is incomplete".to_owned()));
    }

    #[test]
    fn duplicate_output_labels_do_not_use_possible_duplicate() {
        let mut connection = Connection::open_in_memory().expect("db");
        database::initialize(&mut connection).expect("schema");

        insert_file(&connection, "preset.package", Some("111"), 10);
        insert_file(&connection, "preset.package", Some("222"), 10);

        rebuild_duplicates(&mut connection).expect("rebuild");
        let pairs = list_duplicate_pairs(&connection, None, 10).expect("pairs");

        assert!(pairs.iter().all(|pair| {
            !pair
                .classification_label
                .to_ascii_lowercase()
                .contains("possible duplicate")
        }));
    }

    #[test]
    #[ignore = "large synthetic stress harness; run npm run test:library:stress"]
    fn large_same_name_and_version_review_groups_are_bounded() {
        let mut connection = Connection::open_in_memory().expect("db");
        database::initialize(&mut connection).expect("schema");
        let started_at = Instant::now();

        {
            let transaction = connection.transaction().expect("stress transaction");
            {
                let mut statement = transaction
                    .prepare(
                        "INSERT INTO files (
                            path, filename, extension, hash, size, kind, confidence,
                            source_location, relative_depth, safety_notes, parser_warnings
                         ) VALUES (?1, ?2, '.package', ?3, ?4, 'CAS', 0.8, 'mods', 1, '[]', '[]')",
                    )
                    .expect("prepare stress insert");

                for index in 0..2_500 {
                    statement
                        .execute(params![
                            format!(r"C:\Mods\SameNameGroup\same_name_{index:04}.package"),
                            "same_name_stress.package",
                            format!("same-name-hash-{index:04}"),
                            10_i64 + index as i64,
                        ])
                        .expect("insert same-name stress row");
                }

                for index in 0..2_500 {
                    statement
                        .execute(params![
                            format!(r"C:\Mods\VersionGroup\stress_mod_v{index:04}.package"),
                            format!("stress_mod_v{index:04}.package"),
                            format!("version-hash-{index:04}"),
                            20_i64 + index as i64,
                        ])
                        .expect("insert version stress row");
                }

                statement
                    .execute(params![
                        r"C:\Mods\Exact\exact-a.package",
                        "exact-a.package",
                        "shared-exact-hash",
                        30_i64,
                    ])
                    .expect("insert exact stress left");
                statement
                    .execute(params![
                        r"C:\Mods\Exact\exact-b.package",
                        "exact-b.package",
                        "shared-exact-hash",
                        30_i64,
                    ])
                    .expect("insert exact stress right");
            }
            transaction.commit().expect("commit stress rows");
        }

        let count = rebuild_duplicates(&mut connection).expect("rebuild stress duplicates");
        let overview = get_duplicate_overview(&connection).expect("stress duplicate overview");

        assert_eq!(overview.exact_pairs, 1);
        assert_eq!(overview.filename_pairs, MAX_REVIEW_PAIRS_PER_GROUP as i64);
        assert_eq!(overview.version_pairs, MAX_REVIEW_PAIRS_PER_GROUP as i64);
        assert_eq!(overview.total_pairs, count as i64);

        eprintln!(
            "duplicate_review_group_stress rows=5002 pairs={} filename_pairs={} version_pairs={} elapsed_ms={}",
            overview.total_pairs,
            overview.filename_pairs,
            overview.version_pairs,
            started_at.elapsed().as_millis()
        );
    }
}
