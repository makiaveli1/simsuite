# Library True Empty Folder Metadata v1 Report

Date: 2026-05-12

Branch: `codex/library-true-empty-folder-metadata-v1`

## Worktree State

- Started from `codex/library-backend-performance-folder-query-v1`.
- Pre-existing unrelated dirty files were present before this sprint:
  - `.cocoindex_code/cocoindex.db/mdb/data.mdb`
  - `.cocoindex_code/target_sqlite.db`
  - `SESSION_HANDOFF.md`
  - `docs/IMPLEMENTATION_STATUS.md`
  - `src/screens/HomeScreen.tsx`
  - `src/styles/globals.css`
- The generated `.cocoindex_code`, Home, and global CSS changes are outside this sprint and must stay unstaged.
- If `SESSION_HANDOFF.md` or `docs/IMPLEMENTATION_STATUS.md` are updated, only this sprint's notes should be staged.

## 1. Current Empty-Folder Behavior

- `get_folder_tree_metadata` currently builds the folder tree from rows in `files`.
- A folder appears when at least one indexed file row has a path under that folder.
- Truly empty disk folders generally cannot appear because they do not create file rows.
- The frontend can already show an empty selected folder state if a folder node is represented in metadata.
- The frontend can compute an Open Folder path from configured Mods/Tray settings for represented folder nodes.
- The backend does not currently store a real disk path for empty folder nodes.
- Mods and Tray roots appear when file rows or fallback frontend roots are present; empty nested folders are not reliably represented.

## 2. Scanner / Indexer Decision

Safest backend approach:

- Add scan-owned real folder metadata.
- Store folders in a separate table from `files`.
- Never create fake file rows for folders.
- Tie each folder row to `source_location`, a stable relative path, a normalized relative path, and the real disk path.
- Include Mods and Tray root folders, nested folders, and empty folders.
- Rebuild folder metadata for a scanned root during scan so deleted or renamed empty folders do not linger after rescan.
- Keep file indexing, file cache reuse, duplicate truth, and folder file listing behavior intact.

## 3. Database Decision

Plan:

- Add a `library_folders` table.
- Add indexes for:
  - source location
  - source + normalized relative path
  - source + parent normalized relative path
  - source + depth
- Update the initial schema and schema repair path.
- Avoid destructive changes and avoid fake file rows.
- Existing users will need a scan/rescan before older empty folders appear because the app must observe real disk folders first.

## 4. User Experience Impact Plan

Users will see empty folders in Folder view instead of wondering why those folders disappeared. Selecting an empty folder should show `0 files`, and Open Folder should use the real Windows folder path from disk, not a fake `Mods/Empty` Library path. This records folder metadata only; it does not move, delete, or change user files.

## What Was Audited

- Scanner/indexer: `src-tauri/src/core/scanner/mod.rs`.
- Folder query/tree backend: `src-tauri/src/core/library_index/mod.rs`.
- Database setup and initial schema: `src-tauri/src/database/mod.rs`, `database/migrations/0001_initial.sql`.
- Folder API model: `src-tauri/src/models.rs`, `src/lib/types.ts`.
- Folder view path/open boundary: `src/screens/LibraryScreen.tsx`, `src/screens/library/folderTree.ts`.
- Empty-folder frontend behavior test: `src/screens/LibraryScreen.test.tsx`.
- Desktop fixture/proof path: `scripts/desktop/run-tauri-webdriver.ps1`, `scripts/desktop/desktop-library-proof.mjs`.

## Current Empty-Folder Behavior

Before this sprint:

- Folder tree metadata came from indexed file rows.
- Folders with supported Sims files appeared.
- Truly empty disk folders usually disappeared because there was no file row to imply them.
- The UI could show an empty selected folder only when the folder was already represented by metadata.
- Open Folder could fall back to configured root paths, but the backend did not send real disk paths for empty folder nodes.

## What Changed

- Added real scan-owned folder metadata in a separate `library_folders` table.
- Scanner now records real Mods and Tray directories during normal scan, including root folders, nested folders, and folders with zero supported files.
- Folder metadata stores source location, relative path, normalized path, parent path, name, depth, full disk path, scan session, and indexed timestamp.
- Rescan clears and rewrites folder rows for the scanned source root, so removed or renamed empty folders do not linger.
- `get_folder_tree_metadata` now builds the tree from real folder rows and then overlays file counts from indexed files.
- Folder tree nodes now expose `diskPath` to the frontend.
- The Library folder view now prefers backend `diskPath` for Open Folder.
- The desktop fixture now creates a real empty Mods folder named `Empty Proof Folder`.
- Desktop proof now captures `library-empty-folder-metadata.png` after selecting the empty proof folder.

## What This Means For The User

Empty folders inside Mods can now show up in Folder view instead of disappearing. Selecting one should show that it has `0 files`, and Open Folder uses the real Windows folder path. This does not move, delete, rename, or create content files; SimSuite only records real folder metadata during scan.

Existing empty folders will appear after the user scans or rescans, because SimSuite has to walk the disk folder once to record it.

## Database / Migration Notes

- Added `library_folders` to the initial schema and runtime schema repair path.
- Added indexes for source, source+path, source+parent, and source+depth.
- No destructive migration was added.
- Existing databases receive the table through `ensure_schema`.
- The project currently has one initial migration file plus runtime repair/upgrades; no separate migration file was necessary for this additive table.

## Scanner / Indexer Behavior

