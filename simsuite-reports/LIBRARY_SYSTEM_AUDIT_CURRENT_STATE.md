# SimSuite Library System Audit - Current State

Date: April 24, 2026  
Scope: current codebase only, with previous reports treated as history, not truth.  
Mode: audit and planning only. No app behavior was changed.

## 1. Executive Summary

The Library has a serious amount of real work behind it. It is not a mock screen. There is a Rust/Tauri backend, SQLite storage, a scanner, DBPF/package inspection, duplicate detection, update-watch data, folder browsing, list view, grid view, an inspector sidebar, and a More Details sheet.

The honest answer: the Library is a strong prototype / internal alpha, not production-ready yet.

The biggest issue is not the UI. The biggest issue is that the main Library query currently contains invalid SQL in `src-tauri/src/core/library_index/mod.rs`. That means the list, grid, and folder-file loading paths can fail in the real Tauri app even though TypeScript, Vite, `cargo check`, and `cargo build --release` pass.

The second major issue is folder view. The desired product idea is good: browse the real Mods and Tray folders with SimSuite intelligence on top. But the current backend folder metadata command appears broken, and the frontend still depends on fetching full file rows to build usable folder contents. There is also a frontend bug where selecting folders nested deeper than one level can return empty contents because the folder lookup only searches roots and direct children.

The third major issue is relationship/dependency truth. The UI now talks about relationships and safe delete, but the real system does not have a dependency graph, recolor-to-mesh linking, missing mesh detection, package dependency detection, or real safe-delete preflight. It has useful first-pass signals: duplicate, tray bundle, same pack, same creator, and same folder. Some of those are only clues, and one backend "same folder" count is currently wrong enough to create false confidence.

## 2. What Is Fully Implemented

**Implemented**

- The app shell, modes, and navigation exist for Casual, Seasoned, and Creator modes.
- Library has list, grid, and folder view components.
- The frontend has paginated Library loading through `api.listLibraryFiles`.
- The scanner walks configured Mods and Tray folders and indexes supported Sims 4 file types.
- SQLite storage exists for files, creators, bundles, duplicate pairs, review queue, watch sources/results, downloads, snapshots, and settings.
- The scanner stores file path, filename, extension, size, timestamps, source location, relative depth, kind, subtype, confidence, creator, safety notes, parser warnings, and insight JSON.
- DBPF/package inspection extracts resource summaries, embedded names, creator hints, version hints/signals, family hints, and some package thumbnail data.
- Thumbnail support is first-party now: embedded package THUM first, then Sims 4 `localthumbcache.package`.
- Scan thumbnails are deferred, so the scanner does not try to decode every thumbnail during the scan.
- Exact/filename/version duplicate detection is implemented and has its own Duplicates page.
- Update-watch storage is implemented, with saved watch sources and watch results.
- Some update checks are implemented for supported official/special sources and generic GitHub release pages.
- CurseForge is recognized as needing a future approved API/provider path, not treated as fully automatic.
- Organize has a preview/apply/restore flow with snapshots.
- Review queue exists and is populated by low confidence, parser warnings, and safety notes.
- Open/reveal file or folder command is registered as `reveal_file_in_folder` and has a Windows Explorer implementation.

## 3. What Is Partially Implemented

**Partially implemented**

- Folder view is present, but the backend-native folder metadata path is likely broken and the frontend still needs full row data for useful folder content.
- Folder summary exists, but it can rely on false relationship counts and can fail to open empty folders because it may not know the absolute folder path.
- Grid thumbnails work when rows include preview base64, but there is no separate lazy media endpoint.
- Detail thumbnails load on selection, but this means selecting a file can trigger package/cache thumbnail work.
- Relationship signals exist in UI, but they are clue-level only. They are not true dependency detection.
- Safe-delete warnings exist, but they are not a safe-delete system.
- Updates are tracked as watch sources/results, but automatic update matching is limited.
- Staging has a screen and API wrapper functions, but the Tauri commands are not registered, so the Staging screen cannot work in the real app as written.
- The frontend has memoization and some virtualization, but only in narrow places.
- The Library code has mode-aware text and density controls, but some controls are still crowded or too technical for casual players.

## 4. What Is Stubbed / Mocked / Dead Code

**Stub/mock only**

- `src-tauri/src/core/ai_classifier/mod.rs` only contains `STATUS: "planned"`. There is no AI infrastructure.
- Relationship/dependency wording in the UI goes beyond what the backend can prove.
- Swatches are keyword-derived from names/hints, not real extracted color palettes.

**Dead or miswired**

- `get_staging_areas`, `cleanup_staging_areas`, `commit_staging_area`, and `commit_all_staging_areas` exist in `src-tauri/src/commands/mod.rs`, and `src/lib/api.ts` calls them, but they are not registered in `src-tauri/src/lib.rs`. `cargo check` also reports them as unused.
- `scan_library` is still registered, but the app uses `start_scan`. It looks like a legacy synchronous scan command.
- `get_folder_tree_metadata` is registered and called as a warm preload, but it appears to use SQLite `REVERSE()`, which SQLite does not provide by default.
- `database/schema/simsuite-v1.sql` is stale compared with `database/migrations/0001_initial.sql`.
- Thumbnail comments still mention Sims 4 Mod Manager in places, but the current production code path is first-party embedded THUM/cache.

