# Updates Workflow v1 Report

Date: 2026-05-08
Branch: `codex/updates-workflow-v1`

## Audit Note Before Coding

### Implemented

- SQLite migration `database/migrations/0001_initial.sql` contains real watch storage:
  - `content_watch_sources`
  - `content_watch_results`
  - `special_mod_family_state`
  - indexes for watch source kind, anchor file id, and result status
- Rust watch commands are wired through Tauri:
  - `list_library_watch_items`
  - `list_library_watch_setup_items`
  - `list_library_watch_review_items`
  - `save_watch_source_for_file`
  - `save_watch_sources_for_files`
  - `clear_watch_source_for_file`
  - `refresh_watch_source_for_file`
  - `refresh_watched_sources`
  - `get_file_detail`
- Backend watch models exist:
  - `WatchStatus`
  - `WatchSourceKind`
  - `WatchSourceOrigin`
  - `WatchCapability`
  - `WatchResult`
  - setup/review/list response models
- Backend supports saved exact-page and creator-page watch sources for installed Library files.
- Backend blocks user custom watch-source saves for built-in supported special mods until a merge rule exists.
- Backend supports reminder-only creator pages.
- Backend marks CurseForge exact pages as provider-required instead of scraping them.
- Backend has a narrow supported check path for:
  - built-in MCCC downloads page
  - built-in XML Injector page
  - built-in or saved GitHub releases pages
- Backend has app settings and polling control for:
  - `automatic_watch_checks`
  - `watch_check_interval_hours`
  - `watch_auto_last_run_at`
  - `watch_auto_last_error`
  - `silent_special_mod_updates`
- Frontend has an `Updates` route and `UpdatesScreen`.
- Frontend API wrappers exist for watch list/setup/review/save/clear/refresh commands.
- Library detail and Safe Action Preflight already route update-source issues to Updates with a `fileId`.
- Problem Signal integration exists for `no_update_source`, and Library list rows can show `No update source`.

### Partial

- `UpdatesScreen` already separates setup, watched, attention, and reminder-only lanes, but wording still mixes stronger labels such as `Update found`, `Automatic checks supported`, and `Checks for updates automatically`.
- `UpdatesScreen` supports focused `fileId` routing, but the focused-file copy can be clearer when the file is not in the current queue page.
- The setup lane merges `list_library_watch_setup_items` with the first page of `list_library_files({ watchFilter: "not_tracked" })`; this gives a useful v1 "needs source" lane but is capped at `SOURCE_NEEDED_LIMIT` and does not represent every unwatched item in large libraries.
- `content_watch_results.status` has `current`, `exact_update_available`, `possible_update`, `unknown`, and implicit no-result states, but there is no distinct persisted `check_failed` status today.
- `unknown` currently covers both unclear and failed/unsupported checks in several paths. The UI can label it as `Could not check` where a check was attempted, but the backend does not distinguish every failure cause cleanly yet.
- Background polling exists but only refreshes sources whose capability is `can_refresh_now`; this is the right safety boundary, but the UI must not imply broad automatic updating.
- Home has watch summary links to Updates, but it is mostly summary-level and not file-focused.

### Mock/Stub Only

- `src/lib/api.ts` includes mock watch data for development/testing when the Tauri runtime is not present.
- CurseForge runtime integration is placeholder/provider-required only. There is no approved API integration in the app yet.
- Generic provider onboarding is not implemented.

### Dead/Unused Or Stale

- `database/schema/simsuite-v1.sql` is a stale schema mirror. It does not currently include the watch tables and several newer columns that exist in the migration-backed app schema.
- Older wording in the tray tooltip says `confirmed mod updates waiting` and `watched mods look current`; this should be softened to avoid overclaiming.

### Missing

- No full automatic download or replacement flow for updates, by design.
- No source scraping.
- No broad CurseForge matching.
- No generic creator-page parser.
- No provider settings/onboarding screen.
- No batch watch-source setup flow beyond existing bulk command groundwork.
- No watch history/source audit trail.
- No dedicated persisted `reminder_only` or `check_failed` status; these are currently derived from capability/review reason or from `unknown`.
- No exhaustive Updates queue beyond the first capped not-tracked Library page.

## v1 State Model To Use

- No source: `watchResult.sourceOrigin === "none"` or no source kind.
- Possible source: setup candidates returned by `list_library_watch_setup_items`.
- Watched: source exists and capability is `can_refresh_now`, with no active possible-update or unclear result.
- Reminder only: saved source exists and capability is `saved_reference_only`.
- Check failed / could not check: current backend exposes this mostly as `unknown` after a check or unsupported/unclear result.
- Possible update: current backend exposes this as `exact_update_available` or `possible_update`; UI should avoid claiming definite outdated status.
- Checked recently / no update found: current backend exposes this as `current`; UI should say `Checked recently` and `No update found`, not `Up to date`.

## Initial Implementation Direction

- Keep the existing backend storage and command boundary.
- Do not add scraping, provider matching, downloads, or replacement behavior.
- Clean up user-facing update wording to match trust-first states.
- Improve the focused Updates route copy and source-review panel so Library/Preflight navigation lands with context.
- Update stale schema mirror only if it can be done narrowly and safely.
- Add focused tests around truthful labels and focused routing before changing production UI text.

## Implementation Completed

### Backend Changes

- Softened the watch polling tray tooltip wording:
  - exact update results are now described as possible update leads to review
  - calm watched results now say there are no watched update leads right now
- Kept the backend provider/checker boundary unchanged:
  - no download flow
  - no replacement flow
  - no scraping
  - no broad CurseForge matching
  - no generic creator-page parsing
