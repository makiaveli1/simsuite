# SimSuite Library Backend Systems Map

Date: 2026-05-12

This map is based on current repo inspection, originally created on `codex/library-backend-map-duplicates-v1` and refreshed on `codex/library-duplicate-truth-engine-v2`, `codex/library-duplicate-truth-guardrails-v21`, `codex/library-backend-performance-folder-query-v1`, and `codex/library-true-empty-folder-metadata-v1`. It describes what the Library backend does today, where the current truth boundaries are, and where the backend is partial or missing.

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
| `list_library_files` | implemented | Paged Library rows; preview payload is controlled by `include_previews`. |
| `list_library_folder_files` | implemented, recently hardened | Returns direct or recursive folder contents through SQL-scoped source/depth/path filters, supports Library filters/search/sort, pagination, and preview control. |
| `list_library_files_for_tree` | compatibility endpoint, bounded | Still registered for older callers, but now caps returned rows at 5,000 and strips previews. `get_folder_tree_metadata` plus `list_library_folder_files` is the preferred path. |
| `get_folder_tree_metadata` | implemented | Builds folder tree metadata from scan-owned `library_folders` rows plus file-count aggregation, so real empty Mods/Tray folders can appear after scan. |
| `get_file_detail` | implemented | Lazy detail query with watch, duplicate, review, and preview resolution. |
| `reveal_file_in_folder` | implemented | Opens Explorer for file or parent folder; uses real paths. |
| `get_duplicate_overview` | implemented | Counts duplicate rows by stored type. |
| `list_duplicate_pairs` | implemented, now being strengthened | Lists duplicate pairs; previously exposed only `exact`, `filename`, and `version` plus basic fields. |
| `get_review_queue` | implemented | Reads rule-engine review queue data. |
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

## 3. Database Map

Important Library tables:

| Table | Purpose | Writes | Reads | Risks |
| --- | --- | --- | --- | --- |
| `files` | Main indexed Library/download file rows. | Scanner, downloads/staging flows. | Library list, folder, detail, duplicates, watch, review. | Large-library query and folder filtering need stress proof. |
| `creators` / `creator_aliases` / `user_creator_aliases` | Creator metadata and learned aliases. | Seed, scanner, user learning. | Library facets, detail, duplicate display. | Missing creator is common and must remain weak evidence. |
| `bundles` | Same-pack/bundle grouping. | Bundle detector. | Library relationship hints, folder summaries. | Same pack is not duplicate proof. |
| `library_folders` | Real Mods/Tray folder metadata, including empty folders. | Scanner during scan/rescan. | Folder tree metadata and real Open Folder path plumbing. | Existing libraries need a scan/rescan before old empty folders appear. |
| `duplicates` | Stored exact duplicate and comparison rows. | Duplicate detector after scan. | Duplicate overview, Duplicates route, Library duplicate flags. | Schema still stores `exact`, `filename`, `version`; v2 treats only `exact` rows with matching non-empty hashes as user-facing duplicates. |
| `review_queue` | Files needing manual review. | Scanner/rule engine. | Needs Review, Library problem signals. | Review means manual review, not broken content proof. |
| `content_watch_sources` / `content_watch_results` | Update/source watch configuration and last results. | Updates/watch commands. | Library update cues, Updates route. | Provider checks are limited; no official-source claim. |
| `scan_sessions` | Scan summary history. | Scanner. | Home/Library status. | Mostly summary state. |
| `snapshots` / `snapshot_items` | Snapshot/restore support. | Snapshot manager. | Restore flows. | Not duplicate cleanup proof. |
| `app_settings` / `seed_meta` | Settings and seed metadata. | Database/settings code. | App setup and seeded taxonomy. | Schema mirror is partly stale compared with `ensure_schema`. |

Important indexes:

- `idx_files_hash`, `idx_files_filename`, `idx_files_creator_id`, `idx_files_bundle_id`, `idx_files_kind`, `idx_files_source_location`.
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
- Stores real file paths, filename, extension, source location, relative depth, size, timestamps, creator/category metadata, parser warnings, safety notes, and optional hash.
- Uses `scanner-v20` cache fingerprints to reuse unchanged file inspection results.
- Hashes only size-candidate duplicate groups instead of every file, which is good for scan performance.
- Handles inspection errors by recording warnings/default metadata instead of failing the whole scan.
- Rebuilds bundles and duplicate pairs after scan.

Partial or weak areas:

