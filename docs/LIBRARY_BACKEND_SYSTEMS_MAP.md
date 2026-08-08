# SimSuite Library Backend Systems Map

Date: 2026-05-13

This map is based on current repo inspection, originally created on `codex/library-backend-map-duplicates-v1` and refreshed on `codex/library-duplicate-truth-engine-v2`, `codex/library-duplicate-truth-guardrails-v21`, `codex/library-backend-performance-folder-query-v1`, `codex/library-true-empty-folder-metadata-v1`, `codex/library-thumbnail-preview-pipeline-v1`, `codex/library-large-scale-backend-stress-v1`, `codex/trust-boundaries-automation-readiness-v1`, `codex/library-duplicate-truth-engine-v3-fingerprints`, `codex/staging-backend-safety-readiness-v1`, `codex/staging-preview-plan-foundation-v1`, `codex/auto-sorting-rules-audit-v1`, `codex/suggested-plan-generator-v1`, `codex/organize-plan-review-ui-v1`, `codex/rename-staging-plan-preview-v1`, `codex/plan-preview-organize-consolidation-v1`, `codex/organize-pending-plans-ux-clarity-v1`, `codex/inbox-batch-review-clarity-v1`, `codex/apply-safety-contract-design-v1`, `codex/existing-systems-integration-contract-v1`, `codex/applyplan-persistence-audit-v1`, `codex/applyplan-persistence-foundation-v1`, `codex/applyplan-builder-from-stagingplan-v1`, `codex/saved-plan-ui-review-v1`, `codex/validation-conflict-preview-design-v1`, `codex/read-only-applyplan-validation-preview-v1`, `codex/organize-validation-preview-ui-v1`, `codex/backup-restore-result-log-design-v1`, `codex/result-restore-schema-foundation-v1`, `codex/fixture-only-backup-prototype-v1`, `codex/fixture-only-restore-prototype-v1`, `codex/fixture-backup-restore-integration-proof-v1`, `codex/result-restore-review-ui-v1`, `codex/dry-run-apply-design-v1`, `codex/read-only-dry-run-apply-command-v1`, and `codex/organize-dry-run-preview-ui-v1`. It describes what the Library backend does today, where the current truth boundaries are, and where the backend is partial or missing.

## 1. Backend Architecture Overview

The Library backend is a Rust/Tauri backend over SQLite.

- Tauri command registration lives in `src-tauri/src/lib.rs`.
- Command wrappers live mainly in `src-tauri/src/commands/mod.rs`.
- SQLite setup and schema repair live in `src-tauri/src/database/mod.rs`, with the initial schema embedded from `src-tauri/src/database/schema/mod.rs`.
- The Library index/query layer lives in `src-tauri/src/core/library_index/mod.rs`. Library Folder Content Identity V1 uses current-profile folder identity for read-only selected-folder file scoping, prefers component-safe comparison keys, uses exact observed relative paths when keys are unavailable, blocks stale or incomplete identity, and retains the configured-root query only for legacy folders. Library Folder Tree Profile Identity V1 gives each requested source a current-profile, legacy, or blocked mode, derives current nesting from root-relative identity, and counts only matching profile/root files. Library Folder Tree SQL Counts V1 lets SQLite group direct file counts by scan-owned parent identity while Rust preserves the existing visible nodes and recursive totals; rows without parent identity retain the metadata-row compatibility path.
- Scan/indexing lives in `src-tauri/src/core/scanner/mod.rs`. Indexed Folder Identity V1 writes nullable active-profile/root-relative identity and a root-policy-aware comparison key into newly scanned `library_folders` rows without extra filesystem probes. Library Folder Identity Uniqueness V2 uses strict folder inserts after clearing the scanned source snapshot and lets the database apply sensitive, insensitive, unknown-policy, or legacy uniqueness safely. Indexed File Parent Identity V1 also records each installed file's nullable parent root-relative path and optional parent comparison key from the same confirmed scan evidence; Downloads/Inbox rows remain unassigned.
- Package and Tray inspection lives in `src-tauri/src/core/file_inspector/mod.rs`.
- Duplicate detection lives in `src-tauri/src/core/duplicate_detector/mod.rs`.
- Bundle/same-pack grouping lives in `src-tauri/src/core/bundle_detector/mod.rs`.
- Update/watch source logic lives in `src-tauri/src/core/content_versions/mod.rs` and `src-tauri/src/core/watch_polling/mod.rs`.
- Review queue and organization preview logic live in `src-tauri/src/core/rule_engine/mod.rs`.
- Shared preview destination validation and root-policy-aware reservation logic live in `src-tauri/src/core/validator/mod.rs`. Organize and guided-install planning each create one read-only reservation context per batch, probe configured Mods and Tray roots once, and use hash-based sensitive/folded comparison keys without enabling file execution.
- DB-only ApplyPlan draft/preview persistence lives in `src-tauri/src/core/apply_plan_persistence.rs`.
- Read-only ApplyPlan validation/conflict preview logic lives in `src-tauri/src/core/apply_plan_validation.rs`.
- DB-only ApplyPlan run/result/restore metadata helpers live in `src-tauri/src/core/apply_plan_results.rs`.
- Read-only ApplyPlan dry-run classification logic lives in `src-tauri/src/core/apply_plan_dry_run.rs`.
- Fixture-only ApplyPlan backup, restore, and integrated recovery proof logic lives in `src-tauri/src/core/apply_plan_backup_prototype.rs`; it is private backend test/prototype code and is not registered as a command.
- Fixture-only transaction/recovery, Library-membership handoff, replacement/new-ID restore, and sorting-to-Apply/Undo bridge proof logic lives in `src-tauri/src/core/apply_plan_fixture_transaction_prototype.rs`. The module is compiled only under Rust tests. Its sorting bridge starts from the real backend sorting generator and real saved ApplyPlan snapshot, but a `suggest_move` remains non-executable until a test-only authorization receipt proves the exact plan/item fingerprint, physical source hash/size, source/destination paths, and verified backup chain. The macOS fixture now also proves conservative creation/Undo ownership for one missing immediate child organization folder under Mods: creation intent is recorded before one non-recursive `create_dir`, exact device/inode identity is recorded before claiming ownership, and cleanup removes the folder only after verified file/Library Undo when the same directory is still empty on disk and unclaimed by Library rows. Player-added content, Library-only child claims, same-name replacement directories, and interrupted ownership finalization all force the folder to be kept rather than guessed safe to delete. This remains test-only, single-file, one-level macOS proof work; recursive folders, multi-file batch execution, full real-process restart recovery, cross-platform production directory identity/removal, real confirmation, and user-file Apply/Restore remain disabled.
- Frontend API wrappers and mock fallback data live in `src/lib/api.ts` and `src/lib/types.ts`.

