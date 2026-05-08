# Library Production Readiness v1 Report

Date: 2026-05-08

## Audit note before coding

### Worktree

- `git status --short` showed one pre-existing unstaged file: `src/styles/globals.css`.
- The `src/styles/globals.css` diff changes Home hero metric text color and line-height only.
- Classification: unrelated to this Library sprint unless later runtime proof shows it affects Library, which is unlikely.
- Action: leave it untouched and keep it out of the sprint commit.

### Already working

- Library route, list view, grid view, folder view, inspector panel, More Details sheet, Needs Review bridge, Updates bridge, Duplicates bridge, and Safe Action Preflight wiring exist in current code.
- Library list rows use paged `list_library_files` and only request previews in grid mode.
- Folder view uses `get_folder_tree_metadata` for lightweight folder metadata and `list_library_folder_files` for active-folder rows instead of prefetching every Library file.
- `get_file_detail` resolves full detail and thumbnails only after a file is selected.
- Backend folder metadata and folder-file listing have Rust tests for nested folders, direct folder contents, filters, and counts.
- Library wording already uses cautious terms such as `No update source`, `Possible duplicate`, `Same pack`, `Stored in Tray`, `Could not inspect fully`, and `Manual review needed`.

### Broken functionality

- `LibraryCollectionTable` calls `useMemo` inside a conditional render branch. Switching between empty and non-empty rows can break React hook ordering at runtime.
- `LibraryThumbnailGrid` has the same conditional-hook pattern and can break when grid results go from empty to non-empty.
- `VirtualizedLooseFiles` returns before calling `useVirtualizer` for small folders, then calls it for larger folders. Changing a folder from small to large can also break hook ordering.

### Layout/frontend issues

- Folder-root rendering does not fully respect the active source filter in the root folder panes; the root can still show both Mods and Tray shells even when the user filtered to one source.
- Folder view internal names and comments still use old “loose files” terminology. The visible UI mostly says “Direct files,” but the durable report should track the remaining naming debt.
- Grid and list row layout already have overflow safeguards, but the conditional-hook bugs can present as apparent route or view instability.

### Performance risks

- Library list/grid model building is already memoized in intent, but the memoization is placed inside conditional branches. Fixing the hook ordering should preserve the single-pass cache without adding extra row work.
- Folder view still filters active-folder rows client-side after the backend returns the active folder set. This is acceptable for v1 because the backend query is already scoped to one virtual folder, but it remains a future scaling point for very large folders.
- The old frontend `folderTree.ts` helper still exists with cache helpers and tests, but current `LibraryScreen` uses backend metadata for folder rendering. This is partial/legacy code, not the active hot path.

### Misleading wording

- The Library quick filter says `Has Updates` and the sort option says `Has updates first`. That wording is stronger than the trust-first update model because the backend only proves update leads, not definite replacement readiness.
- Library summary says `updates` for the `hasUpdates` count. Safer v1 wording is `update leads` or `possible updates`.
- No current Library wording inspected claims safe delete, safe replace, missing mesh, missing dependency, official source, or definitely outdated in the active Library files.

### Missing proof

- Current desktop proof script verifies Library selection, Safe Action Preflight detail, and Needs Review routing.
- It does not explicitly switch list/grid/folder modes, open/close More Details, verify Updates/Duplicates bridges, or capture folder view screenshots.
- The current sprint should run the existing desktop proof/smoke commands and, if practical, add or use focused runtime checks for the Library view modes.

### Future feature, not v1

- Real dependency detection, missing mesh detection, recolor-to-mesh linking, safe-delete proof, automatic update installation, CurseForge integration, and generic web scraping are outside this sprint.
- Truly empty folders on disk are only represented when folder metadata exists; backend scanner support for indexing every empty disk folder remains future work unless a focused blocker appears.

## Changes made

### Library stability and layout

