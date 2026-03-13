use std::{path::Path, time::Duration};

use chrono::{DateTime, Utc};
use regex::Regex;
use reqwest::{blocking::Client, Url};
use rusqlite::{params, Connection, OptionalExtension};
use sha2::{Digest, Sha256};

use crate::{
    error::{AppError, AppResult},
    models::{
        SpecialExistingInstallState, SpecialInstalledState, SpecialOfficialLatestInfo,
        SpecialVersionStatus,
    },
    seed::GuidedInstallProfileSeed,
};

const LATEST_CACHE_HOURS: i64 = 12;

#[derive(Debug, Clone)]
pub struct StoredSpecialModFamilyState {
    pub installed: SpecialInstalledState,
    pub latest: Option<SpecialOfficialLatestInfo>,
}

#[derive(Debug, Clone)]
pub struct VersionComparison {
    pub incoming_version: Option<String>,
    pub incoming_signature: Option<String>,
    pub version_status: SpecialVersionStatus,
    pub same_version: bool,
    pub same_version_signature_mismatch: bool,
}

#[derive(Debug, Clone)]
pub struct SignatureEntry {
    pub filename: String,
    pub size: i64,
    pub hash: Option<String>,
}

pub fn load_family_state(
    connection: &Connection,
    profile_key: &str,
    profile_name: &str,
    existing_install_state: &SpecialExistingInstallState,
    install_path: Option<String>,
) -> AppResult<StoredSpecialModFamilyState> {
    let row = connection
        .query_row(
            "SELECT
                profile_key,
                profile_name,
                install_state,
                install_path,
                installed_version,
                installed_signature,
                source_item_id,
                checked_at,
                latest_source_url,
                latest_download_url,
                latest_version,
                latest_checked_at,
                latest_confidence,
                latest_status,
                latest_note
             FROM special_mod_family_state
             WHERE profile_key = ?1",
            params![profile_key],
            |row| {
                Ok(StoredSpecialModFamilyState {
                    installed: SpecialInstalledState {
                        profile_key: row.get(0)?,
                        profile_name: row.get(1)?,
                        install_state: parse_install_state(&row.get::<_, String>(2)?),
                        install_path: row.get(3)?,
                        installed_version: row.get(4)?,
                        installed_signature: row.get(5)?,
                        source_item_id: row.get(6)?,
                        checked_at: row.get(7)?,
                    },
                    latest: Some(SpecialOfficialLatestInfo {
                        source_url: row.get(8)?,
                        download_url: row.get(9)?,
                        latest_version: row.get(10)?,
                        checked_at: row.get(11)?,
                        confidence: row.get::<_, Option<f64>>(12)?.unwrap_or_default(),
                        status: row
                            .get::<_, Option<String>>(13)?
                            .unwrap_or_else(|| "unknown".to_owned()),
                        note: row.get(14)?,
                    }),
                })
            },
        )
        .optional()?;

    let mut state = row.unwrap_or_else(|| StoredSpecialModFamilyState {
        installed: SpecialInstalledState {
            profile_key: profile_key.to_owned(),
            profile_name: profile_name.to_owned(),
            install_state: existing_install_state.clone(),
            install_path: install_path.clone(),
            installed_version: None,
            installed_signature: None,
            source_item_id: None,
            checked_at: None,
        },
        latest: None,
    });

    state.installed.install_state = existing_install_state.clone();
    state.installed.install_path = install_path;
    state.installed.profile_name = profile_name.to_owned();
    Ok(state)
}

