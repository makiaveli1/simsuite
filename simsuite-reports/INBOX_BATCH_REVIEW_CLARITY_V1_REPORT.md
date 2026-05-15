# Inbox Batch Review Clarity v1 Report

Date: 2026-05-15

Branch: `codex/inbox-batch-review-clarity-v1`

## 1. Inbox Data Meaning Audit

Current Inbox/Downloads data is intake data, not saved organization-plan data.

- `DownloadsScreen` consumes Downloads watcher and guided intake API data.
- The screen represents downloaded archives, direct downloaded files, review lanes, guided review plans, and blocked intake items.
- Current app-local pending batch data overlaps with Plan Preview/Pending Plans, but it is better understood as imported/downloaded content waiting for review.
- The existing backend still has file-changing Downloads and staging commands, but this sprint keeps the visible Inbox workflow review-only.
- Raw internal names or IDs should not be primary user-facing batch titles. Technical values belong behind collapsed details.

The current data belongs primarily in Inbox because it answers: what arrived, what needs review, and what could later become a preview organization plan.

Organize should own generated organization preview plans, not detailed downloaded/imported batch review.

## 2. Product Decision

Inbox owns imported/downloaded batch review.

Organize owns generated preview organization plans.

Pending Plans can mention imported/downloaded batches, but detailed review belongs in Inbox. This keeps the workflow simple:

- Inbox = review new content.
- Organize = create and review organization preview plans.
- Library = browse installed/indexed Mods and Tray content.
- Plan Preview/Pending Plans = preview work only, with no file changes.

## 3. User Impact Plan

Users should understand that Inbox is where new downloads and imported batches are reviewed before they become part of normal Library or Organize planning. Organize remains the place to create preview organization plans. No files are changed.

## What Changed

- Added a compact Inbox intake summary to explain the page, show review totals, and provide safe next steps.
- Changed visible Downloads wording toward `Inbox`, `review`, `handled`, and `set aside`.
- Hid visible enabled file-changing controls behind a review-only safety note.
- Added friendly labels for raw/internal-looking inbox item names.
- Updated Organize Pending Plans copy so imported/downloaded batch details point back to Inbox.
- Updated the direct Plan Preview route copy to explain that Inbox owns batch review and Organize owns plan creation.
- Updated desktop proof to verify Inbox copy, hidden raw IDs in primary labels, no enabled file-changing controls, and the Organize handoff.

No Rust, schema, or backend command behavior changed.

## What This Means For The User

Inbox is now clearer: it explains that new downloads and imported batches are intake material to review before Library or Organize planning. Users get safe next steps like `Create preview plan`, `Open Organize`, and `Open Library`, but the screen does not apply or move anything.

## Trust / Safety Boundary

No files are moved, deleted, disabled, quarantined, replaced, cleaned up, or auto-sorted.

The current Inbox UI is review-only. Existing backend mutation APIs remain internal and are blocked from the visible workflow in this sprint. Future file-changing actions still need preview, confirmation, backup/restore support, recoverable errors, and proof.

AI is not deciding file actions.

## Inbox vs Organize

- Inbox owns new/imported/downloaded content review.
- Organize owns generated organization preview plans.
- Pending Plans can summarize batch existence, but detailed batch review belongs in Inbox.

## Pending Plans Handoff

Organize Pending Plans now tells users that imported/downloaded batch details live in Inbox. It keeps Create Plan focused on generating preview-only organization suggestions.

## Tests

Focused validation:

- `npx vitest run src/screens/downloads/InboxIntakeSummary.test.tsx src/screens/downloads/DownloadsDecisionPanel.test.tsx src/screens/downloads/DownloadsRail.test.tsx src/screens/downloads/downloadsDisplay.test.ts src/screens/OrganizeScreen.test.tsx src/screens/StagingScreen.test.tsx src/trustBoundaryCopy.test.ts`: passed (`7` files, `20` tests).
- `npx tsc --noEmit`: passed.
- `npm run test:unit`: passed (`28` files, `103` tests).

- `npm run build`: passed; existing Vite chunk-size warning remains.
- `npm run desktop:proof:fixtures`: passed with `DESKTOP_LIBRARY_PROOF_OK`.
- `npm run desktop:smoke:fixtures`: passed with `Desktop smoke passed`.

## Desktop / Runtime Proof

Passed. The proof script captured `inbox-batch-review-clarity-v1.png` under:

`output/desktop/library-proof/2026-05-15T12-17-08-372Z/`

The proof verified:

- Inbox opens from the sidebar.
- Inbox shows intake/review and `No files changed` copy.
- raw internal IDs are not primary visible batch labels.
- no enabled file-changing controls are visible.
- Organize still opens.
- Pending Plans hands batch review toward Inbox.
- the direct Plan Preview route remains safe.
- no severe runtime errors were reported by the proof lane.

## What Could Not Be Verified Yet

- Saved organization-plan persistence does not exist yet.
- Inbox batch review is still based on current Downloads/intake data rather than a new backend model.
- Backend mutation commands were not removed; they remain blocked from the current visible workflow.
- The first parallel desktop proof/smoke attempt failed because both lanes tried to build/launch the Tauri release app at the same time. Sequential reruns passed.

## Linear Updates

Linear updates were added to `VEL-8` and `VEL-11`.

## Recommended Next Sprint

Apply Safety Contract design, or saved organization-plan persistence audit. Do not build real Apply yet.

## Unrelated Worktree Changes

Known unrelated dirty files were left alone unless explicitly noted in the final report:

- `.cocoindex_code/*`
- `src/screens/HomeScreen.tsx`
- unrelated pre-existing hunks in `src/styles/globals.css`
- unrelated pre-existing hunks in `SESSION_HANDOFF.md`
- unrelated pre-existing hunks in `docs/IMPLEMENTATION_STATUS.md`

## Commit

Commit hash is recorded in the final Codex report after commit.

## Final Honest Verdict

Verified: Inbox Batch Review Clarity v1 is working for the tested paths.
