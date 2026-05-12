# Library Backend Performance and Folder Query v1 Report

Date: 2026-05-12

Branch: `codex/library-backend-performance-folder-query-v1`

## Worktree State

- Started from `codex/library-duplicate-truth-guardrails-v21`.
- Pre-existing unrelated dirty files were present before this sprint:
  - `.cocoindex_code/cocoindex.db/mdb/data.mdb`
  - `.cocoindex_code/target_sqlite.db`
  - `SESSION_HANDOFF.md`
  - `docs/IMPLEMENTATION_STATUS.md`
  - `src/screens/HomeScreen.tsx`
  - `src/styles/globals.css`
- The `.cocoindex_code`, Home, and global CSS changes were not part of this sprint.
- `SESSION_HANDOFF.md` and `docs/IMPLEMENTATION_STATUS.md` were touched only for this sprint notes; unrelated pre-existing hunks must stay out of the Library commit.

## 1. Current Query Map

| Path | State before implementation | Classification |
| --- | --- | --- |
| Paged Library list | `list_library_files` builds SQL filters, total count, order, limit/offset, and optional preview compaction. | already SQL-direct and paged |
| Library search/filter/sort | `build_filters` and `build_order_by` are SQL-backed for normal Library list queries. | SQL-direct but still needs broader stress proof |
| Folder tree metadata | `get_folder_tree_metadata` selects path/source/depth rows and builds tree metadata in Rust from filtered rows. It does not transfer full file rows. | partially broad but lightweight; future work for huge trees |
| Folder content listing | `list_library_folder_files` removed limit/offset, called broad `list_library_files`, filtered virtual folder membership in Rust, then paginated in memory. | broad list then in-memory filter |
| Direct folder files | Correct behavior existed, but it was derived after broad loading. | broad list then in-memory filter |
| Recursive selected folder files | Correct behavior existed, but it was derived after broad loading. | broad list then in-memory filter |
| File detail | `get_file_detail` is row-specific and lazy for previews/details. | already SQL-direct and lazy |
| Duplicate counts | Duplicate guardrails v2.1 revalidate exact-file proof through SQL joins. | SQL-direct and bounded to duplicate rows |
| Relationship counts | `load_relationship_peer_counts` loads rows in the current filtered set, then groups folders/bundles in Rust. Not N+1, but broad for large filtered sets. | broad filtered-set aggregation |
| Preview inclusion/exclusion | List/folder rows still read insights JSON, but `compact_library_row_insights` removes preview payloads when previews are not requested. | partially paged/lazy |
| `list_library_files_for_tree` | Tauri command stripped pagination and called `list_library_files` unbounded. Current frontend tests assert the Library no longer calls it. | unbounded compatibility risk |

## 2. Current Index Map

| Query need | Index state before implementation | Classification |
| --- | --- | --- |
| Source location | `idx_files_source_location`, `idx_files_source_location_kind`, `idx_files_source_location_filename` | sufficient |
| Relative depth | `idx_files_relative_depth` | probably sufficient, but source+depth composite is better for folder queries |
| Source + relative depth | no dedicated composite | missing |
| Filename sort/search | `idx_files_filename`, `idx_files_source_location_filename`; LIKE search remains broad | probably sufficient for sort, search still broad |
| Kind/source filters | `idx_files_kind`, `idx_files_source_location_kind` | sufficient |
| Creator filter | `idx_files_creator_id`; canonical name filter joins creators | probably sufficient |
| Hash duplicate exact proof | `idx_files_hash` plus duplicate row indexes | sufficient for current duplicate truth model |
| Duplicate row joins | `idx_duplicates_duplicate_type`, `idx_duplicates_file_id_a`, `idx_duplicates_file_id_b` | sufficient |
| Folder path prefix | no normalized relative path column/index | future work |
| Search/filter/sort combinations | partial coverage only | future stress/index work |

## 3. Current Stress Coverage

| Case | Coverage before implementation | Classification |
| --- | --- | --- |
| 1,000 files | No committed backend stress test found. | missing |
| 5,000 files | No committed backend stress test found. | missing |
| 10,000 files | No committed backend stress test found. | future work |
| Deeply nested folders | Small unit coverage existed. | partially covered |
| Large single folder | No large deterministic folder test found. | missing |
| Empty folder | UI can show represented empty folders, but scanner metadata is file-row based. | future work |
| Direct files at root | Small folder metadata coverage existed. | partially covered |
| Mixed Mods/Tray roots | Small folder metadata coverage existed. | partially covered |
| Long paths | Not meaningfully stress-tested. | missing |
| Missing metadata | Covered in some Library/duplicate tests, not large stress. | partially covered |
| Duplicate-heavy same-name groups | Duplicate tests cover behavior, not large stress. | partially covered |
| Folder selection under filters | Small unit coverage existed. | partially covered |
| Search/filter/sort under load | No committed backend stress timing found. | missing |

## Folder Query Changes

- `list_library_folder_files` now builds a SQL scope for the selected Library folder instead of broad-loading Library rows and filtering in Rust.
- The folder scope uses:
  - `source_location = ?`
  - direct folder depth with `relative_depth = ?`
  - recursive folder depth with `relative_depth >= ?`
  - a parameterized normalized path `LIKE` pattern rooted at saved Mods/Tray settings when available.
- Existing Library search/filter/sort behavior is reused through a new internal `list_library_files_scoped` helper.
- Folder query result rows are still paged with limit/offset.
- Folder preview payloads remain controlled by `include_previews`.
- Folder query limits are bounded:
  - default folder page limit: 500
  - maximum folder page limit: 1,000
