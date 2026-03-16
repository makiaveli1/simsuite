use std::collections::{HashMap, HashSet};

use crate::seed::{normalize_key, HeuristicSeed, SeedPack};

const MAX_CREATOR_WINDOW: usize = 4;
const MAX_CREATOR_OFFSET: usize = 4;
const MAX_PHRASE_WINDOW: usize = 3;
const MAX_TAG_SCAN_CHARS: usize = 72;
const GENERIC_LEADING_TOKENS: &[&str] = &[
    "ts4", "sims4", "package", "packages", "mod", "mods", "tsr", "cf", "mm", "alpha", "maxis",
    "match", "ea",
];
const GENERIC_SHORT_TAGS: &[&str] = &[
    "bg", "ep", "gp", "sp", "ea", "tsr", "cf", "mm", "hq", "wip", "ui", "cas", "bb",
];
const GENERIC_SET_NAME_TOKENS: &[&str] = &["cc", "ts4", "sims4", "mod", "mods"];
const STRONG_CREATOR_SUFFIX_TOKENS: &[&str] = &[
    "crib", "dresser", "bed", "hair", "skirt", "dress", "top", "jeans", "shorts", "trait",
    "preset", "eyeliner", "lashes", "nails", "nail", "menu", "override", "script",
];

#[derive(Debug, Clone)]
pub struct FilenameClassification {
    #[allow(dead_code)]
    pub normalized: String,
    pub tokens: Vec<String>,
    pub possible_creator: Option<String>,
    pub kind: String,
    pub subtype: Option<String>,
    pub set_name: Option<String>,
    pub version_label: Option<String>,
    pub support_tokens: Vec<String>,
    pub warning_flags: Vec<String>,
    pub confidence: f64,
}

#[derive(Debug, Clone)]
struct CreatorMatch {
    canonical_name: String,
    start: usize,
    end: usize,
    tagged: bool,
}

pub fn parse_filename(filename: &str, seed_pack: &SeedPack) -> FilenameClassification {
    let extension = filename
        .rsplit_once('.')
        .map(|(_, ext)| format!(".{}", ext.to_lowercase()))
        .unwrap_or_default();

    let base_name = filename.strip_suffix(&extension).unwrap_or(filename);
    let tokens = tokenize_filename(base_name);
    let normalized = tokens.join("_");

    let mut result = FilenameClassification {
        normalized,
        tokens,
        possible_creator: None,
        kind: "Unknown".to_owned(),
        subtype: None,
        set_name: None,
        version_label: None,
        support_tokens: Vec::new(),
        warning_flags: Vec::new(),
        confidence: 0.18,
    };

    if is_tray_extension(&extension, &seed_pack.heuristics) {
        result.kind = tray_kind_from_extension(&extension).to_owned();
        result.confidence = 0.99;
        return result;
    }

    let creator_match = detect_creator(base_name, &result.tokens, seed_pack);
    if let Some(matched_creator) = &creator_match {
        result.possible_creator = Some(matched_creator.canonical_name.clone());
        let creator_bonus = (0.26 - ((matched_creator.start as f64) * 0.03)).max(0.18)
            + if matched_creator.tagged { 0.04 } else { 0.0 };
        result.confidence += creator_bonus;

        if let Some(profile) = seed_pack
            .creator_profiles
            .get(&matched_creator.canonical_name)
        {
            if profile.likely_subtypes.len() == 1 {
                result.subtype = profile.likely_subtypes.first().cloned();
            }
        }
    }

    let blocked_indices = apply_keyword_scoring(
        &mut result,
        seed_pack,
        extension.as_str(),
        creator_match.as_ref(),
    );
    apply_creator_defaults(&mut result, seed_pack);
    derive_set_name(&mut result, &blocked_indices, creator_match.as_ref());
    apply_confidence_floors(&mut result);

    if result.kind == "Unknown" {
        result.warning_flags.push("no_category_detected".to_owned());
        result.confidence = result.confidence.min(0.35);
    }

    result.confidence = result.confidence.clamp(0.0, 0.99);
    result
}

pub fn detect_creator_hint(value: &str, seed_pack: &SeedPack) -> Option<String> {
    let tokens = tokenize_filename(value);
    detect_creator(value, &tokens, seed_pack).map(|matched| matched.canonical_name)
}

fn tokenize_filename(value: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let characters = value.chars().collect::<Vec<_>>();

    for index in 0..characters.len() {
        let current_char = characters[index];
        if !current_char.is_ascii_alphanumeric() {
            flush_token(&mut current, &mut tokens);
            continue;
        }

        if should_split_before(&characters, index) {
            flush_token(&mut current, &mut tokens);
        }

        current.push(current_char.to_ascii_lowercase());
    }

    flush_token(&mut current, &mut tokens);
    tokens
}

fn should_split_before(characters: &[char], index: usize) -> bool {
    if index == 0 {
        return false;
    }

    let current = characters[index];
    let previous = characters[index - 1];

    (previous.is_ascii_lowercase() && current.is_ascii_uppercase())
        || (previous.is_ascii_uppercase()
            && current.is_ascii_uppercase()
            && characters
                .get(index + 1)
                .is_some_and(|next| next.is_ascii_lowercase()))
}

fn flush_token(current: &mut String, tokens: &mut Vec<String>) {
    if !current.is_empty() {
        tokens.push(std::mem::take(current));
    }
}