pub fn save_family_state(
    connection: &Connection,
    state: &StoredSpecialModFamilyState,
) -> AppResult<()> {
    let latest = state.latest.clone().unwrap_or(SpecialOfficialLatestInfo {
        source_url: None,
        download_url: None,
        latest_version: None,
        checked_at: None,
        confidence: 0.0,
        status: "unknown".to_owned(),
        note: None,
    });

    connection.execute(
        "INSERT INTO special_mod_family_state (
            profile_key,
            profile_name,
            install_state,
            install_path,
            installed_version,
            installed_signature,
            source_item_id,
            checked_at,
            latest_source_url,
            latest_download_url,
            latest_version,
            latest_checked_at,
            latest_confidence,
            latest_status,
            latest_note,
            updated_at
         ) VALUES (
            ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, CURRENT_TIMESTAMP
         )
         ON CONFLICT(profile_key) DO UPDATE SET
            profile_name = excluded.profile_name,
            install_state = excluded.install_state,
            install_path = excluded.install_path,
            installed_version = excluded.installed_version,
            installed_signature = excluded.installed_signature,
            source_item_id = excluded.source_item_id,
            checked_at = excluded.checked_at,
            latest_source_url = excluded.latest_source_url,
            latest_download_url = excluded.latest_download_url,
            latest_version = excluded.latest_version,
            latest_checked_at = excluded.latest_checked_at,
            latest_confidence = excluded.latest_confidence,
            latest_status = excluded.latest_status,
            latest_note = excluded.latest_note,
            updated_at = CURRENT_TIMESTAMP",
        params![
            state.installed.profile_key,
            state.installed.profile_name,
            install_state_label(&state.installed.install_state),
            state.installed.install_path,
            state.installed.installed_version,
            state.installed.installed_signature,
            state.installed.source_item_id,
            state.installed.checked_at,
            latest.source_url,
            latest.download_url,
            latest.latest_version,
            latest.checked_at,
            latest.confidence,
            latest.status,
            latest.note,
        ],
    )?;
    Ok(())
}

pub fn build_signature(entries: &[SignatureEntry]) -> Option<String> {
    if entries.is_empty() {
        return None;
    }

    let mut lines = entries
        .iter()
        .map(|entry| {
            let normalized_hash = entry.hash.clone().unwrap_or_default().to_lowercase();
            let size_token = if normalized_hash.is_empty() {
                entry.size.to_string()
            } else {
                String::new()
            };
            format!(
                "{}|{}|{}",
                entry.filename.to_lowercase(),
                size_token,
                normalized_hash
            )
        })
        .collect::<Vec<_>>();
    lines.sort();

    let mut hasher = Sha256::new();
    for line in &lines {
        hasher.update(line.as_bytes());
        hasher.update(b"\n");
    }

    Some(hex::encode(hasher.finalize()))
}

pub fn extract_version_from_values(values: &[String]) -> Option<String> {
    values
        .iter()
        .flat_map(|value| extract_ranked_version_candidates(value))
        .max_by(compare_ranked_candidates)
        .map(|candidate| candidate.normalized)
}

pub fn extract_version_from_value(value: &str) -> Option<String> {
    extract_ranked_version_candidates(value)
        .into_iter()
        .max_by(compare_ranked_candidates)
        .map(|candidate| candidate.normalized)
}

pub fn extract_version_candidates_from_value(value: &str) -> Vec<String> {
    let mut candidates = extract_ranked_version_candidates(value)
        .into_iter()
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| compare_ranked_candidates(right, left));
    candidates.dedup_by(|left, right| left.normalized == right.normalized);
    candidates
        .into_iter()
        .map(|candidate| candidate.normalized)
        .collect()
}

pub fn compare_versions(
    installed_present: bool,
    installed_version: Option<&str>,
    installed_signature: Option<&str>,
    incoming_version: Option<&str>,
    incoming_signature: Option<&str>,
) -> SpecialVersionStatus {
    if !installed_present {
        return SpecialVersionStatus::NotInstalled;
    }

    if let (Some(left), Some(right)) = (installed_signature, incoming_signature) {
        if !left.is_empty() && left == right {
            return SpecialVersionStatus::SameVersion;
        }
    }

    match (
        installed_version.and_then(parse_version_parts),
        incoming_version.and_then(parse_version_parts),
    ) {
        (Some(installed), Some(incoming)) => match incoming.cmp(&installed) {
            std::cmp::Ordering::Greater => SpecialVersionStatus::IncomingNewer,
            std::cmp::Ordering::Equal => {
                if let (Some(left), Some(right)) = (installed_signature, incoming_signature) {
                    if !left.is_empty() && !right.is_empty() && left != right {
                        SpecialVersionStatus::Unknown
                    } else {
                        SpecialVersionStatus::SameVersion
                    }
                } else {
                    SpecialVersionStatus::SameVersion
                }
            }
            std::cmp::Ordering::Less => SpecialVersionStatus::IncomingOlder,
        },
        _ => SpecialVersionStatus::Unknown,
    }
}

