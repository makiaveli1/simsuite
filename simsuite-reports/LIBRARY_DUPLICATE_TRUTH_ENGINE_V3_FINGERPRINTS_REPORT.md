# Library Duplicate Truth Engine v3 Fingerprints Report

Date: 2026-05-12
Branch: codex/library-duplicate-truth-engine-v3-fingerprints

## Audit Note Before Implementation

### Worktree state

- Started from `codex/trust-boundaries-automation-readiness-v1`.
- Created branch `codex/library-duplicate-truth-engine-v3-fingerprints`.
- Pre-existing unrelated worktree changes were present in `.cocoindex_code/*`, `src/screens/HomeScreen.tsx`, `src/styles/globals.css`, `SESSION_HANDOFF.md`, and `docs/IMPLEMENTATION_STATUS.md`.
- This sprint will not overwrite or stage unrelated Home/global CSS/generated changes.
- If `SESSION_HANDOFF.md` and `docs/IMPLEMENTATION_STATUS.md` are updated, only this sprint's new notes should be staged.

### 1. Current duplicate proof model

- Full file hash proof is the only implemented duplicate proof before this sprint.
- A user-facing `Duplicate` requires two real joined file rows, distinct file IDs, non-empty matching normalized hashes, non-empty paths, and distinct canonical paths.
- Package fingerprints are not implemented yet.
- Script fingerprints are not implemented yet.
- Name matches are `Name match` review rows, not duplicates.
- Version-token matches are `Version review` rows, not duplicates.
- Same folder, same pack, same mod family, and related hints are not duplicate proof.
- Library row `hasDuplicate`, Library summary duplicates, Home/Library summary counts, Duplicates exact counts, inspector duplicate counts, and Preflight duplicate evidence currently count exact deterministic duplicates only.

### 2. Package fingerprint feasibility

| Question | Finding |
| --- | --- |
| Can the current DBPF parser enumerate resource records safely? | Yes. It parses the DBPF header and decompressed index with record-count clamping. |
| Can it access decompressed resource payload bytes safely? | Yes. `read_record_bytes` validates offsets/sizes and uses bounded zlib/legacy decompression. |
| Can it hash resource payloads without unsafe memory use? | Yes, with the existing `MAX_RESOURCE_BYTES` cap per resource and no stored payloads. |
| Can it handle corrupt records without freezing? | Mostly. Existing guards skip or error on bad records; fingerprinting must refuse partial/corrupt packages instead of proving duplicates. |
| Can it ignore container ordering? | Yes. Resource entries can be sorted by stable resource identity before final hashing. |
| Can it avoid using resource keys alone as duplicate proof? | Yes. The fingerprint must include payload hashes, not resource IDs alone. |
| Is scan-time computation safe enough? | Implement now, scan-time only, cached for unchanged files. It may add scan cost for changed packages because all package payloads are hashed. |
| Is a migration needed? | Yes. Duplicate grouping needs indexed persisted fields instead of JSON scans. |

Decision: implement package fingerprints now as deterministic exact proof when every parsed resource has a safely read payload hash. Corrupt, partial, oversized, or unreadable packages do not get duplicate proof.

### 3. Script fingerprint feasibility

| Question | Finding |
| --- | --- |
| Can current code inspect `.ts4script` as ZIP/archive? | Yes. `inspect_ts4script` already uses the `zip` crate. |
| Can it read entry paths and payload bytes safely? | Yes, with entry count and byte caps. |
| Can it ignore ZIP timestamps/order/metadata? | Yes. Fingerprint normalized entry paths and payload hashes in sorted order. |
| Can it skip directory entries and irrelevant OS metadata? | Yes. Directory entries, `__MACOSX/`, and `.DS_Store` can be ignored. |
| Can it handle corrupt archives safely? | Yes. Invalid/corrupt archives should produce no fingerprint and no duplicate proof. |
| Is scan-time computation safe enough? | Implement now, scan-time only, cached for unchanged files. |
| Is a migration needed? | Yes, shared content-fingerprint fields are needed for indexed duplicate grouping. |

Decision: implement script fingerprints now as deterministic exact proof from normalized archive entry paths plus entry payload hashes.

### 4. Persistence decision

Use dedicated fields on `files`:

- `content_fingerprint`
- `content_fingerprint_kind`
- `content_fingerprint_version`
- `content_fingerprint_status`
- `content_fingerprint_error`

Add an index on `(content_fingerprint_kind, content_fingerprint)` for duplicate grouping. Do not store payload bytes, entry names in logs, real paths in diagnostics, or user file contents.

Keep the existing `duplicates.duplicate_type = 'exact'` value for package/script exacts and distinguish proof source with `detection_method` values such as `package_fingerprint_v1` and `script_fingerprint_v1`. This avoids a destructive duplicates-table CHECK rewrite.

