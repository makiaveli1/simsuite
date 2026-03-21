use crate::adapters::{RemoteSnapshot, SnapshotEvidence};
use crate::error::{AppError, AppResult};
use crate::models::AccessTier;
use rusqlite::{params, Connection};
use sha2::{Digest, Sha256};

#[derive(Debug)]
pub struct SnapshotStore;

impl SnapshotStore {
    pub fn store_snapshot(
        conn: &Connection,
        binding_id: &str,
        snapshot: &RemoteSnapshot,
    ) -> AppResult<String> {
        if binding_id.is_empty() {
            return Err(AppError::Message("binding_id cannot be empty".into()));
        }

        tracing::info!("Storing snapshot for binding: {}", binding_id);

        let id = uuid::Uuid::new_v4().to_string();

        let snapshot_hash = compute_snapshot_hash(
            snapshot.title.as_deref(),
            snapshot.version_text.as_deref(),
            snapshot.download_url.as_deref(),
        );

        let raw_summary_json = serde_json::to_string(&snapshot.raw).map_err(|e| {
            tracing::error!("Failed to serialize raw summary: {}", e);
            AppError::Json(e)
        })?;
        let asset_names_json =
            serde_json::to_string(&snapshot.release_asset_names).map_err(|e| {
                tracing::error!("Failed to serialize asset names: {}", e);
                AppError::Json(e)
            })?;
        let image_hashes_json = serde_json::to_string(&snapshot.image_hashes).map_err(|e| {
            tracing::error!("Failed to serialize image hashes: {}", e);
            AppError::Json(e)
        })?;

        conn.execute(
            "INSERT INTO remote_snapshots (
                id, binding_id, snapshot_hash, title, version_text, published_at,
                download_url, changelog_url, release_id, asset_names_json,
                image_hashes_json, raw_summary_json, etag, last_modified, fetched_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            rusqlite::params![
                id,
                binding_id,
                snapshot_hash,
                snapshot.title,
                snapshot.version_text,
                snapshot.published_at,
                snapshot.download_url,
                snapshot.changelog_url,
                snapshot.release_id,
                asset_names_json,
                image_hashes_json,
                raw_summary_json,
                snapshot.etag,
                snapshot.last_modified,
                chrono::Utc::now().to_rfc3339(),
            ],
        )?;

        tracing::debug!("Stored snapshot {} for binding {}", id, binding_id);

        Ok(id)
    }

    pub fn get_latest_snapshot(
        conn: &Connection,
        binding_id: &str,
    ) -> AppResult<Option<RemoteSnapshot>> {
        if binding_id.is_empty() {
            return Err(AppError::Message("binding_id cannot be empty".into()));
        }

        let mut statement = conn.prepare(
            "SELECT id, binding_id, snapshot_hash, title, version_text, published_at,
                    download_url, changelog_url, release_id, asset_names_json,
                    image_hashes_json, raw_summary_json, etag, last_modified, fetched_at
             FROM remote_snapshots
             WHERE binding_id = ?1
             ORDER BY fetched_at DESC
             LIMIT 1",
        )?;

        let result = statement.query_row(params![binding_id], |row| {
            let asset_names_json: String = row.get(9)?;
            let image_hashes_json: String = row.get(10)?;
            let raw_summary_json: String = row.get(11)?;

            let release_asset_names = serde_json::from_str(&asset_names_json).unwrap_or_else(|e| {
                tracing::warn!("Failed to parse asset names JSON: {}", e);
                Vec::new()
            });
            let image_hashes = serde_json::from_str(&image_hashes_json).unwrap_or_else(|e| {
                tracing::warn!("Failed to parse image hashes JSON: {}", e);
                Vec::new()
            });
            let raw = serde_json::from_str(&raw_summary_json).unwrap_or_else(|e| {
                tracing::warn!("Failed to parse raw summary JSON: {}", e);
                serde_json::Value::Null
            });

            Ok(RemoteSnapshot {
                binding_id: row.get(1)?,
                title: row.get(3)?,
                version_text: row.get(4)?,
                published_at: row.get(5)?,
                download_url: row.get(6)?,
                changelog_url: row.get(7)?,
                release_id: row.get(8)?,
                release_asset_names,
                image_hashes,
                etag: row.get(12)?,
                last_modified: row.get(13)?,
                evidence: SnapshotEvidence::default(),
                confidence: 1.0,
                raw,
                access_tier: AccessTier::Public,
                patron_free_version: None,
            })
        });

        match result {
            Ok(snapshot) => Ok(Some(snapshot)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => {
                tracing::error!(
                    "Database error fetching latest snapshot for {}: {}",
                    binding_id,
                    e
                );
                Err(e.into())
            }
        }
    }

    pub fn should_refetch(
        conn: &Connection,
        binding_id: &str,
        etag: Option<&str>,
        last_modified: Option<&str>,
    ) -> bool {
        if etag.is_none() && last_modified.is_none() {
            tracing::debug!(
                "should_refetch({}): no etag or last_modified provided",
                binding_id
            );
            return true;
        }

        let stored = match Self::get_latest_snapshot(conn, binding_id) {
            Ok(Some(s)) => s,
            Ok(None) => {
                tracing::debug!("should_refetch({}): no stored snapshot found", binding_id);
                return true;
            }
            Err(e) => {
                tracing::warn!(
                    "should_refetch({}): failed to get latest snapshot: {}",
                    binding_id,
                    e
                );
                return true;
            }
        };

        if let Some(new_etag) = etag {
            if stored.etag.as_deref() != Some(new_etag) {
                tracing::debug!("should_refetch({}): etag changed", binding_id);
                return true;
            }
        }

        if let Some(new_lm) = last_modified {
            if stored.last_modified.as_deref() != Some(new_lm) {
                tracing::debug!("should_refetch({}): last_modified changed", binding_id);
                return true;
            }
        }

        false
    }

    pub fn get_latest_hash(conn: &Connection, binding_id: &str) -> AppResult<Option<String>> {
        if binding_id.is_empty() {
            return Err(AppError::Message("binding_id cannot be empty".into()));
        }

        let mut statement = conn.prepare(
            "SELECT snapshot_hash FROM remote_snapshots
             WHERE binding_id = ?1
             ORDER BY fetched_at DESC
             LIMIT 1",
        )?;

        let result = statement.query_row(params![binding_id], |row| row.get(0));

        match result {
            Ok(hash) => Ok(Some(hash)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => {
                tracing::error!(
                    "Database error fetching latest hash for {}: {}",
                    binding_id,
                    e
                );
                Err(e.into())
            }
        }
    }
}

fn compute_snapshot_hash(
    title: Option<&str>,
    version: Option<&str>,
    download_url: Option<&str>,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(title.unwrap_or("").as_bytes());
    hasher.update(version.unwrap_or("").as_bytes());
    hasher.update(download_url.unwrap_or("").as_bytes());
    format!("{:x}", hasher.finalize())
}
