use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
};

use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use sha2::Digest;

use crate::{
    core::{
        bundle_detector, downloads_watcher, duplicate_detector, file_inspector::inspect_file,
        filename_parser::parse_filename, install_profile_engine, rule_engine, snapshot_manager,
    },
    database,
    error::{AppError, AppResult},
    models::{
        ApplyGuidedDownloadResult, ApplyPreviewResult, ApplySpecialReviewFixResult,
        GuidedInstallPlan, LibrarySettings, OrganizationPreview, RestoreSnapshotResult,
    },
    seed::SeedPack,
};

pub fn apply_preview_moves(
    connection: &mut Connection,
    settings: &LibrarySettings,
    preset_name: Option<String>,
    _limit: i64,
    approved: bool,
) -> AppResult<ApplyPreviewResult> {
    apply_preview_moves_internal(connection, settings, preset_name, None, approved)
}

pub fn apply_preview_moves_for_files(
    connection: &mut Connection,
    settings: &LibrarySettings,
    preset_name: Option<String>,
    file_ids: &[i64],
    approved: bool,
) -> AppResult<ApplyPreviewResult> {
    if !approved {
        return Err(AppError::Message(
            "Apply was blocked because approval was not confirmed.".to_owned(),
        ));
    }

    let preview =
        rule_engine::build_preview_for_files(connection, settings, preset_name.clone(), file_ids)?;
    apply_preview_moves_internal(connection, settings, preset_name, Some(preview), approved)
}

pub fn apply_guided_download_plan(
    connection: &mut Connection,
    settings: &LibrarySettings,
    seed_pack: &SeedPack,
    app_data_dir: &Path,
    plan: &GuidedInstallPlan,
    approved: bool,
) -> AppResult<ApplyGuidedDownloadResult> {
    if !approved {
        return Err(AppError::Message(
            "Apply was blocked because approval was not confirmed.".to_owned(),
        ));
    }

    if !plan.apply_ready {
        return Err(AppError::Message(
            "This guided install still needs review before it can be applied.".to_owned(),
        ));
    }

    let incoming_moves = plan
        .install_files
        .iter()
        .map(|file| {
            let file_id = file.file_id.ok_or_else(|| {
                AppError::Message("Guided install file is missing its staged file id.".to_owned())
            })?;
            let target_path = file.target_path.clone().ok_or_else(|| {
                AppError::Message("Guided install file is missing its target path.".to_owned())
            })?;
            Ok(PlannedMove {
                file_id,
                current_path: PathBuf::from(&file.current_path),
                final_path: PathBuf::from(target_path),
                kind: file.kind.clone(),
            })
        })
        .collect::<AppResult<Vec<_>>>()?;

    let replace_targets = plan
        .replace_files
        .iter()
        .map(|file| {
            Ok(ReplacementMove {
                file_id: file.file_id,
                original_path: PathBuf::from(&file.current_path),
                backup_path: guided_backup_path(app_data_dir, plan.item_id, &file.filename),
            })
        })
        .collect::<AppResult<Vec<_>>>()?;

    preflight_guided_incoming(&incoming_moves, &replace_targets)?;
    preflight_replacements(&replace_targets, &incoming_moves)?;
    database::record_download_item_event(
        connection,
        plan.item_id,
        "apply_started",
        "Guided install started",
        Some(&format!(
            "Preparing the {} guided install.",
            plan.profile_name
        )),
    )?;

    let snapshot_name = format!(
        "{} Guided Install {}",
        plan.profile_name,
        Utc::now().format("%Y-%m-%d %H:%M:%S")
    );
    let mut snapshot_items = replace_targets
        .iter()
        .map(|item| {
            Ok(snapshot_manager::SnapshotItemRecord {
                file_id: None,
                original_path: item.original_path.to_string_lossy().to_string(),
                original_hash: Some(file_hash(&item.original_path)?),
                backup_path: Some(item.backup_path.to_string_lossy().to_string()),
            })
        })
        .collect::<AppResult<Vec<_>>>()?;
    snapshot_items.extend(
        incoming_moves
            .iter()
            .map(|item| {
                Ok(snapshot_manager::SnapshotItemRecord {
                    file_id: Some(item.file_id),
                    original_path: item.current_path.to_string_lossy().to_string(),
                    original_hash: Some(file_hash(&item.current_path)?),
                    backup_path: None,
                })
            })
            .collect::<AppResult<Vec<_>>>()?,
    );

    let snapshot = snapshot_manager::create_snapshot(
        connection,
        &snapshot_name,
        Some("Auto-created before guided special install"),
        &snapshot_items,
    )?;

    let mut moved_incoming = Vec::new();
    let mut moved_backups = Vec::new();

    for item in &replace_targets {
        if let Err(error) = move_single_file(&item.original_path, &item.backup_path) {
            rollback_guided_changes(
                connection,
                settings,
                seed_pack,
                &moved_incoming,
                &moved_backups,
            )?;
            delete_snapshot(connection, snapshot.id)?;
            return Err(error);
        }

        if let Err(error) =
            update_file_record_on_backup(connection, item.file_id, &item.backup_path)
        {
            rollback_guided_changes(
                connection,
                settings,
                seed_pack,
                &moved_incoming,
                &moved_backups,
            )?;
            delete_snapshot(connection, snapshot.id)?;
            return Err(error);
        }
        moved_backups.push(item.clone());
    }

    for item in &incoming_moves {
        if let Err(error) = move_single_file(&item.current_path, &item.final_path) {
            rollback_guided_changes(
                connection,
                settings,
                seed_pack,
                &moved_incoming,
                &moved_backups,
            )?;
            delete_snapshot(connection, snapshot.id)?;
            return Err(error);
        }
        moved_incoming.push(item.clone());
        if let Err(error) = update_file_record_after_move(connection, settings, item) {
            rollback_guided_changes(
                connection,
                settings,
                seed_pack,
                &moved_incoming,
                &moved_backups,
            )?;
            delete_snapshot(connection, snapshot.id)?;
            return Err(error);
        }
    }

    for item in &replace_targets {
        if let Err(error) = delete_file_record(connection, item.file_id) {
            rollback_guided_changes(
                connection,
                settings,
                seed_pack,
                &moved_incoming,
                &moved_backups,
            )?;
            delete_snapshot(connection, snapshot.id)?;
            return Err(error);
        }
    }

    bundle_detector::rebuild_bundles(connection)?;
    duplicate_detector::rebuild_duplicates(connection)?;
    let affected_item_ids = install_profile_engine::reconcile_special_mod_family(
        connection,
        settings,
        seed_pack,
        &plan.profile_key,
        plan.item_id,
    )?;
    database::record_download_item_event(
        connection,
        plan.item_id,
        "applied",
        "Guided install applied",
        Some(&format!(
            "{} moved {} file(s) into place.",
            plan.profile_name,
            incoming_moves.len()
        )),
    )?;
    for affected_item_id in affected_item_ids {
        if affected_item_id != plan.item_id {
            database::record_download_item_event(
                connection,
                affected_item_id,
                "superseded",
                "Superseded by a newer family item",
                Some(&format!(
                    "{} now uses a newer or repaired family item.",
                    plan.profile_name
                )),
            )?;
        }
        downloads_watcher::refresh_download_item_status(connection, affected_item_id)?;
    }

    Ok(ApplyGuidedDownloadResult {
        snapshot_id: snapshot.id,
        installed_count: incoming_moves.len() as i64,
        replaced_count: replace_targets.len() as i64,
        preserved_count: plan.preserve_files.len() as i64,
        deferred_review_count: plan.review_files.len() as i64,
        snapshot_name: snapshot.snapshot_name,
    })
}