The normal flow is:

1. User configures Mods/Tray/Downloads paths.
2. A scan walks configured roots, inspects files, stores rows in SQLite, queues review items, rebuilds bundle metadata, and rebuilds duplicate pairs.
3. Library commands query paged rows, folder metadata, file details, watch/update evidence, duplicate evidence, and review signals.
4. The React frontend consumes Tauri commands through typed wrappers and keeps broader review workflows in Library, Duplicates, Updates, and Needs Review.

## 2. Current Command Map

| Command | Status | Notes |
| --- | --- | --- |
| `get_library_settings` | implemented | Reads configured roots and preferences. |
| `save_library_paths` | implemented | Writes root paths. |
| `detect_default_library_paths` | implemented | Attempts default Sims folder detection. |
| `scan_library` | implemented | Blocking full scan path; older command still registered. |
| `start_scan` | implemented | Async scan wrapper with scan status tracking. |
| `get_scan_status` | implemented | Exposes current scan progress/state. |
| `get_library_facets` | implemented | Returns creator/type/source facets for filters. |
| `get_library_summary` | implemented | Returns Library summary counts. |
| `get_library_preview_diagnostics` | implemented | Returns sanitized preview coverage counts only; no filenames, paths, thumbnails, or user file payloads. |
| `list_library_files` | implemented | Paged Library rows; preview payload is controlled by `include_previews`. |
| `list_library_folder_files` | implemented, recently hardened | Returns direct or recursive folder contents through SQL-scoped source/depth/path filters, supports Library filters/search/sort, pagination, and preview control. |
| `list_library_files_for_tree` | compatibility endpoint, bounded | Still registered for older callers, but now caps returned rows at 5,000 and strips previews. `get_folder_tree_metadata` plus `list_library_folder_files` is the preferred path. |
| `get_folder_tree_metadata` | implemented, profile-aware | Builds read-only folder tree metadata from scan-owned `library_folders` rows. Each requested Mods or Tray source uses current-profile/root-relative identity when trustworthy, keeps the legacy all-null path/depth fallback, or is omitted when identity is stale, incomplete, or unsafe. File counts use the same source mode, and real empty folders still appear after scan. |
| `get_file_detail` | implemented | Lazy detail query with watch, duplicate, review, and preview resolution. |
| `reveal_file_in_folder` | implemented | Opens Explorer for file or parent folder; uses real paths. |
| `get_duplicate_overview` | implemented | Counts duplicate rows by stored type. |
| `list_duplicate_pairs` | implemented | Lists exact duplicate, name-match review, and version-review pairs. Exact rows can now explain same file, same package, or same script contents. |
| `get_review_queue` | implemented | Reads rule-engine review queue data. |
| `get_staging_areas` | implemented, read-only | Lists app-local staged folders and file counts. Organize consumes this as `Pending batches` handoff data; raw internal IDs should stay hidden behind technical details in the UI. The direct Plan Preview route remains compatibility-only. Internal staging names remain stable. |
| `get_staging_preview_plan` | implemented, read-only | Returns a preview-only `StagingPlan` from current folder-level staging data with `wouldTouchFiles=false`; it does not call commit, cleanup, or move-engine apply paths. Organize displays this as a folder-level pending-data note, not as saved organization-plan rows. |
| `generate_sorting_preview_plan` | implemented, read-only | Returns preview-only organization suggestions for selected Library files or a bounded Mods/Tray folder scope. One shared destination-reservation context per preview compares proposed relative paths under the configured root's proven case policy; insensitive or unknown roots fail closed on case-only aliases, sensitive roots preserve valid case-distinct paths, and distinct Mods/Tray roots remain separate unless they resolve to the same root. Hash-based keys keep reservation checks bounded for low-spec systems. It uses `StagingPlan` items with buckets, source signals, blocked reasons, confidence labels, and `wouldTouchFiles=false`; it does not call legacy Organize apply paths, Staging commit/cleanup commands, or move-engine apply paths. |
| Downloads/Inbox intake commands | implemented, mixed read/write internally | Current visible Inbox copy treats downloaded/imported batches as review-only intake data and blocks file-changing controls. The underlying Downloads backend still has mutation paths for future or legacy workflows, so visible exposure must continue to follow the trust-boundary safety contract. |
| Apply/file-changing command family | implemented internally, blocked from current visible workflows | `apply_preview_organization`, `apply_download_item(s)`, `apply_guided_download_item`, `apply_special_review_fix`, mutating `apply_review_plan_action` branches, `restore_snapshot`, `undo_applied_item`, `reject_download_item(s)`, `restore_rejected_item`, and move-engine helpers can change files or app-local staged files. `docs/planning/APPLY_SAFETY_CONTRACT_V1.md` defines the future gate before any normal UI exposure. |
| ApplyPlan persistence, builder, and saved-plan UI | implemented, DB-only/read-only foundation plus visible review UI | `save_apply_plan_preview`, `list_saved_apply_plans`, `get_apply_plan`, and `delete_draft_apply_plan` save/list/read/soft-cancel draft preview records only. `build_apply_plan_from_staging_plan` builds a saved draft from the existing read-only sorting preview generator and persistence foundation. Organize now uses the builder/list/get/cancel APIs for `Saved plans`. Saved records keep `would_touch_files=0` and `applyable_items=0`. Result/restore metadata tables exist as DB-only records, but real Apply does not exist. |
| ApplyPlan validation/conflict preview | implemented, read-only response v1 with visible Organize review UI | `preview_apply_plan_validation` loads saved draft ApplyPlan records, item snapshots, blockers/signals, current Library file rows, and configured roots to report stale sources, missing sources, missing roots, unsafe destinations, destination conflicts, review-only/duplicate-review blockers, and backup requirements. Organize `Saved plans` now calls this command through `previewApplyPlanValidation` and shows summary counts, caveats, and item reasons. It does not persist validation state, create folders/backups, call file-changing commands, or mark plans ready for confirmation. |
| Backup/restore/result-log ApplyPlan contract | DB-only schema/API foundation plus read-only UI review | `docs/planning/BACKUP_RESTORE_RESULT_LOG_DESIGN_V1.md` recommends future copy-backup-first recovery. `apply_plan_runs`, `apply_plan_results`, and `apply_plan_restore_entries` now exist as metadata tables with DB-only commands/API. Organize `Saved plans` can show existing run/result/restore metadata as read-only Recovery history. No backup execution, restore execution, real Apply, or visible file-changing control exists yet. |
| Fixture-only backup/restore prototype | backend test/prototype only | `apply_plan_backup_prototype` copies and verifies temporary test backup files, then restores from recorded backup references to temporary fixture targets and records safe result/restore metadata. The integrated proof runs both helpers as one temporary-file recovery chain. It is not a Tauri command, not UI/API-exposed, and not a user-file backup or restore system. |
| Dry-run Apply preview | implemented, read-only backend/API plus Organize review UI | `preview_apply_plan_dry_run` classifies saved draft items after validation as blocked, skipped, requiring review, requiring backup, requiring destination review, or candidate after future safety gates. It returns `canProceedToApply=false`, `canProceedToConfirmation=false`, and item `canApply=false`. Organize `Saved plans` can now display those classifications through a user-triggered read-only `Dry-run preview` section. It writes no DB rows, creates no result logs or restore entries, calls no fixture helpers, and does not execute Apply, Restore, backup, folder creation, or file-changing behavior. |
| `cleanup_staging_areas` | implemented backend command, not exposed by current Staging UI | Deletes selected app-local staging folders under the staging root. Future exposure requires the trust-boundary file-change checklist. |
| `commit_staging_area` / `commit_all_staging_areas` | implemented backend commands, not exposed by current Staging UI | Can apply move-engine paths for ReadyNow download items. Future exposure requires preview, confirmation, backup/restore, recoverable errors, and proof. |
| `list_library_watch_items` | implemented | Library watch source overview. |
| `list_library_watch_setup_items` | implemented | Files needing update source setup. |
| `list_library_watch_review_items` | implemented | Watch results needing review. |
| `save_watch_source_for_file` | implemented | Saves one update/watch source. |
| `save_watch_sources_for_files` | implemented | Bulk save watch sources. |
| `clear_watch_source_for_file` | implemented | Clears watch source. |
| `refresh_watch_source_for_file` | implemented | Refreshes one watch source. |
| `refresh_watched_sources` | implemented | Refreshes watched sources. |
| Creator/category audit commands | implemented | Used by Library-adjacent metadata cleanup. |

