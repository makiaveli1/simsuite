use std::{collections::HashMap, path::Path, sync::OnceLock};

use regex::Regex;
use rusqlite::{params, Connection, OptionalExtension};

use crate::{
    error::AppResult,
    models::{DuplicateOverview, DuplicatePair},
};

pub fn rebuild_duplicates(connection: &mut Connection) -> AppResult<usize> {
    connection.execute("DELETE FROM duplicates", [])?;

    connection.execute(
        "INSERT INTO duplicates (file_id_a, file_id_b, duplicate_type, detection_method, created_at)
         SELECT a.id, b.id, 'exact', 'sha256', CURRENT_TIMESTAMP
         FROM files a
         JOIN files b ON a.hash = b.hash AND a.id < b.id
         WHERE a.hash IS NOT NULL AND a.hash <> ''",
        params![],
    )?;

    insert_filename_duplicates(connection)?;

    insert_version_duplicates(connection)?;

    let count: i64 =
        connection.query_row("SELECT COUNT(*) FROM duplicates", [], |row| row.get(0))?;
    Ok(count as usize)
}

pub fn get_duplicate_overview(connection: &Connection) -> AppResult<DuplicateOverview> {
    Ok(DuplicateOverview {
        total_pairs: scalar(connection, "SELECT COUNT(*) FROM duplicates")?,
        exact_pairs: scalar(
            connection,
            "SELECT COUNT(*) FROM duplicates WHERE duplicate_type = 'exact'",
        )?,
        filename_pairs: scalar(
            connection,
            "SELECT COUNT(*) FROM duplicates WHERE duplicate_type = 'filename'",
        )?,
        version_pairs: scalar(
            connection,
            "SELECT COUNT(*) FROM duplicates WHERE duplicate_type = 'version'",
        )?,
    })
}

pub fn list_duplicate_pairs(
    connection: &Connection,
    duplicate_type: Option<String>,
    limit: i64,
) -> AppResult<Vec<DuplicatePair>> {
    let limit = limit.max(1);
    let items = if let Some(duplicate_type) = duplicate_type.filter(|value| !value.is_empty()) {
        let mut statement = connection.prepare(
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
             WHERE d.duplicate_type = ?1
             ORDER BY d.duplicate_type, a.filename COLLATE NOCASE, b.filename COLLATE NOCASE
             LIMIT ?2",
        )?;

        let rows = statement
            .query_map(params![duplicate_type, limit], map_duplicate_pair)?
            .collect::<Result<Vec<_>, _>>()?;
        rows
    } else {
        let mut statement = connection.prepare(
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
                CASE d.duplicate_type
                    WHEN 'exact' THEN 0
                    WHEN 'version' THEN 1
                    ELSE 2
                END,
                a.filename COLLATE NOCASE,
                b.filename COLLATE NOCASE
             LIMIT ?1",
        )?;

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
    let primary_filename: String = row.get(4)?;
    let primary_creator: Option<String> = row.get(6)?;
    let primary_hash: Option<String> = row.get(7)?;
    let primary_size: i64 = row.get(9)?;
    let secondary_filename: String = row.get(11)?;
    let secondary_creator: Option<String> = row.get(13)?;
    let secondary_hash: Option<String> = row.get(14)?;
    let secondary_size: i64 = row.get(16)?;
    let intelligence = classify_duplicate_pair(
        &duplicate_type,
        &detection_method,
        &primary_filename,
        primary_creator.as_deref(),
        primary_hash.as_deref(),
        primary_size,
        &secondary_filename,
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
        primary_file_id: row.get(3)?,
        primary_filename,
        primary_path: row.get(5)?,
        primary_creator,
        primary_hash,
        primary_modified_at: row.get(8)?,
        primary_size,
        secondary_file_id: row.get(10)?,
        secondary_filename,
        secondary_path: row.get(12)?,
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
    primary_filename: &str,
    primary_creator: Option<&str>,
    primary_hash: Option<&str>,
    primary_size: i64,
    secondary_filename: &str,
    secondary_creator: Option<&str>,
    secondary_hash: Option<&str>,
    secondary_size: i64,
) -> DuplicateIntelligence {
    let same_hash = hashes_match(primary_hash, secondary_hash);
    let same_filename = primary_filename.eq_ignore_ascii_case(secondary_filename);
    let primary_versions = version_tokens_from_filename(primary_filename);
    let secondary_versions = version_tokens_from_filename(secondary_filename);
    let has_version_clue = !primary_versions.is_empty() || !secondary_versions.is_empty();
    let version_differs = !primary_versions.is_empty()
        && !secondary_versions.is_empty()
        && primary_versions != secondary_versions;

    let (is_duplicate, comparison_kind, classification, classification_label, confidence_label) =
        if duplicate_type.eq_ignore_ascii_case("exact") && same_hash {
            (
                true,
                "exact_file",
                "duplicate",
                "Duplicate",
                "Same file contents",
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
    if same_hash {
        evidence.push("Same file contents".to_owned());
    } else if primary_hash.filter(|value| !value.is_empty()).is_some()
        && secondary_hash.filter(|value| !value.is_empty()).is_some()
    {
        evidence.push("Content hash differs".to_owned());
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
    match (primary_hash, secondary_hash) {
        (Some(left), Some(right)) => !left.is_empty() && left == right,
        _ => false,
    }
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

fn insert_version_duplicates(connection: &mut Connection) -> AppResult<()> {
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

    for files in grouped.values().filter(|items| items.len() > 1) {
        for left in 0..files.len() {
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

                connection.execute(
                    "INSERT INTO duplicates (file_id_a, file_id_b, duplicate_type, detection_method, created_at)
                     VALUES (?1, ?2, 'version', 'version_token_strip', CURRENT_TIMESTAMP)",
                    params![pair_key.0, pair_key.1],
                )?;
            }
        }
    }

    Ok(())
}

fn insert_filename_duplicates(connection: &mut Connection) -> AppResult<()> {
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

    for files in grouped.values().filter(|items| items.len() > 1) {
        for left in 0..files.len() {
            for right in (left + 1)..files.len() {
                let (left_id, left_hash) = &files[left];
                let (right_id, right_hash) = &files[right];
                if left_hash == right_hash {
                    continue;
                }

                let pair_key = ordered_pair(*left_id, *right_id);
                connection.execute(
                    "INSERT INTO duplicates (file_id_a, file_id_b, duplicate_type, detection_method, created_at)
                     VALUES (?1, ?2, 'filename', 'filename_match', CURRENT_TIMESTAMP)",
                    params![pair_key.0, pair_key.1],
                )?;
            }
        }
    }

    Ok(())
}

fn load_existing_pairs(connection: &Connection) -> AppResult<Vec<(i64, i64)>> {
    let mut statement = connection.prepare("SELECT file_id_a, file_id_b FROM duplicates")?;
    let rows = statement
        .query_map([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)))?
        .collect::<Result<Vec<_>, _>>()?;
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
    fn filename_duplicates_match_case_insensitively_without_readding_exact_pairs() {
        let mut connection = Connection::open_in_memory().expect("db");
        database::initialize(&mut connection).expect("schema");

        insert_file(&connection, "Hair.package", Some("aaa"), 10);
        insert_file(&connection, "hair.package", Some("bbb"), 10);
        insert_file(&connection, "HAIR.package", Some("bbb"), 10);

        rebuild_duplicates(&mut connection).expect("rebuild");
        let overview = get_duplicate_overview(&connection).expect("overview");

        assert_eq!(overview.exact_pairs, 1);
        assert_eq!(overview.filename_pairs, 2);
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
}
