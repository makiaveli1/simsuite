# Desktop Proof Downloads Query Fix v1 Report

Date: 2026-05-13
Branch: codex/desktop-proof-downloads-query-fix-v1

## Audit Note Before Implementation

### Worktree state

- Started from `codex/library-duplicate-truth-engine-v3-fingerprints`.
- Created branch `codex/desktop-proof-downloads-query-fix-v1`.
- Pre-existing unrelated worktree changes were present in `.cocoindex_code/*`, `src/screens/HomeScreen.tsx`, `src/styles/globals.css`, `SESSION_HANDOFF.md`, and `docs/IMPLEMENTATION_STATUS.md`.
- This sprint will not overwrite or stage unrelated Home/global CSS/generated changes.
- If `SESSION_HANDOFF.md` and `docs/IMPLEMENTATION_STATUS.md` are updated, only this sprint's new notes should be staged.

### 1. Runtime failure map

- `npm run desktop:proof:fixtures` timed out during the v3 fingerprint sprint after creating screenshots through `06-duplicates-bridge-exact.png`.
- The timed-out run directory was `output/desktop/library-proof/2026-05-12T22-50-59-231Z`.
- That run did not write `summary.json`, so the exact final wait is not recorded in the failed folder.
- `output/desktop/library-proof/latest-summary.json` still points to an earlier successful proof run, not the timed-out run.
- `npm run desktop:smoke:fixtures` built and launched the release app, then failed in the Downloads inbox path with `Wrong number of parameters passed to query. Got 18, needed 17`, followed by a timeout waiting for the fixture item `MCCC_Update_Test`.
- The smoke failure is separate from duplicate classification. It happens before the Downloads inbox row appears.

### 2. SQL mismatch diagnosis

- Failing area: `src-tauri/src/core/downloads_watcher/mod.rs`, `ingest_processed_source`.
- The Downloads ingest path prepares an `INSERT INTO files` statement with the old pre-fingerprint column shape.
- The shared scanner helper `scanner::insert_parsed_file` now writes content fingerprint metadata too.
- Expected fix: update the Downloads ingest `files` insert statement to match the current scanner file insert shape.
- The mismatch is a runtime query/params contract bug caused by the v3 fingerprint schema/helper change; it is not a fixture selector issue.

### 3. Proof timeout diagnosis

- Screenshots show the Library proof got through the duplicate bridge before the external command timeout.
- No failed-run `summary.json` was produced, which makes the exact last proof wait unavailable.
- The proof script itself writes the summary in `finally`, so the timeout likely killed the wrapper/node process before orderly cleanup.
- The earlier successful summary showed only Tauri callback reload warnings and no severe runtime errors.
- After the Downloads query fix, proof must be rerun to see whether it now completes or still has a separate wait issue.

### 4. User impact plan

Users should not hit a broken Downloads inbox flow when newly detected Downloads files are indexed after the fingerprint metadata change. The desktop proof and smoke lanes should become reliable again so future backend work has real runtime evidence.

## Final Report

### What was audited

- Desktop proof output, especially the incomplete `output/desktop/library-proof/2026-05-12T22-50-59-231Z` run and the current `latest-summary.json`.
- Desktop proof/smoke scripts under `scripts/desktop/`.
- Downloads inbox fixture setup through `run-tauri-webdriver.ps1` and the WebDriver smoke script.
- Downloads ingest backend path in `src-tauri/src/core/downloads_watcher/mod.rs`.
- Current scanner insert helper in `src-tauri/src/core/scanner/mod.rs`.
- Duplicate fingerprint runtime paths only enough to confirm this sprint did not change duplicate truth behavior.

### Root cause

- The smoke SQL error was caused by `ingest_processed_source` in `downloads_watcher`.
- That path still prepared the old `INSERT INTO files` statement with 17 placeholders.
- The shared scanner helper now inserts the v3 fingerprint fields too: `content_fingerprint`, `content_fingerprint_kind`, `content_fingerprint_version`, `content_fingerprint_status`, and `content_fingerprint_error`.
- The Downloads fixture hit this path while indexing the staged `MCCC_Update_Test` archive, so ingestion failed before the row could appear.
- The `MCCC_Update_Test` timeout was secondary: the fixture item was not visible because the backend query failed first.
- The previous Library proof timeout did not produce a failed-run `summary.json`. After the Downloads fix and a long enough harness timeout, the proof command completed cleanly, so no separate proof selector bug was confirmed in this pass.

### What changed

- Updated the Downloads processed-source file insert SQL to match the current scanner `files` insert shape, including all content fingerprint columns and placeholders.
- Added a focused Rust regression test that ingests a staged Downloads package item through the processed-source path and asserts the inserted Downloads file row has fingerprint status metadata.
- Did not change duplicate classification rules, frontend copy, Tauri command contracts, schema, migrations, or desktop proof selectors.

