#![cfg(test)]

use std::{
    fs,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use rusqlite::{params, Connection, ErrorCode, OptionalExtension, Transaction};
use serde::Deserialize;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AuthorityIntent {
    SafeRemoval,
    Malware,
    Compatibility,
    CrashAttribution,
    DependencySafety,
    UpdateStatus,
    UncertainAuthority,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SearchIntentRoute {
    RetrievalSafe,
    AuthorityBlocked(AuthorityIntent),
}

#[derive(Debug, Clone, PartialEq)]
enum RoutedRetrieval<T> {
    Results(T),
    AuthorityBlocked(AuthorityIntent),
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

#[derive(Debug, Deserialize)]
struct PlayerQueryCorpus {
    version: u32,
    purpose: String,
    provenance: String,
    queries: Vec<PlayerQuery>,
}

#[derive(Debug, Deserialize)]
struct PlayerQuery {
    id: String,
    #[serde(rename = "class")]
    class_name: String,
    query: String,
    intent: String,
}

#[derive(Debug, Deserialize)]
struct PlayerTargetCorpus {
    version: u32,
    query_file: String,
    purpose: String,
    targets: Vec<PlayerTarget>,
}

#[derive(Debug, Deserialize)]
struct PlayerTarget {
    id: String,
    status: String,
    expected_ids: Vec<i64>,
    rationale: String,
}

#[derive(Debug, Clone, Copy, Default)]
struct PlayerEvaluationSummary {
    target_count: usize,
    hits_at_five: usize,
    reciprocal_rank_sum: f64,
    no_answer_count: usize,
    no_answer_correct: usize,
    unsupported_count: usize,
    candidate_total: usize,
    measured_queries: usize,
    elapsed_micros: u128,
}

impl PlayerEvaluationSummary {
    fn recall_at_five(self) -> f64 {
        if self.target_count == 0 {
            return 0.0;
        }
        self.hits_at_five as f64 / self.target_count as f64
    }

    fn mean_reciprocal_rank(self) -> f64 {
        if self.target_count == 0 {
            return 0.0;
        }
        self.reciprocal_rank_sum / self.target_count as f64
    }

    fn no_answer_precision(self) -> f64 {
        if self.no_answer_count == 0 {
            return 1.0;
        }
        self.no_answer_correct as f64 / self.no_answer_count as f64
    }

    fn average_candidates(self) -> f64 {
        if self.measured_queries == 0 {
            return 0.0;
        }
        self.candidate_total as f64 / self.measured_queries as f64
    }

    fn average_latency_micros(self) -> f64 {
        if self.measured_queries == 0 {
            return 0.0;
        }
        self.elapsed_micros as f64 / self.measured_queries as f64
    }
}

const TRIGRAM_CANDIDATE_LIMIT: usize = 80;
const STRICT_CANDIDATE_LIMIT: usize = 20;
const FUZZY_MIN_SCORE: f64 = 0.72;
const PRODUCTION_DB_BUSY_TIMEOUT: Duration = Duration::from_secs(5);
const SEARCH_STRUCTURE_MIGRATION_VERSION: i64 = 14;
const SEARCH_STRUCTURE_MIGRATION_NAME: &str = "library_contentless_search_v1";
const SEARCH_SCHEMA_VERSION: i64 = 1;
const SEARCH_DOCUMENT_VERSION: &str = "library-search-doc-v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SearchMigrationInterrupt {
    AfterSchemaCreation,
    AfterBackfill,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SearchEnsureOutcome {
    Installed,
    Ready,
    Rebuilt,
    Repaired,
}

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

fn open_production_like_search_connection(path: &Path) -> AppResult<Connection> {
    let connection = Connection::open(path)?;
    connection.pragma_update(None, "foreign_keys", "ON")?;
    connection.pragma_update(None, "journal_mode", "WAL")?;
    connection.busy_timeout(PRODUCTION_DB_BUSY_TIMEOUT)?;
    Ok(connection)
}

fn production_like_connection_settings(connection: &Connection) -> AppResult<(String, i64)> {
    let journal_mode = connection.query_row("PRAGMA journal_mode", [], |row| row.get::<_, String>(0))?;
    let busy_timeout_ms =
        connection.query_row("PRAGMA busy_timeout", [], |row| row.get::<_, i64>(0))?;
    Ok((journal_mode, busy_timeout_ms))
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

fn ensure_search_migration_table(connection: &Connection) -> AppResult<()> {
    connection.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
            version INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        );",
    )?;
    Ok(())
}

fn create_search_state_schema(connection: &Connection) -> AppResult<()> {
    connection.execute_batch(
        "CREATE TABLE production_search_state (
            id INTEGER PRIMARY KEY CHECK (id = 1),
            schema_version INTEGER NOT NULL,
            document_fingerprint TEXT NOT NULL,
            status TEXT NOT NULL CHECK (status IN ('ready', 'rebuild_required'))
        );",
    )?;
    Ok(())
}

fn sqlite_object_sql(connection: &Connection, name: &str) -> AppResult<Option<String>> {
    connection
        .query_row(
            "SELECT sql FROM sqlite_master WHERE name = ?1 AND type IN ('table', 'view')",
            params![name],
            |row| row.get::<_, Option<String>>(0),
        )
        .optional()
        .map(|value| value.flatten())
        .map_err(Into::into)
}

fn search_state_columns(connection: &Connection) -> AppResult<Option<Vec<String>>> {
    if sqlite_object_sql(connection, "production_search_state")?.is_none() {
        return Ok(None);
    }
    let mut statement = connection.prepare("PRAGMA table_info(production_search_state)")?;
    let columns = statement
        .query_map([], |row| row.get::<_, String>(1))?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Some(columns))
}

fn search_state_shape_is_valid(connection: &Connection) -> AppResult<bool> {
    let Some(columns) = search_state_columns(connection)? else {
        return Ok(false);
    };
    Ok([
        "id",
        "schema_version",
        "document_fingerprint",
        "status",
    ]
    .into_iter()
    .all(|required| columns.iter().any(|column| column == required)))
}

fn read_declared_search_schema_version(connection: &Connection) -> AppResult<Option<i64>> {
    let Some(columns) = search_state_columns(connection)? else {
        return Ok(None);
    };
    if !columns.iter().any(|column| column == "schema_version") {
        return Err(AppError::Message(
            "unrecognized search-state schema without schema_version; refusing destructive repair"
                .to_owned(),
        ));
    }
    if !columns.iter().any(|column| column == "id") {
        return Err(AppError::Message(
            "unrecognized search-state schema without id; refusing destructive repair".to_owned(),
        ));
    }
    connection
        .query_row(
            "SELECT schema_version FROM production_search_state WHERE id = 1",
            [],
            |row| row.get::<_, i64>(0),
        )
        .optional()
        .map_err(Into::into)
}

fn contentless_virtual_table_shape_is_valid(
    connection: &Connection,
    name: &str,
    tokenizer_marker: &str,
    required_columns: &[&str],
) -> AppResult<bool> {
    let Some(sql) = sqlite_object_sql(connection, name)? else {
        return Ok(false);
    };
    let compact = sql
        .to_ascii_lowercase()
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect::<String>();
    if !(compact.contains("createvirtualtable")
        && compact.contains("usingfts5")
        && compact.contains("content=''")
        && compact.contains("contentless_delete=1")
        && compact.contains(tokenizer_marker))
    {
        return Ok(false);
    }

    let pragma = format!("PRAGMA table_info({name})");
    let mut statement = connection.prepare(&pragma)?;
    let columns = statement
        .query_map([], |row| row.get::<_, String>(1))?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(columns
        .iter()
        .map(String::as_str)
        .eq(required_columns.iter().copied()))
}

fn contentless_search_owned_schema_is_valid(connection: &Connection) -> AppResult<bool> {
    Ok(contentless_virtual_table_shape_is_valid(
        connection,
        "production_contentless_fts",
        "unicode61",
        &[
            "filename",
            "creator",
            "aliases",
            "kind_subtype",
            "embedded_names",
            "family_hints",
            "resource_summary",
            "script_namespaces",
        ],
    )? && contentless_virtual_table_shape_is_valid(
        connection,
        "production_contentless_trigram",
        "trigram",
        &["search_text"],
    )? && search_state_shape_is_valid(connection)?)
}

fn any_search_owned_object_exists(connection: &Connection) -> AppResult<bool> {
    for name in [
        "production_contentless_fts",
        "production_contentless_trigram",
        "production_search_state",
    ] {
        if sqlite_object_sql(connection, name)?.is_some() {
            return Ok(true);
        }
    }
    Ok(false)
}

fn search_structure_migration_recorded(connection: &Connection) -> AppResult<bool> {
    ensure_search_migration_table(connection)?;
    connection
        .query_row(
            "SELECT EXISTS(
                SELECT 1 FROM schema_migrations
                WHERE version = ?1 AND name = ?2
            )",
            params![SEARCH_STRUCTURE_MIGRATION_VERSION, SEARCH_STRUCTURE_MIGRATION_NAME],
            |row| row.get::<_, bool>(0),
        )
        .map_err(Into::into)
}

