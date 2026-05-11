# Library Visual Productization v2 Report

Date: 2026-05-11

Branch: `codex/library-visual-productization-v2`

## Pre-Implementation Audit

### Screenshot critique

- List view: layout is stable and readable, but row cues still feel mechanical. `No update source` is calm enough to keep, while duplicate/review cues should stay secondary. Thumbnails are small and do not cover row text.
- Grid view: functional, but cards with no previews read too dim and empty. Resting title text exists, but the fallback surface needs stronger contrast so a missing preview feels intentional.
- Folder view: the tree and root state work, but the center pane is sparse. The folder header needs a stronger root/selected-folder presentation and Direct files guidance should feel designed instead of instructional filler.
- Inspector empty state: centered and calm, but slightly detached from folder browsing.
- Inspector selected state: useful, but still dense. The What this means section is the right place for explanation; it should stay quieter than the row list.
- Inspector resized/collapsed states: proof passed. The resize/collapse affordance works but needs a more deliberate splitter treatment.
- More Details/detail sheet: deserves the most work. The overlay is readable enough but the page behind still competes. The sheet needs a stronger review surface with a clear selected-file summary, preview/no-preview treatment, evidence grouping, and long path wrapping.
- Safe Action Preflight: the content is useful, but repeated `Confirmed` labels can overstate cautious clues. The panel should lead with review steps and use safer evidence labels.
- Duplicates bridge context: works and includes the right compare-before-changing message. Keep any changes small and do not imply cleanup safety.
- Updates bridge context: works and keeps no automatic update claims. The save-source sheet is usable but visually heavy.
- Left nav: readable after recent passes. No route architecture change is needed for v2.
- Toolbar/filter area: `All types` and `All signals` are clear and accessible. Keep grouping and active states.
- Responsive 1366x768 and 1440x900: proof passed without overlap or row clipping. Productization should preserve these geometry checks.

### Information hierarchy decisions

- Row: file identity, type/subtype, one or two strongest safe cues, and one compact fact.
- Grid card: thumbnail or fallback tile, readable identity at rest, type, and only secondary cues on select/reveal.
- Folder view: tree, selected folder/root summary, folder counts, child folders, and Direct files.
- Right inspector: preview, identity, At a glance, What this means, care/preflight, and safe next actions.
- More Details: deeper evidence, long paths, package/tray metadata, parser/inspection notes, update/duplicate clues, related hints, and raw-ish values.
- Safe Action Preflight: action-oriented caution list and route buttons; not a full repeat of More Details.
- Updates route: source setup, what SimSuite can/cannot check, and no-source context.
- Duplicates route: comparison context and compare-before-changing guidance.
- Needs Review route: review queue context and manual review language.

### Visual system audit

- Typography problem: details/preflight section hierarchy is too similar across heading, label, and body.
- Spacing problem: folder root view and sheet body need better grouping rhythm.
- Contrast problem: grid fallback cards and sheet backdrop need stronger readable surfaces.
- Panel/surface problem: detail sheet and preflight need a more opaque, focused modal feel.
- Badge/chip problem: Preflight evidence labels use `Confirmed` too often for cautious signals.
- Empty-state problem: folder empty/root states are acceptable but visually thin.
- Scroll/overflow problem: current geometry proof passes; preserve row clipping checks.
- Responsive problem: current proof passes at target laptop sizes.
- Accessibility issue: route buttons and toolbar controls already have stable accessible names; preserve them.
- Performance risk: do not add heavy animations, dependencies, or preview loading in list mode.
- Future work, not v2: large-library stress fixtures, true empty disk folder metadata, and backend folder query scaling.
- Already acceptable: Updates/Duplicates bridges are functionally covered and only need small visual touches if touched.

### Redundancy audit

- `No update source`: keep a quiet row cue; inspector explains what it means; More Details can show evidence.
- `Possible duplicate`: keep as a row/selected cue; inspector and Duplicates explain comparison. Do not imply safe deletion.
- `Stored in Tray`: keep where it clarifies source; avoid repeating it as both status and fact unless it adds context.
- Type/subtype: row and inspector can repeat because they anchor identity; More Details should add deeper evidence.
- `Confirmed`: replace in Preflight evidence labels with safer wording such as `Detected` where the label could be mistaken for safety proof.
- Full paths: keep out of rows; More Details and Duplicates right panel own long path display.

## Implementation Notes