fn is_tray_extension(extension: &str, heuristics: &HeuristicSeed) -> bool {
    heuristics
        .tray_extensions
        .iter()
        .any(|item| item == extension)
}

fn tray_kind_from_extension(extension: &str) -> &'static str {
    match extension {
        ".householdbinary" | ".hhi" | ".sgi" => "TrayHousehold",
        ".blueprint" | ".bpi" => "TrayLot",
        ".room" | ".rmi" => "TrayRoom",
        _ => "TrayItem",
    }
}

fn detect_creator(
    base_name: &str,
    tokens: &[String],
    seed_pack: &SeedPack,
) -> Option<CreatorMatch> {
    if let Some(bracketed_match) = detect_bracketed_creator(base_name, seed_pack) {
        return Some(bracketed_match);
    }

    if let Some(byline_match) = detect_byline_creator(base_name, seed_pack) {
        return Some(byline_match);
    }

    let token_keys = tokens
        .iter()
        .map(|token| normalize_key(token))
        .collect::<Vec<_>>();
    let search_limit = token_keys
        .len()
        .min(MAX_CREATOR_WINDOW + MAX_CREATOR_OFFSET + 1);

    for start in 0..search_limit.min(MAX_CREATOR_OFFSET + 1) {
        if is_ignored_leading_token(&token_keys[start], seed_pack) {
            continue;
        }

        let max_width = MAX_CREATOR_WINDOW.min(search_limit - start);
        for width in (1..=max_width).rev() {
            let candidate_key = join_token_window(&token_keys, start, width);
            if !seed_pack.parser_lexicon.contains(&candidate_key) {
                continue;
            }

            if let Some(canonical_name) = seed_pack.creator_lookup.get(&candidate_key) {
                return Some(CreatorMatch {
                    canonical_name: canonical_name.clone(),
                    start,
                    end: start + width,
                    tagged: false,
                });
            }
        }
    }

    if let Some(prefix_match) = detect_delimited_prefix_creator(base_name, seed_pack) {
        return Some(prefix_match);
    }

    if let Some(compact_match) = detect_compact_suffix_creator(base_name, tokens, seed_pack) {
        return Some(compact_match);
    }

    None
}

fn detect_bracketed_creator(base_name: &str, seed_pack: &SeedPack) -> Option<CreatorMatch> {
    let scan_slice = base_name
        .char_indices()
        .take_while(|(index, _)| *index <= MAX_TAG_SCAN_CHARS)
        .map(|(_, character)| character)
        .collect::<String>();
    let brackets = [('[', ']'), ('(', ')'), ('{', '}')];

    for (open, close) in brackets {
        let mut search_from = 0;
        while let Some(relative_open) = scan_slice[search_from..].find(open) {
            let absolute_open = search_from + relative_open;
            let content_start = absolute_open + open.len_utf8();
            let Some(relative_close) = scan_slice[content_start..].find(close) else {
                break;
            };
            let absolute_close = content_start + relative_close;
            let inside = scan_slice[content_start..absolute_close].trim();
            let candidate_key = normalize_key(inside);
            let prefix = &scan_slice[..absolute_open];
            if !can_use_bracket_as_creator_prefix(prefix, seed_pack) {
                search_from = absolute_close + close.len_utf8();
                continue;
            }

            if let Some(canonical_name) = seed_pack.creator_lookup.get(&candidate_key) {
                let token_start = tokenize_filename(prefix).len();
                let token_width = tokenize_filename(inside).len().max(1);
                return Some(CreatorMatch {
                    canonical_name: canonical_name.clone(),
                    start: token_start,
                    end: token_start + token_width,
                    tagged: true,
                });
            }

            if let Some(canonical_name) = resolve_creator_candidate(inside, seed_pack) {
                let token_start = tokenize_filename(prefix).len();
                let token_width = tokenize_filename(inside).len().max(1);
                return Some(CreatorMatch {
                    canonical_name,
                    start: token_start,
                    end: token_start + token_width,
                    tagged: true,
                });
            }

            search_from = absolute_close + close.len_utf8();
        }
    }

    None
}

fn is_ignored_leading_token(token: &str, seed_pack: &SeedPack) -> bool {
    seed_pack.version_token_set.contains(token)
        || seed_pack.support_token_set.contains(token)
        || GENERIC_LEADING_TOKENS.contains(&token)
}

fn can_use_bracket_as_creator_prefix(prefix: &str, seed_pack: &SeedPack) -> bool {
    tokenize_filename(prefix).into_iter().all(|token| {
        let normalized = normalize_key(&token);
        normalized
            .chars()
            .all(|character| character.is_ascii_digit())
            || is_ignored_leading_token(&normalized, seed_pack)
    })
}

fn detect_byline_creator(base_name: &str, seed_pack: &SeedPack) -> Option<CreatorMatch> {
    let lower = base_name.to_ascii_lowercase();
    let by_index = lower.find(" by ")?;
    let raw_candidate = base_name[(by_index + 4)..].trim();
    let canonical_name = resolve_creator_candidate(raw_candidate, seed_pack)?;
    let token_start = tokenize_filename(&base_name[..by_index]).len() + 1;
    let token_width = tokenize_filename(raw_candidate).len().max(1);

    Some(CreatorMatch {
        canonical_name,
        start: token_start,
        end: token_start + token_width,
        tagged: false,
    })
}

