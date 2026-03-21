use std::collections::{HashMap, HashSet};

use serde::Deserialize;

use crate::error::{AppError, AppResult};

const CREATORS_JSON: &str = include_str!("../../../seed/creators.json");
const KEYWORDS_JSON: &str = include_str!("../../../seed/keywords.json");
const HEURISTICS_JSON: &str = include_str!("../../../seed/heuristics.json");
const PRESETS_JSON: &str = include_str!("../../../seed/presets.json");
const TAXONOMY_JSON: &str = include_str!("../../../seed/taxonomy.json");
const DEFAULTS_JSON: &str = include_str!("../../../seed/defaults.json");
const INSTALL_PROFILES_JSON: &str = include_str!("../../../seed/install_profiles.json");

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
    pub install_catalog: InstallCatalogSeed,
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

#[derive(Debug, Clone, Deserialize)]
pub struct InstallCatalogSeed {
    #[allow(dead_code)]
    pub seed_version: String,
    #[serde(default)]
    pub guided_profiles: Vec<GuidedInstallProfileSeed>,
    #[serde(default)]
    pub dependency_rules: Vec<DependencyRuleSeed>,
    #[serde(default)]
    pub incompatibility_rules: Vec<IncompatibilityRuleSeed>,
    #[serde(default)]
    pub review_only_patterns: Vec<ReviewOnlyPatternSeed>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct VersionSignalPathPatternSeed {
    pub path_patterns: Vec<String>,
    pub pattern: String,
}

impl Default for VersionSignalPathPatternSeed {
    fn default() -> Self {
        Self {
            path_patterns: Vec::new(),
            pattern: String::new(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct VersionRewriteSeed {
    pub pattern: String,
    pub replace: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct VersionStrategySeed {
    pub incoming_order: Vec<String>,
    pub installed_order: Vec<String>,
    pub filename_patterns: Vec<String>,
    pub payload_patterns: Vec<VersionSignalPathPatternSeed>,
    pub ignored_patterns: Vec<String>,
    pub rewrites: Vec<VersionRewriteSeed>,
    pub same_version_signature_policy: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GuidedInstallProfileSeed {
    pub key: String,
    pub display_name: String,
    pub creator: Option<String>,
    pub family: String,
    pub official_source_url: String,
    #[serde(default)]
    pub official_download_url: Option<String>,
    #[serde(default)]
    pub latest_check_strategy: Option<String>,
    #[serde(default)]
    pub latest_check_url: Option<String>,
    #[serde(default)]
    pub reference_source: Vec<String>,
    pub reviewed_at: String,
    #[serde(default)]
    pub sample_filenames: Vec<String>,
    #[serde(default)]
    pub version_file_hints: Vec<String>,
    #[serde(rename = "versionStrategy", default)]
    pub version_strategy: Option<VersionStrategySeed>,
    pub help_summary: String,
    #[serde(default)]
    pub post_install_notes: Vec<String>,
    pub required_name_clues: Vec<String>,
    pub script_prefixes: Vec<String>,
    pub package_prefixes: Vec<String>,
    pub name_clues: Vec<String>,
    pub text_clues: Vec<String>,
    #[serde(default)]
    pub archive_path_clues: Vec<String>,
    pub install_folder_name: String,
    #[serde(default = "default_true")]
    pub allow_root_install: bool,
    #[serde(default = "default_max_install_depth")]
    pub max_install_depth: usize,
    #[serde(default = "default_minimum_profile_files")]
    pub minimum_profile_files: usize,
    #[serde(default)]
    pub minimum_script_files: usize,
    #[serde(default)]
    pub required_all_filenames: Vec<String>,
    pub preserve_extensions: Vec<String>,
    pub preserve_prefixes: Vec<String>,
    #[serde(default)]
    pub dependency_keys: Vec<String>,
    #[serde(default)]
    pub incompatibility_keys: Vec<String>,
    #[serde(default)]
    pub review_reasons: Vec<String>,
    #[serde(default)]
    pub block_reasons: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct DependencyRuleSeed {
    pub key: String,
    pub display_name: String,
    pub creator: Option<String>,
    pub family: String,
    pub official_source_url: String,
    #[serde(default)]
    pub official_download_url: Option<String>,
    #[serde(default)]
    pub reference_source: Vec<String>,
    pub reviewed_at: String,
    #[serde(default)]
    pub sample_filenames: Vec<String>,
    pub dependency_key: String,
    pub help_summary: String,
    #[serde(default)]
    pub name_clues: Vec<String>,
    #[serde(default)]
    pub text_clues: Vec<String>,
    #[serde(default)]
    pub archive_path_clues: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct IncompatibilityRuleSeed {
    pub key: String,
    pub display_name: String,
    pub official_source_url: String,
    #[serde(default)]
    pub reference_source: Vec<String>,
    pub reviewed_at: String,
    #[serde(default)]
    pub sample_filenames: Vec<String>,
    pub installed_profile_key: String,
    #[serde(default)]
    pub name_clues: Vec<String>,
    #[serde(default)]
    pub text_clues: Vec<String>,
    #[serde(default)]
    pub archive_path_clues: Vec<String>,
    pub warning_message: String,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct ReviewOnlyPatternSeed {
    pub key: String,
    pub display_name: String,
    pub official_source_url: Option<String>,
    #[serde(default)]
    pub reference_source: Vec<String>,
    pub reviewed_at: String,
    #[serde(default)]
    pub sample_filenames: Vec<String>,
    pub help_summary: String,
    #[serde(default)]
    pub name_clues: Vec<String>,
    #[serde(default)]
    pub text_clues: Vec<String>,
    #[serde(default)]
    pub archive_path_clues: Vec<String>,
    pub review_reason: String,
}

pub fn load_seed_pack() -> AppResult<SeedPack> {
    let creators_file: Versioned<Vec<SeedCreator>> = serde_json::from_str(CREATORS_JSON)?;
    let keywords_file: Versioned<KeywordSeed> = serde_json::from_str(KEYWORDS_JSON)?;
    let heuristics: HeuristicSeed = serde_json::from_str(HEURISTICS_JSON)?;
    let presets_file: Versioned<Vec<RulePreset>> = serde_json::from_str(PRESETS_JSON)?;
    let taxonomy: TaxonomySeed = serde_json::from_str(TAXONOMY_JSON)?;
    let defaults: DefaultsSeed = serde_json::from_str(DEFAULTS_JSON)?;
    let install_catalog: InstallCatalogSeed = serde_json::from_str(INSTALL_PROFILES_JSON)?;

    validate_creators(&creators_file.items)?;
    validate_install_catalog(&install_catalog)?;

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
        install_catalog,
    })
}

fn default_true() -> bool {
    true
}

fn default_max_install_depth() -> usize {
    1
}

fn default_minimum_profile_files() -> usize {
    1
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

fn validate_install_catalog(catalog: &InstallCatalogSeed) -> AppResult<()> {
    let mut keys = HashSet::new();

    for profile in &catalog.guided_profiles {
        if !keys.insert(normalize_key(&profile.key)) {
            return Err(AppError::Message(format!(
                "Duplicate guided special-mod profile key: {}",
                profile.key
            )));
        }

        if profile.required_name_clues.is_empty() {
            return Err(AppError::Message(format!(
                "Guided special-mod profile {} is missing required core clues.",
                profile.key
            )));
        }

        if profile.minimum_profile_files == 0 {
            return Err(AppError::Message(format!(
                "Guided special-mod profile {} must require at least one matching file.",
                profile.key
            )));
        }

        if profile.max_install_depth > 1 {
            return Err(AppError::Message(format!(
                "Guided special-mod profile {} allows an unsafe install depth. Only root or one folder deep is supported right now.",
                profile.key
            )));
        }

        if profile.minimum_script_files > profile.minimum_profile_files {
            return Err(AppError::Message(format!(
                "Guided special-mod profile {} cannot require more script files than total matching files.",
                profile.key
            )));
        }

        if profile.script_prefixes.is_empty() && profile.package_prefixes.is_empty() {
            return Err(AppError::Message(format!(
                "Guided special-mod profile {} needs at least one script or package prefix.",
                profile.key
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

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_guided_profile() -> GuidedInstallProfileSeed {
        GuidedInstallProfileSeed {
            key: "sample_profile".to_owned(),
            display_name: "Sample Profile".to_owned(),
            creator: Some("Creator".to_owned()),
            family: "Support Libraries".to_owned(),
            official_source_url: "https://example.com".to_owned(),
            official_download_url: None,
            latest_check_strategy: Some("manual".to_owned()),
            latest_check_url: None,
            reference_source: vec!["official_docs".to_owned()],
            reviewed_at: "2026-03-11".to_owned(),
            sample_filenames: vec!["sample.ts4script".to_owned()],
            version_file_hints: vec!["sample".to_owned()],
            version_strategy: None,
            help_summary: "Sample".to_owned(),
            post_install_notes: Vec::new(),
            required_name_clues: vec!["sample".to_owned()],
            script_prefixes: vec!["sample".to_owned()],
            package_prefixes: Vec::new(),
            name_clues: vec!["sample".to_owned()],
            text_clues: Vec::new(),
            archive_path_clues: Vec::new(),
            install_folder_name: "Sample".to_owned(),
            allow_root_install: true,
            max_install_depth: 1,
            minimum_profile_files: 1,
            minimum_script_files: 1,
            required_all_filenames: Vec::new(),
            preserve_extensions: Vec::new(),
            preserve_prefixes: Vec::new(),
            dependency_keys: Vec::new(),
            incompatibility_keys: Vec::new(),
            review_reasons: Vec::new(),
            block_reasons: Vec::new(),
        }
    }

    #[test]
    fn validate_install_catalog_rejects_unsafe_install_depth() {
        let mut profile = sample_guided_profile();
        profile.max_install_depth = 2;
        let catalog = InstallCatalogSeed {
            seed_version: "test".to_owned(),
            guided_profiles: vec![profile],
            dependency_rules: Vec::new(),
            incompatibility_rules: Vec::new(),
            review_only_patterns: Vec::new(),
        };

        let error = validate_install_catalog(&catalog).expect_err("catalog should fail");
        assert!(error.to_string().contains("unsafe install depth"));
    }
}