fn read_search_state(connection: &Connection) -> AppResult<Option<(i64, String, String)>> {
    if !search_state_shape_is_valid(connection)? {
        return Ok(None);
    }
    connection
        .query_row(
            "SELECT schema_version, document_fingerprint, status
             FROM production_search_state
             WHERE id = 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()
        .map_err(Into::into)
}

fn write_search_state(
    transaction: &Transaction<'_>,
    fingerprint: &str,
    status: &str,
) -> AppResult<()> {
    transaction.execute(
        "INSERT INTO production_search_state(
            id, schema_version, document_fingerprint, status
         ) VALUES (1, ?1, ?2, ?3)
         ON CONFLICT(id) DO UPDATE SET
            schema_version = excluded.schema_version,
            document_fingerprint = excluded.document_fingerprint,
            status = excluded.status",
        params![SEARCH_SCHEMA_VERSION, fingerprint, status],
    )?;
    Ok(())
}

fn future_search_document_fingerprint(
    seed_version: &str,
    creator_learning_version: Option<&str>,
    category_override_version: Option<&str>,
) -> String {
    format!(
        "{SEARCH_DOCUMENT_VERSION}:{seed_version}:{}:{}",
        creator_learning_version.unwrap_or("none"),
        category_override_version.unwrap_or("none")
    )
}

fn drop_search_owned_objects_in_transaction(transaction: &Transaction<'_>) -> AppResult<()> {
    transaction.execute_batch(
        "DROP TABLE IF EXISTS production_contentless_fts;
         DROP TABLE IF EXISTS production_contentless_trigram;
         DROP TABLE IF EXISTS production_search_state;",
    )?;
    Ok(())
}

fn create_and_backfill_search_owned_objects(
    transaction: &Transaction<'_>,
    fingerprint: &str,
    interrupt: Option<SearchMigrationInterrupt>,
) -> AppResult<()> {
    create_contentless_search_schema(transaction)?;
    create_search_state_schema(transaction)?;
    if interrupt == Some(SearchMigrationInterrupt::AfterSchemaCreation) {
        return Err(AppError::Message(
            "synthetic search migration interruption after schema creation".to_owned(),
        ));
    }
    rebuild_contentless_search_in_transaction(transaction, None)?;
    if interrupt == Some(SearchMigrationInterrupt::AfterBackfill) {
        return Err(AppError::Message(
            "synthetic search migration interruption after backfill".to_owned(),
        ));
    }
    write_search_state(transaction, fingerprint, "ready")?;
    Ok(())
}

fn install_contentless_search_structure(
    connection: &mut Connection,
    fingerprint: &str,
    interrupt: Option<SearchMigrationInterrupt>,
) -> AppResult<()> {
    ensure_search_migration_table(connection)?;
    let transaction = connection.transaction()?;
    if any_search_owned_object_exists(&transaction)? {
        drop_search_owned_objects_in_transaction(&transaction)?;
    }
    create_and_backfill_search_owned_objects(&transaction, fingerprint, interrupt)?;
    transaction.execute(
        "INSERT INTO schema_migrations(version, name) VALUES (?1, ?2)",
        params![SEARCH_STRUCTURE_MIGRATION_VERSION, SEARCH_STRUCTURE_MIGRATION_NAME],
    )?;
    transaction.commit()?;
    Ok(())
}

fn repair_contentless_search_owned_objects(
    connection: &mut Connection,
    fingerprint: &str,
) -> AppResult<()> {
    let transaction = connection.transaction()?;
    drop_search_owned_objects_in_transaction(&transaction)?;
    create_and_backfill_search_owned_objects(&transaction, fingerprint, None)?;
    transaction.commit()?;
    Ok(())
}

fn rebuild_contentless_search_for_fingerprint(
    connection: &mut Connection,
    fingerprint: &str,
) -> AppResult<()> {
    let transaction = connection.transaction()?;
    rebuild_contentless_search_in_transaction(&transaction, None)?;
    write_search_state(&transaction, fingerprint, "ready")?;
    transaction.commit()?;
    Ok(())
}

fn ensure_contentless_search_ready(
    connection: &mut Connection,
    fingerprint: &str,
) -> AppResult<SearchEnsureOutcome> {
    ensure_search_migration_table(connection)?;
    let migration_recorded = search_structure_migration_recorded(connection)?;
    let declared_schema_version = read_declared_search_schema_version(connection)?;
    if let Some(schema_version) = declared_schema_version {
        if schema_version > SEARCH_SCHEMA_VERSION {
            return Err(AppError::Message(format!(
                "unsupported future search schema version {schema_version}; refusing destructive repair"
            )));
        }
    }
    let state = read_search_state(connection)?;

    if !migration_recorded {
        install_contentless_search_structure(connection, fingerprint, None)?;
        return Ok(SearchEnsureOutcome::Installed);
    }

    if !contentless_search_owned_schema_is_valid(connection)?
        || state.as_ref().is_none_or(|(version, _, _)| *version != SEARCH_SCHEMA_VERSION)
    {
        repair_contentless_search_owned_objects(connection, fingerprint)?;
        return Ok(SearchEnsureOutcome::Repaired);
    }

    let (_, saved_fingerprint, status) = state.expect("validated search state");
    if saved_fingerprint != fingerprint || status != "ready" {
        rebuild_contentless_search_for_fingerprint(connection, fingerprint)?;
        return Ok(SearchEnsureOutcome::Rebuilt);
    }

    Ok(SearchEnsureOutcome::Ready)
}

fn disable_contentless_search_objects(connection: &mut Connection) -> AppResult<()> {
    let transaction = connection.transaction()?;
    drop_search_owned_objects_in_transaction(&transaction)?;
    transaction.commit()?;
    Ok(())
}

fn resolve_contentless_refresh_scope(
    transaction: &Transaction<'_>,
    explicit_file_ids: &[i64],
    creator_ids: &[i64],
) -> AppResult<Vec<i64>> {
    let mut file_ids = explicit_file_ids.to_vec();
    let mut unique_creator_ids = creator_ids.to_vec();
    unique_creator_ids.sort_unstable();
    unique_creator_ids.dedup();

    for creator_id in unique_creator_ids {
        let mut statement = transaction.prepare(
            "SELECT id
             FROM production_files
             WHERE creator_id = ?1
               AND source_location IN ('mods', 'tray')
             ORDER BY id",
        )?;
        let creator_file_ids = statement
            .query_map(params![creator_id], |row| row.get::<_, i64>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        file_ids.extend(creator_file_ids);
    }

    file_ids.sort_unstable();
    file_ids.dedup();
    Ok(file_ids)
}

fn refresh_contentless_search_scope_in_transaction(
    transaction: &Transaction<'_>,
    explicit_file_ids: &[i64],
    creator_ids: &[i64],
) -> AppResult<Vec<i64>> {
    let file_ids = resolve_contentless_refresh_scope(transaction, explicit_file_ids, creator_ids)?;
    for file_id in &file_ids {
        refresh_contentless_search_file_in_transaction(transaction, *file_id)?;
    }
    Ok(file_ids)
}

fn contentless_alias_owner(
    transaction: &Transaction<'_>,
    alias_name: &str,
) -> AppResult<Option<i64>> {
    transaction
        .query_row(
            "SELECT creator_id
             FROM production_user_creator_aliases
             WHERE alias_name = ?1",
            params![alias_name],
            |row| row.get::<_, i64>(0),
        )
        .optional()
        .map_err(Into::into)
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

fn route_search_intent(query: &str) -> SearchIntentRoute {
    let tokens = normalized_search_tokens(query);
    let normalized = tokens.join(" ");
    let has = |needle: &str| tokens.iter().any(|token| token == needle);
    let has_any = |needles: &[&str]| needles.iter().any(|needle| has(needle));

    if has_any(&["malware", "virus", "trojan", "ransomware"]) {
        return SearchIntentRoute::AuthorityBlocked(AuthorityIntent::Malware);
    }

    let removal = has_any(&["delete", "deleting", "deleted", "remove", "removing", "removed"]);
    let dependency_language = has_any(&["dependency", "dependencies", "dependent", "dependents"])
        || normalized.contains("other mod")
        || normalized.contains("other mods")
        || normalized.contains("without breaking")
        || normalized.contains("break another")
        || normalized.contains("break any other");
    if removal && dependency_language {
        return SearchIntentRoute::AuthorityBlocked(AuthorityIntent::DependencySafety);
    }

    if removal
        && (has_any(&["safe", "safely", "okay", "ok", "should", "can", "definitely"])
            || normalized.contains("without risk"))
    {
        return SearchIntentRoute::AuthorityBlocked(AuthorityIntent::SafeRemoval);
    }

    if has_any(&["compatible", "compatibility", "incompatible"])
        && has_any(&["patch", "update", "version", "latest", "current"])
    {
        return SearchIntentRoute::AuthorityBlocked(AuthorityIntent::Compatibility);
    }

    if has_any(&["crash", "crashes", "crashing", "crashed"])
        && (has_any(&[
            "cause",
            "causes",
            "caused",
            "causing",
            "culprit",
            "responsible",
            "definitely",
            "which",
        ]) || normalized.contains("making my game"))
    {
        return SearchIntentRoute::AuthorityBlocked(AuthorityIntent::CrashAttribution);
    }

    if has_any(&["update", "updates", "updated", "updating", "outdated"])
        && (has_any(&[
            "need",
            "needs",
            "needed",
            "require",
            "requires",
            "required",
            "definitely",
            "current",
            "latest",
            "now",
            "which",
        ]) || normalized.contains("right now"))
    {
        return SearchIntentRoute::AuthorityBlocked(AuthorityIntent::UpdateStatus);
    }

    let certainty = has_any(&[
        "definitely",
        "guaranteed",
        "guarantee",
        "certain",
        "certainly",
        "safe",
        "safely",
        "unsafe",
    ]);
    let risky_subject = removal
        || has_any(&[
            "break",
            "breaking",
            "broken",
            "crash",
            "crashing",
            "compatible",
            "compatibility",
            "update",
            "outdated",
            "dependency",
            "mesh",
            "patch",
            "safe",
            "unsafe",
        ]);
    if certainty && risky_subject {
        return SearchIntentRoute::AuthorityBlocked(AuthorityIntent::UncertainAuthority);
    }

    SearchIntentRoute::RetrievalSafe
}

fn route_before_retrieval<T, F>(query: &str, retrieve: F) -> RoutedRetrieval<T>
where
    F: FnOnce() -> T,
{
    match route_search_intent(query) {
        SearchIntentRoute::RetrievalSafe => RoutedRetrieval::Results(retrieve()),
        SearchIntentRoute::AuthorityBlocked(intent) => RoutedRetrieval::AuthorityBlocked(intent),
    }
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

fn load_player_query_corpora() -> (PlayerQueryCorpus, PlayerTargetCorpus) {
    let query_source = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/test-fixtures/library-smart-search-player-queries-v1.json"
    ));
    let target_source = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/test-fixtures/library-smart-search-player-targets-v1.json"
    ));
    let queries = serde_json::from_str(query_source).expect("valid frozen player query corpus");
    let targets = serde_json::from_str(target_source).expect("valid player target mapping");
    (queries, targets)
}