fn detect_delimited_prefix_creator(base_name: &str, seed_pack: &SeedPack) -> Option<CreatorMatch> {
    for delimiter in [" - ", "_", " x "] {
        let Some(index) = base_name.find(delimiter) else {
            continue;
        };
        if index == 0 || index > MAX_TAG_SCAN_CHARS {
            continue;
        }

        let raw_candidate = base_name[..index].trim();
        let canonical_name = match resolve_creator_candidate(raw_candidate, seed_pack) {
            Some(candidate) => candidate,
            None => continue,
        };
        let token_width = tokenize_filename(raw_candidate).len().max(1);
        return Some(CreatorMatch {
            canonical_name,
            start: 0,
            end: token_width,
            tagged: false,
        });
    }

    None
}

fn detect_compact_suffix_creator(
    base_name: &str,
    tokens: &[String],
    seed_pack: &SeedPack,
) -> Option<CreatorMatch> {
    if base_name.contains(['[', '(', '{']) || base_name.contains('_') || base_name.contains(" - ") {
        return None;
    }

    if tokens.len() < 3 || tokens.len() > MAX_CREATOR_WINDOW + 1 {
        return None;
    }

    let last_token = tokens.last()?;
    if !STRONG_CREATOR_SUFFIX_TOKENS.contains(&last_token.as_str()) {
        return None;
    }

    let prefix_token_count = tokens.len() - 1;
    if prefix_token_count == 0 || prefix_token_count > 2 {
        return None;
    }

    let prefix_len = base_name.len().saturating_sub(last_token.len());
    let raw_candidate = base_name[..prefix_len].trim();
    let canonical_name = resolve_creator_candidate(raw_candidate, seed_pack)?;

    Some(CreatorMatch {
        canonical_name,
        start: 0,
        end: prefix_token_count,
        tagged: false,
    })
}

fn resolve_creator_candidate(value: &str, seed_pack: &SeedPack) -> Option<String> {
    let compact = collapse_whitespace(value);
    if compact.is_empty()
        || compact.len() > 40
        || !compact
            .chars()
            .any(|character| character.is_ascii_alphabetic())
    {
        return None;
    }

    let tokens = tokenize_filename(&compact);
    if tokens.is_empty() || tokens.len() > MAX_CREATOR_WINDOW {
        return None;
    }

    let compact_key = normalize_key(&compact);
    if let Some(canonical_name) = seed_pack.creator_lookup.get(&compact_key) {
        return Some(canonical_name.clone());
    }

    let normalized_tokens = tokens
        .iter()
        .map(|token| normalize_key(token))
        .collect::<Vec<_>>();

    if normalized_tokens
        .iter()
        .all(|token| is_ignored_leading_token(token, seed_pack))
    {
        return None;
    }

    if normalized_tokens.iter().all(|token| {
        seed_pack.version_token_set.contains(token) || seed_pack.support_token_set.contains(token)
    }) {
        return None;
    }

    if normalized_tokens.len() == 1 && GENERIC_SHORT_TAGS.contains(&normalized_tokens[0].as_str()) {
        return None;
    }

    Some(compact)
}

