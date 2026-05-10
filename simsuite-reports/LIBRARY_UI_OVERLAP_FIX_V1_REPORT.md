# Library UI Overlap Fix v1 Report

Date: 2026-05-10

## Audit note before coding

### Worktree

- `git status --short` showed pre-existing unstaged changes in `SESSION_HANDOFF.md`, `docs/IMPLEMENTATION_STATUS.md`, `src/screens/HomeScreen.tsx`, and `src/styles/globals.css`.
- The `src/styles/globals.css` diff is still the known Home hero metric styling tweak.
- The Home screen and session/status doc diffs are also unrelated Home UI polish notes from another pass.
- Classification: unrelated to this Library layout sprint.
- Action: preserve them and keep them out of this sprint commit. If Library CSS changes are needed in `src/styles/globals.css`, stage only the Library hunks.

### Screenshots reviewed

- User screenshot attached to this task.
- Latest successful desktop proof screenshots in `output/desktop/library-proof/2026-05-10T12-18-10-658Z`.
- Latest desktop proof summary: `output/desktop/library-proof/latest-summary.json`.

### Issue classification

- **Confirmed overlap**: the Library list/status/facts area can visually run into the inspector column at the tested desktop size. The inspector does not read as a hard-bounded sidebar.
- **Wrapping/clipping issue**: long row titles, status pills, and at-a-glance facts compete for horizontal space; status text can dominate the row.
- **Scrollbar/viewport issue**: the filter chips and table viewport create awkward scrollbars near the table boundary; the list body and footer do not read as separate regions.
- **Spacing/density issue**: rows are tall but still feel crowded because the status/facts columns are too compressed and badges are too blocky.
- **Hierarchy/readability issue**: status badges like `NO UPDATE SOURCE` and `DUPLICATE` pull attention away from filenames.
- **Responsive issue**: at common desktop widths, the center stage needs stricter minimum-width and inspector-width rules so the table does not slide under the inspector.
- **Left navigation issue**: the rail mostly works, but longer labels such as Needs Review can feel cramped. This is secondary for v1.
- **Inspector issue**: the empty inspector state is readable but visually blends into the page background; selected inspector content needs a clearer column boundary.
- **Table/list issue**: the list grid columns need tighter, intentional desktop sizing; status and facts columns should shrink before the file column becomes unreadable.
- **Future polish, not v1**: a larger navigation label strategy and a complete Library visual redesign remain out of scope.
- **Already acceptable**: Library route flow, list/grid/folder selection, More Details, Safe Action Preflight, Updates bridge, and Duplicates bridge already had runtime proof from the previous sprint.

## Changes made

- Added a scoped Library layout pass in `src/styles/globals.css`.
- Bounded the Library workbench into a predictable center stage and right inspector column.
- Added `min-width: 0`, `min-height: 0`, contained overflow, and separate scroll regions for the toolbar, list body, footer, folder view, and inspector.
- Tightened the toolbar and filter-chip row so controls wrap inside the stage instead of crowding the inspector.
- Made the list body scroll inside its own viewport and kept pagination in a separate footer band.
- Reduced the visual weight of Library status badges while keeping the same trust model.
- Changed Library duplicate badge copy to `Possible duplicate` in list, grid, and shared display helpers.
- Added mode-aware layout tests for Casual, Seasoned, and Creator user modes.
- Added desktop proof geometry checks for:
  - no document-level horizontal overflow
  - stage/list not overlapping the inspector
  - toolbar contained inside the stage
  - filter row not overlapping the table header
  - list shell not overlapping the footer
  - active sidebar label staying inside its nav item
- Updated the proof harness to navigate deterministically between Library and Settings while switching modes.
- Captured a named layout proof screenshot: `library-layout-overlap-fixed.png`.

## Verification

- `npm run build` passed with the existing Vite chunk-size warning.
- `npx tsc --noEmit` passed.
- `npm run test:unit` passed: 20 files, 68 tests.
- `npm run desktop:proof:fixtures` passed from native Windows PowerShell.
- `npm run desktop:smoke:fixtures` passed from native Windows PowerShell.
- Rust code was not touched. The desktop proof/smoke release builds still emitted the existing Rust warnings.

### Runtime and visual proof

- Latest proof folder: `output/desktop/library-proof/2026-05-10T13-49-53-256Z`.
- The proof captured:
  - `00-library-layout-casual.png`
  - `00-library-layout-seasoned.png`
  - `00-library-layout-creator.png`
  - `library-layout-overlap-fixed.png`
  - list selected state
  - grid view
  - folder view
  - detail sheet
  - Safe Action Preflight detail
  - Duplicates bridge context
  - Updates bridge context
- Geometry checks passed for Casual, Seasoned, Creator, and the selected-file list layout.
- Browser log inspection was available. The proof captured warning-level Tauri callback cleanup messages, but no runtime errors for the checked Library bridge flows.

### What could not be verified

- Only the native Windows PowerShell desktop lanes were run in this sprint. WSL runtime was not run.
- The screenshot proof covers the default proof window size. Smaller desktop widths rely on CSS responsive rules and unit coverage, not separate desktop captures.

## Commit

Pending.
