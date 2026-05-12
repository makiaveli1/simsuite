# Library Cohesive Cozy Polish v1 Report

## Pre-Implementation Audit

### 1. Worktree Classification

Uncommitted Library changes found and preserved:

- Grid density slider polish is present in `src/screens/library/LibraryTopStrip.tsx`, `src/styles/phase5k-cards.css`, tests, and the desktop proof script. The grid control is a continuous left-to-right slider with a visible thumb.
- Collapsible filter work is present in `LibraryTopStrip.tsx`, `phase5k-cards.css`, tests, and proof screenshots. The filter deck can collapse, and active filters remain visible on the Filters button.
- Row/folder thumbnail work is present through `src/screens/library/LibraryRowThumbnail.tsx`, row/folder renderers, query preview flags, tests, and proof screenshots. List rows and folder Direct files now share the same preview/fallback logic.
- Desktop proof artifacts from `output/desktop/library-proof/2026-05-12T01-49-51-658Z` show the uncommitted Library follow-ups passing the latest proof path.

Unrelated Home/status changes found:

- `src/screens/HomeScreen.tsx` has a Home module heading layout tweak.
- `src/styles/globals.css` has Home-specific responsive/layout changes.
- `SESSION_HANDOFF.md` and `docs/IMPLEMENTATION_STATUS.md` have pre-existing status notes from prior work.

What will be included:

- Library frontend, Library CSS, Library tests, Library proof script updates, the new report, and focused Library status/handoff notes.

What will be left alone:

- HomeScreen changes and Home-specific `globals.css` hunks will not be staged for the Library commit.

### 2. Cohesion Diagnosis

- Visual inconsistency: Mostly acceptable after the visual productization pass, but the Library still lacks a small user-controllable atmosphere layer that ties list, grid, folder, inspector, detail, and preflight surfaces together.
- Theme/token inconsistency: Library CSS already uses local variables and global theme tokens. The safest improvement is a scoped Library data attribute with CSS variables, not a global theme rewrite.
- Spacing inconsistency: Filter collapse, row thumbnails, grid cards, and More Details are improved. Some cozy emphasis can be added through surfaces and selected states without changing layout dimensions.
- Chip/badge inconsistency: Current chips are calmer than earlier passes. Do not redesign them again unless the atmosphere layer needs subtle token alignment.
- Surface/panel inconsistency: Detail and Preflight are readable. The atmosphere mode can apply warmer surfaces and restrained panel glow across Library-specific surfaces.
- Thumbnail/fallback inconsistency: Shared row/folder fallback logic is present. Grid/detail fallback surfaces can inherit atmosphere variables.
- Mode inconsistency: Casual, Seasoned, and Creator screenshots render correctly. Creator remains more clue-oriented; Casual remains simpler.
- Filter/control inconsistency: Collapsible filter work is present, but collapse state currently needs to use the existing persisted preference path.
- Empty-state inconsistency: Acceptable for this sprint; no new empty-state rewrite needed.
- Motion/interaction issue: Existing filter collapse animation is scoped and has reduced-motion support. Atmosphere mode must be CSS-only and motion-light.
- Accessibility issue: New atmosphere and filter persistence controls need accessible names/states and keyboard operation.
- Performance risk: Avoid JavaScript animation, timers, heavy blur, global theme rewrites, preview refetching, or virtualization changes.
- Already acceptable: Core Library proof paths, row/folder thumbnails, continuous grid size slider, filter collapse visuals, inspector resize/collapse, More Details, Preflight, and route bridges.
- Future work: Large-library stress proof, true empty disk-folder metadata, and real-library thumbnail validation.

### 3. Theme Engine Audit

- Existing theme variables are applied from `UiPreferencesProvider` through `document.documentElement.dataset.theme`, `dataset.density`, and `dataset.userView`.
- User mode labels map to `beginner`, `standard`, and `power`, surfaced as Casual, Seasoned, and Creator in screenshots.
- Layout and filter preferences already persist through `UiPreferencesContext` and mode-scoped localStorage keys.
- `libraryFiltersCollapsed` already exists in the preference context, so the safest path is to wire the Library filter disclosure to that persisted value instead of keeping collapse state inside `LibraryTopStrip`.
- Reduced motion is already respected for filter collapse in `phase5k-cards.css`.
- The app is a dark themed desktop UI with selectable global themes; this sprint should not rewrite the global theme engine.
- The new Library atmosphere should be Library-scoped through a persisted boolean preference and a `data-library-atmosphere` attribute on the Library workbench.

### 4. Wow Feature Decision

Selected feature: Library Atmosphere Mode v1.

What it does:

- Adds a small `Cozy glow` control in the existing Advanced/display area.
- Applies a scoped, CSS-only Library atmosphere that warms Library surfaces, fallback thumbnails, selected states, detail sheet, and preflight panels.
- Leaves all backend data, filtering, sorting, routes, thumbnails, and product-truth rules unchanged.

Where the control lives:

- Inside the Library Advanced drawer so it does not crowd the command bar.

How it persists:

- Through `UiPreferencesContext` and localStorage, matching existing Library preference patterns.

What surfaces it affects:

- Library workbench shell, top strip, list rows, thumbnail fallbacks, grid cards, folder surfaces, inspector, More Details, and Safe Action Preflight.

How it avoids performance issues:

- It changes one data attribute and CSS variables/classes only.
- It uses no timers, canvas, particles, video, preview refetching, or layout measurement.

How it respects reduced motion:

- New visual transitions remain subtle and are disabled or reduced under `prefers-reduced-motion`.

How it avoids reducing readability:

- Text colors are not lowered; contrast-sensitive text remains on the existing dark surfaces.

## Implementation Notes

- Preserved the uncommitted Library follow-ups for the continuous grid density slider, collapsible filters, and row/folder thumbnail renderer.
- Added `libraryAtmosphere` to `UiPreferencesContext`, persisted under `simsuite:library-atmosphere`.
- Added a scoped `data-library-atmosphere` state on the Library workbench and root dataset.
- Added a `Cozy glow` toggle inside the Library Advanced drawer so the command bar does not get more crowded.
- Wired `LibraryTopStrip` filter collapse to `libraryFiltersCollapsed` instead of local component-only state.
- Kept the atmosphere layer CSS-only and scoped to Library surfaces.
- Added subtle warm surface treatment for the stage, inspector, top strip, selected list rows, fallback thumbnails, grid cards, More Details, and Preflight.
- Kept the existing trust-safe Library wording. No safe-delete, dependency, missing-mesh, automatic-update, or confirmed-duplicate claims were added.
- Added no dependency and made no Rust/backend/schema changes.

## Accessibility Notes

- `Cozy glow` is a real button with `aria-pressed` and an accessible on/off name.
- The filter disclosure continues to expose `aria-expanded` and `aria-controls`.
- Focus-visible styling is included for the atmosphere toggle.
- The atmosphere layer does not move essential information behind hover-only UI.
- Reduced motion is respected by disabling the new atmosphere/control transitions where practical.

## Performance Notes

- The atmosphere feature changes one persisted boolean and one Library-scoped data attribute.
- No timers, canvas, video, particles, heavy blur, preview fetching, thumbnail parsing, or virtualization changes were added.
- The filter collapse persistence reuses the existing preference context and localStorage pattern.
- The only new icon comes from the existing `lucide-react` dependency already used by the app.

## Visual Proof

- Proof folder: `output/desktop/library-proof/2026-05-12T02-24-21-812Z`.
- New named screenshots:
  - `library-cozy-polish-atmosphere-on.png`
  - `library-cozy-polish-filters-collapsed.png`
  - `library-cozy-polish-list.png`
  - `library-cozy-polish-grid.png`
  - `library-cozy-polish-folder.png`
  - `library-cozy-polish-detail-sheet.png`
  - `library-cozy-polish-preflight.png`
- Existing proof also covered Casual, Seasoned, Creator, selected row, grid, folder, inspector wide/narrow/collapsed, More Details, Preflight, Duplicates bridge, Updates bridge, and responsive `1366x768` / `1440x900`.

## Responsive Proof

- Default desktop proof passed.
- `1366x768` proof passed.
- `1440x900` proof passed.
- Cozy atmosphere geometry check passed with no horizontal overflow.
- Filter collapsed and expanded checks passed before and after cozy mode was enabled.

## Validation

- `npm run build` passed. Existing Vite chunk-size warning remains.
- `npx tsc --noEmit` passed.
- `npm run test:unit` passed: 22 files, 87 tests.
- `npm run desktop:proof:fixtures` passed with `DESKTOP_LIBRARY_PROOF_OK`.
- `npm run desktop:smoke:fixtures` passed.
- Tauri release builds still emit existing Rust warnings unrelated to this frontend sprint.

## Files Changed

- `src/components/UiPreferencesContext.tsx`: added persisted Library atmosphere preference and root dataset.
- `src/components/layout/Workbench.tsx`: allowed safe div attributes so Library can expose the scoped atmosphere data attribute.
- `src/screens/LibraryScreen.tsx`: wired Library atmosphere and persisted filter collapse into the Library workbench/top strip.
- `src/screens/library/LibraryTopStrip.tsx`: added controlled filter collapse props and the Advanced-drawer `Cozy glow` control.
- `src/styles/phase5k-cards.css`: added scoped atmosphere surfaces, toggle styling, and reduced-motion handling.
- `src/screens/LibraryScreen.test.tsx`: added persisted atmosphere coverage.
- `src/screens/library/LibraryTopStrip.test.tsx`: added controlled filter collapse and atmosphere toggle coverage.
- `scripts/desktop/desktop-library-proof.mjs`: added atmosphere checks and new cozy proof screenshots.
- `SESSION_HANDOFF.md` and `docs/IMPLEMENTATION_STATUS.md`: added Library sprint notes.

## Unrelated Worktree Changes

- `src/screens/HomeScreen.tsx` remains an unrelated Home change and should not be staged for this Library commit.
- Home-specific hunks in `src/styles/globals.css` remain unrelated and should not be staged for this Library commit.
- Pre-existing Home/status notes in the docs were not part of this sprint. The Library notes were added and should be staged selectively.

## Remaining Library Readiness Work

- Large-library stress and edge-case fixture proof.
- True empty disk-folder metadata support, if the backend scanner can support it safely.
- Live real-library thumbnail validation, because fixture data does not contain real Sims thumbnail payloads.
- Longer-term theme-engine consolidation can be considered later, but this sprint intentionally avoided a global theme rewrite.

## Commit Hashes

- `1934ddf` - Bring Library polish and atmosphere together
