# Problem Signals v1 — Grounded Audit

## Scope
Safe, evidence-based warning signals for Library + Needs Review bridge.

## Confirmed persisted signal sources

### File row/detail fields
- `files.safety_notes`
- `files.parser_warnings`
- `files.confidence`
- `files.source_location`
- `files.insights` JSON
- duplicate presence via `duplicates` table / `has_duplicate` / `duplicate_types`
- update/watch state via `watch_status` / `watch_result`

### Confirmed scan-derived codes already in use
- Parser warnings:
  - `no_category_detected`
  - `conflicting_category_signals`
  - `conflicting_creator_signals`
- Safety notes:
  - `unsafe_script_depth`
  - `tray_file_in_mods_root`
- Review-only reason:
  - `low_confidence_parse`

## Review queue
- `scanner::queue_review_items(...)` inserts:
  - `low_confidence_parse` when `confidence < 0.55`
  - all parser warning flags
  - all safety notes
- `review_queue` schema is `UNIQUE(file_id, reason)`
- `ReviewScreen` already renders queue reason, confidence, suggested path, safety notes

## Existing Library behavior
- Quick filter `NeedsAttention` currently means:
  - `f.safety_notes <> '[]' OR f.parser_warnings <> '[]'`
- Current list/grid strongest issue logic:
  1. safety note -> `Needs review`
  2. tray -> `Stored in Tray`
  3. parser warning -> `Warning`
- Current detail surfaces already expose:
  - safety notes
  - parser warnings
  - duplicate count/types
  - watch/update state
  - version evidence
  - tray context

## Safe v1 candidates already supported by real data
- review suggested / already queued for review
- parser warning
- safety note / inspection warning
- duplicate candidate
- no update source
- stored in Tray
- weak classification confidence
- missing creator metadata
- script mod caution

## Important gaps
These are NOT currently persisted as reliable per-file signals:
- inspect-file failure (`inspect_file` errors are logged, then scanner stores default empty inspection outcome)
- DBPF corrupt/out-of-bounds record skip warnings
- thumbnail extraction failure warnings
- unsupported-format problem signals for installed library content

## Current recommendation
Problem Signals v1 should build on existing scan/library/review/watch/duplicate primitives first, then optionally add a small persisted `inspection_failed` signal if we want a safe new bridge without inventing broken-CC logic.

## Problem Signals v1b — Needs Review bridge

### What was missing
The Library surfaces already exposed warnings, but the route into Review was incomplete and inconsistent:
- Review could not focus a specific file from Library.
- The inspector/details action only appeared when the wrong subset of signals was visible.
- Review-worthy safety notes and parser warnings could be present without a clear Library → Review handoff.

### Bridge placement
- **Inspector / Needs attention**: add a focused secondary action so the user can jump into Review without turning Library into the workflow page.
- **More Details / warning-care area**: show the same bridge beside the warning evidence, where the user is already reading why the file is risky.
- **No row/card spam**: list rows and grid cards stay quiet.

### Signals that should route to Review
Route these into Review when present:
- review queue membership / review suggested
- parser warnings
- inspection failure / could not inspect fully
- safety notes / serious care state
- script placement warning

### Signals that should not route by default
These stay in their own lanes:
- **Duplicate** → Duplicates workflow
- **No update source** → Updates / watch setup
- **Stored in Tray** → informational
- **Same pack / related hint** → informational
- **Weak metadata** → visible as a caution, but not enough on its own to trigger the Review bridge

### Runtime proof
Verified against a fresh dev server on `http://localhost:1421/#library` because the older `http://localhost:1420/` session was serving stale transformed modules from the mounted Windows tree.

Grounded checks:
- Selected `MCCC_MCCommandCenter.ts4script` in Library.
- Confirmed Review action appears in the Library details flow (`Review this file`).
- Clicked it and confirmed Review opened at `http://localhost:1421/#review` with the matching queue item selected.
- Confirmed Duplicate-only item `AHarris00_CozyKitchen.package` did **not** show a Review action.
- Confirmed no-update-source-only item `TwistedMexi_BetterExceptions.ts4script` did **not** show a Review action.
- Confirmed no browser console errors beyond normal Vite connection messages.
