use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
    time::Instant,
};

use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use walkdir::WalkDir;

use crate::{
    core::{
        file_inspector,
        special_mod_versions::{
            self, build_signature, build_version_comparison, extract_version_candidates_from_value,
            extract_version_from_values, load_family_state, load_or_refresh_latest_info,
            save_family_state, SignatureEntry, StoredSpecialModFamilyState,
        },
        validator::{self, ValidationRequest},
    },
    error::{AppError, AppResult},
    models::{
        CatalogSourceInfo, DependencyStatus, DownloadIntakeMode, DownloadQueueLane,
        DownloadRiskLevel, FileInsights, GuidedInstallFileEntry, GuidedInstallPlan,
        LibrarySettings, ReviewPlanAction, ReviewPlanActionKind, SpecialDecisionState,
        SpecialExistingInstallState, SpecialFamilyRole, SpecialLocalPackState, SpecialModDecision,
        SpecialReviewPlan, SpecialVersionStatus,
    },
    seed::{
        DependencyRuleSeed, GuidedInstallProfileSeed, IncompatibilityRuleSeed,
        ReviewOnlyPatternSeed, SeedPack,
    },
};

#[derive(Debug, Clone)]
pub struct DownloadItemAssessment {
    pub intake_mode: DownloadIntakeMode,
    pub risk_level: DownloadRiskLevel,
    pub matched_profile_key: Option<String>,
    pub matched_profile_name: Option<String>,
    pub special_family: Option<String>,
    pub assessment_reasons: Vec<String>,
    pub dependency_summary: Vec<String>,
    pub missing_dependencies: Vec<String>,
    pub inbox_dependencies: Vec<String>,
    pub incompatibility_warnings: Vec<String>,
    pub post_install_notes: Vec<String>,
    pub evidence_summary: Vec<String>,
    pub catalog_source: Option<CatalogSourceInfo>,
    pub existing_install_detected: bool,
    pub guided_install_available: bool,
}

#[derive(Debug, Clone)]
struct DownloadItemRecord {
    id: i64,
    display_name: String,
    source_path: String,
    staging_path: Option<String>,
}

#[derive(Debug, Clone)]
struct ProfileFile {
    file_id: i64,
    filename: String,
    path: String,
    archive_member_path: Option<String>,
    extension: String,
    kind: String,
    subtype: Option<String>,
    creator: Option<String>,
    confidence: f64,
    source_location: String,
    size: i64,
    hash: Option<String>,
    insights: FileInsights,
}

#[derive(Debug, Clone)]
struct ExistingInstallFile {
    file_id: Option<i64>,
    filename: String,
    path: String,
    extension: String,
    kind: String,
    subtype: Option<String>,
    creator: Option<String>,
    size: i64,
    hash: Option<String>,
    insights: FileInsights,
    in_target_folder: bool,
}

#[derive(Debug, Clone)]
struct ExistingInstallLayout {
    target_folder: PathBuf,
    existing_files: Vec<ExistingInstallFile>,
    preserve_files: Vec<ExistingInstallFile>,
    warnings: Vec<String>,
    existing_install_detected: bool,
    safe_to_update: bool,
    repair_plan_available: bool,
}

#[derive(Debug, Clone, Default)]
struct VersionEvidence {
    value: Option<String>,
    source: Option<String>,
    lines: Vec<String>,
}

fn push_version_evidence_line(lines: &mut Vec<String>, line: impl Into<String>) {
    let line = line.into();
    if !lines.contains(&line) {
        lines.push(line);
    }
}

fn limit_version_evidence_lines(lines: &mut Vec<String>) {
    if lines.len() > 2 {
        lines.truncate(2);
    }
}

fn string_contains_version_hint(value: &str, version: &str) -> bool {
    extract_version_candidates_from_value(value)
        .iter()
        .any(|candidate| candidate == version)
}

fn special_insights_need_refresh(extension: &str, insights: &FileInsights) -> bool {
    matches!(extension, ".ts4script" | ".package")
        && (insights.version_hints.is_empty() || insights.family_hints.is_empty())
}

fn merge_insight_values(existing: &[String], fresh: &[String]) -> Vec<String> {
    let mut merged = existing.to_vec();
    for value in fresh {
        if !merged.contains(value) {
            merged.push(value.clone());
        }
    }
    merged
}

fn merge_file_insights(existing: &FileInsights, fresh: &FileInsights) -> FileInsights {
    FileInsights {
        format: fresh.format.clone().or_else(|| existing.format.clone()),
        resource_summary: merge_insight_values(&existing.resource_summary, &fresh.resource_summary),
        script_namespaces: merge_insight_values(
            &existing.script_namespaces,
            &fresh.script_namespaces,
        ),
        embedded_names: merge_insight_values(&existing.embedded_names, &fresh.embedded_names),
        creator_hints: merge_insight_values(&existing.creator_hints, &fresh.creator_hints),
        version_hints: merge_insight_values(&existing.version_hints, &fresh.version_hints),
        family_hints: merge_insight_values(&existing.family_hints, &fresh.family_hints),
    }
}

fn refresh_profile_file_insights_if_needed(
    connection: &Connection,
    seed_pack: &SeedPack,
    file: &mut ProfileFile,
) {
    if !special_insights_need_refresh(&file.extension, &file.insights) {
        return;
    }

    let path = Path::new(&file.path);
    if !path.exists() {
        return;
    }

    let Ok(outcome) = file_inspector::inspect_file(path, &file.extension, seed_pack) else {
        return;
    };
    let merged = merge_file_insights(&file.insights, &outcome.insights);
    if merged.version_hints == file.insights.version_hints
        && merged.family_hints == file.insights.family_hints
        && merged.creator_hints == file.insights.creator_hints
        && merged.script_namespaces == file.insights.script_namespaces
        && merged.embedded_names == file.insights.embedded_names
        && merged.resource_summary == file.insights.resource_summary
        && merged.format == file.insights.format
    {
        return;
    }

    file.insights = merged.clone();
    let _ = connection.execute(
        "UPDATE files SET insights = ?2 WHERE id = ?1",
        params![
            file.file_id,
            serde_json::to_string(&merged).unwrap_or_else(|_| "{}".to_owned())
        ],
    );
}

fn refresh_existing_install_file_insights_if_needed(
    connection: &Connection,
    seed_pack: &SeedPack,
    file: &mut ExistingInstallFile,
) {
    if !special_insights_need_refresh(&file.extension, &file.insights) {
        return;
    }

    let path = Path::new(&file.path);
    if !path.exists() {
        return;
    }

    let Ok(outcome) = file_inspector::inspect_file(path, &file.extension, seed_pack) else {
        return;
    };
    let merged = merge_file_insights(&file.insights, &outcome.insights);
    if merged.version_hints == file.insights.version_hints
        && merged.family_hints == file.insights.family_hints
        && merged.creator_hints == file.insights.creator_hints
        && merged.script_namespaces == file.insights.script_namespaces
        && merged.embedded_names == file.insights.embedded_names
        && merged.resource_summary == file.insights.resource_summary
        && merged.format == file.insights.format
    {
        return;
    }

    file.insights = merged.clone();
    if let Some(file_id) = file.file_id {
        let _ = connection.execute(
            "UPDATE files SET insights = ?2 WHERE id = ?1",
            params![
                file_id,
                serde_json::to_string(&merged).unwrap_or_else(|_| "{}".to_owned())
            ],
        );
    }
}

#[derive(Debug, Clone)]
struct InboxDependencyItem {
    id: i64,
    display_name: String,
    intake_mode: DownloadIntakeMode,
    guided_install_available: bool,
}

#[derive(Debug, Clone)]
struct ProfileEvidence {
    reasons: Vec<String>,
    matched_files: i64,
    matched_script_files: i64,
    matched_package_files: i64,
    unmatched_supported_files: i64,
    required_core_present: bool,
    required_exact_filenames_found: i64,
    name_match: bool,
    text_matches: Vec<String>,
    archive_matches: Vec<String>,
}

impl ProfileEvidence {
    fn has_any_signal(&self) -> bool {
        self.matched_files > 0
            || self.name_match
            || !self.text_matches.is_empty()
            || !self.archive_matches.is_empty()
    }

    fn required_exact_filenames_complete(&self, profile: &GuidedInstallProfileSeed) -> bool {
        self.required_exact_filenames_found as usize >= profile.required_all_filenames.len()
    }

    fn strong_match(&self, profile: &GuidedInstallProfileSeed) -> bool {
        self.required_core_present
            && self.matched_files >= profile.minimum_profile_files as i64
            && self.matched_script_files >= profile.minimum_script_files as i64
            && self.required_exact_filenames_complete(profile)
    }

    fn local_pack_state(&self, profile: &GuidedInstallProfileSeed) -> SpecialLocalPackState {
        if self.strong_match(profile) && self.unmatched_supported_files == 0 {
            SpecialLocalPackState::Complete
        } else if self.strong_match(profile) && self.unmatched_supported_files > 0 {
            SpecialLocalPackState::Mixed
        } else if self.has_any_signal() {
            SpecialLocalPackState::Partial
        } else {
            SpecialLocalPackState::Unknown
        }
    }

    fn score(&self, profile: &GuidedInstallProfileSeed) -> i64 {
        (self.matched_files * 100)
            + (self.matched_script_files * 30)
            + (self.required_exact_filenames_found * 40)
            + if self.required_core_present { 120 } else { 0 }
            + if self.name_match { 20 } else { 0 }
            + (self.text_matches.len() as i64 * 5)
            + (self.archive_matches.len() as i64 * 5)
            + if self.strong_match(profile) { 250 } else { 0 }
            - (self.unmatched_supported_files * 30)
    }
}

#[derive(Debug, Clone)]
struct GuidedCandidate {
    profile: GuidedInstallProfileSeed,
    evidence: ProfileEvidence,
}

#[derive(Debug, Clone)]
struct EvaluationResult {
    assessment: DownloadItemAssessment,
    matched_profile: Option<GuidedInstallProfileSeed>,
    matched_pattern: Option<ReviewOnlyPatternSeed>,
    matched_evidence: Option<ProfileEvidence>,
    dependencies: Vec<DependencyStatus>,
    existing_layout_findings: Vec<String>,
    explanation: String,
    recommended_next_step: String,
}

#[derive(Debug, Clone)]
struct FamilySiblingRecord {
    id: i64,
    display_name: String,
    intake_mode: DownloadIntakeMode,
    guided_install_available: bool,
    active_file_count: i64,
    review_file_count: i64,
    updated_at: String,
    status: String,
}