pub fn build_version_comparison(
    installed: &SpecialInstalledState,
    incoming_version: Option<String>,
    incoming_signature: Option<String>,
    allow_same_version_signature_mismatch: bool,
) -> VersionComparison {
    let base_status = compare_versions(
        installed.install_state != SpecialExistingInstallState::NotInstalled,
        installed.installed_version.as_deref(),
        installed.installed_signature.as_deref(),
        incoming_version.as_deref(),
        incoming_signature.as_deref(),
    );
    let same_version_signature_mismatch = matches!(base_status, SpecialVersionStatus::Unknown)
        && allow_same_version_signature_mismatch
        && same_release_label_mismatch_only(
            installed.installed_version.as_deref(),
            installed.installed_signature.as_deref(),
            incoming_version.as_deref(),
            incoming_signature.as_deref(),
        );
    let version_status = if same_version_signature_mismatch {
        SpecialVersionStatus::SameVersion
    } else {
        base_status
    };
    let same_version = version_status == SpecialVersionStatus::SameVersion;

    VersionComparison {
        incoming_version,
        incoming_signature,
        version_status,
        same_version,
        same_version_signature_mismatch,
    }
}

fn same_release_label_mismatch_only(
    installed_version: Option<&str>,
    installed_signature: Option<&str>,
    incoming_version: Option<&str>,
    incoming_signature: Option<&str>,
) -> bool {
    let (Some(installed), Some(incoming), Some(left_signature), Some(right_signature)) = (
        installed_version.and_then(parse_version_parts),
        incoming_version.and_then(parse_version_parts),
        installed_signature,
        incoming_signature,
    ) else {
        return false;
    };

    installed == incoming
        && !left_signature.is_empty()
        && !right_signature.is_empty()
        && left_signature != right_signature
}

pub fn load_or_refresh_latest_info(
    connection: &Connection,
    profile: &GuidedInstallProfileSeed,
    allow_network: bool,
) -> AppResult<Option<SpecialOfficialLatestInfo>> {
    let cached = load_cached_latest_info(connection, &profile.key)?;
    if cached
        .as_ref()
        .is_some_and(|info| latest_cache_is_fresh(info.checked_at.as_deref()))
    {
        return Ok(cached);
    }

    if !allow_network {
        return Ok(cached);
    }

    let latest = fetch_latest_info(profile).or_else(|error| {
        Ok::<_, AppError>(SpecialOfficialLatestInfo {
            source_url: profile
                .latest_check_url
                .clone()
                .or_else(|| Some(profile.official_source_url.clone())),
            download_url: None,
            latest_version: None,
            checked_at: Some(Utc::now().to_rfc3339()),
            confidence: 0.0,
            status: "unknown".to_owned(),
            note: Some(error.to_string()),
        })
    })?;

    connection.execute(
        "INSERT INTO special_mod_family_state (
            profile_key,
            profile_name,
            install_state,
            latest_source_url,
            latest_download_url,
            latest_version,
            latest_checked_at,
            latest_confidence,
            latest_status,
            latest_note,
            updated_at
        ) VALUES (?1, ?2, 'not_installed', ?3, ?4, ?5, ?6, ?7, ?8, ?9, CURRENT_TIMESTAMP)
        ON CONFLICT(profile_key) DO UPDATE SET
            profile_name = excluded.profile_name,
            latest_source_url = excluded.latest_source_url,
            latest_download_url = excluded.latest_download_url,
            latest_version = excluded.latest_version,
            latest_checked_at = excluded.latest_checked_at,
            latest_confidence = excluded.latest_confidence,
            latest_status = excluded.latest_status,
            latest_note = excluded.latest_note,
            updated_at = CURRENT_TIMESTAMP",
        params![
            profile.key,
            profile.display_name,
            latest.source_url,
            latest.download_url,
            latest.latest_version,
            latest.checked_at,
            latest.confidence,
            latest.status,
            latest.note,
        ],
    )?;

    Ok(Some(latest))
}

