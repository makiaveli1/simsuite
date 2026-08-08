#![cfg(test)]

use std::{
    fs,
    path::{Path, PathBuf},
    time::Instant,
};

use rusqlite::{params, Connection, OptionalExtension, Transaction};

use crate::{
    error::{AppError, AppResult},
    models::FileInsights,
};

#[derive(Debug, Clone)]
struct SearchFixture {
    id: i64,
    filename: String,
    path: String,
    creator: Option<String>,
    creator_aliases: Vec<String>,
    kind: String,
    subtype: Option<String>,
    insights: FileInsights,
}

#[derive(Debug, Clone)]
struct SearchDocument {
    filename: String,
    creator: String,
    aliases: String,
    kind_subtype: String,
    embedded_names: String,
    family_hints: String,
    resource_summary: String,
    script_namespaces: String,
}

#[derive(Debug, Clone, Copy)]
struct QueryCase {
    query: &'static str,
    expected_id: i64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct RetrievalScore {
    recall_at_five: f64,
    mean_reciprocal_rank: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum QueryClass {
    Typo,
    Partial,
    VagueLexical,
    Short,
    Ambiguous,
}

#[derive(Debug, Clone, Copy)]
struct HoldoutCase {
    query: &'static str,
    expected_ids: &'static [i64],
    class: QueryClass,
}

#[derive(Debug, Clone, PartialEq)]
struct FuzzySearchDiagnostics {
    ids: Vec<i64>,
    candidate_count: usize,
}

const TRIGRAM_CANDIDATE_LIMIT: usize = 80;
const STRICT_CANDIDATE_LIMIT: usize = 20;
const FUZZY_MIN_SCORE: f64 = 0.72;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ProductionSearchShape {
    unicode_fts: bool,
    trigram_fts: bool,
}

impl ProductionSearchShape {
    const SOURCE_ONLY: Self = Self {
        unicode_fts: false,
        trigram_fts: false,
    };
    const UNICODE_ONLY: Self = Self {
        unicode_fts: true,
        trigram_fts: false,
    };
    const TRIGRAM_ONLY: Self = Self {
        unicode_fts: false,
        trigram_fts: true,
    };
    const BOTH: Self = Self {
        unicode_fts: true,
        trigram_fts: true,
    };
}

fn join_values(values: &[String]) -> String {
    values
        .iter()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

fn search_document(fixture: &SearchFixture) -> SearchDocument {
    SearchDocument {
        filename: fixture.filename.clone(),
        creator: fixture.creator.clone().unwrap_or_default(),
        aliases: join_values(&fixture.creator_aliases),
        kind_subtype: [
            fixture.kind.trim(),
            fixture.subtype.as_deref().unwrap_or_default().trim(),
        ]
        .into_iter()
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>()
        .join(" "),
        embedded_names: join_values(&fixture.insights.embedded_names),
        family_hints: join_values(&fixture.insights.family_hints),
        resource_summary: join_values(&fixture.insights.resource_summary),
        script_namespaces: join_values(&fixture.insights.script_namespaces),
    }
}

fn flattened_search_document(document: &SearchDocument) -> String {
    [
        document.filename.as_str(),
        document.creator.as_str(),
        document.aliases.as_str(),
        document.kind_subtype.as_str(),
        document.embedded_names.as_str(),
        document.family_hints.as_str(),
        document.resource_summary.as_str(),
        document.script_namespaces.as_str(),
    ]
    .into_iter()
    .filter(|value| !value.trim().is_empty())
    .collect::<Vec<_>>()
    .join(" | ")
}

fn create_search_schema(connection: &Connection) -> AppResult<()> {
    connection.execute_batch(
        "CREATE TABLE library_current_like (
            id INTEGER PRIMARY KEY,
            filename TEXT NOT NULL,
            path TEXT NOT NULL,
            creator TEXT NOT NULL,
            subtype TEXT NOT NULL
        );
        CREATE VIRTUAL TABLE library_smart_search_fts USING fts5(
            filename,
            creator,
            aliases,
            kind_subtype,
            embedded_names,
            family_hints,
            resource_summary,
            script_namespaces,
            tokenize = 'unicode61 remove_diacritics 2'
        );
        CREATE VIRTUAL TABLE library_smart_search_trigram USING fts5(
            search_text,
            tokenize = 'trigram'
        );",
    )?;
    Ok(())
}

fn index_fixtures(connection: &mut Connection, fixtures: &[SearchFixture]) -> AppResult<()> {
    create_search_schema(connection)?;
    let transaction = connection.transaction()?;
    {
        let mut current_statement = transaction.prepare(
            "INSERT INTO library_current_like (id, filename, path, creator, subtype)
             VALUES (?1, ?2, ?3, ?4, ?5)",
        )?;
        let mut fts_statement = transaction.prepare(
            "INSERT INTO library_smart_search_fts (
                rowid,
                filename,
                creator,
                aliases,
                kind_subtype,
                embedded_names,
                family_hints,
                resource_summary,
                script_namespaces
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        )?;
        let mut trigram_statement = transaction.prepare(
            "INSERT INTO library_smart_search_trigram (rowid, search_text)
             VALUES (?1, ?2)",
        )?;
        for fixture in fixtures {
            current_statement.execute(params![
                fixture.id,
                fixture.filename,
                fixture.path,
                fixture.creator.as_deref().unwrap_or_default(),
                fixture.subtype.as_deref().unwrap_or_default(),
            ])?;

            let document = search_document(fixture);
            let flattened = flattened_search_document(&document);
            fts_statement.execute(params![
                fixture.id,
                document.filename,
                document.creator,
                document.aliases,
                document.kind_subtype,
                document.embedded_names,
                document.family_hints,
                document.resource_summary,
                document.script_namespaces,
            ])?;
            trigram_statement.execute(params![fixture.id, flattened])?;
        }
    }
    transaction.commit()?;
    Ok(())
}

fn index_trigram_only(connection: &mut Connection, fixtures: &[SearchFixture]) -> AppResult<()> {
    connection.execute_batch(
        "CREATE VIRTUAL TABLE library_smart_search_trigram_only USING fts5(
            search_text,
            tokenize = 'trigram'
        );",
    )?;
    let transaction = connection.transaction()?;
    {
        let mut statement = transaction.prepare(
            "INSERT INTO library_smart_search_trigram_only (rowid, search_text)
             VALUES (?1, ?2)",
        )?;
        for fixture in fixtures {
            let document = search_document(fixture);
            statement.execute(params![fixture.id, flattened_search_document(&document)])?;
        }
    }
    transaction.commit()?;
    Ok(())
}

fn create_production_shape_schema(
    connection: &Connection,
    shape: ProductionSearchShape,
) -> AppResult<()> {
    connection.execute_batch(
        "CREATE TABLE production_creators (
            id INTEGER PRIMARY KEY,
            canonical_name TEXT NOT NULL UNIQUE
        );
        CREATE TABLE production_user_creator_aliases (
            creator_id INTEGER NOT NULL,
            alias_name TEXT NOT NULL UNIQUE,
            FOREIGN KEY (creator_id) REFERENCES production_creators(id) ON DELETE CASCADE
        );
        CREATE INDEX production_alias_creator_idx
            ON production_user_creator_aliases(creator_id);
        CREATE TABLE production_files (
            id INTEGER PRIMARY KEY,
            path TEXT NOT NULL,
            filename TEXT NOT NULL,
            source_location TEXT NOT NULL,
            creator_id INTEGER,
            kind TEXT NOT NULL,
            subtype TEXT,
            insights TEXT NOT NULL DEFAULT '{}',
            FOREIGN KEY (creator_id) REFERENCES production_creators(id)
        );
        CREATE INDEX production_files_source_idx
            ON production_files(source_location);",
    )?;
    if shape.unicode_fts {
        connection.execute_batch(
            "CREATE VIRTUAL TABLE production_search_fts USING fts5(
                filename,
                creator,
                aliases,
                kind_subtype,
                embedded_names,
                family_hints,
                resource_summary,
                script_namespaces,
                tokenize = 'unicode61 remove_diacritics 2'
            );",
        )?;
    }
    if shape.trigram_fts {
        connection.execute_batch(
            "CREATE VIRTUAL TABLE production_search_trigram USING fts5(
                search_text,
                tokenize = 'trigram'
            );",
        )?;
    }
    Ok(())
}

fn ensure_production_creator(
    transaction: &Transaction<'_>,
    creator_name: &str,
) -> AppResult<i64> {
    transaction.execute(
        "INSERT OR IGNORE INTO production_creators(canonical_name) VALUES (?1)",
        params![creator_name],
    )?;
    transaction
        .query_row(
            "SELECT id FROM production_creators WHERE canonical_name = ?1",
            params![creator_name],
            |row| row.get(0),
        )
        .map_err(Into::into)
}

fn insert_production_source_fixture(
    transaction: &Transaction<'_>,
    fixture: &SearchFixture,
    source_location: &str,
) -> AppResult<()> {
    let creator_id = match fixture.creator.as_deref().filter(|name| !name.trim().is_empty()) {
        Some(creator_name) => {
            let creator_id = ensure_production_creator(transaction, creator_name)?;
            for alias in &fixture.creator_aliases {
                let alias = alias.trim();
                if alias.is_empty() {
                    continue;
                }
                transaction.execute(
                    "INSERT INTO production_user_creator_aliases(creator_id, alias_name)
                     VALUES (?1, ?2)
                     ON CONFLICT(alias_name) DO UPDATE SET creator_id = excluded.creator_id",
                    params![creator_id, alias],
                )?;
            }
            Some(creator_id)
        }
        None => None,
    };

    transaction.execute(
        "INSERT INTO production_files(
            id, path, filename, source_location, creator_id, kind, subtype, insights
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            fixture.id,
            fixture.path,
            fixture.filename,
            source_location,
            creator_id,
            fixture.kind,
            fixture.subtype,
            serde_json::to_string(&fixture.insights)?,
        ],
    )?;
    Ok(())
}

fn seed_production_source(
    connection: &mut Connection,
    fixtures: &[SearchFixture],
) -> AppResult<()> {
    let transaction = connection.transaction()?;
    for fixture in fixtures {
        insert_production_source_fixture(&transaction, fixture, "mods")?;
    }
    transaction.commit()?;
    Ok(())
}

fn production_search_document(
    connection: &Connection,
    file_id: i64,
) -> AppResult<Option<SearchDocument>> {
    let row = connection
        .query_row(
            "SELECT
                f.filename,
                COALESCE(c.canonical_name, ''),
                f.creator_id,
                f.kind,
                f.subtype,
                f.insights
             FROM production_files f
             LEFT JOIN production_creators c ON c.id = f.creator_id
             WHERE f.id = ?1
               AND f.source_location IN ('mods', 'tray')",
            params![file_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<i64>>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, String>(5)?,
                ))
            },
        )
        .optional()?;
    let Some((filename, creator, creator_id, kind, subtype, insights_json)) = row else {
        return Ok(None);
    };

    let aliases = if let Some(creator_id) = creator_id {
        let mut statement = connection.prepare(
            "SELECT alias_name
             FROM production_user_creator_aliases
             WHERE creator_id = ?1
             ORDER BY alias_name COLLATE NOCASE",
        )?;
        let aliases = statement
            .query_map(params![creator_id], |row| row.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        aliases
    } else {
        Vec::new()
    };
    let insights: FileInsights = serde_json::from_str(&insights_json).unwrap_or_default();

    Ok(Some(SearchDocument {
        filename,
        creator,
        aliases: join_values(&aliases),
        kind_subtype: [kind.trim(), subtype.as_deref().unwrap_or_default().trim()]
            .into_iter()
            .filter(|value| !value.is_empty())
            .collect::<Vec<_>>()
            .join(" "),
        embedded_names: join_values(&insights.embedded_names),
        family_hints: join_values(&insights.family_hints),
        resource_summary: join_values(&insights.resource_summary),
        script_namespaces: join_values(&insights.script_namespaces),
    }))
}