- Existing users need a scan/rescan before previously existing empty folders appear in Folder view.
- Large-library scan still needs stress proof. Folder query coverage now includes a 1,076-row synthetic backend test, but 5,000 to 10,000 row proof remains future work.
- Thumbnail validation has fixture proof but not live real-library proof.

## 5. Package Inspection Map

Package inspection boundary:

- `.package` files are parsed as DBPF where possible.
- `.ts4script` files are inspected as ZIP-like script mods.
- Tray extensions are classified as Tray content.
- Parser warnings and inspection failures are retained as manual review evidence.
- Thumbnail extraction is deferred during scan and resolved lazily for detail/preview surfaces.

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

- Rows with deterministic exact-file proof are exposed as `isDuplicate = true`, `comparisonKind = exact_file`, label `Duplicate`, and evidence `Same file contents`.
- Exact-file proof requires two joined, distinct file IDs, non-empty normalized hashes that match, non-empty paths, and different normalized Windows paths.
- `filename` rows are exposed as `isDuplicate = false`, `comparisonKind = name_match_review`, label `Name match`.
- `version` rows are exposed as `isDuplicate = false`, `comparisonKind = version_review`, label `Version review`.
- Returned pairs include `is_duplicate`, `comparison_kind`, `classification`, `classification_label`, `confidence_label`, `evidence`, and `cautions`.
- Evidence can include same file contents, same filename, similar filename, version clue found, version differs, contents differ, size matches/differs, creator matches/differs/unknown, and detection method.
- Cautions explicitly keep comparison manual and state when a row is not duplicate proof.
- `get_file_detail`, Library row `has_duplicate`, Library duplicate filters, Home summary duplicate count, and Library summary duplicate count now count exact deterministic duplicates only.
- Stale/malformed exact rows with missing hashes, mismatched hashes, self-pairs, missing file joins, or same canonical path pairs are ignored by duplicate counts and filters.

Known false-positive risks:

- Same filename with different contents can be a different version or manual review case, not a real duplicate.
- Version-token matches are now version reviews, not duplicate claims.
- Missing creator/type should not raise confidence by itself.
- Same folder and same pack must remain related hints only.

Known false-negative risks:

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
- Detail preview resolution is lazy.
- Duplicate hashing is candidate-based during scan.
- Duplicate list command has a limit.
- Core duplicate indexes exist.
- `idx_files_source_location_depth` supports source/depth folder filtering.
- `library_folders` indexes support source/path/parent/depth folder tree loading.

Risks:

- Folder path-prefix matching for file listing still needs a normalized relative path/index if 10,000+ file proof shows it is slow.
- Folder tree file counts still load metadata-only file rows for aggregation and need larger-library proof.
- `list_library_files_for_tree` is still registered for compatibility, though bounded.
- Relationship peer counts currently run for filtered list sets and still need large all-library stress proof.
- Duplicate pair generation can still grow within very large same-name/version-key groups.
- No large real-library stress benchmark is committed yet.

## 14. Missing Feature Map

| Feature | Status | Recommendation |
| --- | --- | --- |
| SQL-direct folder content queries | already solved for v1 | `list_library_folder_files` now uses SQL-scoped source/depth/path filters with paging and preview control. |
| Improved duplicate version classification | already solved for current truth boundary | v2/v2.1 keep only validated same-file-content rows as duplicates; name/version/family rows are review/comparison only. |
| True empty disk-folder metadata | implemented for v1 | Scanner writes real `library_folders` rows for Mods/Tray folders, including empty folders. Existing libraries need a scan/rescan before old empty folders appear. |
| Large-library stress backend proof | should do later | Add 5,000 to 10,000 row synthetic and real-library-safe backend timing proof. |
| Live real-library thumbnail validation | should do later | Validate on safe user-provided or anonymized real content. |
| Dependency detection | do not do until deterministic proof exists | Needs research and strong evidence model. |
| Missing mesh detection | do not do until deterministic proof exists | High false-positive risk. |
| Recolor-to-mesh linking | do later after deterministic metadata work | Must not imply dependency until proven. |
| Safe-delete proof/actions | should not do now | Requires exact identity, path context, backup/recovery, and explicit product design. |
| Duplicate cleanup actions | should not do now | Compare-only for now. |
| Provider onboarding / CurseForge | future product decision | Requires provider architecture and source trust model. |
| Staging backend command cleanup | future work if still relevant | Audit after Library backend stabilization. |
| AI classification | should not do now | Deterministic signals should be stable first. |