- Added/updated Rust tooltip regression coverage for the trust-first wording.

### Frontend Changes

- Cleaned up Updates state labels:
  - `No update source`
  - `Possible source`
  - `Watched`
  - `Reminder only`
  - `Could not check`
  - `Possible update`
  - `Update may be available`
  - `Checked recently`
  - `No update found`
- Removed automatic-update style claims from the Updates page and changed supported check language to explicit user-run checks.
- Improved the focused Updates route behavior by making Library/Preflight context visible when a `fileId` is opened but the file is outside the current lane page.
- Expanded the focused source review panel with:
  - file name and creator context
  - current watch/source state
  - detected clues
  - source confidence
  - what SimSuite can check
  - what SimSuite cannot check
  - only supported safe actions
- Made Home, Library list/detail, More Details compatibility chips, Settings watched-page controls, Downloads special-mod source guidance, and development mock watch data use the same cautious wording.

### Tests Added Or Updated

- Added a focused Updates test that proves possible-update results say `Update may be available` and do not show automatic-check claims.
- Added a focused Updates test that proves a no-source file opened by `fileId` still shows useful context even if it is not present in the current setup list.
- Updated existing Updates filter coverage for `Possible source` and `No update source`.
- Updated the Rust tray tooltip test for possible-update wording.

## Routing Notes

- Library still routes update-source cues to `Updates` with a file focus parameter.
- Safe Action Preflight still routes update-source issues to `Updates` with a file focus parameter.
- Updates now explains when the focused file came from Library or Preflight and is outside the current list, instead of silently showing a generic queue.
- No update checks are run during ordinary Library browsing by this work.

## Providers And Checkers Today

- Real supported checkers remain narrow:
  - built-in MCCC downloads page
  - built-in XML Injector page
  - built-in or saved GitHub releases pages
- Reminder-only sources are saved manual references, not live update checks.
- CurseForge remains provider-required/future work in this sprint.
- Generic web and creator pages are not treated as reliable live checkers unless a supported narrow checker exists.

## Wording Avoided

- Did not introduce `auto update available`.
- Did not introduce `definitely latest version`.
- Did not introduce `definitely outdated`.
- Did not introduce `download replacement now`.
- Did not introduce `safe to replace`.
- Did not introduce `official source found`.
- Did not introduce `source is verified`.
- Did not introduce `broken mod`, `broken CC`, `missing mesh`, `missing dependency`, `safe to delete`, or `unsafe to delete` in the update workflow.

## Validation

- `npm run test:unit -- src/screens/UpdatesScreen.test.tsx`
  - first run failed on the new expected trust-first wording, then passed after implementation
- `cargo test --manifest-path src-tauri/Cargo.toml tray_tooltip_prefers_exact_updates`
  - first run failed on the old tray tooltip wording, then passed after implementation
- `npm run test:unit -- src/screens/library/LibraryDetailsPanel.test.tsx src/screens/library/libraryDisplay.test.ts src/screens/library/actionPreflight.test.ts`
  - passed
- `npm run build`
  - passed with the existing Vite chunk-size warning
- `npx tsc --noEmit`
  - passed
- `npm run test:unit`
  - passed: 15 files, 49 tests
- `cargo fmt --manifest-path src-tauri/Cargo.toml`
  - passed
- `cd src-tauri; cargo check`
  - passed with existing Rust warnings
- `cd src-tauri; cargo build --release`
  - passed with existing Rust warnings
- `npm run test:rust`
  - passed: 219 tests, with existing Rust warnings

## Desktop Runtime Proof

- `npm run desktop:proof:fixtures`
  - failed before launching because the npm script calls `/mnt/c/Windows/System32/WindowsPowerShell/v1.0/powershell.exe`, which is not a valid path from this native Windows PowerShell session.
- `powershell.exe -NoProfile -ExecutionPolicy Bypass -File scripts\desktop\run-desktop-library-proof.ps1`
  - passed
  - proof summary written to `output\desktop\library-proof\latest-summary.json`
- `npm run desktop:smoke:fixtures`
  - failed before launching for the same `/mnt/c/...` wrapper path issue.
- `powershell.exe -NoProfile -ExecutionPolicy Bypass -File scripts\desktop\run-tauri-smoke.ps1`
  - passed against `src-tauri\target\release\simsuite.exe`

## Regression Notes

- Focused frontend coverage exercised Updates routing/context, Library update labels, Library details, More Details update wording, and Safe Action Preflight update-source routing helpers.
- Full unit test coverage passed after the wording and focused-route changes.
- Desktop fixture proof and smoke passed through the native scripts.
- No new provider, scraper, downloader, replacement, or broad matching code was added.

## Tooling Audit

- The desktop fixture npm scripts are written for a WSL-style environment and fail in native Windows PowerShell because of `/mnt/c/...` paths.
- The underlying PowerShell scripts work in this Windows session and were used for runtime verification.
- This is a tooling gap, not an app runtime failure, but the npm wrappers should be made cross-environment in a future maintenance pass.

## Future Work

- Add a distinct persisted `check_failed` status if the backend needs to separate failed checks from unknown/unsupported checks.
- Add a first-class persisted reminder-only state if future workflows need source history beyond capability-derived behavior.
- Make the Updates setup queue exhaustive or paged beyond the current capped no-source list.
- Add provider onboarding/settings only after a safe provider abstraction exists.
- Treat CurseForge as future provider work unless an approved API path is implemented without scraping, guessy matching, or secret exposure.
- Keep automatic download/replacement out of Updates until a separate, explicit, safe workflow is designed.

## Commit

- Implementation commit: `dbd27a4` (`Updates workflow v1 trust states and focused routing`).