#[derive(Default)]
pub struct SpecialDecisionContext {
    layout_cache: HashMap<String, ExistingInstallLayout>,
    sibling_cache: HashMap<String, Vec<FamilySiblingRecord>>,
    family_state_cache: HashMap<String, StoredSpecialModFamilyState>,
    item_cache: HashMap<i64, DownloadItemRecord>,
    active_profile_files_cache: HashMap<i64, Vec<ProfileFile>>,
    all_profile_files_cache: HashMap<i64, Vec<ProfileFile>>,
    evaluation_cache: HashMap<i64, EvaluationResult>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpecialDecisionDetailLevel {
    Queue,
    Full,
}

const SLOW_INSTALL_PROFILE_LOG_THRESHOLD_MS: u128 = 40;

fn log_slow_install_profile_step(
    operation: &str,
    started_at: Instant,
    detail: impl FnOnce() -> String,
) {
    #[cfg(debug_assertions)]
    {
        let elapsed_ms = started_at.elapsed().as_millis();
        if elapsed_ms >= SLOW_INSTALL_PROFILE_LOG_THRESHOLD_MS {
            eprintln!("[perf] {operation} took {elapsed_ms}ms {}", detail());
        }
    }
}

fn load_item_with_staging_cached(
    connection: &Connection,
    item_id: i64,
    context: &mut SpecialDecisionContext,
) -> AppResult<Option<DownloadItemRecord>> {
    if let Some(item) = context.item_cache.get(&item_id) {
        return Ok(Some(item.clone()));
    }

    let item = load_item_with_staging(connection, item_id)?;
    if let Some(item) = item.as_ref() {
        context.item_cache.insert(item_id, item.clone());
    }
    Ok(item)
}

fn load_profile_files_cached(
    connection: &Connection,
    seed_pack: &SeedPack,
    item_id: i64,
    active_only: bool,
    context: &mut SpecialDecisionContext,
) -> AppResult<Vec<ProfileFile>> {
    let cache = if active_only {
        &mut context.active_profile_files_cache
    } else {
        &mut context.all_profile_files_cache
    };
    if let Some(files) = cache.get(&item_id) {
        return Ok(files.clone());
    }

    let files = load_profile_files(connection, seed_pack, item_id, active_only)?;
    cache.insert(item_id, files.clone());
    Ok(files)
}

fn evaluate_download_item_cached(
    connection: &Connection,
    settings: &LibrarySettings,
    seed_pack: &SeedPack,
    item_id: i64,
    context: &mut SpecialDecisionContext,
) -> AppResult<EvaluationResult> {
    if let Some(result) = context.evaluation_cache.get(&item_id) {
        return Ok(result.clone());
    }

    let started_at = Instant::now();
    let result = evaluate_download_item(connection, settings, seed_pack, item_id)?;
    log_slow_install_profile_step("evaluate_download_item", started_at, || {
        format!(
            "for item {} mode={} profile={}",
            item_id,
            intake_mode_label(&result.assessment.intake_mode),
            result
                .matched_profile
                .as_ref()
                .map(|profile| profile.key.as_str())
                .unwrap_or("none")
        )
    });
    context.evaluation_cache.insert(item_id, result.clone());
    Ok(result)
}

pub fn assess_download_item(
    connection: &Connection,
    settings: &LibrarySettings,
    seed_pack: &SeedPack,
    item_id: i64,
) -> AppResult<DownloadItemAssessment> {
    evaluate_download_item(connection, settings, seed_pack, item_id).map(|result| result.assessment)
}

pub fn store_download_item_assessment(
    connection: &Connection,
    item_id: i64,
    assessment: &DownloadItemAssessment,
) -> AppResult<()> {
    connection.execute(
        "UPDATE download_items
         SET intake_mode = ?2,
             risk_level = ?3,
             matched_profile_key = ?4,
             matched_profile_name = ?5,
             special_family = ?6,
             assessment_reasons = ?7,
             dependency_summary = ?8,
             missing_dependencies = ?9,
             inbox_dependencies = ?10,
             incompatibility_warnings = ?11,
             post_install_notes = ?12,
             evidence_summary = ?13,
             catalog_source_url = ?14,
             catalog_download_url = ?15,
             latest_check_url = ?16,
             latest_check_strategy = ?17,
             catalog_reference_source = ?18,
             catalog_reviewed_at = ?19,
             existing_install_detected = ?20,
             guided_install_available = ?21,
             updated_at = CURRENT_TIMESTAMP
         WHERE id = ?1",
        params![
            item_id,
            intake_mode_label(&assessment.intake_mode),
            risk_level_label(&assessment.risk_level),
            assessment.matched_profile_key,
            assessment.matched_profile_name,
            assessment.special_family,
            serde_json::to_string(&assessment.assessment_reasons)?,
            serde_json::to_string(&assessment.dependency_summary)?,
            serde_json::to_string(&assessment.missing_dependencies)?,
            serde_json::to_string(&assessment.inbox_dependencies)?,
            serde_json::to_string(&assessment.incompatibility_warnings)?,
            serde_json::to_string(&assessment.post_install_notes)?,
            serde_json::to_string(&assessment.evidence_summary)?,
            assessment
                .catalog_source
                .as_ref()
                .and_then(|source| source.official_source_url.clone()),
            assessment
                .catalog_source
                .as_ref()
                .and_then(|source| source.official_download_url.clone()),
            assessment
                .catalog_source
                .as_ref()
                .and_then(|source| source.latest_check_url.clone()),
            assessment
                .catalog_source
                .as_ref()
                .and_then(|source| source.latest_check_strategy.clone()),
            serde_json::to_string(
                &assessment
                    .catalog_source
                    .as_ref()
                    .map(|source| source.reference_source.clone())
                    .unwrap_or_default(),
            )?,
            assessment
                .catalog_source
                .as_ref()
                .and_then(|source| source.reviewed_at.clone()),
            if assessment.existing_install_detected {
                1_i64
            } else {
                0_i64
            },
            if assessment.guided_install_available {
                1_i64
            } else {
                0_i64
            },
        ],
    )?;
    Ok(())
}

pub fn build_guided_plan(
    connection: &Connection,
    settings: &LibrarySettings,
    seed_pack: &SeedPack,
    item_id: i64,
) -> AppResult<Option<GuidedInstallPlan>> {
    let mut context = SpecialDecisionContext::default();
    build_guided_plan_cached(connection, settings, seed_pack, item_id, &mut context)
}

pub fn build_guided_plan_cached(
    connection: &Connection,
    settings: &LibrarySettings,
    seed_pack: &SeedPack,
    item_id: i64,
    context: &mut SpecialDecisionContext,
) -> AppResult<Option<GuidedInstallPlan>> {
    build_guided_plan_internal(connection, settings, seed_pack, item_id, false, context)
}

pub fn build_repair_guided_plan(
    connection: &Connection,
    settings: &LibrarySettings,
    seed_pack: &SeedPack,
    item_id: i64,
) -> AppResult<Option<GuidedInstallPlan>> {
    let mut context = SpecialDecisionContext::default();
    build_repair_guided_plan_cached(connection, settings, seed_pack, item_id, &mut context)
}

pub fn build_repair_guided_plan_cached(
    connection: &Connection,
    settings: &LibrarySettings,
    seed_pack: &SeedPack,
    item_id: i64,
    context: &mut SpecialDecisionContext,
) -> AppResult<Option<GuidedInstallPlan>> {
    build_guided_plan_internal(connection, settings, seed_pack, item_id, true, context)
}

fn build_guided_plan_internal(
    connection: &Connection,
    settings: &LibrarySettings,
    seed_pack: &SeedPack,
    item_id: i64,
    allow_repair_layout: bool,
    context: &mut SpecialDecisionContext,
) -> AppResult<Option<GuidedInstallPlan>> {
    let evaluation =
        evaluate_download_item_cached(connection, settings, seed_pack, item_id, context)?;
    if evaluation.assessment.intake_mode != DownloadIntakeMode::Guided && !allow_repair_layout {
        return Ok(None);
    }

    let Some(profile) = evaluation.matched_profile.clone() else {
        return Ok(None);
    };
    let Some(item) = load_item_with_staging_cached(connection, item_id, context)? else {
        return Ok(None);
    };
    let active_files = load_profile_files_cached(connection, seed_pack, item_id, true, context)?;
    let layout = detect_existing_layout_cached(connection, settings, seed_pack, &profile, context)?;
    let can_repair_layout = allow_repair_layout
        && layout.repair_plan_available
        && evaluation.assessment.missing_dependencies.is_empty()
        && evaluation.assessment.inbox_dependencies.is_empty()
        && evaluation.assessment.incompatibility_warnings.is_empty();

    if !layout.safe_to_update && !can_repair_layout {
        return Ok(None);
    }

    let Some(mods_root) = settings
        .mods_path
        .as_ref()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
    else {
        return Err(AppError::Message(
            "Set a Mods folder before using guided special installs.".to_owned(),
        ));
    };

    let incoming = active_files
        .iter()
        .filter(|file| is_profile_content_file(file, &profile))
        .collect::<Vec<_>>();

    let relative_target_folder = layout
        .target_folder
        .strip_prefix(&mods_root)
        .map(|path| path.to_path_buf())
        .unwrap_or_else(|_| PathBuf::from(&profile.install_folder_name));

    let mut install_files = Vec::new();
    let mut review_files = Vec::new();
    let mut warnings = layout.warnings.clone();
    let mut evidence = evaluation.assessment.evidence_summary.clone();
    let mut reserved_targets = HashSet::new();

    for file in incoming {
        let validation_kind = guided_validation_kind(file, &profile);
        let validation = validator::validate_suggestion(
            connection,
            settings,
            &ValidationRequest {
                file_id: file.file_id,
                filename: file.filename.clone(),
                extension: file.extension.clone(),
                kind: validation_kind.clone(),
                subtype: file.subtype.clone(),
                creator: file.creator.clone().or_else(|| profile.creator.clone()),
                bundle_name: None,
                source_location: file.source_location.clone(),
                confidence: file.confidence.max(0.92),
                suggested_relative_path: relative_target_folder
                    .join(&file.filename)
                    .to_string_lossy()
                    .replace('\\', "/"),
                guided_install: true,
                allow_existing_target: layout.existing_install_detected,
            },
            &reserved_targets,
        )?;

        if let Some(path) = validation.final_absolute_path.clone() {
            reserved_targets.insert(path);
        }

        let entry = GuidedInstallFileEntry {
            file_id: Some(file.file_id),
            filename: file.filename.clone(),
            current_path: file.path.clone(),
            target_path: validation.final_absolute_path.clone(),
            archive_member_path: file.archive_member_path.clone(),
            kind: validation_kind,
            subtype: file.subtype.clone(),
            creator: file.creator.clone().or_else(|| profile.creator.clone()),
            notes: validation.notes.clone(),
        };

        if validation.review_required || validation.final_absolute_path.is_none() {
            review_files.push(entry);
        } else {
            install_files.push(entry);
        }
    }

    let replace_files = layout
        .existing_files
        .iter()
        .filter(|file| matches!(file.extension.as_str(), ".package" | ".ts4script"))
        .map(|file| GuidedInstallFileEntry {
            file_id: file.file_id,
            filename: file.filename.clone(),
            current_path: file.path.clone(),
            target_path: Some(
                layout
                    .target_folder
                    .join(&file.filename)
                    .to_string_lossy()
                    .to_string(),
            ),
            archive_member_path: None,
            kind: file.kind.clone(),
            subtype: file.subtype.clone(),
            creator: file.creator.clone().or_else(|| profile.creator.clone()),
            notes: vec![format!(
                "Old {} package or script file that will be replaced.",
                profile.display_name
            )],
        })
        .collect::<Vec<_>>();

    let preserve_files = layout
        .preserve_files
        .iter()
        .map(|file| GuidedInstallFileEntry {
            file_id: file.file_id,
            filename: file.filename.clone(),
            current_path: file.path.clone(),
            target_path: Some(if file.in_target_folder {
                file.path.clone()
            } else {
                layout
                    .target_folder
                    .join(&file.filename)
                    .to_string_lossy()
                    .to_string()
            }),
            archive_member_path: None,
            kind: "Config".to_owned(),
            subtype: None,
            creator: file.creator.clone().or_else(|| profile.creator.clone()),
            notes: vec![if file.in_target_folder {
                "Settings or sidecar file that will stay in place during the update.".to_owned()
            } else {
                "Settings or sidecar file that SimSuite will keep safe during the update."
                    .to_owned()
            }],
        })
        .collect::<Vec<_>>();

    for file in active_files
        .iter()
        .filter(|file| !is_profile_content_file(file, &profile))
    {
        review_files.push(GuidedInstallFileEntry {
            file_id: Some(file.file_id),
            filename: file.filename.clone(),
            current_path: file.path.clone(),
            target_path: None,
            archive_member_path: file.archive_member_path.clone(),
            kind: file.kind.clone(),
            subtype: file.subtype.clone(),
            creator: file.creator.clone(),
            notes: vec![format!(
                "This file does not fit the guided {} install set.",
                profile.display_name
            )],
        });
    }

    if !preserve_files.is_empty() {
        warnings.push(format!(
            "Existing {} settings or sidecar files will be kept.",
            profile.display_name
        ));
    }
    if !replace_files.is_empty() {
        evidence.push(format!(
            "Existing {} package or script files were found and will be replaced.",
            profile.display_name
        ));
    }
    if can_repair_layout {
        warnings.push(format!(
            "SimSuite will clear the older {} setup out of the way before it installs this update.",
            profile.display_name
        ));
    }
    if !item.display_name.trim().is_empty() {
        evidence.push(format!("Source archive: {}", item.display_name));
    }

    let apply_ready = review_files.is_empty();

    Ok(Some(GuidedInstallPlan {
        item_id: item.id,
        profile_key: profile.key.clone(),
        profile_name: profile.display_name.clone(),
        special_family: Some(profile.family.clone()),
        install_target_folder: layout.target_folder.to_string_lossy().to_string(),
        install_files,
        replace_files,
        preserve_files,
        review_files,
        dependencies: evaluation.dependencies,
        incompatibility_warnings: evaluation.assessment.incompatibility_warnings.clone(),
        post_install_notes: evaluation.assessment.post_install_notes.clone(),
        existing_layout_findings: layout.warnings,
        warnings,
        explanation: profile.help_summary.clone(),
        evidence,
        catalog_source: evaluation.assessment.catalog_source.clone(),
        existing_install_detected: layout.existing_install_detected,
        apply_ready,
    }))
}

pub fn build_review_plan(
    connection: &Connection,
    settings: &LibrarySettings,
    seed_pack: &SeedPack,
    item_id: i64,
) -> AppResult<Option<SpecialReviewPlan>> {
    let mut context = SpecialDecisionContext::default();
    build_review_plan_cached(connection, settings, seed_pack, item_id, &mut context)
}

pub fn build_review_plan_cached(
    connection: &Connection,
    settings: &LibrarySettings,
    seed_pack: &SeedPack,
    item_id: i64,
    context: &mut SpecialDecisionContext,
) -> AppResult<Option<SpecialReviewPlan>> {
    let evaluation =
        evaluate_download_item_cached(connection, settings, seed_pack, item_id, context)?;
    if evaluation.assessment.intake_mode == DownloadIntakeMode::Standard {
        return Ok(None);
    }

    let guided_plan = if evaluation.assessment.intake_mode == DownloadIntakeMode::Guided {
        build_guided_plan_cached(connection, settings, seed_pack, item_id, context)?
    } else {
        None
    };
    let special_decision = if evaluation.matched_profile.is_some() {
        build_special_mod_decision_cached(
            connection,
            settings,
            seed_pack,
            item_id,
            context,
            false,
            SpecialDecisionDetailLevel::Full,
        )?
    } else {
        None
    };

    if evaluation.assessment.intake_mode == DownloadIntakeMode::Guided
        && guided_plan.as_ref().is_none_or(|plan| plan.apply_ready)
        && special_decision
            .as_ref()
            .is_none_or(|decision| decision.apply_ready)
    {
        return Ok(None);
    }

    let files = load_profile_files_cached(connection, seed_pack, item_id, true, context)?;
    let repair_guided_plan =
        build_repair_guided_plan_cached(connection, settings, seed_pack, item_id, context)?;
    let repair_layout = evaluation
        .matched_profile
        .as_ref()
        .map(|profile| {
            detect_existing_layout_cached(connection, settings, seed_pack, profile, context)
        })
        .transpose()?;
    let review_files = guided_plan
        .as_ref()
        .map(|plan| plan.review_files.clone())
        .filter(|files| !files.is_empty())
        .unwrap_or_else(|| {
            files
                .iter()
                .map(|file| GuidedInstallFileEntry {
                    file_id: Some(file.file_id),
                    filename: file.filename.clone(),
                    current_path: file.path.clone(),
                    target_path: None,
                    archive_member_path: file.archive_member_path.clone(),
                    kind: file.kind.clone(),
                    subtype: file.subtype.clone(),
                    creator: file.creator.clone(),
                    notes: Vec::new(),
                })
                .collect::<Vec<_>>()
        });

    let (
        repair_plan_available,
        repair_action_label,
        repair_reason,
        repair_target_folder,
        repair_move_files,
        repair_replace_files,
        repair_keep_files,
        repair_warnings,
        repair_can_continue_install,
    ) = if let (Some(profile), Some(layout)) =
        (evaluation.matched_profile.as_ref(), repair_layout.as_ref())
    {
        let scattered_existing = layout
            .existing_files
            .iter()
            .filter(|file| !file.in_target_folder)
            .map(|file| GuidedInstallFileEntry {
                file_id: file.file_id,
                filename: file.filename.clone(),
                current_path: file.path.clone(),
                target_path: Some(
                    layout
                        .target_folder
                        .join(&file.filename)
                        .to_string_lossy()
                        .to_string(),
                ),
                archive_member_path: None,
                kind: file.kind.clone(),
                subtype: file.subtype.clone(),
                creator: file.creator.clone().or_else(|| profile.creator.clone()),
                notes: vec!["Old special-mod file that SimSuite will clean up before the new copy is installed.".to_owned()],
            })
            .collect::<Vec<_>>();
        let repair_keep_files = layout
            .preserve_files
            .iter()
            .map(|file| GuidedInstallFileEntry {
                file_id: file.file_id,
                filename: file.filename.clone(),
                current_path: file.path.clone(),
                target_path: Some(if file.in_target_folder {
                    file.path.clone()
                } else {
                    layout
                        .target_folder
                        .join(&file.filename)
                        .to_string_lossy()
                        .to_string()
                }),
                archive_member_path: None,
                kind: "Config".to_owned(),
                subtype: None,
                creator: file.creator.clone().or_else(|| profile.creator.clone()),
                notes: vec!["Settings or sidecar file that SimSuite will keep.".to_owned()],
            })
            .collect::<Vec<_>>();
        let can_continue_install = repair_guided_plan
            .as_ref()
            .map(|plan| plan.apply_ready)
            .unwrap_or(false);
        let repair_replace_files = repair_guided_plan
            .as_ref()
            .map(|plan| plan.install_files.clone())
            .unwrap_or_default();

        (
            layout.repair_plan_available,
            if layout.repair_plan_available {
                Some(format!("Fix old {} setup", profile.display_name))
            } else {
                None
            },
            if layout.repair_plan_available {
                Some(format!(
                    "SimSuite can clear the older {} files out of the way, keep the settings safe, and then continue with the update.",
                    profile.display_name
                ))
            } else {
                None
            },
            if layout.repair_plan_available {
                Some(layout.target_folder.to_string_lossy().to_string())
            } else {
                None
            },
            scattered_existing,
            repair_replace_files,
            repair_keep_files,
            layout.warnings.clone(),
            can_continue_install,
        )
    } else {
        (
            false,
            None,
            None,
            None,
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            false,
        )
    };

    let available_actions = special_decision
        .as_ref()
        .map(|decision| decision.available_actions.clone())
        .unwrap_or_else(|| {
            build_available_review_actions(seed_pack, &evaluation, &files, repair_layout.as_ref())
        });
    let recommended_next_step = special_decision
        .as_ref()
        .map(|decision| decision.recommended_next_step.clone())
        .unwrap_or_else(|| {
            if evaluation.assessment.intake_mode == DownloadIntakeMode::Guided {
                available_actions
                    .iter()
                    .max_by_key(|action| action.priority)
                    .map(|action| action.description.clone())
                    .unwrap_or_else(|| {
                        "Review the held files and remove anything that does not fit the safe guided install."
                            .to_owned()
                    })
            } else {
                evaluation.recommended_next_step.clone()
            }
        });
    let explanation = special_decision
        .as_ref()
        .map(|decision| decision.explanation.clone())
        .unwrap_or_else(|| {
            if evaluation.assessment.intake_mode == DownloadIntakeMode::Guided {
                let profile_name = evaluation
                    .assessment
                    .matched_profile_name
                    .clone()
                    .unwrap_or_else(|| "special mod".to_owned());
                format!(
                    "SimSuite recognized this as {}, but the guided install still has file checks to clear before anything can move.",
                    profile_name
                )
            } else {
                evaluation.explanation.clone()
            }
        });

    Ok(Some(SpecialReviewPlan {
        item_id,
        mode: evaluation.assessment.intake_mode.clone(),
        profile_key: evaluation.assessment.matched_profile_key.clone(),
        profile_name: evaluation.assessment.matched_profile_name.clone(),
        special_family: evaluation.assessment.special_family.clone(),
        explanation,
        recommended_next_step,
        dependencies: evaluation.dependencies,
        incompatibility_warnings: evaluation.assessment.incompatibility_warnings.clone(),
        review_files,
        evidence: evaluation.assessment.assessment_reasons.clone(),
        existing_layout_findings: evaluation.existing_layout_findings,
        post_install_notes: evaluation.assessment.post_install_notes.clone(),
        catalog_source: evaluation.assessment.catalog_source.clone(),
        available_actions,
        repair_plan_available,
        repair_action_label,
        repair_reason,
        repair_target_folder,
        repair_move_files,
        repair_replace_files,
        repair_keep_files,
        repair_warnings,
        repair_can_continue_install,
    }))
}

#[cfg(test)]
pub fn build_special_mod_decision(
    connection: &Connection,
    settings: &LibrarySettings,
    seed_pack: &SeedPack,
    item_id: i64,
) -> AppResult<Option<SpecialModDecision>> {
    let mut context = SpecialDecisionContext::default();
    build_special_mod_decision_cached(
        connection,
        settings,
        seed_pack,
        item_id,
        &mut context,
        false,
        SpecialDecisionDetailLevel::Full,
    )
}

pub fn build_special_mod_decision_cached(
    connection: &Connection,
    settings: &LibrarySettings,
    seed_pack: &SeedPack,
    item_id: i64,
    context: &mut SpecialDecisionContext,
    allow_network_latest: bool,
    detail_level: SpecialDecisionDetailLevel,
) -> AppResult<Option<SpecialModDecision>> {
    let include_full_details = detail_level == SpecialDecisionDetailLevel::Full;
    let evaluation =
        evaluate_download_item_cached(connection, settings, seed_pack, item_id, context)?;
    let Some(profile) = evaluation.matched_profile.clone() else {
        return Ok(None);
    };
    let item = load_item_with_staging_cached(connection, item_id, context)?
        .unwrap_or_else(empty_item_record);
    let evidence = evaluation
        .matched_evidence
        .clone()
        .unwrap_or_else(|| collect_profile_evidence(&profile, &empty_item_record(), &[], &[], &[]));
    let files = load_profile_files_cached(connection, seed_pack, item_id, true, context)?;
    let layout = detect_existing_layout_cached(connection, settings, seed_pack, &profile, context)?;
    let family_key = format!("special:{}", profile.key);
    let siblings = load_family_siblings_cached(connection, item_id, &profile.key, context)?;
    let primary_family = select_primary_family_item(item_id, &siblings);
    let sibling_item_ids = siblings
        .iter()
        .map(|sibling| sibling.id)
        .filter(|sibling_id| *sibling_id != item_id)
        .collect::<Vec<_>>();
    let primary_family_item_id = primary_family.as_ref().map(|item| item.id);
    let primary_family_item_name = primary_family
        .as_ref()
        .map(|item| item.display_name.clone());
    let primary_family_item_status = primary_family.as_ref().map(|item| item.status.as_str());
    let family_role = match primary_family_item_id {
        Some(primary_id) if primary_id != item_id => SpecialFamilyRole::Superseded,
        Some(_) | None => SpecialFamilyRole::Primary,
    };
    let superseded_by_installed_family = family_role == SpecialFamilyRole::Superseded
        && primary_family_item_status == Some("applied");
    let local_pack_state = evidence.local_pack_state(&profile);
    let existing_install_state = existing_install_state_from_layout(&layout);
    let install_path = layout
        .existing_install_detected
        .then(|| layout.target_folder.to_string_lossy().to_string());
    let mut family_state = load_family_state_cached(
        connection,
        context,
        &profile,
        &existing_install_state,
        install_path.clone(),
    )?;
    let incoming_version_evidence =
        incoming_version_for_profile(&profile, &item.display_name, &files);
    let incoming_version = incoming_version_evidence.value.clone();
    let incoming_signature = incoming_signature_for_profile(&profile, &files);
    let mut installed_version_evidence = installed_version_for_profile(&profile, &layout);
    let installed_version = if installed_version_evidence.value.is_some() {
        installed_version_evidence.value.clone()
    } else {
        let saved_version = family_state.installed.installed_version.clone();
        if saved_version.is_some() && installed_version_evidence.source.is_none() {
            installed_version_evidence.source = Some("saved family state".to_owned());
            if let Some(version) = saved_version.as_deref() {
                push_version_evidence_line(
                    &mut installed_version_evidence.lines,
                    format!("The last saved family record still shows {}.", version),
                );
            }
        }
        saved_version
    };
    let installed_signature = installed_signature_for_profile(&profile, &layout)
        .or_else(|| family_state.installed.installed_signature.clone());

    family_state.installed.install_state = existing_install_state.clone();
    family_state.installed.install_path = install_path;
    family_state.installed.installed_version = installed_version;
    family_state.installed.installed_signature = installed_signature;
    family_state.installed.checked_at = Some(Utc::now().to_rfc3339());

    let official_latest = if include_full_details {
        let latest = load_or_refresh_latest_info(connection, &profile, allow_network_latest)?;
        family_state.latest = latest.clone();
        latest
    } else {
        None
    };
    save_family_state(connection, &family_state)?;
    context
        .family_state_cache
        .insert(profile.key.clone(), family_state.clone());

    let version_comparison = build_version_comparison(
        &family_state.installed,
        incoming_version.clone(),
        incoming_signature.clone(),
    );
    let version_status = version_comparison.version_status.clone();
    let same_version = version_comparison.same_version;
    let apply_ready =
        evaluation.assessment.guided_install_available && family_role == SpecialFamilyRole::Primary;

    let version_blocks_update = matches!(
        version_status,
        SpecialVersionStatus::SameVersion | SpecialVersionStatus::IncomingOlder
    );
    let mut available_actions = if apply_ready || version_blocks_update {
        Vec::new()
    } else {
        build_available_review_actions(seed_pack, &evaluation, &files, Some(&layout))
    };

    if family_role == SpecialFamilyRole::Superseded && !superseded_by_installed_family {
        if let (Some(primary_id), Some(primary_name)) =
            (primary_family_item_id, primary_family_item_name.clone())
        {
            available_actions.insert(
                0,
                ReviewPlanAction {
                    kind: ReviewPlanActionKind::OpenRelatedItem,
                    label: format!("Use {}", primary_name),
                    description: format!(
                        "Open the fuller {} Inbox item first so SimSuite can use the best local pack you already downloaded.",
                        profile.display_name
                    ),
                    priority: 110,
                    related_item_id: Some(primary_id),
                    related_item_name: Some(primary_name),
                    url: None,
                },
            );
        }
    }

    available_actions.sort_by(|left, right| right.priority.cmp(&left.priority));
    let primary_action = if superseded_by_installed_family
        || (version_blocks_update && family_role == SpecialFamilyRole::Primary)
    {
        None
    } else {
        available_actions.first().cloned()
    };
    let state = if apply_ready {
        SpecialDecisionState::GuidedReady
    } else if let Some(action) = primary_action.as_ref() {
        special_decision_state_for_action(&action.kind)
    } else {
        SpecialDecisionState::ReviewManually
    };
    let queue_lane = if superseded_by_installed_family
        || (version_blocks_update && family_role == SpecialFamilyRole::Primary)
    {
        DownloadQueueLane::Done
    } else {
        special_decision_queue_lane(&state, &evaluation.assessment.intake_mode)
    };
    let queue_summary = build_special_queue_summary(
        &profile,
        &state,
        &local_pack_state,
        family_role.clone(),
        primary_family_item_name.clone(),
        superseded_by_installed_family,
        &version_status,
        &evaluation,
    );
    let explanation = if include_full_details && superseded_by_installed_family {
        format!(
            "SimSuite already used a fuller {} pack from this family, so this leftover download no longer needs to lead the install.",
            profile.display_name
        )
    } else if include_full_details && family_role == SpecialFamilyRole::Superseded {
        format!(
            "SimSuite found more than one {} download in the Inbox and picked the fuller local pack to lead with.",
            profile.display_name
        )
    } else if include_full_details && same_version {
        format!(
            "This {} version already matches the copy that is installed.",
            profile.display_name
        )
    } else if include_full_details && version_status == SpecialVersionStatus::IncomingOlder {
        format!(
            "This {} download looks older than the copy that is already installed.",
            profile.display_name
        )
    } else if include_full_details && apply_ready {
        profile.help_summary.clone()
    } else if include_full_details {
        evaluation.explanation.clone()
    } else {
        queue_summary.clone()
    };
    let recommended_next_step = if include_full_details && superseded_by_installed_family {
        format!(
            "Ignore this leftover {} download unless you want to keep the extra archive for reference.",
            profile.display_name
        )
    } else if include_full_details && same_version {
        format!(
            "{} is already current. Reinstall only if you want to replace a damaged copy.",
            profile.display_name
        )
    } else if include_full_details && version_status == SpecialVersionStatus::IncomingOlder {
        format!(
            "Ignore this older {} download unless you want to roll back on purpose.",
            profile.display_name
        )
    } else if include_full_details {
        primary_action
            .as_ref()
            .map(|action| action.description.clone())
            .unwrap_or_else(|| evaluation.recommended_next_step.clone())
    } else {
        primary_action
            .as_ref()
            .map(|action| action.label.clone())
            .unwrap_or_else(|| queue_summary.clone())
    };
    let comparison_source = if include_full_details {
        if incoming_signature.is_some()
            && family_state.installed.installed_signature.is_some()
            && version_status == SpecialVersionStatus::SameVersion
            && (incoming_version.is_none() || family_state.installed.installed_version.is_none())
        {
            Some("file signature".to_owned())
        } else if incoming_version_evidence.source.as_deref() == Some("inside mod")
            || installed_version_evidence.source.as_deref() == Some("inside mod")
        {
            Some("inside mod".to_owned())
        } else if incoming_version_evidence.source.as_deref() == Some("saved family state")
            || installed_version_evidence.source.as_deref() == Some("saved family state")
        {
            Some("saved family state".to_owned())
        } else if incoming_version_evidence.source.is_some()
            || installed_version_evidence.source.is_some()
        {
            Some("local file names".to_owned())
        } else {
            None
        }
    } else {
        None
    };
    let comparison_evidence = if include_full_details {
        build_comparison_evidence_lines(
            incoming_version.as_deref(),
            family_state.installed.installed_version.as_deref(),
            incoming_signature.as_deref(),
            family_state.installed.installed_signature.as_deref(),
            &version_status,
            comparison_source.as_deref(),
        )
    } else {
        Vec::new()
    };
    let (incoming_version_source, incoming_version_evidence_lines) = if include_full_details {
        (
            incoming_version_evidence.source,
            incoming_version_evidence.lines,
        )
    } else {
        (None, Vec::new())
    };
    let (installed_version_source, installed_version_evidence_lines) = if include_full_details {
        (
            installed_version_evidence.source,
            installed_version_evidence.lines,
        )
    } else {
        (None, Vec::new())
    };
    if !include_full_details {
        available_actions.clear();
    }

    Ok(Some(SpecialModDecision {
        item_id,
        profile_key: profile.key.clone(),
        profile_name: profile.display_name.clone(),
        special_family: profile.family.clone(),
        state,
        local_pack_state,
        existing_install_state,
        family_role,
        family_key,
        primary_family_item_id,
        primary_family_item_name,
        sibling_item_ids,
        queue_lane,
        queue_summary,
        explanation,
        recommended_next_step,
        incoming_version,
        incoming_signature,
        incoming_version_source,
        incoming_version_evidence: incoming_version_evidence_lines,
        installed_version_source,
        installed_version_evidence: installed_version_evidence_lines,
        comparison_source,
        comparison_evidence,
        version_status,
        same_version,
        official_latest,
        apply_ready,
        available_actions,
        primary_action: if include_full_details {
            primary_action
        } else {
            None
        },
        installed_state: family_state.installed,
    }))
}

pub fn collect_supported_subset_file_ids(
    connection: &Connection,
    seed_pack: &SeedPack,
    item_id: i64,
    profile_key: &str,
) -> AppResult<Option<(Vec<i64>, Vec<i64>)>> {
    let Some(profile) = seed_pack
        .install_catalog
        .guided_profiles
        .iter()
        .find(|profile| profile.key == profile_key)
    else {
        return Ok(None);
    };

    let files = load_profile_files(connection, seed_pack, item_id, true)?;
    if !has_supported_subset_to_separate(&files, profile) {
        return Ok(None);
    }

    let supported_ids = files
        .iter()
        .filter(|file| is_profile_content_file(file, profile))
        .map(|file| file.file_id)
        .collect::<Vec<_>>();
    let leftover_ids = files
        .iter()
        .filter(|file| !is_profile_content_file(file, profile))
        .map(|file| file.file_id)
        .collect::<Vec<_>>();

    if supported_ids.is_empty() || leftover_ids.is_empty() {
        return Ok(None);
    }

    Ok(Some((supported_ids, leftover_ids)))
}

pub fn reconcile_special_mod_family(
    connection: &Connection,
    settings: &LibrarySettings,
    seed_pack: &SeedPack,
    profile_key: &str,
    applied_item_id: i64,
) -> AppResult<Vec<i64>> {
    let Some(profile) = seed_pack
        .install_catalog
        .guided_profiles
        .iter()
        .find(|profile| profile.key == profile_key)
        .cloned()
    else {
        return Ok(vec![applied_item_id]);
    };

    let mut context = SpecialDecisionContext::default();
    let layout =
        detect_existing_layout_cached(connection, settings, seed_pack, &profile, &mut context)?;
    let existing_install_state = existing_install_state_from_layout(&layout);
    let install_path = layout
        .existing_install_detected
        .then(|| layout.target_folder.to_string_lossy().to_string());
    let item =
        load_item_with_staging(connection, applied_item_id)?.unwrap_or_else(empty_item_record);
    let files = load_profile_files(connection, seed_pack, applied_item_id, false)?;
    let mut family_state = load_family_state_cached(
        connection,
        &mut context,
        &profile,
        &existing_install_state,
        install_path.clone(),
    )?;

    let incoming_version = incoming_version_for_profile(&profile, &item.display_name, &files).value;
    let incoming_signature = incoming_signature_for_profile(&profile, &files);
    family_state.installed.install_state = existing_install_state;
    family_state.installed.install_path = install_path;
    family_state.installed.installed_version = incoming_version.clone().or_else(|| {
        installed_version_for_profile(&profile, &layout)
            .value
            .or_else(|| family_state.installed.installed_version.clone())
    });
    family_state.installed.installed_signature = incoming_signature
        .clone()
        .or_else(|| installed_signature_for_profile(&profile, &layout))
        .or_else(|| family_state.installed.installed_signature.clone());
    family_state.installed.source_item_id = Some(applied_item_id);
    family_state.installed.checked_at = Some(Utc::now().to_rfc3339());
    if family_state.latest.is_none() {
        family_state.latest = load_or_refresh_latest_info(connection, &profile, false)?;
    }
    save_family_state(connection, &family_state)?;

    let mut statement = connection.prepare(
        "SELECT
            di.id,
            (
                SELECT COUNT(*)
                FROM files f
                WHERE f.download_item_id = di.id
                  AND f.source_location = 'downloads'
            ) AS active_file_count
         FROM download_items di
         WHERE di.matched_profile_key = ?1",
    )?;
    let family_rows = statement
        .query_map(params![profile_key], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?))
        })?
        .collect::<Result<Vec<_>, _>>()?;

    connection.execute(
        "UPDATE files
         SET download_item_id = NULL,
             indexed_at = CURRENT_TIMESTAMP
         WHERE source_location <> 'downloads'
           AND download_item_id IN (
                SELECT id
                FROM download_items
                WHERE matched_profile_key = ?1
                  AND id <> ?2
           )",
        params![profile_key, applied_item_id],
    )?;

    for (item_id, active_file_count) in &family_rows {
        if *item_id == applied_item_id || *active_file_count > 0 {
            continue;
        }

        connection.execute(
            "UPDATE download_items
             SET status = 'ignored',
                 updated_at = ?2
             WHERE id = ?1
               AND status NOT IN ('ignored', 'error')",
            params![item_id, Utc::now().to_rfc3339()],
        )?;
    }

    Ok(family_rows
        .into_iter()
        .map(|(item_id, _)| item_id)
        .collect())
}