No command currently proves dependency relationships, missing meshes, safe deletion, safe replacement, official source identity, or automatic update replacement.

Trust-sensitive future work should follow `docs/TRUST_BOUNDARIES_AND_AUTOMATION_READINESS.md`, `docs/planning/APPLY_SAFETY_CONTRACT_V1.md`, and `docs/planning/EXISTING_SYSTEMS_INTEGRATION_CONTRACT_V1.md` before adding sorting apply, update replacement, AI-assisted decisions, cleanup, quarantine, move/disable/delete, provider/source, staging apply controls, or duplicate-handling automation.

The integration contract is the durable guardrail for future systems: reuse scanner/indexer facts, file inspector output, Library APIs, duplicate detector truth, Updates/watch state, Review signals, Inbox intake state, Organize preview plans, and `StagingPlan` before adding new data paths. If a new system needs new data or logic, its final report must explain the existing systems reused and why the new logic was necessary.

## 3. Database Map

Important Library tables:

| Table | Purpose | Writes | Reads | Risks |
| --- | --- | --- | --- | --- |
| `files` | Main indexed Library/download file rows. Newly scanned Mods/Tray rows can carry nullable `installation_profile_id`, `installation_root_id`, slash-normalized `profile_relative_path`, a versioned `profile_relative_path_key`, plus nullable `profile_parent_relative_path` and `profile_parent_relative_path_key` derived from the same confirmed root and case-policy evidence. | Scanner, downloads/staging flows. Downloads/Inbox explicitly leave installed parent identity null. | Library list, folder, detail, duplicates, watch, review. Folder-tree direct counts use the parent-identity index when available. | Existing rows gain comparison and parent identity only after a valid rescan; Downloads-only rows remain unassigned. Null parent identity keeps the compatibility count path. Exact duplicate rebuild, pair classification, and Library counts share one fail-closed identity rule. Complete profile/root/key tuples decide same-file exclusion; legacy rows may prove separation only through clearly different non-case-only paths; mixed, partial, case-only, or same-key identity is not exact duplicate proof. |
| `creators` / `creator_aliases` / `user_creator_aliases` | Creator metadata and learned aliases. | Seed, scanner, user learning. | Library facets, detail, duplicate display. | Missing creator is common and must remain weak evidence. |
| `bundles` | Same-pack/bundle grouping. | Bundle detector. | Library relationship hints, folder summaries. | Same pack is not duplicate proof. |
| `library_folders` | Real Mods/Tray folder metadata, including empty folders, plus nullable scan-owned profile/root-relative identity for newly scanned rows. | Scanner during scan/rescan. Folder rows are inserted strictly after the scanned source snapshot is cleared. | Selected-folder file queries and folder-tree construction use current-profile/root-relative identity when complete; Open Folder continues to use the stored real `full_path`. Genuinely legacy all-null rows retain compatibility behaviour. | Existing libraries need a scan/rescan before old empty folders or folder identity appear; no migration guesses identity for old rows. V2 preserves existing rows and IDs while splitting uniqueness into three partial indexes: keyed identity follows the proven case policy, unknown-key assigned rows remain lowercase-unique within their profile/root, and legacy rows remain lowercase-unique by source. Stale, incomplete, or unsafe identity is still blocked rather than silently merged. |
| `duplicates` | Stored exact duplicate and comparison rows. | Duplicate detector after scan. | Duplicate overview, Duplicates route, Library duplicate flags. | Schema still stores `exact`, `filename`, `version`; exact rows must validate same file/package/script contents before user-facing Duplicate is shown. |
| `review_queue` | Files needing manual review. | Scanner/rule engine. | Needs Review, Library problem signals. | Review means manual review, not broken content proof. |
| `content_watch_sources` / `content_watch_results` | Update/source watch configuration and last results. | Updates/watch commands. | Library update cues, Updates route. | Provider checks are limited; no official-source claim. |
| `scan_sessions` | Scan summary history. | Scanner. | Home/Library status. | Mostly summary state. |
| `snapshots` / `snapshot_items` | Snapshot/restore support. | Snapshot manager. | Restore flows. | Not duplicate cleanup proof. |
| `apply_plans` | Saved draft/preview ApplyPlan records. | DB-only ApplyPlan persistence commands. | Future saved-plan UI and ApplyPlan builder work. | Persistence only; not an Apply executor. |
| `apply_plan_items` | Saved item snapshots for preview candidates, blocked items, and review-only items. | DB-only ApplyPlan persistence commands. | Future validation/build views. | `file_id` is a soft reference; saved snapshots must be revalidated before any future file action. |
| `apply_plan_item_signals` | Normalized source-signal/evidence snapshots. | DB-only ApplyPlan persistence commands. | Future evidence review and audit logs. | Signals explain suggestions; they do not prove safety. |
| `apply_plan_item_blockers` | Normalized blocked/review-only/validation/source-plan reasons. | DB-only ApplyPlan persistence commands. | Future blocker review and validation. | Blockers must remain visible and cannot be silently applied. |
| Validation preview response model | Implemented response-only v1 with Organize UI. | Read-only `preview_apply_plan_validation`. | Organize saved-plan validation preview UI. | Must not write files, create folders, persist Apply readiness, or mark drafts ready for confirmation until the full Apply Safety Contract exists. |
| `apply_plan_runs` | Future ApplyPlan run metadata. | DB-only result/restore foundation commands. | Future result-log and recovery UI. | Metadata only; no runtime Apply executor exists. |
| `apply_plan_results` | Future per-file operation/result-log rows. | DB-only result/restore foundation commands. | Future result-log review. | V1 statuses cannot claim files were applied. |
| `apply_plan_restore_entries` | Future restore-map references scoped to one run/result/item. | DB-only result/restore foundation commands. | Future restore/recovery review. | V1 statuses cannot claim restore execution happened. |
| `app_settings` / `seed_meta` | Settings and seed metadata. | Database/settings code. | App setup and seeded taxonomy. | Schema mirror is partly stale compared with `ensure_schema`. |

