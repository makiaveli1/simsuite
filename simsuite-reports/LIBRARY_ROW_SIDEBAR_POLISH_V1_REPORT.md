# Library Row Sidebar Polish v1

## Audit note

Date: 2026-05-10

Branch: `codex/library-row-sidebar-polish-v1`

Reviewed current code, the user screenshot, and the latest successful desktop proof screenshots in `output/desktop/library-proof/2026-05-10T13-49-53-256Z`.

### Findings

- Row clipping issue: confirmed. The list rows no longer broadly overlap the inspector, but lower row hints could be partly hidden, especially relationship/duplicate hint chips near the bottom of dense rows.
- Row hierarchy issue: confirmed. Rows tried to show file name, type, subtype, update status, review state, duplicate state, relationship hints, and supporting facts at once.
- Unnecessary row element: confirmed. Relationship hint chips are useful context, but they fit better in the inspector, More Details, or Safe Action Preflight than in every list row.
- Essential row element hidden: partial. Core file name/type and primary status were visible, but stacked status badges could crowd each other.
- Status badge issue: confirmed. Three stacked badges such as `No update source`, `Review suggested`, and `Possible duplicate` dominated the status column.
- Metadata chip issue: partial. Type and confidence chips are useful, but rows needed a cap so metadata did not force clipped lower content.
- Right inspector issue: confirmed. The inspector already had resize-capable infrastructure, but Library CSS clamped it to a fixed narrow width at the tested desktop size.
- Left nav issue: partial. The nav was usable, but long labels remained tight and needed containment rather than a full shell redesign.
- Folder sidebar issue: partial. Folder view was bounded and functional, but the folder tree pane needed better min/max containment and label clipping rules.
- Detail sheet issue: partial. The sheet worked, but it should carry deeper row context instead of rows trying to show everything.
- Preflight panel issue: partial. The panel opened and worked, but dense signal rows needed wrapping room and should stay readable.
- Responsive issue: confirmed. The default proof window worked structurally, but fixed inspector sizing and dense rows left too little workspace flexibility.
- Accessibility issue: partial. Main controls had accessible names; row details needed to stop using half-visible compact summaries.
- Proof gap: confirmed. Previous proof had geometry checks but did not check row child clipping or inspector resize/collapse behavior.
- Already acceptable: Library route, list/grid/folder views, file selection, More Details, Safe Action Preflight, Needs Review, Updates bridge, Duplicates bridge, and Open Folder proof survived the previous sprint.
- Future work, not v1: full left-nav resizing, full folder-pane resizing, dependency/missing-mesh detection, cleanup/delete flows, and new update provider work.

### Pre-existing unrelated worktree changes

The worktree still contains unrelated Home/status edits in `src/screens/HomeScreen.tsx`, `src/styles/globals.css`, `SESSION_HANDOFF.md`, and `docs/IMPLEMENTATION_STATUS.md`. The `src/styles/globals.css` change is still a Home hero metric styling tweak. These edits are not part of this sprint and were left unstaged unless a Library-specific hunk was required.

## What changed

- List rows now use a clearer hierarchy: file identity first, then a limited set of trust-safe row cues, then one or two compact at-a-glance facts depending on user mode.
- Relationship hints were removed from the visible list row. They remain available through the inspector, More Details, and Safe Action Preflight.
- The visible `+1 more` overflow marker was removed from rows because it added clutter without giving enough context.
- Status rows are capped to the strongest visible cues so `No update source`, review/care state, and possible duplicate signals do not stack into clipped row content.
- Row height and cell layout now allow visible row content to fit cleanly instead of half-rendering lower chips.
- The Library inspector now uses the existing persisted Library detail width preference, with a practical 300-480px range.
- The inspector can be resized wider/narrower and collapsed/expanded without the Library stage sliding underneath it.
- Folder tree rows now constrain labels and hide secondary clue text in the tree so folder browsing stays readable.
- Safe Action Preflight rows now wrap inside their panel instead of squeezing text into a cramped row.
- Desktop proof now checks that visible row children do not extend outside their row container.
- Desktop proof now verifies inspector resize wider, resize narrower, collapse, and expand behavior.

## Row hierarchy

Visible in list rows:

- file icon or thumbnail placeholder
- checkbox
- file name
- category/type chip
- subtype or identity line where useful
- at most two primary status cues
- one compact at-a-glance fact for Casual/Seasoned and up to two for Creator

Moved out of list rows:

- relationship hint chips
- deeper duplicate/update context
- lower-priority facts that previously appeared as clipped or summarized row chips

Removed from list rows:

- the visible `+1 more` overflow chip

The deeper information is still available in inspector, More Details, and Safe Action Preflight.

## Sidebar and panel polish