## 5. Backend Current State

### Library commands

**Implemented and registered**

- `get_library_settings`
- `save_library_paths`
- `detect_default_library_paths`
- `pick_folder`
- `get_home_overview`
- `scan_library`
- `start_scan`
- `get_scan_status`
- `get_library_facets`
- `get_library_summary`
- `list_library_files`
- `list_library_files_for_tree`
- `get_folder_tree_metadata`
- `get_file_detail`
- `save_creator_learning`
- `save_category_override`
- `get_duplicate_overview`
- `list_duplicate_pairs`
- `list_library_watch_items`
- `list_library_watch_setup_items`
- `list_library_watch_review_items`
- `save_watch_source_for_file`
- `save_watch_sources_for_files`
- `clear_watch_source_for_file`
- `refresh_watch_source_for_file`
- `refresh_watched_sources`
- `reveal_file_in_folder`

**Used by the Library frontend**

- `get_library_facets`
- `get_library_summary`
- `list_library_files`
- `list_library_files_for_tree`
- `get_folder_tree_metadata`
- `get_file_detail`
- `save_creator_learning`
- `save_category_override`
- `reveal_file_in_folder`

**Used by connected pages**

- Duplicates: `get_duplicate_overview`, `list_duplicate_pairs`
- Updates: watch list/setup/review, save/clear/refresh watch source, `get_file_detail`
- Review: `get_review_queue`
- Organize: rule presets, preview organization, apply organization, snapshots, restore snapshot
- Downloads/Inbox: download item and install workflow commands

**Broken or risky**

- `list_library_files` builds invalid SQL. In both paged and unpaged paths it has a comma after `same_pack_peer_count` immediately before `FROM files f`. This is in `src-tauri/src/core/library_index/mod.rs`.
- `get_folder_tree_metadata` uses `REVERSE(f.path)`. No custom SQLite `REVERSE` function was found.
- Relationship counts are computed with weak partitions:
  - `same_folder_peer_count` partitions by `source_location` and `relative_depth`, not actual parent folder.
  - `same_pack_peer_count` partitions by `bundle_id`, which can make all null bundle rows look related.
- `get_library_summary` counts duplicates using only `file_id_a`, so files that only appear as `file_id_b` can be missed.
- `get_library_summary.disabled` is actually the Tray count, not disabled files.

## 6. Frontend Current State

### Major Library components

**Implemented**

- `src/screens/LibraryScreen.tsx`: main owner of Library data, filters, view mode, selected file/folder, page, density, detail sheets, and folder state.
- `src/screens/library/LibraryTopStrip.tsx`: search, sort, page size, view switcher, density slider, kind/status/subtype/filter controls.
- `LibraryCollectionTable.tsx`: list/table view.
- `LibraryThumbnailGrid.tsx`: grid/card view.
- `FolderTreePane.tsx`: folder tree.
- `FolderContentPane.tsx`: selected folder contents and folder summary layout.
- `VirtualizedLooseFiles.tsx`: virtualization for large root loose-file groups.
- `LibraryDetailsPanel.tsx`: right inspector sidebar.
- `LibraryDetailSheet.tsx`: More Details / detail sheet.
- `libraryDisplay.tsx`: display model builders, relationship helpers, detail sheet section builders, folder summary helpers, thumbnail/source helpers.
- `folderTree.ts`: frontend folder tree builder and folder content splitter.

### Ownership

**Implemented**

- `LibraryScreen.tsx` owns data loading.
- `LibraryScreen.tsx` owns view mode state.
- `LibraryScreen.tsx` owns filters, search, sort, page, page size, selected file, selected IDs, active folder, and active detail sheet.
- `LibraryTopStrip.tsx` renders controls and sends changes upward.
- `LibraryDetailsPanel.tsx` receives selected file/folder summary as props.
- `LibraryDetailSheet.tsx` receives selected file and prebuilt section lists as props.
- `libraryDisplay.tsx` transforms backend rows into display models.

**Broken or risky**

- `LibraryScreen.tsx` is doing too much. It is a data loader, filter controller, page controller, folder controller, detail controller, and action dispatcher all at once.
- Relationship helpers are called in row/card/detail contexts, sometimes with only the current page rows.
- `LibraryCollectionTable` and `LibraryThumbnailGrid` call `useMemo` inside an inline function/IIFE inside JSX. The current build passes, but this is a React rules risk and would be better as top-level hook use.
- Folder tree expanded state exists in `LibraryScreen.tsx`, but `FolderTreePane` uses local state, so the parent state is not actually preserving expansion across rebuilds.

## 7. Database / Schema Current State

