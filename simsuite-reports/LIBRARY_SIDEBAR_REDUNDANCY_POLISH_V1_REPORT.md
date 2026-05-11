# Library Sidebar Redundancy Polish v1 Report

Date: 2026-05-11

Branch: `codex/library-sidebar-redundancy-polish-v1`

## Issue

The Seasoned Library inspector could show the same destination action more than once. The clearest case was `Open in Updates`, which appeared both beside a cue explanation and again in the lower action area.

## Decision

The inspector now has one job per area:

- `What this means` explains cues only.
- `Before changing` opens the caution detail only.
- `Open` owns destination buttons such as Updates, Duplicates, Needs Review, Edit, and Open folder.

This keeps the sidebar calmer without hiding the Updates path for no-source files.

## Changes

- Removed inline route buttons from inspector cue explanation cards.
- Removed route buttons from the compact preflight card so it no longer duplicates destination actions.
- Added a centralized `Compare in Duplicates` action to the lower Open action area when duplicate cues exist.
- Kept one `Open in Updates` action in the lower Open action area for no-source/update-tracking cases.
- Avoided duplicating route buttons inside More Details when the Preflight section already owns that route.
- Added a desktop proof assertion that fails if visible inspector route actions repeat.

## Files changed

- `src/screens/library/LibraryDetailsPanel.tsx`
- `src/screens/library/actionPreflight.tsx`
- `src/screens/LibraryScreen.tsx`
- `src/screens/library/LibraryDetailsPanel.test.tsx`
- `scripts/desktop/desktop-library-proof.mjs`

## Validation

Passed:

- `npx tsc --noEmit`
- `npx vitest run src/screens/library/LibraryDetailsPanel.test.tsx src/screens/library/actionPreflight.render.test.tsx src/screens/library/actionPreflight.test.ts --reporter=dot`
- `npm run build`
- `npm run test:unit`
- `npm run desktop:proof:fixtures`
- `npm run desktop:smoke:fixtures`

Desktop proof folder:

- `output/desktop/library-proof/2026-05-11T08-46-44-527Z`

New proof assertion result:

- `Open in Updates`: `1`
- `Compare in Duplicates`: `1`
- `Review this file`: `1`
- repeated route actions: none

## Runtime notes

Desktop proof passed with `DESKTOP_LIBRARY_PROOF_OK`, and desktop smoke passed against the release app. Browser log inspection reported no severe runtime errors; only the known Tauri callback reload warnings appeared. The Tauri build still reports existing Rust warnings, and Vite still reports the existing large chunk warning.

## Unrelated worktree changes

Pre-existing unrelated Home/status changes remain outside this sprint:

- `src/screens/HomeScreen.tsx`
- `src/styles/globals.css`
- old Home note hunks in `SESSION_HANDOFF.md`
- old Home note hunks in `docs/IMPLEMENTATION_STATUS.md`

## Commit

Implementation commit:

- `7784912` - `Remove duplicate Library sidebar actions`

This report was updated after the implementation commit so the final handoff can cite the pushed commit set.