fn delete_production_search_row(
    transaction: &Transaction<'_>,
    file_id: i64,
    shape: ProductionSearchShape,
) -> AppResult<()> {
    if shape.unicode_fts {
        transaction.execute(
            "DELETE FROM production_search_fts WHERE rowid = ?1",
            params![file_id],
        )?;
    }
    if shape.trigram_fts {
        transaction.execute(
            "DELETE FROM production_search_trigram WHERE rowid = ?1",
            params![file_id],
        )?;
    }
    Ok(())
}

fn insert_production_search_document(
    transaction: &Transaction<'_>,
    file_id: i64,
    document: &SearchDocument,
    shape: ProductionSearchShape,
) -> AppResult<()> {
    if shape.unicode_fts {
        transaction.execute(
            "INSERT INTO production_search_fts(
                rowid, filename, creator, aliases, kind_subtype,
                embedded_names, family_hints, resource_summary, script_namespaces
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                file_id,
                document.filename,
                document.creator,
                document.aliases,
                document.kind_subtype,
                document.embedded_names,
                document.family_hints,
                document.resource_summary,
                document.script_namespaces,
            ],
        )?;
    }
    if shape.trigram_fts {
        transaction.execute(
            "INSERT INTO production_search_trigram(rowid, search_text) VALUES (?1, ?2)",
            params![file_id, flattened_search_document(document)],
        )?;
    }
    Ok(())
}

fn refresh_production_search_file_in_transaction(
    transaction: &Transaction<'_>,
    file_id: i64,
    shape: ProductionSearchShape,
) -> AppResult<()> {
    delete_production_search_row(transaction, file_id, shape)?;
    if let Some(document) = production_search_document(transaction, file_id)? {
        insert_production_search_document(transaction, file_id, &document, shape)?;
    }
    Ok(())
}

fn refresh_production_search_files(
    connection: &mut Connection,
    file_ids: &[i64],
    shape: ProductionSearchShape,
) -> AppResult<()> {
    let transaction = connection.transaction()?;
    for file_id in file_ids {
        refresh_production_search_file_in_transaction(&transaction, *file_id, shape)?;
    }
    transaction.commit()?;
    Ok(())
}

fn refresh_production_search_creators(
    connection: &mut Connection,
    creator_ids: &[i64],
    shape: ProductionSearchShape,
) -> AppResult<usize> {
    let transaction = connection.transaction()?;
    let mut unique_creator_ids = creator_ids.to_vec();
    unique_creator_ids.sort_unstable();
    unique_creator_ids.dedup();
    let mut refreshed = 0usize;
    for creator_id in unique_creator_ids {
        let file_ids = {
            let mut statement = transaction.prepare(
                "SELECT id
                 FROM production_files
                 WHERE creator_id = ?1
                   AND source_location IN ('mods', 'tray')
                 ORDER BY id",
            )?;
            let file_ids = statement
                .query_map(params![creator_id], |row| row.get::<_, i64>(0))?
                .collect::<Result<Vec<_>, _>>()?;
            file_ids
        };
        for file_id in &file_ids {
            refresh_production_search_file_in_transaction(&transaction, *file_id, shape)?;
        }
        refreshed += file_ids.len();
    }
    transaction.commit()?;
    Ok(refreshed)
}

fn refresh_production_search_creator(
    connection: &mut Connection,
    creator_id: i64,
    shape: ProductionSearchShape,
) -> AppResult<usize> {
    refresh_production_search_creators(connection, &[creator_id], shape)
}