### What this means for the user

Downloads and Inbox-style runtime paths should no longer break when a downloaded Sims archive is indexed after the duplicate fingerprint work. Desktop proof and smoke are passing again, so future work has a reliable runtime check. This does not move, delete, quarantine, replace, auto-sort, or change any user mod files.

### Trust / safety boundary

No auto-fix, auto-update, quarantine, delete, or safe-delete behavior was added. Duplicate fingerprints remain evidence for review and duplicate comparison only; they are not cleanup instructions. Downloads/update behavior remains review-first, and any future file-changing workflow still needs preview and user confirmation.

### Downloads / Inbox fix

- Fixed `src-tauri/src/core/downloads_watcher/mod.rs`.
- The corrected insert now includes the v3 fingerprint fields expected by `scanner::insert_parsed_file`.
- Regression coverage: `processed_download_ingest_uses_current_file_insert_shape`.
- Desktop smoke now reaches the expected success condition instead of throwing the query parameter mismatch or timing out on `MCCC_Update_Test`.

### Desktop proof fix

- No proof script change was needed.
- Re-running `npm run desktop:proof:fixtures` after the backend fix completed with `DESKTOP_LIBRARY_PROOF_OK`.
- Latest proof output: `output/desktop/library-proof/2026-05-12T23-41-00-611Z`.
- The proof summary reports `ok: true`, duplicate bridge and updates bridge runtime error counts of `0`, and only existing Tauri callback reload warnings in browser logs.

### Duplicate fingerprint regression check

- Duplicate truth behavior was preserved.
- Existing Rust tests still cover exact hash, exact package fingerprint, exact script fingerprint, name-match review, version review, malformed exact rows, and exact-only Library duplicate counts.
- Desktop proof still shows the Duplicates bridge and exact duplicate preflight evidence using review-first wording.

### Files changed

- `src-tauri/src/core/downloads_watcher/mod.rs` - aligned Downloads ingestion with the current `files` insert shape and added regression coverage.
- `simsuite-reports/DESKTOP_PROOF_DOWNLOADS_QUERY_FIX_V1_REPORT.md` - recorded the audit, fix, proof results, and trust boundary.
- `SESSION_HANDOFF.md` - added this sprint handoff note.
- `docs/IMPLEMENTATION_STATUS.md` - added this sprint status note.

### Tests

- `cargo test --manifest-path src-tauri/Cargo.toml processed_download_ingest_uses_current_file_insert_shape -- --nocapture`: passed.
- `cargo fmt`: passed.
- `npm run build`: passed; existing Vite chunk-size warning remains.
- `npx tsc --noEmit`: passed.
- `npm run test:unit`: passed (`23` files, `88` tests).
- `cargo check`: passed with existing unused-code warnings.
- `cargo test`: passed (`252` passed, `2` ignored).
- `cargo build --release`: passed with existing unused-code warnings.
- `npm run test:rust`: passed (`252` passed, `2` ignored).

### Desktop/runtime proof

- `npm run desktop:smoke:fixtures`: passed against `src-tauri/target/release/simsuite.exe`.
- `npm run desktop:proof:fixtures`: passed and wrote `DESKTOP_LIBRARY_PROOF_OK`.
- Latest proof summary: `output/desktop/library-proof/latest-summary.json`.
- Screenshot/proof folder: `output/desktop/library-proof/2026-05-12T23-41-00-611Z`.

### What could not be verified

- The exact final wait in the old timed-out proof run cannot be known because that run was killed before writing `summary.json`.
- Real user Downloads folders and real user mod archives were not scanned.
- Existing Rust warnings and the Vite chunk-size warning remain unchanged.

### Recommended next sprint

Next implementation focus: Staging backend safety/readiness audit. Auto Sorting should still start as a preview-only suggested-plan workflow with no file movement unless explicitly confirmed later.

### Docs updated

- `simsuite-reports/DESKTOP_PROOF_DOWNLOADS_QUERY_FIX_V1_REPORT.md`
- `SESSION_HANDOFF.md`
- `docs/IMPLEMENTATION_STATUS.md`

### Unrelated worktree changes

- Pre-existing `.cocoindex_code/*`, `src/screens/HomeScreen.tsx`, `src/styles/globals.css`, and unrelated older status/handoff hunks were left alone.
- Only this sprint's relevant code, report, and new status/handoff notes should be staged.

### Commit hashes

- Pending before commit; the final assistant response records the committed hash.

### Final honest verdict

Verified: desktop proof recovery and Downloads query fix v1 is working for the tested paths.
