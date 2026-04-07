# Library Inspector + Sheet Visual Pass — 2026-04-07

## Canonical root
- Windows: `C:\Users\likwi\OneDrive\Desktop\PROJS\SimSort\`
- WSL: `/mnt/c/Users/likwi/OneDrive/Desktop/PROJS/SimSort/`

All work in this pass was kept inside the canonical SimSort project.

## Accepted baseline preserved
These were treated as the fixed baseline and not undone:
- `4a3892f` — More details opens the real inspect path
- `d04b939` — standard view gets real inspect/edit surfaces
- `f491cb8` — wording/copy cleanup
- `0594646` — better inspector action hierarchy

## Goal of this pass
Do a real live-browser and screenshot-level critique of the Library inspector and the Library detail sheet flow, then fix visible problems without undoing the routing and hierarchy improvements that already landed.

## Live verification method
A repeatable Playwright audit harness was added:
- `scripts/desktop/library-inspector-sheet-audit.mjs`
- `scripts/desktop/debug-library-sheet-sections.mjs`

The audit ran against the live dev app with representative files:
- CAS: `NorthernSiberiaWinds_Skinblend.package`
- Script/gameplay: `MCCC_MCCommandCenter.ts4script`
- Tray: `OakHousehold_0x00ABCDEF.trayitem`

Experience views checked:
- Casual
- Seasoned
- Creator

Responsive sizes checked:
- 1366×768
- 1440×900
- 1920×1080
- 2560×1440

## Screenshot set reviewed
Audit output:
- `output/library-ui-audit-2026-04-07/`
- `output/library-ui-audit-2026-04-07/audit-results.json`

Representative screenshots captured for:
- inspector state per view
- inspect sheet
- warnings & updates sheet
- edit details sheet
- responsive variants for the script representative item

## What was visibly wrong
### 1. Sheet routing bug
The detail sheet modes were not actually scoped to their intended content.

Root cause:
- `librarySheetSections` used `.filter()` with array literals returned directly in multiple branches instead of booleans.
- In practice this meant `inspect`, `health`, and `edit` could all keep nearly the full inspector section set.

Visible symptom:
- The three sheets felt like the same overloaded surface with different titles.
- Inspect/health/edit depth was not honestly differentiated.

### 2. Sheet footer clipping
The Library detail sheet footer could fall below the viewport, especially on shorter desktop heights.

Visible symptom:
- `Done` sat partially or fully off-screen.
- The sheet body was acting like it could expand instead of respecting a strict viewport-bounded scroll region.

### 3. Weak sheet role clarity before the fix
Before the routing fix landed, the sheet felt deeper only in title copy, not in actual content scope.
That weakened:
- inspector vs sheet role clarity
- action credibility
- per-view differentiation

## Fixes applied in this pass
### A. Library sheet route scoping fixed
File:
- `src/screens/LibraryScreen.tsx`

Fix:
- corrected `librarySheetSections` filtering so each branch returns boolean membership checks with `.includes(section.id)`
- `inspect` now shows only the intended inspect sections
- `health` now shows health/update-focused sections
- `edit` now shows only creator/category editing sections

### B. Library sheet remount stability improved
File:
- `src/screens/library/LibraryDetailSheet.tsx`

Fix:
- keyed the sheet container and `DockSectionStack` by file id + mode + userView
- this prevents stale mode state from lingering when switching between inspect/health/edit

### C. Library sheet viewport behavior fixed
File:
- `src/styles/globals.css`

Fixes:
- moved the Library sheet to a stricter viewport-bounded height
- made the Library sheet use border-box sizing
- made the dock stack the real scroll region
- prevented the body from expanding and pushing the footer out of view
- tightened internal spacing/gap slightly so the footer clears at smaller desktop sizes
- improved inspector card spacing and action-copy rhythm slightly

## Verified state after fixes
### Inspector
What now feels right:
- primary action hierarchy remains intact
- Casual stays calmer with a single main action
- Seasoned keeps the richer action set without turning into a wall of controls
- Creator still feels richest because it keeps the full operational action row

### Sheets
What now feels right:
- Inspect, Warnings & updates, and Edit details are now genuinely different surfaces
- the title matches the actual scope of the sheet
- the sheet footer is visible in the verified responsive pass
- the sheet feels deeper than the sidebar without pretending every mode needs the full evidence stack

### Responsive verification
Verified from audit output:
- footer visible at 1366×768 for the checked responsive script flows
- footer visible at 1440×900, 1920×1080, and 2560×1440
- no horizontal overflow in the verified sheet metrics
- no crowded action row regression in the inspector metrics

## View differentiation notes
### Casual
- still the calmest surface
- one main action is correct
- sheet feels lighter and more approachable after routing is fixed

### Seasoned
- now has the clearest benefit from the fix
- inspect vs warnings vs edit finally read like different work modes instead of relabeled duplicates
- balanced density feels more believable now

### Creator
- still the richest view
- keeping the extra actions makes sense
- creator mode now earns its density better because edit and inspect stop duplicating the same full stack

## Remaining limitations / honest notes
### Specialist agent participation blocker
Required specialist participation was attempted, but the lane is currently degraded:
- Ariadne initial spawn path accepted earlier in the session, but session visibility remained restricted
- Sentinel / Scout / Forge retries repeatedly hit gateway WS timeout on spawn
- gateway restart was attempted during this pass
- `sessions_list` / spawn calls still timed out on the WS path

This is a real infrastructure blocker, not a skipped step.
No fake agent conclusions were invented in this document.

### Remaining visual issues to consider later
Non-blocking follow-ups after this pass:
- re-check if the sheet width should open slightly wider for Creator on very large desktops only
- consider whether the inspector action copy can be shortened one more notch in Seasoned
- consider slightly stronger visual separation between the sheet lead block and the first dock section on Creator

## Files changed in this pass
- `src/screens/LibraryScreen.tsx`
- `src/screens/library/LibraryDetailSheet.tsx`
- `src/styles/globals.css`
- `scripts/desktop/library-inspector-sheet-audit.mjs`
- `scripts/desktop/debug-library-sheet-sections.mjs`

## Smoke / verification evidence
Verified during this pass:
- live dev app launched and reachable
- Library opened
- representative rows selected successfully
- inspector actions opened the intended sheet routes
- sheet titles changed correctly per mode
- sheet section scope changed correctly after the routing fix
- footer visibility verified via Playwright debug probe and full audit rerun
- `npm run build` completed successfully

## Commit tracking
Commit for this pass should mention:
- library sheet mode scoping fix
- footer visibility / scroll-region fix
- live audit scripts added

Add the final commit hash here after commit.