**Implemented**

Source of truth appears to be `database/migrations/0001_initial.sql`.

Library-supporting tables:

- `files`
- `creators`
- `creator_aliases`
- `user_creator_aliases`
- `user_category_overrides`
- `bundles`
- `duplicates`
- `review_queue`
- `content_watch_sources`
- `content_watch_results`
- `scan_sessions`
- `download_items`
- `special_mod_family_state`
- `snapshots`
- `snapshot_items`
- `app_settings`

Important normalized `files` columns:

- `path`
- `filename`
- `extension`
- `hash`
- `size`
- `created_at`
- `modified_at`
- `bundle_id`
- `creator_id`
- `kind`
- `subtype`
- `confidence`
- `source_location`
- `download_item_id`
- `source_origin_path`
- `archive_member_path`
- `scan_session_id`
- `relative_depth`
- `indexed_at`

Important JSON/text fields:

- `files.safety_notes`
- `files.parser_warnings`
- `files.insights`
- `download_items.assessment_reasons`
- `download_items.dependency_summary`
- `download_items.missing_dependencies`
- `download_items.inbox_dependencies`
- `download_items.incompatibility_warnings`
- `download_items.post_install_notes`
- `download_items.evidence_summary`
- `download_items.notes`
- `content_watch_results.evidence`

`FileInsights` currently stores:

- `thumbnailPreview`
- `cachedThumbnailPreview`
- `format`
- `resourceSummary`
- `scriptNamespaces`
- `embeddedNames`
- `creatorHints`
- `versionHints`
- `versionSignals`
- `familyHints`

**Indexes present**

- `files.hash`
- `files.filename`
- `files.creator_id`
- `files.bundle_id`
- `files.kind`
- `files.source_location`
- `files.download_item_id`
- `content_watch_sources.source_kind`
- `content_watch_sources.anchor_file_id`
- `content_watch_results.status`
- `download_items.status`
- `download_items.intake_mode`
- `download_item_events.download_item_id, created_at`
- user override/alias indexes

**Missing or risky indexes**

- No explicit `duplicates.file_id_a` or `duplicates.file_id_b` indexes, despite frequent `EXISTS` checks.
- No composite index for common Library filters like `(source_location, kind)`.
- No index on `files.subtype`.
- Search uses leading-wildcard `LIKE`, so normal indexes will not help much for large libraries.
- No full-text search table.

## 8. Scanner Current State

### What happens when the user clicks Scan / Run Scan / Scan My CC

**Implemented**

- The sidebar button calls `startScan()` in `src/App.tsx`.
- `startScan()` calls the Tauri command `start_scan`.
- `start_scan` starts a background thread and returns a running scan status.
- The frontend listens for scan progress/status events and also polls status while scanning.

### Scan stages

**Implemented**

1. Load settings and seed data.
2. Collect configured Mods and Tray roots.
3. Load cached file rows.
4. Decide full vs incremental scan using a cache fingerprint.
5. Walk files with `WalkDir`.
6. Identify supported file types.
7. Hash only likely duplicate candidates, not every file.
8. Clear old scan data for configured roots.
9. Reuse cached unchanged rows when possible.
10. Inspect changed/new files.
11. Classify filename, folder hints, package/script hints, creator hints, category overrides.
12. Insert rows into `files`.
13. Add review queue entries for low confidence, parser warnings, and safety notes.
14. Rebuild tray bundles.
15. Rebuild duplicates.
16. Save cache fingerprint and scan status.

### File types scanned

**Implemented**

Mods root:

- `.package`
- `.ts4script`
- `.trayitem`
- `.blueprint`
- `.bpi`
- `.householdbinary`
- `.hhi`
- `.sgi`
- `.room`
- `.rmi`

Tray root:

- `.trayitem`
- `.blueprint`
- `.bpi`
- `.householdbinary`
- `.hhi`
- `.sgi`
- `.room`
- `.rmi`

### Extracted metadata

**Implemented**

- Basic file facts: path, filename, extension, size, created/modified time.
- Source: Mods or Tray.
- Relative folder depth.
- Hash for duplicate-sized candidates or reusable cached hash.
- Kind, subtype, confidence.
- Creator and creator hints.
- Package/script resource summary.
- Script namespaces.
- Embedded package names.
- Version hints and version signals.
- Family hints.
- Safety notes and parser warnings.

### Thumbnails in scan

**Implemented**

- Thumbnail parsing is deferred during scan through `THUMBNAIL_DEFERRED`.
- This is good because thumbnail decoding during full scans could be very slow.

### Duplicate/update/problem signals

**Implemented**

- Duplicate pairs are rebuilt after scan.
- Review queue is rebuilt after scan.
- Version hints/signals are extracted and later used by update-watch logic.

**Partially implemented**

- Exact duplicate detection only hashes selected candidate files. The strategy is reasonable for exact duplicates because exact duplicates must share size, but it means hashes are not universal metadata for every file.
- Problem detection is mostly "parse uncertainty" and "placement safety". It is not true broken CC detection.