### 5. User experience impact plan

Users may see more true duplicates because SimSuite can compare package or script contents, not only the outer file hash. Files still will not be deleted, moved, quarantined, or changed. Name and version matches remain review-only, and exact duplicates are still comparison evidence, not a safe-delete instruction.

## Final Report

### What was audited

- `src-tauri/src/core/duplicate_detector/mod.rs`
- `src-tauri/src/core/file_inspector/mod.rs`
- `src-tauri/src/core/scanner/mod.rs`
- `src-tauri/src/core/library_index/mod.rs`
- database migrations and runtime schema repair
- Duplicates route wording and existing frontend API shape
- duplicate stress/report docs and trust-boundary docs

### Fingerprint feasibility

Package fingerprints were safe to implement with the current DBPF parser after one parser bug was fixed: parsed DBPF index values were being read but not assigned into `DbpfRecord`. The new package proof uses resource type, group, instance IDs, and bounded resource payload hashes. Resource keys alone are still not duplicate proof.

Script fingerprints were safe to implement with the existing ZIP archive inspection boundary. The new script proof uses normalized archive entry paths and payload hashes, sorted so ZIP order and metadata do not matter.

### What changed

- Added scan-time content fingerprinting for `.package` and `.ts4script` files.
- Added persisted `files.content_fingerprint*` fields and an index for duplicate grouping.
- Bumped scanner cache version to `scanner-v21` so existing package/script rows are re-inspected once.
- Updated duplicate rebuild to create exact rows from matching package/script fingerprints.
- Updated duplicate API classification to return `exact_package` and `exact_script` with evidence labels.
- Updated Library exact duplicate counts/filters/detail counts to include validated package/script fingerprint matches.
- Kept filename and version review generation separate and capped.

### What this means for the user

SimSuite can now find more real duplicates after a rescan. It can catch some package or script files that have different outer file hashes but the same internal contents. It still does not move, delete, quarantine, replace, or auto-sort anything. Files with only matching names, version clues, folder location, pack grouping, or family hints still need manual review.

### Trust / safety boundary

SimSuite still treats `Duplicate` as proof only, not a safe-delete instruction. Exact duplicates now require validated same file contents, same package contents, or same script contents. Name matches, version reviews, same folder, same pack, and same mod family remain review-only. AI is not deciding anything, and no cleanup/delete/quarantine behavior was added.

### Duplicate rule after this sprint

User-facing `Duplicate` can be shown only when one of these deterministic proofs is true:

- two real distinct file rows have matching non-empty full file hashes and distinct canonical paths;
- two real distinct `.package` rows have matching available package content fingerprints and distinct canonical paths;
- two real distinct `.ts4script` rows have matching available script content fingerprints and distinct canonical paths.

Everything else remains non-duplicate comparison language.

### Package fingerprint behavior

Implemented. The package fingerprint:

- parses DBPF records at scan time;
- sorts records by resource type, group, and instance IDs;
- hashes each safely read/decompressed resource payload;
- includes payload hashes in the final fingerprint;
- refuses duplicate proof for corrupt, partial, empty, unreadable, or oversized packages.

### Script fingerprint behavior

Implemented. The script fingerprint:

- reads `.ts4script` archives at scan time;
- ignores directory entries, ZIP entry order, ZIP timestamps, `__MACOSX/`, and `.DS_Store`;
- hashes normalized entry paths plus entry payload hashes;
- refuses duplicate proof for corrupt archives, duplicate normalized paths, empty archives, and archives beyond entry/byte caps.

### Persistence / migration notes

Added dedicated file metadata fields:

- `content_fingerprint`
- `content_fingerprint_kind`
- `content_fingerprint_version`
- `content_fingerprint_status`
- `content_fingerprint_error`

Added `idx_files_content_fingerprint` on `(content_fingerprint_kind, content_fingerprint)`.

No duplicate-table rewrite was needed. Fingerprint exacts still use `duplicate_type = 'exact'`, and the proof source is stored in `detection_method` as `package_fingerprint_v1` or `script_fingerprint_v1`.

### Evidence labels

- `exact_file`: `Same file contents`
- `exact_package`: `Same package contents`
- `exact_script`: `Same script contents`
- `name_match_review`: `Name match`
- `version_review`: `Version review`

Forbidden wording such as `Possible duplicate`, cleanup claims, safe-delete claims, dependency claims, and broken-content claims was not introduced.

### Performance notes

Fingerprinting runs during scan or scan refresh, not during Library row render, Duplicates render, folder browsing, or ordinary query paths. Fingerprints are stored as small metadata strings, not payloads. Duplicate grouping uses indexed fingerprint fields and existing bounded non-exact review generation.

