use rusqlite::Connection;

use crate::{
    core::{
        apply_plan_persistence_json, apply_plan_preview_snapshots, apply_plan_provenance,
        apply_plan_saves,
    },
    error::{AppError, AppResult},
    models::{
        ApplyPlanContextSignal, ApplyPlanFolderConfig, SaveApplyPlanFromPreviewSnapshotRequest,
        SaveApplyPlanPreviewRequest, SaveApplyPlanPreviewResult, StagingPlan,
    },
};

pub(crate) fn save_apply_plan_from_preview_snapshot(
    connection: &mut Connection,
    request: SaveApplyPlanFromPreviewSnapshotRequest,
) -> AppResult<SaveApplyPlanPreviewResult> {
    let snapshot = apply_plan_preview_snapshots::load_preview_snapshot(
        connection,
        request.preview_snapshot_id,
    )?
    .ok_or_else(|| AppError::Message("Preview snapshot was not found.".to_owned()))?;

    if snapshot.consumed_apply_plan_id.is_some() {
        return Err(AppError::Message(
            "Preview snapshot has already been saved as an ApplyPlan draft.".to_owned(),
        ));
    }

    if snapshot.preview_snapshot_hash != request.preview_snapshot_hash {
        return Err(AppError::Message(
            "Preview snapshot hash mismatch; regenerate the preview before saving.".to_owned(),
        ));
    }

    if snapshot.preview_snapshot_hash_version
        != apply_plan_provenance::PREVIEW_SNAPSHOT_HASH_VERSION
        || snapshot.preview_snapshot_hash_algorithm != apply_plan_provenance::PLAN_HASH_ALGORITHM
    {
        return Err(AppError::Message(
            "Preview snapshot uses an unsupported hash version or algorithm.".to_owned(),
        ));
    }

    let source_plan: StagingPlan = apply_plan_persistence_json::parse_required_json_field(
        &snapshot.source_plan_json,
        "source_plan_json",
        "preview snapshot",
        snapshot.id,
    )?;
    let source_scope: Option<serde_json::Value> =
        apply_plan_persistence_json::parse_optional_json_field(
            snapshot.source_scope_json.as_deref(),
            "source_scope_json",
            "preview snapshot",
            snapshot.id,
        )?;
    let folder_config: Option<ApplyPlanFolderConfig> =
        apply_plan_persistence_json::parse_optional_json_field(
            snapshot.folder_config_json.as_deref(),
            "folder_config_json",
            "preview snapshot",
            snapshot.id,
        )?;
    let context_trail: Vec<ApplyPlanContextSignal> =
        apply_plan_persistence_json::parse_required_json_field(
            &snapshot.context_trail_json,
            "context_trail_json",
            "preview snapshot",
            snapshot.id,
        )?;
    let stored_provenance: serde_json::Value =
        apply_plan_persistence_json::parse_required_json_field(
            &snapshot.preview_snapshot_provenance_json,
            "preview_snapshot_provenance_json",
            "preview snapshot",
            snapshot.id,
        )?;
    let recomputed = apply_plan_provenance::build_preview_snapshot_hash_stamp(
        connection,
        &source_plan,
        &snapshot.source_plan_kind,
        &source_scope,
        &folder_config,
        &context_trail,
        snapshot.scan_session_id,
    )?;
    if recomputed.hash != snapshot.preview_snapshot_hash
        || recomputed.provenance != stored_provenance
    {
        return Err(AppError::Message(
            "Preview snapshot provenance no longer matches stored preview content.".to_owned(),
        ));
    }

    let result = apply_plan_saves::save_apply_plan_preview_draft(
        connection,
        SaveApplyPlanPreviewRequest {
            source_plan,
            source_plan_kind: None,
            source_scope,
            folder_config,
            context_trail,
            scan_session_id: snapshot.scan_session_id,
        },
        &snapshot.source_plan_kind,
        Some(snapshot.id),
        Some(snapshot.preview_snapshot_hash.clone()),
    )?;
    let consumed = apply_plan_preview_snapshots::mark_preview_snapshot_consumed(
        connection,
        snapshot.id,
        result.plan_id,
    )?;
    if !consumed {
        return Err(AppError::Message(
            "Preview snapshot has already been saved as an ApplyPlan draft.".to_owned(),
        ));
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use rusqlite::Connection;

    use crate::{
        core::{apply_plan_loading, apply_plan_preview_snapshots, apply_plan_provenance},
        database,
        models::{
            GenerateSortingPreviewPlanRequest, GenerateSortingPreviewPlanScope, LibrarySettings,
            SaveApplyPlanFromPreviewSnapshotRequest,
        },
    };

    use super::save_apply_plan_from_preview_snapshot;

    fn memory_connection() -> Connection {
        let mut connection = Connection::open_in_memory().expect("memory db");
        database::initialize(&mut connection).expect("schema");
        connection
    }

    fn insert_snapshot_preview_files(connection: &Connection) {
        connection
            .execute(
                "INSERT INTO files (
                    id, path, filename, extension, size, modified_at, hash, kind, subtype,
                    confidence, safety_notes, parser_warnings, source_location, relative_depth
                ) VALUES
                (1, 'C:/Sims/Mods/a.package', 'a.package', 'package', 1, '2026-01-01', 'h1', 'CAS', 'Hair', 0.95, '[]', '[]', 'mods', 1),
                (2, 'C:/Sims/Mods/b.package', 'b.package', 'package', 1, '2026-01-01', 'h2', 'Unknown', NULL, 0.10, '[]', '[\"parser_warning\"]', 'mods', 1)",
                [],
            )
            .expect("files");
    }

    fn snapshot_settings() -> LibrarySettings {
        LibrarySettings {
            mods_path: Some("C:/Sims/Mods".to_owned()),
            tray_path: Some("C:/Sims/Tray".to_owned()),
            ..Default::default()
        }
    }

    fn snapshot_preview_request() -> GenerateSortingPreviewPlanRequest {
        GenerateSortingPreviewPlanRequest {
            scope: GenerateSortingPreviewPlanScope::SelectedFiles {
                file_ids: vec![1, 2],
            },
            folder_config: None,
            context_trail: Vec::new(),
        }
    }

    #[test]
    fn consumer_saves_backend_snapshot_and_marks_consumed() {
        let mut connection = memory_connection();
        insert_snapshot_preview_files(&connection);
        let preview = apply_plan_preview_snapshots::generate_sorting_preview_snapshot(
            &mut connection,
            &snapshot_settings(),
            snapshot_preview_request(),
        )
        .expect("preview snapshot");

        let saved = save_apply_plan_from_preview_snapshot(
            &mut connection,
            SaveApplyPlanFromPreviewSnapshotRequest {
                preview_snapshot_id: preview.preview_snapshot_id,
                preview_snapshot_hash: preview.preview_snapshot_hash.clone(),
            },
        )
        .expect("save through snapshot consumption boundary");
        let persisted = apply_plan_loading::load_saved_apply_plan(&connection, saved.plan_id)
            .expect("get")
            .expect("persisted plan");

        assert_eq!(
            persisted.source_plan_kind,
            apply_plan_provenance::BACKEND_GENERATED_SORTING_PREVIEW_SOURCE_KIND
        );
        assert_eq!(
            persisted.preview_snapshot_id,
            Some(preview.preview_snapshot_id)
        );
        assert_eq!(
            persisted.preview_snapshot_hash.as_deref(),
            Some(preview.preview_snapshot_hash.as_str())
        );

        let consumed_apply_plan_id: Option<i64> = connection
            .query_row(
                "SELECT consumed_apply_plan_id FROM apply_plan_preview_snapshots WHERE id = ?1",
                [preview.preview_snapshot_id],
                |row| row.get(0),
            )
            .expect("consumed snapshot id");
        assert_eq!(consumed_apply_plan_id, Some(saved.plan_id));
    }
}
