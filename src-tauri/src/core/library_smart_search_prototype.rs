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
        for fixture in fixtures {
            current_statement.execute(params![
                fixture.id,
                fixture.filename,
                fixture.path,
                fixture.creator.as_deref().unwrap_or_default(),
                fixture.subtype.as_deref().unwrap_or_default(),
            ])?;

            let document = search_document(fixture);
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
        }
        assert!(
            fts_search(&connection, "---", 5)
                .expect("punctuation-only query")
                .is_empty(),
            "empty normalized query must not broaden into all rows"
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
        let mut fixtures = representative_fixtures();
        fixtures.extend((100..10_093).map(filler_fixture));
        assert_eq!(fixtures.len(), 10_000);

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

        println!(
            "smart-search-10k index_ms={} current_like_ms={} fts_ms={} current_recall_at_5={:.3} current_mrr={:.3} fts_recall_at_5={:.3} fts_mrr={:.3}",
            index_elapsed.as_millis(),
            like_elapsed.as_millis(),
            fts_elapsed.as_millis(),
            current.recall_at_five,
            current.mean_reciprocal_rank,
            fts.recall_at_five,
            fts.mean_reciprocal_rank,
        );

        assert_eq!(fts.recall_at_five, 1.0);
        assert_eq!(fts.mean_reciprocal_rank, 1.0);
        assert!(fts.recall_at_five > current.recall_at_five);
    }
}
