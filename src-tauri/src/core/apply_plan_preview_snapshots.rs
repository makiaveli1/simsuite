use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension, Row};
use sha2::Digest;

use crate::{
    core::{apply_plan_provenance, apply_plan_saves},
    error::{AppError, AppResult},
    models::{
        GenerateSortingPreviewPlanRequest, GenerateSortingPreviewPlanResult, LibrarySettings,
    },
};

pub(crate) fn generate_sorting_preview_snapshot(
    connection: &mut Connection,
    settings: &LibrarySettings,
    request: GenerateSortingPreviewPlanRequest,
) -> AppResult<GenerateSortingPreviewPlanResult> {
    let source_scope = serde_json::to_value(&request.scope)?;
    let folder_config = request.folder_config.clone();
    let context_trail = request.context_trail.clone();
    let source_plan = crate::core::rule_engine::sorting_plan::generate_sorting_preview_plan(
        connection, settings, request,
    )?;
    apply_plan_saves::reject_file_touching_source_plan(&source_plan)?;

    let seed = serde_json::to_vec(&source_plan)?;
    let seed_hash = hex::encode(sha2::Sha256::digest(seed));
    let snapshot_id = format!(
        "apply-preview-{}-{}",
        Utc::now().timestamp_millis(),
        &seed_hash[..12]
    );

    let source_plan_kind = apply_plan_provenance::BACKEND_GENERATED_SORTING_PREVIEW_SOURCE_KIND;
    let hash_stamp = apply_plan_provenance::build_preview_snapshot_hash_stamp(
        connection,
        &source_plan,
        source_plan_kind,
        &Some(source_scope.clone()),
        &folder_config,
        &context_trail,
        None,
    )?;
    let source_plan_json = serde_json::to_string(&source_plan)?;
    let source_scope_json = serde_json::to_string(&source_scope)?;
    let folder_config_json = folder_config
        .as_ref()
        .map(serde_json::to_string)
        .transpose()?;
    let context_trail_json = serde_json::to_string(&context_trail)?;
    let provenance_json = serde_json::to_string(&hash_stamp.provenance)?;
    let created_at = Utc::now().to_rfc3339();

    connection.execute(
        "INSERT INTO apply_plan_preview_snapshots (
            snapshot_id,
            source_plan_kind,
            source_plan_json,
            source_scope_json,
            folder_config_json,
            context_trail_json,
            preview_snapshot_hash,
            preview_snapshot_hash_version,
            preview_snapshot_hash_algorithm,
            preview_snapshot_provenance_json,
            created_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        params![
            snapshot_id,
            source_plan_kind,
            source_plan_json,
            source_scope_json,
            folder_config_json,
            context_trail_json,
            hash_stamp.hash,
            hash_stamp.version,
            hash_stamp.algorithm,
            provenance_json,
            created_at,
        ],
    )?;
    let preview_snapshot_id = connection.last_insert_rowid();

    Ok(GenerateSortingPreviewPlanResult {
        plan: source_plan,
        preview_snapshot_id,
        preview_snapshot_hash: hash_stamp.hash,
        preview_snapshot_hash_version: hash_stamp.version,
        preview_snapshot_hash_algorithm: hash_stamp.algorithm,
        preview_snapshot_created_at: created_at,
    })
}

pub(crate) struct PreviewSnapshotRow {
    pub(crate) id: i64,
    /// Stable human-facing snapshot identity. The current save path keys by DB id,
    /// but this remains part of the immutable preview audit contract.
    #[allow(dead_code)]
    pub(crate) snapshot_id: String,
    pub(crate) source_plan_kind: String,
    pub(crate) source_plan_json: String,
    pub(crate) source_scope_json: Option<String>,
    pub(crate) folder_config_json: Option<String>,
    pub(crate) context_trail_json: String,
    pub(crate) preview_snapshot_hash: String,
    pub(crate) preview_snapshot_hash_version: String,
    pub(crate) preview_snapshot_hash_algorithm: String,
    pub(crate) preview_snapshot_provenance_json: String,
    pub(crate) scan_session_id: Option<i64>,
    pub(crate) consumed_apply_plan_id: Option<i64>,
    /// Audit timestamp retained with the row for future snapshot review/reporting.
    #[allow(dead_code)]
    pub(crate) created_at: String,
    /// Reserved for future preview expiry enforcement; not enforced in Phase C.
    #[allow(dead_code)]
    pub(crate) expires_at: Option<String>,
}

pub(crate) fn load_preview_snapshot(
    connection: &Connection,
    preview_snapshot_id: i64,
) -> AppResult<Option<PreviewSnapshotRow>> {
    connection
        .query_row(
            "SELECT id, snapshot_id, source_plan_kind, source_plan_json, source_scope_json,
                folder_config_json, context_trail_json, preview_snapshot_hash,
                preview_snapshot_hash_version, preview_snapshot_hash_algorithm,
                preview_snapshot_provenance_json, scan_session_id, consumed_apply_plan_id,
                created_at, expires_at
             FROM apply_plan_preview_snapshots
             WHERE id = ?1",
            params![preview_snapshot_id],
            preview_snapshot_row_from_row,
        )
        .optional()
        .map_err(AppError::from)
}