pub fn apply_special_review_fix(
    connection: &mut Connection,
    settings: &LibrarySettings,
    seed_pack: &SeedPack,
    app_data_dir: &Path,
    item_id: i64,
    approved: bool,
) -> AppResult<ApplySpecialReviewFixResult> {
    if !approved {
        return Err(AppError::Message(
            "Repair was blocked because approval was not confirmed.".to_owned(),
        ));
    }

    let review_plan =
        install_profile_engine::build_review_plan(connection, settings, seed_pack, item_id)?
            .ok_or_else(|| {
                AppError::Message("This inbox item does not have a special review plan.".to_owned())
            })?;
    if !review_plan.repair_plan_available {
        return Err(AppError::Message(
            "This inbox item does not have a safe repair plan yet.".to_owned(),
        ));
    }

    let guided_plan =
        install_profile_engine::build_repair_guided_plan(connection, settings, seed_pack, item_id)?
            .ok_or_else(|| {
                AppError::Message("SimSuite could not build the safe repair plan.".to_owned())
            })?;

    let incoming_moves = guided_plan
        .install_files
        .iter()
        .map(|file| {
            let file_id = file.file_id.ok_or_else(|| {
                AppError::Message("Special install file is missing its staged file id.".to_owned())
            })?;
            let target_path = file.target_path.clone().ok_or_else(|| {
                AppError::Message("Special install file is missing its target path.".to_owned())
            })?;
            Ok(PlannedMove {
                file_id,
                current_path: PathBuf::from(&file.current_path),
                final_path: PathBuf::from(target_path),
                kind: file.kind.clone(),
            })
        })
        .collect::<AppResult<Vec<_>>>()?;

    let preserve_moves = review_plan
        .repair_keep_files
        .iter()
        .filter_map(|file| {
            let file_id = file.file_id?;
            let target_path = file.target_path.as_ref()?;
            if normalized_path_key(Path::new(&file.current_path))
                == normalized_path_key(Path::new(target_path))
            {
                return None;
            }

            Some(Ok(PlannedMove {
                file_id,
                current_path: PathBuf::from(&file.current_path),
                final_path: PathBuf::from(target_path),
                kind: "Config".to_owned(),
            }))
        })
        .collect::<AppResult<Vec<_>>>()?;

    let mut all_moves = preserve_moves.clone();
    all_moves.extend(incoming_moves.clone());

    let replace_targets = guided_plan
        .replace_files
        .iter()
        .map(|file| {
            Ok(ReplacementMove {
                file_id: file.file_id,
                original_path: PathBuf::from(&file.current_path),
                backup_path: guided_backup_path(app_data_dir, item_id, &file.filename),
            })
        })
        .collect::<AppResult<Vec<_>>>()?;

    preflight_guided_incoming(&all_moves, &replace_targets)?;
    preflight_special_review_replacements(&replace_targets)?;
    database::record_download_item_event(
        connection,
        item_id,
        "apply_started",
        "Special repair started",
        Some("Preparing the safe repair pass for this special-mod family."),
    )?;

    let snapshot_name = format!(
        "{} Repair {}",
        review_plan
            .profile_name
            .clone()
            .unwrap_or_else(|| "Special Install".to_owned()),
        Utc::now().format("%Y-%m-%d %H:%M:%S")
    );
    let mut snapshot_items = replace_targets
        .iter()
        .map(|item| {
            Ok(snapshot_manager::SnapshotItemRecord {
                file_id: None,
                original_path: item.original_path.to_string_lossy().to_string(),
                original_hash: Some(file_hash(&item.original_path)?),
                backup_path: Some(item.backup_path.to_string_lossy().to_string()),
            })
        })
        .collect::<AppResult<Vec<_>>>()?;
    snapshot_items.extend(
        all_moves
            .iter()
            .map(|item| {
                Ok(snapshot_manager::SnapshotItemRecord {
                    file_id: Some(item.file_id),
                    original_path: item.current_path.to_string_lossy().to_string(),
                    original_hash: Some(file_hash(&item.current_path)?),
                    backup_path: None,
                })
            })
            .collect::<AppResult<Vec<_>>>()?,
    );

    let snapshot = snapshot_manager::create_snapshot(
        connection,
        &snapshot_name,
        Some("Auto-created before a special-mod repair"),
        &snapshot_items,
    )?;

    let mut moved_files = Vec::new();
    let mut moved_backups = Vec::new();

    for item in &replace_targets {
        if let Err(error) = move_single_file(&item.original_path, &item.backup_path) {
            rollback_guided_changes(
                connection,
                settings,
                seed_pack,
                &moved_files,
                &moved_backups,
            )?;
            delete_snapshot(connection, snapshot.id)?;
            return Err(error);
        }

        if let Err(error) =
            update_file_record_on_backup(connection, item.file_id, &item.backup_path)
        {
            rollback_guided_changes(
                connection,
                settings,
                seed_pack,
                &moved_files,
                &moved_backups,
            )?;
            delete_snapshot(connection, snapshot.id)?;
            return Err(error);
        }
        moved_backups.push(item.clone());
    }

    for item in &all_moves {
        if let Err(error) = move_single_file(&item.current_path, &item.final_path) {
            rollback_guided_changes(
                connection,
                settings,
                seed_pack,
                &moved_files,
                &moved_backups,
            )?;
            delete_snapshot(connection, snapshot.id)?;
            return Err(error);
        }
        moved_files.push(item.clone());
        if let Err(error) = update_file_record_after_move(connection, settings, item) {
            rollback_guided_changes(
                connection,
                settings,
                seed_pack,
                &moved_files,
                &moved_backups,
            )?;
            delete_snapshot(connection, snapshot.id)?;
            return Err(error);
        }
    }

    for item in &replace_targets {
        if let Err(error) = delete_file_record(connection, item.file_id) {
            rollback_guided_changes(
                connection,
                settings,
                seed_pack,
                &moved_files,
                &moved_backups,
            )?;
            delete_snapshot(connection, snapshot.id)?;
            return Err(error);
        }
    }

    bundle_detector::rebuild_bundles(connection)?;
    duplicate_detector::rebuild_duplicates(connection)?;
    let profile_key = review_plan
        .profile_key
        .clone()
        .unwrap_or_else(|| guided_plan.profile_key.clone());
    let profile_name = review_plan
        .profile_name
        .clone()
        .unwrap_or_else(|| guided_plan.profile_name.clone());
    let affected_item_ids = install_profile_engine::reconcile_special_mod_family(
        connection,
        settings,
        seed_pack,
        &profile_key,
        item_id,
    )?;
    database::record_download_item_event(
        connection,
        item_id,
        "applied",
        "Special repair applied",
        Some(&format!(
            "{} repaired {} older file(s) and refreshed the family state.",
            profile_name,
            review_plan.repair_move_files.len()
        )),
    )?;
    for affected_item_id in affected_item_ids {
        if affected_item_id != item_id {
            database::record_download_item_event(
                connection,
                affected_item_id,
                "superseded",
                "Superseded by a repaired family item",
                Some(&format!(
                    "{} now follows the repaired family state.",
                    profile_name
                )),
            )?;
        }
        downloads_watcher::refresh_download_item_status(connection, affected_item_id)?;
    }

    Ok(ApplySpecialReviewFixResult {
        snapshot_id: snapshot.id,
        repaired_count: review_plan.repair_move_files.len() as i64,
        installed_count: incoming_moves.len() as i64,
        replaced_count: replace_targets.len() as i64,
        preserved_count: review_plan.repair_keep_files.len() as i64,
        deferred_review_count: guided_plan.review_files.len() as i64,
        snapshot_name: snapshot.snapshot_name,
    })
}