fn special_decision_state_for_action(kind: &ReviewPlanActionKind) -> SpecialDecisionState {
    match kind {
        ReviewPlanActionKind::RepairSpecial => SpecialDecisionState::RepairBeforeUpdate,
        ReviewPlanActionKind::InstallDependency => SpecialDecisionState::InstallDependencyFirst,
        ReviewPlanActionKind::OpenDependency => SpecialDecisionState::OpenDependencyItem,
        ReviewPlanActionKind::OpenRelatedItem => SpecialDecisionState::OpenRelatedItem,
        ReviewPlanActionKind::DownloadMissingFiles => SpecialDecisionState::DownloadMissingFiles,
        ReviewPlanActionKind::OpenOfficialSource => SpecialDecisionState::OpenOfficialSource,
        ReviewPlanActionKind::SeparateSupportedFiles => {
            SpecialDecisionState::SeparateSupportedFiles
        }
    }
}

fn special_decision_queue_lane(
    state: &SpecialDecisionState,
    fallback_mode: &DownloadIntakeMode,
) -> DownloadQueueLane {
    match state {
        SpecialDecisionState::GuidedReady
        | SpecialDecisionState::RepairBeforeUpdate
        | SpecialDecisionState::SeparateSupportedFiles => DownloadQueueLane::SpecialSetup,
        SpecialDecisionState::InstallDependencyFirst
        | SpecialDecisionState::OpenDependencyItem
        | SpecialDecisionState::OpenRelatedItem
        | SpecialDecisionState::DownloadMissingFiles
        | SpecialDecisionState::OpenOfficialSource => DownloadQueueLane::WaitingOnYou,
        SpecialDecisionState::ReviewManually => {
            if *fallback_mode == DownloadIntakeMode::Blocked {
                DownloadQueueLane::Blocked
            } else {
                DownloadQueueLane::WaitingOnYou
            }
        }
    }
}

fn build_special_queue_summary(
    profile: &GuidedInstallProfileSeed,
    state: &SpecialDecisionState,
    local_pack_state: &SpecialLocalPackState,
    family_role: SpecialFamilyRole,
    primary_family_item_name: Option<String>,
    superseded_by_installed_family: bool,
    version_status: &SpecialVersionStatus,
    evaluation: &EvaluationResult,
) -> String {
    if superseded_by_installed_family {
        return format!(
            "A fuller {} pack from this family is already installed.",
            profile.display_name
        );
    }

    if family_role == SpecialFamilyRole::Superseded {
        return format!(
            "A fuller {} pack is already in Inbox as {}.",
            profile.display_name,
            primary_family_item_name.unwrap_or_else(|| "another download".to_owned())
        );
    }

    if *version_status == SpecialVersionStatus::SameVersion {
        return format!(
            "This {} download matches the version that is already installed.",
            profile.display_name
        );
    }

    if *version_status == SpecialVersionStatus::IncomingOlder {
        return format!(
            "This {} download looks older than the version already installed.",
            profile.display_name
        );
    }

    match state {
        SpecialDecisionState::GuidedReady => {
            if evaluation.assessment.existing_install_detected {
                format!(
                    "SimSuite has a safe {} update plan ready with your local download.",
                    profile.display_name
                )
            } else {
                format!(
                    "SimSuite recognized a full {} pack and has a safe install plan ready.",
                    profile.display_name
                )
            }
        }
        SpecialDecisionState::RepairBeforeUpdate => format!(
            "SimSuite found a full {} pack and can fix the older setup before updating.",
            profile.display_name
        ),
        SpecialDecisionState::InstallDependencyFirst => format!(
            "{} is ready, but one required helper needs to be installed first.",
            profile.display_name
        ),
        SpecialDecisionState::OpenDependencyItem => format!(
            "{} is waiting on another Inbox item first.",
            profile.display_name
        ),
        SpecialDecisionState::OpenRelatedItem => format!(
            "Use the fuller {} pack that is already waiting in Inbox.",
            profile.display_name
        ),
        SpecialDecisionState::DownloadMissingFiles => format!(
            "This {} download is incomplete, but SimSuite can fetch the trusted official pack first.",
            profile.display_name
        ),
        SpecialDecisionState::OpenOfficialSource => format!(
            "This {} download is incomplete and still needs the official full pack.",
            profile.display_name
        ),
        SpecialDecisionState::SeparateSupportedFiles => format!(
            "This batch contains a full {} set plus extra files. Separate it first.",
            profile.display_name
        ),
        SpecialDecisionState::ReviewManually => match local_pack_state {
            SpecialLocalPackState::Partial => format!(
                "SimSuite found part of {} but still needs a safer complete set.",
                profile.display_name
            ),
            SpecialLocalPackState::Mixed => format!(
                "SimSuite found {} inside this batch, but the extra files still need review.",
                profile.display_name
            ),
            _ => "This special setup still needs a careful manual check.".to_owned(),
        },
    }
}

fn detect_existing_layout_cached(
    connection: &Connection,
    settings: &LibrarySettings,
    seed_pack: &SeedPack,
    profile: &GuidedInstallProfileSeed,
    context: &mut SpecialDecisionContext,
) -> AppResult<ExistingInstallLayout> {
    if let Some(layout) = context.layout_cache.get(&profile.key) {
        return Ok(layout.clone());
    }

    let layout = detect_existing_layout(connection, settings, seed_pack, profile)?;
    context
        .layout_cache
        .insert(profile.key.clone(), layout.clone());
    Ok(layout)
}

fn load_family_siblings_cached(
    connection: &Connection,
    item_id: i64,
    profile_key: &str,
    context: &mut SpecialDecisionContext,
) -> AppResult<Vec<FamilySiblingRecord>> {
    if let Some(rows) = context.sibling_cache.get(profile_key) {
        return Ok(rows.clone());
    }

    let rows = load_family_siblings(connection, item_id, profile_key)?;
    context
        .sibling_cache
        .insert(profile_key.to_owned(), rows.clone());
    Ok(rows)
}

fn load_family_state_cached(
    connection: &Connection,
    context: &mut SpecialDecisionContext,
    profile: &GuidedInstallProfileSeed,
    existing_install_state: &SpecialExistingInstallState,
    install_path: Option<String>,
) -> AppResult<StoredSpecialModFamilyState> {
    if let Some(state) = context.family_state_cache.get(&profile.key) {
        return Ok(state.clone());
    }

    let state = load_family_state(
        connection,
        &profile.key,
        &profile.display_name,
        existing_install_state,
        install_path,
    )?;
    context
        .family_state_cache
        .insert(profile.key.clone(), state.clone());
    Ok(state)
}

fn existing_install_state_from_layout(
    layout: &ExistingInstallLayout,
) -> SpecialExistingInstallState {
    if !layout.existing_install_detected {
        SpecialExistingInstallState::NotInstalled
    } else if layout.repair_plan_available {
        SpecialExistingInstallState::Repairable
    } else if layout.safe_to_update {
        SpecialExistingInstallState::Clean
    } else {
        SpecialExistingInstallState::Blocked
    }
}

fn incoming_version_for_profile(
    profile: &GuidedInstallProfileSeed,
    display_name: &str,
    files: &[ProfileFile],
) -> VersionEvidence {
    let inside_mod_values = files
        .iter()
        .filter(|file| is_profile_content_file(file, profile))
        .flat_map(|file| file.insights.version_hints.iter().cloned())
        .collect::<Vec<_>>();
    if let Some(version) = extract_version_from_values(&inside_mod_values) {
        let mut lines = Vec::new();
        if let Some(file) = files.iter().find(|file| {
            is_profile_content_file(file, profile)
                && file
                    .insights
                    .version_hints
                    .iter()
                    .any(|hint| hint == &version)
        }) {
            push_version_evidence_line(
                &mut lines,
                format!(
                    "{} hinted {} from inside the download.",
                    file.filename, version
                ),
            );
        } else {
            push_version_evidence_line(
                &mut lines,
                format!("Inside-mod clues pointed to {}.", version),
            );
        }
        limit_version_evidence_lines(&mut lines);
        return VersionEvidence {
            value: Some(version),
            source: Some("inside mod".to_owned()),
            lines,
        };
    }

    let mut values = special_mod_versions::version_hints_from_profile(profile, display_name);
    values.extend(files.iter().map(|file| file.filename.clone()));
    values.extend(
        files
            .iter()
            .filter_map(|file| file.archive_member_path.clone()),
    );
    if let Some(version) = extract_version_from_values(&values) {
        let mut lines = Vec::new();
        if string_contains_version_hint(display_name, &version) {
            push_version_evidence_line(
                &mut lines,
                format!("The download name hinted {}.", version),
            );
        }
        if let Some(file) = files.iter().find(|file| {
            string_contains_version_hint(&file.filename, &version)
                || file
                    .archive_member_path
                    .as_deref()
                    .is_some_and(|path| string_contains_version_hint(path, &version))
        }) {
            push_version_evidence_line(
                &mut lines,
                format!(
                    "{} also hinted {} in the local file names.",
                    file.filename, version
                ),
            );
        }
        if lines.is_empty() {
            push_version_evidence_line(&mut lines, format!("Local file names hinted {}.", version));
        }
        limit_version_evidence_lines(&mut lines);
        return VersionEvidence {
            value: Some(version),
            source: Some("download name".to_owned()),
            lines,
        };
    }

    VersionEvidence::default()
}

fn incoming_signature_for_profile(
    profile: &GuidedInstallProfileSeed,
    files: &[ProfileFile],
) -> Option<String> {
    let entries = files
        .iter()
        .filter(|file| is_profile_content_file(file, profile))
        .map(|file| SignatureEntry {
            filename: file.filename.clone(),
            size: file.size,
            hash: file.hash.clone(),
        })
        .collect::<Vec<_>>();
    build_signature(&entries)
}

fn installed_version_for_profile(
    profile: &GuidedInstallProfileSeed,
    layout: &ExistingInstallLayout,
) -> VersionEvidence {
    let inside_mod_values = layout
        .existing_files
        .iter()
        .chain(layout.preserve_files.iter())
        .flat_map(|file| file.insights.version_hints.iter().cloned())
        .collect::<Vec<_>>();
    if let Some(version) = extract_version_from_values(&inside_mod_values) {
        let mut lines = Vec::new();
        if let Some(file) = layout
            .existing_files
            .iter()
            .chain(layout.preserve_files.iter())
            .find(|file| {
                file.insights
                    .version_hints
                    .iter()
                    .any(|hint| hint == &version)
            })
        {
            push_version_evidence_line(
                &mut lines,
                format!(
                    "{} hinted {} from the installed mod files.",
                    file.filename, version
                ),
            );
        } else {
            push_version_evidence_line(
                &mut lines,
                format!("Installed inside-mod clues pointed to {}.", version),
            );
        }
        limit_version_evidence_lines(&mut lines);
        return VersionEvidence {
            value: Some(version),
            source: Some("inside mod".to_owned()),
            lines,
        };
    }

    let mut values = layout
        .existing_files
        .iter()
        .map(|file| file.filename.clone())
        .collect::<Vec<_>>();
    values.extend(
        layout
            .preserve_files
            .iter()
            .map(|file| file.filename.clone()),
    );
    values.extend(profile.version_file_hints.iter().cloned());
    if let Some(version) = extract_version_from_values(&values) {
        let mut lines = Vec::new();
        if let Some(file) = layout
            .existing_files
            .iter()
            .chain(layout.preserve_files.iter())
            .find(|file| string_contains_version_hint(&file.filename, &version))
        {
            push_version_evidence_line(
                &mut lines,
                format!(
                    "{} hinted {} from the installed file names.",
                    file.filename, version
                ),
            );
        } else {
            push_version_evidence_line(
                &mut lines,
                format!("Installed file names hinted {}.", version),
            );
        }
        limit_version_evidence_lines(&mut lines);
        return VersionEvidence {
            value: Some(version),
            source: Some("installed files".to_owned()),
            lines,
        };
    }

    VersionEvidence::default()
}

fn build_comparison_evidence_lines(
    incoming_version: Option<&str>,
    installed_version: Option<&str>,
    incoming_signature: Option<&str>,
    installed_signature: Option<&str>,
    version_status: &SpecialVersionStatus,
    comparison_source: Option<&str>,
) -> Vec<String> {
    let mut lines = Vec::new();

    if incoming_signature.is_some()
        && installed_signature.is_some()
        && *version_status == SpecialVersionStatus::SameVersion
        && (incoming_version.is_none() || installed_version.is_none())
    {
        push_version_evidence_line(
            &mut lines,
            "Matching file fingerprints confirmed the same version.".to_owned(),
        );
    } else if let (Some(incoming), Some(installed)) = (incoming_version, installed_version) {
        push_version_evidence_line(
            &mut lines,
            format!(
                "SimSuite compared the local versions {} and {}.",
                incoming, installed
            ),
        );
    }

    match comparison_source {
        Some("inside mod") => push_version_evidence_line(
            &mut lines,
            "The best local clue came from inside the mod files.".to_owned(),
        ),
        Some("saved family state") => push_version_evidence_line(
            &mut lines,
            "The installed side fell back to the last saved family record.".to_owned(),
        ),
        Some("local file names") => push_version_evidence_line(
            &mut lines,
            "SimSuite fell back to local archive and file names for this check.".to_owned(),
        ),
        Some("file signature") if lines.is_empty() => push_version_evidence_line(
            &mut lines,
            "Matching file fingerprints were the strongest local clue.".to_owned(),
        ),
        _ => {}
    }

    limit_version_evidence_lines(&mut lines);
    lines
}

fn installed_signature_for_profile(
    profile: &GuidedInstallProfileSeed,
    layout: &ExistingInstallLayout,
) -> Option<String> {
    let entries = layout
        .existing_files
        .iter()
        .filter(|file| is_existing_profile_filename(&file.filename, &file.extension, profile))
        .map(|file| SignatureEntry {
            filename: file.filename.clone(),
            size: file.size,
            hash: file.hash.clone(),
        })
        .collect::<Vec<_>>();
    build_signature(&entries)
}

