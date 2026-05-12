# Library Duplicate Truth Engine v2 Report

Date: 2026-05-12

Branch: `codex/library-duplicate-truth-engine-v2`

## Worktree State

- Pre-existing unrelated files were present before this sprint: `SESSION_HANDOFF.md`, `docs/IMPLEMENTATION_STATUS.md`, `src/screens/HomeScreen.tsx`, and `src/styles/globals.css`.
- Generated local index/database files also changed during validation: `.cocoindex_code/cocoindex.db/mdb/data.mdb` and `.cocoindex_code/target_sqlite.db`.
- This sprint leaves unrelated Home/global CSS/status/generated hunks unstaged.
- Sprint files are limited to duplicate backend/API/frontend wording, duplicate research notes, backend map/report docs, fixture/proof updates, and focused tests.

## Research Findings

Research notes were added in `docs/SIMS_DUPLICATE_DETECTION_NOTES.md`.

Short version:

- exact duplicate language should be reserved for same-content evidence;
- same filename and version clues are comparison/review evidence;
- DBPF resource identity can guide future package fingerprints only when payload hashes are available;
- `.ts4script` can be fingerprinted as a ZIP-like archive in future, but SimSuite should not compute that during normal Library render.

Sources reviewed:

- Sims Mod Assistant on Mod The Sims
- DBPF format overview
- TSR duplicate files help
- The Sims 4 Modders Reference file types

## 1. Current Duplicate Pipeline

Before this sprint:

- `rebuild_duplicates` cleared the `duplicates` table after scan and inserted three stored row types:
  - `exact`: same non-empty file hash;
  - `filename`: same case-insensitive filename with different hash;
  - `version`: filenames that share a key after stripping version tokens.
- `get_duplicate_overview` counted all stored rows by type.
- `list_duplicate_pairs` read rows from `duplicates`, joined file identity/path/creator/hash/size, and computed query-time evidence/classification.
- Library row `hasDuplicate`, Library duplicate filters, Home/Library summary counts, and file detail duplicate counts treated all duplicate-table rows as duplicate-like.
- Frontend labels still exposed uncertain same-name rows as duplicate language in Library/Duplicates/Preflight contexts.

After this sprint:

- The table can still store exact, filename, and version comparison rows.
- Only exact rows with matching non-empty file hashes are user-facing duplicates.
- Same filename and version-token rows are non-duplicate review/comparison rows.
- Library duplicate counts and duplicate filters now count exact deterministic duplicate rows only.

## 2. Current Evidence Available

| Evidence | Available now | v2 decision |
| --- | --- | --- |
| Full file hash | Yes | Used as deterministic duplicate proof when non-empty and equal. |
| Package resource identifiers | Partial | Future package fingerprint input only. |
| Package resource payload hashes | No stored field | Not used; would be too expensive to compute during Library render/query. |
| Package resource count | Partial | Not enough for duplicate proof. |
| Script archive entries | Partial | Future script fingerprint input only. |
| Script archive entry hashes | No stored field | Not used in v2. |
| Filename tokens | Yes | Review evidence only. |
| Version tokens | Yes | Version review evidence only. |
| Creator metadata | Partial | Supporting evidence only; missing creator is common. |
| Source/root/folder paths | Yes | Context evidence only. |
| Installed vs inbox/download context | Partial | Future review evidence. |
| File size | Yes | Supporting evidence only. |
| Modified timestamp | Yes | Supporting evidence only. |

## 3. Classification Model

User-facing duplicate is binary:

- `isDuplicate = true` only when deterministic same-content proof exists.
- `isDuplicate = false` for filename, version, family, folder, pack, or weak metadata comparisons.

Implemented comparison kinds:

- `exact_file`: same non-empty full file hash.
- `version_review`: version-token comparison, not a duplicate.
- `name_match_review`: same filename without same-content proof, not a duplicate.
- `unknown`: manual review fallback.

Planned/future comparison kinds:

- `exact_package`: package fingerprint with sorted resource keys and payload hashes.
- `exact_script`: script archive fingerprint with sorted entry paths and payload hashes.
- `same_mod_family`: bounded family comparison, not a duplicate.
- `related_only`: same folder/pack/tray grouping, not a duplicate.