pub(crate) fn mark_preview_snapshot_consumed(
    connection: &Connection,
    preview_snapshot_id: i64,
    apply_plan_id: i64,
) -> AppResult<bool> {
    let consumed_rows = connection.execute(
        "UPDATE apply_plan_preview_snapshots
         SET consumed_apply_plan_id = ?1
         WHERE id = ?2 AND consumed_apply_plan_id IS NULL",
        params![apply_plan_id, preview_snapshot_id],
    )?;

    Ok(consumed_rows == 1)
}

pub(crate) fn preview_snapshot_row_from_row(row: &Row<'_>) -> rusqlite::Result<PreviewSnapshotRow> {
    Ok(PreviewSnapshotRow {
        id: row.get(0)?,
        snapshot_id: row.get(1)?,
        source_plan_kind: row.get(2)?,
        source_plan_json: row.get(3)?,
        source_scope_json: row.get(4)?,
        folder_config_json: row.get(5)?,
        context_trail_json: row.get(6)?,
        preview_snapshot_hash: row.get(7)?,
        preview_snapshot_hash_version: row.get(8)?,
        preview_snapshot_hash_algorithm: row.get(9)?,
        preview_snapshot_provenance_json: row.get(10)?,
        scan_session_id: row.get(11)?,
        consumed_apply_plan_id: row.get(12)?,
        created_at: row.get(13)?,
        expires_at: row.get(14)?,
    })
}

#[cfg(test)]
mod tests {
    use rusqlite::Connection;

    use crate::{
        core::apply_plan_provenance,
        database,
        models::{
            GenerateSortingPreviewPlanRequest, GenerateSortingPreviewPlanScope, LibrarySettings,
        },
    };

    use super::{
        generate_sorting_preview_snapshot, load_preview_snapshot, preview_snapshot_row_from_row,
    };

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
    fn row_mapper_preserves_hash_and_audit_columns() {
        let connection = Connection::open_in_memory().expect("memory db");

        let snapshot_row = connection
            .query_row(
                "SELECT 45, 'snapshot-stable-id', 'backend_generated_sorting_preview',
                    '{\"plan\":true}', '{\"scope\":true}', '{\"folder\":true}', '[]',
                    'preview-hash', 'apply_plan_preview_snapshot_v3', 'sha256',
                    '{\"provenance\":true}', 12, 43,
                    '2026-01-05T03:04:00Z', '2026-01-06T03:04:00Z'",
                [],
                preview_snapshot_row_from_row,
            )
            .expect("map preview snapshot row");

        assert_eq!(snapshot_row.id, 45);
        assert_eq!(snapshot_row.snapshot_id, "snapshot-stable-id");
        assert_eq!(
            snapshot_row.source_plan_kind,
            "backend_generated_sorting_preview"
        );
        assert_eq!(snapshot_row.source_plan_json, "{\"plan\":true}");
        assert_eq!(
            snapshot_row.source_scope_json.as_deref(),
            Some("{\"scope\":true}")
        );
        assert_eq!(
            snapshot_row.folder_config_json.as_deref(),
            Some("{\"folder\":true}")
        );
        assert_eq!(snapshot_row.context_trail_json, "[]");
        assert_eq!(snapshot_row.preview_snapshot_hash, "preview-hash");
        assert_eq!(
            snapshot_row.preview_snapshot_hash_version,
            "apply_plan_preview_snapshot_v3"
        );
        assert_eq!(snapshot_row.preview_snapshot_hash_algorithm, "sha256");
        assert_eq!(
            snapshot_row.preview_snapshot_provenance_json,
            "{\"provenance\":true}"
        );
        assert_eq!(snapshot_row.scan_session_id, Some(12));
        assert_eq!(snapshot_row.consumed_apply_plan_id, Some(43));
        assert_eq!(snapshot_row.created_at, "2026-01-05T03:04:00Z");
        assert_eq!(
            snapshot_row.expires_at.as_deref(),
            Some("2026-01-06T03:04:00Z")
        );
    }

    #[test]
    fn creator_persists_backend_preview_hash_and_audit_columns() {
        let mut connection = memory_connection();
        insert_snapshot_preview_files(&connection);

        let preview = generate_sorting_preview_snapshot(
            &mut connection,
            &snapshot_settings(),
            snapshot_preview_request(),
        )
        .expect("preview snapshot through boundary module");
        let snapshot = load_preview_snapshot(&connection, preview.preview_snapshot_id)
            .expect("load preview snapshot")
            .expect("persisted preview snapshot");

        assert_eq!(
            snapshot.source_plan_kind,
            apply_plan_provenance::BACKEND_GENERATED_SORTING_PREVIEW_SOURCE_KIND
        );
        assert_eq!(
            snapshot.preview_snapshot_hash,
            preview.preview_snapshot_hash
        );
        assert_eq!(
            snapshot.preview_snapshot_hash_version,
            apply_plan_provenance::PREVIEW_SNAPSHOT_HASH_VERSION
        );
        assert_eq!(snapshot.preview_snapshot_hash_algorithm, "sha256");
        assert!(snapshot.snapshot_id.starts_with("apply-preview-"));
        assert!(snapshot.consumed_apply_plan_id.is_none());
        assert_eq!(preview.preview_snapshot_created_at, snapshot.created_at);
        assert!(snapshot.source_plan_json.contains("\"itemCount\":2"));
        assert!(snapshot.source_scope_json.is_some());
        assert_eq!(snapshot.context_trail_json, "[]");
    }
}