### Freeze/stall risks

**Broken or risky**

- There is no scan cancellation.
- There is no explicit full-rescan button or command. The scanner chooses incremental vs full from cache state.
- Most parser errors are caught, but there is no hard per-file watchdog that isolates a package in a separate process. A truly stuck file read/decompression could still stall scanning.
- Hash progress can emit once per hash candidate. In a library with many same-size files this could be chatty.
- Cache invalidation is versioned by `scanner-v20`, seed version, creator learning version, and category override version. That is good, but it still does not solve bad-file isolation.

## 9. Thumbnails / Media Current State

**Implemented**

- Embedded THUM extraction from package files exists.
- Object/build-buy THUM resource type is recognized.
- Sims 4 `localthumbcache.package` parsing exists.
- DDS thumbnail decoding to PNG base64 exists.
- Thumbnail extraction is first-party now.
- Scan defers thumbnails.
- Detail loading can fill missing embedded/cache thumbnails.
- Grid/list images use `loading="lazy"` and `decoding="async"`.
- Fallback previews exist.

**Partially implemented**

- Thumbnails are stored in `files.insights` as base64 strings when present.
- List view requests lightweight rows by setting `includePreviews: false`.
- Grid view requests rows with previews by setting `includePreviews: true`.
- Detail requests can decode thumbnail data on demand.

**Broken or risky**

- The default backend behavior is `include_previews.unwrap_or(true)`, so any caller that forgets to pass `includePreviews: false` gets heavy preview data.
- There is no separate thumbnail endpoint or file cache path. Grid rows carry base64 payloads.
- Thumbnail base64 in SQLite JSON can make rows and IPC payloads large.
- Detail selection can trigger package/cache parsing work.
- The frontend checks `selectedFile.insights.previewSource`, but the Rust `FileInsights` struct does not define `preview_source`. Preview source is mostly frontend-derived for row/card models, not a durable backend field.
- Some comments still refer to Mod Manager even though the current code path is first-party. That can confuse future work.

**Not implemented**

- Real extracted color swatches.
- A production media cache with file paths, dimensions, timestamps, and invalidation.
- Lazy per-card thumbnail loading independent of Library rows.

## 10. Folder View Current State

**Implemented**

- Folder view has a tree pane and content pane.
- Tree has Mods and Tray roots.
- Frontend `buildFolderTree` can build a tree from file rows.
- Root-level loose files are separated into Mods and Tray loose-file groups.
- "Loose files" currently means files sitting directly in a folder.
- Subfolder files are files inside folders under the selected folder.
- Root loose-file lists use `VirtualizedLooseFiles` when large enough.
- Folder summary panel exists.
- Open folder exists for selected folder summaries when an absolute path is available.

**Partially implemented**

- The backend has a `get_folder_tree_metadata` command meant to build tree metadata without sending all files.
- The frontend warms folder tree state by calling `getFolderTreeMetadata` and `listLibraryFilesForTree`.
- The frontend also falls back to building the usable tree from full file rows because it needs file IDs for contents.

**Broken or risky**

- `get_folder_tree_metadata` likely fails because it uses `REVERSE()`.
- `list_library_files_for_tree` calls `library_index::list_library_files`, so it inherits the invalid SQL problem.
- `getFolderContents` only searches root nodes and direct children. It does not recurse through deeper descendants. Selecting a nested folder more than one level deep can return empty contents.
- Folder tree counts can be wrong or unavailable if the metadata path is used because the metadata tree has no `files` arrays.
- Folder expanded state is local to `FolderTreePane`, not actually persisted by `LibraryScreen`.
- Folder summary open-folder can fail for empty folders because no file path exists to derive a real absolute folder path.
- `FolderTreePane` computes clues with an `allFiles` value that appears to come from root files only, so subfolder clue badges can be misleading.
- Large subfolder contents are not virtualized. They show a first slice, then "Show all" can render the full table.
- The tree is not truly backend-native yet in the useful path.

**Plain-language label recommendation**

"Loose files" is accurate but not normal-player friendly. Use:

- "Files directly in this folder"
- "Direct files"
- "Files inside subfolders"

For casual mode, prefer "Direct files" and a tooltip: "These files sit right inside this folder, not inside another folder."

## 11. List/Grid View Current State

### List view

**Implemented**

- Rows show thumbnail/fallback, filename, type, identity, relationship cue, watch status, health/status labels, duplicate cue, confidence cue, supporting facts, and swatches where available.
- Pagination is implemented.
- Page-size selector supports 50, 100, 250, and 500.
- List view passes `includePreviews: false`, so it asks for lighter rows.
- Selecting a row immediately shows a preview detail from row data, then loads full detail.
- The table component is memoized.

**Partially implemented**

- Rows are paginated, not fully virtualized.
- Tray pack grouping is frontend-only. That means backend total/page counts may not exactly match visible rows if grouped rows are hidden.

**Broken or risky**