- Right inspector: resizable, collapsible, persisted through existing UI preferences, and bounded to avoid overlap with the Library stage.
- Left nav: existing route clarity was preserved; no broad shell redesign was made.
- Folder pane: tree labels now truncate cleanly and the folder pane uses tighter min/max containment.
- Detail sheet: kept as the deeper detail surface for context removed from rows.
- Safe Action Preflight: signal rows now wrap in a one-column layout so explanations stay readable.

## Files changed

- `src/screens/LibraryScreen.tsx`: passes the persisted Library inspector width into the workbench and marks collapsed inspector state for layout.
- `src/screens/library/libraryDisplay.tsx`: adds row status and fact summary helpers for capped, trust-safe list-row content.
- `src/screens/library/LibraryCollectionTable.tsx`: uses capped row status/fact summaries and stops rendering relationship hints or `+n more` chips in rows.
- `src/screens/library/VirtualizedLooseFiles.tsx`: applies the same row hierarchy to virtualized direct-file rows.
- `src/styles/globals.css`: adds Library-specific row, inspector, folder pane, and preflight layout containment. The unrelated Home hunk was not staged for this sprint.
- `scripts/desktop/desktop-library-proof.mjs`: adds row clipping geometry checks and inspector resize/collapse proof.
- `src/screens/library/libraryDisplay.test.ts`: covers status/fact summary behavior.
- `src/screens/library/LibraryCollectionTable.test.tsx`: covers trust-safe visible row cues and verifies the row no longer shows `+1 more`.
- `simsuite-reports/LIBRARY_ROW_SIDEBAR_POLISH_V1_REPORT.md`: records the audit, implementation, proof, and remaining gaps.
- `SESSION_HANDOFF.md` and `docs/IMPLEMENTATION_STATUS.md`: updated with the sprint handoff/status note.

## Tests and proof

Passed:

- `npx vitest run src/screens/library/libraryDisplay.test.ts src/screens/library/LibraryCollectionTable.test.tsx`
- `npx tsc --noEmit`
- `npm run test:unit`
- `npm run build` with the existing Vite chunk-size warning
- `npm run desktop:proof:fixtures`
- `npm run desktop:smoke:fixtures`

Rust files were not touched, so `cargo fmt`, `cargo check`, `cargo build --release`, and `npm run test:rust` were not run directly. The desktop proof/smoke wrappers built and launched the release Tauri app as part of their existing lane.

## Desktop/runtime proof

Latest proof folder:

`output/desktop/library-proof/2026-05-10T23-16-40-647Z`

Screenshots captured:

- `00-library-layout-casual.png`
- `00-library-layout-seasoned.png`
- `00-library-layout-creator.png`
- `01-library-selected-mccc.png`
- `library-layout-overlap-fixed.png`
- `library-row-sidebar-polish.png`
- `08-library-inspector-wider.png`
- `09-library-inspector-narrower.png`
- `10-library-inspector-collapsed.png`
- `02-library-grid-view.png`
- `03-library-folder-view.png`
- `04-library-detail-sheet.png`
- `05-library-preflight-detail-mccc.png`
- `06-duplicates-bridge-mccc.png`
- `07-updates-bridge-mccc.png`

Verified in the proof summary:

- Library opens.
- Casual, Seasoned, and Creator list layouts pass geometry checks.
- List, grid, folder, inspector, detail sheet, Safe Action Preflight, Updates bridge, and Duplicates bridge still work.
- Row clipping geometry returned no failures.
- Inspector width moved from 300px to 396px, then back to 312px in the resize proof.
- Inspector collapse/expand layout passed geometry checks.
- Browser log inspection was supported. No runtime errors were recorded. Only known Tauri callback cleanup warnings appeared.
- The viewport had no accidental body horizontal overflow in the checked states.

## What could not be verified

- WSL runtime was not run in this sprint.
- A separate 1440x900 or 1600x900 proof lane was not added; this proof used the default desktop proof window, whose captured viewport was 1360x880.
- Full left-nav resizing and full folder-pane resizing were not built in v1.
- OS-level Explorer clicking for Open Folder was not repeated in this sprint; the previous frontend/API proof remained covered by the existing test/proof path.
- Existing Vite chunk-size warning and Rust warnings emitted by the desktop wrapper lane remain unchanged.

## Remaining Library readiness work

- Add a dedicated narrow-window desktop proof lane if future screenshots show problems around 1366x768 or 1440x900.
- Consider a full left-nav expand/collapse pattern after the Library work surface stabilizes.
- Consider a resizable folder tree pane if folder-heavy users need it.
- Continue keeping deeper duplicate/update/context details in inspector, More Details, and Safe Action Preflight instead of list rows.

## Commit hashes

- Pending before commit.