- More Details is now treated as a structured evidence sheet instead of a plain metadata dump:
  - stronger opaque sheet surface and overlay
  - selected-file lead card with identity, evidence fields, and preview/no-preview tile
  - clearer section/row grouping and long-value wrapping
  - footer/action area remains stable
- Safe Action Preflight uses calmer evidence labels:
  - `Confirmed` was replaced with `Detected` for problem evidence labels that could sound too strong
  - relationship evidence now uses `Indexed clue`, `Detected clue`, or `Related hint`
  - caution copy remains explicit that SimSuite has limited information and does not decide what to change
- Grid no-preview cards now read as intentional content tiles:
  - no-preview fallback title changed to `no preview available`
  - card surfaces, fallback icons, resting identity labels, and cue placement have stronger contrast
- Folder root view received a visual pass:
  - tree rows, root/selected-folder header, guidance block, folder rows, and `Direct files` section are calmer and more structured
  - no backend empty-folder metadata support was added in this sprint
- Inspector/side panel polish stayed scoped:
  - selected/empty states get clearer surfaces and preview fallback treatment
  - resize/collapse behavior was preserved
- The desktop proof script now captures the v2 named screenshots:
  - `library-visual-productization-list.png`
  - `library-visual-productization-inspector.png`
  - `library-visual-productization-grid.png`
  - `library-visual-productization-folder.png`
  - `library-visual-productization-more-details.png`
  - `library-visual-productization-preflight.png`

No new dependencies were added. No Tauri command contracts, backend schema, file actions, update persistence, scraping, replacement, dependency, or safe-delete behavior changed.

## Validation

- `npx tsc --noEmit`: passed.
- Targeted Library tests:
  - command: `npx vitest run src/screens/library/LibraryDetailSheet.test.tsx src/screens/library/actionPreflight.test.ts src/screens/library/actionPreflight.render.test.tsx src/screens/library/LibraryThumbnailGrid.test.tsx src/screens/library/LibraryCollectionTable.test.tsx src/screens/library/FolderContentPane.test.tsx src/screens/library/LibraryTopStrip.test.tsx --reporter=dot`
  - result: passed, `7` files and `19` tests.
- `npm run build`: passed.
  - Vite still reports the existing large chunk warning.
- `npm run test:unit`: passed, `22` files and `76` tests.
- `npm run desktop:proof:fixtures`: passed with `DESKTOP_LIBRARY_PROOF_OK`.
  - screenshot folder: `output/desktop/library-proof/2026-05-11T01-49-27-273Z`
  - captured list, grid, folder, inspector, More Details, Preflight, Updates bridge, Duplicates bridge, user-mode, and responsive screenshots.
  - geometry checks passed at the default proof size plus `1366x768` and `1440x900`.
  - browser log inspection was supported; no severe runtime errors were reported. The only captured entries were known Tauri callback warnings during app reload.
- `npm run desktop:smoke:fixtures`: passed against the release app.
  - The Tauri build still reports existing Rust warnings; Rust was not touched.

Visual proof reviewed:

- `library-visual-productization-list.png`: list rows remain unclipped and calmer; row/status hierarchy is preserved.
- `library-visual-productization-grid.png`: resting identity labels and no-preview fallback tiles are visible.
- `library-visual-productization-folder.png`: root and folder summary areas are cleaner and still use `Direct files` wording.
- `library-visual-productization-inspector.png`: inspector selected state remains stable with no overlap.
- `library-visual-productization-more-details.png`: More Details now presents selected identity, no-preview state, and evidence sections as a focused sheet.
- `library-visual-productization-preflight.png`: Preflight uses safer `Detected`/clue wording and clearer caution grouping.

What could not be fully verified:

- Real extracted CC/Tray thumbnails from a live user library were not available in this fixture proof.
- Large-library stress behavior and true empty disk folder metadata remain for the separate hardening sprint.
- Updates and Duplicates bridge contexts were smoke/proof verified, but only small Library-opened context polish was in scope.

Unrelated worktree changes:

- Pre-existing Home/status changes remain outside this sprint:
  - `src/screens/HomeScreen.tsx`
  - `src/styles/globals.css`
  - earlier Home UI notes in `SESSION_HANDOFF.md`
  - earlier Home UI notes in `docs/IMPLEMENTATION_STATUS.md`
- This sprint did not edit `src/styles/globals.css`; Library CSS was scoped in `src/styles/phase5k-cards.css`.

## Commit

- Implementation commit: `6b511a6` - `Productize Library visual presentation`.
- Follow-up docs commit: records this implementation hash in the sprint report.