fn collapse_whitespace(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn apply_keyword_scoring(
    result: &mut FilenameClassification,
    seed_pack: &SeedPack,
    extension: &str,
    creator_match: Option<&CreatorMatch>,
) -> HashSet<usize> {
    let token_keys = result
        .tokens
        .iter()
        .map(|token| normalize_key(token))
        .collect::<Vec<_>>();
    let mut scores = HashMap::from([
        ("CAS", 0_i32),
        ("BuildBuy", 0_i32),
        ("Gameplay", 0_i32),
        ("ScriptMods", 0_i32),
        ("OverridesAndDefaults", 0_i32),
        ("PosesAndAnimation", 0_i32),
        ("PresetsAndSliders", 0_i32),
    ]);
    let mut blocked_indices = HashSet::new();
    let mut subtype_score = 0_i32;

    if extension == ".ts4script" {
        add_score(&mut scores, "ScriptMods", 10);
        update_subtype(result, "Utilities", 3, &mut subtype_score);
    }

    let mut index = 0;
    while index < token_keys.len() {
        let mut matched = false;
        let max_width = MAX_PHRASE_WINDOW.min(token_keys.len() - index);

        for width in (1..=max_width).rev() {
            let phrase_key = join_token_window(&token_keys, index, width);
            if width > 1 && !seed_pack.parser_lexicon.contains(&phrase_key) {
                continue;
            }

            if seed_pack.version_token_set.contains(&phrase_key) {
                result.version_label = Some(join_display_window(&result.tokens, index, width));
                block_span(&mut blocked_indices, index, width);
                index += width;
                matched = true;
                break;
            }

            if seed_pack.support_token_set.contains(&phrase_key) {
                push_unique(
                    &mut result.support_tokens,
                    join_display_window(&result.tokens, index, width),
                );
                block_span(&mut blocked_indices, index, width);
                index += width;
                matched = true;
                break;
            }

            if apply_category_match(
                result,
                seed_pack,
                &mut scores,
                &mut blocked_indices,
                &mut subtype_score,
                index,
                width,
                phrase_key.as_str(),
            ) {
                index += width;
                matched = true;
                break;
            }
        }

        if !matched {
            index += 1;
        }
    }

    if should_promote_script_mod(&scores, result, creator_match, seed_pack) {
        add_score(&mut scores, "ScriptMods", 2);
    }

    let mut ordered_scores = scores.into_iter().collect::<Vec<_>>();
    ordered_scores.sort_by(|left, right| right.1.cmp(&left.1));

    if let Some((kind, score)) = ordered_scores.first() {
        if *score > 0 {
            result.kind = (*kind).to_owned();
            result.confidence += ((*score as f64) * 0.06).min(0.5);
        }
    }

    if ordered_scores.len() > 1
        && ordered_scores[0].1 > 0
        && ordered_scores[0].1 == ordered_scores[1].1
    {
        result
            .warning_flags
            .push("conflicting_category_signals".to_owned());
        result.confidence -= 0.08;
    }

    blocked_indices
}

fn apply_category_match(
    result: &mut FilenameClassification,
    seed_pack: &SeedPack,
    scores: &mut HashMap<&'static str, i32>,
    blocked_indices: &mut HashSet<usize>,
    subtype_score: &mut i32,
    start: usize,
    width: usize,
    phrase_key: &str,
) -> bool {
    let lookup = &seed_pack.keyword_lookup;

    if lookup.eyebrows.contains(phrase_key) {
        add_score(scores, "CAS", 4);
        update_subtype(result, "Eyebrows", 4, subtype_score);
        block_span(blocked_indices, start, width);
        return true;
    }

    if lookup.eyelashes.contains(phrase_key) {
        add_score(scores, "CAS", 4);
        update_subtype(result, "Eyelashes", 4, subtype_score);
        block_span(blocked_indices, start, width);
        return true;
    }

    if lookup.eyes.contains(phrase_key) {
        add_score(scores, "CAS", 3);
        update_subtype(result, "Eyes", 3, subtype_score);
        block_span(blocked_indices, start, width);
        return true;
    }

    if lookup.facial_hair.contains(phrase_key) {
        add_score(scores, "CAS", 3);
        update_subtype(result, "Facial Hair", 3, subtype_score);
        block_span(blocked_indices, start, width);
        return true;
    }

    if lookup.hair.contains(phrase_key) {
        add_score(scores, "CAS", 3);
        update_subtype(result, "Hair", 3, subtype_score);
        block_span(blocked_indices, start, width);
        return true;
    }

    if lookup.clothing.contains(phrase_key) {
        add_score(scores, "CAS", 2);
        update_subtype(result, clothing_subtype(phrase_key), 2, subtype_score);
        block_span(blocked_indices, start, width);
        return true;
    }

    if lookup.makeup.contains(phrase_key) {
        add_score(scores, "CAS", 3);
        update_subtype(result, "Makeup", 3, subtype_score);
        block_span(blocked_indices, start, width);
        return true;
    }

    if lookup.skin.contains(phrase_key) {
        add_score(scores, "CAS", 3);
        update_subtype(result, "Skin", 3, subtype_score);
        block_span(blocked_indices, start, width);
        return true;
    }

    if lookup.accessories.contains(phrase_key) {
        add_score(scores, "CAS", 2);
        update_subtype(result, "Accessories", 2, subtype_score);
        block_span(blocked_indices, start, width);
        return true;
    }

    if lookup.tattoos.contains(phrase_key) {
        add_score(scores, "CAS", 2);
        update_subtype(result, "Tattoos", 2, subtype_score);
        block_span(blocked_indices, start, width);
        return true;
    }

    if lookup.build_buy.contains(phrase_key) {
        add_score(scores, "BuildBuy", 2);
        update_subtype(result, build_buy_subtype(phrase_key), 2, subtype_score);
        block_span(blocked_indices, start, width);
        return true;
    }

    if lookup.override_keywords.contains(phrase_key) {
        add_score(scores, "OverridesAndDefaults", 4);
        update_subtype(result, override_subtype(phrase_key), 4, subtype_score);
        block_span(blocked_indices, start, width);
        return true;
    }

    if lookup.pose.contains(phrase_key) {
        add_score(scores, "PosesAndAnimation", 4);
        update_subtype(result, "Poses", 4, subtype_score);
        block_span(blocked_indices, start, width);
        return true;
    }

    if lookup.preset.contains(phrase_key) {
        add_score(scores, "PresetsAndSliders", 4);
        update_subtype(result, preset_subtype(phrase_key), 4, subtype_score);
        block_preset_tokens(blocked_indices, &result.tokens, start, width);
        return true;
    }

    if lookup.gameplay.contains(phrase_key) {
        add_score(scores, "Gameplay", 2);
        update_subtype(result, gameplay_subtype(phrase_key), 2, subtype_score);
        block_gameplay_tokens(blocked_indices, phrase_key, start, width);
        return true;
    }

    if lookup.utilities.contains(phrase_key) {
        add_score(scores, "ScriptMods", 3);
        add_score(scores, "Gameplay", 1);
        update_subtype(result, "Utilities", 3, subtype_score);
        block_utility_tokens(blocked_indices, phrase_key, start, width);
        return true;
    }

    false
}

fn should_promote_script_mod(
    scores: &HashMap<&'static str, i32>,
    result: &FilenameClassification,
    creator_match: Option<&CreatorMatch>,
    seed_pack: &SeedPack,
) -> bool {
    if result.kind == "ScriptMods" {
        return false;
    }

    if scores.get("ScriptMods").copied().unwrap_or_default() >= 4 {
        return true;
    }

    let Some(creator_match) = creator_match else {
        return false;
    };

    let Some(profile) = seed_pack
        .creator_profiles
        .get(&creator_match.canonical_name)
    else {
        return false;
    };

    profile.likely_kinds.iter().any(|kind| kind == "ScriptMods")
}

fn add_score(scores: &mut HashMap<&'static str, i32>, kind: &'static str, amount: i32) {
    *scores.entry(kind).or_default() += amount;
}

fn update_subtype(
    result: &mut FilenameClassification,
    subtype: &'static str,
    score: i32,
    subtype_score: &mut i32,
) {
    if result.subtype.is_none()
        || score > *subtype_score
        || (score == *subtype_score && is_generic_subtype(&result.subtype))
    {
        result.subtype = Some(subtype.to_owned());
        *subtype_score = score;
    }
}

fn is_generic_subtype(subtype: &Option<String>) -> bool {
    matches!(
        subtype.as_deref(),
        Some("Gameplay" | "BuildBuy" | "Clothing" | "Utilities" | "Presets")
    )
}

fn block_span(blocked_indices: &mut HashSet<usize>, start: usize, width: usize) {
    for index in start..(start + width) {
        blocked_indices.insert(index);
    }
}

fn block_preset_tokens(
    blocked_indices: &mut HashSet<usize>,
    tokens: &[String],
    start: usize,
    width: usize,
) {
    for index in start..(start + width) {
        let token = tokens[index].as_str();
        if matches!(token, "preset" | "slider" | "facial" | "body") {
            blocked_indices.insert(index);
        }
    }
}

fn block_gameplay_tokens(
    blocked_indices: &mut HashSet<usize>,
    phrase_key: &str,
    start: usize,
    width: usize,
) {
    if matches!(phrase_key, "system" | "systems" | "mod" | "mods") {
        block_span(blocked_indices, start, width);
    }
}

fn block_utility_tokens(
    blocked_indices: &mut HashSet<usize>,
    phrase_key: &str,
    start: usize,
    width: usize,
) {
    if matches!(
        phrase_key,
        "script"
            | "scripts"
            | "utility"
            | "ui"
            | "menu"
            | "tool"
            | "injector"
            | "xmlinjector"
            | "core"
            | "library"
            | "config"
            | "checker"
    ) {
        block_span(blocked_indices, start, width);
    }
}

fn join_token_window(tokens: &[String], start: usize, width: usize) -> String {
    tokens[start..(start + width)].join("")
}

fn join_display_window(tokens: &[String], start: usize, width: usize) -> String {
    tokens[start..(start + width)]
        .iter()
        .map(|token| title_case(token))
        .collect::<Vec<_>>()
        .join(" ")
}

fn apply_creator_defaults(result: &mut FilenameClassification, seed_pack: &SeedPack) {
    let Some(creator) = &result.possible_creator else {
        return;
    };

    let Some(profile) = seed_pack.creator_profiles.get(creator) else {
        return;
    };

    if result.kind == "Unknown" {
        if let Some(kind) = profile.likely_kinds.first() {
            result.kind = kind.clone();
            result.confidence += 0.18;
        }
    }

    if result.subtype.is_none() || is_generic_subtype(&result.subtype) {
        if let Some(subtype) = profile.likely_subtypes.first() {
            result.subtype = Some(subtype.clone());
        }
    }
}

fn derive_set_name(
    result: &mut FilenameClassification,
    blocked_indices: &HashSet<usize>,
    creator_match: Option<&CreatorMatch>,
) {
    let creator_range = creator_match.map(|matched| matched.start..matched.end);
    let remaining = result
        .tokens
        .iter()
        .enumerate()
        .filter_map(|(index, token)| {
            if creator_range
                .as_ref()
                .is_some_and(|range| range.contains(&index))
                || blocked_indices.contains(&index)
                || GENERIC_SET_NAME_TOKENS.contains(&token.as_str())
            {
                return None;
            }

            Some(title_case(token))
        })
        .collect::<Vec<_>>();

    if !remaining.is_empty() {
        result.set_name = Some(remaining.join(" "));
    }
}

fn clothing_subtype(token: &str) -> &'static str {
    match token {
        "dress" => "Dresses",
        "skirt" | "jeans" | "pants" | "shorts" => "Bottoms",
        "shoes" | "heels" | "boots" | "sneakers" => "Shoes",
        "top" | "shirt" | "jacket" | "coat" | "sweater" | "blouse" | "hoodie" | "cardigan"
        | "bodysuit" => "Tops",
        "swimsuit" => "Swimwear",
        _ => "Clothing",
    }
}

