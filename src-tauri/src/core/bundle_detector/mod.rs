use std::{
    collections::{BTreeMap, HashSet},
    path::Path,
};

use rusqlite::{params, Connection};

use crate::error::AppResult;

#[derive(Debug)]
struct TrayFileRecord {
    id: i64,
    path: String,
    extension: String,
}

pub fn rebuild_bundles(connection: &mut Connection) -> AppResult<usize> {
    connection.execute("UPDATE files SET bundle_id = NULL", [])?;
    connection.execute("DELETE FROM bundles", [])?;

    let mut statement = connection.prepare(
        "SELECT id, path, extension
         FROM files
         WHERE kind LIKE 'Tray%'",
    )?;

    let tray_files = statement
        .query_map([], |row| {
            Ok(TrayFileRecord {
                id: row.get(0)?,
                path: row.get(1)?,
                extension: row.get(2)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    drop(statement);

    let mut grouped = BTreeMap::<String, Vec<TrayFileRecord>>::new();
    for file in tray_files {
        let path = Path::new(&file.path);
        let stem = path
            .file_stem()
            .map(|value| value.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown".to_owned());
        let parent = path
            .parent()
            .map(|value| value.to_string_lossy().to_string())
            .unwrap_or_default();
        grouped
            .entry(format!("{parent}::{stem}"))
            .or_default()
            .push(file);
    }

    let transaction = connection.transaction()?;
    for (bundle_key, files) in grouped {
        let extensions = files
            .iter()
            .map(|file| file.extension.as_str())
            .collect::<HashSet<_>>();

        let bundle_type = if extensions.contains(".householdbinary")
            || extensions.contains(".hhi")
            || extensions.contains(".sgi")
        {
            "household"
        } else if extensions.contains(".blueprint") || extensions.contains(".bpi") {
            "lot"
        } else if extensions.contains(".room") || extensions.contains(".rmi") {
            "room"
        } else {
            "tray"
        };

        let confidence = if files.len() > 1 { 0.96 } else { 0.72 };

        transaction.execute(
            "INSERT INTO bundles (bundle_type, bundle_name, file_count, confidence, created_at)
             VALUES (?1, ?2, ?3, ?4, CURRENT_TIMESTAMP)",
            params![bundle_type, bundle_key, files.len() as i64, confidence],
        )?;
        let bundle_id = transaction.last_insert_rowid();

        for file in files {
            transaction.execute(
                "UPDATE files SET bundle_id = ?1 WHERE id = ?2",
                params![bundle_id, file.id],
            )?;
        }
    }

    transaction.commit()?;

    let count: i64 = connection.query_row("SELECT COUNT(*) FROM bundles", [], |row| row.get(0))?;
    Ok(count as usize)
}