## 4. User-Facing Label Plan

- Exact hash rows now show `Duplicate` with `Same file contents`.
- Same-name rows now show `Name match`.
- Version-token rows now show `Version review`.
- Same family remains future `Same mod family`.
- Same pack/folder stay `Related hint` surfaces, not duplicate proof.
- `Possible duplicate` and `Duplicate candidate` are no longer user-facing labels in touched Library/Duplicates/Preflight surfaces.

## Backend Changes

- Added `is_duplicate` and `comparison_kind` to `DuplicatePair`.
- Reworked duplicate pair classification in `src-tauri/src/core/duplicate_detector/mod.rs`.
- Exact hash pairs now return:
  - `is_duplicate = true`
  - `comparison_kind = exact_file`
  - `classification = duplicate`
  - `classification_label = Duplicate`
  - evidence including `Same file contents`
- Filename rows now return:
  - `is_duplicate = false`
  - `comparison_kind = name_match_review`
  - `classification_label = Name match`
  - cautions explaining same filename is not same-content proof.
- Version rows now return:
  - `is_duplicate = false`
  - `comparison_kind = version_review`
  - `classification_label = Version review`
  - cautions explaining it may be another release of the same mod.
- Library summary, Home duplicate summary, row `hasDuplicate`, duplicate filter, and file-detail duplicate counts now use exact duplicate rows only.
- Fixed a SQL string spacing bug in the exact-only duplicate filter path found by `cargo test duplicate`.

## Frontend / API Changes

- Updated TypeScript duplicate types with `isDuplicate` and `comparisonKind`.
- Updated mock duplicate data to include:
  - one exact duplicate;
  - name-match review rows;
  - version-review rows.
- Updated Library row/grid/inspector/preflight wording from uncertain duplicate language to `Duplicate`, `Name match`, or `Version review`.
- Updated Duplicates screen copy so it is a comparison/review surface:
  - exact rows are under `Duplicates`;
  - same-name rows are under `Name matches`;
  - version rows are under `Version reviews`;
  - copy continues to say compare before changing anything.
- Updated desktop fixture data to include an exact-content duplicate pair for the Duplicates bridge proof.
- Updated desktop proof to route the Duplicates bridge through the exact-content fixture instead of MCCC/version-like data.

## Package / Script Fingerprint Decision

Package and script fingerprints were not implemented in v2.

Reason:

- The current backend does not store normalized package resource payload hashes.
- The current backend does not store normalized `.ts4script` archive entry payload hashes.
- Computing those fingerprints during ordinary Library listing, detail loading, or Duplicates route rendering would add the wrong performance cost.

Recommended future path:

- Compute package and script fingerprints during scan/indexing.
- Store compact signatures with schema/index support.
- Only then allow `exact_package` or `exact_script` duplicate proof.

## Database / Migration Notes

No migration was added.

The existing `duplicates` table remains usable because v2 reclassifies stored rows at query/API boundaries:

- `exact` rows can become true duplicates only with same non-empty hashes.
- `filename` rows become name-match reviews.
- `version` rows become version reviews.

This avoids rewriting user data while making user-facing duplicate language stricter.

## Performance Notes

- No full-library fuzzy matching was added.
- No all-pairs comparison across the whole Library was added.
- No thumbnail/preview loading was added for duplicate detection.
- No package/script parsing was added to ordinary Library or Duplicates render paths.
- Exact duplicate counts use indexed duplicate rows filtered by `duplicate_type = 'exact'`.
- Name/version comparison rows remain available for review without being counted as duplicates.

Remaining performance risk:

- Very large same-name/version groups can still create many stored comparison rows during scan.
- Large-library duplicate stress proof is still future work.

## Tests Added or Updated

Backend/Rust:

- Exact same hash returns `is_duplicate = true` and `comparison_kind = exact_file`.
- Same filename with different content returns `is_duplicate = false` and `comparison_kind = name_match_review`.
- Version-token rows return `is_duplicate = false` and `comparison_kind = version_review`.
- Missing creator/type does not raise duplicate certainty.
- File detail duplicate count only counts exact duplicate rows.
- Library duplicate filter only returns exact content duplicates.
- Backend labels do not produce unsafe duplicate wording.

