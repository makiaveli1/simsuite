# Updates State Persistence v1 Report

Date: 2026-05-08
Branch: `codex/updates-state-persistence-v1`

## Audit Before Coding

### Worktree

- `git status --short --branch` showed the branch start on `codex/updates-workflow-v1` with one pre-existing unstaged file: `src/styles/globals.css`.
- The CSS diff is a Home hero metric styling change only:
  - `.home-hero-metric span` gets a text color.
  - `.home-hero-metric small` gets a different text color and line height.
- This is unrelated to Updates state persistence. It will stay untouched and unstaged for this sprint.
- Created a focused branch for this sprint: `codex/updates-state-persistence-v1`.

### Implemented

- `database/migrations/0001_initial.sql` has real watch storage:
  - `content_watch_sources`
  - `content_watch_results`
  - text `status`, text `confidence`, JSON evidence, `checked_at`, `note`
- `content_watch_results.status` is flexible text today. There is no CHECK constraint, so adding new status labels does not require a schema migration.
- Backend commands are wired:
  - `list_library_watch_items`
  - `list_library_watch_setup_items`
  - `list_library_watch_review_items`
  - `save_watch_source_for_file`
  - `clear_watch_source_for_file`
  - `refresh_watch_source_for_file`
  - `refresh_watched_sources`
  - `get_file_detail`
- Rust watch models already exist:
  - `WatchSourceKind`
  - `WatchCapability`
  - `WatchStatus`
  - `WatchSourceOrigin`
  - `WatchResult`
  - `LibraryWatchReviewReason`
- Real supported generic checks are narrow:
  - GitHub releases pages
  - MCCC downloads page
  - XML Injector page
- CurseForge is provider-required and not checked.
- Generic/creator pages are not scraped.
- Frontend already has trust-first labels for the previous states, including `Could not check` and `Reminder only`.
- Library and Safe Action Preflight already route update-source issues into Updates with file context.

### Partial

- `WatchStatus` supports `unknown` but not `check_failed` or `reminder_only`.
- `unknown` currently covers several different cases:
  - supported check succeeded but result was unclear
  - unsupported/provider/manual cases
  - failed special-mod latest checks
  - refreshed reminder-only source rows
- `refresh_watch_source_for_library_file` propagates real generic checker errors instead of saving a failed result into `content_watch_results`.
- `special_mod_versions::load_or_refresh_latest_info` catches checker errors, but persists them as `latest_status = 'unknown'`.
- Reminder-only is modeled as `WatchCapability::SavedReferenceOnly` and `LibraryWatchReviewReason::ReferenceOnly`, but the persisted watch result status is either missing, `not_watched`, or `unknown`.
- `list_library_watch_review_items` has `provider_needed`, `reference_only`, and `unknown_result`, but no first-class `check_failed` review reason.
- `load_watch_counts` and Home overview group all unclear/failed generic rows into `unknown_watch_items`.
- Frontend `WatchStatus` only knows `unknown`; UI maps it to `Could not check`, which overstates unsupported/manual-only unclear cases.

### Mock/Stub Only

- `src/lib/api.ts` mock watch data builds reminder-only and provider-required states in memory.
- Mock refresh turns non-refreshable sources into `unknown`, matching the old backend behavior.
- No CurseForge runtime integration exists.

### Dead/Unused Or Stale

- `database/schema/simsuite-v1.sql` is a stale mirror and does not include the watch tables that exist in the migration-backed schema.
- This file says migrations are the source of truth. It is not safe to treat the mirror as authoritative in this sprint.

### Missing

- No persisted `check_failed` watch status.
- No persisted `reminder_only` watch status.
- No clear split between failed checks and unclear/unsupported results.
- No frontend type support for `check_failed` or `reminder_only`.
- No dedicated review reason/count for check failures.
- No tests proving failed checks are saved as failed instead of `unknown`.
- No tests proving reminder-only sources are represented as reminder-only after save/refresh.

## Implementation Direction

- Add `check_failed` and `reminder_only` to the status model without a database migration because `content_watch_results.status` is already text.
- Keep reminder-only as a source capability concept, but also persist `status = 'reminder_only'` for saved reference-only sources so the state survives list/detail reads.
- Persist `status = 'check_failed'` only when SimSuite actually attempts a supported check and the check fails.
- Keep unsupported pages as `unknown` or provider/reference states rather than pretending a check ran.
- Keep all provider boundaries unchanged:
  - no scraping
  - no CurseForge integration
  - no automatic download
  - no automatic replacement

## Validation