Package fingerprinting can add scan cost for changed packages because payloads are hashed. Existing unchanged rows are reused after the `scanner-v21` refresh has populated fingerprints.

### Files changed

- `src-tauri/src/core/file_inspector/mod.rs`: added package/script fingerprint extraction and fixed DBPF record value assignment.
- `src-tauri/src/core/scanner/mod.rs`: computes and caches content fingerprints at scan time.
- `src-tauri/src/core/duplicate_detector/mod.rs`: groups exact package/script fingerprint matches as deterministic duplicates.
- `src-tauri/src/core/library_index/mod.rs`: counts exact package/script fingerprint matches in Library duplicate totals and filters.
- `src-tauri/src/database/mod.rs` and `database/migrations/0001_initial.sql`: added persisted fingerprint metadata and index repair.
- `src/screens/DuplicatesScreen.tsx`: adjusted exact duplicate detail copy so file, package, and script content proof all fit.
- `docs/LIBRARY_BACKEND_SYSTEMS_MAP.md` and `docs/SIMS_DUPLICATE_DETECTION_NOTES.md`: recorded the new proof model.
- `SESSION_HANDOFF.md` and `docs/IMPLEMENTATION_STATUS.md`: added sprint handoff/status notes.

### Tests

- `npm run build`: passed. Existing Vite chunk-size warning remains.
- `npx tsc --noEmit`: passed.
- `npm run test:unit`: passed, `23` files / `88` tests.
- `cargo fmt`: passed.
- `cargo check`: passed with existing unused-code warnings.
- `cargo test`: passed, `251` passed / `2` ignored.
- `cargo build --release`: passed with existing unused-code warnings.
- `npm run test:rust`: passed, `251` passed / `2` ignored.
- `cargo test --manifest-path src-tauri/Cargo.toml fingerprint -- --nocapture`: passed, `12` focused fingerprint tests.
- `cargo test --manifest-path src-tauri/Cargo.toml duplicate_library_counts_include_exact_package_fingerprints -- --nocapture`: passed.
- `cargo test --manifest-path src-tauri/Cargo.toml initialize_upgrades_existing_files_table_before_creating_download_index -- --nocapture`: passed.

### Stress proof

- `npm run test:library:stress`: passed.
- 10,000-row Library stress timings were reported for list, search, filter, sort, folder tree metadata, folder direct/recursive listing, empty folder listing, relationship peer counts, file detail, duplicate overview, and preview diagnostics.
- The duplicate review group stress reported `5002` rows, `4001` review pairs, and `35817 ms`. It remained bounded, but it is still comparatively heavy and should stay on the backend watch list.

### Desktop/runtime proof

- `npm run desktop:proof:fixtures`: timed out after about five minutes, so it is not counted as passed. It did create screenshots under `output/desktop/library-proof/2026-05-12T22-50-59-231Z`, including the duplicate bridge screenshot, but the proof runner did not finish cleanly.
- `npm run desktop:smoke:fixtures`: built and launched the release app, then failed in the Downloads inbox fixture path with `Wrong number of parameters passed to query. Got 18, needed 17` and timed out waiting for `MCCC_Update_Test`.
- The smoke failure was not in the duplicate fingerprint path. It is recorded as a runtime proof limitation rather than fixed here because this sprint must not broaden into Downloads/Staging work.

### What could not be verified yet

- Real user package/script libraries were not scanned.
- Package fingerprint coverage on every Sims 4 DBPF variant is not proven.
- Scan-time cost on a real large Mods folder is not measured yet.
- Package/script fingerprints do not prove safe deletion, dependencies, missing meshes, broken CC, or update state.
- Desktop proof did not complete cleanly, and desktop smoke is currently blocked by an existing Downloads inbox fixture/query issue outside this sprint.

### Next backend recommendation

Next: Staging backend safety/readiness audit, or Auto Sorting Suggested Plan v1 as preview-only with no file movement unless the user confirms. If real-library traces later show relationship counts are broad, prioritize relationship count SQL aggregation/cache v2 first.

### Docs updated

- `simsuite-reports/LIBRARY_DUPLICATE_TRUTH_ENGINE_V3_FINGERPRINTS_REPORT.md`
- `docs/LIBRARY_BACKEND_SYSTEMS_MAP.md`
- `docs/SIMS_DUPLICATE_DETECTION_NOTES.md`
- `SESSION_HANDOFF.md`
- `docs/IMPLEMENTATION_STATUS.md`

### Unrelated worktree changes

Pre-existing `.cocoindex_code/*`, Home screen, global CSS, and unrelated status/handoff changes were left alone.

### Commit

Pending final commit.

### Final honest verdict

Partially verified: fingerprint work passed tests and backend stress proof, but desktop proof/smoke and real-library proof were limited.
