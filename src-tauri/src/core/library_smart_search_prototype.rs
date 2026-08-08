#![cfg(test)]

use std::time::Instant;

use rusqlite::{params, Connection};

use crate::{error::AppResult, models::FileInsights};

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
    fn smart_search_prototype_stays_out_of_normal_command_registration() {
        let commands_source = include_str!("../commands/mod.rs");
        assert!(!commands_source.contains("library_smart_search_prototype"));
        assert!(!commands_source.contains("library_smart_search_fts"));
        assert!(!commands_source.contains("library_smart_search_trigram"));

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
}