- Fixed conditional React hook usage in `LibraryCollectionTable`, `LibraryThumbnailGrid`, and `VirtualizedLooseFiles`.
- Kept row/card model caches memoized at stable component hook positions so list, grid, and folder direct-file rendering can switch between empty and populated data without hook-order crashes.
- Split the large direct-files virtualized renderer into a child component so the virtualizer hook only exists on the large-list path where that component is mounted.
- Made folder root panes respect the active source filter:
  - Mods filter shows the Mods folder root and hides Tray.
  - Tray filter shows the Tray folder root and hides Mods.
  - switching source filters clears an incompatible selected folder instead of leaving the folder view pointed at the wrong root.
- Left the existing `VirtualizedLooseFiles` internal component name in place to keep the diff focused; visible user copy continues to use `Direct files`.

### Honest wording

- Changed the Library update quick filter from `Has Updates` to `Possible updates`.
- Changed the Library update sort label from `Has updates first` to `Update leads first`.
- Changed the Library summary label from `updates` to `update leads`.
- Did not add or keep Library-facing claims for safe delete, safe replace, missing mesh, missing dependency, official source, definitely outdated, or automatic replacement.

### Runtime proof coverage

- Extended `scripts/desktop/desktop-library-proof.mjs` so the standard proof now verifies more of the Library surface:
  - selects a fixture Library item in list view
  - switches to grid view and waits for grid cards
  - switches to folder view and waits for the folder layout
  - returns to list view
  - opens and closes the Library detail sheet
  - checks the Safe Action Preflight detail surface
  - opens the Needs Review route with the selected file context
- New proof screenshots are produced under `output/desktop/library-proof/<run>/`.

### Tests added or updated

- Added regression tests for list/grid/direct-file transitions that previously risked hook-order crashes.
- Added a top-strip wording test for the new update lead labels.
- Added a Library folder source-filter test that proves the Mods source filter asks for Mods root files and does not show the Tray root button.

## Verification

- `npm run test:unit -- src/screens/library/LibraryCollectionTable.test.tsx src/screens/library/LibraryThumbnailGrid.test.tsx src/screens/library/VirtualizedLooseFiles.test.tsx src/screens/library/LibraryTopStrip.test.tsx src/screens/LibraryScreen.test.tsx` passed after the new tests and fixes.
- `npx tsc --noEmit` passed.
- `npm run build` passed with the existing Vite chunk-size warning.
- `npm run test:unit` passed: 19 files, 59 tests.
- `npm run desktop:proof:fixtures` passed from native Windows PowerShell and reached `DESKTOP_LIBRARY_PROOF_OK`.
- `npm run desktop:smoke:fixtures` passed from native Windows PowerShell and reached `Desktop smoke passed`.
- Rust source was not changed, so separate `cargo check` was not required. The desktop proof and smoke lanes still built the Tauri release app and produced the existing Rust warnings.

### Desktop proof details

Latest proof summary:

- `output/desktop/library-proof/latest-summary.json`
- App path: `src-tauri/target/release/simsuite.exe`
- Screenshots captured:
  - `01-library-selected-mccc.png`
  - `02-library-grid-view.png`
  - `03-library-folder-view.png`
  - `04-library-detail-sheet.png`
  - `05-library-preflight-detail-mccc.png`
  - `06-review-route.png`

Verified by the proof lane:

- Library route loaded.
- List view loaded and selected the MCCC fixture row.
- Grid view loaded and rendered Library cards.
- Folder view loaded and rendered the folder work surface.
- More Details / Inspect file opened and closed.
- Safe Action Preflight detail surfaced cautious wording.
- Needs Review route opened with selected-file context.

Verified by the smoke lane:

- The packaged desktop app started from the npm wrapper.
- The fixture smoke proof completed against the release executable.

### Could not verify in this sprint

- WSL runtime was not run; this sprint verified native Windows PowerShell.
- The desktop proof did not click a real `Open folder` action.
- The desktop proof did not click the Updates or Duplicates bridge from Library.
- Browser console log inspection was not run separately.
- Truly empty disk folders still depend on scanner metadata. If the backend does not index an empty folder, the UI cannot show it from real data yet.

## Commit

Pending before commit.
