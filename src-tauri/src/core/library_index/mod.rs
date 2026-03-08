use rusqlite::{params, params_from_iter, types::Value, Connection, OptionalExtension};

use crate::{
    error::AppResult,
    models::{
        CategoryOverrideInfo, CreatorLearningInfo, FileDetail, FileInsights, HomeOverview,
        LibraryFacets, LibraryFileRow, LibraryListResponse, LibraryQuery,
    },
    seed::TaxonomySeed,
};

pub fn get_home_overview(connection: &Connection) -> AppResult<HomeOverview> {
    let total_files = scalar(connection, "SELECT COUNT(*) FROM files")?;
    let mods_count = scalar(
        connection,
        "SELECT COUNT(*) FROM files WHERE kind NOT LIKE 'Tray%'",
    )?;
    let tray_count = scalar(
        connection,
        "SELECT COUNT(*) FROM files WHERE kind LIKE 'Tray%'",
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
    let duplicates_count = scalar(connection, "SELECT COUNT(*) FROM duplicates")?;
    let review_count = scalar(connection, "SELECT COUNT(*) FROM review_queue")?;
    let unsafe_count = scalar(
        connection,
        "SELECT COUNT(*) FROM files WHERE safety_notes <> '[]'",
    )?;
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

    Ok(HomeOverview {
        total_files,
        mods_count,
        tray_count,
        script_mods_count,
        creator_count,
        bundles_count,
        duplicates_count,
        review_count,
        unsafe_count,
        last_scan_at,
        read_only_mode: true,
    })
}

pub fn get_library_facets(connection: &Connection, taxonomy: &TaxonomySeed) -> AppResult<LibraryFacets> {
    let creators = string_list(
        connection,
        "SELECT DISTINCT canonical_name FROM creators ORDER BY canonical_name COLLATE NOCASE",
    )?;
    let kinds = string_list(
        connection,
        "SELECT DISTINCT kind FROM files ORDER BY kind COLLATE NOCASE",
    )?;
    let subtypes = string_list(
        connection,
        "SELECT DISTINCT subtype FROM files WHERE subtype IS NOT NULL AND subtype <> '' ORDER BY subtype COLLATE NOCASE",
    )?;
    let sources = string_list(
        connection,
        "SELECT DISTINCT source_location FROM files ORDER BY source_location COLLATE NOCASE",
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

pub fn list_library_files(
    connection: &Connection,
    query: LibraryQuery,
) -> AppResult<LibraryListResponse> {
    let (filters, params) = build_filters(&query);
    let total_sql = format!(
        "SELECT COUNT(*)
         FROM files f
         LEFT JOIN creators c ON f.creator_id = c.id
         LEFT JOIN bundles b ON f.bundle_id = b.id
         WHERE 1 = 1 {filters}"
    );

    let total = connection.query_row(&total_sql, params_from_iter(params.iter()), |row| {
        row.get(0)
    })?;

    let mut row_params = params.clone();
    row_params.push(Value::Integer(query.limit.unwrap_or(100)));
    row_params.push(Value::Integer(query.offset.unwrap_or(0)));

    let rows_sql = format!(
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
            f.relative_depth,
            f.safety_notes
         FROM files f
         LEFT JOIN creators c ON f.creator_id = c.id
         LEFT JOIN bundles b ON f.bundle_id = b.id
         WHERE 1 = 1 {filters}
         ORDER BY f.filename COLLATE NOCASE
         LIMIT ? OFFSET ?"
    );

    let mut statement = connection.prepare(&rows_sql)?;
    let items = statement
        .query_map(params_from_iter(row_params.iter()), |row| {
            Ok(LibraryFileRow {
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
                relative_depth: row.get(13)?,
                safety_notes: parse_string_array(row.get::<_, String>(14)?),
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(LibraryListResponse { total, items })
}

pub fn get_file_detail(connection: &Connection, file_id: i64) -> AppResult<Option<FileDetail>> {
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
             WHERE f.id = ?1",
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
                    relative_depth: row.get(13)?,
                    safety_notes: parse_string_array(row.get::<_, String>(14)?),
                    hash: row.get(15)?,
                    created_at: row.get(16)?,
                    parser_warnings: parse_string_array(row.get::<_, String>(17)?),
                    insights: parse_insights(row.get::<_, String>(18)?),
                    creator_learning: CreatorLearningInfo {
                        locked_by_user: row.get::<_, i64>(20)? != 0,
                        preferred_path: row.get(21)?,
                        learned_aliases: Vec::new(),
                    },
                    category_override: {
                        let kind: Option<String> = row.get(22)?;
                        CategoryOverrideInfo {
                            saved_by_user: kind.is_some(),
                            kind,
                            subtype: row.get(23)?,
                        }
                    },
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
            Ok(Some(detail))
        }
        None => Ok(None),
    }
}

fn build_filters(query: &LibraryQuery) -> (String, Vec<Value>) {
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

    (sql, params)
}

fn scalar(connection: &Connection, sql: &str) -> AppResult<i64> {
    connection
        .query_row(sql, [], |row| row.get(0))
        .map_err(Into::into)
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

fn parse_insights(value: String) -> FileInsights {
    serde_json::from_str(&value).unwrap_or_default()
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