- `cargo test --manifest-path src-tauri/Cargo.toml supported_watch_failure_persists_check_failed_state -- --nocapture` passed after the new backend failure persistence path was implemented.
- `cargo test --manifest-path src-tauri/Cargo.toml reminder_only -- --nocapture` passed after saved reference-only sources persisted `reminder_only`.
- `cargo test --manifest-path src-tauri/Cargo.toml unknown_status_remains_unclear_successful_check_not_failure -- --nocapture` passed after `unknown` stayed separate from `check_failed`.
- `npm run test:unit -- UpdatesScreen.test.tsx --runInBand` did not run because Vitest does not support the Jest `--runInBand` option.
- `npm run test:unit -- UpdatesScreen.test.tsx` passed: 6 tests.
- `npm run build` passed with the existing Vite chunk-size warning.
- `npx tsc --noEmit` passed.
- `npm run test:unit` passed: 16 files, 54 tests.
- `cargo fmt --manifest-path src-tauri/Cargo.toml` passed.
- `cd src-tauri; cargo check` passed with existing Rust warnings.
- `cd src-tauri; cargo build --release` passed with existing Rust warnings.
- `npm run test:rust` first failed because one old test still expected a generic reference-only exact page to be `not_watched`; the test was updated to the new persisted `reminder_only` behavior.
- `npm run test:rust` then passed: 221 tests.
- `npm run desktop:proof:fixtures` passed from native Windows PowerShell and reached `DESKTOP_LIBRARY_PROOF_OK`.
- `npm run desktop:smoke:fixtures` passed from native Windows PowerShell and reached `Desktop smoke passed`.

## Changes Made

### Backend

- Added `check_failed` and `reminder_only` to `WatchStatus`.
- Added `CheckFailed` to `LibraryWatchReviewReason` and added `check_failed_count` to the review response.
- Added `check_failed_watch_items` to watch refresh summaries and Home overview counts.
- Persisted `reminder_only` in `content_watch_results` when a saved source is reference-only.
- Kept `checked_at` empty for reminder-only rows because no automatic check was attempted.
- Refactored generic refresh through a testable fetcher path so supported checker failures can be saved instead of thrown away.
- Persisted `check_failed` when a supported generic check is attempted and fails.
- Updated special-mod latest checks so caught network/provider/parser errors persist `latest_status = 'check_failed'`.
- Kept provider-required sources as provider-blocked review items instead of treating them as failed checks.
- Kept unsupported/manual-only sources out of automatic refresh targets.
- Updated Library row parsing and update sorting for the new statuses.
- Updated tray/Home counts to keep failed checks separate from unclear results.

No database migration was added because `content_watch_results.status` and `special_mod_family_state.latest_status` are flexible text columns with no CHECK constraint.

### Frontend

- Added TypeScript support for `check_failed`, `reminder_only`, `checkFailedCount`, and `checkFailedWatchItems`.
- Updated Updates attention filters:
  - `Could not check` now maps to `check_failed`.
  - `Manual review needed` maps to `unknown`.
- Updated Updates detail/source copy so:
  - `check_failed` says SimSuite tried but could not finish.
  - `reminder_only` says the source is saved for manual follow-up.
  - `unknown` no longer implies a failed check.
- Updated Library list, inspector, More Details, Home, Settings, Downloads special-mod copy, Field Guide, and mock API state mapping.
- Added mock data for a `check_failed` saved source and a `reminder_only` saved source.
- Added frontend tests for check-failed and reminder-only rendering and wording guardrails.

## State Model Now

- `not_watched`: no saved update source/result.
- `reminder_only`: a saved source/reference exists, but SimSuite cannot check it automatically today.
- `current`: a supported check ran and no update lead was found; UI renders this as `Checked recently` / `No update found`.
- `exact_update_available`: a stronger possible update lead exists; UI renders this as `Update may be available`.
- `possible_update`: cautious possible update lead; UI renders this as `Possible update`.
- `check_failed`: SimSuite attempted a supported check and could not finish.
- `unknown`: unclear or unsupported result, not a failed check by itself.

## Providers And Checkers

Real today:

- GitHub releases, only for supported release-page URLs.
- MCCC downloads page helper.
- XML Injector page helper.

Future work:

- CurseForge remains provider-required/future approved API work.
- Generic creator pages remain reminder-only.
- Generic website scraping remains out of scope.
- Automatic downloads and replacement remain out of scope.

## Runtime Proof

- Desktop proof wrapper started the app and reached the expected library proof.
- Desktop smoke wrapper started the app and reached the expected smoke proof.
- The desktop fixture data did not naturally expose a persisted `check_failed` or `reminder_only` state during those scripted proofs.
- State-specific UI behavior is covered by unit tests and mock API data.

## Pre-existing CSS Change

- `src/styles/globals.css` remains a pre-existing, unrelated Home hero metric styling change.
- It was inspected at the start of the sprint.
- It was not edited for this sprint and must not be included in the sprint commit.

## Remaining Future Work

- Add real fixture data that naturally exercises `check_failed` and `reminder_only` in desktop proof runs.
- Consider a future schema mirror cleanup for `database/schema/simsuite-v1.sql`; migrations remain the source of truth.
- Design approved provider onboarding before adding CurseForge or other provider integrations.
- Keep broad matching, scraping, automatic download, and automatic replacement out of scope until explicitly designed.

## Commit

- Implementation commit: `4aaa7e7` (`Persist update check failure and reminder-only states`).
