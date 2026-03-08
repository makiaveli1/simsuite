use std::collections::{HashMap, HashSet};

use serde::Deserialize;

use crate::error::{AppError, AppResult};

const CREATORS_JSON: &str = include_str!("../../../seed/creators.json");
const KEYWORDS_JSON: &str = include_str!("../../../seed/keywords.json");
const HEURISTICS_JSON: &str = include_str!("../../../seed/heuristics.json");
const PRESETS_JSON: &str = include_str!("../../../seed/presets.json");
const TAXONOMY_JSON: &str = include_str!("../../../seed/taxonomy.json");
const DEFAULTS_JSON: &str = include_str!("../../../seed/defaults.json");

#[derive(Debug, Clone)]
pub struct SeedPack {
    pub seed_version: String,
    pub creators: Vec<SeedCreator>,
    pub creator_lookup: HashMap<String, String>,
    pub creator_profiles: HashMap<String, SeedCreator>,
    #[allow(dead_code)]
    pub keywords: KeywordSeed,
    pub keyword_lookup: KeywordLookup,
    pub heuristics: HeuristicSeed,
    pub version_token_set: HashSet<String>,
    pub support_token_set: HashSet<String>,
    pub parser_lexicon: HashSet<String>,
    pub presets: Vec<RulePreset>,
    #[allow(dead_code)]
    pub taxonomy: TaxonomySeed,
    pub defaults: DefaultsSeed,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SeedCreator {
    pub canonical_name: String,
    pub aliases: Vec<String>,
    pub likely_kinds: Vec<String>,
    pub likely_subtypes: Vec<String>,
    pub notes: String,
    #[serde(default)]
    pub locked_by_user: bool,
    #[serde(default)]
    pub created_by_user: bool,
    #[serde(default)]
    pub preferred_path: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct KeywordSeed {
    pub hair: Vec<String>,
    pub clothing: Vec<String>,
    pub makeup: Vec<String>,
    pub skin: Vec<String>,
    pub build_buy: Vec<String>,
    pub gameplay: Vec<String>,
    pub override_keywords: Vec<String>,
    pub pose: Vec<String>,
    pub preset: Vec<String>,
    pub eyes: Vec<String>,
    pub eyebrows: Vec<String>,
    pub eyelashes: Vec<String>,
    pub facial_hair: Vec<String>,
    pub accessories: Vec<String>,
    pub tattoos: Vec<String>,
    pub utilities: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct KeywordLookup {
    pub hair: HashSet<String>,
    pub clothing: HashSet<String>,
    pub makeup: HashSet<String>,
    pub skin: HashSet<String>,
    pub build_buy: HashSet<String>,
    pub gameplay: HashSet<String>,
    pub override_keywords: HashSet<String>,
    pub pose: HashSet<String>,
    pub preset: HashSet<String>,
    pub eyes: HashSet<String>,
    pub eyebrows: HashSet<String>,
    pub eyelashes: HashSet<String>,
    pub facial_hair: HashSet<String>,
    pub accessories: HashSet<String>,
    pub tattoos: HashSet<String>,
    pub utilities: HashSet<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct HeuristicSeed {
    #[allow(dead_code)]
    pub seed_version: String,
    #[allow(dead_code)]
    pub mod_extensions: Vec<String>,
    pub tray_extensions: Vec<String>,
    pub version_tokens: Vec<String>,
    pub support_tokens: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RulePreset {
    pub name: String,
    pub template: String,
    pub priority: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TaxonomySeed {
    #[allow(dead_code)]
    pub seed_version: String,
    #[allow(dead_code)]
    pub kinds: Vec<String>,
    #[allow(dead_code)]
    pub tray_kinds: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DefaultsSeed {
    #[allow(dead_code)]
    pub seed_version: String,
    pub settings: HashMap<String, String>,
}

pub fn load_seed_pack() -> AppResult<SeedPack> {
    let creators_file: Versioned<Vec<SeedCreator>> = serde_json::from_str(CREATORS_JSON)?;
    let keywords_file: Versioned<KeywordSeed> = serde_json::from_str(KEYWORDS_JSON)?;
    let heuristics: HeuristicSeed = serde_json::from_str(HEURISTICS_JSON)?;
    let presets_file: Versioned<Vec<RulePreset>> = serde_json::from_str(PRESETS_JSON)?;
    let taxonomy: TaxonomySeed = serde_json::from_str(TAXONOMY_JSON)?;
    let defaults: DefaultsSeed = serde_json::from_str(DEFAULTS_JSON)?;

    validate_creators(&creators_file.items)?;

    let mut creator_lookup = HashMap::new();
    let mut creator_profiles = HashMap::new();
    for creator in &creators_file.items {
        creator_profiles.insert(creator.canonical_name.clone(), creator.clone());
        creator_lookup.insert(
            normalize_key(&creator.canonical_name),
            creator.canonical_name.clone(),
        );
        for alias in &creator.aliases {
            creator_lookup.insert(normalize_key(alias), creator.canonical_name.clone());
        }
    }

    let keyword_lookup = build_keyword_lookup(&keywords_file.items);
    let version_token_set = normalize_token_set(&heuristics.version_tokens);
    let support_token_set = normalize_token_set(&heuristics.support_tokens);
    let parser_lexicon = build_parser_lexicon(
        &creators_file.items,
        &keyword_lookup,
        &version_token_set,
        &support_token_set,
    );

    Ok(SeedPack {
        seed_version: creators_file.seed_version,
        creators: creators_file.items,
        creator_lookup,
        creator_profiles,
        keywords: keywords_file.items,
        keyword_lookup,
        heuristics,
        version_token_set,
        support_token_set,
        parser_lexicon,
        presets: presets_file.items,
        taxonomy,
        defaults,
    })
}

pub fn normalize_key(value: &str) -> String {
    value
        .to_lowercase()
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .collect()
}

fn validate_creators(creators: &[SeedCreator]) -> AppResult<()> {
    let mut names = HashSet::new();
    for creator in creators {
        if !names.insert(normalize_key(&creator.canonical_name)) {
            return Err(AppError::Message(format!(
                "Duplicate canonical creator in seed data: {}",
                creator.canonical_name
            )));
        }
    }

    Ok(())
}

fn build_keyword_lookup(keywords: &KeywordSeed) -> KeywordLookup {
    KeywordLookup {
        hair: normalize_token_set(&keywords.hair),
        clothing: normalize_token_set(&keywords.clothing),
        makeup: normalize_token_set(&keywords.makeup),
        skin: normalize_token_set(&keywords.skin),
        build_buy: normalize_token_set(&keywords.build_buy),
        gameplay: normalize_token_set(&keywords.gameplay),
        override_keywords: normalize_token_set(&keywords.override_keywords),
        pose: normalize_token_set(&keywords.pose),
        preset: normalize_token_set(&keywords.preset),
        eyes: normalize_token_set(&keywords.eyes),
        eyebrows: normalize_token_set(&keywords.eyebrows),
        eyelashes: normalize_token_set(&keywords.eyelashes),
        facial_hair: normalize_token_set(&keywords.facial_hair),
        accessories: normalize_token_set(&keywords.accessories),
        tattoos: normalize_token_set(&keywords.tattoos),
        utilities: normalize_token_set(&keywords.utilities),
    }
}

fn normalize_token_set(values: &[String]) -> HashSet<String> {
    values
        .iter()
        .map(|value| normalize_key(value))
        .filter(|value| !value.is_empty())
        .collect()
}

fn build_parser_lexicon(
    creators: &[SeedCreator],
    keyword_lookup: &KeywordLookup,
    version_token_set: &HashSet<String>,
    support_token_set: &HashSet<String>,
) -> HashSet<String> {
    let mut lexicon = HashSet::new();

    for creator in creators {
        lexicon.insert(normalize_key(&creator.canonical_name));
        for alias in &creator.aliases {
            lexicon.insert(normalize_key(alias));
        }
    }

    for token in [
        &keyword_lookup.hair,
        &keyword_lookup.clothing,
        &keyword_lookup.makeup,
        &keyword_lookup.skin,
        &keyword_lookup.build_buy,
        &keyword_lookup.gameplay,
        &keyword_lookup.override_keywords,
        &keyword_lookup.pose,
        &keyword_lookup.preset,
        &keyword_lookup.eyes,
        &keyword_lookup.eyebrows,
        &keyword_lookup.eyelashes,
        &keyword_lookup.facial_hair,
        &keyword_lookup.accessories,
        &keyword_lookup.tattoos,
        &keyword_lookup.utilities,
        version_token_set,
        support_token_set,
    ] {
        lexicon.extend(token.iter().cloned());
    }

    lexicon
}

#[derive(Debug, Clone, Deserialize)]
struct Versioned<T> {
    seed_version: String,
    items: T,
}