fn build_buy_subtype(token: &str) -> &'static str {
    match token {
        "sofa" | "chair" | "table" | "desk" | "bed" | "dresser" | "bench" | "couch"
        | "armchair" | "cabinet" | "bookcase" | "counter" | "barback" | "barbacks"
        | "fireplace" | "fireplaces" => "Furniture",
        "lamp" | "rug" | "clutter" | "decor" | "plant" | "shelf" | "shelves" | "frame"
        | "mirror" | "mirrors" => "Decor",
        "wall" | "walls" | "floor" | "floors" | "door" | "doors" | "window" | "windows"
        | "wallpaper" | "wallpapers" | "tile" | "tiles" | "paint" | "paints" | "foundation"
        | "foundations" | "terrain" | "roof" | "roofs" | "stair" | "stairs" | "staircase"
        | "fence" | "fences" | "railing" | "railings" | "spandrel" | "spandrels" | "frieze"
        | "friezes" | "trim" | "trims" | "ceiling" | "ceilings" | "panel" | "panels"
        | "panelling" | "paneling" | "entryway" | "entryways" | "entrance" | "entrances" => {
            "Build Surfaces"
        }
        "bathroom" | "kitchen" | "bedroom" | "living" | "dining" => "Room Sets",
        _ => "BuildBuy",
    }
}

