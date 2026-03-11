use rusqlite::{params, Connection};

use crate::{error::AppResult, models::SnapshotSummary};

#[derive(Debug, Clone)]
pub struct SnapshotItemRecord {
    pub file_id: Option<i64>,
    pub original_path: String,
    pub original_hash: Option<String>,
    pub backup_path: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SnapshotRecord {
    pub id: i64,
    pub snapshot_name: String,
}

pub fn create_snapshot(
    connection: &mut Connection,
    snapshot_name: &str,
    description: Option<&str>,
    items: &[SnapshotItemRecord],
) -> AppResult<SnapshotRecord> {
    let transaction = connection.transaction()?;
    transaction.execute(
        "INSERT INTO snapshots (snapshot_name, description, created_at)
         VALUES (?1, ?2, CURRENT_TIMESTAMP)",
        params![snapshot_name, description],
    )?;
    let snapshot_id = transaction.last_insert_rowid();

    {
        let mut insert_item = transaction.prepare(
            "INSERT INTO snapshot_items (snapshot_id, file_id, original_path, original_hash, backup_path)
             VALUES (?1, ?2, ?3, ?4, ?5)",
        )?;
        for item in items {
            insert_item.execute(params![
                snapshot_id,
                item.file_id,
                item.original_path,
                item.original_hash,
                item.backup_path
            ])?;
        }
    }

    transaction.commit()?;

    Ok(SnapshotRecord {
        id: snapshot_id,
        snapshot_name: snapshot_name.to_owned(),
    })
}

pub fn list_snapshots(connection: &Connection, limit: i64) -> AppResult<Vec<SnapshotSummary>> {
    let mut statement = connection.prepare(
        "SELECT
            s.id,
            s.snapshot_name,
            s.description,
            s.created_at,
            COUNT(si.id) AS item_count
         FROM snapshots s
         LEFT JOIN snapshot_items si ON si.snapshot_id = s.id
         GROUP BY s.id
         ORDER BY s.created_at DESC, s.id DESC
         LIMIT ?1",
    )?;

    let items = statement
        .query_map(params![limit], |row| {
            Ok(SnapshotSummary {
                id: row.get(0)?,
                snapshot_name: row.get(1)?,
                description: row.get(2)?,
                created_at: row.get(3)?,
                item_count: row.get(4)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(items)
}