Important indexes:

- `idx_files_hash`, `idx_files_content_fingerprint`, `idx_files_filename`, `idx_files_creator_id`, `idx_files_bundle_id`, `idx_files_kind`, `idx_files_source_location`.
- `idx_files_download_item_id`, `idx_files_source_location_kind`, `idx_files_source_location_filename`, `idx_files_relative_depth`, `idx_files_source_location_depth`.
- `idx_library_folders_source_location`, `idx_library_folders_source_path`, `idx_library_folders_source_parent`, `idx_library_folders_source_depth` remain the current compatibility indexes.
- `idx_library_folders_installation_identity` and `idx_library_folders_installation_comparison` support later profile/root-aware folder adoption without changing current Library queries.
- `idx_duplicates_duplicate_type`, `idx_duplicates_file_id_a`, `idx_duplicates_file_id_b`.
- `idx_review_queue_created_at`, `idx_review_queue_file_id`.
- `idx_content_watch_sources_kind`, `idx_content_watch_sources_anchor_file_id`, `idx_content_watch_results_status`.
- `idx_apply_plans_status_created_at`, `idx_apply_plans_source_staging_plan_id`, `idx_apply_plan_items_plan_id`, `idx_apply_plan_items_file_id`, `idx_apply_plan_items_status`, `idx_apply_plan_item_signals_item_id`, `idx_apply_plan_item_signals_kind`, `idx_apply_plan_item_blockers_item_id`, `idx_apply_plan_item_blockers_reason`.
- `idx_apply_plan_runs_plan_id`, `idx_apply_plan_runs_status_created_at`, `idx_apply_plan_results_run_id`, `idx_apply_plan_results_plan_item_id`, `idx_apply_plan_results_status`, `idx_apply_plan_restore_entries_run_id`, `idx_apply_plan_restore_entries_result_id`, `idx_apply_plan_restore_entries_item_id`, `idx_apply_plan_restore_entries_status`.