fn apply_preview_moves_internal(
    connection: &mut Connection,
    settings: &LibrarySettings,
    preset_name: Option<String>,
    preview: Option<OrganizationPreview>,
    approved: bool,
) -> AppResult<ApplyPreviewResult> {
    if !approved {
        return Err(AppError::Message(
            "Apply was blocked because approval was not confirmed.".to_owned(),
        ));
    }

    let preview = match preview {
        Some(preview) => preview,
        None => rule_engine::build_preview_full(connection, settings, preset_name.clone())?,
    };
    let actionable = preview
        .suggestions
        .into_iter()
        .filter(|item| !item.review_required)
        .filter_map(|item| {
            item.final_absolute_path
                .clone()
                .filter(|destination| destination != &item.current_path)
                .map(|destination| PlannedMove {
                    file_id: item.file_id,
                    current_path: PathBuf::from(item.current_path),
                    final_path: PathBuf::from(destination),
                    kind: item.kind,
                })
        })
        .collect::<Vec<_>>();

    if actionable.is_empty() {
        return Err(AppError::Message(
            "No safe preview moves are ready to apply.".to_owned(),
        ));
    }

    preflight_moves(&actionable)?;

    let snapshot_name = format!(
        "{} Preview {}",
        preset_name.unwrap_or_else(|| "Category First".to_owned()),
        Utc::now().format("%Y-%m-%d %H:%M:%S")
    );
    let snapshot = snapshot_manager::create_snapshot(
        connection,
        &snapshot_name,
        Some("Auto-created before approved preview batch"),
        &actionable
            .iter()
            .map(|item| {
                Ok(snapshot_manager::SnapshotItemRecord {
                    file_id: Some(item.file_id),
                    original_path: item.current_path.to_string_lossy().to_string(),
                    original_hash: Some(file_hash(&item.current_path)?),
                    backup_path: None,
                })
            })
            .collect::<AppResult<Vec<_>>>()?,
    )?;

    let mut applied_moves = Vec::new();
    for item in &actionable {
        if let Err(error) = move_single_file(&item.current_path, &item.final_path) {
            rollback_applied_moves(connection, settings, &applied_moves)?;
            delete_snapshot(connection, snapshot.id)?;
            return Err(error);
        }

        applied_moves.push(item.clone());
        if let Err(error) = update_file_record_after_move(connection, settings, item) {
            rollback_applied_moves(connection, settings, &applied_moves)?;
            delete_snapshot(connection, snapshot.id)?;
            return Err(error);
        }
    }

    let moved_count = applied_moves.len() as i64;
    Ok(ApplyPreviewResult {
        snapshot_id: snapshot.id,
        moved_count,
        deferred_review_count: preview.review_count,
        skipped_count: preview.total_considered - moved_count - preview.review_count,
        snapshot_name: snapshot.snapshot_name,
    })
}

pub fn restore_snapshot(
    connection: &mut Connection,
    seed_pack: &SeedPack,
    snapshot_id: i64,
    approved: bool,
) -> AppResult<RestoreSnapshotResult> {
    if !approved {
        return Err(AppError::Message(
            "Rollback was blocked because approval was not confirmed.".to_owned(),
        ));
    }

    rollback_snapshot_internal(connection, seed_pack, snapshot_id)
}

