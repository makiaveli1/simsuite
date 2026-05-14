# SimSuite Library Backend Systems Map

Date: 2026-05-13

This map is based on current repo inspection, originally created on `codex/library-backend-map-duplicates-v1` and refreshed on `codex/library-duplicate-truth-engine-v2`, `codex/library-duplicate-truth-guardrails-v21`, `codex/library-backend-performance-folder-query-v1`, `codex/library-true-empty-folder-metadata-v1`, `codex/library-thumbnail-preview-pipeline-v1`, `codex/library-large-scale-backend-stress-v1`, `codex/trust-boundaries-automation-readiness-v1`, `codex/library-duplicate-truth-engine-v3-fingerprints`, `codex/staging-backend-safety-readiness-v1`, `codex/staging-preview-plan-foundation-v1`, `codex/auto-sorting-rules-audit-v1`, `codex/suggested-plan-generator-v1`, and `codex/organize-plan-review-ui-v1`. It describes what the Library backend does today, where the current truth boundaries are, and where the backend is partial or missing.

## 1. Backend Architecture Overview

The Library backend is a Rust/Tauri backend over SQLite.

- Tauri command registration lives in `src-tauri/src/lib.rs`.
- Command wrappers live mainly in `src-tauri/src/commands/mod.rs`.
- SQLite setup and schema repair live in `src-tauri/src/database/mod.rs`, with the initial schema embedded from `src-tauri/src/database/schema/mod.rs`.
- The Library index/query layer lives in `src-tauri/src/core/library_index/mod.rs`.
- Scan/indexing lives in `src-tauri/src/core/scanner/mod.rs`.
- Package and Tray inspection lives in `src-tauri/src/core/file_inspector/mod.rs`.
- Duplicate detection lives in `src-tauri/src/core/duplicate_detector/mod.rs`.
- Bundle/same-pack grouping lives in `src-tauri/src/core/bundle_detector/mod.rs`.
- Update/watch source logic lives in `src-tauri/src/core/content_versions/mod.rs` and `src-tauri/src/core/watch_polling/mod.rs`.
- Review queue and organization preview logic live in `src-tauri/src/core/rule_engine/mod.rs`.
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
| `get_folder_tree_metadata` | implemented | Builds folder tree metadata from scan-owned `library_folders` rows plus file-count aggregation, so real empty Mods/Tray folders can appear after scan. |
| `get_file_detail` | implemented | Lazy detail query with watch, duplicate, review, and preview resolution. |
| `reveal_file_in_folder` | implemented | Opens Explorer for file or parent folder; uses real paths. |
| `get_duplicate_overview` | implemented | Counts duplicate rows by stored type. |
| `list_duplicate_pairs` | implemented | Lists exact duplicate, name-match review, and version-review pairs. Exact rows can now explain same file, same package, or same script contents. |
| `get_review_queue` | implemented | Reads rule-engine review queue data. |
| `get_staging_areas` | implemented, read-only | Lists app-local staged folders and file counts. The current user-facing Plan Preview UI uses this as preview/readiness data only; internal staging names remain stable. |
| `get_staging_preview_plan` | implemented, read-only | Returns a preview-only `StagingPlan` from current folder-level staging data with `wouldTouchFiles=false`; it does not call commit, cleanup, or move-engine apply paths. User-facing copy calls this Plan Preview/Pending Plans. |
| `generate_sorting_preview_plan` | implemented, read-only | Returns preview-only organization suggestions for selected Library files or a bounded Mods/Tray folder scope. It uses `StagingPlan` items with buckets, source signals, blocked reasons, confidence labels, and `wouldTouchFiles=false`; it does not call legacy Organize apply paths, Staging commit/cleanup commands, or move-engine apply paths. |
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

Trust-sensitive future work should follow `docs/TRUST_BOUNDARIES_AND_AUTOMATION_READINESS.md` before adding sorting, update replacement, AI-assisted decisions, cleanup, quarantine, move/disable/delete, provider/source, staging apply controls, or duplicate-handling automation.

## 3. Database Map

Important Library tables:

| Table | Purpose | Writes | Reads | Risks |
| --- | --- | --- | --- | --- |
| `files` | Main indexed Library/download file rows. | Scanner, downloads/staging flows. | Library list, folder, detail, duplicates, watch, review. | Large-library query and folder filtering need stress proof. |
| `creators` / `creator_aliases` / `user_creator_aliases` | Creator metadata and learned aliases. | Seed, scanner, user learning. | Library facets, detail, duplicate display. | Missing creator is common and must remain weak evidence. |
| `bundles` | Same-pack/bundle grouping. | Bundle detector. | Library relationship hints, folder summaries. | Same pack is not duplicate proof. |
| `library_folders` | Real Mods/Tray folder metadata, including empty folders. | Scanner during scan/rescan. | Folder tree metadata and real Open Folder path plumbing. | Existing libraries need a scan/rescan before old empty folders appear. |
| `duplicates` | Stored exact duplicate and comparison rows. | Duplicate detector after scan. | Duplicate overview, Duplicates route, Library duplicate flags. | Schema still stores `exact`, `filename`, `version`; exact rows must validate same file/package/script contents before user-facing Duplicate is shown. |
| `review_queue` | Files needing manual review. | Scanner/rule engine. | Needs Review, Library problem signals. | Review means manual review, not broken content proof. |
| `content_watch_sources` / `content_watch_results` | Update/source watch configuration and last results. | Updates/watch commands. | Library update cues, Updates route. | Provider checks are limited; no official-source claim. |
| `scan_sessions` | Scan summary history. | Scanner. | Home/Library status. | Mostly summary state. |
| `snapshots` / `snapshot_items` | Snapshot/restore support. | Snapshot manager. | Restore flows. | Not duplicate cleanup proof. |
| `app_settings` / `seed_meta` | Settings and seed metadata. | Database/settings code. | App setup and seeded taxonomy. | Schema mirror is partly stale compared with `ensure_schema`. |

Important indexes:

- `idx_files_hash`, `idx_files_content_fingerprint`, `idx_files_filename`, `idx_files_creator_id`, `idx_files_bundle_id`, `idx_files_kind`, `idx_files_source_location`.
- `idx_files_download_item_id`, `idx_files_source_location_kind`, `idx_files_source_location_filename`, `idx_files_relative_depth`, `idx_files_source_location_depth`.
- `idx_library_folders_source_location`, `idx_library_folders_source_path`, `idx_library_folders_source_parent`, `idx_library_folders_source_depth`.
- `idx_duplicates_duplicate_type`, `idx_duplicates_file_id_a`, `idx_duplicates_file_id_b`.
- `idx_review_queue_created_at`, `idx_review_queue_file_id`.
- `idx_content_watch_sources_kind`, `idx_content_watch_sources_anchor_file_id`, `idx_content_watch_results_status`.

`database/migrations/0001_initial.sql` is closer to current state than `database/schema/simsuite-v1.sql`. The schema mirror is stale and should not be treated as the full current schema.

## 4. Scanner / Indexing Map

Scanner behavior:

- Walks configured Mods and Tray roots.
- Records real folder metadata for the root, child, and nested directories, including folders with no supported files.
- Indexes only supported Sims file rows as Library content; folder rows are separate and do not create fake files.
- Stores real file paths, filename, extension, source location, relative depth, size, timestamps, creator/category metadata, parser warnings, safety notes, optional hash, and optional package/script content fingerprint metadata.
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

- `get_folder_tree_metadata` builds a folder tree from scan-owned `library_folders` rows, then applies indexed file rows for direct and total file counts.
- `list_library_folder_files` returns direct or recursive files under a folder path through SQL-scoped source/depth/path criteria.
- Mods/Tray source roots are represented through `source_location` and normalized folder paths.
- Empty folder nodes carry `disk_path` when the scanner has seen the real folder.
- Open/reveal uses real disk paths. The frontend prefers backend `diskPath` for folder nodes and falls back to configured roots only when needed.

Weak areas:

- `list_library_folder_files` no longer broad-loads the Library before filtering, but path-prefix matching still lacks a dedicated normalized relative path column/index.
- Folder tree file counts still aggregate from indexed file rows; a future normalized relative path/index could make very large folder trees cheaper.
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
- `library_folders` indexes support source/path/parent/depth folder tree loading.
- The opt-in `npm run test:library:stress` harness inserts 10,000 synthetic Library rows plus 5,002 duplicate stress rows and prints query timings without committing generated output.
- Recent 10,000-row synthetic timings were informational: list first page about 103 ms, search about 18 ms, filter about 17 ms, sort about 110 ms, folder tree metadata about 119 ms, large direct folder page about 80 ms, recursive folder page about 67 ms, relationship-heavy page about 29 ms, file detail about 4 ms, duplicate overview about 0 ms, preview diagnostics about 73 ms on this machine.

Risks:

- Folder path-prefix matching for file listing still needs a normalized relative path/index if 10,000+ file proof shows it is slow.
- Folder tree file counts still load metadata-only file rows for aggregation, but the 10,000-row synthetic proof was acceptable on this machine.
- `list_library_files_for_tree` is still registered for compatibility, though bounded.
- Relationship peer counts still aggregate over filtered list sets, but the stress harness showed the relationship-heavy filtered page path remained reasonable at 1,500 related rows. A more advanced SQL/cache pass remains future work if real libraries show broader filtered-set costs.
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

Current Plan Preview route behavior:

- Visible in Seasoned and Creator modes.
- Loads app-local staged folder metadata through `get_staging_areas`.
- Loads a read-only preview plan through `get_staging_preview_plan`.
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
- The visible Organize route no longer calls the legacy preview/apply/snapshot APIs.
- Legacy backend apply/snapshot commands still exist for older surfaces and must stay unexposed from the new suggested-plan UI until the Apply Safety Contract is designed.
