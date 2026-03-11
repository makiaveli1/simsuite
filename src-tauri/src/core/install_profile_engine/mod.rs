use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
};

use rusqlite::{params, Connection, OptionalExtension};
use walkdir::WalkDir;

use crate::{
    core::validator::{self, ValidationRequest},
    error::{AppError, AppResult},
    models::{
        CatalogSourceInfo, DependencyStatus, DownloadIntakeMode, DownloadRiskLevel, FileInsights,
        GuidedInstallFileEntry, GuidedInstallPlan, LibrarySettings, ReviewPlanAction,
        ReviewPlanActionKind, SpecialReviewPlan,
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
    unmatched_supported_files: i64,
    required_core_present: bool,
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

    fn strong_match(&self) -> bool {
        self.matched_files > 0 && self.required_core_present
    }

    fn score(&self) -> i64 {
        (self.matched_files * 100)
            + if self.required_core_present { 50 } else { 0 }
            + if self.name_match { 20 } else { 0 }
            + (self.text_matches.len() as i64 * 5)
            + (self.archive_matches.len() as i64 * 5)
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
    dependencies: Vec<DependencyStatus>,
    existing_layout_findings: Vec<String>,
    explanation: String,
    recommended_next_step: String,
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
             catalog_reference_source = ?16,
             catalog_reviewed_at = ?17,
             existing_install_detected = ?18,
             guided_install_available = ?19,
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
            if assessment.existing_install_detected { 1_i64 } else { 0_i64 },
            if assessment.guided_install_available { 1_i64 } else { 0_i64 },
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
    build_guided_plan_internal(connection, settings, seed_pack, item_id, false)
}

pub fn build_repair_guided_plan(
    connection: &Connection,
    settings: &LibrarySettings,
    seed_pack: &SeedPack,
    item_id: i64,
) -> AppResult<Option<GuidedInstallPlan>> {
    build_guided_plan_internal(connection, settings, seed_pack, item_id, true)
}

fn build_guided_plan_internal(
    connection: &Connection,
    settings: &LibrarySettings,
    seed_pack: &SeedPack,
    item_id: i64,
    allow_repair_layout: bool,
) -> AppResult<Option<GuidedInstallPlan>> {
    let evaluation = evaluate_download_item(connection, settings, seed_pack, item_id)?;
    if evaluation.assessment.intake_mode != DownloadIntakeMode::Guided && !allow_repair_layout {
        return Ok(None);
    }

    let Some(profile) = evaluation.matched_profile.clone() else {
        return Ok(None);
    };
    let Some(item) = load_item_with_staging(connection, item_id)? else {
        return Ok(None);
    };
    let active_files = load_profile_files(connection, item_id, true)?;
    let layout = detect_existing_layout(connection, settings, &profile)?;
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

    let reserved_targets = incoming
        .iter()
        .map(|file| layout.target_folder.join(&file.filename).to_string_lossy().to_string())
        .collect::<HashSet<_>>();

    let mut install_files = Vec::new();
    let mut review_files = Vec::new();
    let mut warnings = layout.warnings.clone();
    let mut evidence = evaluation.assessment.evidence_summary.clone();

    for file in incoming {
        let validation = validator::validate_suggestion(
            connection,
            settings,
            &ValidationRequest {
                file_id: file.file_id,
                filename: file.filename.clone(),
                extension: file.extension.clone(),
                kind: file.kind.clone(),
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

        let entry = GuidedInstallFileEntry {
            file_id: Some(file.file_id),
            filename: file.filename.clone(),
            current_path: file.path.clone(),
            target_path: validation.final_absolute_path.clone(),
            archive_member_path: file.archive_member_path.clone(),
            kind: file.kind.clone(),
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
            target_path: Some(
                if file.in_target_folder {
                    file.path.clone()
                } else {
                    layout
                        .target_folder
                        .join(&file.filename)
                        .to_string_lossy()
                        .to_string()
                },
            ),
            archive_member_path: None,
            kind: "Config".to_owned(),
            subtype: None,
            creator: file.creator.clone().or_else(|| profile.creator.clone()),
            notes: vec![if file.in_target_folder {
                "Settings or sidecar file that will stay in place during the update.".to_owned()
            } else {
                "Settings or sidecar file that SimSuite will gather into the safe folder before the update.".to_owned()
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
            "SimSuite will tidy the older {} setup into one safe folder before it installs this update.",
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
    let evaluation = evaluate_download_item(connection, settings, seed_pack, item_id)?;
    if evaluation.assessment.intake_mode == DownloadIntakeMode::Standard {
        return Ok(None);
    }

    let guided_plan = if evaluation.assessment.intake_mode == DownloadIntakeMode::Guided {
        build_guided_plan(connection, settings, seed_pack, item_id)?
    } else {
        None
    };

    if evaluation.assessment.intake_mode == DownloadIntakeMode::Guided
        && guided_plan.as_ref().is_none_or(|plan| plan.apply_ready)
    {
        return Ok(None);
    }

    let files = load_profile_files(connection, item_id, true)?;
    let repair_guided_plan = build_repair_guided_plan(connection, settings, seed_pack, item_id)?;
    let repair_layout = evaluation
        .matched_profile
        .as_ref()
        .map(|profile| detect_existing_layout(connection, settings, profile))
        .transpose()?;
    let review_files = guided_plan
        .as_ref()
        .map(|plan| plan.review_files.clone())
        .filter(|files| !files.is_empty())
        .unwrap_or_else(|| {
            files.iter()
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

    let (repair_plan_available, repair_action_label, repair_reason, repair_target_folder,
        repair_move_files, repair_replace_files, repair_keep_files, repair_warnings,
        repair_can_continue_install) = if let (Some(profile), Some(layout)) =
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
                target_path: Some(
                    if file.in_target_folder {
                        file.path.clone()
                    } else {
                        layout
                            .target_folder
                            .join(&file.filename)
                            .to_string_lossy()
                            .to_string()
                    },
                ),
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
                    "SimSuite can gather the older {} files into one safe folder and then continue with the update.",
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

    let available_actions =
        build_available_review_actions(seed_pack, &evaluation, &files, repair_layout.as_ref());
    let recommended_next_step = if evaluation.assessment.intake_mode == DownloadIntakeMode::Guided {
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
    };
    let explanation = if evaluation.assessment.intake_mode == DownloadIntakeMode::Guided {
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
    };

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

    let files = load_profile_files(connection, item_id, true)?;
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
    let files = load_profile_files(connection, item_id, true)?;
    let text_clues = read_text_clues(item.staging_path.as_deref(), &files)?;
    let archive_path_clues = collect_archive_path_clues(&files);
    let review_patterns =
        collect_review_patterns(seed_pack, &item, &files, &text_clues, &archive_path_clues);
    let candidates =
        collect_guided_candidates(seed_pack, &item, &files, &text_clues, &archive_path_clues);
    let strong_candidates = candidates
        .iter()
        .filter(|candidate| candidate.evidence.strong_match())
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
        .max_by_key(|candidate| candidate.evidence.score());
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
        let layout = detect_existing_layout(connection, settings, &candidate.profile)?;
        let dependency_rules =
            collect_required_dependency_rules(seed_pack, &candidate.profile, &matched_dependency_rules);
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

        if !candidate.evidence.strong_match() {
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
            dependencies,
            existing_layout_findings: layout.warnings,
            explanation: candidate.profile.help_summary.clone(),
            recommended_next_step:
                "Review the guided plan, then approve the special setup if everything looks right."
                    .to_owned(),
        });
    }

    let dependencies =
        resolve_dependency_status(connection, settings, seed_pack, item_id, &matched_dependency_rules)?;
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
        dependencies,
        existing_layout_findings: Vec::new(),
        explanation:
            "This looks like a normal download, so it can use the standard safe hand-off preview."
                .to_owned(),
        recommended_next_step: "Open the normal hand-off preview and review the safe batch.".to_owned(),
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
    let required_core_present = files
        .iter()
        .any(|file| matches_required_core(&file.filename, &file.extension, &profile.required_name_clues));
    let matched_files = files
        .iter()
        .filter(|file| is_profile_content_file(file, profile))
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
        unmatched_supported_files,
        required_core_present,
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
            let summary = if item.guided_install_available && item.intake_mode == DownloadIntakeMode::Guided
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
                format!("{} is required before this mod can be installed safely.", rule.display_name),
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
        .filter(|rule| matches_incompatibility_rule(rule, item, files, text_clues, archive_path_clues))
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

    for rule in matched_rules
        .iter()
        .chain(seed_pack.install_catalog.incompatibility_rules.iter().filter(|rule| {
            profile.incompatibility_keys.iter().any(|key| key == &rule.key)
        }))
    {
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
                || !collect_matched_clues(archive_path_clues, &pattern.archive_path_clues).is_empty()
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
        return detect_existing_layout(connection, settings, profile)
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
    profile: &GuidedInstallProfileSeed,
) -> AppResult<ExistingInstallLayout> {
    let mods_root = settings
        .mods_path
        .as_ref()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .ok_or_else(|| {
            AppError::Message("Set a Mods folder before using special installs.".to_owned())
        })?;
    let target_folder = mods_root.join(&profile.install_folder_name);
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
                insights: serde_json::from_str(&row.get::<_, String>(7)?).unwrap_or_default(),
                in_target_folder: false,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    let mut existing_files = Vec::new();
    let mut preserve_files = Vec::new();
    for mut file in installed_files {
        let in_target = path_starts_with(&file.path, &target_folder);
        file.in_target_folder = in_target;
        let matches_profile = is_existing_profile_file(&file, profile);
        if matches_profile {
            existing_install_detected = true;
            if !in_target {
                safe_to_update = false;
                scattered_match_count += 1;
            }
            existing_files.push(file);
        } else if is_related_preserve_file(&file.filename, &file.extension, in_target, profile) {
            existing_install_detected = true;
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
    for preserve_path in scan_preserve_files(&target_folder, profile)? {
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

    let foreign_target_files = scan_foreign_target_files(&target_folder, profile)?;
    if !foreign_target_files.is_empty() {
        safe_to_update = false;
        warnings.push(format!(
            "The target {} folder already contains files that do not belong to {}.",
            profile.install_folder_name, profile.display_name
        ));
    }

    let repair_plan_available =
        (scattered_match_count > 0 || scattered_preserve_count > 0) && foreign_target_files.is_empty();

    Ok(ExistingInstallLayout {
        target_folder,
        existing_files,
        preserve_files,
        warnings,
        existing_install_detected,
        safe_to_update,
        repair_plan_available,
    })
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
    item_id: i64,
    active_only: bool,
) -> AppResult<Vec<ProfileFile>> {
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
            COALESCE(f.insights, '{{}}')
         FROM files f
         LEFT JOIN creators c ON c.id = f.creator_id
         WHERE f.download_item_id = ?1
           {location_filter}
         ORDER BY f.filename COLLATE NOCASE"
    );
    let mut statement = connection.prepare(&sql)?;
    let rows = statement
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
                insights: serde_json::from_str(&row.get::<_, String>(10)?).unwrap_or_default(),
            })
        })?
        .collect::<Result<Vec<_>, _>>()
        .map_err(AppError::from)?;
    Ok(rows)
}

fn read_text_clues(staging_path: Option<&str>, files: &[ProfileFile]) -> AppResult<Vec<String>> {
    let mut clues = Vec::new();

    for file in files {
        clues.push(normalized(&file.filename));
        if let Some(member_path) = &file.archive_member_path {
            clues.push(normalized(member_path));
        }
        clues.extend(file.insights.embedded_names.iter().map(|value| normalized(value)));
        clues.extend(file.insights.resource_summary.iter().map(|value| normalized(value)));
        clues.extend(file.insights.creator_hints.iter().map(|value| normalized(value)));
        clues.extend(file.insights.script_namespaces.iter().map(|value| normalized(value)));
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

fn collect_extra_supported_files<'a>(
    files: &'a [ProfileFile],
    profile: &GuidedInstallProfileSeed,
) -> Vec<&'a ProfileFile> {
    files.iter()
        .filter(|file| is_supported_special_extension(&file.extension))
        .filter(|file| !is_profile_content_file(file, profile))
        .collect()
}

fn has_supported_subset_to_separate(
    files: &[ProfileFile],
    profile: &GuidedInstallProfileSeed,
) -> bool {
    files.iter().any(|file| is_profile_content_file(file, profile))
        && has_required_core(files, profile)
        && !collect_extra_supported_files(files, profile).is_empty()
}

fn is_profile_content_file(file: &ProfileFile, profile: &GuidedInstallProfileSeed) -> bool {
    is_profile_content_name(&file.filename, &file.extension, profile)
}

fn is_existing_profile_file(file: &ExistingInstallFile, profile: &GuidedInstallProfileSeed) -> bool {
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

    in_target_folder || name_matches_profile(filename, profile)
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
    let mut inputs = vec![normalized(&item.display_name), normalized(&item.source_path)];
    for file in files {
        inputs.push(normalized(&file.filename));
        if let Some(member_path) = &file.archive_member_path {
            inputs.push(normalized(member_path));
        }
    }
    inputs
}

fn build_evidence_summary(evidence: &ProfileEvidence) -> Vec<String> {
    let mut summary = Vec::new();
    if evidence.matched_files > 0 {
        summary.push(format!(
            "{} matching special-mod files were detected.",
            evidence.matched_files
        ));
    }
    if evidence.required_core_present {
        summary.push("Required core files were found.".to_owned());
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
                    "Tuck the older {} files into one safe folder, keep the settings files safe, and continue the update.",
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

        let has_profile_files = files.iter().any(|file| is_profile_content_file(file, profile));
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
) -> AppResult<Vec<PathBuf>> {
    if !target_folder.exists() {
        return Ok(Vec::new());
    }

    let mut preserve_paths = Vec::new();
    for entry in WalkDir::new(target_folder)
        .max_depth(3)
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
) -> AppResult<Vec<PathBuf>> {
    if !target_folder.exists() {
        return Ok(Vec::new());
    }

    let mut foreign = Vec::new();
    for entry in WalkDir::new(target_folder)
        .max_depth(3)
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
        reference_source: profile.reference_source.clone(),
        reviewed_at: Some(profile.reviewed_at.clone()),
    }
}

fn pattern_catalog_source(pattern: &ReviewOnlyPatternSeed) -> CatalogSourceInfo {
    CatalogSourceInfo {
        official_source_url: pattern.official_source_url.clone(),
        official_download_url: None,
        reference_source: pattern.reference_source.clone(),
        reviewed_at: Some(pattern.reviewed_at.clone()),
    }
}

fn dependency_catalog_source(rule: &DependencyRuleSeed) -> CatalogSourceInfo {
    CatalogSourceInfo {
        official_source_url: Some(rule.official_source_url.clone()),
        official_download_url: rule.official_download_url.clone(),
        reference_source: rule.reference_source.clone(),
        reviewed_at: Some(rule.reviewed_at.clone()),
    }
}

fn incompatibility_catalog_source(rule: &IncompatibilityRuleSeed) -> CatalogSourceInfo {
    CatalogSourceInfo {
        official_source_url: Some(rule.official_source_url.clone()),
        official_download_url: None,
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

fn path_starts_with(path: &str, root: &Path) -> bool {
    normalized(path).contains(&normalized(&root.to_string_lossy()))
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
        normalized, store_download_item_assessment,
    };
    use crate::{
        database::{initialize, save_library_paths, seed_database},
        models::{DownloadIntakeMode, LibrarySettings, ReviewPlanActionKind},
        seed::{load_seed_pack, GuidedInstallProfileSeed, SeedPack},
    };
    use rusqlite::{params, Connection};
    use std::{fs, path::{Path, PathBuf}};
    use tempfile::tempdir;

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
                params![path.to_string_lossy().to_string(), filename, extension, kind],
            )
            .expect("installed file");
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
        insert_download_file(connection, item_id, item_id * 100 + 1, &file_path, filename, kind);
    }

    fn install_target_for_profile(temp_root: &Path, profile: &GuidedInstallProfileSeed) -> PathBuf {
        temp_root.join("Mods").join(&profile.install_folder_name)
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
                let preserve = target.join(format!("{}_settings{}", profile.key, profile.preserve_extensions[0]));
                fs::write(&preserve, b"keep").expect("preserve");
            }

            let plan = build_guided_plan(&connection, &settings, &seed_pack, item_id)
                .expect("plan")
                .expect("guided");
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
    fn creator_named_regular_mods_stay_on_standard_path() {
        let (_temp, connection, seed_pack, settings) = setup_env();
        let staging_root = PathBuf::from(settings.downloads_path.clone().expect("downloads"));

        let near_misses = [
            ("Lot51_Normal_Set.zip", "lot51_table.package", "BuildBuy"),
            ("ColonolNutty_Shelf.zip", "colonolnutty_bookshelf.package", "BuildBuy"),
            ("Lumpinou_Relationship_Overhaul.zip", "lumpinou_relationship_overhaul.package", "Gameplay"),
            ("Andirz_Custom_Trait.zip", "andirz_custom_trait.package", "Gameplay"),
            ("Triplis_Cafe_Set.zip", "triplis_cafe_counter.package", "BuildBuy"),
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
            let assessment =
                assess_download_item(&connection, &settings, &seed_pack, item_id).expect("assessment");
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
        assert!(action
            .url
            .as_deref()
            .is_some_and(|url| url.contains("deaderpool-mccc.com") || url.contains("drive.google.com")));
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
            Some("https://deaderpool-mccc.com/installation.html")
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
        assert_eq!(dependency.inbox_item_name.as_deref(), Some("XML_Injector.zip"));
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
    fn scattered_existing_install_blocks_guided_update() {
        let (temp, connection, seed_pack, settings) = setup_env();
        let staging_root = PathBuf::from(settings.downloads_path.clone().expect("downloads"));
        let profile = seed_pack
            .install_catalog
            .guided_profiles
            .iter()
            .find(|profile| profile.key == "mccc")
            .expect("mccc");
        build_sample_download(&staging_root, &connection, 227, profile);

        let scattered = temp.path().join("Mods").join("Gameplay").join("mc_cmd_center.ts4script");
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
    fn guided_items_with_held_files_return_a_review_plan() {
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
        assert_eq!(assessment.intake_mode, DownloadIntakeMode::Guided);

        let guided_plan = build_guided_plan(&connection, &settings, &seed_pack, 231)
            .expect("guided plan")
            .expect("guided");
        assert!(!guided_plan.apply_ready);
        assert_eq!(guided_plan.review_files.len(), 1);

        let review_plan = build_review_plan(&connection, &settings, &seed_pack, 231)
            .expect("review plan")
            .expect("review");
        assert_eq!(review_plan.mode, DownloadIntakeMode::Guided);
        assert_eq!(review_plan.review_files.len(), 1);
        assert!(review_plan
            .available_actions
            .iter()
            .any(|action| action.kind == ReviewPlanActionKind::OpenOfficialSource));
    }

    #[test]
    fn normalized_helper_collapses_symbol_noise() {
        assert_eq!(normalized("[MCCC] MC_Command_Center"), " mccc  mc command center");
    }

    #[test]
    fn evidence_summary_lists_core_signals() {
        let summary = build_evidence_summary(&super::ProfileEvidence {
            reasons: Vec::new(),
            matched_files: 2,
            unmatched_supported_files: 1,
            required_core_present: true,
            name_match: false,
            text_matches: vec!["xml injector".to_owned()],
            archive_matches: vec!["pick one".to_owned()],
        });
        assert!(summary
            .iter()
            .any(|value| value.contains("matching special-mod files")));
        assert!(summary.iter().any(|value| value.contains("Required core files")));
        assert!(summary.iter().any(|value| value.contains("xml injector")));
    }
}
