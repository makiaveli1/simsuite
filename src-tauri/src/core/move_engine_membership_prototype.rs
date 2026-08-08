#![cfg(test)]

use rusqlite::{params, Connection, OptionalExtension, Transaction};

use crate::{
    database,
    error::{AppError, AppResult},
};

#[derive(Debug, Clone, PartialEq, Eq)]
struct FixtureMembershipState {
    file_id: i64,
    path: String,
    source_location: String,
    download_item_id: Option<i64>,
}

#[derive(Debug, Clone)]
enum FixtureMembershipAction {
    Transition {
        expected: FixtureMembershipState,
        final_state: FixtureMembershipState,
    },
    Delete {
        expected: FixtureMembershipState,
    },
    InsertRestored {
        final_state: FixtureMembershipState,
    },
}

fn load_fixture_membership_state(
    connection: &Connection,
    file_id: i64,
) -> AppResult<Option<FixtureMembershipState>> {
    connection
        .query_row(
            "SELECT id, path, source_location, download_item_id
             FROM files
             WHERE id = ?1",
            params![file_id],
            |row| {
                Ok(FixtureMembershipState {
                    file_id: row.get(0)?,
                    path: row.get(1)?,
                    source_location: row.get(2)?,
                    download_item_id: row.get(3)?,
                })
            },
        )
        .optional()
        .map_err(Into::into)
}

fn load_fixture_membership_state_in_transaction(
    transaction: &Transaction<'_>,
    file_id: i64,
) -> AppResult<Option<FixtureMembershipState>> {
    transaction
        .query_row(
            "SELECT id, path, source_location, download_item_id
             FROM files
             WHERE id = ?1",
            params![file_id],
            |row| {
                Ok(FixtureMembershipState {
                    file_id: row.get(0)?,
                    path: row.get(1)?,
                    source_location: row.get(2)?,
                    download_item_id: row.get(3)?,
                })
            },
        )
        .optional()
        .map_err(Into::into)
}

fn require_same_file_id(
    expected: &FixtureMembershipState,
    final_state: &FixtureMembershipState,
) -> AppResult<()> {
    if expected.file_id != final_state.file_id {
        return Err(AppError::Message(format!(
            "Fixture membership transition cannot change file id {} to {}.",
            expected.file_id, final_state.file_id
        )));
    }
    Ok(())
}

fn apply_fixture_membership_action(
    transaction: &Transaction<'_>,
    action: &FixtureMembershipAction,
) -> AppResult<()> {
    match action {
        FixtureMembershipAction::Transition {
            expected,
            final_state,
        } => {
            require_same_file_id(expected, final_state)?;
            let current = load_fixture_membership_state_in_transaction(
                transaction,
                expected.file_id,
            )?
            .ok_or_else(|| {
                AppError::Message(format!(
                    "Fixture membership row {} disappeared before transition.",
                    expected.file_id
                ))
            })?;

            if current == *final_state {
                return Ok(());
            }
            if current != *expected {
                return Err(AppError::Message(format!(
                    "Fixture membership row {} changed before transition.",
                    expected.file_id
                )));
            }

            let updated = transaction.execute(
                "UPDATE files
                 SET path = ?1,
                     source_location = ?2,
                     download_item_id = ?3,
                     indexed_at = CURRENT_TIMESTAMP
                 WHERE id = ?4
                   AND path = ?5
                   AND source_location = ?6
                   AND download_item_id IS ?7",
                params![
                    final_state.path,
                    final_state.source_location,
                    final_state.download_item_id,
                    expected.file_id,
                    expected.path,
                    expected.source_location,
                    expected.download_item_id,
                ],
            )?;
            if updated != 1 {
                return Err(AppError::Message(format!(
                    "Fixture membership row {} became stale during transition.",
                    expected.file_id
                )));
            }
        }
        FixtureMembershipAction::Delete { expected } => {
            let Some(current) =
                load_fixture_membership_state_in_transaction(transaction, expected.file_id)?
            else {
                return Ok(());
            };
            if current != *expected {
                return Err(AppError::Message(format!(
                    "Fixture membership row {} changed before delete.",
                    expected.file_id
                )));
            }

            let deleted = transaction.execute(
                "DELETE FROM files
                 WHERE id = ?1
                   AND path = ?2
                   AND source_location = ?3
                   AND download_item_id IS ?4",
                params![
                    expected.file_id,
                    expected.path,
                    expected.source_location,
                    expected.download_item_id,
                ],
            )?;
            if deleted != 1 {
                return Err(AppError::Message(format!(
                    "Fixture membership row {} became stale during delete.",
                    expected.file_id
                )));
            }
        }
        FixtureMembershipAction::InsertRestored { final_state } => {
            if let Some(current) =
                load_fixture_membership_state_in_transaction(transaction, final_state.file_id)?
            {
                if current == *final_state {
                    return Ok(());
                }
                return Err(AppError::Message(format!(
                    "Fixture restored row id {} is already owned by different membership.",
                    final_state.file_id
                )));
            }

            let existing_path_owner: Option<i64> = transaction
                .query_row(
                    "SELECT id FROM files WHERE path = ?1",
                    params![final_state.path],
                    |row| row.get(0),
                )
                .optional()?;
            if let Some(owner_id) = existing_path_owner {
                return Err(AppError::Message(format!(
                    "Fixture restored path is already owned by row {owner_id}."
                )));
            }

            transaction.execute(
                "INSERT INTO files (
                    id, path, filename, extension, hash, size, kind, subtype, confidence,
                    source_location, download_item_id, safety_notes, parser_warnings,
                    insights, indexed_at
                 ) VALUES (
                    ?1, ?2, ?3, '.package', 'fixture-hash', 100, 'Gameplay',
                    'Fixture', 1.0, ?4, ?5, '[]', '[]', '{}', CURRENT_TIMESTAMP
                 )",
                params![
                    final_state.file_id,
                    final_state.path,
                    final_state
                        .path
                        .rsplit('/')
                        .next()
                        .unwrap_or("restored.package"),
                    final_state.source_location,
                    final_state.download_item_id,
                ],
            )?;
        }
    }

    Ok(())
}