`database/migrations/0001_initial.sql` is closer to current state than `database/schema/simsuite-v1.sql`. The schema mirror is stale and should not be treated as the full current schema.

## 4. Scanner / Indexing Map

Scanner behavior:

- Walks configured Mods and Tray roots.
- Records real folder metadata for the root, child, and nested directories, including folders with no supported files.
- Indexes only supported Sims file rows as Library content; folder rows are separate and do not create fake files.
- Stores real file paths, filename, extension, source location, relative depth, size, timestamps, creator/category metadata, parser warnings, safety notes, optional hash, and optional package/script content fingerprint metadata. For active-profile-matched Mods/Tray scans it also stores scan-owned profile ID, root ID, and root-relative path metadata; mismatches and non-Unicode relative paths remain unassigned rather than guessed.
- Uses `scanner-v21` cache fingerprints to reuse unchanged file inspection results after package/script content fingerprints have been populated.
- Hashes only size-candidate duplicate groups instead of every file, which is good for scan performance.
- Handles inspection errors by recording warnings/default metadata instead of failing the whole scan.
- Rebuilds bundles and duplicate pairs after scan.

Partial or weak areas:

- Existing users need a scan/rescan before previously existing empty folders appear in Folder view.
- Large-library query proof now includes an opt-in 10,000-row synthetic backend stress harness. Full scan/watcher stress on real disk trees remains future work.
- Thumbnail validation has fixture proof but not live real-library proof.

## 5. Package Inspection Map

Package inspection boundary:

- `.package` files are parsed as DBPF where possible.
- `.ts4script` files are inspected as ZIP-like script mods.
- Tray extensions are classified as Tray content.
- Parser warnings and inspection failures are retained as manual review evidence.
- Thumbnail extraction is deferred during scan and resolved lazily for detail/preview surfaces.
- Selected `.package` detail loading can now persist a newly found embedded/game-cache preview back into indexed `files.insights`, so later row/grid/folder queries can reuse the preview without parsing during normal browsing.
- `.package` files can now receive a scan-time package content fingerprint when the DBPF index and every bounded resource payload can be read safely. The fingerprint uses resource type, group, instance IDs, and payload hashes in stable sorted order.
- `.ts4script` files can now receive a scan-time script content fingerprint when the archive can be read safely. The fingerprint uses normalized archive entry paths and entry payload hashes in stable sorted order.
- Fingerprint failures are sanitized status metadata. They do not become duplicate proof.

Thumbnail/preview state:

- Preview data currently lives in `FileInsights.thumbnail_preview` and `FileInsights.cached_thumbnail_preview`.
- `get_library_preview_diagnostics` classifies indexed rows as preview available, package deferred-or-missing, or unsupported by current extractor, using sanitized aggregate counts.
- Failure and stale-cache states are not persistently tracked yet. Diagnostics expose that limitation instead of pretending those states are known.
- `.ts4script`, Tray content, and unsupported file types do not currently have a committed thumbnail extractor.
- Fixture proof still mostly covers fallback behavior because fixture files do not contain real Sims thumbnail payloads.
- Real-library thumbnail validation remains future work and should report only sanitized counts unless the user explicitly provides shareable fixture content.

What is not proven:

- Dependency relationships.
- Missing mesh relationships.
- Recolor-to-mesh linking.
- Broken mod/CC state.
- Safe delete or safe replace.

## 6. Folder Backend Map

Folder systems:

- `get_folder_tree_metadata` chooses one read-only mode independently for each requested Mods or Tray source.
- Current-profile mode uses scan-owned profile, root, and root-relative folder identity. File counts include only matching profile/root rows, and nesting comes from `profile_relative_path` rather than absolute paths.
- Legacy mode keeps the previous absolute-path and `relative_depth` behaviour only for genuinely all-null folder and file identity.
- Blocked mode omits stale-profile, incomplete, or unsafe identity instead of mixing uncertain rows into the visible tree.
- `list_library_folder_files` follows its own current-profile, legacy, or blocked identity decision. Empty folders retain `disk_path`, and Open/reveal continues to use real disk paths.