fn gameplay_subtype(token: &str) -> &'static str {
    match token {
        "trait" | "traits" => "Traits",
        "career" | "careers" => "Careers",
        "aspiration" | "aspirations" => "Aspirations",
        "relationship" => "Relationship Systems",
        "pregnancy" | "childbirth" => "Pregnancy",
        "school" | "education" => "Education",
        "business" | "banking" | "taxes" => "Business",
        "recipe" | "recipes" => "Gameplay",
        "interaction" | "interactions" => "Gameplay",
        "lottrait" | "lottraits" | "lotchallenge" | "lotchallenges" => "Gameplay",
        "cheat" | "cheats" => "Gameplay",
        "autonomy" => "Autonomy",
        "npc" => "NPC Behavior",
        "healthcare" => "Healthcare",
        "romance" => "Romance",
        "family" => "Family Gameplay",
        _ => "Gameplay",
    }
}

fn override_subtype(token: &str) -> &'static str {
    match token {
        "default" | "defaults" => "Defaults",
        _ => "Overrides",
    }
}

fn preset_subtype(token: &str) -> &'static str {
    match token {
        "bodypreset" | "body" => "Body Presets",
        "facialpreset" | "jawpreset" | "chinpreset" => "Facial Presets",
        "nosepreset" => "Nose Presets",
        "lippreset" => "Lip Presets",
        "eyepreset" => "Eye Presets",
        "slider" => "Sliders",
        _ => "Presets",
    }
}

fn apply_confidence_floors(result: &mut FilenameClassification) {
    if result.kind == "PresetsAndSliders"
        && matches!(
            result.subtype.as_deref(),
            Some(
                "Body Presets" | "Facial Presets" | "Nose Presets" | "Lip Presets" | "Eye Presets"
            )
        )
    {
        result.confidence = result.confidence.max(0.58);
    }

    if result.kind == "OverridesAndDefaults"
        && matches!(result.subtype.as_deref(), Some("Overrides" | "Defaults"))
    {
        result.confidence = result.confidence.max(0.56);
    }

    if result.kind == "PosesAndAnimation" && matches!(result.subtype.as_deref(), Some("Poses")) {
        result.confidence = result.confidence.max(0.56);
    }

    if result.kind == "Gameplay" && matches!(result.subtype.as_deref(), Some("Pregnancy")) {
        result.confidence = result.confidence.max(0.56);
    }
}

fn push_unique(values: &mut Vec<String>, candidate: String) {
    if !values
        .iter()
        .any(|value| value.eq_ignore_ascii_case(&candidate))
    {
        values.push(candidate);
    }
}