fn rollback_snapshot_internal(
    connection: &mut Connection,
    seed_pack: &SeedPack,
    snapshot_id: i64,
) -> AppResult<RestoreSnapshotResult> {
    let settings = database::get_library_settings(connection)?;
    let mut statement = connection.prepare(
        "SELECT file_id, original_path, original_hash, backup_path
         FROM snapshot_items
         WHERE snapshot_id = ?1
         ORDER BY id DESC",
    )?;
    let items = statement
        .query_map(params![snapshot_id], |row| {
            Ok((
                row.get::<_, Option<i64>>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, Option<String>>(3)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    drop(statement);

    let mut restored_count = 0_i64;
    let mut skipped_count = 0_i64;

    for (file_id, original_path, original_hash, backup_path) in items {
        let Some(file_id) = file_id else {
            if let Some(backup_path) = backup_path {
                if restore_backed_up_file(
                    connection,
                    &settings,
                    seed_pack,
                    Path::new(&backup_path),
                    Path::new(&original_path),
                    original_hash.as_deref(),
                )? {
                    restored_count += 1;
                } else {
                    skipped_count += 1;
                }
            } else {
                skipped_count += 1;
            }
            continue;
        };

        let current_path: Option<String> = connection
            .query_row(
                "SELECT path FROM files WHERE id = ?1",
                params![file_id],
                |row| row.get(0),
            )
            .optional()?;

        let Some(current_path) = current_path else {
            skipped_count += 1;
            continue;
        };

        if current_path == original_path {
            skipped_count += 1;
            continue;
        }

        let current = PathBuf::from(&current_path);
        let original = PathBuf::from(&original_path);

        if !current.exists() {
            skipped_count += 1;
            continue;
        }

        if let Some(expected_hash) = original_hash.as_deref() {
            let current_hash = file_hash(&current)?;
            if current_hash != expected_hash {
                return Err(AppError::Message(format!(
                    "Rollback blocked because file contents changed for {}",
                    current.display()
                )));
            }
        }

        if original.exists() && original != current {
            return Err(AppError::Message(format!(
                "Rollback blocked because original destination already exists: {}",
                original.display()
            )));
        }

        move_single_file(&current, &original)?;
        update_file_record_on_restore(connection, &settings, file_id, &original)?;
        restored_count += 1;
    }

    bundle_detector::rebuild_bundles(connection)?;
    duplicate_detector::rebuild_duplicates(connection)?;

    Ok(RestoreSnapshotResult {
        snapshot_id,
        restored_count,
        skipped_count,
    })
}

fn rollback_applied_moves(
    connection: &Connection,
    settings: &LibrarySettings,
    applied_moves: &[PlannedMove],
) -> AppResult<()> {
    for item in applied_moves.iter().rev() {
        if item.final_path.exists() && !item.current_path.exists() {
            move_single_file(&item.final_path, &item.current_path)?;
        }

        update_file_record_on_restore(connection, settings, item.file_id, &item.current_path)?;
    }

    Ok(())
}

fn rollback_guided_changes(
    connection: &mut Connection,
    settings: &LibrarySettings,
    seed_pack: &SeedPack,
    incoming_moves: &[PlannedMove],
    backup_moves: &[ReplacementMove],
) -> AppResult<()> {
    rollback_applied_moves(connection, settings, incoming_moves)?;

    for item in backup_moves.iter().rev() {
        if item.backup_path.exists() && !item.original_path.exists() {
            move_single_file(&item.backup_path, &item.original_path)?;
        }

        let existing_record = item
            .file_id
            .map(|file_id| {
                connection
                    .query_row(
                        "SELECT id FROM files WHERE id = ?1",
                        params![file_id],
                        |row| row.get::<_, i64>(0),
                    )
                    .optional()
            })
            .transpose()?
            .flatten();

        if let Some(file_id) = existing_record {
            update_file_record_on_restore(connection, settings, file_id, &item.original_path)?;
        } else if item.original_path.exists() {
            restore_deleted_file_record(connection, settings, seed_pack, &item.original_path)?;
        }
    }

    Ok(())
}

fn preflight_moves(moves: &[PlannedMove]) -> AppResult<()> {
    for item in moves {
        if !item.current_path.exists() {
            return Err(AppError::Message(format!(
                "Move blocked because source file is missing: {}",
                item.current_path.display()
            )));
        }

        if item.final_path.exists() {
            return Err(AppError::Message(format!(
                "Move blocked because destination already exists: {}",
                item.final_path.display()
            )));
        }
    }

    Ok(())
}

fn preflight_guided_incoming(
    incoming_moves: &[PlannedMove],
    replacements: &[ReplacementMove],
) -> AppResult<()> {
    let replaceable_targets = replacements
        .iter()
        .map(|item| normalized_path_key(&item.original_path))
        .collect::<HashSet<_>>();

    for item in incoming_moves {
        if !item.current_path.exists() {
            return Err(AppError::Message(format!(
                "Move blocked because source file is missing: {}",
                item.current_path.display()
            )));
        }

        if item.final_path.exists()
            && !replaceable_targets.contains(&normalized_path_key(&item.final_path))
        {
            return Err(AppError::Message(format!(
                "Move blocked because destination already exists: {}",
                item.final_path.display()
            )));
        }
    }

    Ok(())
}

fn preflight_replacements(
    replacements: &[ReplacementMove],
    incoming_moves: &[PlannedMove],
) -> AppResult<()> {
    let incoming_targets = incoming_moves
        .iter()
        .map(|item| normalized_path_key(&item.final_path))
        .collect::<HashSet<_>>();

    for item in replacements {
        if !item.original_path.exists() {
            return Err(AppError::Message(format!(
                "Guided update blocked because the old file is missing: {}",
                item.original_path.display()
            )));
        }

        if !incoming_targets.contains(&normalized_path_key(&item.original_path)) {
            return Err(AppError::Message(format!(
                "Guided update blocked because SimSuite cannot safely match the old file {} to an incoming replacement.",
                item.original_path.display()
            )));
        }
    }

    Ok(())
}

fn preflight_special_review_replacements(replacements: &[ReplacementMove]) -> AppResult<()> {
    for item in replacements {
        if !item.original_path.exists() {
            return Err(AppError::Message(format!(
                "Special repair was blocked because the old file is missing: {}",
                item.original_path.display()
            )));
        }
    }

    Ok(())
}

fn update_file_record_on_backup(
    connection: &Connection,
    file_id: Option<i64>,
    backup_path: &Path,
) -> AppResult<()> {
    let Some(file_id) = file_id else {
        return Ok(());
    };
    connection.execute(
        "UPDATE files
         SET path = ?1,
             indexed_at = CURRENT_TIMESTAMP
         WHERE id = ?2",
        params![backup_path.to_string_lossy().to_string(), file_id],
    )?;
    Ok(())
}

fn update_file_record_after_move(
    connection: &Connection,
    settings: &LibrarySettings,
    item: &PlannedMove,
) -> AppResult<()> {
    let from_path = item.current_path.to_string_lossy().to_string();
    let to_path = item.final_path.to_string_lossy().to_string();
    connection.execute(
        "UPDATE files
         SET path = ?1,
             source_location = ?2,
             relative_depth = ?3,
             indexed_at = CURRENT_TIMESTAMP
         WHERE id = ?4",
        params![
            &to_path,
            target_source_location(item),
            relative_depth(settings, &item.final_path, &item.kind),
            item.file_id
        ],
    )?;
    database::sync_category_override_path(connection, &from_path, &to_path)?;

    Ok(())
}

fn update_file_record_on_restore(
    connection: &Connection,
    settings: &LibrarySettings,
    file_id: i64,
    restored_path: &Path,
) -> AppResult<()> {
    let previous_path: String = connection.query_row(
        "SELECT path FROM files WHERE id = ?1",
        params![file_id],
        |row| row.get(0),
    )?;
    let (kind, has_download_item): (String, Option<i64>) = connection.query_row(
        "SELECT kind, download_item_id FROM files WHERE id = ?1",
        params![file_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    let source_location = if has_download_item.is_some() {
        "downloads"
    } else if is_tray_path(restored_path) {
        "tray"
    } else {
        "mods"
    };

    connection.execute(
        "UPDATE files
         SET path = ?1,
             source_location = ?2,
             relative_depth = ?3,
             indexed_at = CURRENT_TIMESTAMP
         WHERE id = ?4",
        params![
            restored_path.to_string_lossy().to_string(),
            source_location,
            relative_depth(&settings, restored_path, &kind),
            file_id
        ],
    )?;
    database::sync_category_override_path(
        connection,
        &previous_path,
        &restored_path.to_string_lossy(),
    )?;

    Ok(())
}

fn delete_file_record(connection: &Connection, file_id: Option<i64>) -> AppResult<()> {
    let Some(file_id) = file_id else {
        return Ok(());
    };
    connection.execute("DELETE FROM files WHERE id = ?1", params![file_id])?;
    Ok(())
}

fn restore_backed_up_file(
    connection: &mut Connection,
    settings: &LibrarySettings,
    seed_pack: &SeedPack,
    backup_path: &Path,
    original_path: &Path,
    original_hash: Option<&str>,
) -> AppResult<bool> {
    if !backup_path.exists() {
        return Ok(false);
    }

    if let Some(expected_hash) = original_hash {
        let current_hash = file_hash(backup_path)?;
        if current_hash != expected_hash {
            return Err(AppError::Message(format!(
                "Rollback blocked because backup contents changed for {}",
                backup_path.display()
            )));
        }
    }

    if original_path.exists() && original_path != backup_path {
        return Err(AppError::Message(format!(
            "Rollback blocked because original destination already exists: {}",
            original_path.display()
        )));
    }

    move_single_file(backup_path, original_path)?;
    restore_deleted_file_record(connection, settings, seed_pack, original_path)?;
    Ok(true)
}

fn restore_deleted_file_record(
    connection: &mut Connection,
    settings: &LibrarySettings,
    seed_pack: &SeedPack,
    restored_path: &Path,
) -> AppResult<()> {
    let extension = restored_path
        .extension()
        .map(|value| format!(".{}", value.to_string_lossy().to_lowercase()))
        .unwrap_or_default();
    if !matches!(extension.as_str(), ".package" | ".ts4script") {
        return Ok(());
    }

    let metadata = fs::metadata(restored_path)?;
    let root_path = if let Some(tray_root) = settings.tray_path.as_deref() {
        if restored_path.starts_with(tray_root) {
            PathBuf::from(tray_root)
        } else {
            PathBuf::from(
                settings
                    .mods_path
                    .as_deref()
                    .ok_or_else(|| AppError::Message("Mods folder is not set.".to_owned()))?,
            )
        }
    } else {
        PathBuf::from(
            settings
                .mods_path
                .as_deref()
                .ok_or_else(|| AppError::Message("Mods folder is not set.".to_owned()))?,
        )
    };
    let source_location = if restored_path.starts_with(settings.tray_path.as_deref().unwrap_or(""))
    {
        "tray".to_owned()
    } else {
        "mods".to_owned()
    };
    let filename = restored_path
        .file_name()
        .map(|value| value.to_string_lossy().to_string())
        .ok_or_else(|| AppError::Message("Restored file name is missing.".to_owned()))?;
    let created_at = metadata.created().ok().map(system_time_to_rfc3339);
    let modified_at = metadata.modified().ok().map(system_time_to_rfc3339);
    let relative_depth_value = restored_path
        .strip_prefix(&root_path)
        .ok()
        .and_then(|relative| {
            relative
                .parent()
                .map(|parent| parent.components().count() as i64)
        })
        .unwrap_or_default();
    let parsed = parse_filename(&filename, seed_pack);
    let inspection = inspect_file(restored_path, &extension, seed_pack)?;
    let creator_name = inspection
        .creator_hint
        .clone()
        .or(parsed.possible_creator.clone());
    let creator_id = creator_name
        .as_deref()
        .map(|name| ensure_creator(connection, name))
        .transpose()?;
    let kind = inspection.kind_hint.clone().unwrap_or(parsed.kind);
    let subtype = inspection.subtype_hint.clone().or(parsed.subtype);
    let confidence = (parsed.confidence + inspection.confidence_boost).min(1.0);
    let safety_notes = serde_json::to_string(&Vec::<String>::new())?;
    let parser_warnings = serde_json::to_string(&parsed.warning_flags)?;
    let insights_json = serde_json::to_string(&inspection.insights)?;

    connection.execute(
        "DELETE FROM files WHERE path = ?1",
        params![restored_path.to_string_lossy().to_string()],
    )?;
    connection.execute(
        "INSERT INTO files (
            path, filename, extension, hash, size, created_at, modified_at, creator_id,
            kind, subtype, confidence, source_location, relative_depth,
            safety_notes, parser_warnings, insights, indexed_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, CURRENT_TIMESTAMP)",
        params![
            restored_path.to_string_lossy().to_string(),
            filename,
            extension,
            file_hash(restored_path)?,
            metadata.len() as i64,
            created_at,
            modified_at,
            creator_id,
            kind,
            subtype,
            confidence,
            source_location,
            relative_depth_value,
            safety_notes,
            parser_warnings,
            insights_json
        ],
    )?;

    Ok(())
}

fn delete_snapshot(connection: &Connection, snapshot_id: i64) -> AppResult<()> {
    connection.execute("DELETE FROM snapshots WHERE id = ?1", params![snapshot_id])?;
    Ok(())
}

fn move_single_file(source: &Path, destination: &Path) -> AppResult<()> {
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)?;
    }

    match fs::rename(source, destination) {
        Ok(()) => Ok(()),
        Err(_) => {
            fs::copy(source, destination)?;
            fs::remove_file(source)?;
            Ok(())
        }
    }
}

fn file_hash(path: &Path) -> AppResult<String> {
    let bytes = fs::read(path)?;
    Ok(hex::encode(sha2::Sha256::digest(bytes)))
}

fn relative_depth(settings: &LibrarySettings, path: &Path, kind: &str) -> i64 {
    let root = if let Some(downloads_root) = settings.downloads_path.as_deref() {
        if path.starts_with(downloads_root) {
            Some(downloads_root)
        } else if kind.starts_with("Tray") || is_tray_path(path) {
            settings.tray_path.as_deref()
        } else {
            settings.mods_path.as_deref()
        }
    } else if kind.starts_with("Tray") || is_tray_path(path) {
        settings.tray_path.as_deref()
    } else {
        settings.mods_path.as_deref()
    };

    root.and_then(|root| {
        path.strip_prefix(root).ok().and_then(|relative| {
            relative
                .parent()
                .map(|parent| parent.components().count() as i64)
        })
    })
    .unwrap_or_default()
}

fn target_source_location(item: &PlannedMove) -> &'static str {
    if item.kind.starts_with("Tray") || is_tray_path(&item.final_path) {
        "tray"
    } else {
        "mods"
    }
}

fn is_tray_path(path: &Path) -> bool {
    path.components().any(|component| {
        component
            .as_os_str()
            .to_string_lossy()
            .eq_ignore_ascii_case("Tray")
    }) || path
        .to_string_lossy()
        .to_ascii_lowercase()
        .contains("\\tray\\")
        || path
            .to_string_lossy()
            .to_ascii_lowercase()
            .contains("/tray/")
}

#[derive(Debug, Clone)]
struct PlannedMove {
    file_id: i64,
    current_path: PathBuf,
    final_path: PathBuf,
    kind: String,
}

#[derive(Debug, Clone)]
struct ReplacementMove {
    file_id: Option<i64>,
    original_path: PathBuf,
    backup_path: PathBuf,
}

fn guided_backup_path(app_data_dir: &Path, item_id: i64, filename: &str) -> PathBuf {
    app_data_dir
        .join("guided_backups")
        .join(item_id.to_string())
        .join(format!(
            "{}_{}",
            Utc::now().format("%Y%m%d%H%M%S"),
            filename.replace(['<', '>', ':', '"', '/', '\\', '|', '?', '*'], "_")
        ))
}

fn ensure_creator(connection: &Connection, creator_name: &str) -> AppResult<i64> {
    connection.execute(
        "INSERT INTO creators (canonical_name, notes, created_by_user)
         VALUES (?1, '', 0)
         ON CONFLICT(canonical_name) DO NOTHING",
        params![creator_name],
    )?;

    connection
        .query_row(
            "SELECT id FROM creators WHERE canonical_name = ?1",
            params![creator_name],
            |row| row.get(0),
        )
        .map_err(Into::into)
}

fn normalized_path_key(path: &Path) -> String {
    path.to_string_lossy()
        .replace('\\', "/")
        .to_ascii_lowercase()
}

fn system_time_to_rfc3339(time: std::time::SystemTime) -> String {
    let date_time: chrono::DateTime<Utc> = time.into();
    date_time.to_rfc3339()
}

#[cfg(test)]
mod tests {
    use std::fs;

    use rusqlite::params;
    use tempfile::tempdir;
    use zip::write::SimpleFileOptions;

    use crate::{
        core::install_profile_engine::{build_guided_plan, store_download_item_assessment},
        database,
        models::LibrarySettings,
        seed::load_seed_pack,
    };

    use super::{apply_guided_download_plan, apply_preview_moves, restore_snapshot};

    fn setup_guided_env() -> (
        tempfile::TempDir,
        std::path::PathBuf,
        std::path::PathBuf,
        std::path::PathBuf,
        rusqlite::Connection,
        crate::seed::SeedPack,
        LibrarySettings,
    ) {
        let temp = tempdir().expect("tempdir");
        let mods = temp.path().join("Mods");
        let tray = temp.path().join("Tray");
        let downloads = temp.path().join("Downloads");
        fs::create_dir_all(&mods).expect("mods");
        fs::create_dir_all(&tray).expect("tray");
        fs::create_dir_all(&downloads).expect("downloads");

        let mut connection = rusqlite::Connection::open_in_memory().expect("in-memory db");
        database::initialize(&mut connection).expect("schema");
        let seed_pack = load_seed_pack().expect("seed");
        database::seed_database(&mut connection, &seed_pack).expect("seed db");
        let settings = LibrarySettings {
            mods_path: Some(mods.to_string_lossy().to_string()),
            tray_path: Some(tray.to_string_lossy().to_string()),
            downloads_path: Some(downloads.to_string_lossy().to_string()),
        };
        database::save_library_paths(&mut connection, &settings).expect("settings");

        (temp, mods, tray, downloads, connection, seed_pack, settings)
    }

    fn insert_guided_download_item(
        connection: &rusqlite::Connection,
        item_id: i64,
        display_name: &str,
        staging_path: &std::path::Path,
    ) {
        connection
            .execute(
                "INSERT INTO download_items (
                    id, source_path, display_name, source_kind, archive_format, staging_path,
                    source_size, detected_file_count, status, notes
                 ) VALUES (?1, ?2, ?3, 'archive', 'zip', ?4, 100, 0, 'pending', '[]')",
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
        connection: &rusqlite::Connection,
        item_id: i64,
        file_id: i64,
        path: &std::path::Path,
        filename: &str,
        extension: &str,
    ) {
        connection
            .execute(
                "INSERT INTO files (
                    id, path, filename, extension, hash, size, kind, subtype, confidence,
                    source_location, download_item_id, source_origin_path, archive_member_path,
                    safety_notes, parser_warnings, insights, indexed_at
                 ) VALUES (
                    ?1, ?2, ?3, ?4, 'hash', 100, 'ScriptMods', 'Utility', 0.98,
                    'downloads', ?5, ?2, ?3, '[]', '[]', '{}', CURRENT_TIMESTAMP
                 )",
                params![
                    file_id,
                    path.to_string_lossy().to_string(),
                    filename,
                    extension,
                    item_id
                ],
            )
            .expect("download file");
    }

    fn insert_installed_file(
        connection: &rusqlite::Connection,
        path: &std::path::Path,
        filename: &str,
        extension: &str,
    ) {
        connection
            .execute(
                "INSERT INTO files (
                    path, filename, extension, hash, size, kind, subtype, confidence,
                    source_location, safety_notes, parser_warnings, insights, indexed_at
                 ) VALUES (
                    ?1, ?2, ?3, 'hash', 100, 'ScriptMods', 'Utility', 0.99,
                    'mods', '[]', '[]', '{}', CURRENT_TIMESTAMP
                 )",
                params![path.to_string_lossy().to_string(), filename, extension],
            )
            .expect("installed file");
    }

    fn write_ts4script_archive(path: &std::path::Path, entry_name: &str, bytes: &[u8]) {
        let file = fs::File::create(path).expect("script archive");
        let mut zip = zip::ZipWriter::new(file);
        zip.start_file(entry_name, SimpleFileOptions::default())
            .expect("start script entry");
        use std::io::Write as _;
        zip.write_all(bytes).expect("write script entry");
        zip.finish().expect("finish script archive");
    }

    #[test]
    fn apply_and_rollback_preview_moves_are_reversible() {
        let temp = tempdir().expect("tempdir");
        let mods = temp.path().join("Mods");
        let tray = temp.path().join("Tray");
        fs::create_dir_all(&mods).expect("mods");
        fs::create_dir_all(&tray).expect("tray");

        let source_file = mods.join("messy_hair.package");
        fs::write(&source_file, b"hair-data").expect("source file");

        let mut connection = rusqlite::Connection::open_in_memory().expect("in-memory db");
        database::initialize(&mut connection).expect("schema");
        let seed_pack = load_seed_pack().expect("seed");
        database::seed_database(&mut connection, &seed_pack).expect("seed db");
        database::save_library_paths(
            &mut connection,
            &LibrarySettings {
                mods_path: Some(mods.to_string_lossy().to_string()),
                tray_path: Some(tray.to_string_lossy().to_string()),
                downloads_path: None,
            },
        )
        .expect("settings");

        let creator_id: i64 = connection
            .query_row(
                "SELECT id FROM creators WHERE canonical_name = ?1",
                params!["Simstrouble"],
                |row| row.get(0),
            )
            .expect("creator");
        connection
            .execute(
                "INSERT INTO files (
                    path, filename, extension, kind, subtype, confidence, source_location, creator_id
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    source_file.to_string_lossy().to_string(),
                    "messy_hair.package",
                    ".package",
                    "CAS",
                    "Hair",
                    0.92_f64,
                    "mods",
                    creator_id
                ],
            )
            .expect("file");

        let apply_result = apply_preview_moves(
            &mut connection,
            &LibrarySettings {
                mods_path: Some(mods.to_string_lossy().to_string()),
                tray_path: Some(tray.to_string_lossy().to_string()),
                downloads_path: None,
            },
            Some("Category First".to_owned()),
            20,
            true,
        )
        .expect("apply");

        assert_eq!(apply_result.moved_count, 1);
        let moved_path = mods
            .join("CAS")
            .join("Hair")
            .join("Simstrouble")
            .join("messy_hair.package");
        assert!(moved_path.exists());
        assert!(!source_file.exists());

        let rollback_result =
            restore_snapshot(&mut connection, &seed_pack, apply_result.snapshot_id, true)
                .expect("rollback");
        assert_eq!(rollback_result.restored_count, 1);
        assert!(source_file.exists());
        assert!(!moved_path.exists());
    }

    #[test]
    fn apply_preview_moves_uses_full_batch_even_when_preview_limit_is_small() {
        let temp = tempdir().expect("tempdir");
        let mods = temp.path().join("Mods");
        let tray = temp.path().join("Tray");
        fs::create_dir_all(&mods).expect("mods");
        fs::create_dir_all(&tray).expect("tray");

        let first_source = mods.join("messy_hair.package");
        let second_source = mods.join("messy_top.package");
        fs::write(&first_source, b"hair-data").expect("first file");
        fs::write(&second_source, b"top-data").expect("second file");

        let mut connection = rusqlite::Connection::open_in_memory().expect("in-memory db");
        database::initialize(&mut connection).expect("schema");
        let seed_pack = load_seed_pack().expect("seed");
        database::seed_database(&mut connection, &seed_pack).expect("seed db");
        database::save_library_paths(
            &mut connection,
            &LibrarySettings {
                mods_path: Some(mods.to_string_lossy().to_string()),
                tray_path: Some(tray.to_string_lossy().to_string()),
                downloads_path: None,
            },
        )
        .expect("settings");

        let creator_id: i64 = connection
            .query_row(
                "SELECT id FROM creators WHERE canonical_name = ?1",
                params!["Simstrouble"],
                |row| row.get(0),
            )
            .expect("creator");

        for path in [&first_source, &second_source] {
            connection
                .execute(
                    "INSERT INTO files (
                        path, filename, extension, kind, subtype, confidence, source_location, creator_id
                     ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                    params![
                        path.to_string_lossy().to_string(),
                        path.file_name()
                            .expect("name")
                            .to_string_lossy()
                            .to_string(),
                        ".package",
                        "CAS",
                        "Hair",
                        0.98_f64,
                        "mods",
                        creator_id
                    ],
                )
                .expect("file row");
        }

        let apply_result = apply_preview_moves(
            &mut connection,
            &LibrarySettings {
                mods_path: Some(mods.to_string_lossy().to_string()),
                tray_path: Some(tray.to_string_lossy().to_string()),
                downloads_path: None,
            },
            Some("Category First".to_owned()),
            1,
            true,
        )
        .expect("apply");

        assert_eq!(apply_result.moved_count, 2);
        assert!(mods
            .join("CAS")
            .join("Hair")
            .join("Simstrouble")
            .join("messy_hair.package")
            .exists());
        assert!(mods
            .join("CAS")
            .join("Hair")
            .join("Simstrouble")
            .join("messy_top.package")
            .exists());
    }

    #[test]
    fn guided_mccc_first_install_creates_snapshot_and_moves_files() {
        let (temp, mods, _tray, downloads, mut connection, seed_pack, settings) =
            setup_guided_env();
        let app_data_dir = temp.path().join("AppData");
        fs::create_dir_all(&app_data_dir).expect("app data");

        let staging = downloads.join("Inbox").join("MCCC");
        fs::create_dir_all(&staging).expect("staging");
        insert_guided_download_item(&connection, 201, "MC_Command_Center_2026.3.0.zip", &staging);

        let core_script = staging.join("mc_cmd_center.ts4script");
        let core_package = staging.join("mc_cmd_center.package");
        write_ts4script_archive(
            &core_script,
            "mc_cmd_center/__init__.py",
            b"new-core-script",
        );
        fs::write(&core_package, b"new-core-package").expect("core package");
        insert_download_file(
            &connection,
            201,
            20101,
            &core_script,
            "mc_cmd_center.ts4script",
            ".ts4script",
        );
        insert_download_file(
            &connection,
            201,
            20102,
            &core_package,
            "mc_cmd_center.package",
            ".package",
        );

        let assessment = crate::core::install_profile_engine::assess_download_item(
            &connection,
            &settings,
            &seed_pack,
            201,
        )
        .expect("assessment");
        store_download_item_assessment(&connection, 201, &assessment).expect("stored");
        let plan = build_guided_plan(&connection, &settings, &seed_pack, 201)
            .expect("plan")
            .expect("guided plan");

        let result = apply_guided_download_plan(
            &mut connection,
            &settings,
            &seed_pack,
            &app_data_dir,
            &plan,
            true,
        )
        .expect("apply guided");

        assert_eq!(result.installed_count, 2);
        assert_eq!(result.replaced_count, 0);
        assert!(result.snapshot_id > 0);
        assert!(mods.join("MCCC").join("mc_cmd_center.ts4script").exists());
        assert!(mods.join("MCCC").join("mc_cmd_center.package").exists());
        assert!(!core_script.exists());
        assert!(!core_package.exists());

        let snapshot_items: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM snapshot_items WHERE snapshot_id = ?1",
                params![result.snapshot_id],
                |row| row.get(0),
            )
            .expect("snapshot item count");
        assert_eq!(snapshot_items, 2);
    }

    #[test]
    fn guided_mccc_update_keeps_cfg_and_rollback_restores_old_files() {
        let (temp, mods, _tray, downloads, mut connection, seed_pack, settings) =
            setup_guided_env();
        let app_data_dir = temp.path().join("AppData");
        fs::create_dir_all(&app_data_dir).expect("app data");

        let target = mods.join("MCCC");
        fs::create_dir_all(&target).expect("target");
        let old_script = target.join("mc_cmd_center.ts4script");
        let old_package = target.join("mc_cmd_center.package");
        let cfg = target.join("mc_settings.cfg");
        write_ts4script_archive(&old_script, "mc_cmd_center/__init__.py", b"old-core-script");
        fs::write(&old_package, b"old-core-package").expect("old package");
        fs::write(&cfg, b"user-settings").expect("cfg");
        insert_installed_file(
            &connection,
            &old_script,
            "mc_cmd_center.ts4script",
            ".ts4script",
        );
        insert_installed_file(
            &connection,
            &old_package,
            "mc_cmd_center.package",
            ".package",
        );

        let staging = downloads.join("Inbox").join("MCCC_Update");
        fs::create_dir_all(&staging).expect("staging");
        insert_guided_download_item(&connection, 202, "MC_Command_Center_2026.4.0.zip", &staging);

        let new_script = staging.join("mc_cmd_center.ts4script");
        let new_package = staging.join("mc_cmd_center.package");
        let new_module = staging.join("mc_woohoo.package");
        write_ts4script_archive(&new_script, "mc_cmd_center/__init__.py", b"new-core-script");
        fs::write(&new_package, b"new-core-package").expect("new package");
        fs::write(&new_module, b"new-module").expect("new module");
        insert_download_file(
            &connection,
            202,
            20201,
            &new_script,
            "mc_cmd_center.ts4script",
            ".ts4script",
        );
        insert_download_file(
            &connection,
            202,
            20202,
            &new_package,
            "mc_cmd_center.package",
            ".package",
        );
        insert_download_file(
            &connection,
            202,
            20203,
            &new_module,
            "mc_woohoo.package",
            ".package",
        );

        let assessment = crate::core::install_profile_engine::assess_download_item(
            &connection,
            &settings,
            &seed_pack,
            202,
        )
        .expect("assessment");
        store_download_item_assessment(&connection, 202, &assessment).expect("stored");
        let plan = build_guided_plan(&connection, &settings, &seed_pack, 202)
            .expect("plan")
            .expect("guided plan");

        let apply_result = apply_guided_download_plan(
            &mut connection,
            &settings,
            &seed_pack,
            &app_data_dir,
            &plan,
            true,
        )
        .expect("guided apply");

        assert_eq!(apply_result.installed_count, 3);
        assert_eq!(apply_result.replaced_count, 2);
        assert_eq!(apply_result.preserved_count, 1);
        assert!(!fs::read(&old_script).expect("installed script").is_empty());
        assert_eq!(
            fs::read(&old_package).expect("installed package"),
            b"new-core-package"
        );
        assert_eq!(
            fs::read(target.join("mc_woohoo.package")).expect("installed module"),
            b"new-module"
        );
        assert_eq!(fs::read(&cfg).expect("cfg after apply"), b"user-settings");
        assert!(!new_script.exists());
        assert!(!new_package.exists());
        assert!(!new_module.exists());

        let rollback =
            restore_snapshot(&mut connection, &seed_pack, apply_result.snapshot_id, true)
                .expect("rollback");
        assert_eq!(rollback.restored_count, 5);
        assert!(!fs::read(&old_script).expect("restored script").is_empty());
        assert_eq!(
            fs::read(&old_package).expect("restored package"),
            b"old-core-package"
        );
        assert_eq!(
            fs::read(&cfg).expect("cfg after rollback"),
            b"user-settings"
        );
        assert!(new_script.exists());
        assert!(new_package.exists());
        assert!(new_module.exists());
        assert!(!target.join("mc_woohoo.package").exists());
    }

    #[test]
    fn guided_mccc_update_allows_disk_only_existing_replacements() {
        let (temp, mods, _tray, downloads, mut connection, seed_pack, settings) =
            setup_guided_env();
        let app_data_dir = temp.path().join("AppData");
        fs::create_dir_all(&app_data_dir).expect("app data");

        let target = mods.join("MCCC");
        fs::create_dir_all(&target).expect("target");
        let old_script = target.join("mc_cmd_center.ts4script");
        let old_package = target.join("mc_cmd_center.package");
        write_ts4script_archive(&old_script, "mc_cmd_center/__init__.py", b"old-core-script");
        fs::write(&old_package, b"old-core-package").expect("old package");

        let staging = downloads.join("Inbox").join("MCCC_DiskOnly_Update");
        fs::create_dir_all(&staging).expect("staging");
        insert_guided_download_item(&connection, 203, "MC_Command_Center_2026.4.0.zip", &staging);

        let new_script = staging.join("mc_cmd_center.ts4script");
        let new_package = staging.join("mc_cmd_center.package");
        write_ts4script_archive(&new_script, "mc_cmd_center/__init__.py", b"new-core-script");
        fs::write(&new_package, b"new-core-package").expect("new package");
        insert_download_file(
            &connection,
            203,
            20301,
            &new_script,
            "mc_cmd_center.ts4script",
            ".ts4script",
        );
        insert_download_file(
            &connection,
            203,
            20302,
            &new_package,
            "mc_cmd_center.package",
            ".package",
        );

        let assessment = crate::core::install_profile_engine::assess_download_item(
            &connection,
            &settings,
            &seed_pack,
            203,
        )
        .expect("assessment");
        store_download_item_assessment(&connection, 203, &assessment).expect("stored");
        let plan = build_guided_plan(&connection, &settings, &seed_pack, 203)
            .expect("plan")
            .expect("guided plan");

        assert!(plan.replace_files.iter().all(|file| file.file_id.is_none()));

        let apply_result = apply_guided_download_plan(
            &mut connection,
            &settings,
            &seed_pack,
            &app_data_dir,
            &plan,
            true,
        )
        .expect("guided apply");

        assert_eq!(apply_result.installed_count, 2);
        assert_eq!(apply_result.replaced_count, 2);
        assert_eq!(
            fs::read(&old_package).expect("installed package"),
            b"new-core-package"
        );
        assert!(!new_script.exists());
        assert!(!new_package.exists());
    }
}