- The main backend list query is invalid SQL right now.
- Selecting a row triggers `get_file_detail`, which can do thumbnail work.
- Relationship cues can overclaim because backend peer counts are flawed.
- The list is dense and useful, but casual mode still exposes a lot of detail.

### Grid view

**Implemented**

- Cards show a dominant thumbnail or fallback.
- Source badges/dots are shown when a preview source exists.
- Type is visible at rest.
- More file info appears on hover/focus/reveal.
- Density/card-size slider exists.
- Grid requests previews.
- Grid images use lazy loading.
- Grid is paginated.

**Partially implemented**

- Swatches are name-derived color hints only.
- Fallback icons are manually drawn SVGs, not Lucide icons.
- Cards are not virtualized.

**Broken or risky**

- Grid payloads can be large because preview base64 comes with paged rows.
- Card source badge labels are cryptic. For example, embedded preview badge can read like "M" even though embedded THUM is not Mod Manager.
- Relationship cues can be inaccurate.
- There is no separate media loading pipeline for fast wardrobe-style browsing.

## 12. Inspector + Detail Sheet Current State

### Inspector sidebar

**Implemented**

- No file selected: shows "Select a file", or a folder summary when a folder is selected.
- File selected: shows header, preview/fallback, quick facts, care/health summary, duplicate info, safe-delete warning, and actions.
- Folder selected: shows folder summary, type distribution, creator summary, relationship clusters, and Open folder.
- Preview works when thumbnail data is present.
- Long values appear to have wrapping CSS, but this still needs real desktop verification.
- Open folder action exists in the sidebar for Creator/power view and folder summary.

**Partially implemented**

- Open folder for files is hidden from Casual/Seasoned and only visible in Creator/power mode.
- Safe-delete warning is a heuristic, not a real preflight.
- Folder summary relationship clusters can inherit bad relationship counts.

**Needs verification in real Tauri desktop app**

- Long filenames.
- Long creator names.
- Long folder paths.
- Relationship text wrapping.
- Small-width layout.
- Actual Explorer opening for file and folder paths.

### More Details / detail sheet

**Implemented**

- Detail sheet has modes for health, inspect, and edit-style detail.
- It avoids repeating the large thumbnail and uses an evidence board instead.
- Sections include file facts, contents/tray context, attribution, compatibility, relationships, path, warnings, and editing controls depending on mode and user view.
- Full path appears in the sheet footer/path area.
- Swatches can appear in the evidence board for CAS/power contexts.

**Partially implemented**

- More Details is different enough from the sidebar in intent, but some facts still overlap.
- Relationships are shown, but not backed by a real dependency graph.
- Swatches are guessed from names/hints.

**Not implemented**

- Open folder action inside the detail sheet.
- Real dependency reasoning.
- Recolor-to-mesh proof.
- Missing mesh proof.
- Update source matching/editing directly in the detail sheet.
- Safe action preflight.

## 13. Relationships / Dependencies Current State

**Implemented**

Current relationship signals:

- Exact duplicate
- Version duplicate
- Same pack/bundle
- Tray bundle
- Same creator
- Same folder
- Folder heuristic

Visible in:

- List view: relationship cue
- Grid view: reveal relationship cue
- Folder view: folder summary/clusters
- Inspector: at-a-glance relationship row
- More Details: relationships section

Proof levels exist in frontend language:

- Confirmed/fact
- Likely/claim
- Possible/heuristic

**Partially implemented**

- Duplicate relationships use real duplicate rows.
- Tray grouping uses bundle/group counts.
- Same creator and same folder can be computed from current visible rows.
- Backend same-folder/same-pack counts are intended to avoid full frontend scans, but the current SQL logic is wrong.

**Broken or risky**

- Same-folder backend count is not actual same folder. It is same source plus same depth.
- Same-pack backend count can overcount null bundle groups.
- Same creator and same folder helpers often only see the current page, not the full Library.
- More Details can say "confirmed relationships" for same-folder signals that are not actually proven in the backend.

**Not implemented**

- Recolor to mesh linking.
- Missing mesh detection.
- Package dependency detection.
- Safe-delete protection.
- Full dependency graph.
- Dependency proof from actual package resources.
- "What breaks if I remove this?" preflight.

## 14. Duplicates / Updates / Needs Review Current State

### Duplicates

**Implemented**

- Exact duplicate detection: SHA-256 hash matches.
- Filename duplicate detection: same filename, different hash.
- Version duplicate detection: filename version-token stripping.
- Duplicates table stores pair rows.
- Duplicates page shows pair list, overview counts, detection method, file paths, creator, size, modified date, and hashes in power mode.
- Library can filter to duplicates and shows duplicate cues.

**Partially implemented**

- Exact hash detection depends on the scanner hashing duplicate-size candidates.
- Detail duplicate count is the number of duplicate types, not the number of duplicate pairs. The UI wording can be misleading.

**Not implemented**