fn title_case(value: &str) -> String {
    let mut characters = value.chars();
    match characters.next() {
        Some(first) => format!("{}{}", first.to_ascii_uppercase(), characters.as_str()),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use crate::seed::load_seed_pack;

    use super::parse_filename;

    #[test]
    fn parses_creator_and_hair_keywords() {
        let seed_pack = load_seed_pack().expect("seed pack");
        let parsed = parse_filename("simstrouble_breezy_hair_v2.package", &seed_pack);

        assert_eq!(parsed.possible_creator.as_deref(), Some("Simstrouble"));
        assert_eq!(parsed.kind, "CAS");
        assert_eq!(parsed.subtype.as_deref(), Some("Hair"));
        assert_eq!(parsed.version_label.as_deref(), Some("V2"));
        assert_eq!(parsed.set_name.as_deref(), Some("Breezy"));
        assert!(parsed.confidence >= 0.6);
    }

    #[test]
    fn detects_script_mods_by_extension_and_creator_aliases() {
        let seed_pack = load_seed_pack().expect("seed pack");
        let parsed = parse_filename("deaderpool_MCCC_MCCommandCenter.ts4script", &seed_pack);

        assert_eq!(parsed.possible_creator.as_deref(), Some("Deaderpool"));
        assert_eq!(parsed.kind, "ScriptMods");
        assert!(parsed.confidence > 0.9);
    }

    #[test]
    fn detects_camel_case_creator_names() {
        let seed_pack = load_seed_pack().expect("seed pack");
        let parsed = parse_filename("LittleMsSam_SendSimsToBed.package", &seed_pack);

        assert_eq!(parsed.possible_creator.as_deref(), Some("LittleMsSam"));
        assert_eq!(parsed.kind, "Gameplay");
        assert_eq!(parsed.set_name.as_deref(), Some("Send Sims To Bed"));
    }

    #[test]
    fn detects_bracketed_creator_tags() {
        let seed_pack = load_seed_pack().expect("seed pack");
        let parsed = parse_filename("[LMS] SendSimsToBed.package", &seed_pack);

        assert_eq!(parsed.possible_creator.as_deref(), Some("LittleMsSam"));
        assert_eq!(parsed.kind, "Gameplay");
        assert_eq!(parsed.set_name.as_deref(), Some("Send Sims To Bed"));
    }

    #[test]
    fn skips_release_tags_before_bracketed_creators() {
        let seed_pack = load_seed_pack().expect("seed pack");
        let parsed = parse_filename("[TSR] [PralineSims] Alfred Eyebrows.package", &seed_pack);

        assert_eq!(parsed.possible_creator.as_deref(), Some("Pralinesims"));
        assert_eq!(parsed.kind, "CAS");
        assert_eq!(parsed.subtype.as_deref(), Some("Eyebrows"));
    }

    #[test]
    fn detects_unknown_bracketed_creators_without_seed_aliases() {
        let seed_pack = load_seed_pack().expect("seed pack");
        let parsed = parse_filename("[cosmosim]Eyeliner_N3.package", &seed_pack);

        assert_eq!(parsed.possible_creator.as_deref(), Some("cosmosim"));
        assert_eq!(parsed.kind, "CAS");
    }

    #[test]
    fn detects_unknown_bracketed_creators_with_no_separator_after_tag() {
        let seed_pack = load_seed_pack().expect("seed pack");
        let parsed = parse_filename("[dogsill]abigail_hair.package", &seed_pack);

        assert_eq!(parsed.possible_creator.as_deref(), Some("dogsill"));
        assert_eq!(parsed.kind, "CAS");
        assert_eq!(parsed.subtype.as_deref(), Some("Hair"));
    }

    #[test]
    fn detects_spaced_bracketed_creators_and_skips_release_source_tags() {
        let seed_pack = load_seed_pack().expect("seed pack");
        let parsed = parse_filename(
            "[Gabymelove Sims] Aesthetic butterfly tattoo set - Butterfly Garden (Series I).package",
            &seed_pack,
        );
        let prefixed = parse_filename(
            "[TSR] [ERosetta]Modular Bespoke Study_Computer.package",
            &seed_pack,
        );

        assert_eq!(parsed.possible_creator.as_deref(), Some("Gabymelove Sims"));
        assert_eq!(prefixed.possible_creator.as_deref(), Some("ERosetta"));
    }

    #[test]
    fn detects_mixed_case_multi_word_bracketed_creators() {
        let seed_pack = load_seed_pack().expect("seed pack");
        let parsed = parse_filename(
            "[FinJingSims] FinJing Retail Store CAS Interactions.package",
            &seed_pack,
        );

        assert_eq!(parsed.possible_creator.as_deref(), Some("FinJingSims"));
    }

    #[test]
    fn detects_creator_patterns_from_real_world_filenames() {
        let seed_pack = load_seed_pack().expect("seed pack");
        let cases = [
            (
                "SERAWIS - Alive ( skin undertones - freckles ).package",
                "SERAWIS",
            ),
            ("08eva bottom by LUCKYEIGHT.package", "LUCKYEIGHT"),
            ("1_[SS] Recipes_Toddler_Porridge_SPA_ES_laura.package", "SS"),
            ("7NANA - (Ella) Suspender Shorts Set.package", "7NANA"),
            ("[LN] Auto-Classes (LOT VALUE ONLY).package", "LN"),
            ("[LS] BG Bookshelf Book v1.package", "LS"),
            ("[PB] Boho v2 toddler bed.package", "PB"),
            ("[QICC]Blissful_Bener_Clips_Accessory.package", "QICC"),
            ("[SS] EA Override Menu.package", "SS"),
            (
                "aithsims4 - Crystal Nail - ManiRoundSolidNeutrals M.package",
                "aithsims4",
            ),
            ("AdrienPastel x Natasha Skirt.package", "AdrienPastel"),
            ("AggressiveKitty_Toy_Corgi.package", "AggressiveKitty"),
            (
                "adrienpastel_top_landon_jacket_kids.package",
                "adrienpastel",
            ),
            ("Artemissy_Sensitive_Trait_V2.package", "Artemissy"),
            ("ANGISSI_F_LIPS_PRESET_29.package", "ANGISSI"),
            ("Andirz_SmartCoreScript_v.2.9.0.ts4script", "Andirz"),
            ("babybeesims_TwinkleToes.package", "babybeesims"),
            ("AuSims_F11_skysimseditfixed.package", "AuSims"),
            ("Aurum_RedsinTop_4_Female.package", "Aurum"),
            ("BabyBooCrib.package", "BabyBoo"),
            ("BabyBooDresser.package", "BabyBoo"),
            ("BackTrack_fSpicy_Bikini(Bottom).package", "BackTrack"),
            (
                "BackTrack_fNoelia_Jeans_Shorts_BeltACC.package",
                "BackTrack",
            ),
        ];

        for (filename, expected_creator) in cases {
            let parsed = parse_filename(filename, &seed_pack);
            assert_eq!(
                parsed.possible_creator.as_deref(),
                Some(expected_creator),
                "failed to detect creator for {filename}"
            );
        }
    }

    #[test]
    fn detects_eyebrow_subtype_and_multi_token_creator() {
        let seed_pack = load_seed_pack().expect("seed pack");
        let parsed = parse_filename("PralineSims_Alfred_Eyebrows_N78.package", &seed_pack);

        assert_eq!(parsed.possible_creator.as_deref(), Some("Pralinesims"));
        assert_eq!(parsed.kind, "CAS");
        assert_eq!(parsed.subtype.as_deref(), Some("Eyebrows"));
        assert_eq!(parsed.set_name.as_deref(), Some("Alfred N78"));
    }

    #[test]
    fn detects_build_buy_sets_with_collection_names() {
        let seed_pack = load_seed_pack().expect("seed pack");
        let parsed = parse_filename("myshunosun_crux_kitchen_counter.package", &seed_pack);

        assert_eq!(parsed.possible_creator.as_deref(), Some("Myshunosun"));
        assert_eq!(parsed.kind, "BuildBuy");
        assert_eq!(parsed.subtype.as_deref(), Some("Room Sets"));
        assert_eq!(parsed.set_name.as_deref(), Some("Crux"));
    }

    #[test]
    fn detects_plural_build_surface_terms() {
        let seed_pack = load_seed_pack().expect("seed pack");
        let parsed = parse_filename("Aesthetic Walls.package", &seed_pack);

        assert_eq!(parsed.kind, "BuildBuy");
        assert_eq!(parsed.subtype.as_deref(), Some("Build Surfaces"));
    }

    #[test]
    fn detects_more_real_world_build_buy_object_terms() {
        let seed_pack = load_seed_pack().expect("seed pack");
        let entryway = parse_filename(
            "PC-TS4-CountryCrafter-GrandEntrywayTextures.package",
            &seed_pack,
        );
        let barback = parse_filename("PC-TS4-CBK-TrattoriaLargeBarback-DR.package", &seed_pack);
        let fireplace = parse_filename("SYB_Advent2022_Fireplace_medium.package", &seed_pack);

        assert_eq!(entryway.kind, "BuildBuy");
        assert_eq!(entryway.subtype.as_deref(), Some("Build Surfaces"));
        assert_eq!(barback.kind, "BuildBuy");
        assert_eq!(barback.subtype.as_deref(), Some("Furniture"));
        assert_eq!(fireplace.kind, "BuildBuy");
        assert_eq!(fireplace.subtype.as_deref(), Some("Furniture"));
    }

    #[test]
    fn detects_gameplay_sets_without_hiding_the_mod_name() {
        let seed_pack = load_seed_pack().expect("seed pack");
        let parsed = parse_filename("adeepindigo_HealthcareRedux.package", &seed_pack);

        assert_eq!(parsed.possible_creator.as_deref(), Some("adeepindigo"));
        assert_eq!(parsed.kind, "Gameplay");
        assert_eq!(parsed.set_name.as_deref(), Some("Healthcare Redux"));
    }

    #[test]
    fn detects_plural_gameplay_terms() {
        let seed_pack = load_seed_pack().expect("seed pack");
        let parsed = parse_filename("BosseladyTV_Child_Aspirations_Bundle.package", &seed_pack);

        assert_eq!(parsed.kind, "Gameplay");
        assert_eq!(parsed.subtype.as_deref(), Some("Aspirations"));
    }

    #[test]
    fn detects_preset_subtype_from_phrase_tokens() {
        let seed_pack = load_seed_pack().expect("seed pack");
        let parsed = parse_filename("obscurus_nose_preset.package", &seed_pack);

        assert_eq!(parsed.possible_creator.as_deref(), Some("Obscurus"));
        assert_eq!(parsed.kind, "PresetsAndSliders");
        assert_eq!(parsed.subtype.as_deref(), Some("Nose Presets"));
        assert_eq!(parsed.set_name.as_deref(), Some("Nose"));
        assert!(parsed.confidence >= 0.58);
    }

    #[test]
    fn pose_pack_keywords_clear_the_low_confidence_floor() {
        let seed_pack = load_seed_pack().expect("seed pack");
        let parsed = parse_filename("SWIClingToYouPosePack.package", &seed_pack);

        assert_eq!(parsed.kind, "PosesAndAnimation");
        assert_eq!(parsed.subtype.as_deref(), Some("Poses"));
        assert!(parsed.confidence >= 0.56);
    }

    #[test]
    fn replacement_keywords_clear_the_low_confidence_floor() {
        let seed_pack = load_seed_pack().expect("seed pack");
        let parsed = parse_filename("Royalty Mod Easel Replacements Large.package", &seed_pack);

        assert_eq!(parsed.kind, "OverridesAndDefaults");
        assert_eq!(parsed.subtype.as_deref(), Some("Overrides"));
        assert!(parsed.confidence >= 0.56);
    }

    #[test]
    fn childbirth_packages_clear_the_low_confidence_floor() {
        let seed_pack = load_seed_pack().expect("seed pack");
        let parsed = parse_filename(
            "z_Pandasama_ChildBirth_mod_v1.95_SPA_ES_Dareksimmer.package",
            &seed_pack,
        );

        assert_eq!(parsed.kind, "Gameplay");
        assert_eq!(parsed.subtype.as_deref(), Some("Pregnancy"));
        assert!(parsed.confidence >= 0.56);
    }

    #[test]
    fn detects_tray_files_without_mod_classification() {
        let seed_pack = load_seed_pack().expect("seed pack");
        let parsed = parse_filename("0x00112233.householdbinary", &seed_pack);

        assert_eq!(parsed.kind, "TrayHousehold");
        assert!(parsed.confidence > 0.9);
    }

    #[test]
    fn flags_unknown_files_for_review() {
        let seed_pack = load_seed_pack().expect("seed pack");
        let parsed = parse_filename("ab12_final_new.package", &seed_pack);

        assert_eq!(parsed.kind, "Unknown");
        assert!(parsed
            .warning_flags
            .contains(&"no_category_detected".to_owned()));
    }
}
