use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    path::{Path, PathBuf},
};

use rusqlite::Connection;

use crate::{
    error::AppResult,
    models::{
        CreatorAuditFile, CreatorAuditGroup, CreatorAuditQuery, CreatorAuditResponse, FileInsights,
        LibrarySettings,
    },
    seed::{normalize_key, SeedPack},
};

const MAX_SAMPLE_FILES: usize = 8;
const MAX_UNRESOLVED_SAMPLES: usize = 12;
const MAX_ALIAS_SAMPLES: usize = 6;
const MIN_GROUP_SCORE: f64 = 0.52;
const HIGH_CONFIDENCE_GROUP: f64 = 0.86;
const GENERIC_FOLDERS: &[&str] = &[
    "mods",
    "tray",
    "downloads",
    "buildbuy",
    "build_buy",
    "build-buy",
    "cas",
    "gameplay",
    "scriptmods",
    "script_mods",
    "scripts",
    "review",
    "unknown",
    "misc",
    "unsorted",
];
const FAMILY_SUFFIX_TOKENS: &[&str] = &[
    "accessory",
    "bed",
    "bookshelf",
    "chair",
    "changingtable",
    "coloringbook",
    "counter",
    "crib",
    "desk",
    "dresser",
    "eyeliner",
    "hair",
    "jacket",
    "lashes",
    "lipstick",
    "menu",
    "nails",
    "preset",
    "shelf",
    "shorts",
    "skirt",
    "table",
    "tattoo",
    "top",
    "trait",
    "vase",
];

#[derive(Debug, Clone)]
struct AuditRecord {
    id: i64,
    filename: String,
    path: String,
    kind: String,
    subtype: Option<String>,
    confidence: f64,
    source_location: String,
    current_creator: Option<String>,
    insights: FileInsights,
}

#[derive(Debug, Clone)]
struct CandidateSignal {
    creator_name: String,
    normalized_key: String,
    score: f64,
    signal: String,
    alias_sample: Option<String>,
    known_creator: bool,
}

#[derive(Debug, Default)]
struct CandidateAggregate {
    creator_name: String,
    score: f64,
    known_creator: bool,
    signals: BTreeSet<String>,
    alias_samples: BTreeSet<String>,
}

#[derive(Debug)]
struct Selection {
    creator_name: String,
    normalized_key: String,
    score: f64,
    known_creator: bool,
    signals: Vec<String>,
    alias_samples: Vec<String>,
}

#[derive(Debug, Default)]
struct GroupAccumulator {
    suggested_creator: String,
    confidence_sum: f64,
    known_creator: bool,
    file_ids: Vec<i64>,
    source_signals: BTreeSet<String>,
    alias_samples: BTreeSet<String>,
    sample_files: Vec<CreatorAuditFile>,
    kind_counts: BTreeMap<String, i64>,
}

