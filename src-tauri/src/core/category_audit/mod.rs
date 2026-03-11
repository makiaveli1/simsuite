use std::{
    collections::{BTreeSet, HashMap},
    path::Path,
};

use rusqlite::Connection;

use crate::{
    core::filename_parser::parse_filename,
    error::AppResult,
    models::{
        CategoryAuditFile, CategoryAuditGroup, CategoryAuditQuery, CategoryAuditResponse,
        FileInsights, LibrarySettings,
    },
    seed::{normalize_key, SeedPack},
};

const MAX_SAMPLE_FILES: usize = 8;
const MAX_UNRESOLVED_SAMPLES: usize = 12;
const MAX_KEYWORD_SAMPLES: usize = 6;
const MIN_GROUP_SCORE: f64 = 0.56;
const HIGH_CONFIDENCE_GROUP: f64 = 0.84;
const GENERIC_FOLDERS: &[&str] = &[
    "mods",
    "tray",
    "downloads",
    "review",
    "unknown",
    "misc",
    "unsorted",
    "creators",
];

#[derive(Debug, Clone)]
struct AuditRecord {
    id: i64,
    filename: String,
    path: String,
    current_kind: String,
    current_subtype: Option<String>,
    confidence: f64,
    source_location: String,
    parser_warnings: Vec<String>,
    insights: FileInsights,
}

#[derive(Debug, Clone)]
struct CategorySignal {
    kind: String,
    subtype: Option<String>,
    score: f64,
    signal: String,
    keyword_sample: Option<String>,
}

#[derive(Debug, Default)]
struct CategoryAggregate {
    kind: String,
    subtype: Option<String>,
    score: f64,
    signals: BTreeSet<String>,
    keyword_samples: BTreeSet<String>,
}

#[derive(Debug)]
struct Selection {
    id: String,
    kind: String,
    subtype: Option<String>,
    score: f64,
    signals: Vec<String>,
    keyword_samples: Vec<String>,
}

#[derive(Debug, Default)]
struct GroupAccumulator {
    suggested_kind: String,
    suggested_subtype: Option<String>,
    confidence_sum: f64,
    file_ids: Vec<i64>,
    source_signals: BTreeSet<String>,
    keyword_samples: BTreeSet<String>,
    sample_files: Vec<CategoryAuditFile>,
}