fn reconcile_fixture_membership_batch(
    connection: &Connection,
    actions: &[FixtureMembershipAction],
) -> AppResult<()> {
    let transaction = connection.unchecked_transaction()?;
    for action in actions {
        apply_fixture_membership_action(&transaction, action)?;
    }
    transaction.commit()?;
    Ok(())
}

fn fixture_installed_ids(connection: &Connection) -> AppResult<Vec<i64>> {
    let mut statement = connection.prepare(
        "SELECT id
         FROM files
         WHERE source_location IN ('mods', 'tray')
         ORDER BY id",
    )?;
    let rows = statement
        .query_map([], |row| row.get::<_, i64>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

fn fixture_installed_paths(connection: &Connection) -> AppResult<Vec<String>> {
    let mut statement = connection.prepare(
        "SELECT path
         FROM files
         WHERE source_location IN ('mods', 'tray')
         ORDER BY path",
    )?;
    let rows = statement
        .query_map([], |row| row.get::<_, String>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOWNLOAD_ITEM_ID: i64 = 77;

    fn setup_fixture_connection() -> Connection {
        let mut connection = Connection::open_in_memory().expect("fixture database");
        database::initialize(&mut connection).expect("production schema");
        connection
            .execute(
                "INSERT INTO download_items (
                    id, source_path, display_name, source_kind, source_size,
                    detected_file_count, status, notes
                 ) VALUES (?1, '/fixture/download.zip', 'Fixture download', 'archive',
                           100, 0, 'pending', '[]')",
                params![DOWNLOAD_ITEM_ID],
            )
            .expect("fixture download item");
        connection
    }

    fn state(
        file_id: i64,
        path: &str,
        source_location: &str,
        download_item_id: Option<i64>,
    ) -> FixtureMembershipState {
        FixtureMembershipState {
            file_id,
            path: path.to_owned(),
            source_location: source_location.to_owned(),
            download_item_id,
        }
    }

    fn insert_state(connection: &Connection, value: &FixtureMembershipState) {
        connection
            .execute(
                "INSERT INTO files (
                    id, path, filename, extension, hash, size, kind, subtype, confidence,
                    source_location, download_item_id, safety_notes, parser_warnings,
                    insights, indexed_at
                 ) VALUES (
                    ?1, ?2, ?3, '.package', 'fixture-hash', 100, 'Gameplay',
                    'Fixture', 1.0, ?4, ?5, '[]', '[]', '{}', CURRENT_TIMESTAMP
                 )",
                params![
                    value.file_id,
                    value.path,
                    value.path.rsplit('/').next().unwrap_or("fixture.package"),
                    value.source_location,
                    value.download_item_id,
                ],
            )
            .expect("insert fixture membership row");
    }

    #[test]
    fn exact_membership_transitions_and_restored_insert_are_idempotent() {
        let connection = setup_fixture_connection();
        let download = state(
            1,
            "/fixture/Downloads/incoming.package",
            "downloads",
            Some(DOWNLOAD_ITEM_ID),
        );
        let installed_to_downloads = state(2, "/fixture/Mods/old.package", "mods", None);
        let installed_delete = state(3, "/fixture/Tray/old.trayitem", "tray", None);
        insert_state(&connection, &download);
        insert_state(&connection, &installed_to_downloads);
        insert_state(&connection, &installed_delete);
        assert_eq!(fixture_installed_ids(&connection).expect("initial membership"), vec![2, 3]);

        let actions = vec![
            FixtureMembershipAction::Transition {
                expected: download.clone(),
                final_state: state(
                    1,
                    "/fixture/Mods/incoming.package",
                    "mods",
                    Some(DOWNLOAD_ITEM_ID),
                ),
            },
            FixtureMembershipAction::Transition {
                expected: installed_to_downloads.clone(),
                final_state: state(
                    2,
                    "/fixture/Downloads/old.package",
                    "downloads",
                    Some(DOWNLOAD_ITEM_ID),
                ),
            },
            FixtureMembershipAction::Delete {
                expected: installed_delete.clone(),
            },
            FixtureMembershipAction::InsertRestored {
                final_state: state(4, "/fixture/Tray/restored.package", "tray", None),
            },
        ];

        reconcile_fixture_membership_batch(&connection, &actions)
            .expect("apply exact membership batch");
        assert_eq!(fixture_installed_ids(&connection).expect("final membership"), vec![1, 4]);
        assert!(load_fixture_membership_state(&connection, 3)
            .expect("deleted state")
            .is_none());

        reconcile_fixture_membership_batch(&connection, &actions)
            .expect("repeat reconciliation must be idempotent");
        assert_eq!(fixture_installed_ids(&connection).expect("repeated membership"), vec![1, 4]);
    }

    #[test]
    fn second_membership_write_failure_rolls_back_first_change() {
        let connection = setup_fixture_connection();
        let first = state(
            10,
            "/fixture/Downloads/first.package",
            "downloads",
            Some(DOWNLOAD_ITEM_ID),
        );
        let second = state(
            11,
            "/fixture/Downloads/second.package",
            "downloads",
            Some(DOWNLOAD_ITEM_ID),
        );
        insert_state(&connection, &first);
        insert_state(&connection, &second);
        connection
            .execute_batch(
                "CREATE TRIGGER reject_second_membership_write
                 BEFORE UPDATE ON files
                 WHEN OLD.id = 11
                 BEGIN
                    SELECT RAISE(ABORT, 'forced second membership failure');
                 END;",
            )
            .expect("install second write failure");

        let error = reconcile_fixture_membership_batch(
            &connection,
            &[
                FixtureMembershipAction::Transition {
                    expected: first.clone(),
                    final_state: state(
                        10,
                        "/fixture/Mods/first.package",
                        "mods",
                        Some(DOWNLOAD_ITEM_ID),
                    ),
                },
                FixtureMembershipAction::Transition {
                    expected: second.clone(),
                    final_state: state(
                        11,
                        "/fixture/Mods/second.package",
                        "mods",
                        Some(DOWNLOAD_ITEM_ID),
                    ),
                },
            ],
        )
        .expect_err("second DB write must abort the full membership batch");
        assert!(error.to_string().contains("forced second membership failure"));
        assert_eq!(load_fixture_membership_state(&connection, 10).expect("first row"), Some(first));
        assert_eq!(load_fixture_membership_state(&connection, 11).expect("second row"), Some(second));
        assert!(fixture_installed_ids(&connection)
            .expect("rolled-back installed set")
            .is_empty());
    }

    #[test]
    fn stale_path_location_and_download_ownership_fail_closed_without_overwrite() {
        let connection = setup_fixture_connection();
        let expected = state(
            20,
            "/fixture/Downloads/original.package",
            "downloads",
            Some(DOWNLOAD_ITEM_ID),
        );
        insert_state(&connection, &expected);
        let newer = state(20, "/fixture/Tray/newer.package", "tray", None);
        connection
            .execute(
                "UPDATE files
                 SET path = ?1,
                     source_location = ?2,
                     download_item_id = ?3
                 WHERE id = ?4",
                params![
                    newer.path,
                    newer.source_location,
                    newer.download_item_id,
                    newer.file_id,
                ],
            )
            .expect("simulate newer DB ownership state");

        let error = reconcile_fixture_membership_batch(
            &connection,
            &[FixtureMembershipAction::Transition {
                expected,
                final_state: state(
                    20,
                    "/fixture/Mods/original.package",
                    "mods",
                    Some(DOWNLOAD_ITEM_ID),
                ),
            }],
        )
        .expect_err("stale membership evidence must fail closed");
        assert!(error
            .to_string()
            .contains("changed before transition"));
        assert_eq!(
            load_fixture_membership_state(&connection, 20).expect("newer state survives"),
            Some(newer)
        );
        assert_eq!(
            fixture_installed_ids(&connection).expect("newer installed membership survives"),
            vec![20]
        );
    }

    #[test]
    fn forward_then_reverse_reconciliation_restores_semantic_membership_with_new_row_id() {
        let connection = setup_fixture_connection();
        let incoming_download = state(
            30,
            "/fixture/Downloads/new.package",
            "downloads",
            Some(DOWNLOAD_ITEM_ID),
        );
        let old_installed = state(31, "/fixture/Mods/old.package", "mods", None);
        insert_state(&connection, &incoming_download);
        insert_state(&connection, &old_installed);
        let original_installed_paths =
            fixture_installed_paths(&connection).expect("initial installed paths");

        let incoming_installed = state(
            30,
            "/fixture/Mods/new.package",
            "mods",
            Some(DOWNLOAD_ITEM_ID),
        );
        reconcile_fixture_membership_batch(
            &connection,
            &[
                FixtureMembershipAction::Transition {
                    expected: incoming_download.clone(),
                    final_state: incoming_installed.clone(),
                },
                FixtureMembershipAction::Delete {
                    expected: old_installed.clone(),
                },
            ],
        )
        .expect("forward membership batch");
        assert_eq!(
            fixture_installed_paths(&connection).expect("forward installed paths"),
            vec!["/fixture/Mods/new.package".to_owned()]
        );

        reconcile_fixture_membership_batch(
            &connection,
            &[
                FixtureMembershipAction::Transition {
                    expected: incoming_installed,
                    final_state: incoming_download,
                },
                FixtureMembershipAction::InsertRestored {
                    final_state: state(310, "/fixture/Mods/old.package", "mods", None),
                },
            ],
        )
        .expect("reverse membership batch");
        assert_eq!(
            fixture_installed_paths(&connection).expect("restored installed paths"),
            original_installed_paths
        );
        assert!(load_fixture_membership_state(&connection, 31)
            .expect("old row id")
            .is_none());
        assert_eq!(
            load_fixture_membership_state(&connection, 310)
                .expect("restored new row")
                .expect("restored row exists")
                .path,
            "/fixture/Mods/old.package"
        );

        reconcile_fixture_membership_batch(
            &connection,
            &[
                FixtureMembershipAction::Transition {
                    expected: state(
                        30,
                        "/fixture/Mods/new.package",
                        "mods",
                        Some(DOWNLOAD_ITEM_ID),
                    ),
                    final_state: state(
                        30,
                        "/fixture/Downloads/new.package",
                        "downloads",
                        Some(DOWNLOAD_ITEM_ID),
                    ),
                },
                FixtureMembershipAction::InsertRestored {
                    final_state: state(310, "/fixture/Mods/old.package", "mods", None),
                },
            ],
        )
        .expect("repeat reverse reconciliation");
        assert_eq!(
            fixture_installed_paths(&connection).expect("idempotent restored membership"),
            original_installed_paths
        );
    }

    #[test]
    fn membership_prototype_is_database_only_and_not_registered_as_a_command() {
        let prototype_source = include_str!("move_engine_membership_prototype.rs");
        assert!(prototype_source.starts_with("#![cfg(test)]"));
        let forbidden_std_fs = ["std", "::fs"].concat();
        let forbidden_move = ["move_single_", "file("].concat();
        let forbidden_restore = ["restore_backed_up_", "file("].concat();
        let forbidden_hash = ["file_", "hash("].concat();
        assert!(!prototype_source.contains(&forbidden_std_fs));
        assert!(!prototype_source.contains(&forbidden_move));
        assert!(!prototype_source.contains(&forbidden_restore));
        assert!(!prototype_source.contains(&forbidden_hash));

        let commands_source = include_str!("../commands/mod.rs");
        assert!(!commands_source.contains("move_engine_membership_prototype"));
        assert!(!commands_source.contains("reconcile_fixture_membership_batch"));

        let core_source = include_str!("mod.rs");
        assert!(core_source.contains(
            "#[cfg(test)]\npub mod move_engine_membership_prototype;"
        ));
    }
}