- Cleanup actions.
- Keep/newer recommendation.
- Safe removal workflow.
- Duplicate table indexes for file lookup.

### Updates

**Implemented**

- Watch sources can be saved per subject.
- Watch results are stored.
- Updates page has tracked/setup/review lanes.
- Built-in/special sources can be checked when supported.
- Generic GitHub release pages can be checked.
- Creator pages are saved as reminders for now.
- CurseForge pages are recognized as provider-required, not fully supported.
- The watch poller exists and can refresh automatic sources when enabled.

**Partially implemented**

- Creator/version extraction is real but heuristic.
- Library surfaces watch status through quick filters and badges.
- Detail loads installed version summary and watch result.
- Watch setup suggestions exist, but not full automatic source matching.

**Not implemented**

- Full CurseForge integration.
- General creator-page scraping.
- Automatic matching to arbitrary update pages.
- Safe update install pipeline for all mods.

### Needs Review / Problems

**Implemented**

- Review queue receives:
  - low confidence parse
  - conflicting creator signals
  - other filename/parser warnings
  - unsafe script depth
  - tray content in Mods
- Needs Review page is read-only and explains why files stopped.
- Library can filter to "Needs review".

**Partially implemented**

- Corrupt package handling is mostly parser-error tolerance. If `inspect_file` errors, scanner logs and continues with default inspection.

**Not implemented**

- Broken CC detection.
- Texture clash detection.
- Missing dependency warnings for installed Library files.
- Missing mesh detection.
- Full problem list with severity and repair action.

## 15. Performance Findings

| Priority | Finding | Where | Why it matters | Fix direction | Side | Risk |
|---|---|---|---|---|---|---|
| Critical | Main Library SQL is invalid | `src-tauri/src/core/library_index/mod.rs` | List/grid/folder row loading can fail in real Tauri. | Remove the extra comma and add a regression test that prepares/runs list queries. | Backend | High |
| Critical | Folder metadata SQL likely fails | `src-tauri/src/commands/mod.rs` | `REVERSE()` is not a built-in SQLite function. Warm folder tree path can fail. | Rewrite path handling without unsupported SQL, or register a scalar function. | Backend | High |
| Critical | Rust tests do not compile | `src-tauri/src/core/file_inspector/mod.rs` tests | Blocks test suite from catching runtime regressions. | Update tests to pass the new `defer_thumbnails` argument. | Backend | Medium |
| High | Folder view fetches all rows for tree contents | `LibraryScreen.tsx`, `folderTree.ts` | A 100GB-300GB library may mean huge IPC payloads and frontend memory churn. | Build backend-native folder contents and direct child queries. | Both | High |
| High | Relationship peer counts can massively overcount | `library_index/mod.rs` | UI can tell users files are related when they are not. | Store/compute actual parent folder path and guard null bundle partitions. | Backend | High |
| High | Grid rows carry base64 previews | `list_library_files`, `LibraryThumbnailGrid` | Base64 JSON over IPC gets heavy fast. | Add thumbnail media table/cache and lazy `get_thumbnail(file_id)` style endpoint. | Both | Medium |
| High | Selecting a row can parse thumbnails | `get_file_detail` | Rapid clicking can do repeated package/cache work. | Cache resolved thumbnails and avoid decode on every selection. | Backend | Medium |
| High | Staging page calls unregistered commands | `StagingScreen.tsx`, `api.ts`, `lib.rs` | User can navigate to a page that cannot work in Tauri. | Register commands or remove/merge Staging. | Backend/UI | Medium |
| Medium | Search uses `%LIKE%` | `build_filters` | Slow on big libraries. | Add FTS/search index or token table. | Backend | Medium |
| Medium | Duplicates table lacks lookup indexes | migration | `EXISTS` duplicate checks can slow as pairs grow. | Add indexes on `file_id_a`, `file_id_b`, maybe duplicate type. | Backend | Low |
| Medium | Folder lookup is not recursive | `folderTree.ts` | Deep folder selection can show empty results. | Recursive map by folder path or flat map from path to node. | Frontend | High |
| Medium | Large subfolders are not virtualized | `FolderContentPane.tsx` | "Show all" can render too many rows. | Virtualize all folder content lists, not just root loose groups. | Frontend | Medium |
| Medium | Virtualized loose files still builds all models | `VirtualizedLooseFiles.tsx` | Virtualization saves DOM but still spends CPU on every file. | Memoize model cache and build only visible models where possible. | Frontend | Low |
| Medium | Relationships only see current page in places | `LibraryScreen.tsx`, `libraryDisplay.tsx` | Detail/sidebar cues can change depending on page/filter. | Move relationship summaries into backend detail response. | Both | Medium |
| Medium | Repeated facet loads | `LibraryScreen.tsx` | Extra IPC calls around kind/refresh changes. | Consolidate facet loading and cache by filter. | Frontend | Low |
| Low | Stale comments and schema docs | file inspector/schema docs | Future work can follow wrong assumptions. | Clean docs/comments after functional fixes. | Docs | Low |
| Low | Vite chunk-size warning | frontend build | Not blocking, but first app load can grow. | Further route/component splitting or manual chunks. | Frontend | Low |