pub fn load_category_audit(
    connection: &Connection,
    settings: &LibrarySettings,
    seed_pack: &SeedPack,
    query: CategoryAuditQuery,
) -> AppResult<CategoryAuditResponse> {
    let records = load_candidate_records(connection)?;
    let total_candidate_files = records.len() as i64;
    let unknown_files = records
        .iter()
        .filter(|record| record.current_kind == "Unknown")
        .count() as i64;
    let mut groups = HashMap::<String, GroupAccumulator>::new();
    let mut unresolved_samples = Vec::new();
    let mut grouped_files = 0_i64;
    let mut unresolved_files = 0_i64;

    for record in records {
        let selection = select_candidate(&record, settings, seed_pack);
        let Some(selection) = selection.filter(|selected| selected.score >= MIN_GROUP_SCORE) else {
            unresolved_files += 1;
            if unresolved_samples.len() < MAX_UNRESOLVED_SAMPLES {
                unresolved_samples.push(CategoryAuditFile {
                    id: record.id,
                    filename: record.filename,
                    path: record.path,
                    current_kind: record.current_kind,
                    current_subtype: record.current_subtype,
                    confidence: record.confidence,
                    source_location: record.source_location,
                    keyword_samples: Vec::new(),
                    match_reasons: vec!["No strong category cluster signal yet".to_owned()],
                });
            }
            continue;
        };

        grouped_files += 1;
        let entry = groups
            .entry(selection.id.clone())
            .or_insert_with(|| GroupAccumulator {
                suggested_kind: selection.kind.clone(),
                suggested_subtype: selection.subtype.clone(),
                ..GroupAccumulator::default()
            });

        entry.suggested_kind = selection.kind.clone();
        entry.suggested_subtype = selection.subtype.clone();
        entry.confidence_sum += selection.score;
        entry.file_ids.push(record.id);
        for signal in &selection.signals {
            entry.source_signals.insert(signal.clone());
        }
        for keyword in &selection.keyword_samples {
            if entry.keyword_samples.len() < MAX_KEYWORD_SAMPLES {
                entry.keyword_samples.insert(keyword.clone());
            }
        }
        if entry.sample_files.len() < MAX_SAMPLE_FILES {
            entry.sample_files.push(CategoryAuditFile {
                id: record.id,
                filename: record.filename,
                path: record.path,
                current_kind: record.current_kind,
                current_subtype: record.current_subtype,
                confidence: record.confidence,
                source_location: record.source_location,
                keyword_samples: selection.keyword_samples.clone(),
                match_reasons: selection.signals.clone(),
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

            let audit_group = CategoryAuditGroup {
                id,
                suggested_kind: group.suggested_kind,
                suggested_subtype: group.suggested_subtype,
                confidence: (group.confidence_sum / (group.file_ids.len() as f64)).clamp(0.0, 0.99),
                item_count,
                source_signals: group.source_signals.into_iter().collect(),
                keyword_samples: group
                    .keyword_samples
                    .into_iter()
                    .take(MAX_KEYWORD_SAMPLES)
                    .collect(),
                file_ids: group.file_ids,
                sample_files: group.sample_files,
            };

            if let Some(search) = &search {
                let haystack = format!(
                    "{} {} {} {}",
                    audit_group.suggested_kind,
                    audit_group.suggested_subtype.as_deref().unwrap_or_default(),
                    audit_group.keyword_samples.join(" "),
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
            .then_with(|| left.suggested_kind.cmp(&right.suggested_kind))
            .then_with(|| left.suggested_subtype.cmp(&right.suggested_subtype))
    });

    let total_groups = group_items.len() as i64;
    let high_confidence_groups = group_items
        .iter()
        .filter(|group| group.confidence >= HIGH_CONFIDENCE_GROUP)
        .count() as i64;
    group_items.truncate(limit);

    Ok(CategoryAuditResponse {
        total_candidate_files,
        grouped_files,
        unresolved_files,
        unknown_files,
        total_groups,
        high_confidence_groups,
        groups: group_items,
        unresolved_samples,
    })
}

pub fn load_category_group_files(
    connection: &Connection,
    settings: &LibrarySettings,
    seed_pack: &SeedPack,
    group_id: &str,
) -> AppResult<Vec<CategoryAuditFile>> {
    let records = load_candidate_records(connection)?;
    let mut matching_records = Vec::new();
    let mut keyword_samples = BTreeSet::new();
    let mut source_signals = BTreeSet::new();

    for record in records {
        let Some(selection) = select_candidate(&record, settings, seed_pack)
            .filter(|selected| selected.score >= MIN_GROUP_SCORE && selected.id == group_id)
        else {
            continue;
        };

        for signal in &selection.signals {
            source_signals.insert(signal.clone());
        }
        for keyword in &selection.keyword_samples {
            if keyword_samples.len() < MAX_KEYWORD_SAMPLES {
                keyword_samples.insert(keyword.clone());
            }
        }

        matching_records.push((record, selection.score));
    }

    let shared_keywords = keyword_samples
        .into_iter()
        .take(MAX_KEYWORD_SAMPLES)
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
        .map(|(record, _)| CategoryAuditFile {
            id: record.id,
            filename: record.filename,
            path: record.path,
            current_kind: record.current_kind,
            current_subtype: record.current_subtype,
            confidence: record.confidence,
            source_location: record.source_location,
            keyword_samples: shared_keywords.clone(),
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
            f.parser_warnings,
            f.insights
         FROM files f
         LEFT JOIN user_category_overrides o ON o.match_path = f.path
         WHERE f.extension IN ('.package', '.ts4script')
           AND f.source_location <> 'tray'
           AND o.id IS NULL
           AND (
                f.kind = 'Unknown'
                OR f.confidence < 0.78
                OR f.parser_warnings LIKE '%no_category_detected%'
                OR f.parser_warnings LIKE '%conflicting_category_signals%'
           )
         ORDER BY f.filename COLLATE NOCASE",
    )?;

    let rows = statement
        .query_map([], |row| {
            Ok(AuditRecord {
                id: row.get(0)?,
                filename: row.get(1)?,
                path: row.get(2)?,
                current_kind: row.get(3)?,
                current_subtype: row.get(4)?,
                confidence: row.get(5)?,
                source_location: row.get(6)?,
                parser_warnings: serde_json::from_str::<Vec<String>>(&row.get::<_, String>(7)?)
                    .unwrap_or_default(),
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
    let mut aggregates = HashMap::<String, CategoryAggregate>::new();

    for signal in collect_signals(record, settings, seed_pack) {
        let key = category_group_id(&signal.kind, signal.subtype.as_deref());
        let entry = aggregates.entry(key).or_insert_with(|| CategoryAggregate {
            kind: signal.kind.clone(),
            subtype: signal.subtype.clone(),
            ..CategoryAggregate::default()
        });
        entry.kind = signal.kind.clone();
        entry.subtype = signal.subtype.clone();
        entry.score += signal.score;
        entry.signals.insert(signal.signal);
        if let Some(keyword) = signal.keyword_sample {
            entry.keyword_samples.insert(keyword);
        }
    }

    let mut ordered = aggregates
        .into_iter()
        .map(|(id, aggregate)| {
            (
                id,
                aggregate.kind,
                aggregate.subtype,
                aggregate.score.clamp(0.0, 0.99),
                aggregate.signals.into_iter().collect::<Vec<_>>(),
                aggregate.keyword_samples.into_iter().collect::<Vec<_>>(),
            )
        })
        .collect::<Vec<_>>();

    ordered.sort_by(|left, right| right.3.total_cmp(&left.3));
    let (id, kind, subtype, score, signals, keyword_samples) = ordered.into_iter().next()?;

    Some(Selection {
        id,
        kind,
        subtype,
        score,
        signals,
        keyword_samples,
    })
}

fn collect_signals(
    record: &AuditRecord,
    settings: &LibrarySettings,
    seed_pack: &SeedPack,
) -> Vec<CategorySignal> {
    let mut signals = Vec::new();

    if record.current_kind != "Unknown" && !record.current_kind.starts_with("Tray") {
        let mut score = (0.44 + (record.confidence * 0.34)).clamp(0.44, 0.8);
        if record
            .parser_warnings
            .iter()
            .any(|warning| warning == "conflicting_category_signals")
        {
            score = (score - 0.08).max(0.4);
        }

        push_signal(
            &mut signals,
            &record.current_kind,
            record.current_subtype.clone(),
            score,
            "Current parser",
            record
                .current_subtype
                .clone()
                .or_else(|| Some(record.current_kind.clone())),
        );
    }

    let parsed = parse_filename(&record.filename, seed_pack);
    if parsed.kind != "Unknown" && !parsed.kind.starts_with("Tray") {
        push_signal(
            &mut signals,
            &parsed.kind,
            parsed.subtype.clone(),
            parsed.confidence.max(0.62),
            "Filename keywords",
            parsed
                .subtype
                .clone()
                .or_else(|| parsed.support_tokens.first().cloned())
                .or(parsed.set_name.clone()),
        );
    }

    let extension = Path::new(&record.filename)
        .extension()
        .map(|value| format!(".{}", value.to_string_lossy().to_lowercase()))
        .unwrap_or_else(|| ".package".to_owned());
    for name in record.insights.embedded_names.iter().take(4) {
        let parsed = parse_filename(&format!("{name}{extension}"), seed_pack);
        if parsed.kind == "Unknown" || parsed.kind.starts_with("Tray") {
            continue;
        }

        push_signal(
            &mut signals,
            &parsed.kind,
            parsed.subtype.clone(),
            0.68,
            "Embedded name",
            Some(name.clone()),
        );
    }

    if let Some((kind, subtype, keyword)) = infer_category_from_insights(record) {
        push_signal(
            &mut signals,
            &kind,
            subtype,
            0.78,
            "Inspection metadata",
            keyword,
        );
    }

    for (folder, weight) in
        collect_folder_candidates(&record.path, &record.source_location, settings)
    {
        if let Some((kind, subtype)) = infer_category_from_folder(&folder) {
            push_signal(
                &mut signals,
                kind,
                subtype,
                weight,
                "Folder path",
                Some(folder),
            );
        }
    }

    signals
}

fn push_signal(
    signals: &mut Vec<CategorySignal>,
    kind: &str,
    subtype: Option<String>,
    score: f64,
    signal: &str,
    keyword_sample: Option<String>,
) {
    if kind.is_empty() || kind == "Unknown" || kind.starts_with("Tray") {
        return;
    }

    let normalized_subtype = sanitize_subtype(subtype);
    let cleaned_keyword = keyword_sample.and_then(clean_keyword_sample);
    signals.push(CategorySignal {
        kind: kind.to_owned(),
        subtype: normalized_subtype.clone(),
        score: score.clamp(0.0, 0.99),
        signal: signal.to_owned(),
        keyword_sample: cleaned_keyword.clone(),
    });

    if normalized_subtype.is_some() {
        signals.push(CategorySignal {
            kind: kind.to_owned(),
            subtype: None,
            score: (score * 0.62).clamp(0.0, 0.99),
            signal: signal.to_owned(),
            keyword_sample: cleaned_keyword,
        });
    }
}

fn sanitize_subtype(value: Option<String>) -> Option<String> {
    value.and_then(|subtype| {
        let trimmed = subtype.trim();
        if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("unknown") {
            None
        } else {
            Some(trimmed.to_owned())
        }
    })
}

fn clean_keyword_sample(value: String) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_owned())
    }
}

fn infer_category_from_insights(
    record: &AuditRecord,
) -> Option<(String, Option<String>, Option<String>)> {
    let format = record.insights.format.as_deref().unwrap_or_default();
    let summary = record
        .insights
        .resource_summary
        .iter()
        .map(|value| value.to_ascii_lowercase())
        .collect::<Vec<_>>();

    if format == "ts4script-zip" || !record.insights.script_namespaces.is_empty() {
        return Some((
            "ScriptMods".to_owned(),
            Some("Utilities".to_owned()),
            Some("ts4script".to_owned()),
        ));
    }

    if summary.iter().any(|value| value.contains("hotspotcontrol")) {
        return Some((
            "PresetsAndSliders".to_owned(),
            Some("Sliders".to_owned()),
            Some("HotSpotControl".to_owned()),
        ));
    }

    if summary
        .iter()
        .any(|value| value.contains("caspart") || value.contains("skintone"))
    {
        let subtype = if summary.iter().any(|value| value.contains("skintone")) {
            Some("Skin".to_owned())
        } else {
            record.current_subtype.clone()
        };
        return Some((
            "CAS".to_owned(),
            sanitize_subtype(subtype),
            Some("CASPart".to_owned()),
        ));
    }

    if summary
        .iter()
        .any(|value| value.contains("catalog") || value.contains("definition"))
    {
        return Some((
            "BuildBuy".to_owned(),
            sanitize_subtype(record.current_subtype.clone()),
            Some("Catalog".to_owned()),
        ));
    }

    if summary.iter().any(|value| value.contains("scriptresource")) {
        return Some((
            "ScriptMods".to_owned(),
            Some("Utilities".to_owned()),
            Some("ScriptResource".to_owned()),
        ));
    }

    None
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
    for (index, folder) in ancestors.iter().rev().take(4).enumerate() {
        let normalized = normalize_key(folder);
        if normalized.is_empty() || GENERIC_FOLDERS.contains(&normalized.as_str()) {
            continue;
        }

        let weight = match index {
            0 => 0.76,
            1 => 0.68,
            2 => 0.6,
            _ => 0.56,
        };
        results.push((folder.clone(), weight));
    }

    results
}

fn infer_category_from_folder(folder: &str) -> Option<(&'static str, Option<String>)> {
    let normalized = normalize_key(folder);
    if normalized.is_empty() {
        return None;
    }

    let result = match normalized.as_str() {
        "cas" => ("CAS", None),
        "hair" | "hairstyles" | "bangs" | "ponytail" | "braid" => ("CAS", Some("Hair".to_owned())),
        "facialhair" | "beard" | "beards" | "mustache" | "mustaches" => {
            ("CAS", Some("Facial Hair".to_owned()))
        }
        "tops" | "top" | "shirt" | "shirts" | "jacket" | "jackets" | "hoodie" | "hoodies"
        | "sweater" | "sweaters" | "blouse" | "blouses" | "cardigan" | "cardigans" | "bodysuit"
        | "bodysuits" => ("CAS", Some("Tops".to_owned())),
        "bottoms" | "bottom" | "jeans" | "pants" | "shorts" | "skirt" | "skirts" => {
            ("CAS", Some("Bottoms".to_owned()))
        }
        "dress" | "dresses" | "fullbody" => ("CAS", Some("Dresses".to_owned())),
        "makeup" | "eyeliner" | "eyeshadow" | "blush" | "lipstick" | "lips" => {
            ("CAS", Some("Makeup".to_owned()))
        }
        "skin" | "skins" | "skinblend" | "skinblends" | "skintone" | "skintones" => {
            ("CAS", Some("Skin".to_owned()))
        }
        "tattoo" | "tattoos" => ("CAS", Some("Tattoos".to_owned())),
        "accessory" | "accessories" | "jewelry" | "hats" | "glasses" | "earrings" | "necklaces"
        | "rings" => ("CAS", Some("Accessories".to_owned())),
        "preset" | "presets" => ("PresetsAndSliders", Some("Presets".to_owned())),
        "slider" | "sliders" => ("PresetsAndSliders", Some("Sliders".to_owned())),
        "buildbuy" | "build" | "buy" => ("BuildBuy", None),
        "furniture" | "kitchen" | "bathroom" | "bedroom" | "living" | "dining" | "nursery" => {
            ("BuildBuy", Some("Furniture".to_owned()))
        }
        "decor" | "clutter" | "plant" | "plants" | "lighting" => {
            ("BuildBuy", Some("Decor".to_owned()))
        }
        "walls" | "wall" | "floors" | "floor" | "windows" | "window" | "doors" | "door" => {
            ("BuildBuy", Some("Build Surfaces".to_owned()))
        }
        "gameplay" => ("Gameplay", None),
        "trait" | "traits" => ("Gameplay", Some("Traits".to_owned())),
        "career" | "careers" => ("Gameplay", Some("Careers".to_owned())),
        "aspiration" | "aspirations" => ("Gameplay", Some("Aspirations".to_owned())),
        "relationship" | "relationships" | "romance" | "family" => {
            ("Gameplay", Some("Relationship Systems".to_owned()))
        }
        "pregnancy" | "childbirth" => ("Gameplay", Some("Pregnancy".to_owned())),
        "scriptmods" | "scripts" | "script" => ("ScriptMods", Some("Utilities".to_owned())),
        "core" => ("ScriptMods", Some("Core".to_owned())),
        "ui" => ("ScriptMods", Some("Utilities".to_owned())),
        "override" | "overrides" | "default" | "defaults" | "replacement" | "replacements" => {
            ("OverridesAndDefaults", Some("Defaults".to_owned()))
        }
        "lightingoverride" | "terrainoverride" | "walkstyleoverride" | "wateroverride"
        | "skyoverride" => ("OverridesAndDefaults", Some("Overrides".to_owned())),
        "pose" | "poses" | "animation" | "animations" => {
            ("PosesAndAnimation", Some("Poses".to_owned()))
        }
        _ => return None,
    };

    Some(result)
}

fn category_group_id(kind: &str, subtype: Option<&str>) -> String {
    let kind_key = normalize_key(kind);
    let subtype_key = subtype.map(normalize_key).unwrap_or_default();
    if subtype_key.is_empty() {
        kind_key
    } else {
        format!("{kind_key}:{subtype_key}")
    }
}

#[cfg(test)]
mod tests {
    use rusqlite::{params, Connection};

    use crate::{
        database,
        models::{FileInsights, LibrarySettings},
        seed::load_seed_pack,
    };

    use super::load_category_audit;

    #[test]
    fn category_audit_groups_unknown_files_by_shared_category_signals() {
        let mut connection = Connection::open_in_memory().expect("db");
        database::initialize(&mut connection).expect("schema");
        let seed = load_seed_pack().expect("seed");
        database::seed_database(&mut connection, &seed).expect("seed db");

        connection
            .execute(
                "INSERT INTO files (
                    path, filename, extension, kind, subtype, confidence, source_location, parser_warnings, insights
                 ) VALUES (?1, ?2, '.package', 'Unknown', NULL, 0.34, 'mods', ?3, ?4)",
                params![
                    "C:/Mods/Hair/alpha_hair.package",
                    "alpha_hair.package",
                    serde_json::to_string(&vec!["no_category_detected"]).expect("warnings"),
                    serde_json::to_string(&FileInsights::default()).expect("insights")
                ],
            )
            .expect("file 1");
        connection
            .execute(
                "INSERT INTO files (
                    path, filename, extension, kind, subtype, confidence, source_location, parser_warnings, insights
                 ) VALUES (?1, ?2, '.package', 'Unknown', NULL, 0.31, 'mods', ?3, ?4)",
                params![
                    "C:/Mods/Hair/beta_hair.package",
                    "beta_hair.package",
                    serde_json::to_string(&vec!["no_category_detected"]).expect("warnings"),
                    serde_json::to_string(&FileInsights::default()).expect("insights")
                ],
            )
            .expect("file 2");

        let response = load_category_audit(
            &connection,
            &LibrarySettings {
                mods_path: Some("C:/Mods".to_owned()),
                tray_path: None,
                downloads_path: None,
            },
            &seed,
            crate::models::CategoryAuditQuery::default(),
        )
        .expect("audit");

        assert_eq!(response.total_candidate_files, 2);
        assert_eq!(response.unresolved_files, 0);
        assert_eq!(response.groups.len(), 1);
        assert_eq!(response.groups[0].suggested_kind, "CAS");
        assert_eq!(
            response.groups[0].suggested_subtype.as_deref(),
            Some("Hair")
        );
    }
}