fn evaluate_player_queries<F>(
    queries: &[PlayerQuery],
    targets: &[PlayerTarget],
    class_filter: Option<&str>,
    mut search: F,
) -> PlayerEvaluationSummary
where
    F: FnMut(&str) -> FuzzySearchDiagnostics,
{
    let mut summary = PlayerEvaluationSummary::default();

    for query in queries
        .iter()
        .filter(|query| class_filter.is_none_or(|wanted| query.class_name == wanted))
    {
        let target = targets
            .iter()
            .find(|target| target.id == query.id)
            .unwrap_or_else(|| panic!("missing target mapping for {}", query.id));
        match target.status.as_str() {
            "unsupported" => {
                summary.unsupported_count += 1;
            }
            "target" | "no_answer" => {
                let started = Instant::now();
                let result = search(&query.query);
                summary.elapsed_micros += started.elapsed().as_micros();
                summary.candidate_total += result.candidate_count;
                summary.measured_queries += 1;

                if target.status == "no_answer" {
                    summary.no_answer_count += 1;
                    if result.ids.is_empty() {
                        summary.no_answer_correct += 1;
                    }
                    continue;
                }

                summary.target_count += 1;
                if let Some(position) = result
                    .ids
                    .iter()
                    .position(|id| target.expected_ids.contains(id))
                {
                    if position < 5 {
                        summary.hits_at_five += 1;
                    }
                    summary.reciprocal_rank_sum += 1.0 / (position as f64 + 1.0);
                }
            }
            status => panic!("unknown player target status {status:?} for {}", query.id),
        }
    }

    summary
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

    fn setup_source_only_migration_database(path: &std::path::Path) -> Connection {
        let fixtures = representative_fixtures();
        let mut connection =
            open_production_like_search_connection(path).expect("production-like migration database");
        create_production_shape_schema(&connection, ProductionSearchShape::SOURCE_ONLY)
            .expect("source-only schema");
        seed_production_source(&mut connection, &fixtures).expect("seed source truth");
        ensure_search_migration_table(&connection).expect("migration ledger");
        connection
    }

    fn installed_source_row_count(connection: &Connection) -> i64 {
        connection
            .query_row(
                "SELECT COUNT(*) FROM production_files WHERE source_location IN ('mods', 'tray')",
                [],
                |row| row.get(0),
            )
            .expect("installed source row count")
    }

    #[test]
    fn frozen_player_query_corpus_is_separate_complete_and_untuned() {
        let query_source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/test-fixtures/library-smart-search-player-queries-v1.json"
        ));
        assert!(
            !query_source.contains("expected_ids"),
            "the frozen player wording file must not contain benchmark answers"
        );

        let (queries, targets) = load_player_query_corpora();
        assert_eq!(queries.version, 1);
        assert_eq!(targets.version, 1);
        assert_eq!(queries.queries.len(), 36);
        assert_eq!(targets.targets.len(), 36);
        assert!(queries.purpose.contains("no expected fixture IDs"));
        assert!(queries.provenance.contains("separate Sims mod-management"));
        assert_eq!(
            targets.query_file,
            "library-smart-search-player-queries-v1.json"
        );
        assert!(targets.purpose.contains("only after"));

        let mut query_ids = queries
            .queries
            .iter()
            .map(|query| query.id.as_str())
            .collect::<Vec<_>>();
        query_ids.sort_unstable();
        query_ids.dedup();
        assert_eq!(query_ids.len(), 36, "player query ids must stay unique");

        let mut target_ids = targets
            .targets
            .iter()
            .map(|target| target.id.as_str())
            .collect::<Vec<_>>();
        target_ids.sort_unstable();
        target_ids.dedup();
        assert_eq!(target_ids.len(), 36, "target mapping ids must stay unique");
        assert_eq!(query_ids, target_ids, "every frozen query must map exactly once");

        let classes = [
            "exact_name",
            "creator_alias",
            "typo_partial",
            "vague_lexical",
            "semantic_paraphrase",
            "authority_no_answer",
        ];
        for class_name in classes {
            assert_eq!(
                queries
                    .queries
                    .iter()
                    .filter(|query| query.class_name == class_name)
                    .count(),
                6,
                "each frozen query class must contain six entries"
            );
        }
        assert!(queries
            .queries
            .iter()
            .all(|query| !query.query.trim().is_empty() && !query.intent.trim().is_empty()));
        assert!(targets
            .targets
            .iter()
            .all(|target| !target.rationale.trim().is_empty()));

        let target_count = targets
            .targets
            .iter()
            .filter(|target| target.status == "target")
            .count();
        let unsupported_count = targets
            .targets
            .iter()
            .filter(|target| target.status == "unsupported")
            .count();
        let no_answer_count = targets
            .targets
            .iter()
            .filter(|target| target.status == "no_answer")
            .count();
        assert_eq!((target_count, unsupported_count, no_answer_count), (15, 15, 6));
        for target in &targets.targets {
            match target.status.as_str() {
                "target" => assert!(!target.expected_ids.is_empty()),
                "unsupported" | "no_answer" => assert!(target.expected_ids.is_empty()),
                other => panic!("unknown frozen player target status: {other}"),
            }
        }
    }

    #[test]
    fn independent_player_phrasing_baseline_reports_without_tuning_search() {
        let fixtures = representative_fixtures();
        let mut connection = Connection::open_in_memory().expect("memory sqlite");
        index_fixtures(&mut connection, &fixtures).expect("index representative fixtures");
        let (queries, targets) = load_player_query_corpora();

        let strict = evaluate_player_queries(&queries.queries, &targets.targets, None, |query| {
            let ids = fts_search(&connection, query, STRICT_CANDIDATE_LIMIT)
                .expect("strict player-query search");
            FuzzySearchDiagnostics {
                candidate_count: ids.len(),
                ids,
            }
        });
        let fuzzy = evaluate_player_queries(&queries.queries, &targets.targets, None, |query| {
            fuzzy_trigram_search(&connection, query, STRICT_CANDIDATE_LIMIT)
                .expect("fuzzy player-query search")
        });

        println!(
            "player-search-v1 overall targets={} unsupported={} no_answer={} strict_recall_at_5={:.3} strict_mrr={:.3} strict_no_answer_precision={:.3} strict_avg_candidates={:.1} strict_avg_us={:.1} fuzzy_recall_at_5={:.3} fuzzy_mrr={:.3} fuzzy_no_answer_precision={:.3} fuzzy_avg_candidates={:.1} fuzzy_avg_us={:.1}",
            fuzzy.target_count,
            fuzzy.unsupported_count,
            fuzzy.no_answer_count,
            strict.recall_at_five(),
            strict.mean_reciprocal_rank(),
            strict.no_answer_precision(),
            strict.average_candidates(),
            strict.average_latency_micros(),
            fuzzy.recall_at_five(),
            fuzzy.mean_reciprocal_rank(),
            fuzzy.no_answer_precision(),
            fuzzy.average_candidates(),
            fuzzy.average_latency_micros(),
        );

        for class_name in [
            "exact_name",
            "creator_alias",
            "typo_partial",
            "vague_lexical",
            "semantic_paraphrase",
            "authority_no_answer",
        ] {
            let strict_class = evaluate_player_queries(
                &queries.queries,
                &targets.targets,
                Some(class_name),
                |query| {
                    let ids = fts_search(&connection, query, STRICT_CANDIDATE_LIMIT)
                        .expect("strict player-query class search");
                    FuzzySearchDiagnostics {
                        candidate_count: ids.len(),
                        ids,
                    }
                },
            );
            let fuzzy_class = evaluate_player_queries(
                &queries.queries,
                &targets.targets,
                Some(class_name),
                |query| {
                    fuzzy_trigram_search(&connection, query, STRICT_CANDIDATE_LIMIT)
                        .expect("fuzzy player-query class search")
                },
            );
            println!(
                "player-search-v1 class={} targets={} unsupported={} no_answer={} strict_recall_at_5={:.3} strict_mrr={:.3} strict_no_answer_precision={:.3} fuzzy_recall_at_5={:.3} fuzzy_mrr={:.3} fuzzy_no_answer_precision={:.3} fuzzy_avg_candidates={:.1}",
                class_name,
                fuzzy_class.target_count,
                fuzzy_class.unsupported_count,
                fuzzy_class.no_answer_count,
                strict_class.recall_at_five(),
                strict_class.mean_reciprocal_rank(),
                strict_class.no_answer_precision(),
                fuzzy_class.recall_at_five(),
                fuzzy_class.mean_reciprocal_rank(),
                fuzzy_class.no_answer_precision(),
                fuzzy_class.average_candidates(),
            );
        }

        for query in &queries.queries {
            let target = targets
                .targets
                .iter()
                .find(|target| target.id == query.id)
                .expect("mapped frozen player query");
            if target.status == "unsupported" {
                continue;
            }
            let strict_ids = fts_search(&connection, &query.query, 5)
                .expect("strict player-query outcome search");
            let fuzzy_result = fuzzy_trigram_search(&connection, &query.query, 5)
                .expect("fuzzy player-query outcome search");
            let strict_ok = if target.status == "no_answer" {
                strict_ids.is_empty()
            } else {
                strict_ids.iter().any(|id| target.expected_ids.contains(id))
            };
            let fuzzy_ok = if target.status == "no_answer" {
                fuzzy_result.ids.is_empty()
            } else {
                fuzzy_result
                    .ids
                    .iter()
                    .any(|id| target.expected_ids.contains(id))
            };
            if !strict_ok || !fuzzy_ok {
                println!(
                    "player-search-v1 outcome id={} class={} status={} expected={:?} strict={:?} fuzzy={:?} strict_ok={} fuzzy_ok={}",
                    query.id,
                    query.class_name,
                    target.status,
                    target.expected_ids,
                    strict_ids,
                    fuzzy_result.ids,
                    strict_ok,
                    fuzzy_ok,
                );
            }
        }

        assert_eq!(fuzzy.target_count, 15);
        assert_eq!(fuzzy.unsupported_count, 15);
        assert_eq!(fuzzy.no_answer_count, 6);
        assert_eq!(fuzzy.measured_queries, 21);
        for value in [
            strict.recall_at_five(),
            strict.mean_reciprocal_rank(),
            strict.no_answer_precision(),
            fuzzy.recall_at_five(),
            fuzzy.mean_reciprocal_rank(),
            fuzzy.no_answer_precision(),
        ] {
            assert!((0.0..=1.0).contains(&value));
        }
        assert!(fuzzy.average_candidates() <= (TRIGRAM_CANDIDATE_LIMIT + STRICT_CANDIDATE_LIMIT) as f64);
    }

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
    fn production_like_wal_contentless_sync_is_atomic_across_connections() {
        let temp = tempfile::tempdir().expect("temporary WAL proof directory");
        let path = temp.path().join("contentless-wal-atomic.sqlite");
        let fixtures = representative_fixtures();

        let mut setup =
            open_production_like_search_connection(&path).expect("production-like setup connection");
        create_production_shape_schema(&setup, ProductionSearchShape::SOURCE_ONLY)
            .expect("source schema");
        create_contentless_search_schema(&setup).expect("contentless schema");
        seed_production_source(&mut setup, &fixtures).expect("seed source");
        rebuild_contentless_search(&mut setup).expect("initial contentless rebuild");
        assert_eq!(
            production_like_connection_settings(&setup).expect("setup connection settings"),
            ("wal".to_owned(), 5_000)
        );
        drop(setup);

        let mut reader =
            open_production_like_search_connection(&path).expect("production-like reader");
        let mut writer =
            open_production_like_search_connection(&path).expect("production-like writer");
        let fresh_reader =
            open_production_like_search_connection(&path).expect("production-like fresh reader");

        let reader_transaction = reader.transaction().expect("reader snapshot");
        assert!(contentless_fts_search(&reader_transaction, "ConcurrencyMarker", 5)
            .expect("old snapshot before writer")
            .is_empty());

        {
            let writer_transaction = writer.transaction().expect("writer transaction");
            writer_transaction
                .execute(
                    "UPDATE production_files SET subtype = 'ConcurrencyMarker' WHERE id = 5",
                    [],
                )
                .expect("update source inside writer transaction");
            refresh_contentless_search_file_in_transaction(&writer_transaction, 5)
                .expect("refresh search in same writer transaction");

            assert!(contentless_fts_search(&reader_transaction, "ConcurrencyMarker", 5)
                .expect("old reader must not see uncommitted search state")
                .is_empty());
            writer_transaction
                .commit()
                .expect("commit source and search atomically");
        }

        assert!(contentless_fts_search(&reader_transaction, "ConcurrencyMarker", 5)
            .expect("open WAL reader snapshot stays on previous complete state")
            .is_empty());
        assert_eq!(
            contentless_fts_search(&fresh_reader, "ConcurrencyMarker", 5)
                .expect("fresh reader sees committed complete state"),
            vec![5]
        );

        reader_transaction.commit().expect("finish old reader snapshot");
        assert_eq!(
            contentless_fts_search(&reader, "ConcurrencyMarker", 5)
                .expect("reader sees new state after ending old snapshot"),
            vec![5]
        );
    }

    #[test]
    fn production_like_single_writer_contention_and_contentless_repair_are_bounded() {
        let temp = tempfile::tempdir().expect("temporary writer proof directory");
        let path = temp.path().join("contentless-writer-repair.sqlite");
        let fixtures = representative_fixtures();

        let mut setup =
            open_production_like_search_connection(&path).expect("production-like setup connection");
        create_production_shape_schema(&setup, ProductionSearchShape::SOURCE_ONLY)
            .expect("source schema");
        create_contentless_search_schema(&setup).expect("contentless schema");
        seed_production_source(&mut setup, &fixtures).expect("seed source");
        rebuild_contentless_search(&mut setup).expect("initial contentless rebuild");
        drop(setup);

        let mut writer =
            open_production_like_search_connection(&path).expect("production-like first writer");
        let contender =
            open_production_like_search_connection(&path).expect("production-like contender");
        let observer =
            open_production_like_search_connection(&path).expect("production-like observer");
        assert_eq!(
            production_like_connection_settings(&contender).expect("contender connection settings"),
            ("wal".to_owned(), 5_000)
        );

        let writer_transaction = writer.transaction().expect("first writer transaction");
        writer_transaction
            .execute("UPDATE production_files SET path = path WHERE id = 4", [])
            .expect("first writer acquires SQLite write lock");

        contender
            .busy_timeout(Duration::ZERO)
            .expect("bounded zero-timeout contention probe");
        let contention_error = contender
            .execute("UPDATE production_files SET path = path WHERE id = 5", [])
            .expect_err("second writer must not bypass SQLite single-writer lock");
        match contention_error {
            rusqlite::Error::SqliteFailure(error, _) => assert!(matches!(
                error.code,
                ErrorCode::DatabaseBusy | ErrorCode::DatabaseLocked
            )),
            other => panic!("expected SQLite busy/locked error, got {other:?}"),
        }
        writer_transaction
            .rollback()
            .expect("release first writer lock without changing source");
        contender
            .busy_timeout(PRODUCTION_DB_BUSY_TIMEOUT)
            .expect("restore production-like busy timeout after bounded probe");
        contender
            .execute("UPDATE production_files SET path = path WHERE id = 5", [])
            .expect("second writer succeeds after first writer releases lock");

        writer
            .execute(
                "UPDATE production_files SET subtype = 'RepairMarker' WHERE id = 5",
                [],
            )
            .expect("commit source change before simulated interrupted search refresh");
        assert!(contentless_fts_search(&observer, "RepairMarker", 5)
            .expect("observer sees previous complete search index before repair")
            .is_empty());

        let interrupted = rebuild_contentless_search_with_interrupt(&mut writer, 2)
            .expect_err("interrupted search rebuild should roll back");
        assert!(interrupted
            .to_string()
            .contains("synthetic contentless search-index rebuild interruption"));
        assert!(contentless_fts_search(&observer, "RepairMarker", 5)
            .expect("interrupted rebuild remains invisible to another connection")
            .is_empty());
        assert_eq!(
            contentless_fts_search(&observer, "BetterExceptions", 5)
                .expect("previous complete index remains readable after interruption"),
            vec![5]
        );

        rebuild_contentless_search(&mut writer).expect("deterministic full repair");
        assert_eq!(
            contentless_fts_search(&observer, "RepairMarker", 5)
                .expect("observer sees repaired complete index"),
            vec![5]
        );
    }

    #[test]
    fn authority_router_blocks_before_retrieval_and_keeps_lookup_intent_separate() {
        let (queries, _) = load_player_query_corpora();
        let expected_authority = [
            ("authority_01", AuthorityIntent::SafeRemoval),
            ("authority_02", AuthorityIntent::Malware),
            ("authority_03", AuthorityIntent::Compatibility),
            ("authority_04", AuthorityIntent::CrashAttribution),
            ("authority_05", AuthorityIntent::DependencySafety),
            ("authority_06", AuthorityIntent::UpdateStatus),
        ];

        for (query_id, expected_intent) in expected_authority {
            let query = queries
                .queries
                .iter()
                .find(|query| query.id == query_id)
                .expect("frozen authority query");
            assert_eq!(
                route_search_intent(&query.query),
                SearchIntentRoute::AuthorityBlocked(expected_intent),
                "authority routing must be explicit for {}",
                query.query
            );

            let retrieval_called = std::cell::Cell::new(false);
            let routed = route_before_retrieval(&query.query, || {
                retrieval_called.set(true);
                vec![999_i64]
            });
            assert_eq!(routed, RoutedRetrieval::AuthorityBlocked(expected_intent));
            assert!(
                !retrieval_called.get(),
                "blocked authority intent must never invoke retrieval"
            );
        }

        for query in queries
            .queries
            .iter()
            .filter(|query| query.class_name != "authority_no_answer")
        {
            assert_eq!(
                route_search_intent(&query.query),
                SearchIntentRoute::RetrievalSafe,
                "ordinary lookup/semantic wording should remain eligible for retrieval: {}",
                query.query
            );
        }

        let cautious_variants = [
            ("can i safely remove this package", AuthorityIntent::SafeRemoval),
            (
                "could deleting this break another mod",
                AuthorityIntent::DependencySafety,
            ),
            (
                "is this mod compatible with the current patch",
                AuthorityIntent::Compatibility,
            ),
            (
                "which package caused my game to crash",
                AuthorityIntent::CrashAttribution,
            ),
            (
                "does this package need updating right now",
                AuthorityIntent::UpdateStatus,
            ),
            (
                "is this guaranteed safe after the patch",
                AuthorityIntent::UncertainAuthority,
            ),
        ];
        for (query, expected_intent) in cautious_variants {
            assert_eq!(
                route_search_intent(query),
                SearchIntentRoute::AuthorityBlocked(expected_intent),
                "uncertain or authoritative wording must fail closed: {query}"
            );
        }

        for query in [
            "better exceptions",
            "bettr exceptions",
            "crash report helper",
            "mods with update notes",
            "the mod that helps me figure out what broke my game",
        ] {
            let retrieval_called = std::cell::Cell::new(false);
            let routed = route_before_retrieval(query, || {
                retrieval_called.set(true);
                vec![1_i64]
            });
            assert_eq!(routed, RoutedRetrieval::Results(vec![1]));
            assert!(retrieval_called.get(), "retrieval-safe intent should invoke retrieval");
        }
    }

    #[test]
    fn contentless_search_structure_and_backfill_rollback_together_under_wal() {
        let temp = tempfile::tempdir().expect("temporary migration proof directory");
        let path = temp.path().join("search-migration-rollback.sqlite");
        let mut connection = setup_source_only_migration_database(&path);
        let source_count = installed_source_row_count(&connection);
        let fingerprint = future_search_document_fingerprint(
            "seed-v1",
            Some("creator-v1"),
            Some("category-v1"),
        );
        assert_eq!(
            production_like_connection_settings(&connection).expect("production-like settings"),
            ("wal".to_owned(), 5_000)
        );

        for interrupt in [
            SearchMigrationInterrupt::AfterSchemaCreation,
            SearchMigrationInterrupt::AfterBackfill,
        ] {
            let error = install_contentless_search_structure(
                &mut connection,
                &fingerprint,
                Some(interrupt),
            )
            .expect_err("deliberately interrupted migration must roll back");
            assert!(error.to_string().contains("synthetic search migration interruption"));
            assert!(
                !any_search_owned_object_exists(&connection)
                    .expect("search objects absent after rollback"),
                "SQLite DDL and backfill must roll back with the migration transaction"
            );
            assert!(
                !search_structure_migration_recorded(&connection)
                    .expect("migration marker absent after rollback")
            );
            assert_eq!(installed_source_row_count(&connection), source_count);
        }

        install_contentless_search_structure(&mut connection, &fingerprint, None)
            .expect("complete structural install");
        assert!(contentless_search_owned_schema_is_valid(&connection)
            .expect("installed search schema valid"));
        assert!(search_structure_migration_recorded(&connection)
            .expect("structural migration recorded only after complete install"));
        assert_eq!(
            read_search_state(&connection).expect("search state"),
            Some((SEARCH_SCHEMA_VERSION, fingerprint.clone(), "ready".to_owned()))
        );
        assert_eq!(
            contentless_fts_search(&connection, "BetterExceptions", 5)
                .expect("installed search query"),
            vec![5]
        );
        assert_eq!(installed_source_row_count(&connection), source_count);
        assert_eq!(
            ensure_contentless_search_ready(&mut connection, &fingerprint)
                .expect("idempotent startup ensure"),
            SearchEnsureOutcome::Ready
        );
    }

    #[test]
    fn contentless_search_startup_ensure_repairs_missing_and_malformed_owned_objects() {
        let temp = tempfile::tempdir().expect("temporary repair proof directory");
        let path = temp.path().join("search-startup-repair.sqlite");
        let mut connection = setup_source_only_migration_database(&path);
        let source_count = installed_source_row_count(&connection);
        let fingerprint = future_search_document_fingerprint("seed-v1", None, None);
        assert_eq!(
            ensure_contentless_search_ready(&mut connection, &fingerprint)
                .expect("first startup ensure installs search"),
            SearchEnsureOutcome::Installed
        );

        connection
            .execute_batch("DROP TABLE production_contentless_trigram;")
            .expect("simulate missing trigram table");
        assert_eq!(
            ensure_contentless_search_ready(&mut connection, &fingerprint)
                .expect("repair missing trigram table"),
            SearchEnsureOutcome::Repaired
        );
        assert!(contentless_search_owned_schema_is_valid(&connection)
            .expect("repaired schema valid"));

        connection
            .execute_batch(
                "DROP TABLE production_contentless_fts;
                 CREATE TABLE production_contentless_fts (not_an_fts_column TEXT);",
            )
            .expect("simulate malformed ordinary table under search-owned name");
        assert_eq!(
            ensure_contentless_search_ready(&mut connection, &fingerprint)
                .expect("repair malformed search-owned table"),
            SearchEnsureOutcome::Repaired
        );
        assert_eq!(
            contentless_fts_search(&connection, "BetterExceptions", 5)
                .expect("search works after malformed-object repair"),
            vec![5]
        );

        connection
            .execute_batch(
                "DROP TABLE production_contentless_fts;
                 CREATE VIRTUAL TABLE production_contentless_fts USING fts5(
                    filename,
                    tokenize = 'unicode61 remove_diacritics 2',
                    content = '',
                    contentless_delete = 1
                 );",
            )
            .expect("simulate superficially valid FTS table with missing search columns");
        assert_eq!(
            ensure_contentless_search_ready(&mut connection, &fingerprint)
                .expect("repair malformed FTS column shape"),
            SearchEnsureOutcome::Repaired
        );
        assert_eq!(
            contentless_fts_search(&connection, "BetterExceptions", 5)
                .expect("search works after malformed FTS-column repair"),
            vec![5]
        );

        connection
            .execute_batch(
                "DROP TABLE production_contentless_fts;
                 CREATE VIRTUAL TABLE production_contentless_fts USING fts5(
                    creator,
                    filename,
                    aliases,
                    kind_subtype,
                    embedded_names,
                    family_hints,
                    resource_summary,
                    script_namespaces,
                    tokenize = 'unicode61 remove_diacritics 2',
                    content = '',
                    contentless_delete = 1
                 );",
            )
            .expect("simulate FTS table with the right names in the wrong weighted order");
        assert_eq!(
            ensure_contentless_search_ready(&mut connection, &fingerprint)
                .expect("repair FTS column-order drift"),
            SearchEnsureOutcome::Repaired
        );
        assert_eq!(
            contentless_fts_search(&connection, "BetterExceptions", 5)
                .expect("search works after FTS column-order repair"),
            vec![5]
        );

        connection
            .execute_batch("DROP TABLE production_search_state;")
            .expect("simulate missing derived state marker");
        assert_eq!(
            ensure_contentless_search_ready(&mut connection, &fingerprint)
                .expect("repair missing state marker"),
            SearchEnsureOutcome::Repaired
        );
        assert!(search_structure_migration_recorded(&connection)
            .expect("repair never rewrites structural history"));
        assert_eq!(installed_source_row_count(&connection), source_count);
    }

    #[test]
    fn contentless_search_fingerprint_rebuild_and_feature_disable_preserve_source_truth() {
        let temp = tempfile::tempdir().expect("temporary fingerprint proof directory");
        let path = temp.path().join("search-fingerprint-disable.sqlite");
        let mut connection = setup_source_only_migration_database(&path);
        let source_count = installed_source_row_count(&connection);
        let fingerprint_v1 = future_search_document_fingerprint(
            "seed-v1",
            Some("creator-v1"),
            Some("category-v1"),
        );
        assert_eq!(
            ensure_contentless_search_ready(&mut connection, &fingerprint_v1)
                .expect("initial search install"),
            SearchEnsureOutcome::Installed
        );

        let twistedmexi_id: i64 = connection
            .query_row(
                "SELECT id FROM production_creators WHERE canonical_name = 'TwistedMexi'",
                [],
                |row| row.get(0),
            )
            .expect("TwistedMexi creator id");
        connection
            .execute(
                "INSERT INTO production_user_creator_aliases(creator_id, alias_name)
                 VALUES (?1, 'ExceptionDoctor')",
                params![twistedmexi_id],
            )
            .expect("simulate learned alias meaning change");
        assert!(contentless_fts_search(&connection, "ExceptionDoctor", 5)
            .expect("stale search before fingerprint rebuild")
            .is_empty());

        let fingerprint_v2 = future_search_document_fingerprint(
            "seed-v1",
            Some("creator-v2"),
            Some("category-v1"),
        );
        assert_ne!(fingerprint_v1, fingerprint_v2);
        assert_eq!(
            ensure_contentless_search_ready(&mut connection, &fingerprint_v2)
                .expect("fingerprint mismatch rebuild"),
            SearchEnsureOutcome::Rebuilt
        );
        assert_eq!(
            contentless_fts_search(&connection, "ExceptionDoctor", 5)
                .expect("learned alias visible after rebuild"),
            vec![5]
        );
        assert_eq!(
            ensure_contentless_search_ready(&mut connection, &fingerprint_v2)
                .expect("second ensure is idempotent"),
            SearchEnsureOutcome::Ready
        );

        disable_contentless_search_objects(&mut connection)
            .expect("feature-disable drops only search-owned objects");
        assert!(!any_search_owned_object_exists(&connection)
            .expect("search objects disabled"));
        assert!(search_structure_migration_recorded(&connection)
            .expect("feature disable preserves migration history"));
        assert_eq!(installed_source_row_count(&connection), source_count);

        assert_eq!(
            ensure_contentless_search_ready(&mut connection, &fingerprint_v2)
                .expect("re-enable repairs disposable search objects"),
            SearchEnsureOutcome::Repaired
        );
        assert_eq!(
            contentless_fts_search(&connection, "ExceptionDoctor", 5)
                .expect("rebuilt search after re-enable"),
            vec![5]
        );
        assert_eq!(installed_source_row_count(&connection), source_count);
    }

    #[test]
    fn contentless_search_future_versions_fail_closed_without_destructive_repair() {
        let temp = tempfile::tempdir().expect("temporary future-version proof directory");
        let path = temp.path().join("search-future-version.sqlite");
        let mut connection = setup_source_only_migration_database(&path);
        let source_count = installed_source_row_count(&connection);
        let fingerprint = future_search_document_fingerprint("seed-v1", None, None);
        ensure_contentless_search_ready(&mut connection, &fingerprint)
            .expect("initial search install");
        connection
            .execute(
                "UPDATE production_search_state SET schema_version = 99 WHERE id = 1",
                [],
            )
            .expect("simulate future search schema version");

        let error = ensure_contentless_search_ready(&mut connection, &fingerprint)
            .expect_err("unknown future search schema must fail closed");
        assert!(error
            .to_string()
            .contains("unsupported future search schema version 99"));
        assert_eq!(
            read_search_state(&connection)
                .expect("future search state remains readable")
                .expect("future state row")
                .0,
            99
        );
        assert!(contentless_search_owned_schema_is_valid(&connection)
            .expect("future-owned schema was not destructively altered"));
        assert_eq!(
            contentless_fts_search(&connection, "BetterExceptions", 5)
                .expect("existing future-version search remains untouched"),
            vec![5]
        );
        assert_eq!(installed_source_row_count(&connection), source_count);
        assert!(search_structure_migration_recorded(&connection)
            .expect("future-version block preserves migration history"));

        connection
            .execute_batch(
                "DROP TABLE production_search_state;
                 CREATE TABLE production_search_state (
                    id INTEGER PRIMARY KEY,
                    schema_version INTEGER NOT NULL
                 );
                 INSERT INTO production_search_state(id, schema_version) VALUES (1, 99);",
            )
            .expect("simulate future state shape that differs from the current schema");
        assert!(!search_state_shape_is_valid(&connection)
            .expect("future changed state shape is not current-version valid"));
        let changed_shape_error = ensure_contentless_search_ready(&mut connection, &fingerprint)
            .expect_err("future version must block repair even when other state columns changed");
        assert!(changed_shape_error
            .to_string()
            .contains("unsupported future search schema version 99"));
        assert_eq!(
            read_declared_search_schema_version(&connection)
                .expect("future version remains readable"),
            Some(99)
        );
        assert_eq!(
            contentless_fts_search(&connection, "BetterExceptions", 5)
                .expect("future changed state shape does not destroy search objects"),
            vec![5]
        );

        connection
            .execute_batch(
                "DROP TABLE production_search_state;
                 CREATE TABLE production_search_state (id INTEGER PRIMARY KEY);",
            )
            .expect("simulate unrecognized state shape with no readable schema version");
        let unrecognized_error = ensure_contentless_search_ready(&mut connection, &fingerprint)
            .expect_err("unrecognized state shape must fail closed instead of guessing repair");
        assert!(unrecognized_error
            .to_string()
            .contains("without schema_version; refusing destructive repair"));
        assert!(sqlite_object_sql(&connection, "production_search_state")
            .expect("unrecognized state table remains inspectable")
            .is_some());
        assert_eq!(installed_source_row_count(&connection), source_count);

        let second_path = temp.path().join("search-structural-version-conflict.sqlite");
        let mut conflicting = setup_source_only_migration_database(&second_path);
        conflicting
            .execute(
                "INSERT INTO schema_migrations(version, name) VALUES (?1, 'unknown_future_owner')",
                params![SEARCH_STRUCTURE_MIGRATION_VERSION],
            )
            .expect("simulate conflicting future structural ownership");
        let conflict_error = ensure_contentless_search_ready(&mut conflicting, &fingerprint)
            .expect_err("migration-version ownership conflict must not be overwritten");
        assert!(conflict_error.to_string().to_ascii_lowercase().contains("unique")
            || conflict_error.to_string().to_ascii_lowercase().contains("constraint"));
        assert!(!any_search_owned_object_exists(&conflicting)
            .expect("failed conflicting install rolls back search-owned DDL"));
        assert_eq!(
            installed_source_row_count(&conflicting),
            representative_fixtures().len() as i64
        );
    }

    #[test]
    fn contentless_creator_learning_scope_is_exact_old_new_and_explicit_union() {
        let temp = tempfile::tempdir().expect("temporary creator-scope proof directory");
        let path = temp.path().join("search-creator-scope.sqlite");
        let mut connection = setup_source_only_migration_database(&path);
        let fingerprint_v1 = future_search_document_fingerprint(
            "seed-v1",
            Some("creator-v1"),
            Some("category-v1"),
        );
        assert_eq!(
            ensure_contentless_search_ready(&mut connection, &fingerprint_v1)
                .expect("initial search install"),
            SearchEnsureOutcome::Installed
        );

        let twisted_mexi_id: i64 = connection
            .query_row(
                "SELECT id FROM production_creators WHERE canonical_name = 'TwistedMexi'",
                [],
                |row| row.get(0),
            )
            .expect("TwistedMexi creator id");
        let simpliciaty_id: i64 = connection
            .query_row(
                "SELECT id FROM production_creators WHERE canonical_name = 'Simpliciaty'",
                [],
                |row| row.get(0),
            )
            .expect("Simpliciaty creator id");

        {
            let transaction = connection.transaction().expect("initial alias transaction");
            transaction
                .execute(
                    "INSERT INTO production_user_creator_aliases(creator_id, alias_name)
                     VALUES (?1, 'MexiWorkshop')",
                    params![twisted_mexi_id],
                )
                .expect("add initial learned alias");
            let refreshed = refresh_contentless_search_scope_in_transaction(
                &transaction,
                &[],
                &[twisted_mexi_id],
            )
            .expect("refresh initial creator scope");
            assert_eq!(refreshed, vec![5]);
            write_search_state(&transaction, &fingerprint_v1, "ready")
                .expect("advance search state only after complete initial scope refresh");
            transaction.commit().expect("commit initial alias scope");
        }
        assert_eq!(
            contentless_fts_search(&connection, "MexiWorkshop", 10)
                .expect("initial alias search"),
            vec![5]
        );

        let fingerprint_v2 = future_search_document_fingerprint(
            "seed-v1",
            Some("creator-v2"),
            Some("category-v1"),
        );
        let refreshed = {
            let transaction = connection.transaction().expect("alias reassignment transaction");
            let old_alias_owner = contentless_alias_owner(&transaction, "MexiWorkshop")
                .expect("read old alias owner before upsert")
                .expect("old alias owner exists");
            assert_eq!(old_alias_owner, twisted_mexi_id);

            transaction
                .execute(
                    "INSERT INTO production_user_creator_aliases(creator_id, alias_name)
                     VALUES (?1, 'MexiWorkshop')
                     ON CONFLICT(alias_name) DO UPDATE SET creator_id = excluded.creator_id",
                    params![simpliciaty_id],
                )
                .expect("reassign learned alias");
            transaction
                .execute(
                    "UPDATE production_files SET creator_id = ?1 WHERE id = 2",
                    params![simpliciaty_id],
                )
                .expect("mirror explicit creator reassignment for selected file");

            let refreshed = refresh_contentless_search_scope_in_transaction(
                &transaction,
                &[2, 2],
                &[old_alias_owner, simpliciaty_id, simpliciaty_id],
            )
            .expect("refresh exact old/new creator plus explicit file union");
            assert_eq!(refreshed, vec![1, 2, 5]);
            write_search_state(&transaction, &fingerprint_v2, "ready")
                .expect("advance search state only after complete creator-learning scope refresh");
            transaction.commit().expect("commit creator-learning-like boundary");
            refreshed
        };
        assert_eq!(refreshed, vec![1, 2, 5]);

        let reassigned_alias_matches = contentless_fts_search(&connection, "MexiWorkshop", 10)
            .expect("reassigned alias search");
        assert!(reassigned_alias_matches.contains(&1));
        assert!(reassigned_alias_matches.contains(&2));
        assert!(!reassigned_alias_matches.contains(&5));
        assert_eq!(
            read_search_state(&connection).expect("search state after creator learning"),
            Some((SEARCH_SCHEMA_VERSION, fingerprint_v2.clone(), "ready".to_owned()))
        );

        let untouched_terms = [
            (3_i64, "MCCC"),
            (4_i64, "Realistic Childbirth"),
            (6_i64, "Minimal Main Menu"),
            (7_i64, "Thai Translation"),
        ];
        for (id, term) in untouched_terms {
            assert_eq!(
                contentless_fts_search(&connection, term, 10)
                    .expect("unrelated search row remains intact"),
                vec![id]
            );
        }

        let row_count_before: i64 = connection
            .query_row("SELECT COUNT(*) FROM production_contentless_fts", [], |row| row.get(0))
            .expect("contentless row count before idempotent refresh");
        {
            let transaction = connection.transaction().expect("repeat creator scope transaction");
            let repeated = refresh_contentless_search_scope_in_transaction(
                &transaction,
                &[2],
                &[twisted_mexi_id, simpliciaty_id],
            )
            .expect("repeat exact creator scope");
            assert_eq!(repeated, vec![1, 2, 5]);
            write_search_state(&transaction, &fingerprint_v2, "ready")
                .expect("re-assert ready only after repeated complete creator scope refresh");
            transaction.commit().expect("commit repeated creator scope");
        }
        let row_count_after: i64 = connection
            .query_row("SELECT COUNT(*) FROM production_contentless_fts", [], |row| row.get(0))
            .expect("contentless row count after idempotent refresh");
        assert_eq!(row_count_after, row_count_before);
    }

    #[test]
    fn contentless_scoped_metadata_membership_and_rollback_stay_exact() {
        let temp = tempfile::tempdir().expect("temporary scoped-boundary proof directory");
        let path = temp.path().join("search-scoped-boundaries.sqlite");
        let mut connection = setup_source_only_migration_database(&path);
        let fingerprint_v1 = future_search_document_fingerprint(
            "seed-v1",
            Some("creator-v1"),
            Some("category-v1"),
        );
        ensure_contentless_search_ready(&mut connection, &fingerprint_v1)
            .expect("initial search install");

        let searchable_insights = FileInsights {
            embedded_names: vec!["Aurora Catalog Name".to_owned()],
            family_hints: vec!["Aurora family".to_owned()],
            resource_summary: vec!["Aurora surface resource".to_owned()],
            script_namespaces: vec!["aurora.catalog".to_owned()],
            ..FileInsights::default()
        };
        let fingerprint_v2 = future_search_document_fingerprint(
            "seed-v1",
            Some("creator-v1"),
            Some("category-v2"),
        );
        {
            let transaction = connection.transaction().expect("category/insight transaction");
            transaction
                .execute(
                    "UPDATE production_files
                     SET kind = 'BuildBuy', subtype = 'Reclassified Surface', insights = ?1
                     WHERE id = 1",
                    params![serde_json::to_string(&searchable_insights).expect("serialize insights")],
                )
                .expect("update exact searchable metadata row");
            let refreshed = refresh_contentless_search_scope_in_transaction(
                &transaction,
                &[1, 1],
                &[],
            )
            .expect("refresh exact changed file");
            assert_eq!(refreshed, vec![1]);
            write_search_state(&transaction, &fingerprint_v2, "ready")
                .expect("advance search state only after exact metadata refresh completes");
            transaction.commit().expect("commit exact metadata boundary");
        }
        assert_eq!(
            contentless_fts_search(&connection, "Reclassified Surface", 5)
                .expect("category term search"),
            vec![1]
        );
        assert_eq!(
            contentless_fts_search(&connection, "Aurora Catalog", 5)
                .expect("searchable insight term search"),
            vec![1]
        );

        let rows_before_repeat: i64 = connection
            .query_row("SELECT COUNT(*) FROM production_contentless_fts", [], |row| row.get(0))
            .expect("row count before repeated exact file refresh");
        {
            let transaction = connection.transaction().expect("repeat exact-file transaction");
            let repeated = refresh_contentless_search_scope_in_transaction(
                &transaction,
                &[1, 1, 1],
                &[],
            )
            .expect("repeat exact file refresh");
            assert_eq!(repeated, vec![1]);
            write_search_state(&transaction, &fingerprint_v2, "ready")
                .expect("re-assert ready only after repeated exact-file refresh");
            transaction.commit().expect("commit repeated exact-file refresh");
        }
        let rows_after_repeat: i64 = connection
            .query_row("SELECT COUNT(*) FROM production_contentless_fts", [], |row| row.get(0))
            .expect("row count after repeated exact file refresh");
        assert_eq!(rows_after_repeat, rows_before_repeat);

        {
            let transaction = connection.transaction().expect("membership transition transaction");
            transaction
                .execute(
                    "UPDATE production_files SET source_location = 'downloads' WHERE id = 7",
                    [],
                )
                .expect("move exact source row outside installed search scope");
            assert_eq!(
                refresh_contentless_search_scope_in_transaction(&transaction, &[7], &[])
                    .expect("remove exact out-of-scope search row"),
                vec![7]
            );
            transaction.commit().expect("commit membership transition");
        }
        assert!(contentless_fts_search(&connection, "Thai Translation", 5)
            .expect("downloads transition removes search membership")
            .is_empty());

        {
            let transaction = connection.transaction().expect("delete boundary transaction");
            transaction
                .execute("DELETE FROM production_files WHERE id = 2", [])
                .expect("delete exact source row");
            assert_eq!(
                refresh_contentless_search_scope_in_transaction(&transaction, &[2], &[])
                    .expect("remove exact deleted search row"),
                vec![2]
            );
            transaction.commit().expect("commit delete boundary");
        }
        assert!(contentless_fts_search(&connection, "Baysic", 5)
            .expect("deleted source no longer searchable")
            .is_empty());

        let state_before_rollback = read_search_state(&connection)
            .expect("search state before rollback")
            .expect("ready search state");
        {
            let transaction = connection.transaction().expect("interrupted scoped transaction");
            transaction
                .execute(
                    "UPDATE production_files
                     SET subtype = 'Transient Search State'
                     WHERE id = 1",
                    [],
                )
                .expect("temporary category change");
            transaction
                .execute(
                    "UPDATE production_files SET source_location = 'downloads' WHERE id = 3",
                    [],
                )
                .expect("temporary membership change");
            let interrupted_fingerprint = future_search_document_fingerprint(
                "seed-v1",
                Some("creator-v1"),
                Some("category-interrupted"),
            );
            assert_eq!(
                refresh_contentless_search_scope_in_transaction(
                    &transaction,
                    &[1, 3],
                    &[],
                )
                .expect("refresh temporary scoped changes"),
                vec![1, 3]
            );
            write_search_state(&transaction, &interrupted_fingerprint, "ready")
                .expect("advance temporary search state only after temporary scope refresh");
            assert_eq!(
                contentless_fts_search(&transaction, "Transient Search State", 5)
                    .expect("temporary search state visible only inside transaction"),
                vec![1]
            );
            transaction.rollback().expect("simulate interruption before commit");
        }

        let restored_subtype: Option<String> = connection
            .query_row("SELECT subtype FROM production_files WHERE id = 1", [], |row| row.get(0))
            .expect("rolled-back subtype");
        assert_eq!(restored_subtype.as_deref(), Some("Reclassified Surface"));
        let restored_source: String = connection
            .query_row("SELECT source_location FROM production_files WHERE id = 3", [], |row| row.get(0))
            .expect("rolled-back source location");
        assert_eq!(restored_source, "mods");
        assert!(contentless_fts_search(&connection, "Transient Search State", 5)
            .expect("rolled-back search row is invisible")
            .is_empty());
        assert_eq!(
            contentless_fts_search(&connection, "MCCC", 5)
                .expect("pre-interruption search row remains"),
            vec![3]
        );
        assert_eq!(
            read_search_state(&connection)
                .expect("search state after rollback")
                .expect("ready state after rollback"),
            state_before_rollback
        );
    }

    #[test]
    fn contentless_scan_replacement_and_search_state_share_one_wal_transaction() {
        let temp = tempfile::tempdir().expect("temporary scanner-boundary proof directory");
        let path = temp.path().join("search-scan-boundary.sqlite");
        let mut writer = setup_source_only_migration_database(&path);
        let fingerprint = future_search_document_fingerprint(
            "seed-v1",
            Some("creator-v1"),
            Some("category-v1"),
        );
        ensure_contentless_search_ready(&mut writer, &fingerprint)
            .expect("initial search install");
        let observer = open_production_like_search_connection(&path)
            .expect("separate WAL observer connection");
        observer.execute_batch("BEGIN").expect("open observer snapshot");
        assert_eq!(
            contentless_fts_search(&observer, "MCCC", 5).expect("pin old search snapshot"),
            vec![3]
        );

        let replacement = SearchFixture {
            id: 501,
            filename: "ReplacementScanItem.package".to_owned(),
            path: "/fixture/Mods/ReplacementScanItem.package".to_owned(),
            creator: Some("ReplacementCreator".to_owned()),
            creator_aliases: Vec::new(),
            kind: "BuildBuy".to_owned(),
            subtype: Some("Furniture".to_owned()),
            insights: FileInsights {
                embedded_names: vec!["Replacement Scan Sofa".to_owned()],
                family_hints: vec!["replacement scan family".to_owned()],
                ..FileInsights::default()
            },
        };
        {
            let transaction = writer.transaction().expect("scanner replacement transaction");
            transaction
                .execute(
                    "DELETE FROM production_files WHERE source_location IN ('mods', 'tray')",
                    [],
                )
                .expect("clear installed source truth");
            insert_production_source_fixture(&transaction, &replacement, "mods")
                .expect("insert replacement installed source");
            assert_eq!(
                rebuild_contentless_search_in_transaction(&transaction, None)
                    .expect("full contentless rebuild inside scanner transaction"),
                1
            );
            write_search_state(&transaction, &fingerprint, "ready")
                .expect("keep search state in scanner transaction");
            transaction.commit().expect("commit source+search+state together");
        }

        assert_eq!(
            contentless_fts_search(&observer, "MCCC", 5)
                .expect("old reader stays on old complete snapshot"),
            vec![3]
        );
        assert!(contentless_fts_search(&observer, "Replacement Scan Sofa", 5)
            .expect("old reader cannot see new snapshot yet")
            .is_empty());
        observer.execute_batch("COMMIT").expect("end old observer snapshot");
        assert!(contentless_fts_search(&observer, "MCCC", 5)
            .expect("fresh observer view drops old scan rows")
            .is_empty());
        assert_eq!(
            contentless_fts_search(&observer, "Replacement Scan Sofa", 5)
                .expect("fresh observer view sees replacement scan"),
            vec![501]
        );

        let interrupted = SearchFixture {
            id: 601,
            filename: "InterruptedScanItem.package".to_owned(),
            path: "/fixture/Mods/InterruptedScanItem.package".to_owned(),
            creator: Some("InterruptedCreator".to_owned()),
            creator_aliases: Vec::new(),
            kind: "Gameplay".to_owned(),
            subtype: Some("Script Mod".to_owned()),
            insights: FileInsights {
                embedded_names: vec!["Interrupted Scan Marker".to_owned()],
                ..FileInsights::default()
            },
        };
        {
            let transaction = writer.transaction().expect("interrupted scanner transaction");
            transaction
                .execute(
                    "DELETE FROM production_files WHERE source_location IN ('mods', 'tray')",
                    [],
                )
                .expect("clear installed source before interrupted replacement");
            insert_production_source_fixture(&transaction, &interrupted, "mods")
                .expect("insert interrupted replacement source");
            rebuild_contentless_search_in_transaction(&transaction, None)
                .expect("rebuild temporary interrupted search state");
            write_search_state(
                &transaction,
                &future_search_document_fingerprint(
                    "seed-v1",
                    Some("creator-interrupted"),
                    Some("category-v1"),
                ),
                "ready",
            )
            .expect("write temporary interrupted search state");
            transaction.rollback().expect("simulate scanner interruption before commit");
        }
        assert_eq!(
            contentless_fts_search(&writer, "Replacement Scan Sofa", 5)
                .expect("last committed replacement survives interruption"),
            vec![501]
        );
        assert!(contentless_fts_search(&writer, "Interrupted Scan Marker", 5)
            .expect("interrupted replacement never becomes visible")
            .is_empty());
        assert_eq!(
            writer
                .query_row(
                    "SELECT COUNT(*) FROM production_files WHERE source_location IN ('mods', 'tray')",
                    [],
                    |row| row.get::<_, i64>(0),
                )
                .expect("committed installed source count"),
            1
        );
        assert_eq!(
            read_search_state(&writer).expect("search state after interrupted scan"),
            Some((SEARCH_SCHEMA_VERSION, fingerprint, "ready".to_owned()))
        );
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
        assert!(!commands_source.contains("route_search_intent"));
        assert!(!commands_source.contains("open_production_like_search_connection"));
        assert!(!commands_source.contains("ensure_contentless_search_ready"));
        assert!(!commands_source.contains("install_contentless_search_structure"));
        assert!(!commands_source.contains("disable_contentless_search_objects"));
        assert!(!commands_source.contains("resolve_contentless_refresh_scope"));
        assert!(!commands_source.contains("refresh_contentless_search_scope_in_transaction"));
        assert!(!commands_source.contains("contentless_alias_owner"));

        let core_source = include_str!("mod.rs");
        assert!(core_source.contains(
            "#[cfg(test)]\npub mod library_smart_search_prototype;"
        ));
        let prototype_source = include_str!("library_smart_search_prototype.rs");
        assert!(prototype_source.starts_with("#![cfg(test)]"));
    }

    #[test]
    #[ignore = "10k frozen player-phrasing search benchmark; run pnpm run test:library:stress"]
    fn large_independent_player_phrasing_baseline_reports_relevance_and_latency() {
        let mut fixtures = representative_fixtures();
        fixtures.extend((100..10_093).map(filler_fixture));
        assert_eq!(fixtures.len(), 10_000);

        let mut connection = Connection::open_in_memory().expect("memory sqlite");
        let index_started = Instant::now();
        index_fixtures(&mut connection, &fixtures).expect("index 10k player-search fixtures");
        let index_ms = index_started.elapsed().as_millis();
        let (queries, targets) = load_player_query_corpora();

        let strict = evaluate_player_queries(&queries.queries, &targets.targets, None, |query| {
            let ids = fts_search(&connection, query, STRICT_CANDIDATE_LIMIT)
                .expect("10k strict player search");
            FuzzySearchDiagnostics {
                candidate_count: ids.len(),
                ids,
            }
        });
        let fuzzy = evaluate_player_queries(&queries.queries, &targets.targets, None, |query| {
            fuzzy_trigram_search(&connection, query, STRICT_CANDIDATE_LIMIT)
                .expect("10k fuzzy player search")
        });

        println!(
            "player-search-v1-10k index_ms={} targets={} unsupported={} no_answer={} strict_recall_at_5={:.3} strict_mrr={:.3} strict_no_answer_precision={:.3} strict_avg_candidates={:.1} strict_avg_us={:.1} fuzzy_recall_at_5={:.3} fuzzy_mrr={:.3} fuzzy_no_answer_precision={:.3} fuzzy_avg_candidates={:.1} fuzzy_avg_us={:.1}",
            index_ms,
            fuzzy.target_count,
            fuzzy.unsupported_count,
            fuzzy.no_answer_count,
            strict.recall_at_five(),
            strict.mean_reciprocal_rank(),
            strict.no_answer_precision(),
            strict.average_candidates(),
            strict.average_latency_micros(),
            fuzzy.recall_at_five(),
            fuzzy.mean_reciprocal_rank(),
            fuzzy.no_answer_precision(),
            fuzzy.average_candidates(),
            fuzzy.average_latency_micros(),
        );

        for class_name in [
            "exact_name",
            "creator_alias",
            "typo_partial",
            "vague_lexical",
            "semantic_paraphrase",
            "authority_no_answer",
        ] {
            let strict_class = evaluate_player_queries(
                &queries.queries,
                &targets.targets,
                Some(class_name),
                |query| {
                    let ids = fts_search(&connection, query, STRICT_CANDIDATE_LIMIT)
                        .expect("10k strict player class search");
                    FuzzySearchDiagnostics {
                        candidate_count: ids.len(),
                        ids,
                    }
                },
            );
            let fuzzy_class = evaluate_player_queries(
                &queries.queries,
                &targets.targets,
                Some(class_name),
                |query| {
                    fuzzy_trigram_search(&connection, query, STRICT_CANDIDATE_LIMIT)
                        .expect("10k fuzzy player class search")
                },
            );
            println!(
                "player-search-v1-10k class={} targets={} unsupported={} no_answer={} strict_recall_at_5={:.3} strict_mrr={:.3} strict_no_answer_precision={:.3} fuzzy_recall_at_5={:.3} fuzzy_mrr={:.3} fuzzy_no_answer_precision={:.3} fuzzy_avg_candidates={:.1} fuzzy_avg_us={:.1}",
                class_name,
                fuzzy_class.target_count,
                fuzzy_class.unsupported_count,
                fuzzy_class.no_answer_count,
                strict_class.recall_at_five(),
                strict_class.mean_reciprocal_rank(),
                strict_class.no_answer_precision(),
                fuzzy_class.recall_at_five(),
                fuzzy_class.mean_reciprocal_rank(),
                fuzzy_class.no_answer_precision(),
                fuzzy_class.average_candidates(),
                fuzzy_class.average_latency_micros(),
            );
        }

        assert_eq!(fuzzy.target_count, 15);
        assert_eq!(fuzzy.unsupported_count, 15);
        assert_eq!(fuzzy.no_answer_count, 6);
        assert_eq!(fuzzy.measured_queries, 21);
        assert!(fuzzy.average_candidates() <= (TRIGRAM_CANDIDATE_LIMIT + STRICT_CANDIDATE_LIMIT) as f64);
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