fn load_cached_latest_info(
    connection: &Connection,
    profile_key: &str,
) -> AppResult<Option<SpecialOfficialLatestInfo>> {
    connection
        .query_row(
            "SELECT
                latest_source_url,
                latest_download_url,
                latest_version,
                latest_checked_at,
                latest_confidence,
                latest_status,
                latest_note
             FROM special_mod_family_state
             WHERE profile_key = ?1",
            params![profile_key],
            |row| {
                Ok(SpecialOfficialLatestInfo {
                    source_url: row.get(0)?,
                    download_url: row.get(1)?,
                    latest_version: row.get(2)?,
                    checked_at: row.get(3)?,
                    confidence: row.get::<_, Option<f64>>(4)?.unwrap_or_default(),
                    status: row
                        .get::<_, Option<String>>(5)?
                        .unwrap_or_else(|| "unknown".to_owned()),
                    note: row.get(6)?,
                })
            },
        )
        .optional()
        .map_err(Into::into)
}

fn latest_cache_is_fresh(checked_at: Option<&str>) -> bool {
    let Some(checked_at) = checked_at else {
        return false;
    };
    let Ok(parsed) = DateTime::parse_from_rfc3339(checked_at) else {
        return false;
    };
    Utc::now()
        .signed_duration_since(parsed.with_timezone(&Utc))
        .num_hours()
        < LATEST_CACHE_HOURS
}

fn fetch_latest_info(profile: &GuidedInstallProfileSeed) -> AppResult<SpecialOfficialLatestInfo> {
    match profile.latest_check_strategy.as_deref().unwrap_or("manual") {
        "mccc_downloads_page" => fetch_mccc_latest_info(profile),
        "xml_injector_page" => fetch_xml_injector_latest_info(profile),
        "github_releases" => fetch_github_latest_info(profile),
        "protected_page" | "manual" => Ok(unknown_latest(
            profile,
            "This official site blocks safe automatic version checks right now.",
        )),
        _ => Ok(unknown_latest(
            profile,
            "This special mod does not have a supported latest-version checker yet.",
        )),
    }
}

fn fetch_mccc_latest_info(
    profile: &GuidedInstallProfileSeed,
) -> AppResult<SpecialOfficialLatestInfo> {
    let source_url = profile
        .latest_check_url
        .clone()
        .unwrap_or_else(|| profile.official_source_url.clone());
    let response = client()?.get(&source_url).send()?.error_for_status()?;
    let body = response.text()?;
    Ok(parse_mccc_latest_html(profile, &source_url, &body))
}

fn fetch_github_latest_info(
    profile: &GuidedInstallProfileSeed,
) -> AppResult<SpecialOfficialLatestInfo> {
    let source_url = profile.latest_check_url.clone().unwrap_or_else(|| {
        format!(
            "{}/releases/latest",
            profile.official_source_url.trim_end_matches('/')
        )
    });
    let response = client()?.get(&source_url).send()?.error_for_status()?;
    let final_url = response.url().to_string();
    Ok(parse_github_latest_url(profile, &final_url))
}

fn fetch_xml_injector_latest_info(
    profile: &GuidedInstallProfileSeed,
) -> AppResult<SpecialOfficialLatestInfo> {
    let source_url = profile
        .latest_check_url
        .clone()
        .unwrap_or_else(|| profile.official_source_url.clone());
    let response = client()?.get(&source_url).send()?.error_for_status()?;
    let final_url = response.url().to_string();
    let body = response.text()?;
    Ok(parse_xml_injector_latest_html(profile, &final_url, &body))
}

