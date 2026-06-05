use rusqlite::Connection;

use crate::{
    error::AppResult,
    models::{
        BuildApplyPlanFromStagingPlanRequest, LibrarySettings,
        SaveApplyPlanFromPreviewSnapshotRequest, SaveApplyPlanPreviewResult,
    },
};

use super::{apply_plan_preview_snapshot_consumption, apply_plan_preview_snapshots};

pub(crate) fn build_apply_plan_from_staging_plan(
    connection: &mut Connection,
    settings: &LibrarySettings,
    request: BuildApplyPlanFromStagingPlanRequest,
) -> AppResult<SaveApplyPlanPreviewResult> {
    let mut preview_request = request.preview_request;
    preview_request.folder_config = request.folder_config.or(preview_request.folder_config);
    if !request.context_trail.is_empty() {
        preview_request.context_trail = request.context_trail;
    }

    let preview = apply_plan_preview_snapshots::generate_sorting_preview_snapshot(
        connection,
        settings,
        preview_request,
    )?;
    apply_plan_preview_snapshot_consumption::save_apply_plan_from_preview_snapshot(
        connection,
        SaveApplyPlanFromPreviewSnapshotRequest {
            preview_snapshot_id: preview.preview_snapshot_id,
            preview_snapshot_hash: preview.preview_snapshot_hash,
        },
    )
}

#[cfg(test)]
mod tests {
    use rusqlite::Connection;

    use crate::{
        core::{apply_plan_loading, apply_plan_provenance, apply_plan_records},
        database,
        models::{
            BuildApplyPlanFromStagingPlanRequest, GenerateSortingPreviewPlanRequest,
            GenerateSortingPreviewPlanScope, LibrarySettings, ListSavedApplyPlansRequest,
            PersistedApplyPlanItemStatus,
        },
    };

    use super::build_apply_plan_from_staging_plan;

    fn memory_connection() -> Connection {
        let mut connection = Connection::open_in_memory().expect("memory db");
        database::initialize(&mut connection).expect("schema");
        connection
    }

    fn sample_builder_request() -> BuildApplyPlanFromStagingPlanRequest {
        BuildApplyPlanFromStagingPlanRequest {
            preview_request: GenerateSortingPreviewPlanRequest {
                scope: GenerateSortingPreviewPlanScope::SelectedFiles {
                    file_ids: vec![1, 2],
                },
                folder_config: None,
                context_trail: Vec::new(),
            },
            source_plan_kind: None,
            folder_config: None,
            context_trail: Vec::new(),
        }
    }

    fn insert_builder_preview_files(connection: &Connection) {
        connection
            .execute(
                "INSERT INTO files (
                    path, filename, extension, size, modified_at, hash, kind, subtype,
                    confidence, safety_notes, parser_warnings, source_location, relative_depth
                ) VALUES
                ('C:/Sims/Mods/hair.package', 'hair.package', 'package', 1, '2026-01-01', 'h1', 'CAS', 'Hair', 0.95, '[]', '[]', 'mods', 1),
                ('C:/Sims/Mods/unknown.package', 'unknown.package', 'package', 1, '2026-01-01', 'h2', 'Unknown', NULL, 0.10, '[]', '[\"parser_warning\"]', 'mods', 1)",
                [],
            )
            .expect("files");
    }

    #[test]
    fn builder_module_generates_snapshot_bound_draft() {
        let mut connection = memory_connection();
        let settings = LibrarySettings {
            mods_path: Some("C:/Sims/Mods".to_owned()),
            tray_path: Some("C:/Sims/Tray".to_owned()),
            ..Default::default()
        };
        insert_builder_preview_files(&connection);

        let result = build_apply_plan_from_staging_plan(
            &mut connection,
            &settings,
            sample_builder_request(),
        )
        .expect("build saved draft through builder boundary");

        assert!(result.plan_id > 0);
        assert_eq!(
            result.plan.source_plan_kind,
            apply_plan_provenance::BACKEND_GENERATED_SORTING_PREVIEW_SOURCE_KIND
        );
        assert!(!result.plan.would_touch_files);
        assert_eq!(result.plan.applyable_items, 0);
        assert_eq!(result.plan.total_items, 2);

        let summaries = apply_plan_records::list_saved_apply_plans(
            &connection,
            ListSavedApplyPlansRequest::default(),
        )
        .expect("list");
        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].id, result.plan_id);

        let saved = apply_plan_loading::load_saved_apply_plan(&connection, result.plan_id)
            .expect("get")
            .expect("saved");
        assert_eq!(
            saved
                .source_scope
                .as_ref()
                .and_then(|scope| scope.get("kind"))
                .and_then(|value| value.as_str()),
            Some("selected_files")
        );
        assert!(saved
            .caveats
            .iter()
            .any(|caveat| caveat.contains("No files changed")));
        assert!(saved.items.iter().all(|item| !item.signals.is_empty()));
        assert!(saved
            .items
            .iter()
            .all(|item| !item.blocked || !item.blockers.is_empty()));
        assert!(saved
            .items
            .iter()
            .any(|item| item.item_status == PersistedApplyPlanItemStatus::Blocked));
        assert!(
            saved.preview_snapshot_id.is_some(),
            "backend-generated builder saves must be bound to a preview snapshot"
        );
        assert!(saved.preview_snapshot_hash.is_some());
    }
}