## 16. UI/UX Findings

**Feels production-grade now**

- The app has a real desktop workbench shape.
- Library has three useful browsing modes.
- Page size and grid density controls are practical.
- The sidebar/detail split is a good product pattern.
- The scanner overlay and scan status flow are clear.
- Duplicates, Updates, Review, and Organize have distinct enough foundations.
- Casual/Seasoned/Creator mode language exists and changes the experience.

**Still unfinished**

- Relationship wording is ahead of the proof system.
- Folder view cannot yet be trusted for huge libraries or deep folders.
- Open folder is not consistently available where users expect it.
- More Details does not yet own the deep safety/dependency story.
- Staging is exposed but not wired in Tauri.
- Updates can look more automatic than they really are.

**Too cluttered**

- The top strip has search, sort, page size, view mode, density, advanced filter, kind chips, status chips, subtype chips, and summary pills.
- Sidebar sometimes repeats facts that should live only in More Details.
- Creator and Types pages may be too much top-level navigation for casual users.

**Too hidden**

- Open folder is hidden from casual/seasoned file inspector.
- Update source setup is mostly in Updates, not near the Library item.
- Folder meaning for loose/direct files needs clearer language.

**Confusing for casual users**

- "Loose files" may sound like broken files. Use "Direct files" or "Files directly in this folder".
- Thumbnail source badges like `M`, `C`, `EM`, `CH` are too cryptic.
- "Not tracked" can sound like a problem, when it only means no update page has been saved.
- Relationship/dependency labels can imply proof that does not exist yet.

**Missing for power users**

- Real dependency graph.
- Safe-delete preflight.
- Bulk Library actions with undo.
- Saved filters/searches.
- Fast folder path search.
- Per-folder stats that are backend-native.
- Media cache controls.
- Full path/open-folder everywhere in details.

## 17. AI Readiness

**Not implemented**

- No model runner abstraction.
- No local inference service.
- No task queue for AI work.
- No prompt/schema system.
- No structured output validation layer.
- No privacy/safety boundary for local model tasks.

**Existing placeholder**

- `src-tauri/src/core/ai_classifier/mod.rs` says `STATUS: "planned"`.

**Realistic future AI features**

- Smart tag suggestions.
- Creator name cleanup suggestions.
- Folder organization suggestions.
- Plain-English explanations of review/problems.
- Grouping suggestions.
- "What should I fix first?" helper.
- Update-match explanation helper.

**Should wait**

- AI should not replace deterministic package parsing, duplicate hashing, folder truth, dependency detection, or safe-delete rules.
- Build the deterministic relationship/dependency foundation first, then let AI explain or suggest.

## 18. Product Direction Options

### Option A - Trust & Safety First

Benefits:

- Makes the app trustworthy before it gets more powerful.
- Supports safe delete, missing mesh, dependency warnings, and Needs Review.
- Reduces the risk of users breaking a large Mods folder.

Risks:

- Harder engineering work.
- Requires careful proof levels and conservative language.
- Some Sims 4 dependency signals may be incomplete or fuzzy.

Required foundation:

- Correct actual folder path tracking.
- Real relationship/dependency data model.
- Backend detail endpoint that returns relationship evidence.
- Problem severity model.
- Safe action preflight.

Recommended order:

1. Fix Library runtime SQL and tests.
2. Fix folder truth and relationship count truth.
3. Build dependency/safe-delete preflight.
4. Route real warnings into Needs Review and detail sheet.

### Option B - Visual Library First

Benefits:

- Makes Library feel delightful and useful for CC browsing.
- Helps normal Simmers recognize items faster.
- Best path for wardrobe-style CAS/grid browsing.

Risks:

- Media payloads can get huge if base64 remains in row data.
- Visual polish can hide weak safety logic.
- Localthumbcache availability varies by machine.

Required foundation:

- Thumbnail media cache table.
- Lazy thumbnail endpoint.
- Clear preview source labels.
- Real swatches or no swatch claims.
- Backend-native folder browsing.

Recommended order:

1. Fix Library SQL.
2. Build thumbnail cache/lazy media loading.
3. Improve grid/folder browsing.
4. Add visual filters and real swatches later.

### Option C - Automation First

Benefits:

- Moves the app toward actual mod management, not just browsing.
- Organize already has snapshots, so there is a base.
- Helps users act on Inbox, Updates, Review, and Duplicates.

Risks:

- Dangerous without dependency/safe-delete preflight.
- Update automation can be misleading without reliable source matching.
- Needs excellent undo/restore and logging.

Required foundation:

- Trustworthy scan/index.
- Safe action preflight.
- Snapshot/restore coverage for all file operations.
- Dependency and duplicate proof levels.
- Update source confidence model.

Recommended order:

1. Stabilize Library and tests.
2. Expand safe action preflight.
3. Connect Organize/Review/Inbox with clear undo.
4. Add update automation only for high-confidence sources.

## 19. Recommended Next 3 Sprints

### Sprint 1 - Make Library Truthful And Runnable

Goal: the Library must load reliably and stop overclaiming.

- Fix `list_library_files` SQL.
- Fix or disable `get_folder_tree_metadata` until it is correct.
- Fix Rust tests that still call the old `inspect_file` signature.
- Add tests for list/grid/tree queries.
- Correct same-folder and same-pack peer counts.
- Rename misleading `disabled` summary count or remove it from Library.
- Decide whether Staging should be registered, hidden, or merged.

### Sprint 2 - Backend-Native Folder And Detail Foundation

Goal: folder view should feel like real Mods/Tray browsing without huge frontend payloads.

- Store or compute normalized parent folder paths.
- Add backend folder children query.
- Add backend folder contents query with pagination.
- Add backend folder summary query.
- Make Open folder work for empty folders.
- Make nested folder selection recursive and tested.
- Move relationship evidence into backend detail response.

### Sprint 3 - Trust/Safety Relationship Foundation

Goal: turn relationship labels into evidence users can trust.

- Add a relationship/evidence data model.
- Separate "confirmed", "likely", and "possible" in backend data, not just frontend text.
- Add safe-delete preflight endpoint.
- Start with duplicates, same package/bundle, script mod caution, and tray group protection.
- Route preflight warnings into More Details and Needs Review.
- Leave recolor/mesh detection for a later focused sprint unless package evidence is clearly available.

## 20. Open Questions

- Should Staging remain a top-level page, or should it be merged into Inbox?
- Should Creators and Types remain top-level pages, or move into Library filters/editing plus Review shortcuts?
- Should casual mode hide advanced filters by default?
- Should Open folder be available in all modes?
- What should the app call "loose files" in casual mode?
- Should the next push be safety-first, visual-first, or automation-first?
- How much thumbnail storage is acceptable locally?
- Should SimSuite require a fresh scan after upgrading scanner/cache versions?
- What proof level is acceptable before the UI says "dependency"?

## 21. Verification Results

Commands run from `C:\Users\likwi\OneDrive\Desktop\PROJS\SimSort` unless noted.

| Command | Result | Notes |
|---|---|---|
| `git status --short --branch` | Passed | Worktree was dirty before audit. Branch `main` is ahead of `origin/main` by 83 commits. |
| `git log --oneline -30` | Passed | Latest commits are recent Library relationship/folder/open-folder work. |
| `Get-Content package.json` | Passed | Scripts include `build`, `test:unit`, `test:rust`, `tauri:build`. |
| `npm run build` | Passed | Runs `tsc && vite build`. Vite warns that `assets/index-*.js` is slightly over 500 kB. |
| `npx tsc --noEmit` | Passed | No TypeScript errors. |
| `npm run test:unit` | Passed | Vitest: 9 files, 35 tests passed. |
| `cargo check` in `src-tauri` | Passed with warnings | Warnings include unused/dead staging commands and other cleanup warnings. |
| `cargo build --release` in `src-tauri` | Passed with warnings | Release build succeeds. |
| `cargo test` in `src-tauri` | Failed | Test build fails before running tests. Eight calls to `inspect_file` in `src-tauri/src/core/file_inspector/mod.rs` tests pass 3 args, but the function now requires 4 args including `defer_thumbnails: bool`. |

Exact `cargo test` failure type:

```text
error[E0061]: this function takes 4 arguments but 3 arguments were supplied
```

Affected test call lines reported by Rust:

- `src/core/file_inspector/mod.rs:2536`
- `src/core/file_inspector/mod.rs:2565`
- `src/core/file_inspector/mod.rs:2597`
- `src/core/file_inspector/mod.rs:2630`
- `src/core/file_inspector/mod.rs:2662`
- `src/core/file_inspector/mod.rs:2696`
- `src/core/file_inspector/mod.rs:2724`
- `src/core/file_inspector/mod.rs:2758`

Other verification notes:

- `rg` could not be used in this environment because the bundled `rg.exe` returned "Access is denied". I used `git grep`, PowerShell, and direct file reads instead.
- I did not run the real Tauri desktop app or take screenshots in this pass. UI wrapping, Explorer opening, and real folder browsing still need desktop verification after the critical backend fixes.

## 22. Final Verdict

The SimSuite Library is much further than a mockup. It has a real scanner, real SQLite storage, real frontend views, real duplicate detection, real update-watch plumbing, real package inspection, and real first-party thumbnail work.

It is not production-ready.

Before broader release, the next sprint should be Trust & Safety First, starting with the boring but important stuff: make the Library queries run, make tests run, make folder truth correct, and stop relationship/dependency labels from overpromising. After that, the app can safely choose whether to lean visual browsing, automation, or deeper dependency protection.

Clear recommendation: Sprint 1 should fix Library runtime correctness and relationship truth before adding new user-facing features.
