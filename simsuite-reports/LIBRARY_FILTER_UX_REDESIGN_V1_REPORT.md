# Library Filter UX Redesign v1 Report

Date: 2026-05-12

Branch: `codex/library-filter-ux-redesign-v1`

## Audit note before implementation

### Filter inventory

- Search: top primary command row, searches name/creator in the current Library query. Basic control, visible in Casual, Seasoned, and Creator. Tested for accessible textbox.
- Sort: top primary command row, changes ordering. Command state, not a narrowing filter. Tested for accessible combobox.
- Page size: top primary command row, changes pagination size. Command state, not a narrowing filter. Tested for accessible combobox.
- View mode: top primary command row, switches list/grid/folder. Display control, visible in all modes. Tested by desktop proof.
- Type filters: filter deck chips, visible in all modes. Basic narrowing filter. Current labels are understandable but the row reads like loose chips.
- Signal filters: filter deck chips, visible in all modes. Basic attention/care filter. Wording is trust-safe, but grouping can be stronger.
- Advanced filters: drawer with creator, source, confidence, subtype. Advanced narrowing filters, visible in all modes. Needs clearer active count and relation to filter deck.
- Density: grid-only command row control. Display preference, not part of filter reset.
- Clear/reset: currently lives mainly in Advanced drawer. Needs a more obvious clear path when filters are active.
- Active filter summary: only partial, mostly inside Advanced drawer. Needs a compact always-visible active row when filters/search are active.

### User-mode audit

- Casual (`beginner`): same filtering capability appears with simpler row language. The amount of control is acceptable, but visual hierarchy should make advanced controls secondary.
- Seasoned (`standard`): balanced default. Current type/signal split works but should be presented as one command surface.
- Creator (`power`): accepts more detail, and table language changes to `Clues`. Advanced filters should remain easy to discover without crowding the deck.

### Information architecture decision

- Primary command row: search, sort, page size, view mode, grid density, Advanced.
- Filter deck: Types and Signals groups with clear labels and active pressed states.
- Active state row: removable filter pills, `Clear filters`, and a separate `Sorted: ...` command pill with `Reset sort`.
- Advanced drawer: creator/source/confidence/subtype plus precision active pills. It remains secondary and is not a new feature area.

### Redundancy and confusion audit

- `All types` and `All signals` are useful because they live in separate labeled groups.
- Sort was previously counted with narrowing filters, which made reset semantics less clear.
- Active filter state was not visible enough outside Advanced.
- Clear all was hidden in the drawer, so users could miss it after applying search/type/signal filters.
- Signal labels stay trust-safe: `Possible updates`, `Needs review`, `No update source`, `Duplicates`.

### Responsive audit

- Latest proof screenshots reviewed from `output/desktop/library-proof/2026-05-11T08-46-44-527Z`.
- Default proof size, `1366x768`, and `1440x900` have no geometry failures, but the top strip uses substantial height and feels stacked.
- Inspector-collapsed proof shows the filter area can compress to less height when width is available.
- The redesign should keep the existing no-overlap geometry while making wrapping intentional.

## Implementation summary

- Reworked `LibraryTopStrip` into a command row, grouped filter deck, and active state row.
- Kept search, sort, page size, view toggles, grid density, and Advanced in the primary command row.
- Grouped content filters under `Types` and signal/care filters under `Signals`.
- Added removable active filter pills for search, type, signal, creator, source, confidence, and subtype.
- Added a visible `Clear filters` action that clears narrowing filters and search without changing view mode, page size, grid density, inspector state, or sort.
- Moved sort into a separate command-state pill: `Sorted: ...` with `Reset sort`.
- Kept `All types` and `All signals` labels and preserved the existing filter behavior across Casual, Seasoned, and Creator modes.
- Updated no-results copy so it points users toward clearing search/type/signal filters without implying missing or broken files.
- Added tolerant desktop proof geometry checks for command control overlap, filter/header overlap, active-row overlap, search visibility, Advanced visibility, and horizontal overflow.
- No dependency, backend, Rust, SQLite, or Tauri command changes were made.

## Validation

- `npx vitest run src/screens/library/LibraryTopStrip.test.tsx src/screens/library/LibraryCollectionTable.test.tsx src/screens/library/LibraryThumbnailGrid.test.tsx --reporter=dot` passed after tightening one selector: `3` files, `18` tests.
- `npx tsc --noEmit` passed.
- `npm run test:unit` passed: `22` files, `80` tests.
- `npm run build` passed with the existing Vite chunk-size warning.
- `npm run desktop:proof:fixtures` passed with `DESKTOP_LIBRARY_PROOF_OK`.
- `npm run desktop:smoke:fixtures` passed.
- Rust validation was not run because this sprint did not touch Rust. The Tauri release builds used by proof/smoke still emit existing unrelated Rust warnings.

## Desktop proof

- Latest proof folder: `output/desktop/library-proof/2026-05-12T00-03-33-220Z`.
- New filter screenshots captured:
  - `library-filter-ux-casual.png`
  - `library-filter-ux-seasoned.png`
  - `library-filter-ux-creator.png`
  - `library-filter-active-state.png`
  - `library-filter-no-results.png`
  - `library-filter-advanced-open.png`
  - `library-filter-responsive-1366x768.png`
  - `library-filter-responsive-1440x900.png`
- Filter geometry checks passed for active filters, Advanced open state, no-results state, and cleared-filter layout.
- Proof preserved the existing Library list/grid/folder, inspector resize/collapse, More Details, Safe Action Preflight, Updates bridge, Duplicates bridge, Needs Review, Open Folder, and responsive proof paths.
- Runtime proof reported no severe runtime errors. The captured browser log still includes known Tauri callback reload warnings.

## Unrelated worktree changes

Pre-existing unrelated Home/status changes must remain outside this sprint:

- `src/screens/HomeScreen.tsx`
- `src/styles/globals.css` Home hunks
- old Home/status hunks in `SESSION_HANDOFF.md`
- old Home/status hunks in `docs/IMPLEMENTATION_STATUS.md`

## Commit

Pending until the sprint commit is created.