fn unknown_latest(profile: &GuidedInstallProfileSeed, note: &str) -> SpecialOfficialLatestInfo {
    SpecialOfficialLatestInfo {
        source_url: profile
            .latest_check_url
            .clone()
            .or_else(|| Some(profile.official_source_url.clone())),
        download_url: profile.official_download_url.clone(),
        latest_version: None,
        checked_at: Some(Utc::now().to_rfc3339()),
        confidence: 0.0,
        status: "unknown".to_owned(),
        note: Some(note.to_owned()),
    }
}

fn parse_mccc_latest_html(
    profile: &GuidedInstallProfileSeed,
    source_url: &str,
    body: &str,
) -> SpecialOfficialLatestInfo {
    let version_re =
        Regex::new(r"(?i)MC Command Center\s+([0-9]+(?:\.[0-9]+)+)").expect("mccc version regex");
    let download_re = Regex::new(r#"(?i)href="([^"]*McCmdCenter_AllModules_[^"]+\.zip[^"]*)""#)
        .expect("mccc download regex");

    let latest_version = version_re
        .captures(body)
        .and_then(|captures| captures.get(1))
        .map(|matched| normalize_version_value(matched.as_str()));
    let download_url = download_re
        .captures(body)
        .and_then(|captures| captures.get(1))
        .map(|matched| matched.as_str().to_owned());

    SpecialOfficialLatestInfo {
        source_url: Some(source_url.to_owned()),
        download_url: download_url.or_else(|| profile.official_download_url.clone()),
        latest_version,
        checked_at: Some(Utc::now().to_rfc3339()),
        confidence: 0.94,
        status: "known".to_owned(),
        note: None,
    }
}

fn parse_github_latest_url(
    profile: &GuidedInstallProfileSeed,
    final_url: &str,
) -> SpecialOfficialLatestInfo {
    let latest_version = Regex::new(r"/tag/v?([^/?#]+)$")
        .expect("github tag regex")
        .captures(final_url)
        .and_then(|captures| captures.get(1))
        .map(|matched| normalize_version_value(matched.as_str()));

    SpecialOfficialLatestInfo {
        source_url: Some(final_url.to_owned()),
        download_url: profile.official_download_url.clone(),
        latest_version,
        checked_at: Some(Utc::now().to_rfc3339()),
        confidence: 0.92,
        status: "known".to_owned(),
        note: None,
    }
}

fn parse_xml_injector_latest_html(
    profile: &GuidedInstallProfileSeed,
    source_url: &str,
    body: &str,
) -> SpecialOfficialLatestInfo {
    let version_re = Regex::new(
        r"(?i)(?:current version of the xml injector is version\s*|xmlinjector_script_v)([0-9]+(?:\.[0-9]+)+)",
    )
    .expect("xml injector version regex");
    let download_re =
        Regex::new(r#"(?i)href=['"]([^'"]*XmlInjector_Script_v[^'"]+\.zip[^'"]*)['"]"#)
            .expect("xml injector download regex");

    let latest_version = version_re
        .captures(body)
        .and_then(|captures| captures.get(1))
        .map(|matched| normalize_version_value(matched.as_str()));
    let download_url = download_re
        .captures(body)
        .and_then(|captures| captures.get(1))
        .and_then(|matched| resolve_url(source_url, matched.as_str()));

    SpecialOfficialLatestInfo {
        source_url: Some(source_url.to_owned()),
        download_url: download_url.or_else(|| profile.official_download_url.clone()),
        latest_version,
        checked_at: Some(Utc::now().to_rfc3339()),
        confidence: 0.93,
        status: "known".to_owned(),
        note: None,
    }
}

fn resolve_url(base_url: &str, value: &str) -> Option<String> {
    if value.trim().is_empty() {
        return None;
    }

    if value.starts_with("https://") || value.starts_with("http://") {
        return Some(value.to_owned());
    }

    Url::parse(base_url)
        .ok()
        .and_then(|base| base.join(value).ok())
        .map(|resolved| resolved.to_string())
}

fn client() -> AppResult<Client> {
    Client::builder()
        .redirect(reqwest::redirect::Policy::limited(6))
        .timeout(Duration::from_secs(12))
        .user_agent("SimSuite/0.1 latest-check")
        .build()
        .map_err(AppError::from)
}

fn parse_install_state(value: &str) -> SpecialExistingInstallState {
    match value {
        "clean" => SpecialExistingInstallState::Clean,
        "repairable" => SpecialExistingInstallState::Repairable,
        "blocked" => SpecialExistingInstallState::Blocked,
        _ => SpecialExistingInstallState::NotInstalled,
    }
}

fn install_state_label(value: &SpecialExistingInstallState) -> &'static str {
    match value {
        SpecialExistingInstallState::NotInstalled => "not_installed",
        SpecialExistingInstallState::Clean => "clean",
        SpecialExistingInstallState::Repairable => "repairable",
        SpecialExistingInstallState::Blocked => "blocked",
    }
}

pub fn normalize_version_value(value: &str) -> String {
    value
        .trim()
        .trim_start_matches(['v', 'V'])
        .replace(['_', '-'], ".")
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionCandidate {
    pub raw_value: String,
    pub normalized: String,
    pub score: i64,
    pub parts: Vec<i64>,
}

fn extract_ranked_version_candidates(value: &str) -> Vec<VersionCandidate> {
    let mut candidates = Vec::new();
    let version_with_prefix =
        Regex::new(r"(?i)\b(?:version|ver|v)[\s._-]*([0-9]+(?:[._-][0-9]+){0,3})\b")
            .expect("version regex");
    for captures in version_with_prefix.captures_iter(value) {
        if let Some(matched) = captures.get(1) {
            if let Some(candidate) = build_ranked_candidate(value, matched.as_str(), true) {
                candidates.push(candidate);
            }
        }
    }

    let sequence = Regex::new(r"(?i)(?:^|[^0-9])([0-9]{1,4}(?:[._-][0-9]{1,4}){1,3})(?:[^0-9]|$)")
        .expect("sequence regex");
    for captures in sequence.captures_iter(value) {
        if let Some(matched) = captures.get(1) {
            if let Some(candidate) = build_ranked_candidate(value, matched.as_str(), false) {
                candidates.push(candidate);
            }
        }
    }

    candidates
}

fn build_ranked_candidate(
    source: &str,
    raw_value: &str,
    prefixed: bool,
) -> Option<VersionCandidate> {
    let normalized = normalize_version_value(raw_value);
    let parts = parse_version_parts(&normalized)?;
    Some(VersionCandidate {
        raw_value: raw_value.to_owned(),
        normalized,
        score: score_version_candidate(source, &parts, prefixed),
        parts,
    })
}

fn score_version_candidate(source: &str, parts: &[i64], prefixed: bool) -> i64 {
    let mut score = (parts.len() as i64) * 4;
    let lowered = source.to_ascii_lowercase();
    if prefixed || lowered.contains("version") {
        score += 40;
    }

    let major = parts.first().copied().unwrap_or_default();
    if (2000..=2100).contains(&major) {
        score += 50;
    }

    if major <= 2
        && parts.len() >= 3
        && parts.get(1).copied().unwrap_or_default() >= 50
        && parts.get(2).copied().unwrap_or_default() >= 50
    {
        // Sims game patch numbers like 1.113.277 should not beat actual mod releases.
        score -= 40;
    }

    if major == 0 {
        score -= 8;
    }

    score
}

fn compare_ranked_candidates(
    left: &VersionCandidate,
    right: &VersionCandidate,
) -> std::cmp::Ordering {
    left.score
        .cmp(&right.score)
        .then_with(|| left.parts.cmp(&right.parts))
        .then_with(|| left.normalized.len().cmp(&right.normalized.len()))
}

pub fn parse_version_parts(value: &str) -> Option<Vec<i64>> {
    let normalized = normalize_version_value(value);
    let parts = normalized
        .split('.')
        .filter(|part| !part.trim().is_empty())
        .map(|part| part.parse::<i64>().ok())
        .collect::<Option<Vec<_>>>()?;
    if parts.is_empty() {
        None
    } else {
        Some(parts)
    }
}

pub fn extract_version_candidates_with_scores(value: &str) -> Vec<VersionCandidate> {
    let mut candidates = extract_ranked_version_candidates(value);
    candidates.sort_by(|left, right| compare_ranked_candidates(right, left));
    candidates.dedup_by(|left, right| left.normalized == right.normalized);
    candidates
}

pub fn version_hints_from_profile(
    profile: &GuidedInstallProfileSeed,
    display_name: &str,
) -> Vec<String> {
    let mut values = vec![display_name.to_owned()];
    values.extend(profile.version_file_hints.iter().cloned());
    values.extend(profile.sample_filenames.iter().cloned());
    values
}

pub fn signature_entries_from_paths(paths: &[&Path]) -> Vec<SignatureEntry> {
    paths
        .iter()
        .filter_map(|path| {
            let metadata = path.metadata().ok()?;
            let filename = path.file_name()?.to_string_lossy().to_string();
            Some(SignatureEntry {
                filename,
                size: metadata.len() as i64,
                hash: None,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{
        build_signature, compare_versions, extract_version_candidates_from_value,
        extract_version_from_values, parse_github_latest_url, parse_mccc_latest_html,
        parse_xml_injector_latest_html, unknown_latest, SignatureEntry,
    };
    use crate::models::SpecialVersionStatus;
    use crate::seed::load_seed_pack;

    #[test]
    fn lumpinou_toolbox_uses_same_release_signature_policy() {
        let pack = load_seed_pack().expect("seed pack");
        let profile = pack
            .install_catalog
            .guided_profiles
            .into_iter()
            .find(|profile| profile.key == "lumpinou_toolbox")
            .expect("lumpinou toolbox profile");
        let policy = profile
            .version_strategy
            .as_ref()
            .and_then(|strategy| strategy.same_version_signature_policy.as_deref());
        assert_eq!(policy, Some("same_release"));
    }

    #[test]
    fn parses_mccc_latest_from_downloads_page_html() {
        let mut profile = load_seed_pack()
            .expect("seed pack")
            .install_catalog
            .guided_profiles
            .into_iter()
            .find(|profile| profile.key == "mccc")
            .expect("mccc");
        profile.official_download_url = Some(
            "https://deaderpool-mccc.com/release/McCmdCenter_AllModules_2026_1_1.zip".to_owned(),
        );
        let html = r#"
            <html>
              <body>
                <a href="https://deaderpool-mccc.com/release/McCmdCenter_AllModules_2026_1_1.zip">Download</a>
                <p>MC Command Center 2026.1.1</p>
              </body>
            </html>
        "#;

        let parsed =
            parse_mccc_latest_html(&profile, "https://deaderpool-mccc.com/downloads.html", html);

        assert_eq!(parsed.latest_version.as_deref(), Some("2026.1.1"));
        assert_eq!(
            parsed.download_url.as_deref(),
            Some("https://deaderpool-mccc.com/release/McCmdCenter_AllModules_2026_1_1.zip")
        );
        assert_eq!(parsed.status, "known");
    }

    #[test]
    fn parses_github_latest_tag_from_redirect_url() {
        let mut profile = load_seed_pack()
            .expect("seed pack")
            .install_catalog
            .guided_profiles
            .into_iter()
            .find(|profile| profile.key == "sims_4_community_library")
            .expect("s4cl");
        profile.official_download_url =
            Some("https://github.com/ColonolNutty/Sims4CommunityLibrary/releases/download/v2.9.0/Sims4CommunityLibrary.zip".to_owned());

        let parsed = parse_github_latest_url(
            &profile,
            "https://github.com/ColonolNutty/Sims4CommunityLibrary/releases/tag/v2.9.0",
        );

        assert_eq!(parsed.latest_version.as_deref(), Some("2.9.0"));
        assert_eq!(
            parsed.download_url.as_deref(),
            Some("https://github.com/ColonolNutty/Sims4CommunityLibrary/releases/download/v2.9.0/Sims4CommunityLibrary.zip")
        );
        assert_eq!(parsed.status, "known");
    }

    #[test]
    fn parses_xml_injector_latest_from_official_page_html() {
        let profile = load_seed_pack()
            .expect("seed pack")
            .install_catalog
            .guided_profiles
            .into_iter()
            .find(|profile| profile.key == "xml_injector")
            .expect("xml injector");
        let html = r#"
            <html>
              <body>
                <p>The current version of the XML Injector is version 4.2, and denoted by the _v4.2 in the filenames.</p>
                <a href="/s/XmlInjector_Script_v42.zip">- DOWNLOAD -</a>
              </body>
            </html>
        "#;

        let parsed = parse_xml_injector_latest_html(
            &profile,
            "https://scumbumbomods.com/xml-injector",
            html,
        );

        assert_eq!(parsed.latest_version.as_deref(), Some("4.2"));
        assert_eq!(
            parsed.download_url.as_deref(),
            Some("https://scumbumbomods.com/s/XmlInjector_Script_v42.zip")
        );
        assert_eq!(parsed.status, "known");
    }

    #[test]
    fn protected_sources_fall_back_to_unknown_without_guessing() {
        let profile = load_seed_pack()
            .expect("seed pack")
            .install_catalog
            .guided_profiles
            .into_iter()
            .find(|profile| profile.key == "xml_injector")
            .expect("xml injector");
        let info = unknown_latest(
            &profile,
            "This official site blocks safe automatic version checks right now.",
        );

        assert_eq!(info.status, "unknown");
        assert!(info.latest_version.is_none());
        assert!(info.note.is_some());
    }

    #[test]
    fn prefers_mod_release_versions_over_game_patch_numbers() {
        let versions = extract_version_candidates_from_value(
            "mc_cmd_version.pyc supports 1.113.277 and release 2026.1.1",
        );

        assert!(versions.iter().any(|value| value == "1.113.277"));
        assert!(versions.iter().any(|value| value == "2026.1.1"));
        assert_eq!(
            extract_version_from_values(&[String::from(
                "mc_cmd_version.pyc supports 1.113.277 and release 2026.1.1",
            )]),
            Some("2026.1.1".to_owned())
        );
    }

    #[test]
    fn different_signatures_do_not_mark_same_labeled_versions_as_current() {
        let status = compare_versions(
            true,
            Some("2026.1.1"),
            Some("installed-signature"),
            Some("2026.1.1"),
            Some("incoming-signature"),
        );

        assert_eq!(status, SpecialVersionStatus::Unknown);
    }

    #[test]
    fn hashed_signature_entries_ignore_outer_wrapper_size_noise() {
        let left = build_signature(&[
            SignatureEntry {
                filename: "Lot51_CoreLibrary.ts4script".to_owned(),
                size: 120,
                hash: Some("abc123".to_owned()),
            },
            SignatureEntry {
                filename: "lot51_core_library.package".to_owned(),
                size: 220,
                hash: Some("def456".to_owned()),
            },
        ]);
        let right = build_signature(&[
            SignatureEntry {
                filename: "Lot51_CoreLibrary.ts4script".to_owned(),
                size: 121,
                hash: Some("abc123".to_owned()),
            },
            SignatureEntry {
                filename: "lot51_core_library.package".to_owned(),
                size: 220,
                hash: Some("def456".to_owned()),
            },
        ]);

        assert_eq!(left, right);
    }
}