pub fn load_creator_audit(
    connection: &Connection,
    settings: &LibrarySettings,
    seed_pack: &SeedPack,
    query: CreatorAuditQuery,
) -> AppResult<CreatorAuditResponse> {
    let records = load_candidate_records(connection)?;
    let total_candidate_files = records.len() as i64;
    let mut groups = HashMap::<String, GroupAccumulator>::new();
    let mut unresolved_samples = Vec::new();
    let mut grouped_files = 0_i64;
    let mut unresolved_files = 0_i64;
    let mut root_loose_files = 0_i64;

    for record in records {
        if is_root_loose(&record, settings) {
            root_loose_files += 1;
        }

        let selection = select_candidate(&record, settings, seed_pack);
        let Some(selection) = selection.filter(|selected| selected.score >= MIN_GROUP_SCORE) else {
            unresolved_files += 1;
            if unresolved_samples.len() < MAX_UNRESOLVED_SAMPLES {
                unresolved_samples.push(CreatorAuditFile {
                    id: record.id,
                    filename: record.filename,
                    path: record.path,
                    kind: record.kind,
                    subtype: record.subtype,
                    confidence: record.confidence,
                    source_location: record.source_location,
                    current_creator: record.current_creator,
                    alias_samples: Vec::new(),
                    match_reasons: vec!["No strong creator cluster signal yet".to_owned()],
                });
            }
            continue;
        };

        grouped_files += 1;
        let entry = groups
            .entry(selection.normalized_key.clone())
            .or_insert_with(|| GroupAccumulator {
                suggested_creator: selection.creator_name.clone(),
                known_creator: selection.known_creator,
                ..GroupAccumulator::default()
            });

        entry.suggested_creator = selection.creator_name.clone();
        entry.confidence_sum += selection.score;
        entry.known_creator |= selection.known_creator;
        entry.file_ids.push(record.id);
        for signal in selection.signals {
            entry.source_signals.insert(signal);
        }
        for alias in selection.alias_samples {
            if entry.alias_samples.len() < MAX_ALIAS_SAMPLES {
                entry.alias_samples.insert(alias);
            }
        }
        *entry.kind_counts.entry(record.kind.clone()).or_insert(0) += 1;
        if entry.sample_files.len() < MAX_SAMPLE_FILES {
            entry.sample_files.push(CreatorAuditFile {
                id: record.id,
                filename: record.filename,
                path: record.path,
                kind: record.kind,
                subtype: record.subtype,
                confidence: record.confidence,
                source_location: record.source_location,
                current_creator: record.current_creator,
                alias_samples: entry.alias_samples.iter().cloned().collect(),
                match_reasons: entry.source_signals.iter().cloned().collect(),
            });
        }
    }

    let search = query
        .search
        .as_ref()
        .map(|value| value.trim().to_lowercase())
        .filter(|value| !value.is_empty());
    let min_group_size = query.min_group_size.unwrap_or(2).max(1);
    let limit = query.limit.unwrap_or(48).max(1) as usize;

    let mut group_items = groups
        .into_iter()
        .filter_map(|(id, group)| {
            let item_count = group.file_ids.len() as i64;
            if item_count < min_group_size {
                unresolved_files += item_count;
                return None;
            }

            let dominant_kind = group
                .kind_counts
                .iter()
                .max_by_key(|(_, count)| *count)
                .map(|(kind, _)| kind.clone())
                .unwrap_or_else(|| "Unknown".to_owned());
            let confidence = (group.confidence_sum / (group.file_ids.len() as f64)).clamp(0.0, 0.99);

            let audit_group = CreatorAuditGroup {
                id,
                suggested_creator: group.suggested_creator,
                confidence,
                known_creator: group.known_creator,
                item_count,
                dominant_kind,
                source_signals: group.source_signals.into_iter().collect(),
                alias_samples: group.alias_samples.into_iter().take(MAX_ALIAS_SAMPLES).collect(),
                file_ids: group.file_ids,
                sample_files: group.sample_files,
            };

            if let Some(search) = &search {
                let haystack = format!(
                    "{} {} {}",
                    audit_group.suggested_creator,
                    audit_group.alias_samples.join(" "),
                    audit_group
                        .sample_files
                        .iter()
                        .map(|file| file.filename.as_str())
                        .collect::<Vec<_>>()
                        .join(" ")
                )
                .to_lowercase();
                if !haystack.contains(search) {
                    return None;
                }
            }

            Some(audit_group)
        })
        .collect::<Vec<_>>();

    group_items.sort_by(|left, right| {
        right
            .item_count
            .cmp(&left.item_count)
            .then_with(|| right.confidence.total_cmp(&left.confidence))
            .then_with(|| left.suggested_creator.cmp(&right.suggested_creator))
    });

    let total_groups = group_items.len() as i64;
    let high_confidence_groups = group_items
        .iter()
        .filter(|group| group.confidence >= HIGH_CONFIDENCE_GROUP)
        .count() as i64;
    group_items.truncate(limit);

    Ok(CreatorAuditResponse {
        total_candidate_files,
        grouped_files,
        unresolved_files,
        root_loose_files,
        total_groups,
        high_confidence_groups,
        groups: group_items,
        unresolved_samples,
    })
}

pub fn load_creator_group_files(
    connection: &Connection,
    settings: &LibrarySettings,
    seed_pack: &SeedPack,
    group_id: &str,
) -> AppResult<Vec<CreatorAuditFile>> {
    let records = load_candidate_records(connection)?;
    let mut matching_records = Vec::new();
    let mut alias_samples = BTreeSet::new();
    let mut source_signals = BTreeSet::new();

    for record in records {
        let Some(selection) = select_candidate(&record, settings, seed_pack)
            .filter(|selected| {
                selected.score >= MIN_GROUP_SCORE && selected.normalized_key == group_id
            })
        else {
            continue;
        };

        for signal in &selection.signals {
            source_signals.insert(signal.clone());
        }
        for alias in &selection.alias_samples {
            if alias_samples.len() < MAX_ALIAS_SAMPLES {
                alias_samples.insert(alias.clone());
            }
        }

        matching_records.push((record, selection.score));
    }

    let shared_aliases = alias_samples
        .into_iter()
        .take(MAX_ALIAS_SAMPLES)
        .collect::<Vec<_>>();
    let shared_reasons = source_signals.into_iter().collect::<Vec<_>>();

    matching_records.sort_by(|left, right| {
        right
            .1
            .total_cmp(&left.1)
            .then_with(|| left.0.filename.cmp(&right.0.filename))
    });

    Ok(matching_records
        .into_iter()
        .map(|(record, _)| CreatorAuditFile {
            id: record.id,
            filename: record.filename,
            path: record.path,
            kind: record.kind,
            subtype: record.subtype,
            confidence: record.confidence,
            source_location: record.source_location,
            current_creator: record.current_creator,
            alias_samples: shared_aliases.clone(),
            match_reasons: shared_reasons.clone(),
        })
        .collect())
}