fn rebuild_production_search_in_transaction(
    transaction: &Transaction<'_>,
    shape: ProductionSearchShape,
    interrupt_after: Option<usize>,
) -> AppResult<usize> {
    if shape.unicode_fts {
        transaction.execute("DELETE FROM production_search_fts", [])?;
    }
    if shape.trigram_fts {
        transaction.execute("DELETE FROM production_search_trigram", [])?;
    }

    let file_ids = {
        let mut statement = transaction.prepare(
            "SELECT id
             FROM production_files
             WHERE source_location IN ('mods', 'tray')
             ORDER BY id",
        )?;
        let file_ids = statement
            .query_map([], |row| row.get::<_, i64>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        file_ids
    };

    for (index, file_id) in file_ids.iter().enumerate() {
        refresh_production_search_file_in_transaction(transaction, *file_id, shape)?;
        if interrupt_after.is_some_and(|limit| index + 1 == limit) {
            return Err(AppError::Message(
                "synthetic search-index rebuild interruption".to_owned(),
            ));
        }
    }
    Ok(file_ids.len())
}

fn rebuild_production_search(
    connection: &mut Connection,
    shape: ProductionSearchShape,
) -> AppResult<usize> {
    let transaction = connection.transaction()?;
    let count = rebuild_production_search_in_transaction(&transaction, shape, None)?;
    transaction.commit()?;
    Ok(count)
}

fn rebuild_production_search_with_interrupt(
    connection: &mut Connection,
    shape: ProductionSearchShape,
    interrupt_after: usize,
) -> AppResult<usize> {
    let transaction = connection.transaction()?;
    let result = rebuild_production_search_in_transaction(
        &transaction,
        shape,
        Some(interrupt_after),
    );
    match result {
        Ok(count) => {
            transaction.commit()?;
            Ok(count)
        }
        Err(error) => Err(error),
    }
}

fn production_fts_search(
    connection: &Connection,
    query: &str,
    limit: usize,
) -> AppResult<Vec<i64>> {
    let Some(query) = normalized_fts_query(query) else {
        return Ok(Vec::new());
    };
    let mut statement = connection.prepare(
        "SELECT rowid
         FROM production_search_fts
         WHERE production_search_fts MATCH ?1
         ORDER BY bm25(
             production_search_fts,
             8.0, 7.0, 6.0, 4.0, 5.0, 5.0, 3.0, 4.0
         ) ASC, rowid ASC
         LIMIT ?2",
    )?;
    let ids = statement
        .query_map(params![query, limit as i64], |row| row.get::<_, i64>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(ids)
}

fn production_trigram_candidate_rows(
    connection: &Connection,
    query: &str,
    limit: usize,
) -> AppResult<Vec<(i64, String)>> {
    let Some(query) = normalized_trigram_query(query) else {
        return Ok(Vec::new());
    };
    let mut statement = connection.prepare(
        "SELECT rowid, search_text
         FROM production_search_trigram
         WHERE production_search_trigram MATCH ?1
         ORDER BY bm25(production_search_trigram) ASC, rowid ASC
         LIMIT ?2",
    )?;
    let rows = statement
        .query_map(params![query, limit as i64], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

fn production_fuzzy_search(
    connection: &Connection,
    query: &str,
    limit: usize,
) -> AppResult<FuzzySearchDiagnostics> {
    if benchmark_authority_query(query) {
        return Ok(FuzzySearchDiagnostics {
            ids: Vec::new(),
            candidate_count: 0,
        });
    }
    let strict_ids = production_fts_search(connection, query, STRICT_CANDIDATE_LIMIT)?;
    let mut candidates =
        production_trigram_candidate_rows(connection, query, TRIGRAM_CANDIDATE_LIMIT)?;
    for strict_id in &strict_ids {
        if candidates.iter().any(|(id, _)| id == strict_id) {
            continue;
        }
        let document = connection.query_row(
            "SELECT search_text FROM production_search_trigram WHERE rowid = ?1",
            params![strict_id],
            |row| row.get::<_, String>(0),
        )?;
        candidates.push((*strict_id, document));
    }
    let candidate_count = candidates.len();
    let mut ranked = candidates
        .into_iter()
        .filter_map(|(id, document)| {
            let strict_rank = strict_ids.iter().position(|strict_id| *strict_id == id);
            let fuzzy_score = document_similarity(query, &document);
            if strict_rank.is_none() && fuzzy_score < FUZZY_MIN_SCORE {
                return None;
            }
            let score = strict_rank
                .map(|rank| 2.0 - (rank as f64 * 0.01))
                .unwrap_or(fuzzy_score);
            Some((id, score))
        })
        .collect::<Vec<_>>();
    ranked.sort_by(|(left_id, left_score), (right_id, right_score)| {
        right_score
            .total_cmp(left_score)
            .then_with(|| left_id.cmp(right_id))
    });
    ranked.dedup_by_key(|(id, _)| *id);
    Ok(FuzzySearchDiagnostics {
        ids: ranked.into_iter().take(limit).map(|(id, _)| id).collect(),
        candidate_count,
    })
}

fn build_production_shape_database(
    path: &Path,
    fixtures: &[SearchFixture],
    shape: ProductionSearchShape,
) -> AppResult<(u64, u128)> {
    if path.exists() {
        fs::remove_file(path)?;
    }
    let mut connection = Connection::open(path)?;
    create_production_shape_schema(&connection, shape)?;
    seed_production_source(&mut connection, fixtures)?;
    let started = Instant::now();
    if shape != ProductionSearchShape::SOURCE_ONLY {
        rebuild_production_search(&mut connection, shape)?;
    }
    let backfill_ms = started.elapsed().as_millis();
    connection.execute_batch("VACUUM")?;
    drop(connection);
    Ok((fs::metadata(path)?.len(), backfill_ms))
}

fn production_shape_path(root: &Path, name: &str) -> PathBuf {
    root.join(format!("{name}.sqlite"))
}

fn create_contentless_search_schema(connection: &Connection) -> AppResult<()> {
    connection.execute_batch(
        "CREATE VIRTUAL TABLE production_contentless_fts USING fts5(
            filename,
            creator,
            aliases,
            kind_subtype,
            embedded_names,
            family_hints,
            resource_summary,
            script_namespaces,
            tokenize = 'unicode61 remove_diacritics 2',
            content = '',
            contentless_delete = 1
        );
        CREATE VIRTUAL TABLE production_contentless_trigram USING fts5(
            search_text,
            tokenize = 'trigram',
            content = '',
            contentless_delete = 1
        );",
    )?;
    Ok(())
}

fn delete_contentless_search_row(
    transaction: &Transaction<'_>,
    file_id: i64,
) -> AppResult<()> {
    transaction.execute(
        "DELETE FROM production_contentless_fts WHERE rowid = ?1",
        params![file_id],
    )?;
    transaction.execute(
        "DELETE FROM production_contentless_trigram WHERE rowid = ?1",
        params![file_id],
    )?;
    Ok(())
}

fn insert_contentless_search_document(
    transaction: &Transaction<'_>,
    file_id: i64,
    document: &SearchDocument,
) -> AppResult<()> {
    transaction.execute(
        "INSERT INTO production_contentless_fts(
            rowid, filename, creator, aliases, kind_subtype,
            embedded_names, family_hints, resource_summary, script_namespaces
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            file_id,
            document.filename,
            document.creator,
            document.aliases,
            document.kind_subtype,
            document.embedded_names,
            document.family_hints,
            document.resource_summary,
            document.script_namespaces,
        ],
    )?;
    transaction.execute(
        "INSERT INTO production_contentless_trigram(rowid, search_text) VALUES (?1, ?2)",
        params![file_id, flattened_search_document(document)],
    )?;
    Ok(())
}

fn refresh_contentless_search_file_in_transaction(
    transaction: &Transaction<'_>,
    file_id: i64,
) -> AppResult<()> {
    delete_contentless_search_row(transaction, file_id)?;
    if let Some(document) = production_search_document(transaction, file_id)? {
        insert_contentless_search_document(transaction, file_id, &document)?;
    }
    Ok(())
}

fn rebuild_contentless_search_in_transaction(
    transaction: &Transaction<'_>,
    interrupt_after: Option<usize>,
) -> AppResult<usize> {
    transaction.execute("DELETE FROM production_contentless_fts", [])?;
    transaction.execute("DELETE FROM production_contentless_trigram", [])?;
    let file_ids = {
        let mut statement = transaction.prepare(
            "SELECT id
             FROM production_files
             WHERE source_location IN ('mods', 'tray')
             ORDER BY id",
        )?;
        let file_ids = statement
            .query_map([], |row| row.get::<_, i64>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        file_ids
    };
    for (index, file_id) in file_ids.iter().enumerate() {
        refresh_contentless_search_file_in_transaction(transaction, *file_id)?;
        if interrupt_after.is_some_and(|limit| index + 1 == limit) {
            return Err(AppError::Message(
                "synthetic contentless search-index rebuild interruption".to_owned(),
            ));
        }
    }
    Ok(file_ids.len())
}

fn rebuild_contentless_search(connection: &mut Connection) -> AppResult<usize> {
    let transaction = connection.transaction()?;
    let count = rebuild_contentless_search_in_transaction(&transaction, None)?;
    transaction.commit()?;
    Ok(count)
}

fn rebuild_contentless_search_with_interrupt(
    connection: &mut Connection,
    interrupt_after: usize,
) -> AppResult<usize> {
    let transaction = connection.transaction()?;
    let result = rebuild_contentless_search_in_transaction(
        &transaction,
        Some(interrupt_after),
    );
    match result {
        Ok(count) => {
            transaction.commit()?;
            Ok(count)
        }
        Err(error) => Err(error),
    }
}

fn contentless_fts_search(
    connection: &Connection,
    query: &str,
    limit: usize,
) -> AppResult<Vec<i64>> {
    let Some(query) = normalized_fts_query(query) else {
        return Ok(Vec::new());
    };
    let mut statement = connection.prepare(
        "SELECT rowid
         FROM production_contentless_fts
         WHERE production_contentless_fts MATCH ?1
         ORDER BY bm25(
             production_contentless_fts,
             8.0, 7.0, 6.0, 4.0, 5.0, 5.0, 3.0, 4.0
         ) ASC, rowid ASC
         LIMIT ?2",
    )?;
    let ids = statement
        .query_map(params![query, limit as i64], |row| row.get::<_, i64>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(ids)
}

fn contentless_trigram_candidate_ids(
    connection: &Connection,
    query: &str,
    limit: usize,
) -> AppResult<Vec<i64>> {
    let Some(query) = normalized_trigram_query(query) else {
        return Ok(Vec::new());
    };
    let mut statement = connection.prepare(
        "SELECT rowid
         FROM production_contentless_trigram
         WHERE production_contentless_trigram MATCH ?1
         ORDER BY bm25(production_contentless_trigram) ASC, rowid ASC
         LIMIT ?2",
    )?;
    let ids = statement
        .query_map(params![query, limit as i64], |row| row.get::<_, i64>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(ids)
}

fn contentless_fuzzy_search(
    connection: &Connection,
    query: &str,
    limit: usize,
) -> AppResult<FuzzySearchDiagnostics> {
    if benchmark_authority_query(query) {
        return Ok(FuzzySearchDiagnostics {
            ids: Vec::new(),
            candidate_count: 0,
        });
    }
    let strict_ids = contentless_fts_search(connection, query, STRICT_CANDIDATE_LIMIT)?;
    let mut candidate_ids =
        contentless_trigram_candidate_ids(connection, query, TRIGRAM_CANDIDATE_LIMIT)?;
    for strict_id in &strict_ids {
        if !candidate_ids.contains(strict_id) {
            candidate_ids.push(*strict_id);
        }
    }
    let candidate_count = candidate_ids.len();
    let mut ranked = Vec::new();
    for id in candidate_ids {
        let strict_rank = strict_ids.iter().position(|strict_id| *strict_id == id);
        let Some(document) = production_search_document(connection, id)? else {
            continue;
        };
        let fuzzy_score = document_similarity(query, &flattened_search_document(&document));
        if strict_rank.is_none() && fuzzy_score < FUZZY_MIN_SCORE {
            continue;
        }
        let score = strict_rank
            .map(|rank| 2.0 - (rank as f64 * 0.01))
            .unwrap_or(fuzzy_score);
        ranked.push((id, score));
    }
    ranked.sort_by(|(left_id, left_score), (right_id, right_score)| {
        right_score
            .total_cmp(left_score)
            .then_with(|| left_id.cmp(right_id))
    });
    ranked.dedup_by_key(|(id, _)| *id);
    Ok(FuzzySearchDiagnostics {
        ids: ranked.into_iter().take(limit).map(|(id, _)| id).collect(),
        candidate_count,
    })
}

fn build_contentless_production_database(
    path: &Path,
    fixtures: &[SearchFixture],
) -> AppResult<(u64, u128)> {
    if path.exists() {
        fs::remove_file(path)?;
    }
    let mut connection = Connection::open(path)?;
    create_production_shape_schema(&connection, ProductionSearchShape::SOURCE_ONLY)?;
    create_contentless_search_schema(&connection)?;
    seed_production_source(&mut connection, fixtures)?;
    let started = Instant::now();
    rebuild_contentless_search(&mut connection)?;
    let backfill_ms = started.elapsed().as_millis();
    connection.execute_batch("VACUUM")?;
    drop(connection);
    Ok((fs::metadata(path)?.len(), backfill_ms))
}

fn current_like_search(connection: &Connection, query: &str, limit: usize) -> AppResult<Vec<i64>> {
    let needle = query.trim();
    if needle.is_empty() {
        return Ok(Vec::new());
    }
    let pattern = format!("%{needle}%");
    let mut statement = connection.prepare(
        "SELECT id
         FROM library_current_like
         WHERE filename LIKE ?1
            OR path LIKE ?1
            OR creator LIKE ?1
            OR subtype LIKE ?1
         ORDER BY id ASC
         LIMIT ?2",
    )?;
    let ids = statement
        .query_map(params![pattern, limit as i64], |row| row.get::<_, i64>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(ids)
}

fn normalized_fts_query(query: &str) -> Option<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();

    for character in query.chars() {
        if character.is_alphanumeric() {
            current.extend(character.to_lowercase());
        } else if !current.is_empty() {
            tokens.push(std::mem::take(&mut current));
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }

    tokens.retain(|token| !token.is_empty());
    tokens.dedup();
    if tokens.is_empty() {
        return None;
    }

    Some(
        tokens
            .into_iter()
            .map(|token| format!("\"{token}\""))
            .collect::<Vec<_>>()
            .join(" AND "),
    )
}

fn fts_search(connection: &Connection, query: &str, limit: usize) -> AppResult<Vec<i64>> {
    let Some(query) = normalized_fts_query(query) else {
        return Ok(Vec::new());
    };

    let mut statement = connection.prepare(
        "SELECT rowid
         FROM library_smart_search_fts
         WHERE library_smart_search_fts MATCH ?1
         ORDER BY bm25(
             library_smart_search_fts,
             8.0,
             7.0,
             6.0,
             4.0,
             5.0,
             5.0,
             3.0,
             4.0
         ) ASC, rowid ASC
         LIMIT ?2",
    )?;

    let ids = statement
        .query_map(params![query, limit as i64], |row| row.get::<_, i64>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(ids)
}

fn normalized_search_tokens(value: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut previous_was_lower_or_digit = false;

    for character in value.chars() {
        if !character.is_alphanumeric() {
            if !current.is_empty() {
                tokens.push(std::mem::take(&mut current));
            }
            previous_was_lower_or_digit = false;
            continue;
        }

        if !current.is_empty() && character.is_uppercase() && previous_was_lower_or_digit {
            tokens.push(std::mem::take(&mut current));
        }
        current.extend(character.to_lowercase());
        previous_was_lower_or_digit = character.is_lowercase() || character.is_numeric();
    }
    if !current.is_empty() {
        tokens.push(current);
    }

    tokens.retain(|token| !token.is_empty());
    tokens
}

fn token_trigrams(token: &str) -> Vec<String> {
    let characters = token.chars().collect::<Vec<_>>();
    if characters.len() < 3 {
        return Vec::new();
    }

    characters
        .windows(3)
        .map(|window| window.iter().collect::<String>())
        .collect()
}

fn normalized_trigram_query(query: &str) -> Option<String> {
    let mut trigrams = normalized_search_tokens(query)
        .into_iter()
        .flat_map(|token| token_trigrams(&token))
        .collect::<Vec<_>>();
    trigrams.sort();
    trigrams.dedup();
    if trigrams.is_empty() {
        return None;
    }

    Some(
        trigrams
            .into_iter()
            .map(|trigram| format!("\"{trigram}\""))
            .collect::<Vec<_>>()
            .join(" OR "),
    )
}

fn benchmark_authority_query(query: &str) -> bool {
    let normalized = normalized_search_tokens(query).join(" ");
    (normalized.contains("safe to delete") || normalized.contains("safe to remove"))
        || (normalized.contains("remove")
            && normalized.contains("without")
            && normalized.contains("breaking"))
        || (normalized.contains("compatible")
            && (normalized.contains("patch") || normalized.contains("update")))
        || normalized.contains("malware")
        || normalized.contains("virus")
        || normalized.contains("missing mesh")
        || (normalized.contains("broken")
            && (normalized.contains("patch") || normalized.contains("update")))
}

fn levenshtein_distance(left: &str, right: &str) -> usize {
    let right_chars = right.chars().collect::<Vec<_>>();
    let mut previous = (0..=right_chars.len()).collect::<Vec<_>>();

    for (left_index, left_character) in left.chars().enumerate() {
        let mut current = Vec::with_capacity(right_chars.len() + 1);
        current.push(left_index + 1);
        for (right_index, right_character) in right_chars.iter().enumerate() {
            let substitution = previous[right_index]
                + usize::from(left_character != *right_character);
            let insertion = current[right_index] + 1;
            let deletion = previous[right_index + 1] + 1;
            current.push(substitution.min(insertion).min(deletion));
        }
        previous = current;
    }

    previous[right_chars.len()]
}

fn token_similarity(left: &str, right: &str) -> f64 {
    if left == right {
        return 1.0;
    }
    let left_len = left.chars().count();
    let right_len = right.chars().count();
    let longest = left_len.max(right_len);
    if longest == 0 {
        return 1.0;
    }

    if left_len.min(right_len) >= 3 && (left.starts_with(right) || right.starts_with(left)) {
        let coverage = left_len.min(right_len) as f64 / longest as f64;
        return 0.82 + (0.18 * coverage);
    }

    1.0 - (levenshtein_distance(left, right) as f64 / longest as f64)
}

fn document_similarity(query: &str, document: &str) -> f64 {
    let query_tokens = normalized_search_tokens(query);
    let document_tokens = normalized_search_tokens(document);
    if query_tokens.is_empty() || document_tokens.is_empty() {
        return 0.0;
    }

    let total = query_tokens
        .iter()
        .map(|query_token| {
            document_tokens
                .iter()
                .map(|document_token| token_similarity(query_token, document_token))
                .fold(0.0_f64, f64::max)
        })
        .sum::<f64>();
    total / query_tokens.len() as f64
}

fn trigram_candidate_rows(
    connection: &Connection,
    query: &str,
    limit: usize,
) -> AppResult<Vec<(i64, String)>> {
    let Some(query) = normalized_trigram_query(query) else {
        return Ok(Vec::new());
    };
    let mut statement = connection.prepare(
        "SELECT rowid, search_text
         FROM library_smart_search_trigram
         WHERE library_smart_search_trigram MATCH ?1
         ORDER BY bm25(library_smart_search_trigram) ASC, rowid ASC
         LIMIT ?2",
    )?;
    let rows = statement
        .query_map(params![query, limit as i64], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

fn fuzzy_trigram_search(
    connection: &Connection,
    query: &str,
    limit: usize,
) -> AppResult<FuzzySearchDiagnostics> {
    if benchmark_authority_query(query) {
        return Ok(FuzzySearchDiagnostics {
            ids: Vec::new(),
            candidate_count: 0,
        });
    }

    let strict_ids = fts_search(connection, query, STRICT_CANDIDATE_LIMIT)?;
    let mut candidates = trigram_candidate_rows(connection, query, TRIGRAM_CANDIDATE_LIMIT)?;

    for strict_id in &strict_ids {
        if candidates.iter().any(|(id, _)| id == strict_id) {
            continue;
        }
        let document = connection.query_row(
            "SELECT search_text FROM library_smart_search_trigram WHERE rowid = ?1",
            params![strict_id],
            |row| row.get::<_, String>(0),
        )?;
        candidates.push((*strict_id, document));
    }

    let candidate_count = candidates.len();
    let mut ranked = candidates
        .into_iter()
        .filter_map(|(id, document)| {
            let strict_rank = strict_ids.iter().position(|strict_id| *strict_id == id);
            let fuzzy_score = document_similarity(query, &document);
            if strict_rank.is_none() && fuzzy_score < FUZZY_MIN_SCORE {
                return None;
            }
            let score = strict_rank
                .map(|rank| 2.0 - (rank as f64 * 0.01))
                .unwrap_or(fuzzy_score);
            Some((id, score))
        })
        .collect::<Vec<_>>();
    ranked.sort_by(|(left_id, left_score), (right_id, right_score)| {
        right_score
            .total_cmp(left_score)
            .then_with(|| left_id.cmp(right_id))
    });
    ranked.dedup_by_key(|(id, _)| *id);

    Ok(FuzzySearchDiagnostics {
        ids: ranked
            .into_iter()
            .take(limit)
            .map(|(id, _)| id)
            .collect(),
        candidate_count,
    })
}

fn score_cases<F>(cases: &[QueryCase], mut search: F) -> RetrievalScore
where
    F: FnMut(&str) -> Vec<i64>,
{
    let mut hits_at_five = 0usize;
    let mut reciprocal_rank = 0.0f64;

    for case in cases {
        let results = search(case.query);
        if let Some(position) = results.iter().position(|id| *id == case.expected_id) {
            if position < 5 {
                hits_at_five += 1;
            }
            reciprocal_rank += 1.0 / (position as f64 + 1.0);
        }
    }

    let denominator = cases.len().max(1) as f64;
    RetrievalScore {
        recall_at_five: hits_at_five as f64 / denominator,
        mean_reciprocal_rank: reciprocal_rank / denominator,
    }
}

fn representative_fixtures() -> Vec<SearchFixture> {
    vec![
        SearchFixture {
            id: 1,
            filename: "JolieHair.package".to_owned(),
            path: "/Users/demo/Documents/Electronic Arts/The Sims 4/Mods/CAS/Hair/JolieHair.package"
                .to_owned(),
            creator: Some("Simpliciaty".to_owned()),
            creator_aliases: Vec::new(),
            kind: "CAS".to_owned(),
            subtype: Some("Hair".to_owned()),
            insights: FileInsights {
                embedded_names: vec!["Jolie Toddler Hair".to_owned()],
                family_hints: vec!["Pink swatches".to_owned()],
                resource_summary: vec!["CAS hair geometry".to_owned()],
                ..FileInsights::default()
            },
        },
        SearchFixture {
            id: 2,
            filename: "HARRIE_Baysic_Set.package".to_owned(),
            path: "/Users/demo/Documents/Electronic Arts/The Sims 4/Mods/BuildBuy/Baysic/HARRIE_Baysic_Set.package"
                .to_owned(),
            creator: Some("HARRIE".to_owned()),
            creator_aliases: vec!["HeyHarrie".to_owned()],
            kind: "BuildBuy".to_owned(),
            subtype: Some("Furniture".to_owned()),
            insights: FileInsights {
                embedded_names: vec!["Baysic Kitchen Counter".to_owned()],
                family_hints: vec!["Baysic kitchen".to_owned()],
                resource_summary: vec!["Decor clutter objects".to_owned()],
                ..FileInsights::default()
            },
        },
        SearchFixture {
            id: 3,
            filename: "mc_cmd_center.ts4script".to_owned(),
            path: "/Users/demo/Documents/Electronic Arts/The Sims 4/Mods/MCCC/mc_cmd_center.ts4script"
                .to_owned(),
            creator: Some("Deaderpool".to_owned()),
            creator_aliases: vec!["MC Command Center".to_owned()],
            kind: "Gameplay".to_owned(),
            subtype: Some("Script Mod".to_owned()),
            insights: FileInsights {
                embedded_names: vec!["MC Command Center".to_owned()],
                family_hints: vec!["MCCC modules".to_owned()],
                script_namespaces: vec![
                    "mc_cmd_center".to_owned(),
                    "mc_population".to_owned(),
                    "mc_woohoo".to_owned(),
                ],
                ..FileInsights::default()
            },
        },
        SearchFixture {
            id: 4,
            filename: "PandaSama_ChildBirth.package".to_owned(),
            path: "/Users/demo/Documents/Electronic Arts/The Sims 4/Mods/Gameplay/PandaSama_ChildBirth.package"
                .to_owned(),
            creator: Some("PandaSama".to_owned()),
            creator_aliases: Vec::new(),
            kind: "Gameplay".to_owned(),
            subtype: Some("Gameplay Mod".to_owned()),
            insights: FileInsights {
                embedded_names: vec!["Realistic Childbirth".to_owned()],
                family_hints: vec!["Childbirth gameplay mod".to_owned()],
                ..FileInsights::default()
            },
        },
        SearchFixture {
            id: 5,
            filename: "BetterExceptions.ts4script".to_owned(),
            path: "/Users/demo/Documents/Electronic Arts/The Sims 4/Mods/TwistedMexi/BetterExceptions.ts4script"
                .to_owned(),
            creator: Some("TwistedMexi".to_owned()),
            creator_aliases: vec!["Twisted Mexi".to_owned(), "TMex".to_owned()],
            kind: "Gameplay".to_owned(),
            subtype: Some("Script Mod".to_owned()),
            insights: FileInsights {
                embedded_names: vec!["Better Exceptions".to_owned()],
                family_hints: vec!["exception diagnostics".to_owned()],
                script_namespaces: vec!["better_exceptions".to_owned()],
                ..FileInsights::default()
            },
        },
        SearchFixture {
            id: 6,
            filename: "CleanUI.package".to_owned(),
            path: "/Users/demo/Documents/Electronic Arts/The Sims 4/Mods/UI/CleanUI.package"
                .to_owned(),
            creator: Some("UIWorkshop".to_owned()),
            creator_aliases: Vec::new(),
            kind: "Gameplay".to_owned(),
            subtype: Some("UI".to_owned()),
            insights: FileInsights {
                embedded_names: vec!["Minimal Main Menu".to_owned()],
                family_hints: vec!["main menu replacement".to_owned()],
                ..FileInsights::default()
            },
        },
        SearchFixture {
            id: 7,
            filename: "ThaiStrings.package".to_owned(),
            path: "/Users/demo/Documents/Electronic Arts/The Sims 4/Mods/Translations/ThaiStrings.package"
                .to_owned(),
            creator: Some("TranslatorX".to_owned()),
            creator_aliases: Vec::new(),
            kind: "Gameplay".to_owned(),
            subtype: Some("Translation".to_owned()),
            insights: FileInsights {
                embedded_names: vec!["Thai Translation".to_owned()],
                family_hints: vec!["Thai strings translation".to_owned()],
                resource_summary: vec!["String table".to_owned()],
                ..FileInsights::default()
            },
        },
    ]
}

fn representative_cases() -> Vec<QueryCase> {
    vec![
        QueryCase {
            query: "pink toddler hair",
            expected_id: 1,
        },
        QueryCase {
            query: "harrie kitchen clutter",
            expected_id: 2,
        },
        QueryCase {
            query: "mccc modules",
            expected_id: 3,
        },
        QueryCase {
            query: "childbirth mod",
            expected_id: 4,
        },
        QueryCase {
            query: "tmex better exceptions",
            expected_id: 5,
        },
        QueryCase {
            query: "main menu replacement",
            expected_id: 6,
        },
        QueryCase {
            query: "thai translation strings",
            expected_id: 7,
        },
        QueryCase {
            query: "deaderpool",
            expected_id: 3,
        },
        QueryCase {
            query: "BetterExceptions",
            expected_id: 5,
        },
    ]
}

fn holdout_cases() -> Vec<HoldoutCase> {
    vec![
        HoldoutCase {
            query: "simplcity jolie hir",
            expected_ids: &[1],
            class: QueryClass::Typo,
        },
        HoldoutCase {
            query: "hey harie baysic kitchn",
            expected_ids: &[2],
            class: QueryClass::Typo,
        },
        HoldoutCase {
            query: "mc comand cnter",
            expected_ids: &[3],
            class: QueryClass::Typo,
        },
        HoldoutCase {
            query: "pandasam chldbirth",
            expected_ids: &[4],
            class: QueryClass::Typo,
        },
        HoldoutCase {
            query: "twistd mexi exceptions",
            expected_ids: &[5],
            class: QueryClass::Typo,
        },
        HoldoutCase {
            query: "minmal main menue",
            expected_ids: &[6],
            class: QueryClass::Typo,
        },
        HoldoutCase {
            query: "thi transltion strings",
            expected_ids: &[7],
            class: QueryClass::Typo,
        },
        HoldoutCase {
            query: "jolie simplic",
            expected_ids: &[1],
            class: QueryClass::Partial,
        },
        HoldoutCase {
            query: "baysic harr",
            expected_ids: &[2],
            class: QueryClass::Partial,
        },
        HoldoutCase {
            query: "cmd center",
            expected_ids: &[3],
            class: QueryClass::Partial,
        },
        HoldoutCase {
            query: "panda birth",
            expected_ids: &[4],
            class: QueryClass::Partial,
        },
        HoldoutCase {
            query: "tmex exceptions",
            expected_ids: &[5],
            class: QueryClass::Partial,
        },
        HoldoutCase {
            query: "menu replac",
            expected_ids: &[6],
            class: QueryClass::Partial,
        },
        HoldoutCase {
            query: "thai str",
            expected_ids: &[7],
            class: QueryClass::Partial,
        },
        HoldoutCase {
            query: "clutter kitchen harrie",
            expected_ids: &[2],
            class: QueryClass::VagueLexical,
        },
        HoldoutCase {
            query: "toddler pink jolie",
            expected_ids: &[1],
            class: QueryClass::VagueLexical,
        },
        HoldoutCase {
            query: "exceptions diagnostic",
            expected_ids: &[5],
            class: QueryClass::VagueLexical,
        },
        HoldoutCase {
            query: "minimal replacement menu",
            expected_ids: &[6],
            class: QueryClass::VagueLexical,
        },
        HoldoutCase {
            query: "mccc",
            expected_ids: &[3],
            class: QueryClass::Short,
        },
        HoldoutCase {
            query: "tmex",
            expected_ids: &[5],
            class: QueryClass::Short,
        },
        HoldoutCase {
            query: "baysic",
            expected_ids: &[2],
            class: QueryClass::Short,
        },
        HoldoutCase {
            query: "script mod",
            expected_ids: &[3, 5],
            class: QueryClass::Ambiguous,
        },
        HoldoutCase {
            query: "gameplay mod",
            expected_ids: &[3, 4, 5],
            class: QueryClass::Ambiguous,
        },
    ]
}

fn score_holdout_cases<F>(
    cases: &[HoldoutCase],
    class: Option<QueryClass>,
    mut search: F,
) -> RetrievalScore
where
    F: FnMut(&str) -> Vec<i64>,
{
    let selected = cases
        .iter()
        .filter(|case| class.is_none_or(|wanted| case.class == wanted))
        .collect::<Vec<_>>();
    let mut hits_at_five = 0usize;
    let mut reciprocal_rank = 0.0f64;

    for case in &selected {
        let results = search(case.query);
        if let Some(position) = results
            .iter()
            .position(|id| case.expected_ids.contains(id))
        {
            if position < 5 {
                hits_at_five += 1;
            }
            reciprocal_rank += 1.0 / (position as f64 + 1.0);
        }
    }

    let denominator = selected.len().max(1) as f64;
    RetrievalScore {
        recall_at_five: hits_at_five as f64 / denominator,
        mean_reciprocal_rank: reciprocal_rank / denominator,
    }
}

fn filler_fixture(id: i64) -> SearchFixture {
    SearchFixture {
        id,
        filename: format!("EverydayAccessory_{id:05}.package"),
        path: format!(
            "/Users/demo/Documents/Electronic Arts/The Sims 4/Mods/CAS/Accessories/EverydayAccessory_{id:05}.package"
        ),
        creator: Some(format!("FixtureCreator{:03}", id % 100)),
        creator_aliases: Vec::new(),
        kind: "CAS".to_owned(),
        subtype: Some("Accessory".to_owned()),
        insights: FileInsights {
            embedded_names: vec![format!("Everyday accessory item {id:05}")],
            family_hints: vec!["cas accessory collection".to_owned()],
            resource_summary: vec!["CAS catalog resource".to_owned()],
            ..FileInsights::default()
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_sqlite_supports_fts5_for_local_search_prototypes() {
        let connection = Connection::open_in_memory().expect("memory sqlite");
        create_search_schema(&connection).expect("bundled SQLite must support FTS5");
        connection
            .execute(
                "INSERT INTO library_smart_search_fts (rowid, filename, creator) VALUES (1, 'Test.package', 'Creator')",
                [],
            )
            .expect("insert FTS fixture");
        assert_eq!(
            fts_search(&connection, "creator", 5).expect("FTS search"),
            vec![1]
        );
    }

    #[test]
    fn richer_deterministic_search_recovers_metadata_and_alias_queries_current_like_misses() {
        let fixtures = representative_fixtures();
        let cases = representative_cases();
        let mut connection = Connection::open_in_memory().expect("memory sqlite");
        index_fixtures(&mut connection, &fixtures).expect("index fixtures");

        let current = score_cases(&cases, |query| {
            current_like_search(&connection, query, 5).expect("current LIKE query")
        });
        let fts = score_cases(&cases, |query| {
            fts_search(&connection, query, 5).expect("FTS query")
        });

        assert!(
            current.recall_at_five <= 0.34,
            "representative fixtures should expose today's phrase-search gap: {current:?}"
        );
        assert_eq!(fts.recall_at_five, 1.0);
        assert_eq!(fts.mean_reciprocal_rank, 1.0);
        assert!(fts.recall_at_five > current.recall_at_five);
        assert!(fts.mean_reciprocal_rank > current.mean_reciprocal_rank);
    }

    #[test]
    fn richer_search_has_explicit_no_answer_behavior_for_unsupported_safety_questions() {
        let fixtures = representative_fixtures();
        let mut connection = Connection::open_in_memory().expect("memory sqlite");
        index_fixtures(&mut connection, &fixtures).expect("index fixtures");

        for query in [
            "safe to delete",
            "compatible with current patch",
            "missing mesh",
            "malware free",
        ] {
            assert!(
                fts_search(&connection, query, 5)
                    .expect("negative-control query")
                    .is_empty(),
                "unsupported safety query must not be turned into a positive retrieval: {query}"
            );
            assert!(
                fuzzy_trigram_search(&connection, query, 5)
                    .expect("fuzzy negative-control query")
                    .ids
                    .is_empty(),
                "fuzzy retrieval must stay behind the benchmark authority gate: {query}"
            );
        }
        assert!(
            fts_search(&connection, "---", 5)
                .expect("punctuation-only query")
                .is_empty(),
            "empty normalized query must not broaden into all rows"
        );
        assert!(
            fuzzy_trigram_search(&connection, "---", 5)
                .expect("punctuation-only fuzzy query")
                .ids
                .is_empty(),
            "empty fuzzy query must not broaden into all rows"
        );
    }

    #[test]
    fn bundled_sqlite_trigram_candidates_are_available_and_bounded() {
        let fixtures = representative_fixtures();
        let mut connection = Connection::open_in_memory().expect("memory sqlite");
        index_fixtures(&mut connection, &fixtures).expect("index fixtures with trigram");

        let candidates = trigram_candidate_rows(&connection, "baysc", 3)
            .expect("trigram candidate query");
        assert!(candidates.len() <= 3);
        assert!(
            candidates.iter().any(|(id, _)| *id == 2),
            "bundled trigram tokenizer should recover a near-spelling candidate"
        );
    }

    #[test]
    fn deterministic_fuzzy_search_recovers_holdout_typos_without_losing_strict_matches() {
        let fixtures = representative_fixtures();
        let holdout = holdout_cases();
        let representative = representative_cases();
        let mut connection = Connection::open_in_memory().expect("memory sqlite");
        index_fixtures(&mut connection, &fixtures).expect("index fixtures");

        let strict_holdout = score_holdout_cases(&holdout, None, |query| {
            fts_search(&connection, query, 5).expect("strict holdout query")
        });
        let fuzzy_holdout = score_holdout_cases(&holdout, None, |query| {
            fuzzy_trigram_search(&connection, query, 5)
                .expect("fuzzy holdout query")
                .ids
        });
        let strict_typos = score_holdout_cases(&holdout, Some(QueryClass::Typo), |query| {
            fts_search(&connection, query, 5).expect("strict typo query")
        });
        let fuzzy_typos = score_holdout_cases(&holdout, Some(QueryClass::Typo), |query| {
            fuzzy_trigram_search(&connection, query, 5)
                .expect("fuzzy typo query")
                .ids
        });
        let fuzzy_representative = score_cases(&representative, |query| {
            fuzzy_trigram_search(&connection, query, 5)
                .expect("fuzzy representative query")
                .ids
        });

        assert!(fuzzy_holdout.recall_at_five > strict_holdout.recall_at_five);
        assert!(fuzzy_typos.recall_at_five > strict_typos.recall_at_five);
        assert_eq!(fuzzy_representative.recall_at_five, 1.0);
        assert_eq!(fuzzy_representative.mean_reciprocal_rank, 1.0);

        for case in &holdout {
            let result = fuzzy_trigram_search(&connection, case.query, 5)
                .expect("bounded fuzzy holdout query");
            assert!(
                result.candidate_count <= TRIGRAM_CANDIDATE_LIMIT + STRICT_CANDIDATE_LIMIT,
                "fuzzy candidate set must remain bounded: {} -> {}",
                case.query,
                result.candidate_count
            );
        }
        assert!(
            fuzzy_trigram_search(&connection, "totally unrelated spaceship", 5)
                .expect("low-confidence query")
                .ids
                .is_empty(),
            "low-confidence fuzzy search should preserve a real no-answer outcome"
        );
    }

    #[test]
    fn deterministic_fts_leaves_real_semantic_gaps_for_a_future_embedding_benchmark() {
        let fixtures = representative_fixtures();
        let mut connection = Connection::open_in_memory().expect("memory sqlite");
        index_fixtures(&mut connection, &fixtures).expect("index fixtures");

        for (query, expected_semantic_target) in [
            ("rose kids hairstyle", 1),
            ("pregnancy delivery gameplay", 4),
            ("game error helper", 5),
        ] {
            let results = fts_search(&connection, query, 5).expect("semantic-gap query");
            assert!(
                !results.contains(&expected_semantic_target),
                "deterministic FTS should not be credited with semantic understanding it does not have: {query}"
            );
            let fuzzy = fuzzy_trigram_search(&connection, query, 5)
                .expect("fuzzy semantic-gap query");
            assert!(
                !fuzzy.ids.contains(&expected_semantic_target),
                "spelling similarity must not be credited with semantic understanding it does not have: {query}"
            );
        }
    }

    #[test]
    fn normalized_search_document_does_not_index_absolute_path_only_matches() {
        let fixtures = vec![SearchFixture {
            id: 99,
            filename: "Neutral.package".to_owned(),
            path: "/Users/privateusername/Documents/Electronic Arts/The Sims 4/Mods/Neutral.package"
                .to_owned(),
            creator: Some("NeutralCreator".to_owned()),
            creator_aliases: Vec::new(),
            kind: "CAS".to_owned(),
            subtype: Some("Accessory".to_owned()),
            insights: FileInsights::default(),
        }];
        let mut connection = Connection::open_in_memory().expect("memory sqlite");
        index_fixtures(&mut connection, &fixtures).expect("index fixture");
        assert_eq!(
            current_like_search(&connection, "privateusername", 5)
                .expect("current path query"),
            vec![99]
        );
        assert!(
            fts_search(&connection, "privateusername", 5)
                .expect("path privacy query")
                .is_empty(),
            "absolute path text must not dominate the richer search document"
        );
    }

    #[test]
    fn production_shape_explicit_sync_tracks_metadata_scope_and_privacy() {
        let mut fixtures = representative_fixtures();
        fixtures.push(SearchFixture {
            id: 8,
            filename: "Tmex_TOOL.ts4script".to_owned(),
            path: "/Users/demo/Documents/Electronic Arts/The Sims 4/Mods/TwistedMexi/Tmex_TOOL.ts4script"
                .to_owned(),
            creator: Some("TwistedMexi".to_owned()),
            creator_aliases: Vec::new(),
            kind: "BuildBuy".to_owned(),
            subtype: Some("Script Mod".to_owned()),
            insights: FileInsights {
                embedded_names: vec!["T.O.O.L.".to_owned()],
                family_hints: vec!["object placement utility".to_owned()],
                script_namespaces: vec!["tmex_tool".to_owned()],
                ..FileInsights::default()
            },
        });

        let mut connection = Connection::open_in_memory().expect("memory sqlite");
        create_production_shape_schema(&connection, ProductionSearchShape::BOTH)
            .expect("production-shape schema");
        seed_production_source(&mut connection, &fixtures).expect("seed installed source");

        let downloads_only = SearchFixture {
            id: 90,
            filename: "PrivateDownloadOnly.package".to_owned(),
            path: "/Users/privateusername/Downloads/PrivateDownloadOnly.package".to_owned(),
            creator: Some("DownloadCreator".to_owned()),
            creator_aliases: vec!["downloadsecretalias".to_owned()],
            kind: "CAS".to_owned(),
            subtype: Some("Accessory".to_owned()),
            insights: FileInsights {
                embedded_names: vec!["downloadonlysecretterm".to_owned()],
                ..FileInsights::default()
            },
        };
        {
            let transaction = connection.transaction().expect("download fixture transaction");
            insert_production_source_fixture(&transaction, &downloads_only, "downloads")
                .expect("insert downloads-only source");
            transaction.commit().expect("commit downloads fixture");
        }
        connection
            .execute(
                "UPDATE production_files
                 SET path = '/Users/privateusername/Documents/Electronic Arts/The Sims 4/Mods/JolieHair.package'
                 WHERE id = 1",
                [],
            )
            .expect("make installed path privacy-sensitive");

        assert_eq!(
            rebuild_production_search(&mut connection, ProductionSearchShape::BOTH)
                .expect("initial backfill"),
            fixtures.len()
        );
        assert!(production_fts_search(&connection, "privateusername", 5)
            .expect("private path search")
            .is_empty());
        assert!(production_fts_search(&connection, "downloadonlysecretterm", 5)
            .expect("downloads-only search")
            .is_empty());
        let indexed_private_paths: i64 = connection
            .query_row(
                "SELECT COUNT(*)
                 FROM production_search_trigram
                 WHERE lower(search_text) LIKE '%privateusername%'",
                [],
                |row| row.get(0),
            )
            .expect("inspect trigram privacy boundary");
        assert_eq!(indexed_private_paths, 0);

        connection
            .execute(
                "UPDATE production_files
                 SET kind = 'BuildBuy', subtype = 'Reclassified Surface'
                 WHERE id = 1",
                [],
            )
            .expect("update category metadata");
        refresh_production_search_files(&mut connection, &[1], ProductionSearchShape::BOTH)
            .expect("refresh category search row");
        assert_eq!(
            production_fts_search(&connection, "reclassified surface", 5)
                .expect("category search"),
            vec![1]
        );

        let refreshed_insights = FileInsights {
            embedded_names: vec!["Aurora Catalog Name".to_owned()],
            family_hints: vec!["pink swatches".to_owned()],
            ..FileInsights::default()
        };
        connection
            .execute(
                "UPDATE production_files SET insights = ?1 WHERE id = 1",
                params![serde_json::to_string(&refreshed_insights).expect("serialize insights")],
            )
            .expect("refresh searchable insights");
        refresh_production_search_files(&mut connection, &[1], ProductionSearchShape::BOTH)
            .expect("refresh insight search row");
        assert_eq!(
            production_fts_search(&connection, "aurora catalog", 5)
                .expect("insight search"),
            vec![1]
        );

        let twisted_mexi_id: i64 = connection
            .query_row(
                "SELECT id FROM production_creators WHERE canonical_name = 'TwistedMexi'",
                [],
                |row| row.get(0),
            )
            .expect("TwistedMexi creator id");
        connection
            .execute(
                "INSERT INTO production_user_creator_aliases(creator_id, alias_name)
                 VALUES (?1, 'MexiWorkshop')",
                params![twisted_mexi_id],
            )
            .expect("insert learned creator alias");
        assert_eq!(
            refresh_production_search_creator(
                &mut connection,
                twisted_mexi_id,
                ProductionSearchShape::BOTH,
            )
            .expect("refresh creator-wide search rows"),
            2
        );
        let alias_matches = production_fts_search(&connection, "MexiWorkshop", 10)
            .expect("creator-wide alias search");
        assert!(alias_matches.contains(&5));
        assert!(alias_matches.contains(&8));

        let simpliciaty_id: i64 = connection
            .query_row(
                "SELECT id FROM production_creators WHERE canonical_name = 'Simpliciaty'",
                [],
                |row| row.get(0),
            )
            .expect("Simpliciaty creator id");
        connection
            .execute(
                "INSERT INTO production_user_creator_aliases(creator_id, alias_name)
                 VALUES (?1, 'MexiWorkshop')
                 ON CONFLICT(alias_name) DO UPDATE SET creator_id = excluded.creator_id",
                params![simpliciaty_id],
            )
            .expect("reassign learned creator alias");
        assert_eq!(
            refresh_production_search_creators(
                &mut connection,
                &[twisted_mexi_id, simpliciaty_id],
                ProductionSearchShape::BOTH,
            )
            .expect("refresh old and new alias owner search rows"),
            3
        );
        let reassigned_alias_matches = production_fts_search(&connection, "MexiWorkshop", 10)
            .expect("reassigned creator alias search");
        assert!(reassigned_alias_matches.contains(&1));
        assert!(!reassigned_alias_matches.contains(&5));
        assert!(!reassigned_alias_matches.contains(&8));

        connection
            .execute("DELETE FROM production_files WHERE id = 2", [])
            .expect("delete installed source row");
        refresh_production_search_files(&mut connection, &[2], ProductionSearchShape::BOTH)
            .expect("remove stale search row");
        assert!(production_fts_search(&connection, "Baysic", 5)
            .expect("removed file search")
            .is_empty());

        connection
            .execute(
                "UPDATE production_files SET source_location = 'downloads' WHERE id = 7",
                [],
            )
            .expect("move source outside installed Library scope");
        refresh_production_search_files(&mut connection, &[7], ProductionSearchShape::BOTH)
            .expect("remove out-of-scope search row");
        assert!(production_fts_search(&connection, "Thai Translation", 5)
            .expect("out-of-scope file search")
            .is_empty());

        let new_fixture = SearchFixture {
            id: 50,
            filename: "NewlyIndexed.package".to_owned(),
            path: "/Users/demo/Documents/Electronic Arts/The Sims 4/Mods/NewlyIndexed.package"
                .to_owned(),
            creator: Some("FreshCreator".to_owned()),
            creator_aliases: Vec::new(),
            kind: "CAS".to_owned(),
            subtype: Some("Hair".to_owned()),
            insights: FileInsights {
                embedded_names: vec!["Fresh Search Insert".to_owned()],
                ..FileInsights::default()
            },
        };
        {
            let transaction = connection.transaction().expect("new source transaction");
            insert_production_source_fixture(&transaction, &new_fixture, "mods")
                .expect("insert new source row");
            refresh_production_search_file_in_transaction(
                &transaction,
                new_fixture.id,
                ProductionSearchShape::BOTH,
            )
            .expect("index new source row in same transaction");
            transaction.commit().expect("commit new source and search row");
        }
        assert_eq!(
            production_fuzzy_search(&connection, "fresh serch insert", 5)
                .expect("fuzzy new-row search")
                .ids,
            vec![50]
        );
    }

    #[test]
    fn production_shape_rebuild_is_atomic_repairable_and_scan_replace_safe() {
        let fixtures = representative_fixtures();
        let mut connection = Connection::open_in_memory().expect("memory sqlite");
        create_production_shape_schema(&connection, ProductionSearchShape::BOTH)
            .expect("production-shape schema");
        seed_production_source(&mut connection, &fixtures).expect("seed source");
        rebuild_production_search(&mut connection, ProductionSearchShape::BOTH)
            .expect("initial search rebuild");

        connection
            .execute(
                "INSERT INTO production_search_fts(rowid, filename) VALUES (99999, 'staleghost')",
                [],
            )
            .expect("inject stale unicode row");
        connection
            .execute(
                "INSERT INTO production_search_trigram(rowid, search_text) VALUES (99999, 'staleghost')",
                [],
            )
            .expect("inject stale trigram row");
        assert_eq!(
            production_fts_search(&connection, "staleghost", 5).expect("stale row visible"),
            vec![99999]
        );
        rebuild_production_search(&mut connection, ProductionSearchShape::BOTH)
            .expect("repair stale index");
        assert!(production_fts_search(&connection, "staleghost", 5)
            .expect("stale row repaired")
            .is_empty());

        connection
            .execute(
                "UPDATE production_files SET filename = 'UniqueFreshRename.package' WHERE id = 5",
                [],
            )
            .expect("change source before interrupted rebuild");
        let error = rebuild_production_search_with_interrupt(
            &mut connection,
            ProductionSearchShape::BOTH,
            2,
        )
        .expect_err("interrupted rebuild should roll back");
        assert!(error
            .to_string()
            .contains("synthetic search-index rebuild interruption"));
        assert!(production_fts_search(&connection, "UniqueFreshRename", 5)
            .expect("interrupted rebuild must not expose partial new index")
            .is_empty());
        assert_eq!(
            production_fts_search(&connection, "MCCC", 5)
                .expect("previous complete index remains queryable"),
            vec![3]
        );

        rebuild_production_search(&mut connection, ProductionSearchShape::BOTH)
            .expect("successful rebuild after interruption");
        assert_eq!(
            production_fts_search(&connection, "UniqueFreshRename", 5)
                .expect("fresh source visible after repair"),
            vec![5]
        );

        let replacement = SearchFixture {
            id: 501,
            filename: "ReplacementScanItem.package".to_owned(),
            path: "/Users/demo/Documents/Electronic Arts/The Sims 4/Mods/ReplacementScanItem.package"
                .to_owned(),
            creator: Some("ReplacementCreator".to_owned()),
            creator_aliases: Vec::new(),
            kind: "BuildBuy".to_owned(),
            subtype: Some("Furniture".to_owned()),
            insights: FileInsights {
                embedded_names: vec!["Replacement Scan Sofa".to_owned()],
                ..FileInsights::default()
            },
        };
        {
            let transaction = connection.transaction().expect("scan replacement transaction");
            transaction
                .execute(
                    "DELETE FROM production_files WHERE source_location IN ('mods', 'tray')",
                    [],
                )
                .expect("clear installed source rows");
            insert_production_source_fixture(&transaction, &replacement, "mods")
                .expect("insert replacement scan row");
            assert_eq!(
                rebuild_production_search_in_transaction(
                    &transaction,
                    ProductionSearchShape::BOTH,
                    None,
                )
                .expect("rebuild inside scan replacement transaction"),
                1
            );
            transaction.commit().expect("commit scan replacement and search together");
        }
        assert!(production_fts_search(&connection, "MCCC", 5)
            .expect("old scan result removed")
            .is_empty());
        assert_eq!(
            production_fts_search(&connection, "Replacement Scan Sofa", 5)
                .expect("replacement scan search"),
            vec![501]
        );

        let unicode_count: i64 = connection
            .query_row("SELECT COUNT(*) FROM production_search_fts", [], |row| row.get(0))
            .expect("unicode count");
        let trigram_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM production_search_trigram",
                [],
                |row| row.get(0),
            )
            .expect("trigram count");
        assert_eq!(unicode_count, 1);
        assert_eq!(trigram_count, 1);
        rebuild_production_search(&mut connection, ProductionSearchShape::BOTH)
            .expect("idempotent rebuild");
        assert_eq!(
            connection
                .query_row("SELECT COUNT(*) FROM production_search_fts", [], |row| row.get::<_, i64>(0))
                .expect("idempotent unicode count"),
            1
        );
    }

    #[test]
    fn contentless_delete_indexes_support_explicit_sync_repair_and_bounded_fuzzy_lookup() {
        let fixtures = representative_fixtures();
        let mut connection = Connection::open_in_memory().expect("memory sqlite");
        create_production_shape_schema(&connection, ProductionSearchShape::SOURCE_ONLY)
            .expect("source schema");
        create_contentless_search_schema(&connection)
            .expect("bundled SQLite must support contentless-delete FTS5");
        seed_production_source(&mut connection, &fixtures).expect("seed source");
        assert_eq!(
            rebuild_contentless_search(&mut connection).expect("contentless rebuild"),
            fixtures.len()
        );
        assert_eq!(
            contentless_fts_search(&connection, "MCCC", 5).expect("contentless strict search"),
            vec![3]
        );
        assert_eq!(
            contentless_fuzzy_search(&connection, "twistd mexi exceptions", 5)
                .expect("contentless fuzzy search")
                .ids,
            vec![5]
        );

        let stored_filename: Option<String> = connection
            .query_row(
                "SELECT filename FROM production_contentless_fts WHERE rowid = 3",
                [],
                |row| row.get(0),
            )
            .expect("contentless user column query");
        assert!(
            stored_filename.is_none(),
            "contentless index must not retain a second readable copy of source text"
        );

        {
            let transaction = connection.transaction().expect("contentless refresh transaction");
            transaction
                .execute(
                    "UPDATE production_files SET filename = 'BetterDiagnosticsRenamed.ts4script' WHERE id = 5",
                    [],
                )
                .expect("update source filename");
            refresh_contentless_search_file_in_transaction(&transaction, 5)
                .expect("refresh contentless row");
            transaction.commit().expect("commit source and contentless refresh");
        }
        assert!(contentless_fts_search(&connection, "BetterExceptions", 5)
            .expect("old contentless term")
            .is_empty());
        assert_eq!(
            contentless_fts_search(&connection, "BetterDiagnosticsRenamed", 5)
                .expect("new contentless term"),
            vec![5]
        );

        connection
            .execute(
                "INSERT INTO production_contentless_fts(rowid, filename) VALUES (99999, 'contentlessghost')",
                [],
            )
            .expect("inject stale contentless unicode row");
        connection
            .execute(
                "INSERT INTO production_contentless_trigram(rowid, search_text) VALUES (99999, 'contentlessghost')",
                [],
            )
            .expect("inject stale contentless trigram row");
        assert_eq!(
            contentless_fts_search(&connection, "contentlessghost", 5)
                .expect("stale contentless row visible"),
            vec![99999]
        );
        rebuild_contentless_search(&mut connection).expect("repair contentless index");
        assert!(contentless_fts_search(&connection, "contentlessghost", 5)
            .expect("stale contentless row repaired")
            .is_empty());

        connection
            .execute(
                "UPDATE production_files SET filename = 'InterruptedContentlessRename.ts4script' WHERE id = 5",
                [],
            )
            .expect("change source before interrupted contentless rebuild");
        let interrupted = rebuild_contentless_search_with_interrupt(&mut connection, 2)
            .expect_err("interrupted contentless rebuild should roll back");
        assert!(interrupted
            .to_string()
            .contains("synthetic contentless search-index rebuild interruption"));
        assert!(contentless_fts_search(&connection, "InterruptedContentlessRename", 5)
            .expect("partial contentless rebuild must stay invisible")
            .is_empty());
        assert_eq!(
            contentless_fts_search(&connection, "BetterDiagnosticsRenamed", 5)
                .expect("previous complete contentless index remains visible"),
            vec![5]
        );
        rebuild_contentless_search(&mut connection).expect("repair after interrupted rebuild");
        assert_eq!(
            contentless_fts_search(&connection, "InterruptedContentlessRename", 5)
                .expect("new contentless source visible after complete rebuild"),
            vec![5]
        );

        {
            let transaction = connection.transaction().expect("contentless delete transaction");
            transaction
                .execute("DELETE FROM production_files WHERE id = 4", [])
                .expect("delete source row");
            refresh_contentless_search_file_in_transaction(&transaction, 4)
                .expect("delete contentless search row");
            transaction.commit().expect("commit source/index deletion");
        }
        assert!(contentless_fts_search(&connection, "Childbirth", 5)
            .expect("deleted contentless row search")
            .is_empty());
    }

    #[test]
    fn smart_search_prototype_stays_out_of_normal_command_registration() {
        let commands_source = include_str!("../commands/mod.rs");
        assert!(!commands_source.contains("library_smart_search_prototype"));
        assert!(!commands_source.contains("library_smart_search_fts"));
        assert!(!commands_source.contains("library_smart_search_trigram"));
        assert!(!commands_source.contains("production_search_fts"));
        assert!(!commands_source.contains("production_search_trigram"));
        assert!(!commands_source.contains("production_contentless_fts"));
        assert!(!commands_source.contains("production_contentless_trigram"));

        let core_source = include_str!("mod.rs");
        assert!(core_source.contains(
            "#[cfg(test)]\npub mod library_smart_search_prototype;"
        ));
        let prototype_source = include_str!("library_smart_search_prototype.rs");
        assert!(prototype_source.starts_with("#![cfg(test)]"));
    }

    #[test]
    #[ignore = "10k synthetic search benchmark; run pnpm run test:library:stress"]
    fn large_library_smart_search_baseline_reports_relevance_and_latency() {
        let cases = representative_cases();
        let holdout = holdout_cases();
        let mut fixtures = representative_fixtures();
        fixtures.extend((100..10_093).map(filler_fixture));
        assert_eq!(fixtures.len(), 10_000);

        let mut trigram_only_connection = Connection::open_in_memory().expect("trigram memory sqlite");
        let trigram_only_started = Instant::now();
        index_trigram_only(&mut trigram_only_connection, &fixtures)
            .expect("index standalone trigram fixtures");
        let trigram_only_elapsed = trigram_only_started.elapsed();

        let mut connection = Connection::open_in_memory().expect("memory sqlite");
        let index_started = Instant::now();
        index_fixtures(&mut connection, &fixtures).expect("index 10k fixtures");
        let index_elapsed = index_started.elapsed();

        let like_started = Instant::now();
        let current = score_cases(&cases, |query| {
            current_like_search(&connection, query, 5).expect("current LIKE query")
        });
        let like_elapsed = like_started.elapsed();

        let fts_started = Instant::now();
        let fts = score_cases(&cases, |query| {
            fts_search(&connection, query, 5).expect("FTS query")
        });
        let fts_elapsed = fts_started.elapsed();

        let holdout_fts_started = Instant::now();
        let holdout_fts = score_holdout_cases(&holdout, None, |query| {
            fts_search(&connection, query, 5).expect("holdout FTS query")
        });
        let holdout_fts_elapsed = holdout_fts_started.elapsed();

        let holdout_fuzzy_started = Instant::now();
        let holdout_fuzzy = score_holdout_cases(&holdout, None, |query| {
            fuzzy_trigram_search(&connection, query, 5)
                .expect("holdout fuzzy query")
                .ids
        });
        let holdout_fuzzy_elapsed = holdout_fuzzy_started.elapsed();

        let typo_fts = score_holdout_cases(&holdout, Some(QueryClass::Typo), |query| {
            fts_search(&connection, query, 5).expect("typo FTS query")
        });
        let typo_fuzzy = score_holdout_cases(&holdout, Some(QueryClass::Typo), |query| {
            fuzzy_trigram_search(&connection, query, 5)
                .expect("typo fuzzy query")
                .ids
        });
        let partial_fts = score_holdout_cases(&holdout, Some(QueryClass::Partial), |query| {
            fts_search(&connection, query, 5).expect("partial FTS query")
        });
        let partial_fuzzy = score_holdout_cases(&holdout, Some(QueryClass::Partial), |query| {
            fuzzy_trigram_search(&connection, query, 5)
                .expect("partial fuzzy query")
                .ids
        });
        let vague_fts = score_holdout_cases(&holdout, Some(QueryClass::VagueLexical), |query| {
            fts_search(&connection, query, 5).expect("vague FTS query")
        });
        let vague_fuzzy = score_holdout_cases(&holdout, Some(QueryClass::VagueLexical), |query| {
            fuzzy_trigram_search(&connection, query, 5)
                .expect("vague fuzzy query")
                .ids
        });
        let short_fts = score_holdout_cases(&holdout, Some(QueryClass::Short), |query| {
            fts_search(&connection, query, 5).expect("short FTS query")
        });
        let short_fuzzy = score_holdout_cases(&holdout, Some(QueryClass::Short), |query| {
            fuzzy_trigram_search(&connection, query, 5)
                .expect("short fuzzy query")
                .ids
        });
        let ambiguous_fts = score_holdout_cases(&holdout, Some(QueryClass::Ambiguous), |query| {
            fts_search(&connection, query, 5).expect("ambiguous FTS query")
        });
        let ambiguous_fuzzy = score_holdout_cases(&holdout, Some(QueryClass::Ambiguous), |query| {
            fuzzy_trigram_search(&connection, query, 5)
                .expect("ambiguous fuzzy query")
                .ids
        });

        let mut candidate_total = 0usize;
        let mut candidate_max = 0usize;
        for case in &holdout {
            let result = fuzzy_trigram_search(&connection, case.query, 5)
                .expect("candidate diagnostic query");
            candidate_total += result.candidate_count;
            candidate_max = candidate_max.max(result.candidate_count);
        }
        let candidate_average = candidate_total as f64 / holdout.len() as f64;

        println!(
            "smart-search-10k index_ms={} trigram_only_index_ms={} current_like_ms={} fts_ms={} current_recall_at_5={:.3} current_mrr={:.3} fts_recall_at_5={:.3} fts_mrr={:.3} holdout_fts_ms={} holdout_fuzzy_ms={} holdout_fts_recall_at_5={:.3} holdout_fts_mrr={:.3} holdout_fuzzy_recall_at_5={:.3} holdout_fuzzy_mrr={:.3} typo_fts_recall_at_5={:.3} typo_fuzzy_recall_at_5={:.3} partial_fts_recall_at_5={:.3} partial_fuzzy_recall_at_5={:.3} vague_fts_recall_at_5={:.3} vague_fuzzy_recall_at_5={:.3} short_fts_recall_at_5={:.3} short_fuzzy_recall_at_5={:.3} ambiguous_fts_recall_at_5={:.3} ambiguous_fuzzy_recall_at_5={:.3} fuzzy_candidate_avg={:.1} fuzzy_candidate_max={}",
            index_elapsed.as_millis(),
            trigram_only_elapsed.as_millis(),
            like_elapsed.as_millis(),
            fts_elapsed.as_millis(),
            current.recall_at_five,
            current.mean_reciprocal_rank,
            fts.recall_at_five,
            fts.mean_reciprocal_rank,
            holdout_fts_elapsed.as_millis(),
            holdout_fuzzy_elapsed.as_millis(),
            holdout_fts.recall_at_five,
            holdout_fts.mean_reciprocal_rank,
            holdout_fuzzy.recall_at_five,
            holdout_fuzzy.mean_reciprocal_rank,
            typo_fts.recall_at_five,
            typo_fuzzy.recall_at_five,
            partial_fts.recall_at_five,
            partial_fuzzy.recall_at_five,
            vague_fts.recall_at_five,
            vague_fuzzy.recall_at_five,
            short_fts.recall_at_five,
            short_fuzzy.recall_at_five,
            ambiguous_fts.recall_at_five,
            ambiguous_fuzzy.recall_at_five,
            candidate_average,
            candidate_max,
        );

        assert_eq!(fts.recall_at_five, 1.0);
        assert_eq!(fts.mean_reciprocal_rank, 1.0);
        assert!(fts.recall_at_five > current.recall_at_five);
        assert!(holdout_fuzzy.recall_at_five > holdout_fts.recall_at_five);
        assert!(typo_fuzzy.recall_at_five > typo_fts.recall_at_five);
        assert!(candidate_max <= TRIGRAM_CANDIDATE_LIMIT + STRICT_CANDIDATE_LIMIT);
    }

    #[test]
    #[ignore = "10k production-shape search storage benchmark; run pnpm run test:library:stress"]
    fn large_production_shape_search_index_lifecycle_reports_costs() {
        let mut fixtures = representative_fixtures();
        fixtures.extend((100..10_093).map(filler_fixture));
        assert_eq!(fixtures.len(), 10_000);

        let temp = tempfile::tempdir().expect("temporary benchmark directory");
        let source_path = production_shape_path(temp.path(), "source-only");
        let unicode_path = production_shape_path(temp.path(), "unicode-only");
        let trigram_path = production_shape_path(temp.path(), "trigram-only");
        let both_path = production_shape_path(temp.path(), "both-indexes");
        let contentless_path = production_shape_path(temp.path(), "contentless-both-indexes");

        let (source_bytes, source_backfill_ms) = build_production_shape_database(
            &source_path,
            &fixtures,
            ProductionSearchShape::SOURCE_ONLY,
        )
        .expect("build source-only database");
        let (unicode_bytes, unicode_backfill_ms) = build_production_shape_database(
            &unicode_path,
            &fixtures,
            ProductionSearchShape::UNICODE_ONLY,
        )
        .expect("build unicode FTS database");
        let (trigram_bytes, trigram_backfill_ms) = build_production_shape_database(
            &trigram_path,
            &fixtures,
            ProductionSearchShape::TRIGRAM_ONLY,
        )
        .expect("build trigram FTS database");
        let (both_bytes, both_backfill_ms) = build_production_shape_database(
            &both_path,
            &fixtures,
            ProductionSearchShape::BOTH,
        )
        .expect("build dual-index database");
        let (contentless_bytes, contentless_backfill_ms) =
            build_contentless_production_database(&contentless_path, &fixtures)
                .expect("build contentless dual-index database");

        let unicode_overhead = unicode_bytes.saturating_sub(source_bytes);
        let trigram_overhead = trigram_bytes.saturating_sub(source_bytes);
        let both_overhead = both_bytes.saturating_sub(source_bytes);
        let contentless_overhead = contentless_bytes.saturating_sub(source_bytes);
        assert!(unicode_bytes > source_bytes);
        assert!(trigram_bytes > source_bytes);
        assert!(both_bytes >= unicode_bytes.max(trigram_bytes));
        assert!(
            contentless_bytes < both_bytes,
            "contentless indexes should reduce duplicate-text storage before they are recommended"
        );

        let mut connection = Connection::open(&both_path).expect("open dual-index database");
        let changed_ids = (100_i64..200_i64).collect::<Vec<_>>();
        let incremental_started = Instant::now();
        {
            let transaction = connection.transaction().expect("incremental update transaction");
            for file_id in &changed_ids {
                let insights = FileInsights {
                    embedded_names: vec![format!("Incremental Refresh Item {file_id}")],
                    family_hints: vec!["incremental lifecycle marker".to_owned()],
                    ..FileInsights::default()
                };
                transaction
                    .execute(
                        "UPDATE production_files SET insights = ?1 WHERE id = ?2",
                        params![
                            serde_json::to_string(&insights).expect("serialize incremental insights"),
                            file_id
                        ],
                    )
                    .expect("update source insight");
                refresh_production_search_file_in_transaction(
                    &transaction,
                    *file_id,
                    ProductionSearchShape::BOTH,
                )
                .expect("refresh changed search row");
            }
            transaction.commit().expect("commit 100 source/search updates");
        }
        let incremental_100_ms = incremental_started.elapsed().as_millis();
        assert!(!production_fts_search(&connection, "incremental refresh", 5)
            .expect("incremental strict search")
            .is_empty());
        assert!(!production_fuzzy_search(&connection, "incremntal refrsh", 5)
            .expect("incremental fuzzy search")
            .ids
            .is_empty());

        let strict_query_started = Instant::now();
        for _ in 0..100 {
            production_fts_search(&connection, "incremental refresh", 5)
                .expect("warm strict query");
        }
        let strict_query_100_us = strict_query_started.elapsed().as_micros();
        let fuzzy_query_started = Instant::now();
        for _ in 0..100 {
            production_fuzzy_search(&connection, "incremntal refrsh", 5)
                .expect("warm fuzzy query");
        }
        let fuzzy_query_100_us = fuzzy_query_started.elapsed().as_micros();

        let cleanup_started = Instant::now();
        {
            let transaction = connection.transaction().expect("cleanup transaction");
            for file_id in &changed_ids {
                transaction
                    .execute("DELETE FROM production_files WHERE id = ?1", params![file_id])
                    .expect("delete source row");
                refresh_production_search_file_in_transaction(
                    &transaction,
                    *file_id,
                    ProductionSearchShape::BOTH,
                )
                .expect("remove stale search row");
            }
            transaction.commit().expect("commit cleanup");
        }
        let cleanup_100_ms = cleanup_started.elapsed().as_millis();
        let unicode_count: i64 = connection
            .query_row("SELECT COUNT(*) FROM production_search_fts", [], |row| row.get(0))
            .expect("unicode row count after cleanup");
        let trigram_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM production_search_trigram",
                [],
                |row| row.get(0),
            )
            .expect("trigram row count after cleanup");
        assert_eq!(unicode_count, 9_900);
        assert_eq!(trigram_count, 9_900);

        let repair_started = Instant::now();
        let repaired = rebuild_production_search(&mut connection, ProductionSearchShape::BOTH)
            .expect("idempotent repair rebuild");
        let repair_ms = repair_started.elapsed().as_millis();
        assert_eq!(repaired, 9_900);
        assert_eq!(
            connection
                .query_row("SELECT COUNT(*) FROM production_search_fts", [], |row| row.get::<_, i64>(0))
                .expect("unicode count after repair"),
            9_900
        );

        let mut contentless_connection =
            Connection::open(&contentless_path).expect("open contentless dual-index database");
        let contentless_incremental_started = Instant::now();
        {
            let transaction = contentless_connection
                .transaction()
                .expect("contentless incremental transaction");
            for file_id in &changed_ids {
                let insights = FileInsights {
                    embedded_names: vec![format!("Contentless Refresh Item {file_id}")],
                    family_hints: vec!["contentless lifecycle marker".to_owned()],
                    ..FileInsights::default()
                };
                transaction
                    .execute(
                        "UPDATE production_files SET insights = ?1 WHERE id = ?2",
                        params![
                            serde_json::to_string(&insights)
                                .expect("serialize contentless incremental insights"),
                            file_id
                        ],
                    )
                    .expect("update contentless source insight");
                refresh_contentless_search_file_in_transaction(&transaction, *file_id)
                    .expect("refresh contentless changed search row");
            }
            transaction
                .commit()
                .expect("commit 100 contentless source/search updates");
        }
        let contentless_incremental_100_ms = contentless_incremental_started.elapsed().as_millis();
        assert!(!contentless_fts_search(&contentless_connection, "contentless refresh", 5)
            .expect("contentless incremental strict search")
            .is_empty());
        assert!(!contentless_fuzzy_search(&contentless_connection, "contentles refrsh", 5)
            .expect("contentless incremental fuzzy search")
            .ids
            .is_empty());

        let contentless_strict_started = Instant::now();
        for _ in 0..100 {
            contentless_fts_search(&contentless_connection, "contentless refresh", 5)
                .expect("warm contentless strict query");
        }
        let contentless_strict_query_100_us = contentless_strict_started.elapsed().as_micros();
        let contentless_fuzzy_started = Instant::now();
        for _ in 0..100 {
            contentless_fuzzy_search(&contentless_connection, "contentles refrsh", 5)
                .expect("warm contentless fuzzy query");
        }
        let contentless_fuzzy_query_100_us = contentless_fuzzy_started.elapsed().as_micros();

        println!(
            "production-search-10k source_bytes={} unicode_bytes={} trigram_bytes={} both_bytes={} contentless_bytes={} unicode_overhead_bytes={} trigram_overhead_bytes={} both_overhead_bytes={} contentless_overhead_bytes={} source_backfill_ms={} unicode_backfill_ms={} trigram_backfill_ms={} both_backfill_ms={} contentless_backfill_ms={} incremental_100_ms={} contentless_incremental_100_ms={} cleanup_100_ms={} repair_9900_ms={} strict_query_100_us={} fuzzy_query_100_us={} contentless_strict_query_100_us={} contentless_fuzzy_query_100_us={} strict_query_avg_us={:.1} fuzzy_query_avg_us={:.1} contentless_strict_query_avg_us={:.1} contentless_fuzzy_query_avg_us={:.1}",
            source_bytes,
            unicode_bytes,
            trigram_bytes,
            both_bytes,
            contentless_bytes,
            unicode_overhead,
            trigram_overhead,
            both_overhead,
            contentless_overhead,
            source_backfill_ms,
            unicode_backfill_ms,
            trigram_backfill_ms,
            both_backfill_ms,
            contentless_backfill_ms,
            incremental_100_ms,
            contentless_incremental_100_ms,
            cleanup_100_ms,
            repair_ms,
            strict_query_100_us,
            fuzzy_query_100_us,
            contentless_strict_query_100_us,
            contentless_fuzzy_query_100_us,
            strict_query_100_us as f64 / 100.0,
            fuzzy_query_100_us as f64 / 100.0,
            contentless_strict_query_100_us as f64 / 100.0,
            contentless_fuzzy_query_100_us as f64 / 100.0,
        );
    }
}