Weak areas:

- Case-only siblings are representable only after a trustworthy scan supplies complete profile/root identity and a non-null comparison key from a proven case-sensitive root. Legacy rows and roots with unknown case policy intentionally remain conservative and cannot represent case-only aliases.
- Rescanned current-profile rows now use indexed parent identity and grouped SQLite direct counts. Legacy and pre-v13 rows intentionally retain metadata-row aggregation until rescan, so mixed old libraries may not receive the full memory reduction immediately.
- Folder metadata is scan-time state, not a live filesystem watcher. Users need a scan/rescan for newly created or removed empty folders.

## 7. Detail / Inspector Backend Map

`get_file_detail` returns:

- Identity fields, source, kind/subtype, confidence, creator, bundle metadata, size/path/timestamps.
- Parser warnings and scan safety notes.
- File insights.
- Watch/update source summary.
- Exact duplicate pair count and duplicate type list.
- Problem signals.
- Lazy preview/thumbnail resolution.

The detail backend is the right place for deeper evidence. List rows should remain summary-first.

## 8. Problem Signals Map

Current signal sources:

| Signal | Source | Strength | Route | Does not prove |
| --- | --- | --- | --- | --- |
| `review_suggested` | `review_queue` | strong enough for Needs Review | Needs Review | Broken content. |
| `inspection_failed` | parser/review reason | manual review | Needs Review | Broken content. |
| `safety_note` | scan safety note | manual review | Needs Review | Safe delete/dependency proof. |
| `parser_warning` | parser warnings | detected clue | Needs Review | Broken content. |
| `duplicate_candidate` | exact duplicate rows only for Library/detail signals | Duplicates | Safe delete or cleanup choice. |
| `stored_in_tray` | source location | factual source state | none | Active mod behavior. |
| `no_update_source` | watch state | factual missing watch config | Updates | Outdated state. |
| `weak_metadata` | low classification confidence | weak metadata clue | Needs Review | Broken content. |
| `missing_creator_metadata` | creator missing | metadata gap | none | Creator identity. |
| `script_mod_caution` | kind = ScriptMods | informational | none | Broken script mod. |

## 9. Related Item / Relationship Map

Relationship hints include:

- Duplicate/exact-content comparison from the duplicate detector.
- Same pack from bundle grouping.
- Same folder from folder peer counts.
- Tray grouping hints where available.

Only exact duplicate detector evidence should route from Library as a duplicate. Same-name, version, same pack, and same folder are review or related hints, not duplicate proof. None of these relationships prove dependencies.

## 10. Duplicate Detection Map

Before this sprint, duplicate detection worked as follows:

- `exact`: inserted when two files share the same non-empty hash.
- `filename`: inserted when filenames match case-insensitively and the pair is not already an exact hash pair.
- `version`: inserted when a version token can be stripped from different filenames to form the same canonical key.
- Overview counts rows by `duplicate_type`.
- Pair listing returns basic file identity, path, creator, hash, modified date, size, duplicate type, and detection method.

Duplicate Truth Engine v2 keeps the same database schema but makes the user-facing rule binary:

- Rows with deterministic exact proof are exposed as `isDuplicate = true`, label `Duplicate`, and careful evidence.
- `exact_file` proof requires two joined, distinct file IDs, non-empty normalized hashes that match, non-empty paths, and different normalized Windows paths.
- `exact_package` proof requires two joined, distinct `.package` rows with available matching `package` content fingerprints, matching fingerprint version, non-empty paths, and different normalized Windows paths.
- `exact_script` proof requires two joined, distinct `.ts4script` rows with available matching `script` content fingerprints, matching fingerprint version, non-empty paths, and different normalized Windows paths.
- Package/script fingerprint exacts still use `duplicates.duplicate_type = 'exact'`; the source is stored in `detection_method` as `package_fingerprint_v1` or `script_fingerprint_v1`.
- `filename` rows are exposed as `isDuplicate = false`, `comparisonKind = name_match_review`, label `Name match`.
- `version` rows are exposed as `isDuplicate = false`, `comparisonKind = version_review`, label `Version review`.
- Non-exact filename/version review pair generation is capped per candidate group to avoid unbounded all-pairs growth in large same-name/version-key groups. Exact duplicate truth rules are unchanged.
- Returned pairs include `is_duplicate`, `comparison_kind`, `classification`, `classification_label`, `confidence_label`, `evidence`, and `cautions`.
- Evidence can include same file contents, same package contents, same script contents, same filename, similar filename, version clue found, version differs, contents differ, size matches/differs, creator matches/differs/unknown, and detection method.
- Cautions explicitly keep comparison manual and state when a row is not duplicate proof.
- `get_file_detail`, Library row `has_duplicate`, Library duplicate filters, Home summary duplicate count, and Library summary duplicate count now count exact deterministic duplicates only.
- Stale/malformed exact rows with missing hashes, mismatched hashes, unavailable fingerprints, self-pairs, missing file joins, or same canonical path pairs are ignored by duplicate counts and filters.

Known false-positive risks:

- Same filename with different contents can be a different version or manual review case, not a real duplicate.
- Version-token matches are now version reviews, not duplicate claims.
- Missing creator/type should not raise confidence by itself.
- Same folder and same pack must remain related hints only.

Known false-negative risks:

- Package/script fingerprints are only populated after a scan or rescan with `scanner-v21`; older indexed rows need refresh before this proof is available.
- Same mod family with strongly different filenames may not be grouped.
- Installed-vs-inbox version comparisons are not fully modeled by duplicate detection today.
- Metadata-derived title/creator matches are not yet used as a bounded duplicate key.