fn load_candidate_records(connection: &Connection) -> AppResult<Vec<AuditRecord>> {
    let mut statement = connection.prepare(
        "SELECT
            f.id,
            f.filename,
            f.path,
            f.kind,
            f.subtype,
            f.confidence,
            f.source_location,
            c.canonical_name,
            f.insights
         FROM files f
         LEFT JOIN creators c ON f.creator_id = c.id
         WHERE f.extension IN ('.package', '.ts4script')
           AND c.id IS NULL
         ORDER BY f.filename COLLATE NOCASE",
    )?;

    let rows = statement
        .query_map([], |row| {
            Ok(AuditRecord {
                id: row.get(0)?,
                filename: row.get(1)?,
                path: row.get(2)?,
                kind: row.get(3)?,
                subtype: row.get(4)?,
                confidence: row.get(5)?,
                source_location: row.get(6)?,
                current_creator: row.get(7)?,
                insights: serde_json::from_str::<FileInsights>(&row.get::<_, String>(8)?)
                    .unwrap_or_default(),
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(rows)
}

fn select_candidate(
    record: &AuditRecord,
    settings: &LibrarySettings,
    seed_pack: &SeedPack,
) -> Option<Selection> {
    let mut aggregates = HashMap::<String, CandidateAggregate>::new();

    for signal in collect_signals(record, settings, seed_pack) {
        let entry = aggregates
            .entry(signal.normalized_key.clone())
            .or_insert_with(|| CandidateAggregate {
                creator_name: signal.creator_name.clone(),
                ..CandidateAggregate::default()
            });
        entry.creator_name = signal.creator_name.clone();
        entry.score += signal.score;
        entry.known_creator |= signal.known_creator;
        entry.signals.insert(signal.signal);
        if let Some(alias) = signal.alias_sample {
            entry.alias_samples.insert(alias);
        }
    }

    let mut ordered = aggregates
        .into_iter()
        .map(|(key, aggregate)| {
            (
                key,
                aggregate.creator_name,
                aggregate.score.clamp(0.0, 0.99),
                aggregate.known_creator,
                aggregate.signals.into_iter().collect::<Vec<_>>(),
                aggregate.alias_samples.into_iter().collect::<Vec<_>>(),
            )
        })
        .collect::<Vec<_>>();

    ordered.sort_by(|left, right| right.2.total_cmp(&left.2));
    let (normalized_key, creator_name, score, known_creator, signals, alias_samples) =
        ordered.into_iter().next()?;

    Some(Selection {
        creator_name,
        normalized_key,
        score,
        known_creator,
        signals,
        alias_samples,
    })
}

fn collect_signals(
    record: &AuditRecord,
    settings: &LibrarySettings,
    seed_pack: &SeedPack,
) -> Vec<CandidateSignal> {
    let mut signals = Vec::new();
    let base_name = Path::new(&record.filename)
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or(&record.filename);

    for hint in &record.insights.creator_hints {
        if let Some(signal) = build_signal(hint, "Inspection metadata", 0.94, None, seed_pack) {
            signals.push(signal);
        }
    }

    for raw in [
        extract_bracket_candidate(base_name),
        extract_byline_candidate(base_name),
        extract_spaced_prefix_candidate(base_name),
        extract_underscore_prefix_candidate(base_name),
        extract_collab_prefix_candidate(base_name),
        extract_family_prefix_candidate(base_name),
    ]
    .into_iter()
    .flatten()
    {
        if let Some(signal) = build_signal(&raw, "Filename pattern", 0.82, Some(raw.clone()), seed_pack) {
            signals.push(signal);
        }
    }

    for (folder, weight) in collect_folder_candidates(&record.path, &record.source_location, settings) {
        if let Some(signal) = build_signal(&folder, "Folder path", weight, Some(folder.clone()), seed_pack) {
            signals.push(signal);
        }
    }

    signals
}

fn build_signal(
    raw_value: &str,
    signal: &str,
    base_score: f64,
    alias_sample: Option<String>,
    seed_pack: &SeedPack,
) -> Option<CandidateSignal> {
    let creator_name = resolve_candidate(raw_value, seed_pack)?;
    let normalized_key = normalize_key(&creator_name);
    if normalized_key.is_empty() {
        return None;
    }

    let known_creator = seed_pack.creator_profiles.contains_key(&creator_name)
        || seed_pack
            .creator_lookup
            .get(&normalize_key(raw_value))
            .is_some_and(|canonical| canonical == &creator_name);
    let alias_sample = alias_sample.filter(|value| normalize_key(value) != normalized_key);
    let score = if known_creator {
        base_score
    } else {
        (base_score - 0.08).max(0.45)
    };

    Some(CandidateSignal {
        creator_name,
        normalized_key,
        score,
        signal: signal.to_owned(),
        alias_sample,
        known_creator,
    })
}

fn resolve_candidate(value: &str, seed_pack: &SeedPack) -> Option<String> {
    let compact = collapse_whitespace(value);
    if compact.is_empty() || !compact.chars().any(|character| character.is_ascii_alphabetic()) {
        return None;
    }

    let normalized = normalize_key(&compact);
    if normalized.is_empty() {
        return None;
    }

    if let Some(canonical) = seed_pack.creator_lookup.get(&normalized) {
        return Some(canonical.clone());
    }

    if compact.len() > 40 || GENERIC_FOLDERS.contains(&normalized.as_str()) {
        return None;
    }

    Some(compact)
}

fn collapse_whitespace(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn extract_bracket_candidate(base_name: &str) -> Option<String> {
    let trimmed = base_name.trim();
    if !trimmed.starts_with(['[', '(', '{']) {
        return None;
    }

    let close = match trimmed.chars().next()? {
        '[' => ']',
        '(' => ')',
        '{' => '}',
        _ => return None,
    };
    let close_index = trimmed.find(close)?;
    let inside = trimmed[1..close_index].trim();
    if inside.is_empty() {
        None
    } else {
        Some(inside.to_owned())
    }
}

fn extract_byline_candidate(base_name: &str) -> Option<String> {
    let lower = base_name.to_ascii_lowercase();
    let index = lower.find(" by ")?;
    let candidate = base_name[(index + 4)..].trim();
    if candidate.is_empty() {
        None
    } else {
        Some(candidate.to_owned())
    }
}

fn extract_spaced_prefix_candidate(base_name: &str) -> Option<String> {
    let index = base_name.find(" - ")?;
    let candidate = base_name[..index].trim();
    if candidate.is_empty() {
        None
    } else {
        Some(candidate.to_owned())
    }
}

fn extract_underscore_prefix_candidate(base_name: &str) -> Option<String> {
    let index = base_name.find('_')?;
    let candidate = base_name[..index].trim();
    if candidate.is_empty() {
        None
    } else {
        Some(candidate.to_owned())
    }
}

fn extract_collab_prefix_candidate(base_name: &str) -> Option<String> {
    let lower = base_name.to_ascii_lowercase();
    let index = lower.find(" x ")?;
    let candidate = base_name[..index].trim();
    if candidate.is_empty() {
        None
    } else {
        Some(candidate.to_owned())
    }
}

fn extract_family_prefix_candidate(base_name: &str) -> Option<String> {
    if base_name.contains(['[', '(', '{', '_']) || base_name.contains(" - ") || base_name.contains(" x ") {
        return None;
    }

    let tokens = tokenize_compact_name(base_name);
    if tokens.len() < 2 {
        return None;
    }

    let compact = tokens
        .iter()
        .map(|token| token.to_ascii_lowercase())
        .collect::<String>();
    let matched_suffix = FAMILY_SUFFIX_TOKENS
        .iter()
        .filter(|suffix| compact.ends_with(**suffix))
        .max_by_key(|suffix| suffix.len())?;

    let mut suffix_length = 0usize;
    let mut split_index = tokens.len();
    for index in (0..tokens.len()).rev() {
        suffix_length += tokens[index].len();
        split_index = index;
        if suffix_length >= matched_suffix.len() {
            break;
        }
    }

    if suffix_length != matched_suffix.len() || split_index == 0 {
        return None;
    }

    let candidate = tokens[..split_index].join("");
    if candidate.is_empty() {
        None
    } else {
        Some(candidate)
    }
}

fn tokenize_compact_name(value: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let characters = value.chars().collect::<Vec<_>>();

    for index in 0..characters.len() {
        let character = characters[index];
        if !character.is_ascii_alphanumeric() {
            if !current.is_empty() {
                tokens.push(std::mem::take(&mut current));
            }
            continue;
        }

        if index > 0 {
            let previous = characters[index - 1];
            let split = (previous.is_ascii_lowercase() && character.is_ascii_uppercase())
                || (previous.is_ascii_uppercase()
                    && character.is_ascii_uppercase()
                    && characters
                        .get(index + 1)
                        .is_some_and(|next| next.is_ascii_lowercase()));
            if split && !current.is_empty() {
                tokens.push(std::mem::take(&mut current));
            }
        }

        current.push(character);
    }

    if !current.is_empty() {
        tokens.push(current);
    }

    tokens
}

fn collect_folder_candidates(
    path: &str,
    source_location: &str,
    settings: &LibrarySettings,
) -> Vec<(String, f64)> {
    let root = match source_location {
        "tray" => settings.tray_path.as_deref(),
        _ => settings.mods_path.as_deref(),
    };
    let Some(root) = root else {
        return Vec::new();
    };

    let file_path = Path::new(path);
    let relative = match file_path.strip_prefix(root) {
        Ok(relative) => relative,
        Err(_) => return Vec::new(),
    };

    let ancestors = relative
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_default()
        .components()
        .map(|component| component.as_os_str().to_string_lossy().to_string())
        .collect::<Vec<_>>();

    let mut results = Vec::new();
    for (index, folder) in ancestors.iter().rev().take(3).enumerate() {
        let normalized = normalize_key(folder);
        if normalized.is_empty() || GENERIC_FOLDERS.contains(&normalized.as_str()) {
            continue;
        }

        let weight = match index {
            0 => 0.72,
            1 => 0.64,
            _ => 0.58,
        };
        results.push((folder.clone(), weight));
    }

    results
}

fn is_root_loose(record: &AuditRecord, settings: &LibrarySettings) -> bool {
    let root = match record.source_location.as_str() {
        "tray" => settings.tray_path.as_deref(),
        _ => settings.mods_path.as_deref(),
    };
    let Some(root) = root else {
        return false;
    };

    PathBuf::from(&record.path)
        .strip_prefix(root)
        .ok()
        .and_then(|relative| relative.parent())
        .is_none_or(|parent| parent.as_os_str().is_empty())
}

#[cfg(test)]
mod tests {
    use rusqlite::params;
    use rusqlite::Connection;

    use crate::{
        database,
        models::{FileInsights, LibrarySettings},
        seed::load_seed_pack,
    };

    use super::load_creator_audit;

    #[test]
    fn creator_audit_groups_unknown_files_by_shared_signals() {
        let mut connection = Connection::open_in_memory().expect("db");
        database::initialize(&mut connection).expect("schema");
        let seed = load_seed_pack().expect("seed");
        database::seed_database(&mut connection, &seed).expect("seed db");

        connection.execute(
            "INSERT INTO files (path, filename, extension, kind, subtype, confidence, source_location, insights)
             VALUES (?1, ?2, '.package', 'BuildBuy', 'Furniture', 0.41, 'mods', ?3)",
            params![
                "C:/Mods/BabyBooBookshelf.package",
                "BabyBooBookshelf.package",
                serde_json::to_string(&FileInsights::default()).expect("insights")
            ],
        ).expect("file 1");
        connection.execute(
            "INSERT INTO files (path, filename, extension, kind, subtype, confidence, source_location, insights)
             VALUES (?1, ?2, '.package', 'BuildBuy', 'Furniture', 0.39, 'mods', ?3)",
            params![
                "C:/Mods/BabyBooChangingTable.package",
                "BabyBooChangingTable.package",
                serde_json::to_string(&FileInsights::default()).expect("insights")
            ],
        ).expect("file 2");

        let response = load_creator_audit(
            &connection,
            &LibrarySettings {
                mods_path: Some("C:/Mods".to_owned()),
                tray_path: None,
                downloads_path: None,
            },
            &seed,
            crate::models::CreatorAuditQuery::default(),
        )
        .expect("audit");

        assert_eq!(response.total_candidate_files, 2);
        assert_eq!(response.unresolved_files, 0);
        assert_eq!(response.groups.len(), 1);
        assert_eq!(response.groups[0].suggested_creator, "BabyBoo");
        assert_eq!(response.groups[0].item_count, 2);
    }
}