Frontend/API:

- Duplicates screen labels distinguish Duplicates, Name matches, and Version reviews.
- Library rows/cards use deterministic duplicate wording.
- Inspector/preflight copy explains exact-content duplicate evidence.
- No user-facing `Possible duplicate` appears in touched Library/Duplicates/Preflight tests.

Proof:

- Fixture proof now includes an exact-content duplicate pair.
- Duplicates bridge proof opens the exact duplicate fixture and verifies Library focus context.

## Validation Results

Passed:

- `npm run build`
- `npx tsc --noEmit`
- `npm run test:unit`
- `cd src-tauri; cargo fmt`
- `cd src-tauri; cargo check`
- `cd src-tauri; cargo test`
- `cd src-tauri; cargo build --release`
- `npm run test:rust`
- `npm run desktop:proof:fixtures`
- `npm run desktop:smoke:fixtures`

Notes:

- Rust validation still prints existing warnings in older modules.
- Vite still prints the existing large chunk warning.
- The first desktop proof attempt failed because the proof still tried to open Duplicates from MCCC, which is now a version/update-style review target rather than an exact duplicate. The fixture/proof were corrected, then proof passed.

## Desktop Proof

Latest proof folder:

`output/desktop/library-proof/2026-05-12T11-32-16-213Z`

Important screenshots:

- `06-duplicates-bridge-exact.png`
- `07-updates-bridge-mccc.png`
- `01-library-selected-mccc.png`
- `05-library-preflight-detail-mccc.png`
- `library-responsive-1366x768.png`
- `library-responsive-1440x900.png`

Proof summary:

- `DESKTOP_LIBRARY_PROOF_OK`
- Duplicates bridge target: `Generic_Watch_Mod_v1.0.package`
- Duplicates route showed `Duplicate` / `Same file contents` for the exact fixture.
- Duplicates route also showed separate `Name matches` and `Version reviews` lanes.
- Runtime error count: 0 severe runtime errors.
- Browser log contained existing Tauri callback-id warnings during reload, but no proof-blocking runtime errors.

## What Could Not Be Verified

- Real user large-library duplicate behavior was not tested.
- Package resource payload fingerprints were not implemented or verified.
- Script archive entry fingerprints were not implemented or verified.
- Same-mod-family matching beyond filename/version keys remains future work.
- Large duplicate-group stress performance was not measured.
- True empty disk-folder metadata, live real-library thumbnail validation, and SQL-direct folder file optimization remain future Library backend work.

## Recommended Next Backend Sprints

1. Duplicate Truth Engine v3 / real-world fixture hardening: add scan-time package/script fingerprints and larger duplicate fixtures.
2. Large-library stress and backend query performance: measure list, folder, duplicate, and detail paths against synthetic large data.
3. SQL-direct folder content query optimization: replace broad list-and-filter folder loading.
4. True empty disk-folder metadata: represent real empty folders without fake file rows.
5. Live real-library thumbnail validation and preview pipeline hardening.
6. Stale schema mirror cleanup and `list_library_files_for_tree` deprecation/removal if still unbounded.
7. Safe Action Preflight backend evidence model, still without safe-delete claims.
8. Updates provider onboarding architecture.
9. Staging backend cleanup.
10. Dependency/missing-mesh research spike only after deterministic systems are stable.

## Docs Updated

- `docs/SIMS_DUPLICATE_DETECTION_NOTES.md`
- `docs/LIBRARY_BACKEND_SYSTEMS_MAP.md`
- `simsuite-reports/LIBRARY_DUPLICATE_TRUTH_ENGINE_V2_REPORT.md`
- `SESSION_HANDOFF.md`
- `docs/IMPLEMENTATION_STATUS.md`

## Commit

Implementation commit:

- `fbb7945` - Make Library duplicate detection deterministic

This report was updated after the implementation commit so the final branch history also contains a docs-only hash record commit.