## 11. Update / Watch Backend Map

Library-relevant update/watch behavior:

- Files without a watch source can show `No update source`.
- Saved watch sources can produce watch setup/review lists.
- Refresh commands exist for one file or many watched sources.
- Check failed and unknown result states are review/update-source states.

Not implemented:

- CurseForge integration.
- Generic web scraping.
- Automatic downloading/replacement.
- Official source proof.
- Definitive outdated proof.

## 12. Review Queue / Needs Review Map

Items enter review from scanner/rule-engine warnings such as:

- inspection failed,
- parser warning,
- safety note,
- weak or incomplete metadata,
- placement warnings.

Needs Review is a manual review queue. It is not a broken-content detector.

## 13. Performance and Scalability Map

What is already safer:

- Library list supports limit/offset paging.
- Folder file listing is now SQL-scoped and paged instead of broad list plus in-memory folder filtering.
- Folder tree metadata now has scan-owned folder rows, so empty folders no longer require fake file rows.
- Legacy `list_library_files_for_tree` is capped at 5,000 preview-light rows.
- Preview payloads are controlled by `include_previews`.
- Detail preview resolution is lazy and selected-file only; found previews are persisted for later indexed row/grid/folder display.
- Preview diagnostics are explicit and sanitized, not part of ordinary Library browsing.
- Duplicate hashing is candidate-based during scan.
- Duplicate list command has a limit.
- Core duplicate indexes exist.
- `idx_files_source_location_depth` supports source/depth folder filtering.
- `idx_files_installation_parent_identity` supports grouped current-profile folder-tree direct counts and exact current-profile direct-folder content lookups.
- `idx_files_missing_parent_identity` contains only identity-owned rows that still lack parent identity, making the old-row compatibility check cheap; it becomes empty after a full source rescan.
- `library_folders` indexes support source/path/parent/depth folder tree loading.
- The opt-in `pnpm run test:library:stress` harness inserts 10,000 synthetic Library rows plus 5,002 duplicate stress rows and prints query timings without committing generated output. `pnpm run test:library:folder-tree-stress` and `pnpm run test:library:folder-content-stress` run the focused 10,000-row Library comparison.
- The focused same-dataset folder-tree proof first runs all 10,000 files through the legacy fallback lane, then upgrades the in-memory rows to trustworthy profile/root/parent identity and reruns the grouped lane. Recent runs returned 504 grouped rows instead of 10,000 fallback rows, a stable 19.84x row reduction, while serialized `FolderTreeMetadata` remained exactly equal. Informational samples on this machine were about 549 ms versus 196 ms and 296 ms versus 110 ms; no timing threshold is enforced.
- The same fixture measures selected-folder content before and after the identity upgrade and records `EXPLAIN QUERY PLAN` evidence. Legacy and current responses remain exactly equal. Before parent identity, the direct 5,000-file target uses the 6,500-row `idx_files_source_location_depth` lane. After complete parent identity, SQLite uses the covering `idx_files_installation_parent_identity` lookup for the exact source/profile/root/parent tuple, while the compatibility probe uses `idx_files_missing_parent_identity`.
- Recursive selected-folder production listing now uses the measured component-boundary-preserving binary range strategy whenever trustworthy current-profile folder identity is available. Exact observed-path ranges use `idx_files_installation_identity`; comparison-key ranges use `idx_files_installation_comparison`. In the 10,000-file fixture, both current-profile lanes reduce the recursive candidate set from 9,000 source/depth rows to the exact 2,000 descendants while preserving case-sensitive siblings, unknown-policy exact spelling, lookalike-folder exclusion, stale-profile blocking, source filters, search, sorting, paging, preview payloads, and exact serialized responses. Legacy folders intentionally retain the previous source/depth/path compatibility query, and recursive root listing keeps its existing profile/root scope.
- A measurement-only general-listing breakdown on the same 10,000-row fixture isolated relationship-peer counting as the dominant paged-list cost before optimization. One run measured a 50-row Name page at 62 ms total: about 3 ms for the count query, 4 ms for the row-selection/order probe, and 54 ms for the old relationship path while materializing all 10,000 filtered rows. A 100-row Recently Modified page showed the same shape at 63 ms total, about 3 ms count, 5 ms row probe, and 52 ms relationship work over all 10,000 rows. `EXPLAIN QUERY PLAN` reported `USE TEMP B-TREE FOR ORDER BY` for both probes.
- Paged general Library responses now fetch the visible page before relationship counting. Same-folder counting narrows candidates to only the filtered source/effective-depth lanes represented on that page, then still applies the exact existing Rust folder-key comparison; same-pack counting uses SQL aggregation only for bundle IDs represented on the page. Unpaged compatibility calls retain the previous full peer-count path. On the same 10,000-row fixture, a 50-row Name page compared 10,000 rows in the old full map with 2,000 filtered folder candidates plus 25 bundle groups in the paged path; the isolated peer-count sample was 88 ms full versus 19 ms paged. A 100-row Recently Modified page showed the same 10,000-to-2,000 candidate reduction plus 25 bundle groups, with 64 ms full versus 22 ms paged. Every visible same-folder and same-pack count matched the old full-map result exactly. Normal regressions also cover off-page peers, active filters, and the existing `relative_depth.max(0)` root semantics. Timings remain informational and no performance threshold is enforced.