fn load_family_siblings(
    connection: &Connection,
    item_id: i64,
    profile_key: &str,
) -> AppResult<Vec<FamilySiblingRecord>> {
    let mut statement = connection.prepare(
        "SELECT
            di.id,
            di.display_name,
            di.intake_mode,
            COALESCE(di.guided_install_available, 0),
            (
                SELECT COUNT(*)
                FROM files f
                WHERE f.download_item_id = di.id
                  AND f.source_location = 'downloads'
            ) AS active_file_count,
            (
                SELECT COUNT(DISTINCT rq.file_id)
                FROM review_queue rq
                JOIN files f ON f.id = rq.file_id
                WHERE f.download_item_id = di.id
                  AND f.source_location = 'downloads'
            ) AS review_file_count,
            di.updated_at,
            di.status
         FROM download_items di
         WHERE di.matched_profile_key = ?1
            AND di.status NOT IN ('ignored', 'error')
         ORDER BY di.updated_at DESC, di.id DESC",
    )?;

    let rows = statement
        .query_map(params![profile_key], |row| {
            Ok(FamilySiblingRecord {
                id: row.get(0)?,
                display_name: row.get(1)?,
                intake_mode: parse_download_intake_mode(row.get::<_, String>(2)?),
                guided_install_available: row.get::<_, i64>(3)? != 0,
                active_file_count: row.get(4)?,
                review_file_count: row.get(5)?,
                updated_at: row.get(6)?,
                status: row.get(7)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    if rows.iter().all(|row| row.id != item_id) {
        return Ok(rows);
    }

    Ok(rows)
}

fn select_primary_family_item(
    item_id: i64,
    siblings: &[FamilySiblingRecord],
) -> Option<FamilySiblingRecord> {
    siblings
        .iter()
        .cloned()
        .max_by(|left, right| {
            let left_score = family_item_score(left);
            let right_score = family_item_score(right);
            left_score
                .cmp(&right_score)
                .then_with(|| left.updated_at.cmp(&right.updated_at))
                .then_with(|| left.id.cmp(&right.id))
        })
        .or_else(|| {
            siblings
                .iter()
                .find(|sibling| sibling.id == item_id)
                .cloned()
        })
}

fn family_item_score(item: &FamilySiblingRecord) -> i64 {
    let guided_bonus = if item.guided_install_available {
        4000
    } else {
        0
    };
    let status_bonus = match item.status.as_str() {
        "ready" => 350,
        "partial" => 220,
        "needs_review" => 120,
        _ => 0,
    };
    let intake_bonus = match item.intake_mode {
        DownloadIntakeMode::Guided => 1500,
        DownloadIntakeMode::NeedsReview => 900,
        DownloadIntakeMode::Blocked => 500,
        DownloadIntakeMode::Standard => 100,
    };

    guided_bonus + status_bonus + intake_bonus + (item.active_file_count * 25)
        - (item.review_file_count * 60)
}

fn parse_download_intake_mode(value: String) -> DownloadIntakeMode {
    match value.as_str() {
        "guided" => DownloadIntakeMode::Guided,
        "needs_review" => DownloadIntakeMode::NeedsReview,
        "blocked" => DownloadIntakeMode::Blocked,
        _ => DownloadIntakeMode::Standard,
    }
}

fn evaluate_download_item(
    connection: &Connection,
    settings: &LibrarySettings,
    seed_pack: &SeedPack,
    item_id: i64,
) -> AppResult<EvaluationResult> {
    let item = load_item_with_staging(connection, item_id)?.ok_or_else(|| {
        AppError::Message(
            "Inbox item was not found while building the special install plan.".to_owned(),
        )
    })?;
    let files = load_profile_files(connection, seed_pack, item_id, true)?;
    let text_clues = read_text_clues(item.staging_path.as_deref(), &files)?;
    let archive_path_clues = collect_archive_path_clues(&files);
    let review_patterns =
        collect_review_patterns(seed_pack, &item, &files, &text_clues, &archive_path_clues);
    let candidates =
        collect_guided_candidates(seed_pack, &item, &files, &text_clues, &archive_path_clues);
    let strong_candidates = candidates
        .iter()
        .filter(|candidate| candidate.evidence.strong_match(&candidate.profile))
        .cloned()
        .collect::<Vec<_>>();

    if strong_candidates.len() > 1 {
        let names = strong_candidates
            .iter()
            .map(|candidate| candidate.profile.display_name.clone())
            .collect::<Vec<_>>()
            .join(", ");
        return Ok(EvaluationResult {
            assessment: DownloadItemAssessment {
                intake_mode: DownloadIntakeMode::Blocked,
                risk_level: DownloadRiskLevel::High,
                matched_profile_key: None,
                matched_profile_name: None,
                special_family: None,
                assessment_reasons: vec![format!(
                    "SimSuite matched more than one special setup family in one inbox item: {names}."
                )],
                dependency_summary: Vec::new(),
                missing_dependencies: Vec::new(),
                inbox_dependencies: Vec::new(),
                incompatibility_warnings: Vec::new(),
                post_install_notes: Vec::new(),
                evidence_summary: vec![
                    "Multiple special setup families were detected in one archive.".to_owned()
                ],
                catalog_source: None,
                existing_install_detected: false,
                guided_install_available: false,
            },
            matched_profile: None,
            matched_pattern: None,
            matched_evidence: None,
            dependencies: Vec::new(),
            existing_layout_findings: Vec::new(),
            explanation:
                "This download mixes more than one known special-mod family, so SimSuite will not guess."
                    .to_owned(),
            recommended_next_step:
                "Split this download into separate special-mod installs or keep it in review."
                    .to_owned(),
        });
    }

    let best_candidate = candidates
        .into_iter()
        .max_by_key(|candidate| candidate.evidence.score(&candidate.profile));
    let matched_dependency_rules =
        collect_dependency_matches(seed_pack, &item, &files, &text_clues, &archive_path_clues);
    let matched_incompatibility_rules = collect_incompatibility_rule_matches(
        seed_pack,
        &item,
        &files,
        &text_clues,
        &archive_path_clues,
    );

    if let Some(candidate) = best_candidate {
        let layout = detect_existing_layout(connection, settings, seed_pack, &candidate.profile)?;
        let dependency_rules = collect_required_dependency_rules(
            seed_pack,
            &candidate.profile,
            &matched_dependency_rules,
        );
        let dependencies =
            resolve_dependency_status(connection, settings, seed_pack, item_id, &dependency_rules)?;
        let incompatibility_warnings = collect_incompatibility_warnings(
            connection,
            settings,
            seed_pack,
            &candidate.profile,
            &matched_incompatibility_rules,
        )?;
        let missing_dependencies = dependencies
            .iter()
            .filter(|dependency| dependency.status == "missing")
            .map(|dependency| dependency.display_name.clone())
            .collect::<Vec<_>>();
        let inbox_dependencies = dependencies
            .iter()
            .filter(|dependency| dependency.status == "inbox")
            .map(|dependency| dependency.display_name.clone())
            .collect::<Vec<_>>();
        let dependency_summary = dependencies
            .iter()
            .map(|dependency| dependency.summary.clone())
            .collect::<Vec<_>>();
        let mut reasons = candidate.evidence.reasons.clone();
        let mut evidence_summary = build_evidence_summary(&candidate.evidence);
        let catalog_source = Some(profile_catalog_source(&candidate.profile));
        let matched_evidence = Some(candidate.evidence.clone());

        if !layout.warnings.is_empty() {
            evidence_summary.extend(layout.warnings.iter().cloned());
        }

        if !candidate.evidence.required_core_present && candidate.evidence.matched_files > 0 {
            reasons.push(profile_block_reason(
                &candidate.profile,
                0,
                "Required core files are missing from this special-mod set.",
            ));
            return Ok(EvaluationResult {
                assessment: DownloadItemAssessment {
                    intake_mode: DownloadIntakeMode::Blocked,
                    risk_level: DownloadRiskLevel::High,
                    matched_profile_key: Some(candidate.profile.key.clone()),
                    matched_profile_name: Some(candidate.profile.display_name.clone()),
                    special_family: Some(candidate.profile.family.clone()),
                    assessment_reasons: reasons,
                    dependency_summary,
                    missing_dependencies,
                    inbox_dependencies,
                    incompatibility_warnings,
                    post_install_notes: candidate.profile.post_install_notes.clone(),
                    evidence_summary,
                    catalog_source,
                    existing_install_detected: layout.existing_install_detected,
                    guided_install_available: false,
                },
                matched_profile: Some(candidate.profile.clone()),
                matched_pattern: None,
                matched_evidence,
                dependencies,
                existing_layout_findings: layout.warnings,
                explanation: profile_block_reason(
                    &candidate.profile,
                    0,
                    "SimSuite found part of a known special suite, but the required core files are missing.",
                ),
                recommended_next_step:
                    "Download the full special-mod archive again or keep this item in review."
                        .to_owned(),
            });
        }

        if layout.repair_plan_available {
            reasons.push(profile_block_reason(
                &candidate.profile,
                2,
                "The older install is spread around Mods and needs one safe repair pass first.",
            ));
            return Ok(EvaluationResult {
                assessment: DownloadItemAssessment {
                    intake_mode: DownloadIntakeMode::Blocked,
                    risk_level: DownloadRiskLevel::High,
                    matched_profile_key: Some(candidate.profile.key.clone()),
                    matched_profile_name: Some(candidate.profile.display_name.clone()),
                    special_family: Some(candidate.profile.family.clone()),
                    assessment_reasons: reasons,
                    dependency_summary,
                    missing_dependencies,
                    inbox_dependencies,
                    incompatibility_warnings,
                    post_install_notes: candidate.profile.post_install_notes.clone(),
                    evidence_summary,
                    catalog_source,
                    existing_install_detected: layout.existing_install_detected,
                    guided_install_available: false,
                },
                matched_profile: Some(candidate.profile.clone()),
                matched_pattern: None,
                matched_evidence,
                dependencies,
                existing_layout_findings: layout.warnings,
                explanation: format!(
                    "SimSuite found a full {} update, but the older {} files are still spread around Mods.",
                    candidate.profile.display_name, candidate.profile.display_name
                ),
                recommended_next_step: format!(
                    "Fix the old {} setup first, then let SimSuite finish the update safely.",
                    candidate.profile.display_name
                ),
            });
        }

        if !layout.safe_to_update {
            reasons.push(profile_block_reason(
                &candidate.profile,
                2,
                "Existing files for this special mod are scattered across an unsafe layout.",
            ));
            return Ok(EvaluationResult {
                assessment: DownloadItemAssessment {
                    intake_mode: DownloadIntakeMode::Blocked,
                    risk_level: DownloadRiskLevel::High,
                    matched_profile_key: Some(candidate.profile.key.clone()),
                    matched_profile_name: Some(candidate.profile.display_name.clone()),
                    special_family: Some(candidate.profile.family.clone()),
                    assessment_reasons: reasons,
                    dependency_summary,
                    missing_dependencies,
                    inbox_dependencies,
                    incompatibility_warnings,
                    post_install_notes: candidate.profile.post_install_notes.clone(),
                    evidence_summary,
                    catalog_source,
                    existing_install_detected: layout.existing_install_detected,
                    guided_install_available: false,
                },
                matched_profile: Some(candidate.profile.clone()),
                matched_pattern: None,
                matched_evidence,
                dependencies,
                existing_layout_findings: layout.warnings,
                explanation: profile_block_reason(
                    &candidate.profile,
                    2,
                    "Existing files for this special mod are scattered across multiple locations.",
                ),
                recommended_next_step:
                    "Open the Library or Review queue and clean up the current install before updating."
                        .to_owned(),
            });
        }

        if !candidate.evidence.strong_match(&candidate.profile) {
            reasons.push(profile_review_message(&candidate.profile));
            return Ok(EvaluationResult {
                assessment: DownloadItemAssessment {
                    intake_mode: DownloadIntakeMode::NeedsReview,
                    risk_level: DownloadRiskLevel::Medium,
                    matched_profile_key: Some(candidate.profile.key.clone()),
                    matched_profile_name: Some(candidate.profile.display_name.clone()),
                    special_family: Some(candidate.profile.family.clone()),
                    assessment_reasons: reasons,
                    dependency_summary,
                    missing_dependencies,
                    inbox_dependencies,
                    incompatibility_warnings,
                    post_install_notes: candidate.profile.post_install_notes.clone(),
                    evidence_summary,
                    catalog_source,
                    existing_install_detected: layout.existing_install_detected,
                    guided_install_available: false,
                },
                matched_profile: Some(candidate.profile.clone()),
                matched_pattern: None,
                matched_evidence,
                dependencies,
                existing_layout_findings: layout.warnings,
                explanation: candidate.profile.help_summary.clone(),
                recommended_next_step:
                    "Check the file list and confirm that this is the full special-mod set before installing."
                        .to_owned(),
            });
        }

        if let Some(pattern) = review_patterns.first() {
            reasons.push(pattern.review_reason.clone());
            return Ok(EvaluationResult {
                assessment: DownloadItemAssessment {
                    intake_mode: DownloadIntakeMode::NeedsReview,
                    risk_level: DownloadRiskLevel::Medium,
                    matched_profile_key: Some(candidate.profile.key.clone()),
                    matched_profile_name: Some(candidate.profile.display_name.clone()),
                    special_family: Some(candidate.profile.family.clone()),
                    assessment_reasons: reasons,
                    dependency_summary,
                    missing_dependencies,
                    inbox_dependencies,
                    incompatibility_warnings,
                    post_install_notes: candidate.profile.post_install_notes.clone(),
                    evidence_summary,
                    catalog_source: Some(pattern_catalog_source(pattern)),
                    existing_install_detected: layout.existing_install_detected,
                    guided_install_available: false,
                },
                matched_profile: Some(candidate.profile.clone()),
                matched_pattern: Some(pattern.clone()),
                matched_evidence,
                dependencies,
                existing_layout_findings: layout.warnings,
                explanation: pattern.help_summary.clone(),
                recommended_next_step:
                    "Pick the correct option manually, then keep the item in review or unpack only the version you want."
                        .to_owned(),
            });
        }

        if has_supported_subset_to_separate(&files, &candidate.profile) {
            let extra_supported = collect_extra_supported_files(&files, &candidate.profile);
            reasons.push(profile_block_reason(
                &candidate.profile,
                1,
                "This inbox batch mixes a supported special-mod set with other script or package files.",
            ));
            evidence_summary.push(format!(
                "{} extra script or package file(s) are mixed into this inbox batch.",
                extra_supported.len()
            ));

            return Ok(EvaluationResult {
                assessment: DownloadItemAssessment {
                    intake_mode: DownloadIntakeMode::NeedsReview,
                    risk_level: DownloadRiskLevel::Medium,
                    matched_profile_key: Some(candidate.profile.key.clone()),
                    matched_profile_name: Some(candidate.profile.display_name.clone()),
                    special_family: Some(candidate.profile.family.clone()),
                    assessment_reasons: reasons,
                    dependency_summary,
                    missing_dependencies,
                    inbox_dependencies,
                    incompatibility_warnings,
                    post_install_notes: candidate.profile.post_install_notes.clone(),
                    evidence_summary,
                    catalog_source,
                    existing_install_detected: layout.existing_install_detected,
                    guided_install_available: false,
                },
                matched_profile: Some(candidate.profile.clone()),
                matched_pattern: None,
                matched_evidence,
                dependencies,
                existing_layout_findings: layout.warnings,
                explanation: format!(
                    "SimSuite found a full {} set, but this batch also contains other script or package files.",
                    candidate.profile.display_name
                ),
                recommended_next_step: format!(
                    "Separate the {} files first, then let SimSuite continue with the special install.",
                    candidate.profile.display_name
                ),
            });
        }

        if !missing_dependencies.is_empty()
            || !inbox_dependencies.is_empty()
            || !incompatibility_warnings.is_empty()
        {
            let explanation = if !missing_dependencies.is_empty() {
                format!(
                    "{} needs another support library before SimSuite can install it safely.",
                    candidate.profile.display_name
                )
            } else if !inbox_dependencies.is_empty() {
                format!(
                    "{} depends on another special mod that is also waiting in the Inbox.",
                    candidate.profile.display_name
                )
            } else {
                format!(
                    "{} has a known incompatibility warning that needs review first.",
                    candidate.profile.display_name
                )
            };

            return Ok(EvaluationResult {
                assessment: DownloadItemAssessment {
                    intake_mode: DownloadIntakeMode::NeedsReview,
                    risk_level: DownloadRiskLevel::Medium,
                    matched_profile_key: Some(candidate.profile.key.clone()),
                    matched_profile_name: Some(candidate.profile.display_name.clone()),
                    special_family: Some(candidate.profile.family.clone()),
                    assessment_reasons: reasons,
                    dependency_summary,
                    missing_dependencies,
                    inbox_dependencies,
                    incompatibility_warnings,
                    post_install_notes: candidate.profile.post_install_notes.clone(),
                    evidence_summary,
                    catalog_source,
                    existing_install_detected: layout.existing_install_detected,
                    guided_install_available: false,
                },
                matched_profile: Some(candidate.profile.clone()),
                matched_pattern: None,
                matched_evidence,
                dependencies,
                existing_layout_findings: layout.warnings,
                explanation,
                recommended_next_step:
                    "Resolve the dependency or incompatibility first, then come back to this special setup."
                        .to_owned(),
            });
        }

        return Ok(EvaluationResult {
            assessment: DownloadItemAssessment {
                intake_mode: DownloadIntakeMode::Guided,
                risk_level: DownloadRiskLevel::Medium,
                matched_profile_key: Some(candidate.profile.key.clone()),
                matched_profile_name: Some(candidate.profile.display_name.clone()),
                special_family: Some(candidate.profile.family.clone()),
                assessment_reasons: reasons,
                dependency_summary,
                missing_dependencies,
                inbox_dependencies,
                incompatibility_warnings,
                post_install_notes: candidate.profile.post_install_notes.clone(),
                evidence_summary,
                catalog_source,
                existing_install_detected: layout.existing_install_detected,
                guided_install_available: true,
            },
            matched_profile: Some(candidate.profile.clone()),
            matched_pattern: None,
            matched_evidence,
            dependencies,
            existing_layout_findings: layout.warnings,
            explanation: candidate.profile.help_summary.clone(),
            recommended_next_step:
                "Review the guided plan, then approve the special setup if everything looks right."
                    .to_owned(),
        });
    }

    let dependencies = resolve_dependency_status(
        connection,
        settings,
        seed_pack,
        item_id,
        &matched_dependency_rules,
    )?;
    let missing_dependencies = dependencies
        .iter()
        .filter(|dependency| dependency.status == "missing")
        .map(|dependency| dependency.display_name.clone())
        .collect::<Vec<_>>();
    let inbox_dependencies = dependencies
        .iter()
        .filter(|dependency| dependency.status == "inbox")
        .map(|dependency| dependency.display_name.clone())
        .collect::<Vec<_>>();
    let dependency_summary = dependencies
        .iter()
        .map(|dependency| dependency.summary.clone())
        .collect::<Vec<_>>();
    let incompatibility_warnings = collect_non_profile_incompatibility_warnings(
        connection,
        settings,
        seed_pack,
        &matched_incompatibility_rules,
    )?;

    if let Some(pattern) = review_patterns.first() {
        return Ok(EvaluationResult {
            assessment: DownloadItemAssessment {
                intake_mode: DownloadIntakeMode::NeedsReview,
                risk_level: DownloadRiskLevel::Medium,
                matched_profile_key: None,
                matched_profile_name: None,
                special_family: Some("Manual review".to_owned()),
                assessment_reasons: vec![pattern.review_reason.clone()],
                dependency_summary,
                missing_dependencies,
                inbox_dependencies,
                incompatibility_warnings,
                post_install_notes: Vec::new(),
                evidence_summary: vec![pattern.help_summary.clone()],
                catalog_source: Some(pattern_catalog_source(pattern)),
                existing_install_detected: false,
                guided_install_available: false,
            },
            matched_profile: None,
            matched_pattern: Some(pattern.clone()),
            matched_evidence: None,
            dependencies,
            existing_layout_findings: Vec::new(),
            explanation: pattern.help_summary.clone(),
            recommended_next_step:
                "Choose the correct option manually and keep this item in review until it is clear."
                    .to_owned(),
        });
    }

    if !missing_dependencies.is_empty()
        || !inbox_dependencies.is_empty()
        || !incompatibility_warnings.is_empty()
    {
        let explanation = if !missing_dependencies.is_empty() {
            "This download mentions another support library that is not installed yet.".to_owned()
        } else if !inbox_dependencies.is_empty() {
            "This download depends on another library that is also waiting in the Inbox.".to_owned()
        } else {
            "This download has a known incompatibility warning and should be reviewed first."
                .to_owned()
        };

        return Ok(EvaluationResult {
            assessment: DownloadItemAssessment {
                intake_mode: DownloadIntakeMode::NeedsReview,
                risk_level: DownloadRiskLevel::Medium,
                matched_profile_key: None,
                matched_profile_name: None,
                special_family: Some("Dependencies".to_owned()),
                assessment_reasons: dependency_summary.clone(),
                dependency_summary,
                missing_dependencies,
                inbox_dependencies,
                incompatibility_warnings,
                post_install_notes: Vec::new(),
                evidence_summary: Vec::new(),
                catalog_source: matched_dependency_rules
                    .first()
                    .map(dependency_catalog_source)
                    .or_else(|| {
                        matched_incompatibility_rules
                            .first()
                            .map(incompatibility_catalog_source)
                    }),
                existing_install_detected: false,
                guided_install_available: false,
            },
            matched_profile: None,
            matched_pattern: None,
            matched_evidence: None,
            dependencies,
            existing_layout_findings: Vec::new(),
            explanation,
            recommended_next_step:
                "Install the missing dependency first or review the conflict before moving this item."
                    .to_owned(),
        });
    }

    Ok(EvaluationResult {
        assessment: DownloadItemAssessment {
            intake_mode: DownloadIntakeMode::Standard,
            risk_level: DownloadRiskLevel::Low,
            matched_profile_key: None,
            matched_profile_name: None,
            special_family: None,
            assessment_reasons: vec!["No special setup profile matched this download.".to_owned()],
            dependency_summary,
            missing_dependencies,
            inbox_dependencies,
            incompatibility_warnings,
            post_install_notes: Vec::new(),
            evidence_summary: Vec::new(),
            catalog_source: None,
            existing_install_detected: false,
            guided_install_available: false,
        },
        matched_profile: None,
        matched_pattern: None,
        matched_evidence: None,
        dependencies,
        existing_layout_findings: Vec::new(),
        explanation:
            "This looks like a normal download, so it can use the standard safe hand-off preview."
                .to_owned(),
        recommended_next_step: "Open the normal hand-off preview and review the safe batch."
            .to_owned(),
    })
}

fn collect_guided_candidates(
    seed_pack: &SeedPack,
    item: &DownloadItemRecord,
    files: &[ProfileFile],
    text_clues: &[String],
    archive_path_clues: &[String],
) -> Vec<GuidedCandidate> {
    seed_pack
        .install_catalog
        .guided_profiles
        .iter()
        .filter_map(|profile| {
            let evidence =
                collect_profile_evidence(profile, item, files, text_clues, archive_path_clues);
            if evidence.has_any_signal() {
                Some(GuidedCandidate {
                    profile: profile.clone(),
                    evidence,
                })
            } else {
                None
            }
        })
        .collect()
}

fn collect_profile_evidence(
    profile: &GuidedInstallProfileSeed,
    item: &DownloadItemRecord,
    files: &[ProfileFile],
    text_clues: &[String],
    archive_path_clues: &[String],
) -> ProfileEvidence {
    let item_inputs = collect_item_inputs(item, files);
    let name_match = item_inputs.iter().any(|input| {
        clue_list_matches(input, &profile.name_clues)
            || clue_list_matches(input, &profile.sample_filenames)
    });
    let required_core_present = files.iter().any(|file| {
        matches_required_core(
            &file.filename,
            &file.extension,
            &profile.required_name_clues,
        )
    });
    let matched_files = files
        .iter()
        .filter(|file| is_profile_content_file(file, profile))
        .count() as i64;
    let matched_script_files = files
        .iter()
        .filter(|file| file.extension == ".ts4script" && is_profile_content_file(file, profile))
        .count() as i64;
    let matched_package_files = files
        .iter()
        .filter(|file| file.extension == ".package" && is_profile_content_file(file, profile))
        .count() as i64;
    let required_exact_filenames_found = profile
        .required_all_filenames
        .iter()
        .filter(|required_filename| {
            files
                .iter()
                .any(|file| normalized(&file.filename) == normalized(required_filename))
        })
        .count() as i64;
    let unmatched_supported_files = files
        .iter()
        .filter(|file| is_supported_special_extension(&file.extension))
        .filter(|file| !is_profile_content_file(file, profile))
        .filter(|file| !matches_preserve_rule(&file.filename, &file.extension, profile))
        .count() as i64;
    let text_matches = collect_matched_clues(text_clues, &profile.text_clues);
    let archive_matches = collect_matched_clues(archive_path_clues, &profile.archive_path_clues);

    let mut reasons = Vec::new();
    if matched_files > 0 {
        reasons.push(format!(
            "{} matching package or script files were found for {}.",
            matched_files, profile.display_name
        ));
    }
    if required_core_present {
        reasons.push(format!(
            "Required core files for {} were found.",
            profile.display_name
        ));
    }
    if matched_script_files > 0 {
        reasons.push(format!(
            "{} matching script file(s) fit {}.",
            matched_script_files, profile.display_name
        ));
    }
    if !profile.required_all_filenames.is_empty() && required_exact_filenames_found > 0 {
        reasons.push(format!(
            "{} required exact file name(s) for {} were found.",
            required_exact_filenames_found, profile.display_name
        ));
    }
    if !text_matches.is_empty() {
        reasons.push(format!(
            "Readme or embedded text mentions {}.",
            text_matches.join(", ")
        ));
    }
    if !archive_matches.is_empty() {
        reasons.push(format!(
            "Folder or archive names mention {}.",
            archive_matches.join(", ")
        ));
    }
    if unmatched_supported_files > 0 {
        reasons.push(format!(
            "{} extra supported files do not fit the {} set.",
            unmatched_supported_files, profile.display_name
        ));
    }

    ProfileEvidence {
        reasons,
        matched_files,
        matched_script_files,
        matched_package_files,
        unmatched_supported_files,
        required_core_present,
        required_exact_filenames_found,
        name_match,
        text_matches,
        archive_matches,
    }
}

fn collect_dependency_matches(
    seed_pack: &SeedPack,
    item: &DownloadItemRecord,
    files: &[ProfileFile],
    text_clues: &[String],
    archive_path_clues: &[String],
) -> Vec<DependencyRuleSeed> {
    seed_pack
        .install_catalog
        .dependency_rules
        .iter()
        .filter(|rule| matches_dependency_rule(rule, item, files, text_clues, archive_path_clues))
        .cloned()
        .collect()
}

fn collect_required_dependency_rules(
    seed_pack: &SeedPack,
    profile: &GuidedInstallProfileSeed,
    matched_rules: &[DependencyRuleSeed],
) -> Vec<DependencyRuleSeed> {
    let mut keys = HashSet::new();
    let mut rules = Vec::new();

    for dependency_key in &profile.dependency_keys {
        for rule in &seed_pack.install_catalog.dependency_rules {
            if &rule.dependency_key == dependency_key && keys.insert(rule.key.clone()) {
                rules.push(rule.clone());
            }
        }
    }

    for rule in matched_rules {
        if keys.insert(rule.key.clone()) {
            rules.push(rule.clone());
        }
    }

    rules
}

fn resolve_dependency_status(
    connection: &Connection,
    settings: &LibrarySettings,
    seed_pack: &SeedPack,
    item_id: i64,
    rules: &[DependencyRuleSeed],
) -> AppResult<Vec<DependencyStatus>> {
    let mut seen = HashSet::new();
    let mut result = Vec::new();

    for rule in rules {
        if !seen.insert(rule.dependency_key.clone()) {
            continue;
        }

        let installed =
            dependency_is_installed(connection, settings, seed_pack, &rule.dependency_key)?;
        let inbox_item = if installed {
            None
        } else {
            dependency_in_inbox(connection, item_id, &rule.dependency_key)?
        };

        let (status, summary) = if installed {
            (
                "installed".to_owned(),
                format!("{} is already installed.", rule.display_name),
            )
        } else if let Some(item) = &inbox_item {
            let summary = if item.guided_install_available
                && item.intake_mode == DownloadIntakeMode::Guided
            {
                format!(
                    "{} is in the Inbox and ready for safe setup as {}.",
                    rule.display_name, item.display_name
                )
            } else {
                format!(
                    "{} is also in the Inbox as {} and needs its own check first.",
                    rule.display_name, item.display_name
                )
            };
            ("inbox".to_owned(), summary)
        } else {
            (
                "missing".to_owned(),
                format!(
                    "{} is required before this mod can be installed safely.",
                    rule.display_name
                ),
            )
        };

        result.push(DependencyStatus {
            key: rule.dependency_key.clone(),
            display_name: rule.display_name.clone(),
            status,
            summary,
            inbox_item_id: inbox_item.as_ref().map(|item| item.id),
            inbox_item_name: inbox_item.as_ref().map(|item| item.display_name.clone()),
            inbox_item_intake_mode: inbox_item.as_ref().map(|item| item.intake_mode.clone()),
            inbox_item_guided_install_available: inbox_item
                .as_ref()
                .map(|item| item.guided_install_available)
                .unwrap_or(false),
        });
    }

    Ok(result)
}

fn collect_incompatibility_rule_matches(
    seed_pack: &SeedPack,
    item: &DownloadItemRecord,
    files: &[ProfileFile],
    text_clues: &[String],
    archive_path_clues: &[String],
) -> Vec<IncompatibilityRuleSeed> {
    seed_pack
        .install_catalog
        .incompatibility_rules
        .iter()
        .filter(|rule| {
            matches_incompatibility_rule(rule, item, files, text_clues, archive_path_clues)
        })
        .cloned()
        .collect()
}

fn collect_incompatibility_warnings(
    connection: &Connection,
    settings: &LibrarySettings,
    seed_pack: &SeedPack,
    profile: &GuidedInstallProfileSeed,
    matched_rules: &[IncompatibilityRuleSeed],
) -> AppResult<Vec<String>> {
    let mut seen = HashSet::new();
    let mut warnings = Vec::new();

    for rule in matched_rules.iter().chain(
        seed_pack
            .install_catalog
            .incompatibility_rules
            .iter()
            .filter(|rule| {
                profile
                    .incompatibility_keys
                    .iter()
                    .any(|key| key == &rule.key)
            }),
    ) {
        if !seen.insert(rule.key.clone()) {
            continue;
        }

        if dependency_is_installed(connection, settings, seed_pack, &rule.installed_profile_key)? {
            warnings.push(rule.warning_message.clone());
        }
    }

    Ok(warnings)
}

fn collect_non_profile_incompatibility_warnings(
    connection: &Connection,
    settings: &LibrarySettings,
    seed_pack: &SeedPack,
    matched_rules: &[IncompatibilityRuleSeed],
) -> AppResult<Vec<String>> {
    let mut warnings = Vec::new();
    for rule in matched_rules {
        if dependency_is_installed(connection, settings, seed_pack, &rule.installed_profile_key)? {
            warnings.push(rule.warning_message.clone());
        }
    }
    Ok(warnings)
}

fn collect_review_patterns(
    seed_pack: &SeedPack,
    item: &DownloadItemRecord,
    files: &[ProfileFile],
    text_clues: &[String],
    archive_path_clues: &[String],
) -> Vec<ReviewOnlyPatternSeed> {
    let item_inputs = collect_item_inputs(item, files);
    seed_pack
        .install_catalog
        .review_only_patterns
        .iter()
        .filter(|pattern| {
            item_inputs
                .iter()
                .any(|input| clue_list_matches(input, &pattern.name_clues))
                || !collect_matched_clues(text_clues, &pattern.text_clues).is_empty()
                || !collect_matched_clues(archive_path_clues, &pattern.archive_path_clues)
                    .is_empty()
        })
        .cloned()
        .collect()
}

fn dependency_is_installed(
    connection: &Connection,
    settings: &LibrarySettings,
    seed_pack: &SeedPack,
    dependency_key: &str,
) -> AppResult<bool> {
    if let Some(profile) = seed_pack
        .install_catalog
        .guided_profiles
        .iter()
        .find(|profile| profile.key == dependency_key)
    {
        return detect_existing_layout(connection, settings, seed_pack, profile)
            .map(|layout| layout.existing_install_detected);
    }

    let needle = normalized(dependency_key);
    let count: i64 = connection.query_row(
        "SELECT COUNT(*)
         FROM files
         WHERE source_location <> 'downloads'
           AND LOWER(filename) LIKE ?1",
        params![format!("%{needle}%")],
        |row| row.get(0),
    )?;
    Ok(count > 0)
}

fn dependency_in_inbox(
    connection: &Connection,
    item_id: i64,
    dependency_key: &str,
) -> AppResult<Option<InboxDependencyItem>> {
    connection
        .query_row(
            "SELECT
                id,
                display_name,
                intake_mode,
                guided_install_available
         FROM download_items
         WHERE id <> ?1
           AND matched_profile_key = ?2
           AND status NOT IN ('applied', 'ignored', 'error')
         ORDER BY
            CASE intake_mode
                WHEN 'guided' THEN 0
                WHEN 'needs_review' THEN 1
                WHEN 'blocked' THEN 2
                ELSE 3
            END,
            updated_at DESC
         LIMIT 1",
            params![item_id, dependency_key],
            |row| {
                let intake_mode: String = row.get(2)?;
                Ok(InboxDependencyItem {
                    id: row.get(0)?,
                    display_name: row.get(1)?,
                    intake_mode: match intake_mode.as_str() {
                        "guided" => DownloadIntakeMode::Guided,
                        "needs_review" => DownloadIntakeMode::NeedsReview,
                        "blocked" => DownloadIntakeMode::Blocked,
                        _ => DownloadIntakeMode::Standard,
                    },
                    guided_install_available: row.get::<_, i64>(3)? != 0,
                })
            },
        )
        .optional()
        .map_err(AppError::from)
}

fn detect_existing_layout(
    connection: &Connection,
    settings: &LibrarySettings,
    seed_pack: &SeedPack,
    profile: &GuidedInstallProfileSeed,
) -> AppResult<ExistingInstallLayout> {
    let started_at = Instant::now();
    let mods_root = settings
        .mods_path
        .as_ref()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .ok_or_else(|| {
            AppError::Message("Set a Mods folder before using special installs.".to_owned())
        })?;
    let default_target_folder = mods_root.join(&profile.install_folder_name);
    let mut warnings = Vec::new();
    let mut safe_to_update = true;
    let mut existing_install_detected = false;
    let mut scattered_match_count = 0_i64;
    let mut scattered_preserve_count = 0_i64;

    let like_root = format!("{}%", mods_root.to_string_lossy());
    let mut statement = connection.prepare(
        "SELECT
            f.id,
            f.filename,
            f.path,
            f.extension,
            f.kind,
            f.subtype,
            c.canonical_name,
            f.size,
            f.hash,
            COALESCE(f.insights, '{}')
         FROM files f
         LEFT JOIN creators c ON c.id = f.creator_id
         WHERE f.source_location <> 'downloads'
           AND f.path LIKE ?1",
    )?;
    let installed_files = statement
        .query_map(params![like_root], |row| {
            Ok(ExistingInstallFile {
                file_id: Some(row.get(0)?),
                filename: row.get(1)?,
                path: row.get(2)?,
                extension: row.get(3)?,
                kind: row.get(4)?,
                subtype: row.get(5)?,
                creator: row.get(6)?,
                size: row.get(7)?,
                hash: row.get(8)?,
                insights: serde_json::from_str(&row.get::<_, String>(9)?).unwrap_or_default(),
                in_target_folder: false,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    let mut existing_candidates = Vec::new();
    let mut preserve_candidates = Vec::new();
    for file in installed_files {
        if !Path::new(&file.path).exists() {
            continue;
        }

        if is_existing_profile_file(&file, profile) {
            existing_candidates.push(file);
        } else if matches_preserve_rule(&file.filename, &file.extension, profile) {
            preserve_candidates.push(file);
        }
    }

    merge_disk_existing_candidates(
        &mods_root,
        profile,
        &mut existing_candidates,
        &mut preserve_candidates,
    )?;

    for file in &mut existing_candidates {
        refresh_existing_install_file_insights_if_needed(connection, seed_pack, file);
    }
    for file in &mut preserve_candidates {
        refresh_existing_install_file_insights_if_needed(connection, seed_pack, file);
    }

    let target_folder = select_existing_target_folder(
        &mods_root,
        &default_target_folder,
        &existing_candidates,
        &preserve_candidates,
        profile,
    );
    let shared_root_target = target_folder == mods_root;

    let mut existing_files = Vec::new();
    let mut preserve_files = Vec::new();
    for mut file in existing_candidates {
        existing_install_detected = true;
        let in_target =
            is_in_selected_special_folder(&file.path, &target_folder, shared_root_target);
        file.in_target_folder = in_target;
        if !in_target {
            safe_to_update = false;
            scattered_match_count += 1;
        }
        existing_files.push(file);
    }

    for mut file in preserve_candidates {
        let in_target =
            is_in_selected_special_folder(&file.path, &target_folder, shared_root_target);
        if is_related_preserve_file(&file.filename, &file.extension, in_target, profile) {
            existing_install_detected = true;
            file.in_target_folder = in_target;
            if !in_target {
                safe_to_update = false;
                scattered_preserve_count += 1;
            }
            preserve_files.push(file);
        }
    }

    let known_preserve_paths = preserve_files
        .iter()
        .map(|file| normalize_path_key(&file.path))
        .collect::<HashSet<_>>();
    for preserve_path in scan_preserve_files(&target_folder, profile, shared_root_target)? {
        let preserve_path_string = preserve_path.to_string_lossy().to_string();
        if known_preserve_paths.contains(&normalize_path_key(&preserve_path_string)) {
            continue;
        }

        existing_install_detected = true;
        preserve_files.push(ExistingInstallFile {
            file_id: None,
            filename: preserve_path
                .file_name()
                .map(|value| value.to_string_lossy().to_string())
                .unwrap_or_else(|| preserve_path_string.clone()),
            path: preserve_path_string,
            extension: normalize_extension(&preserve_path),
            kind: "Config".to_owned(),
            subtype: None,
            creator: profile.creator.clone(),
            size: preserve_path
                .metadata()
                .map(|meta| meta.len() as i64)
                .unwrap_or_default(),
            hash: None,
            insights: FileInsights::default(),
            in_target_folder: true,
        });
    }

    if scattered_match_count > 0 || scattered_preserve_count > 0 {
        warnings.push(format!(
            "{} already has files outside the expected {} folder.",
            profile.display_name, profile.install_folder_name
        ));
    }

    let foreign_target_files =
        scan_foreign_target_files(&target_folder, profile, shared_root_target)?;
    if !foreign_target_files.is_empty() {
        safe_to_update = false;
        warnings.push(format!(
            "The target {} folder already contains files that do not belong to {}.",
            profile.install_folder_name, profile.display_name
        ));
    }

    let repair_plan_available = (scattered_match_count > 0 || scattered_preserve_count > 0)
        && foreign_target_files.is_empty();

    let layout = ExistingInstallLayout {
        target_folder,
        existing_files,
        preserve_files,
        warnings,
        existing_install_detected,
        safe_to_update,
        repair_plan_available,
    };
    log_slow_install_profile_step("detect_existing_layout", started_at, || {
        format!(
            "for profile {} existing={} preserve={} safe={} repairable={}",
            profile.key,
            layout.existing_files.len(),
            layout.preserve_files.len(),
            layout.safe_to_update,
            layout.repair_plan_available
        )
    });
    Ok(layout)
}

fn merge_disk_existing_candidates(
    mods_root: &Path,
    profile: &GuidedInstallProfileSeed,
    existing_candidates: &mut Vec<ExistingInstallFile>,
    preserve_candidates: &mut Vec<ExistingInstallFile>,
) -> AppResult<()> {
    let known_existing_paths = existing_candidates
        .iter()
        .map(|file| normalize_path_key(&file.path))
        .collect::<HashSet<_>>();
    let known_preserve_paths = preserve_candidates
        .iter()
        .map(|file| normalize_path_key(&file.path))
        .collect::<HashSet<_>>();

    for entry in WalkDir::new(mods_root)
        .max_depth(profile.max_install_depth + 4)
        .into_iter()
        .filter_map(Result::ok)
    {
        if !entry.file_type().is_file() {
            continue;
        }

        let path = entry.path();
        let filename = entry.file_name().to_string_lossy().to_string();
        let extension = normalize_extension(path);
        if !matches!(extension.as_str(), ".ts4script" | ".package" | ".cfg") {
            continue;
        }

        let path_string = path.to_string_lossy().to_string();
        let normalized_path = normalize_path_key(&path_string);

        if is_existing_profile_filename(&filename, &extension, profile) {
            if known_existing_paths.contains(&normalized_path) {
                continue;
            }

            existing_candidates.push(ExistingInstallFile {
                file_id: None,
                filename,
                path: path_string,
                extension: extension.clone(),
                kind: if extension == ".ts4script" {
                    "Script Mods".to_owned()
                } else {
                    "Mods".to_owned()
                },
                subtype: None,
                creator: profile.creator.clone(),
                size: path
                    .metadata()
                    .map(|meta| meta.len() as i64)
                    .unwrap_or_default(),
                hash: None,
                insights: FileInsights::default(),
                in_target_folder: false,
            });
            continue;
        }

        if matches_preserve_rule(&filename, &extension, profile)
            && !known_preserve_paths.contains(&normalized_path)
        {
            preserve_candidates.push(ExistingInstallFile {
                file_id: None,
                filename,
                path: path_string,
                extension,
                kind: "Config".to_owned(),
                subtype: None,
                creator: profile.creator.clone(),
                size: path
                    .metadata()
                    .map(|meta| meta.len() as i64)
                    .unwrap_or_default(),
                hash: None,
                insights: FileInsights::default(),
                in_target_folder: false,
            });
        }
    }

    Ok(())
}

fn select_existing_target_folder(
    mods_root: &Path,
    default_target_folder: &Path,
    existing_candidates: &[ExistingInstallFile],
    preserve_candidates: &[ExistingInstallFile],
    profile: &GuidedInstallProfileSeed,
) -> PathBuf {
    let mut selected_folder: Option<PathBuf> = None;

    for file in existing_candidates.iter().chain(
        preserve_candidates
            .iter()
            .filter(|file| file_matches_profile_prefix(&file.filename, profile)),
    ) {
        let Some(parent) = Path::new(&file.path).parent() else {
            return default_target_folder.to_path_buf();
        };
        let Ok(relative) = parent.strip_prefix(mods_root) else {
            return default_target_folder.to_path_buf();
        };
        let relative_depth = relative.components().count();
        if relative_depth > profile.max_install_depth {
            return default_target_folder.to_path_buf();
        }
        if relative_depth == 0 && !profile.allow_root_install {
            return default_target_folder.to_path_buf();
        }

        match &selected_folder {
            Some(folder) if folder != parent => return default_target_folder.to_path_buf(),
            Some(_) => {}
            None => selected_folder = Some(parent.to_path_buf()),
        }
    }

    selected_folder.unwrap_or_else(|| default_target_folder.to_path_buf())
}

fn is_in_selected_special_folder(
    path: &str,
    target_folder: &Path,
    shared_root_target: bool,
) -> bool {
    let Some(parent) = Path::new(path).parent() else {
        return false;
    };

    if shared_root_target {
        return parent == target_folder;
    }

    parent == target_folder
}

fn load_item_with_staging(
    connection: &Connection,
    item_id: i64,
) -> AppResult<Option<DownloadItemRecord>> {
    connection
        .query_row(
            "SELECT id, display_name, source_path, staging_path
             FROM download_items
             WHERE id = ?1",
            params![item_id],
            |row| {
                Ok(DownloadItemRecord {
                    id: row.get(0)?,
                    display_name: row.get(1)?,
                    source_path: row.get(2)?,
                    staging_path: row.get(3)?,
                })
            },
        )
        .optional()
        .map_err(Into::into)
}

fn load_profile_files(
    connection: &Connection,
    seed_pack: &SeedPack,
    item_id: i64,
    active_only: bool,
) -> AppResult<Vec<ProfileFile>> {
    let started_at = Instant::now();
    let location_filter = if active_only {
        "AND f.source_location = 'downloads'"
    } else {
        ""
    };
    let sql = format!(
        "SELECT
            f.id,
            f.filename,
            f.path,
            f.archive_member_path,
            f.extension,
            f.kind,
            f.subtype,
            c.canonical_name,
            f.confidence,
            f.source_location,
            f.size,
            f.hash,
            COALESCE(f.insights, '{{}}')
         FROM files f
         LEFT JOIN creators c ON c.id = f.creator_id
         WHERE f.download_item_id = ?1
           {location_filter}
         ORDER BY f.filename COLLATE NOCASE"
    );
    let mut statement = connection.prepare(&sql)?;
    let mut rows = statement
        .query_map(params![item_id], |row| {
            Ok(ProfileFile {
                file_id: row.get(0)?,
                filename: row.get(1)?,
                path: row.get(2)?,
                archive_member_path: row.get(3)?,
                extension: row.get(4)?,
                kind: row.get(5)?,
                subtype: row.get(6)?,
                creator: row.get(7)?,
                confidence: row.get(8)?,
                source_location: row.get(9)?,
                size: row.get(10)?,
                hash: row.get(11)?,
                insights: serde_json::from_str(&row.get::<_, String>(12)?).unwrap_or_default(),
            })
        })?
        .collect::<Result<Vec<_>, _>>()
        .map_err(AppError::from)?;
    for file in &mut rows {
        refresh_profile_file_insights_if_needed(connection, seed_pack, file);
    }
    log_slow_install_profile_step("load_profile_files", started_at, || {
        format!(
            "for item {} loaded {} {} file(s)",
            item_id,
            rows.len(),
            if active_only { "active" } else { "family" }
        )
    });
    Ok(rows)
}

fn read_text_clues(staging_path: Option<&str>, files: &[ProfileFile]) -> AppResult<Vec<String>> {
    let mut clues = Vec::new();

    for file in files {
        clues.push(normalized(&file.filename));
        if let Some(member_path) = &file.archive_member_path {
            clues.push(normalized(member_path));
        }
        clues.extend(
            file.insights
                .embedded_names
                .iter()
                .map(|value| normalized(value)),
        );
        clues.extend(
            file.insights
                .resource_summary
                .iter()
                .map(|value| normalized(value)),
        );
        clues.extend(
            file.insights
                .creator_hints
                .iter()
                .map(|value| normalized(value)),
        );
        clues.extend(
            file.insights
                .script_namespaces
                .iter()
                .map(|value| normalized(value)),
        );
        clues.extend(
            file.insights
                .family_hints
                .iter()
                .map(|value| normalized(value)),
        );
        clues.extend(
            file.insights
                .version_hints
                .iter()
                .map(|value| normalized(value)),
        );
    }

    if let Some(staging_path) = staging_path {
        let root = PathBuf::from(staging_path);
        if root.exists() {
            for entry in WalkDir::new(&root)
                .max_depth(4)
                .into_iter()
                .filter_map(Result::ok)
                .filter(|entry| entry.file_type().is_file())
            {
                let extension = normalize_extension(entry.path());
                if !matches!(extension.as_str(), ".txt" | ".md" | ".rtf") {
                    continue;
                }

                if let Ok(text) = fs::read_to_string(entry.path()) {
                    if !text.trim().is_empty() {
                        clues.push(normalized(&text));
                    }
                }
            }
        }
    }

    Ok(clues)
}

fn collect_archive_path_clues(files: &[ProfileFile]) -> Vec<String> {
    let mut clues = Vec::new();
    for file in files {
        if let Some(member_path) = &file.archive_member_path {
            clues.push(normalized(member_path));
        }
        clues.push(normalized(&file.path));
    }
    clues
}

fn has_required_core(files: &[ProfileFile], profile: &GuidedInstallProfileSeed) -> bool {
    files.iter().any(|file| {
        matches_required_core(
            &file.filename,
            &file.extension,
            &profile.required_name_clues,
        )
    })
}

fn profile_pack_is_complete(files: &[ProfileFile], profile: &GuidedInstallProfileSeed) -> bool {
    let evidence = collect_profile_evidence(profile, &empty_item_record(), files, &[], &[]);
    evidence.strong_match(profile)
}

fn collect_extra_supported_files<'a>(
    files: &'a [ProfileFile],
    profile: &GuidedInstallProfileSeed,
) -> Vec<&'a ProfileFile> {
    files
        .iter()
        .filter(|file| is_supported_special_extension(&file.extension))
        .filter(|file| !is_profile_content_file(file, profile))
        .collect()
}

fn has_supported_subset_to_separate(
    files: &[ProfileFile],
    profile: &GuidedInstallProfileSeed,
) -> bool {
    files
        .iter()
        .any(|file| is_profile_content_file(file, profile))
        && profile_pack_is_complete(files, profile)
        && !collect_extra_supported_files(files, profile).is_empty()
}

fn is_profile_content_file(file: &ProfileFile, profile: &GuidedInstallProfileSeed) -> bool {
    is_profile_content_name(&file.filename, &file.extension, profile)
}

fn is_existing_profile_file(
    file: &ExistingInstallFile,
    profile: &GuidedInstallProfileSeed,
) -> bool {
    is_profile_content_name(&file.filename, &file.extension, profile)
}

fn is_related_preserve_file(
    filename: &str,
    extension: &str,
    in_target_folder: bool,
    profile: &GuidedInstallProfileSeed,
) -> bool {
    if !matches_preserve_rule(filename, extension, profile) {
        return false;
    }

    in_target_folder
        || name_matches_profile(filename, profile)
        || file_matches_profile_prefix(filename, profile)
}

fn is_profile_content_name(
    filename: &str,
    extension: &str,
    profile: &GuidedInstallProfileSeed,
) -> bool {
    if !is_supported_special_extension(extension) {
        return false;
    }

    let normalized_name = normalized(filename);
    let prefix_match = if extension == ".ts4script" {
        profile
            .script_prefixes
            .iter()
            .any(|prefix| normalized_name.starts_with(&normalized(prefix)))
    } else {
        profile
            .package_prefixes
            .iter()
            .any(|prefix| normalized_name.starts_with(&normalized(prefix)))
    };

    prefix_match || name_matches_profile(filename, profile)
}

fn guided_validation_kind(file: &ProfileFile, profile: &GuidedInstallProfileSeed) -> String {
    let original_kind = file.kind.trim();
    if !original_kind.is_empty() && original_kind != "Unknown" {
        return file.kind.clone();
    }

    if is_profile_content_file(file, profile) {
        if file.extension == ".ts4script" {
            return "Script Mods".to_owned();
        }
        if file.extension == ".package" {
            return "Mods".to_owned();
        }
    }

    file.kind.clone()
}

fn file_matches_profile_prefix(filename: &str, profile: &GuidedInstallProfileSeed) -> bool {
    let normalized_name = normalized(filename);
    profile
        .script_prefixes
        .iter()
        .chain(profile.package_prefixes.iter())
        .any(|prefix| normalized_name.starts_with(&normalized(prefix)))
}

fn is_existing_profile_filename(
    filename: &str,
    extension: &str,
    profile: &GuidedInstallProfileSeed,
) -> bool {
    if matches_preserve_rule(filename, extension, profile) {
        return false;
    }

    matches_required_core(filename, extension, &profile.required_name_clues)
        || file_matches_profile_prefix(filename, profile)
        || profile
            .required_all_filenames
            .iter()
            .any(|required| normalized(required) == normalized(filename))
}

fn name_matches_profile(filename: &str, profile: &GuidedInstallProfileSeed) -> bool {
    let normalized_name = normalized(filename);
    clue_list_matches(&normalized_name, &profile.required_name_clues)
        || clue_list_matches(&normalized_name, &profile.name_clues)
        || clue_list_matches(&normalized_name, &profile.sample_filenames)
}

fn matches_required_core(filename: &str, extension: &str, required_name_clues: &[String]) -> bool {
    if !is_supported_special_extension(extension) {
        return false;
    }
    let normalized_name = normalized(filename);
    clue_list_matches(&normalized_name, required_name_clues)
}

fn matches_preserve_rule(
    filename: &str,
    extension: &str,
    profile: &GuidedInstallProfileSeed,
) -> bool {
    profile
        .preserve_extensions
        .iter()
        .any(|value| normalize_extension_str(value) == extension)
        || profile
            .preserve_prefixes
            .iter()
            .any(|prefix| normalized(filename).starts_with(&normalized(prefix)))
}

fn matches_dependency_rule(
    rule: &DependencyRuleSeed,
    item: &DownloadItemRecord,
    files: &[ProfileFile],
    text_clues: &[String],
    archive_path_clues: &[String],
) -> bool {
    let inputs = collect_item_inputs(item, files);
    inputs
        .iter()
        .any(|input| clue_list_matches(input, &rule.name_clues))
        || !collect_matched_clues(text_clues, &rule.text_clues).is_empty()
        || !collect_matched_clues(archive_path_clues, &rule.archive_path_clues).is_empty()
}

fn matches_incompatibility_rule(
    rule: &IncompatibilityRuleSeed,
    item: &DownloadItemRecord,
    files: &[ProfileFile],
    text_clues: &[String],
    archive_path_clues: &[String],
) -> bool {
    let inputs = collect_item_inputs(item, files);
    inputs
        .iter()
        .any(|input| clue_list_matches(input, &rule.name_clues))
        || !collect_matched_clues(text_clues, &rule.text_clues).is_empty()
        || !collect_matched_clues(archive_path_clues, &rule.archive_path_clues).is_empty()
}

fn collect_item_inputs(item: &DownloadItemRecord, files: &[ProfileFile]) -> Vec<String> {
    let mut inputs = vec![
        normalized(&item.display_name),
        normalized(&item.source_path),
    ];
    for file in files {
        inputs.push(normalized(&file.filename));
        if let Some(member_path) = &file.archive_member_path {
            inputs.push(normalized(member_path));
        }
    }
    inputs
}

fn empty_item_record() -> DownloadItemRecord {
    DownloadItemRecord {
        id: 0,
        display_name: String::new(),
        source_path: String::new(),
        staging_path: None,
    }
}

fn build_evidence_summary(evidence: &ProfileEvidence) -> Vec<String> {
    let mut summary = Vec::new();
    if evidence.matched_files > 0 {
        summary.push(format!(
            "{} matching special-mod files were detected.",
            evidence.matched_files
        ));
    }
    if evidence.matched_script_files > 0 {
        summary.push(format!(
            "{} matching script file(s) were detected.",
            evidence.matched_script_files
        ));
    }
    if evidence.matched_package_files > 0 {
        summary.push(format!(
            "{} matching package file(s) were detected.",
            evidence.matched_package_files
        ));
    }
    if evidence.required_core_present {
        summary.push("Required core files were found.".to_owned());
    }
    if evidence.required_exact_filenames_found > 0 {
        summary.push(format!(
            "{} exact required file name(s) were found.",
            evidence.required_exact_filenames_found
        ));
    }
    if !evidence.text_matches.is_empty() {
        summary.push(format!(
            "Readme or embedded text mentions {}.",
            evidence.text_matches.join(", ")
        ));
    }
    if !evidence.archive_matches.is_empty() {
        summary.push(format!(
            "Folder names mention {}.",
            evidence.archive_matches.join(", ")
        ));
    }
    if evidence.unmatched_supported_files > 0 {
        summary.push(format!(
            "{} extra supported files did not match the catalog entry.",
            evidence.unmatched_supported_files
        ));
    }
    summary
}

fn dependency_rule_for_key<'a>(
    seed_pack: &'a SeedPack,
    dependency_key: &str,
) -> Option<&'a DependencyRuleSeed> {
    seed_pack
        .install_catalog
        .dependency_rules
        .iter()
        .find(|rule| rule.dependency_key == dependency_key)
}

fn build_available_review_actions(
    seed_pack: &SeedPack,
    evaluation: &EvaluationResult,
    files: &[ProfileFile],
    repair_layout: Option<&ExistingInstallLayout>,
) -> Vec<ReviewPlanAction> {
    let mut actions = Vec::new();

    if let (Some(profile), Some(layout)) = (evaluation.matched_profile.as_ref(), repair_layout) {
        if layout.repair_plan_available {
            actions.push(ReviewPlanAction {
                kind: ReviewPlanActionKind::RepairSpecial,
                label: format!("Fix old {} setup", profile.display_name),
                description: format!(
                    "Move the older {} files out of the way, keep the settings files safe, and continue the update.",
                    profile.display_name
                ),
                priority: 100,
                related_item_id: None,
                related_item_name: Some(profile.display_name.clone()),
                url: None,
            });
        }

        if has_supported_subset_to_separate(files, profile) {
            let extra_supported = collect_extra_supported_files(files, profile);
            actions.push(ReviewPlanAction {
                kind: ReviewPlanActionKind::SeparateSupportedFiles,
                label: format!("Separate the {} files", profile.display_name),
                description: format!(
                    "Pull the clean {} files into their own batch and leave {} extra script or package file(s) behind for review.",
                    profile.display_name,
                    extra_supported.len()
                ),
                priority: 95,
                related_item_id: None,
                related_item_name: Some(profile.display_name.clone()),
                url: None,
            });
        }

        let has_profile_files = files
            .iter()
            .any(|file| is_profile_content_file(file, profile));
        if has_profile_files && !has_required_core(files, profile) {
            if let Some(download_url) = profile.official_download_url.clone() {
                actions.push(ReviewPlanAction {
                    kind: ReviewPlanActionKind::DownloadMissingFiles,
                    label: format!("Download missing {} files", profile.display_name),
                    description: format!(
                        "Fetch the official {} archive into the Inbox first, then re-check it before installing anything.",
                        profile.display_name
                    ),
                    priority: 92,
                    related_item_id: None,
                    related_item_name: Some(profile.display_name.clone()),
                    url: Some(download_url),
                });
            } else {
                actions.push(ReviewPlanAction {
                    kind: ReviewPlanActionKind::OpenOfficialSource,
                    label: format!("Open the {} page", profile.display_name),
                    description: format!(
                        "Open the official {} page so you can grab the full archive yourself.",
                        profile.display_name
                    ),
                    priority: 88,
                    related_item_id: None,
                    related_item_name: Some(profile.display_name.clone()),
                    url: Some(profile.official_source_url.clone()),
                });
            }
        }
    }

    for dependency in &evaluation.dependencies {
        if dependency.status == "inbox" {
            if dependency.inbox_item_intake_mode == Some(DownloadIntakeMode::Guided)
                && dependency.inbox_item_guided_install_available
            {
                actions.push(ReviewPlanAction {
                    kind: ReviewPlanActionKind::InstallDependency,
                    label: format!("Install {} first", dependency.display_name),
                    description: format!(
                        "Set up {} first, then let SimSuite come back and re-check this inbox item.",
                        dependency.display_name
                    ),
                    priority: 90,
                    related_item_id: dependency.inbox_item_id,
                    related_item_name: dependency.inbox_item_name.clone(),
                    url: None,
                });
            } else {
                actions.push(ReviewPlanAction {
                    kind: ReviewPlanActionKind::OpenDependency,
                    label: format!("Open {}", dependency.display_name),
                    description: format!(
                        "Jump to the {} inbox item so you can sort its setup first.",
                        dependency.display_name
                    ),
                    priority: 84,
                    related_item_id: dependency.inbox_item_id,
                    related_item_name: dependency.inbox_item_name.clone(),
                    url: None,
                });
            }
        }

        if dependency.status == "missing" {
            if let Some(rule) = dependency_rule_for_key(seed_pack, &dependency.key) {
                if let Some(download_url) = rule.official_download_url.clone() {
                    actions.push(ReviewPlanAction {
                        kind: ReviewPlanActionKind::DownloadMissingFiles,
                        label: format!("Download {}", dependency.display_name),
                        description: format!(
                            "Fetch the official {} file into the Inbox, then re-check this download.",
                            dependency.display_name
                        ),
                        priority: 89,
                        related_item_id: None,
                        related_item_name: Some(dependency.display_name.clone()),
                        url: Some(download_url),
                    });
                } else {
                    actions.push(ReviewPlanAction {
                        kind: ReviewPlanActionKind::OpenOfficialSource,
                        label: format!("Open {} page", dependency.display_name),
                        description: format!(
                            "Open the official {} page so you can grab the required library.",
                            dependency.display_name
                        ),
                        priority: 82,
                        related_item_id: None,
                        related_item_name: Some(dependency.display_name.clone()),
                        url: Some(rule.official_source_url.clone()),
                    });
                }
            }
        }
    }

    if actions.is_empty() {
        if let Some(profile) = evaluation.matched_profile.as_ref() {
            actions.push(ReviewPlanAction {
                kind: ReviewPlanActionKind::OpenOfficialSource,
                label: format!("Open the {} page", profile.display_name),
                description: format!(
                    "Open the official {} page for the install notes and the latest download.",
                    profile.display_name
                ),
                priority: 60,
                related_item_id: None,
                related_item_name: Some(profile.display_name.clone()),
                url: Some(profile.official_source_url.clone()),
            });
        } else if let Some(pattern) = evaluation.matched_pattern.as_ref() {
            if let Some(url) = pattern.official_source_url.clone() {
                actions.push(ReviewPlanAction {
                    kind: ReviewPlanActionKind::OpenOfficialSource,
                    label: format!("Open {}", pattern.display_name),
                    description: pattern.help_summary.clone(),
                    priority: 50,
                    related_item_id: None,
                    related_item_name: Some(pattern.display_name.clone()),
                    url: Some(url),
                });
            }
        }
    }

    actions.sort_by(|left, right| right.priority.cmp(&left.priority));
    actions
}

fn scan_preserve_files(
    target_folder: &Path,
    profile: &GuidedInstallProfileSeed,
    shared_root_target: bool,
) -> AppResult<Vec<PathBuf>> {
    if !target_folder.exists() {
        return Ok(Vec::new());
    }

    let mut preserve_paths = Vec::new();
    if shared_root_target {
        for entry in fs::read_dir(target_folder)? {
            let entry = entry?;
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let filename = entry.file_name().to_string_lossy().to_string();
            let extension = normalize_extension(&path);
            if matches_preserve_rule(&filename, &extension, profile)
                && file_matches_profile_prefix(&filename, profile)
            {
                preserve_paths.push(path);
            }
        }
        return Ok(preserve_paths);
    }

    for entry in WalkDir::new(target_folder)
        .max_depth(profile.max_install_depth + 1)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
    {
        let filename = entry.file_name().to_string_lossy().to_string();
        let extension = normalize_extension(entry.path());
        if matches_preserve_rule(&filename, &extension, profile) {
            preserve_paths.push(entry.path().to_path_buf());
        }
    }
    Ok(preserve_paths)
}

fn scan_foreign_target_files(
    target_folder: &Path,
    profile: &GuidedInstallProfileSeed,
    shared_root_target: bool,
) -> AppResult<Vec<PathBuf>> {
    if !target_folder.exists() {
        return Ok(Vec::new());
    }

    if shared_root_target {
        return Ok(Vec::new());
    }

    let mut foreign = Vec::new();
    for entry in WalkDir::new(target_folder)
        .max_depth(profile.max_install_depth + 1)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
    {
        let filename = entry.file_name().to_string_lossy().to_string();
        let extension = normalize_extension(entry.path());
        if !is_supported_special_extension(&extension) {
            continue;
        }
        if !is_profile_content_name(&filename, &extension, profile)
            && !matches_preserve_rule(&filename, &extension, profile)
        {
            foreign.push(entry.path().to_path_buf());
        }
    }
    Ok(foreign)
}

fn normalize_extension(path: &Path) -> String {
    path.extension()
        .map(|value| format!(".{}", value.to_string_lossy().to_ascii_lowercase()))
        .unwrap_or_default()
}

fn normalize_extension_str(value: &str) -> String {
    if value.starts_with('.') {
        value.to_ascii_lowercase()
    } else {
        format!(".{}", value.to_ascii_lowercase())
    }
}

fn is_supported_special_extension(extension: &str) -> bool {
    matches!(extension, ".package" | ".ts4script")
}

fn profile_catalog_source(profile: &GuidedInstallProfileSeed) -> CatalogSourceInfo {
    CatalogSourceInfo {
        official_source_url: Some(profile.official_source_url.clone()),
        official_download_url: profile.official_download_url.clone(),
        latest_check_url: profile.latest_check_url.clone(),
        latest_check_strategy: profile.latest_check_strategy.clone(),
        reference_source: profile.reference_source.clone(),
        reviewed_at: Some(profile.reviewed_at.clone()),
    }
}

fn pattern_catalog_source(pattern: &ReviewOnlyPatternSeed) -> CatalogSourceInfo {
    CatalogSourceInfo {
        official_source_url: pattern.official_source_url.clone(),
        official_download_url: None,
        latest_check_url: None,
        latest_check_strategy: None,
        reference_source: pattern.reference_source.clone(),
        reviewed_at: Some(pattern.reviewed_at.clone()),
    }
}

fn dependency_catalog_source(rule: &DependencyRuleSeed) -> CatalogSourceInfo {
    CatalogSourceInfo {
        official_source_url: Some(rule.official_source_url.clone()),
        official_download_url: rule.official_download_url.clone(),
        latest_check_url: None,
        latest_check_strategy: None,
        reference_source: rule.reference_source.clone(),
        reviewed_at: Some(rule.reviewed_at.clone()),
    }
}

fn incompatibility_catalog_source(rule: &IncompatibilityRuleSeed) -> CatalogSourceInfo {
    CatalogSourceInfo {
        official_source_url: Some(rule.official_source_url.clone()),
        official_download_url: None,
        latest_check_url: None,
        latest_check_strategy: None,
        reference_source: rule.reference_source.clone(),
        reviewed_at: Some(rule.reviewed_at.clone()),
    }
}

fn profile_review_message(profile: &GuidedInstallProfileSeed) -> String {
    profile.review_reasons.first().cloned().unwrap_or_else(|| {
        format!(
            "SimSuite is not fully certain that this is a complete {} set.",
            profile.display_name
        )
    })
}

fn profile_block_reason(
    profile: &GuidedInstallProfileSeed,
    index: usize,
    fallback: &str,
) -> String {
    profile
        .block_reasons
        .get(index)
        .cloned()
        .unwrap_or_else(|| fallback.to_owned())
}

fn collect_matched_clues(inputs: &[String], clues: &[String]) -> Vec<String> {
    let mut matched = Vec::new();
    let mut seen = HashSet::new();
    for clue in clues {
        let normalized_clue = normalized(clue);
        if normalized_clue.is_empty() || !seen.insert(normalized_clue.clone()) {
            continue;
        }
        if inputs.iter().any(|input| input.contains(&normalized_clue)) {
            matched.push(clue.clone());
        }
    }
    matched
}

fn clue_list_matches(input: &str, clues: &[String]) -> bool {
    let normalized_input = normalized(input);
    clues.iter().any(|clue| {
        let normalized_clue = normalized(clue);
        !normalized_clue.is_empty() && normalized_input.contains(&normalized_clue)
    })
}

fn normalized(value: &str) -> String {
    value
        .to_ascii_lowercase()
        .chars()
        .map(|character| match character {
            'a'..='z' | '0'..='9' => character,
            _ => ' ',
        })
        .collect::<String>()
}

fn normalize_path_key(path: &str) -> String {
    path.replace('\\', "/").to_lowercase()
}

fn intake_mode_label(mode: &DownloadIntakeMode) -> &'static str {
    match mode {
        DownloadIntakeMode::Standard => "standard",
        DownloadIntakeMode::Guided => "guided",
        DownloadIntakeMode::NeedsReview => "needs_review",
        DownloadIntakeMode::Blocked => "blocked",
    }
}

fn risk_level_label(level: &DownloadRiskLevel) -> &'static str {
    match level {
        DownloadRiskLevel::Low => "low",
        DownloadRiskLevel::Medium => "medium",
        DownloadRiskLevel::High => "high",
    }
}

#[cfg(test)]
mod tests {
    use super::{
        assess_download_item, build_evidence_summary, build_guided_plan, build_review_plan,
        build_special_mod_decision, normalized, reconcile_special_mod_family,
        store_download_item_assessment,
    };
    use crate::{
        database::{initialize, save_library_paths, seed_database},
        models::{
            DownloadIntakeMode, DownloadQueueLane, FileInsights, LibrarySettings,
            ReviewPlanActionKind, SpecialDecisionState, SpecialFamilyRole, SpecialVersionStatus,
        },
        seed::{load_seed_pack, GuidedInstallProfileSeed, SeedPack},
    };
    use chrono::Utc;
    use rusqlite::{params, Connection};
    use serde_json::to_string;
    use std::{
        fs,
        fs::File,
        io::Write,
        path::{Path, PathBuf},
    };
    use tempfile::tempdir;
    use zip::write::SimpleFileOptions;

    fn setup_env() -> (tempfile::TempDir, Connection, SeedPack, LibrarySettings) {
        let temp = tempdir().expect("tempdir");
        let mods = temp.path().join("Mods");
        let tray = temp.path().join("Tray");
        let downloads = temp.path().join("Downloads");
        fs::create_dir_all(&mods).expect("mods");
        fs::create_dir_all(&tray).expect("tray");
        fs::create_dir_all(&downloads).expect("downloads");

        let mut connection = Connection::open_in_memory().expect("in-memory");
        initialize(&mut connection).expect("schema");
        let seed_pack = load_seed_pack().expect("seed");
        seed_database(&mut connection, &seed_pack).expect("seed db");
        let settings = LibrarySettings {
            mods_path: Some(mods.to_string_lossy().to_string()),
            tray_path: Some(tray.to_string_lossy().to_string()),
            downloads_path: Some(downloads.to_string_lossy().to_string()),
        };
        save_library_paths(&mut connection, &settings).expect("paths");

        (temp, connection, seed_pack, settings)
    }

    fn insert_download_item(
        connection: &Connection,
        item_id: i64,
        display_name: &str,
        staging_path: &Path,
    ) {
        connection
            .execute(
                "INSERT INTO download_items (
                    id,
                    source_path,
                    display_name,
                    source_kind,
                    archive_format,
                    staging_path,
                    source_size,
                    status,
                    notes
                 ) VALUES (?1, ?2, ?3, 'archive', 'zip', ?4, 100, 'pending', '[]')",
                params![
                    item_id,
                    format!("C:/Downloads/{display_name}"),
                    display_name,
                    staging_path.to_string_lossy().to_string()
                ],
            )
            .expect("download item");
    }

    fn insert_download_file(
        connection: &Connection,
        item_id: i64,
        file_id: i64,
        path: &Path,
        archive_member_path: &str,
        kind: &str,
    ) {
        let extension = path
            .extension()
            .map(|value| format!(".{}", value.to_string_lossy().to_ascii_lowercase()))
            .unwrap_or_default();
        let filename = path
            .file_name()
            .map(|value| value.to_string_lossy().to_string())
            .unwrap_or_else(|| "file".to_owned());

        connection
            .execute(
                "INSERT INTO files (
                    id,
                    path,
                    filename,
                    extension,
                    kind,
                    confidence,
                    source_location,
                    download_item_id,
                    parser_warnings,
                    source_origin_path,
                    archive_member_path,
                    insights
                 ) VALUES (?1, ?2, ?3, ?4, ?5, 0.94, 'downloads', ?6, '[]', ?2, ?7, '{}')",
                params![
                    file_id,
                    path.to_string_lossy().to_string(),
                    filename,
                    extension,
                    kind,
                    item_id,
                    archive_member_path
                ],
            )
            .expect("download file");
    }

    fn insert_installed_file(connection: &Connection, path: &Path, kind: &str) {
        let extension = path
            .extension()
            .map(|value| format!(".{}", value.to_string_lossy().to_ascii_lowercase()))
            .unwrap_or_default();
        let filename = path
            .file_name()
            .map(|value| value.to_string_lossy().to_string())
            .unwrap_or_else(|| "file".to_owned());
        connection
            .execute(
                "INSERT INTO files (
                    path,
                    filename,
                    extension,
                    kind,
                    confidence,
                    source_location,
                    parser_warnings,
                    insights
                 ) VALUES (?1, ?2, ?3, ?4, 0.95, 'mods', '[]', '{}')",
                params![
                    path.to_string_lossy().to_string(),
                    filename,
                    extension,
                    kind
                ],
            )
            .expect("installed file");
    }

    fn write_script_archive(path: &Path, entry_name: &str, contents: &[u8]) {
        let file = File::create(path).expect("archive file");
        let mut writer = zip::ZipWriter::new(file);
        writer
            .start_file(entry_name, SimpleFileOptions::default())
            .expect("start archive file");
        writer.write_all(contents).expect("write archive file");
        writer.finish().expect("finish archive file");
    }

    fn build_sample_download(
        root: &Path,
        connection: &Connection,
        item_id: i64,
        profile: &GuidedInstallProfileSeed,
    ) {
        let staging = root.join(item_id.to_string());
        fs::create_dir_all(&staging).expect("staging");
        insert_download_item(
            connection,
            item_id,
            &format!("{}.zip", profile.display_name.replace(' ', "_")),
            &staging,
        );

        for (index, sample) in profile.sample_filenames.iter().enumerate() {
            let file_path = staging.join(sample);
            if let Some(parent) = file_path.parent() {
                fs::create_dir_all(parent).expect("parents");
            }
            fs::write(&file_path, b"sample").expect("write sample");
            insert_download_file(
                connection,
                item_id,
                item_id * 100 + index as i64 + 1,
                &file_path,
                sample,
                if sample.ends_with(".ts4script") {
                    "Script Mods"
                } else {
                    "Mods"
                },
            );
        }
    }

    fn build_near_miss_download(
        root: &Path,
        connection: &Connection,
        item_id: i64,
        display_name: &str,
        filename: &str,
        kind: &str,
    ) {
        let staging = root.join(format!("near_miss_{item_id}"));
        fs::create_dir_all(&staging).expect("staging");
        insert_download_item(connection, item_id, display_name, &staging);
        let file_path = staging.join(filename);
        fs::write(&file_path, b"sample").expect("write sample");
        insert_download_file(
            connection,
            item_id,
            item_id * 100 + 1,
            &file_path,
            filename,
            kind,
        );
    }

    fn install_target_for_profile(temp_root: &Path, profile: &GuidedInstallProfileSeed) -> PathBuf {
        temp_root.join("Mods").join(&profile.install_folder_name)
    }

    fn insert_family_state(
        connection: &Connection,
        profile_key: &str,
        profile_name: &str,
        install_state: &str,
        install_path: Option<&Path>,
        installed_version: Option<&str>,
        source_item_id: Option<i64>,
    ) {
        connection
            .execute(
                "INSERT INTO special_mod_family_state (
                    profile_key,
                    profile_name,
                    install_state,
                    install_path,
                    installed_version,
                    source_item_id,
                    checked_at,
                    latest_status,
                    updated_at
                 ) VALUES (
                    ?1, ?2, ?3, ?4, ?5, ?6, CURRENT_TIMESTAMP, 'unknown', CURRENT_TIMESTAMP
                 )",
                params![
                    profile_key,
                    profile_name,
                    install_state,
                    install_path.map(|value| value.to_string_lossy().to_string()),
                    installed_version,
                    source_item_id
                ],
            )
            .expect("family state");
    }

    fn update_file_insights_by_id(connection: &Connection, file_id: i64, insights: &FileInsights) {
        connection
            .execute(
                "UPDATE files SET insights = ?2 WHERE id = ?1",
                params![file_id, to_string(insights).expect("insights json")],
            )
            .expect("update insights");
    }

    fn update_file_insights_by_path(connection: &Connection, path: &Path, insights: &FileInsights) {
        connection
            .execute(
                "UPDATE files SET insights = ?2 WHERE path = ?1",
                params![
                    path.to_string_lossy().to_string(),
                    to_string(insights).expect("insights json")
                ],
            )
            .expect("update insights by path");
    }

    #[test]
    fn first_wave_guided_profiles_assess_as_guided() {
        let (_temp, connection, seed_pack, settings) = setup_env();
        let staging_root = PathBuf::from(settings.downloads_path.clone().expect("downloads"));

        for (index, profile) in seed_pack.install_catalog.guided_profiles.iter().enumerate() {
            build_sample_download(&staging_root, &connection, 100 + index as i64, profile);
            let assessment =
                assess_download_item(&connection, &settings, &seed_pack, 100 + index as i64)
                    .expect("assessment");
            assert_eq!(assessment.intake_mode, DownloadIntakeMode::Guided);
        }
    }

    #[test]
    fn first_wave_guided_profiles_build_update_plans() {
        let (temp, connection, seed_pack, settings) = setup_env();
        let staging_root = PathBuf::from(settings.downloads_path.clone().expect("downloads"));

        for (index, profile) in seed_pack.install_catalog.guided_profiles.iter().enumerate() {
            let item_id = 300 + index as i64;
            build_sample_download(&staging_root, &connection, item_id, profile);

            let target = install_target_for_profile(temp.path(), profile);
            fs::create_dir_all(&target).expect("target");
            let existing_sample = profile
                .sample_filenames
                .first()
                .expect("sample filename")
                .replace('/', "\\");
            let existing_path = target.join(existing_sample);
            if let Some(parent) = existing_path.parent() {
                fs::create_dir_all(parent).expect("parent");
            }
            fs::write(&existing_path, b"old").expect("old");
            insert_installed_file(
                &connection,
                &existing_path,
                if existing_path
                    .extension()
                    .and_then(|value| value.to_str())
                    .is_some_and(|value| value.eq_ignore_ascii_case("ts4script"))
                {
                    "Script Mods"
                } else {
                    "Mods"
                },
            );

            if !profile.preserve_extensions.is_empty() {
                let preserve = target.join(format!(
                    "{}_settings{}",
                    profile.key, profile.preserve_extensions[0]
                ));
                fs::write(&preserve, b"keep").expect("preserve");
            }

            let plan = build_guided_plan(&connection, &settings, &seed_pack, item_id)
                .expect("plan")
                .expect("guided");
            assert!(
                plan.apply_ready,
                "expected a ready guided update plan for {}",
                profile.key
            );
            assert!(
                plan.review_files.is_empty(),
                "expected no held files for {}",
                profile.key
            );
            assert!(
                !plan.replace_files.is_empty(),
                "expected replace files for {}",
                profile.key
            );
            if !profile.preserve_extensions.is_empty() {
                assert!(
                    !plan.preserve_files.is_empty(),
                    "expected preserve files for {}",
                    profile.key
                );
            }
        }
    }

    #[test]
    fn first_wave_guided_profiles_allow_clean_root_level_updates() {
        let (temp, connection, seed_pack, settings) = setup_env();
        let staging_root = PathBuf::from(settings.downloads_path.clone().expect("downloads"));

        for (index, profile) in seed_pack.install_catalog.guided_profiles.iter().enumerate() {
            let item_id = 340 + index as i64;
            build_sample_download(&staging_root, &connection, item_id, profile);

            let existing_name =
                Path::new(profile.sample_filenames.first().expect("sample filename"))
                    .file_name()
                    .and_then(|value| value.to_str())
                    .expect("basename");
            let existing_path = temp.path().join("Mods").join(existing_name);
            fs::write(&existing_path, b"old").expect("old");
            insert_installed_file(
                &connection,
                &existing_path,
                if existing_path
                    .extension()
                    .and_then(|value| value.to_str())
                    .is_some_and(|value| value.eq_ignore_ascii_case("ts4script"))
                {
                    "Script Mods"
                } else {
                    "Mods"
                },
            );

            let plan = build_guided_plan(&connection, &settings, &seed_pack, item_id)
                .expect("plan")
                .expect("guided");
            assert!(
                plan.apply_ready,
                "expected a ready root-level guided update plan for {}",
                profile.key
            );
            assert!(
                !plan.replace_files.is_empty(),
                "expected replace files for {}",
                profile.key
            );
        }
    }

    #[test]
    fn creator_named_regular_mods_stay_on_standard_path() {
        let (_temp, connection, seed_pack, settings) = setup_env();
        let staging_root = PathBuf::from(settings.downloads_path.clone().expect("downloads"));

        let near_misses = [
            ("Lot51_Normal_Set.zip", "lot51_table.package", "BuildBuy"),
            (
                "ColonolNutty_Shelf.zip",
                "colonolnutty_bookshelf.package",
                "BuildBuy",
            ),
            (
                "Lumpinou_Relationship_Overhaul.zip",
                "lumpinou_relationship_overhaul.package",
                "Gameplay",
            ),
            (
                "Andirz_Custom_Trait.zip",
                "andirz_custom_trait.package",
                "Gameplay",
            ),
            (
                "Triplis_Cafe_Set.zip",
                "triplis_cafe_counter.package",
                "BuildBuy",
            ),
        ];

        for (index, (archive_name, filename, kind)) in near_misses.iter().enumerate() {
            let item_id = 400 + index as i64;
            build_near_miss_download(
                &staging_root,
                &connection,
                item_id,
                archive_name,
                filename,
                kind,
            );
            let assessment = assess_download_item(&connection, &settings, &seed_pack, item_id)
                .expect("assessment");
            assert_eq!(
                assessment.intake_mode,
                DownloadIntakeMode::Standard,
                "expected standard flow for near miss {filename}"
            );
        }
    }

    #[test]
    fn generic_cc_stays_on_standard_path() {
        let (_temp, connection, seed_pack, settings) = setup_env();
        let staging_root = PathBuf::from(settings.downloads_path.clone().expect("downloads"));
        let staging = staging_root.join("generic");
        fs::create_dir_all(&staging).expect("staging");
        insert_download_item(&connection, 220, "Generic_CC.zip", &staging);
        let file = staging.join("nice_hair.package");
        fs::write(&file, b"hair").expect("file");
        insert_download_file(&connection, 220, 22001, &file, "nice_hair.package", "CAS");

        let assessment =
            assess_download_item(&connection, &settings, &seed_pack, 220).expect("assessment");
        assert_eq!(assessment.intake_mode, DownloadIntakeMode::Standard);
    }

    #[test]
    fn missing_required_core_blocks_mccc() {
        let (_temp, connection, seed_pack, settings) = setup_env();
        let staging_root = PathBuf::from(settings.downloads_path.clone().expect("downloads"));
        let staging = staging_root.join("mccc_partial");
        fs::create_dir_all(&staging).expect("staging");
        insert_download_item(&connection, 221, "MCCC_Partial.zip", &staging);
        let file = staging.join("mc_woohoo.package");
        fs::write(&file, b"woohoo").expect("file");
        insert_download_file(&connection, 221, 22101, &file, "mc_woohoo.package", "Mods");

        let assessment =
            assess_download_item(&connection, &settings, &seed_pack, 221).expect("assessment");
        assert_eq!(assessment.intake_mode, DownloadIntakeMode::Blocked);
    }

    #[test]
    fn partial_mccc_returns_download_action_when_trusted_link_exists() {
        let (_temp, connection, seed_pack, settings) = setup_env();
        let staging_root = PathBuf::from(settings.downloads_path.clone().expect("downloads"));
        let staging = staging_root.join("mccc_partial_action");
        fs::create_dir_all(&staging).expect("staging");
        insert_download_item(&connection, 228, "MCCC_Partial_Action.zip", &staging);
        let file = staging.join("mc_woohoo.package");
        fs::write(&file, b"woohoo").expect("file");
        insert_download_file(&connection, 228, 22801, &file, "mc_woohoo.package", "Mods");

        let plan = build_review_plan(&connection, &settings, &seed_pack, 228)
            .expect("plan")
            .expect("review");
        let action = plan
            .available_actions
            .into_iter()
            .find(|action| action.kind == ReviewPlanActionKind::DownloadMissingFiles)
            .expect("download action");
        assert!(action.url.as_deref().is_some_and(
            |url| url.contains("deaderpool-mccc.com") || url.contains("drive.google.com")
        ));
    }

    #[test]
    fn partial_mccc_falls_back_to_official_page_without_trusted_link() {
        let (_temp, connection, mut seed_pack, settings) = setup_env();
        let staging_root = PathBuf::from(settings.downloads_path.clone().expect("downloads"));
        let staging = staging_root.join("mccc_partial_fallback");
        fs::create_dir_all(&staging).expect("staging");
        insert_download_item(&connection, 229, "MCCC_Partial_Fallback.zip", &staging);
        let file = staging.join("mc_woohoo.package");
        fs::write(&file, b"woohoo").expect("file");
        insert_download_file(&connection, 229, 22901, &file, "mc_woohoo.package", "Mods");

        let profile = seed_pack
            .install_catalog
            .guided_profiles
            .iter_mut()
            .find(|profile| profile.key == "mccc")
            .expect("mccc profile");
        profile.official_download_url = None;

        let plan = build_review_plan(&connection, &settings, &seed_pack, 229)
            .expect("plan")
            .expect("review");
        let action = plan
            .available_actions
            .into_iter()
            .find(|action| action.kind == ReviewPlanActionKind::OpenOfficialSource)
            .expect("official source action");
        assert_eq!(
            action.url.as_deref(),
            Some("https://deaderpool-mccc.com/downloads.html")
        );
    }

    #[test]
    fn missing_dependency_moves_item_to_review() {
        let (_temp, connection, seed_pack, settings) = setup_env();
        let staging_root = PathBuf::from(settings.downloads_path.clone().expect("downloads"));
        let staging = staging_root.join("requires_xml");
        fs::create_dir_all(&staging).expect("staging");
        insert_download_item(&connection, 222, "Needs_XML.zip", &staging);
        let readme = staging.join("README_requires_xml_injector.txt");
        fs::write(&readme, b"This mod requires XML Injector.").expect("readme");
        let file = staging.join("my_trait.package");
        fs::write(&file, b"trait").expect("file");
        insert_download_file(&connection, 222, 22201, &file, "my_trait.package", "Mods");

        let assessment =
            assess_download_item(&connection, &settings, &seed_pack, 222).expect("assessment");
        assert_eq!(assessment.intake_mode, DownloadIntakeMode::NeedsReview);
        assert!(assessment
            .missing_dependencies
            .iter()
            .any(|value| value.contains("XML Injector")));
    }

    #[test]
    fn dependency_in_inbox_keeps_item_in_review() {
        let (_temp, connection, seed_pack, settings) = setup_env();
        let staging_root = PathBuf::from(settings.downloads_path.clone().expect("downloads"));
        let xml_profile = seed_pack
            .install_catalog
            .guided_profiles
            .iter()
            .find(|profile| profile.key == "xml_injector")
            .expect("xml injector");
        build_sample_download(&staging_root, &connection, 223, xml_profile);
        let xml_assessment =
            assess_download_item(&connection, &settings, &seed_pack, 223).expect("xml");
        store_download_item_assessment(&connection, 223, &xml_assessment).expect("store");

        let staging = staging_root.join("requires_xml_inbox");
        fs::create_dir_all(&staging).expect("staging");
        insert_download_item(&connection, 224, "Needs_XML_Inbox.zip", &staging);
        let readme = staging.join("README_requires_xml_injector.txt");
        fs::write(&readme, b"This mod requires XML Injector.").expect("readme");
        let file = staging.join("my_trait.package");
        fs::write(&file, b"trait").expect("file");
        insert_download_file(&connection, 224, 22401, &file, "my_trait.package", "Mods");

        let assessment =
            assess_download_item(&connection, &settings, &seed_pack, 224).expect("assessment");
        assert_eq!(assessment.intake_mode, DownloadIntakeMode::NeedsReview);
        assert!(assessment
            .inbox_dependencies
            .iter()
            .any(|value| value.contains("XML Injector")));
        let review_plan =
            build_review_plan(&connection, &settings, &seed_pack, 224).expect("review plan");
        let dependency = review_plan
            .expect("review")
            .dependencies
            .into_iter()
            .find(|dependency| dependency.key == "xml_injector")
            .expect("xml dependency");
        assert_eq!(dependency.status, "inbox");
        assert_eq!(dependency.inbox_item_id, Some(223));
        assert_eq!(
            dependency.inbox_item_name.as_deref(),
            Some("XML_Injector.zip")
        );
        assert_eq!(
            dependency.inbox_item_intake_mode,
            Some(DownloadIntakeMode::Guided)
        );
        assert!(dependency.inbox_item_guided_install_available);
    }

    #[test]
    fn option_pack_pattern_stays_in_review() {
        let (_temp, connection, seed_pack, settings) = setup_env();
        let staging_root = PathBuf::from(settings.downloads_path.clone().expect("downloads"));
        let staging = staging_root.join("choose_one");
        let variant = staging.join("Pick One");
        fs::create_dir_all(&variant).expect("variant");
        insert_download_item(&connection, 225, "Choose_One_Set.zip", &staging);
        let option_a = variant.join("Option_A.package");
        fs::write(&option_a, b"a").expect("option");
        insert_download_file(
            &connection,
            225,
            22501,
            &option_a,
            "Pick One/Option_A.package",
            "Mods",
        );

        let assessment =
            assess_download_item(&connection, &settings, &seed_pack, 225).expect("assessment");
        assert_eq!(assessment.intake_mode, DownloadIntakeMode::NeedsReview);

        let plan = build_review_plan(&connection, &settings, &seed_pack, 225)
            .expect("plan")
            .expect("review");
        assert!(plan.explanation.contains("multiple install choices"));
    }

    #[test]
    fn guided_plan_detects_existing_replace_and_preserve_files() {
        let (temp, connection, seed_pack, settings) = setup_env();
        let staging_root = PathBuf::from(settings.downloads_path.clone().expect("downloads"));
        let profile = seed_pack
            .install_catalog
            .guided_profiles
            .iter()
            .find(|profile| profile.key == "mccc")
            .expect("mccc");
        build_sample_download(&staging_root, &connection, 226, profile);

        let target = temp.path().join("Mods").join("MCCC");
        fs::create_dir_all(&target).expect("target");
        let old_script = target.join("mc_cmd_center.ts4script");
        let cfg = target.join("mc_settings.cfg");
        fs::write(&old_script, b"old").expect("old");
        fs::write(&cfg, b"cfg").expect("cfg");
        insert_installed_file(&connection, &old_script, "Script Mods");

        let plan = build_guided_plan(&connection, &settings, &seed_pack, 226)
            .expect("plan")
            .expect("guided");
        assert!(!plan.replace_files.is_empty());
        assert!(!plan.preserve_files.is_empty());
    }

    #[test]
    fn deep_existing_install_blocks_guided_update() {
        let (temp, connection, seed_pack, settings) = setup_env();
        let staging_root = PathBuf::from(settings.downloads_path.clone().expect("downloads"));
        let profile = seed_pack
            .install_catalog
            .guided_profiles
            .iter()
            .find(|profile| profile.key == "mccc")
            .expect("mccc");
        build_sample_download(&staging_root, &connection, 227, profile);

        let scattered = temp
            .path()
            .join("Mods")
            .join("Gameplay")
            .join("MCCC")
            .join("mc_cmd_center.ts4script");
        fs::create_dir_all(scattered.parent().expect("parent")).expect("dir");
        fs::write(&scattered, b"old").expect("old");
        insert_installed_file(&connection, &scattered, "Script Mods");

        let assessment =
            assess_download_item(&connection, &settings, &seed_pack, 227).expect("assessment");
        assert_eq!(assessment.intake_mode, DownloadIntakeMode::Blocked);
    }

    #[test]
    fn mixed_mccc_archive_returns_separate_supported_files_action() {
        let (_temp, connection, seed_pack, settings) = setup_env();
        let staging_root = PathBuf::from(settings.downloads_path.clone().expect("downloads"));
        let profile = seed_pack
            .install_catalog
            .guided_profiles
            .iter()
            .find(|profile| profile.key == "mccc")
            .expect("mccc");
        let staging = staging_root.join("mccc_mixed");
        fs::create_dir_all(&staging).expect("staging");
        insert_download_item(&connection, 230, "MCCC_Mixed.zip", &staging);

        for (index, sample) in profile.sample_filenames.iter().enumerate() {
            let file_path = staging.join(sample);
            fs::write(&file_path, b"sample").expect("sample");
            insert_download_file(
                &connection,
                230,
                23000 + index as i64 + 1,
                &file_path,
                sample,
                "Script Mods",
            );
        }

        let extra = staging.join("othermod.ts4script");
        fs::write(&extra, b"extra").expect("extra");
        insert_download_file(
            &connection,
            230,
            23099,
            &extra,
            "othermod.ts4script",
            "Script Mods",
        );

        let assessment =
            assess_download_item(&connection, &settings, &seed_pack, 230).expect("assessment");
        assert_eq!(assessment.intake_mode, DownloadIntakeMode::NeedsReview);

        let plan = build_review_plan(&connection, &settings, &seed_pack, 230)
            .expect("plan")
            .expect("review");
        assert!(plan
            .available_actions
            .iter()
            .any(|action| action.kind == ReviewPlanActionKind::SeparateSupportedFiles));
    }

    #[test]
    fn partial_special_mods_still_return_review_actions() {
        let (_temp, connection, seed_pack, settings) = setup_env();
        let staging_root = PathBuf::from(settings.downloads_path.clone().expect("downloads"));
        let staging = staging_root.join("mccc_guided_hold");
        fs::create_dir_all(&staging).expect("staging");
        insert_download_item(&connection, 231, "MCCC_Guided_Hold.zip", &staging);

        let file = staging.join("mc_cmd_center.ts4script");
        fs::write(&file, b"core").expect("file");
        insert_download_file(
            &connection,
            231,
            23101,
            &file,
            "mc_cmd_center.ts4script",
            "Unknown",
        );

        let assessment =
            assess_download_item(&connection, &settings, &seed_pack, 231).expect("assessment");
        assert_eq!(assessment.intake_mode, DownloadIntakeMode::NeedsReview);

        let review_plan = build_review_plan(&connection, &settings, &seed_pack, 231)
            .expect("review plan")
            .expect("review");
        assert_eq!(review_plan.mode, DownloadIntakeMode::NeedsReview);
        assert_eq!(review_plan.review_files.len(), 1);
        assert!(review_plan.available_actions.iter().any(|action| {
            matches!(
                action.kind,
                ReviewPlanActionKind::DownloadMissingFiles
                    | ReviewPlanActionKind::OpenOfficialSource
            )
        }));
    }

    #[test]
    fn full_mccc_update_pack_stays_ready_for_guided_update() {
        let (temp, connection, seed_pack, settings) = setup_env();
        let staging_root = PathBuf::from(settings.downloads_path.clone().expect("downloads"));
        let staging = staging_root.join("mccc_full_update");
        fs::create_dir_all(&staging).expect("staging");
        insert_download_item(
            &connection,
            232,
            "McCmdCenter_AllModules_2026_1_1.zip",
            &staging,
        );

        let filenames = [
            "mc_career.ts4script",
            "mc_cas.ts4script",
            "mc_cheats.ts4script",
            "mc_cleaner.ts4script",
            "mc_clubs.ts4script",
            "mc_cmd_center.package",
            "mc_cmd_center.ts4script",
            "mc_control.ts4script",
            "mc_dresser.ts4script",
            "mc_gedcom.ts4script",
            "mc_occult.ts4script",
            "mc_population.ts4script",
            "mc_pregnancy.ts4script",
            "mc_tuner.ts4script",
        ];

        for (index, filename) in filenames.iter().enumerate() {
            let file_path = staging.join(filename);
            fs::write(&file_path, b"full-mccc").expect("file");
            insert_download_file(
                &connection,
                232,
                23200 + index as i64 + 1,
                &file_path,
                filename,
                if filename.ends_with(".ts4script") {
                    "Script Mods"
                } else {
                    "Mods"
                },
            );
        }

        let target = temp.path().join("Mods").join("MCCC");
        fs::create_dir_all(&target).expect("target");
        let old_script = target.join("mc_cmd_center.ts4script");
        let old_package = target.join("mc_cmd_center.package");
        let old_cfg = target.join("mc_settings.cfg");
        fs::write(&old_script, b"old-script").expect("old script");
        fs::write(&old_package, b"old-package").expect("old package");
        fs::write(&old_cfg, b"settings").expect("cfg");
        insert_installed_file(&connection, &old_script, "Script Mods");
        insert_installed_file(&connection, &old_package, "Mods");

        let plan = build_guided_plan(&connection, &settings, &seed_pack, 232)
            .expect("plan")
            .expect("guided");
        assert!(plan.apply_ready);
        assert!(plan.review_files.is_empty());
        assert_eq!(plan.install_files.len(), filenames.len());
        assert_eq!(plan.replace_files.len(), 2);
        assert_eq!(plan.preserve_files.len(), 1);

        let review_plan =
            build_review_plan(&connection, &settings, &seed_pack, 232).expect("review plan");
        assert!(review_plan.is_none());
    }

    #[test]
    fn disk_only_root_level_mccc_install_still_builds_a_ready_update() {
        let (temp, connection, seed_pack, settings) = setup_env();
        let staging_root = PathBuf::from(settings.downloads_path.clone().expect("downloads"));
        let profile = seed_pack
            .install_catalog
            .guided_profiles
            .iter()
            .find(|profile| profile.key == "mccc")
            .expect("mccc");

        build_sample_download(&staging_root, &connection, 234, profile);

        let mods_root = temp.path().join("Mods");
        for sample in &profile.sample_filenames {
            if sample == "mc_cmd_center.package" {
                continue;
            }

            let path = mods_root.join(sample);
            fs::write(&path, b"installed").expect("installed file");
        }

        let assessment =
            assess_download_item(&connection, &settings, &seed_pack, 234).expect("assessment");
        assert_eq!(assessment.intake_mode, DownloadIntakeMode::Guided);
        assert!(assessment.existing_install_detected);

        let plan = build_guided_plan(&connection, &settings, &seed_pack, 234)
            .expect("plan")
            .expect("guided");
        assert!(plan.apply_ready);
        assert!(plan.existing_install_detected);
        assert!(plan.review_files.is_empty());

        let decision = build_special_mod_decision(&connection, &settings, &seed_pack, 234)
            .expect("decision")
            .expect("special decision");
        assert!(decision.apply_ready);
        assert_eq!(decision.state, SpecialDecisionState::GuidedReady);
    }

    #[test]
    fn same_version_mccc_download_is_marked_as_already_current() {
        let (temp, connection, seed_pack, settings) = setup_env();
        let staging_root = PathBuf::from(settings.downloads_path.clone().expect("downloads"));
        let profile = seed_pack
            .install_catalog
            .guided_profiles
            .iter()
            .find(|profile| profile.key == "mccc")
            .expect("mccc");
        let staging = staging_root.join("mccc_same_version");
        fs::create_dir_all(&staging).expect("staging");
        insert_download_item(
            &connection,
            238,
            "McCmdCenter_AllModules_2026_1_1.zip",
            &staging,
        );

        for (index, sample) in profile.sample_filenames.iter().enumerate() {
            let file_path = staging.join(sample);
            fs::write(&file_path, b"sample").expect("sample");
            insert_download_file(
                &connection,
                238,
                23800 + index as i64 + 1,
                &file_path,
                sample,
                if sample.ends_with(".ts4script") {
                    "Script Mods"
                } else {
                    "Mods"
                },
            );
        }

        let existing_root = temp.path().join("Mods").join("MCCC");
        fs::create_dir_all(&existing_root).expect("existing");
        for sample in &profile.sample_filenames {
            let file_path = existing_root.join(sample);
            if let Some(parent) = file_path.parent() {
                fs::create_dir_all(parent).expect("existing parents");
            }
            fs::write(&file_path, b"sample").expect("installed sample");
            insert_installed_file(
                &connection,
                &file_path,
                if sample.ends_with(".ts4script") {
                    "Script Mods"
                } else {
                    "Mods"
                },
            );
        }
        insert_family_state(
            &connection,
            "mccc",
            "MC Command Center",
            "clean",
            Some(&existing_root),
            Some("2026.1.1"),
            None,
        );

        let assessment =
            assess_download_item(&connection, &settings, &seed_pack, 238).expect("assessment");
        store_download_item_assessment(&connection, 238, &assessment).expect("stored");

        let decision = build_special_mod_decision(&connection, &settings, &seed_pack, 238)
            .expect("decision")
            .expect("special decision");
        assert!(decision.apply_ready);
        assert!(decision.same_version);
        assert_eq!(decision.version_status, SpecialVersionStatus::SameVersion);
        assert_eq!(decision.queue_lane, DownloadQueueLane::Done);
        assert!(decision.primary_action.is_none());
        assert!(decision.recommended_next_step.contains("already current"));
    }

    #[test]
    fn inside_mod_version_hints_drive_same_version_detection() {
        let (temp, connection, seed_pack, settings) = setup_env();
        let staging_root = PathBuf::from(settings.downloads_path.clone().expect("downloads"));
        let profile = seed_pack
            .install_catalog
            .guided_profiles
            .iter()
            .find(|profile| profile.key == "mccc")
            .expect("mccc");
        build_sample_download(&staging_root, &connection, 262, profile);
        update_file_insights_by_id(
            &connection,
            26201,
            &FileInsights {
                version_hints: vec!["2026.1.1".to_owned()],
                family_hints: vec!["mccc".to_owned(), "mc cmd center".to_owned()],
                ..FileInsights::default()
            },
        );

        let existing_root = temp.path().join("Mods").join("MCCC");
        fs::create_dir_all(&existing_root).expect("existing");
        for sample in &profile.sample_filenames {
            let file_path = existing_root.join(sample);
            if let Some(parent) = file_path.parent() {
                fs::create_dir_all(parent).expect("existing parents");
            }
            fs::write(&file_path, b"sample").expect("installed sample");
            insert_installed_file(
                &connection,
                &file_path,
                if sample.ends_with(".ts4script") {
                    "Script Mods"
                } else {
                    "Mods"
                },
            );
        }
        let current_script = existing_root.join("mc_cmd_center.ts4script");
        update_file_insights_by_path(
            &connection,
            &current_script,
            &FileInsights {
                version_hints: vec!["2026.1.1".to_owned()],
                family_hints: vec!["mccc".to_owned()],
                ..FileInsights::default()
            },
        );

        let assessment =
            assess_download_item(&connection, &settings, &seed_pack, 262).expect("assessment");
        store_download_item_assessment(&connection, 262, &assessment).expect("stored");

        let decision = build_special_mod_decision(&connection, &settings, &seed_pack, 262)
            .expect("decision")
            .expect("special decision");
        assert_eq!(decision.incoming_version.as_deref(), Some("2026.1.1"));
        assert_eq!(
            decision.incoming_version_source.as_deref(),
            Some("inside mod")
        );
        assert_eq!(
            decision.installed_version_source.as_deref(),
            Some("inside mod")
        );
        assert_eq!(decision.comparison_source.as_deref(), Some("inside mod"));
        assert_eq!(decision.version_status, SpecialVersionStatus::SameVersion);
        assert!(decision.same_version);
        assert!(decision
            .incoming_version_evidence
            .iter()
            .any(|line| line.contains("inside the download")));
        assert!(decision
            .installed_version_evidence
            .iter()
            .any(|line| line.contains("installed mod files")));
        assert!(decision
            .comparison_evidence
            .iter()
            .any(|line| line.contains("inside the mod files")));
    }

    #[test]
    fn incoming_special_mod_versions_prefer_inside_mod_hints_over_download_name() {
        let (_temp, connection, seed_pack, settings) = setup_env();
        let staging_root = PathBuf::from(settings.downloads_path.clone().expect("downloads"));
        let profile = seed_pack
            .install_catalog
            .guided_profiles
            .iter()
            .find(|profile| profile.key == "mccc")
            .expect("mccc");
        let staging = staging_root.join("mccc_inside_mod_priority");
        fs::create_dir_all(&staging).expect("staging");
        insert_download_item(
            &connection,
            266,
            "McCmdCenter_AllModules_2025_4_0.zip",
            &staging,
        );

        for (index, sample) in profile.sample_filenames.iter().enumerate() {
            let file_path = staging.join(sample);
            fs::write(&file_path, b"sample").expect("sample");
            insert_download_file(
                &connection,
                266,
                26600 + index as i64 + 1,
                &file_path,
                sample,
                if sample.ends_with(".ts4script") {
                    "Script Mods"
                } else {
                    "Mods"
                },
            );
        }
        update_file_insights_by_id(
            &connection,
            26601,
            &FileInsights {
                version_hints: vec!["2026.1.1".to_owned()],
                family_hints: vec!["mccc".to_owned()],
                ..FileInsights::default()
            },
        );

        let assessment =
            assess_download_item(&connection, &settings, &seed_pack, 266).expect("assessment");
        store_download_item_assessment(&connection, 266, &assessment).expect("stored");

        let decision = build_special_mod_decision(&connection, &settings, &seed_pack, 266)
            .expect("decision")
            .expect("special decision");
        assert_eq!(decision.incoming_version.as_deref(), Some("2026.1.1"));
        assert_eq!(
            decision.incoming_version_source.as_deref(),
            Some("inside mod")
        );
        assert!(decision
            .incoming_version_evidence
            .iter()
            .any(|line| line.contains("inside the download")));
    }

    #[test]
    fn older_mccc_download_is_not_treated_as_the_next_update() {
        let (temp, connection, seed_pack, settings) = setup_env();
        let staging_root = PathBuf::from(settings.downloads_path.clone().expect("downloads"));
        let profile = seed_pack
            .install_catalog
            .guided_profiles
            .iter()
            .find(|profile| profile.key == "mccc")
            .expect("mccc");
        let staging = staging_root.join("mccc_older_version");
        fs::create_dir_all(&staging).expect("staging");
        insert_download_item(
            &connection,
            239,
            "McCmdCenter_AllModules_2025_4_0.zip",
            &staging,
        );

        for (index, sample) in profile.sample_filenames.iter().enumerate() {
            let file_path = staging.join(sample);
            fs::write(&file_path, b"sample").expect("sample");
            insert_download_file(
                &connection,
                239,
                23900 + index as i64 + 1,
                &file_path,
                sample,
                if sample.ends_with(".ts4script") {
                    "Script Mods"
                } else {
                    "Mods"
                },
            );
        }

        let existing_root = temp.path().join("Mods").join("MCCC");
        fs::create_dir_all(&existing_root).expect("existing");
        let current_script = existing_root.join("mc_cmd_center.ts4script");
        let current_package = existing_root.join("mc_cmd_center.package");
        fs::write(&current_script, b"installed").expect("script");
        fs::write(&current_package, b"installed").expect("package");
        insert_installed_file(&connection, &current_script, "Script Mods");
        insert_installed_file(&connection, &current_package, "Mods");
        insert_family_state(
            &connection,
            "mccc",
            "MC Command Center",
            "clean",
            Some(&existing_root),
            Some("2026.1.1"),
            None,
        );

        let assessment =
            assess_download_item(&connection, &settings, &seed_pack, 239).expect("assessment");
        store_download_item_assessment(&connection, 239, &assessment).expect("stored");

        let decision = build_special_mod_decision(&connection, &settings, &seed_pack, 239)
            .expect("decision")
            .expect("special decision");
        assert!(decision.apply_ready);
        assert_eq!(decision.version_status, SpecialVersionStatus::IncomingOlder);
        assert_eq!(decision.queue_lane, DownloadQueueLane::Done);
        assert!(decision.primary_action.is_none());
        assert!(decision
            .recommended_next_step
            .contains("older MC Command Center download"));
    }

    #[test]
    fn saved_family_state_can_supply_installed_version_evidence() {
        let (temp, connection, seed_pack, settings) = setup_env();
        let staging_root = PathBuf::from(settings.downloads_path.clone().expect("downloads"));
        let profile = seed_pack
            .install_catalog
            .guided_profiles
            .iter()
            .find(|profile| profile.key == "mccc")
            .expect("mccc");
        let staging = staging_root.join("mccc_saved_state_version");
        fs::create_dir_all(&staging).expect("staging");
        insert_download_item(
            &connection,
            263,
            "McCmdCenter_AllModules_2026_1_1.zip",
            &staging,
        );

        for (index, sample) in profile.sample_filenames.iter().enumerate() {
            let file_path = staging.join(sample);
            fs::write(&file_path, b"sample").expect("sample");
            insert_download_file(
                &connection,
                263,
                26300 + index as i64 + 1,
                &file_path,
                sample,
                if sample.ends_with(".ts4script") {
                    "Script Mods"
                } else {
                    "Mods"
                },
            );
        }

        let existing_root = temp.path().join("Mods").join("MCCC");
        fs::create_dir_all(&existing_root).expect("existing");
        let current_script = existing_root.join("mc_cmd_center.ts4script");
        let current_package = existing_root.join("mc_cmd_center.package");
        fs::write(&current_script, b"installed").expect("script");
        fs::write(&current_package, b"installed").expect("package");
        insert_installed_file(&connection, &current_script, "Script Mods");
        insert_installed_file(&connection, &current_package, "Mods");
        insert_family_state(
            &connection,
            "mccc",
            "MC Command Center",
            "clean",
            Some(&existing_root),
            Some("2026.1.1"),
            None,
        );

        let assessment =
            assess_download_item(&connection, &settings, &seed_pack, 263).expect("assessment");
        store_download_item_assessment(&connection, 263, &assessment).expect("stored");

        let decision = build_special_mod_decision(&connection, &settings, &seed_pack, 263)
            .expect("decision")
            .expect("special decision");
        assert_eq!(
            decision.installed_version_source.as_deref(),
            Some("saved family state")
        );
        assert_eq!(decision.version_status, SpecialVersionStatus::Unknown);
        assert!(!decision.same_version);
        assert!(decision
            .installed_version_evidence
            .iter()
            .any(|line| line.contains("last saved family record")));
        assert!(decision
            .comparison_evidence
            .iter()
            .any(|line| line.contains("saved family record")));
    }

    #[test]
    fn matching_signatures_add_file_fingerprint_evidence() {
        let (temp, connection, seed_pack, settings) = setup_env();
        let staging_root = PathBuf::from(settings.downloads_path.clone().expect("downloads"));
        let profile = seed_pack
            .install_catalog
            .guided_profiles
            .iter()
            .find(|profile| profile.key == "mccc")
            .expect("mccc");
        build_sample_download(&staging_root, &connection, 267, profile);

        let existing_root = temp.path().join("Mods").join("MCCC");
        fs::create_dir_all(&existing_root).expect("existing");
        for sample in &profile.sample_filenames {
            let file_path = existing_root.join(sample);
            if let Some(parent) = file_path.parent() {
                fs::create_dir_all(parent).expect("existing parents");
            }
            fs::write(&file_path, b"sample").expect("installed sample");
            insert_installed_file(
                &connection,
                &file_path,
                if sample.ends_with(".ts4script") {
                    "Script Mods"
                } else {
                    "Mods"
                },
            );
        }

        let assessment =
            assess_download_item(&connection, &settings, &seed_pack, 267).expect("assessment");
        store_download_item_assessment(&connection, 267, &assessment).expect("stored");

        let decision = build_special_mod_decision(&connection, &settings, &seed_pack, 267)
            .expect("decision")
            .expect("special decision");
        assert_eq!(decision.version_status, SpecialVersionStatus::SameVersion);
        assert!(decision.same_version);
        assert_eq!(
            decision.comparison_source.as_deref(),
            Some("file signature")
        );
        assert!(decision.incoming_version.is_none());
        assert!(decision
            .comparison_evidence
            .iter()
            .any(|line| line.contains("fingerprint")));
    }

    #[test]
    fn stale_special_mod_insights_refresh_from_live_archive_contents() {
        let (_temp, connection, seed_pack, settings) = setup_env();
        let staging_root = PathBuf::from(settings.downloads_path.clone().expect("downloads"));
        let profile = seed_pack
            .install_catalog
            .guided_profiles
            .iter()
            .find(|profile| profile.key == "mccc")
            .expect("mccc");

        build_sample_download(&staging_root, &connection, 264, profile);

        let staging = staging_root.join("264");
        let target_sample = profile
            .sample_filenames
            .iter()
            .enumerate()
            .find(|(_, sample)| sample.ends_with("mc_cmd_center.ts4script"))
            .expect("mccc script");
        let sample_path = staging.join(target_sample.1);
        write_script_archive(
            &sample_path,
            "deaderpool/mccc/mc_cmd_version.pyc",
            b"\0supports patch 1.113.277 and version 2026_1_1",
        );
        update_file_insights_by_id(
            &connection,
            26400 + target_sample.0 as i64 + 1,
            &FileInsights::default(),
        );

        let assessment =
            assess_download_item(&connection, &settings, &seed_pack, 264).expect("assessment");
        store_download_item_assessment(&connection, 264, &assessment).expect("stored");
        let decision = build_special_mod_decision(&connection, &settings, &seed_pack, 264)
            .expect("decision")
            .expect("special decision");

        assert_eq!(decision.incoming_version.as_deref(), Some("2026.1.1"));
        assert_eq!(
            decision.incoming_version_source.as_deref(),
            Some("inside mod")
        );

        let stored_insights: String = connection
            .query_row(
                "SELECT insights FROM files WHERE id = ?1",
                params![26400 + target_sample.0 as i64 + 1],
                |row| row.get(0),
            )
            .expect("stored insights");
        let stored: FileInsights = serde_json::from_str(&stored_insights).expect("insights json");
        assert!(stored.version_hints.iter().any(|value| value == "2026.1.1"));
    }

    #[test]
    fn matching_versions_with_different_file_sets_stay_installable() {
        let (temp, connection, seed_pack, settings) = setup_env();
        let staging_root = PathBuf::from(settings.downloads_path.clone().expect("downloads"));
        let profile = seed_pack
            .install_catalog
            .guided_profiles
            .iter()
            .find(|profile| profile.key == "mccc")
            .expect("mccc");

        connection
            .execute(
                "INSERT INTO download_items (
                    id,
                    source_path,
                    display_name,
                    source_kind,
                    archive_format,
                    staging_path,
                    source_size,
                    status,
                    notes
                 ) VALUES (?1, ?2, ?3, 'archive', 'zip', ?4, 100, 'pending', '[]')",
                params![
                    265_i64,
                    "C:/Downloads/McCmdCenter_AllModules_2026_1_1.zip",
                    "McCmdCenter_AllModules_2026_1_1.zip",
                    staging_root.join("265").to_string_lossy().to_string()
                ],
            )
            .expect("download item");
        let staging = staging_root.join("265");
        fs::create_dir_all(&staging).expect("staging");

        for (index, sample) in profile.sample_filenames.iter().enumerate() {
            let file_path = staging.join(sample);
            if let Some(parent) = file_path.parent() {
                fs::create_dir_all(parent).expect("parents");
            }
            if sample.ends_with("mc_cmd_center.ts4script")
                || sample.ends_with("mc_career.ts4script")
            {
                let entry_name = if sample.ends_with("mc_cmd_center.ts4script") {
                    "deaderpool/mccc/mc_cmd_version.pyc"
                } else {
                    "deaderpool/mccc/mc_career_version.pyc"
                };
                write_script_archive(
                    &file_path,
                    entry_name,
                    b"\0supports patch 1.113.277 and version 2026_1_1",
                );
            } else {
                fs::write(&file_path, b"sample").expect("sample");
            }
            insert_download_file(
                &connection,
                265,
                26500 + index as i64 + 1,
                &file_path,
                sample,
                if sample.ends_with(".ts4script") {
                    "Script Mods"
                } else {
                    "Mods"
                },
            );
        }

        let installed_root = temp.path().join("Mods");
        let installed_script = installed_root.join("mc_cmd_center.ts4script");
        write_script_archive(
            &installed_script,
            "deaderpool/mccc/mc_cmd_version.pyc",
            b"\0supports patch 1.113.277 and version 2026_1_1",
        );
        insert_installed_file(&connection, &installed_script, "Script Mods");

        let assessment =
            assess_download_item(&connection, &settings, &seed_pack, 265).expect("assessment");
        store_download_item_assessment(&connection, 265, &assessment).expect("stored");
        let decision = build_special_mod_decision(&connection, &settings, &seed_pack, 265)
            .expect("decision")
            .expect("special decision");

        assert!(decision.apply_ready);
        assert_eq!(decision.version_status, SpecialVersionStatus::Unknown);
        assert!(!decision.same_version);
    }

    #[test]
    fn reconcile_clears_stale_special_mod_family_ownership() {
        let (temp, connection, seed_pack, settings) = setup_env();
        let staging_root = PathBuf::from(settings.downloads_path.clone().expect("downloads"));
        let profile = seed_pack
            .install_catalog
            .guided_profiles
            .iter()
            .find(|profile| profile.key == "mccc")
            .expect("mccc");

        connection
            .execute(
                "INSERT INTO download_items (
                    id,
                    source_path,
                    display_name,
                    source_kind,
                    archive_format,
                    staging_path,
                    source_size,
                    status,
                    notes,
                    intake_mode,
                    matched_profile_key,
                    matched_profile_name
                 ) VALUES (
                    260,
                    'C:/Downloads/old_mccc.zip',
                    'Old_MCCC.zip',
                    'archive',
                    'zip',
                    ?1,
                    100,
                    'needs_review',
                    '[]',
                    'guided',
                    'mccc',
                    'MC Command Center'
                 )",
                params![staging_root.join("old_mccc").to_string_lossy().to_string()],
            )
            .expect("old item");

        let stale_root = temp.path().join("Mods").join("MCCC");
        fs::create_dir_all(&stale_root).expect("stale root");
        let stale_script = stale_root.join("mc_cmd_center.ts4script");
        let stale_package = stale_root.join("mc_cmd_center.package");
        fs::write(&stale_script, b"stale").expect("stale script");
        fs::write(&stale_package, b"stale").expect("stale package");
        connection
            .execute(
                "INSERT INTO files (
                    path,
                    filename,
                    extension,
                    kind,
                    confidence,
                    source_location,
                    download_item_id,
                    parser_warnings,
                    insights
                 ) VALUES (?1, 'mc_cmd_center.ts4script', '.ts4script', 'Script Mods', 0.95, 'mods', 260, '[]', '{}')",
                params![stale_script.to_string_lossy().to_string()],
            )
            .expect("stale script row");
        connection
            .execute(
                "INSERT INTO files (
                    path,
                    filename,
                    extension,
                    kind,
                    confidence,
                    source_location,
                    download_item_id,
                    parser_warnings,
                    insights
                 ) VALUES (?1, 'mc_cmd_center.package', '.package', 'Mods', 0.95, 'mods', 260, '[]', '{}')",
                params![stale_package.to_string_lossy().to_string()],
            )
            .expect("stale package row");

        let applied_staging = staging_root.join("mccc_new_family");
        fs::create_dir_all(&applied_staging).expect("applied staging");
        insert_download_item(
            &connection,
            261,
            "McCmdCenter_AllModules_2026_1_1.zip",
            &applied_staging,
        );
        for (index, sample) in profile.sample_filenames.iter().enumerate() {
            let file_path = applied_staging.join(sample);
            fs::write(&file_path, b"sample").expect("sample");
            insert_download_file(
                &connection,
                261,
                26100 + index as i64 + 1,
                &file_path,
                sample,
                if sample.ends_with(".ts4script") {
                    "Script Mods"
                } else {
                    "Mods"
                },
            );
        }
        let assessment =
            assess_download_item(&connection, &settings, &seed_pack, 261).expect("assessment");
        store_download_item_assessment(&connection, 261, &assessment).expect("stored");

        let affected =
            reconcile_special_mod_family(&connection, &settings, &seed_pack, "mccc", 261)
                .expect("reconcile");
        assert!(affected.contains(&260));

        let stale_owned: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM files WHERE download_item_id = 260 AND source_location != 'downloads'",
                [],
                |row| row.get(0),
            )
            .expect("stale ownership count");
        assert_eq!(stale_owned, 0);

        let old_status: String = connection
            .query_row(
                "SELECT status FROM download_items WHERE id = 260",
                [],
                |row| row.get(0),
            )
            .expect("old status");
        assert_eq!(old_status, "ignored");

        let (stored_version, stored_source_item_id): (Option<String>, Option<i64>) = connection
            .query_row(
                "SELECT installed_version, source_item_id
                 FROM special_mod_family_state
                 WHERE profile_key = 'mccc'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("family state");
        assert_eq!(stored_version.as_deref(), Some("2026.1.1"));
        assert_eq!(stored_source_item_id, Some(261));
    }

    #[test]
    fn duplicate_special_mod_family_prefers_the_fuller_local_pack() {
        let (_temp, connection, seed_pack, settings) = setup_env();
        let staging_root = PathBuf::from(settings.downloads_path.clone().expect("downloads"));

        let partial_staging = staging_root.join("mccc_partial_family");
        fs::create_dir_all(&partial_staging).expect("partial staging");
        insert_download_item(&connection, 240, "MCCC_partial_old.zip", &partial_staging);
        let partial_file = partial_staging.join("mc_cmd_center.ts4script");
        fs::write(&partial_file, b"core").expect("partial file");
        insert_download_file(
            &connection,
            240,
            24001,
            &partial_file,
            "mc_cmd_center.ts4script",
            "Script Mods",
        );
        let partial_assessment =
            assess_download_item(&connection, &settings, &seed_pack, 240).expect("partial");
        store_download_item_assessment(&connection, 240, &partial_assessment)
            .expect("store partial");

        let profile = seed_pack
            .install_catalog
            .guided_profiles
            .iter()
            .find(|profile| profile.key == "mccc")
            .expect("mccc");
        build_sample_download(&staging_root, &connection, 241, profile);
        let full_assessment =
            assess_download_item(&connection, &settings, &seed_pack, 241).expect("full");
        store_download_item_assessment(&connection, 241, &full_assessment).expect("store full");

        let decision = build_special_mod_decision(&connection, &settings, &seed_pack, 240)
            .expect("decision")
            .expect("special decision");
        assert_eq!(decision.family_role, SpecialFamilyRole::Superseded);
        assert_eq!(decision.primary_family_item_id, Some(241));
        assert!(decision
            .primary_action
            .as_ref()
            .is_some_and(|action| action.kind == ReviewPlanActionKind::OpenRelatedItem));

        let primary_decision = build_special_mod_decision(&connection, &settings, &seed_pack, 241)
            .expect("primary decision")
            .expect("primary special decision");
        assert_eq!(primary_decision.family_role, SpecialFamilyRole::Primary);
        assert!(primary_decision.apply_ready);
    }

    #[test]
    fn applied_full_family_pack_keeps_leftover_partial_out_of_the_waiting_lane() {
        let (temp, connection, seed_pack, settings) = setup_env();
        let staging_root = PathBuf::from(settings.downloads_path.clone().expect("downloads"));

        let partial_staging = staging_root.join("mccc_partial_after_apply");
        fs::create_dir_all(&partial_staging).expect("partial staging");
        insert_download_item(
            &connection,
            270,
            "MCCC_partial_after_apply.zip",
            &partial_staging,
        );
        let partial_file = partial_staging.join("mc_cmd_center.ts4script");
        fs::write(&partial_file, b"core").expect("partial file");
        insert_download_file(
            &connection,
            270,
            27001,
            &partial_file,
            "mc_cmd_center.ts4script",
            "Script Mods",
        );
        let partial_assessment =
            assess_download_item(&connection, &settings, &seed_pack, 270).expect("partial");
        store_download_item_assessment(&connection, 270, &partial_assessment)
            .expect("store partial");

        let profile = seed_pack
            .install_catalog
            .guided_profiles
            .iter()
            .find(|profile| profile.key == "mccc")
            .expect("mccc");
        build_sample_download(&staging_root, &connection, 271, profile);
        let full_assessment =
            assess_download_item(&connection, &settings, &seed_pack, 271).expect("full");
        store_download_item_assessment(&connection, 271, &full_assessment).expect("store full");

        let install_root = install_target_for_profile(temp.path(), profile);
        fs::create_dir_all(&install_root).expect("install root");
        for (index, sample) in profile.sample_filenames.iter().enumerate() {
            let source_path = staging_root.join("271").join(sample.replace('/', "\\"));
            let installed_path = install_root.join(sample.replace('/', "\\"));
            if let Some(parent) = installed_path.parent() {
                fs::create_dir_all(parent).expect("install parent");
            }
            fs::copy(&source_path, &installed_path).expect("copy installed file");
            connection
                .execute(
                    "UPDATE files
                     SET path = ?2,
                         source_location = 'mods',
                         relative_depth = 1,
                         indexed_at = CURRENT_TIMESTAMP
                     WHERE id = ?1",
                    params![
                        27100 + index as i64 + 1,
                        installed_path.to_string_lossy().to_string()
                    ],
                )
                .expect("mark installed file");
        }

        connection
            .execute(
                "UPDATE download_items
                 SET status = 'applied',
                     updated_at = ?2
                 WHERE id = ?1",
                params![271, Utc::now().to_rfc3339()],
            )
            .expect("mark applied");

        let affected =
            reconcile_special_mod_family(&connection, &settings, &seed_pack, "mccc", 271)
                .expect("reconcile");
        assert!(affected.contains(&270));

        let decision = build_special_mod_decision(&connection, &settings, &seed_pack, 270)
            .expect("decision")
            .expect("special decision");
        assert_eq!(decision.family_role, SpecialFamilyRole::Superseded);
        assert_eq!(decision.primary_family_item_id, Some(271));
        assert_eq!(decision.queue_lane, DownloadQueueLane::Done);
        assert!(decision.primary_action.is_none());
        assert!(decision.queue_summary.contains("already installed"));
        assert!(!decision.queue_summary.contains("already in Inbox"));
        assert!(decision
            .recommended_next_step
            .contains("Ignore this leftover"));
    }

    #[test]
    fn stale_indexed_special_files_do_not_block_fresh_guided_install() {
        let (temp, connection, seed_pack, settings) = setup_env();
        let staging_root = PathBuf::from(settings.downloads_path.clone().expect("downloads"));
        let profile = seed_pack
            .install_catalog
            .guided_profiles
            .iter()
            .find(|profile| profile.key == "mccc")
            .expect("mccc");
        build_sample_download(&staging_root, &connection, 233, profile);

        let stale_path = temp
            .path()
            .join("Mods")
            .join("MCCC")
            .join("mc_cmd_center.ts4script");
        insert_installed_file(&connection, &stale_path, "Script Mods");

        let assessment =
            assess_download_item(&connection, &settings, &seed_pack, 233).expect("assessment");
        assert_eq!(assessment.intake_mode, DownloadIntakeMode::Guided);
        assert!(!assessment.existing_install_detected);

        let plan = build_guided_plan(&connection, &settings, &seed_pack, 233)
            .expect("plan")
            .expect("guided");
        assert!(plan.apply_ready);
        assert!(plan.review_files.is_empty());

        let review_plan =
            build_review_plan(&connection, &settings, &seed_pack, 233).expect("review plan");
        assert!(review_plan.is_none());
    }

    #[test]
    fn normalized_helper_collapses_symbol_noise() {
        assert_eq!(
            normalized("[MCCC] MC_Command_Center"),
            " mccc  mc command center"
        );
    }

    #[test]
    fn evidence_summary_lists_core_signals() {
        let summary = build_evidence_summary(&super::ProfileEvidence {
            reasons: Vec::new(),
            matched_files: 2,
            matched_script_files: 1,
            matched_package_files: 1,
            unmatched_supported_files: 1,
            required_core_present: true,
            required_exact_filenames_found: 1,
            name_match: false,
            text_matches: vec!["xml injector".to_owned()],
            archive_matches: vec!["pick one".to_owned()],
        });
        assert!(summary
            .iter()
            .any(|value| value.contains("matching special-mod files")));
        assert!(summary
            .iter()
            .any(|value| value.contains("Required core files")));
        assert!(summary.iter().any(|value| value.contains("xml injector")));
    }
}