- Mismatched source filters are handled safely by returning an empty folder response.

## `list_library_files_for_tree` Decision

- The command is retained for compatibility because the Tauri command is still registered and the TypeScript API wrapper still exists.
- It is now bounded to a maximum of 5,000 returned rows.
- It forces `include_previews = false`.
- The response still carries the real total count from `list_library_files`, but the returned row slice cannot be unbounded.
- Preferred folder browsing remains `get_folder_tree_metadata` plus `list_library_folder_files`.

## Fixture and Stress Tests Added

Added deterministic Rust tests in `src-tauri/src/core/library_index/mod.rs`:

- Root-scoped folder SQL does not match same folder tails outside the configured root.
- Direct selected folder query returns only requested page.
- Recursive selected folder query includes child folder files and caps returned rows at 1,000.
- Search works inside folder query.
- Sort works inside folder query.
- `include_previews = false` removes thumbnail preview payloads.
- `include_previews = true` returns intended preview fields.
- Mods root direct files work.
- Tray root direct files work.
- Source filter mismatch returns empty.

Added command test in `src-tauri/src/commands/mod.rs`:

- Legacy tree-file query is capped at 5,000 rows, normalizes negative offset to 0, and strips previews.

The synthetic folder stress test inserts 1,076 folder rows: 1,001 direct files plus 75 nested child files.

## Performance Measurements

Focused Rust timing from:

`cargo test library_index::tests::folder_file_listing -- --nocapture`

Measured line:

`library_folder_stress rows=1076 elapsed_ms=212`

Notes:

- This is an informational timing from a local in-memory SQLite test, not a hard production threshold.
- The test verifies that only the requested page is returned for direct folder browsing.
- The recursive query verifies the configured 1,000-row cap when a caller asks for 2,000 rows.
- No 5,000 or 10,000 row hard benchmark was added in this sprint; those remain future stress work.

## Relationship Count Findings

- Relationship peer counts are not per-row N+1 queries.
- They still aggregate across the current filtered set in Rust.
- The new folder SQL scope narrows that filtered set for folder browsing, which reduces the folder-mode blast radius.
- Large all-library filtered sets still need future stress proof or a SQL aggregation/cache pass if they prove slow.
- Same folder and same pack remain related hints only. No dependency or safe-delete claim was added.

## Duplicate Group Performance Findings

- Duplicate truth rules were not changed.
- Current exact duplicate counts still passed the duplicate guardrail tests in the full Rust suite.
- Duplicate rebuild can still grow inside very large same-name/version-key groups; this remains a future duplicate stress topic.
- No package/script fingerprinting was added.

## Empty Folder Metadata Decision

True empty disk-folder metadata remains future work.

Reason:

- The current folder tree is derived from indexed file rows.
- Adding true empty disk folders cleanly needs scanner/indexer folder metadata, likely a folder table or equivalent real-directory metadata path.
- This sprint did not create fake file rows or virtual Explorer paths.

## Database / Migration Notes

- Added `idx_files_source_location_depth` for `files (source_location, relative_depth)`.
- The index is added in both:
  - `database/migrations/0001_initial.sql`
  - `src-tauri/src/database/mod.rs` schema repair/ensure path
- No destructive migration was added.
- Existing databases should receive the index through `CREATE INDEX IF NOT EXISTS` in schema initialization/repair.
- A normalized relative path column/index remains future work if path-prefix filtering needs further optimization.

## Validation

Passed:

- `cargo fmt`
- `cargo check`
- `cargo test`
- `cargo test library_index::tests::folder_file_listing -- --nocapture`
- `cargo test legacy_tree_file_query_is_bounded_and_preview_light -- --nocapture`
- `cargo build --release`
- `npm run build`
- `npx tsc --noEmit`
- `npm run test:unit`
- `npm run test:rust`
- `npm run desktop:proof:fixtures`
- `npm run desktop:smoke:fixtures`

Known validation notes:

- Rust builds still emit pre-existing unused-code/import warnings in older backend modules.
- Vite still emits the existing chunk-size warning.

## Desktop Proof

Desktop proof passed:

- `DESKTOP_LIBRARY_PROOF_OK`
- Summary: `output/desktop/library-proof/latest-summary.json`
- Screenshot folder: `output/desktop/library-proof/2026-05-12T15-59-53-326Z`
- Runtime errors: none reported in `runtimeErrors`.
- Browser logs contained Tauri callback warnings during proof reloads, but no severe runtime errors.

Desktop smoke passed:

- `Desktop smoke passed against ...\src-tauri\target\release\simsuite.exe`

## What Could Not Be Verified

- Real user 10,000+ file Library behavior was not tested.
- Real disk folder performance with very deep Windows paths was not tested.
- True empty disk folders are still not indexed.
- The folder path SQL still relies on saved Mods/Tray root settings for the strongest root-scoped prefix; fallback matching without saved roots is weaker and should be revisited with a normalized relative path field.
- Relationship count performance on very large all-library filtered result sets remains future work.
- Duplicate large-group rebuild stress remains future work.

## Next Backend Sprint Recommendation

Next: true empty disk-folder metadata, unless real-library thumbnail hydration is more urgent.

Recommended order:

1. True empty disk-folder metadata with a real folder metadata table/path.
2. Live real-library thumbnail validation and preview pipeline hardening.
3. Duplicate Truth Engine v3 scan-time package/script fingerprints.
4. Larger 5,000 to 10,000 row backend stress command/test harness.
5. Staging backend cleanup or Updates provider onboarding architecture.

## Commit

Pending commit.