Risks:

- Current-profile files from before parent identity was introduced deliberately retain the previous prefix query until a full source rescan makes parent identity complete. Legacy and pre-v13 folder-tree counts likewise keep their compatibility fallback until rescan, so their memory and timing profile remains different from the grouped path even though both paths have same-dataset stress coverage.
- `list_library_files_for_tree` is still registered for compatibility, though bounded.
- Paged same-folder relationship counting still has to inspect all filtered rows in the page-relevant source/effective-depth lanes before exact Rust folder-key grouping. The synthetic Name and Recently Modified pages reduce this to 2,000 candidates from 10,000, but other filter/depth shapes can be wider. Representative sorts still use a temporary SQLite B-tree on this fixture, so sort-index work should remain a separate measured optimization rather than being mixed into the relationship-count change.
- Exact duplicate pair generation can still grow within very large same-hash groups. Non-exact filename/version review groups are now capped.
- No large real-library stress benchmark is committed yet.

## 14. Missing Feature Map

| Feature | Status | Recommendation |
| --- | --- | --- |
| SQL-direct folder content queries | already solved for v1 | `list_library_folder_files` now uses SQL-scoped source/depth/path filters with paging and preview control. |
| Improved duplicate version classification | already solved for current truth boundary | v2/v2.1 keep only validated same-file-content rows as duplicates; name/version/family rows are review/comparison only. |
| True empty disk-folder metadata | implemented for v1 | Scanner writes real `library_folders` rows for Mods/Tray folders, including empty folders. Existing libraries need a scan/rescan before old empty folders appear. |
| Large-library stress backend proof | partially solved | Added an opt-in 10,000-row synthetic backend stress harness and a 5,002-row duplicate review group stress harness. Real-library proof remains future work. |
| Live real-library thumbnail validation | partially prepared | Sanitized preview diagnostics and selected-file preview persistence now exist. Real CC/Tray thumbnail validation on user content is still future work. |
| Dependency detection | do not do until deterministic proof exists | Needs research and strong evidence model. |
| Missing mesh detection | do not do until deterministic proof exists | High false-positive risk. |
| Recolor-to-mesh linking | do later after deterministic metadata work | Must not imply dependency until proven. |
| Safe-delete proof/actions | should not do now | Requires exact identity, path context, backup/recovery, and explicit product design. |
| Duplicate cleanup actions | should not do now | Compare-only for now. |
| Provider onboarding / CurseForge | future product decision | Requires provider architecture and source trust model. |
| Plan Preview / internal Staging safety readiness | guarded for v1 | Current Plan Preview route is preview/readiness only and does not expose commit/reject controls. Backend mutating staging commands still exist and need a future Level 4 safety contract before UI exposure. |
| AI classification | should not do now | Deterministic signals should be stable first. |

## 15. Plan Preview / Internal Staging Readiness Map

Current Plan Preview / Pending Plans behavior:

- Normal top-level navigation no longer exposes Plan Preview in Casual, Seasoned, or Creator modes.
- Organize owns the user-facing `Saved plans` tab for saved draft preview records and the `Pending batches` tab for app-local staged folder handoff metadata through `get_staging_areas`.
- Organize and the direct compatibility route load a read-only preview plan through `get_staging_preview_plan`.
- The direct internal Plan Preview route remains routable and points users back to Organize.
- Shows item counts, pending plan folders/subfolders, byte totals, and preview-only readiness copy.
- Shows folder-level review items only; per-file organization suggestions remain future work.
- Does not call `commit_staging_area`, `commit_all_staging_areas`, or `cleanup_staging_areas`.
- Does not expose enabled move, delete, reject, quarantine, or apply controls.

Backend staging commands still exist:

- `get_staging_areas` is read-only.
- `get_staging_preview_plan` is read-only and returns `wouldTouchFiles=false`.
- `cleanup_staging_areas` can delete app-local staged extracted folders under `downloads_inbox`.
- `commit_staging_area` and `commit_all_staging_areas` can call the move engine for ReadyNow standard download items.

Future Plan Preview/internal Staging work must not expose the mutating commands until the workflow has:

- a per-file preview plan,
- evidence and caveats for each suggested action,
- user confirmation,
- backup/restore support,
- path validation,
- duplicate destination handling,
- recoverable error handling,
- per-file result logging,
- focused tests and desktop proof.

## 16. Auto Sorting Rules Readiness Map

`docs/planning/AUTO_SORTING_RULES_AUDIT_V1.md` defines the safe rule foundation for future Auto Sorting Suggested Plans.

Current backend and UI implications:

- `generate_sorting_preview_plan` exists and returns preview-only `StagingPlan` items with `wouldTouchFiles=false`.
- The generator uses Library metadata, scanner evidence, parser warnings, review queue state, duplicate proof, and bundle hints according to their evidence level.
- The generator does not call legacy apply paths such as `apply_preview_organization`, move-engine apply helpers, internal Staging commit commands, or cleanup commands.
- The visible Organize route now consumes `generate_sorting_preview_plan` and renders buckets, reasons, caveats, source signals, and blocked reasons.
- Organize can save a generated preview through `build_apply_plan_from_staging_plan`, then list, load, and soft-cancel saved draft records through the DB-only ApplyPlan APIs.
- The visible Organize route no longer calls the legacy preview/apply/snapshot APIs.
- Legacy backend apply/snapshot commands still exist for older surfaces and must stay unexposed from the new suggested-plan UI until the Apply Safety Contract is designed.
