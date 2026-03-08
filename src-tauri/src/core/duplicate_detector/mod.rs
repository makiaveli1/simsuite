use std::{collections::HashMap, path::Path};

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

    connection.execute(
        "INSERT INTO duplicates (file_id_a, file_id_b, duplicate_type, detection_method, created_at)
         SELECT a.id, b.id, 'filename', 'filename_match', CURRENT_TIMESTAMP
         FROM files a
         JOIN files b ON LOWER(a.filename) = LOWER(b.filename) AND a.id < b.id
         WHERE COALESCE(a.hash, '') <> COALESCE(b.hash, '')",
        params![],
    )?;

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
    Ok(DuplicatePair {
        id: row.get(0)?,
        duplicate_type: row.get(1)?,
        detection_method: row.get(2)?,
        primary_file_id: row.get(3)?,
        primary_filename: row.get(4)?,
        primary_path: row.get(5)?,
        primary_creator: row.get(6)?,
        primary_hash: row.get(7)?,
        primary_modified_at: row.get(8)?,
        primary_size: row.get(9)?,
        secondary_file_id: row.get(10)?,
        secondary_filename: row.get(11)?,
        secondary_path: row.get(12)?,
        secondary_creator: row.get(13)?,
        secondary_hash: row.get(14)?,
        secondary_modified_at: row.get(15)?,
        secondary_size: row.get(16)?,
    })
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
        connection
            .execute(
                "INSERT INTO files (
                    path, filename, extension, hash, size, kind, confidence, source_location,
                    relative_depth, safety_notes, parser_warnings
                 ) VALUES (?1, ?2, ?3, ?4, ?5, 'CAS', 0.8, 'mods', 0, '[]', '[]')",
                params![
                    format!(r"C:\Mods\{}\{filename}", hash.unwrap_or("no-hash")),
                    filename,
                    Path::new(filename)
                        .extension()
                        .map(|value| format!(".{}", value.to_string_lossy().to_lowercase()))
                        .unwrap_or_default(),
                    hash,
                    size
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
}