- Directory walking now collects both files and folders.
- Supported Sims files still become rows in `files`.
- Folders become rows in `library_folders`.
- Empty folders never create fake file rows.
- Unsupported files do not become Library content just because their folder is recorded.
- Mods and Tray folders are separated by `source_location`.
- Deleted empty folders are removed from metadata after rescan.
- Inaccessible folder behavior follows the existing `WalkDir` error path: the scan records an error and continues where possible.

## Folder Tree Behavior

- Mods and Tray roots can be represented from `library_folders`.
- Nested empty folders can appear.
- Folders with files still appear and keep their direct/total file counts.
- Empty folders show direct and total file counts of `0`.
- Source filters are respected when loading folder rows.
- File counts are still based on indexed file rows, so folder metadata does not claim anything about unsupported files.

## Folder Listing Behavior

- `list_library_folder_files` remains the SQL-direct paged folder query from the previous sprint.
- Selecting an empty folder returns a normal empty page: `total = 0`, no items, no fake rows.
- Root Mods/Tray direct files and nested folder listings keep existing behavior.
- Preview payload controls were not changed.

## Open Folder Safety

- Empty folder nodes can carry a real `diskPath` from backend folder metadata.
- The frontend uses that real disk path before any configured-root fallback.
- The desktop proof does not click OS Explorer, but frontend unit coverage verifies that Open Folder calls the reveal API with the backend disk path for an empty folder.
- Virtual paths such as `Mods/Empty` are not used for represented empty folders that have real metadata.

## Performance Notes

- Folder metadata is collected during scan, not by repeatedly walking disk during ordinary folder browsing.
- Folder rows are inserted in the existing scan transaction.
- Folder tree loading uses indexed `library_folders` rows plus metadata-only file rows for counts.
- No previews or thumbnails are loaded for folder metadata.
- No broad frontend/UI redesign was added.
- Future large-library work should still measure folder tree count aggregation at 5,000 to 10,000 rows.

## Files Changed

- `database/migrations/0001_initial.sql`: added `library_folders` and indexes for fresh databases.
- `src-tauri/src/database/mod.rs`: added runtime schema repair for `library_folders`.
- `src-tauri/src/core/scanner/mod.rs`: collects, inserts, and refreshes real folder metadata.
- `src-tauri/src/core/library_index/mod.rs`: builds folder trees from folder metadata plus file counts; added empty folder tests.
- `src-tauri/src/models.rs`: added `disk_path` to folder tree nodes.
- `src/lib/types.ts`, `src/screens/library/folderTree.ts`: added frontend type support for `diskPath`.
- `src/screens/LibraryScreen.tsx`: Open Folder now prefers the backend disk path.
- `src/screens/LibraryScreen.test.tsx`: verifies empty folder Open Folder uses backend disk path.
- `scripts/desktop/run-tauri-webdriver.ps1`: adds a real empty fixture folder.
- `scripts/desktop/desktop-library-proof.mjs`: captures empty folder metadata proof.
- `docs/LIBRARY_BACKEND_SYSTEMS_MAP.md`: updated backend map for true empty folders.

## Tests

Passed so far:

- `cargo fmt`
- `cargo test true_empty -- --nocapture`: 2 passed
- `cargo test rescan_removes_deleted_empty_folder_metadata -- --nocapture`: 1 passed
- `cargo test empty_folder -- --nocapture`: 2 passed
- `npx vitest run src/screens/LibraryScreen.test.tsx`: 4 passed
- `cargo check`
- `npm run build`
- `npm run test:unit`: 22 files, 87 tests
- `npx tsc --noEmit`
- `cargo test`: 238 passed
- `cargo build --release`
- `npm run test:rust`: 238 passed
- `npm run desktop:proof:fixtures`: passed with `DESKTOP_LIBRARY_PROOF_OK`
- `npm run desktop:smoke:fixtures`: passed

## Desktop / Runtime Proof

Passed:

- `npm run desktop:proof:fixtures`
- `npm run desktop:smoke:fixtures`

Proof output:

- `C:\Users\likwi\OneDrive\Desktop\PROJS\SimSort\output\desktop\library-proof\2026-05-12T17-39-02-393Z`
- `library-empty-folder-metadata.png`

Proof summary:

- `ok: true`
- `emptyProofFolder: Empty Proof Folder`
- runtime errors: `0`

## What Could Not Be Verified Yet

- Live real-user Mods/Tray folders were not scanned.
- Real Windows Explorer opening was not clicked in proof, to avoid launching OS UI during automation.
- 5,000 to 10,000 row folder-tree performance remains future work.
- The proof verified one real empty Mods fixture folder, not a large real-world folder tree.

## Next Backend Sprint Recommendation

Recommended next backend sprint:

1. Live real-library thumbnail validation and preview pipeline hardening.
2. Larger 5,000 to 10,000 row backend stress command/test harness.
3. Duplicate Truth Engine v3 scan-time package/script fingerprints.
4. Relationship count SQL aggregation/cache pass.
5. Staging backend cleanup.

## Unrelated Worktree Changes

The following pre-existing unrelated files remain outside this sprint and should stay unstaged:

- `.cocoindex_code/cocoindex.db/mdb/data.mdb`
- `.cocoindex_code/target_sqlite.db`
- `src/screens/HomeScreen.tsx`
- unrelated global/Home hunks in `src/styles/globals.css`
- unrelated prior notes in `SESSION_HANDOFF.md`
- unrelated prior notes in `docs/IMPLEMENTATION_STATUS.md`

## Commit

- `46682c0` - Index true empty Library folders
- A docs-only follow-up commit records this sprint report after the implementation commit.
