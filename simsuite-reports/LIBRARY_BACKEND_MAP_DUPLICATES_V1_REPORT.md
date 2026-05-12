# Library Backend Map and Duplicate Intelligence v1 Report

Date: 2026-05-12
Branch: `codex/library-backend-map-duplicates-v1`

## Pre-Implementation Worktree Classification

Current branch at start: `codex/library-cohesive-cozy-polish-v1`.

New branch created for this sprint: `codex/library-backend-map-duplicates-v1`.

Pre-existing dirty files found before backend work:

- `SESSION_HANDOFF.md`
- `docs/IMPLEMENTATION_STATUS.md`
- `src/screens/HomeScreen.tsx`
- `src/styles/globals.css`

These were recurring Home/status/global CSS changes from earlier work. They are unrelated to this backend duplicate sprint and will be left out of the commit unless a staged hunk is explicitly for this sprint's handoff/status notes.

## Backend Systems Map Summary

The full map is in `docs/LIBRARY_BACKEND_SYSTEMS_MAP.md`.

The Library backend is structured around:

- Tauri command registration in `src-tauri/src/lib.rs`;
- command wrappers in `src-tauri/src/commands/mod.rs`;
- SQLite schema and schema repair in `src-tauri/src/database/mod.rs`;
- scanning/indexing in `src-tauri/src/core/scanner/mod.rs`;
- file/package inspection in `src-tauri/src/core/file_inspector/mod.rs`;
- Library list/folder/detail queries in `src-tauri/src/core/library_index/mod.rs`;
- duplicate detection in `src-tauri/src/core/duplicate_detector/mod.rs`;
- watch/update source logic in `src-tauri/src/core/content_versions/mod.rs` and `watch_polling`;
- review queue logic in `src-tauri/src/core/rule_engine/mod.rs`.

## Systems Working

- Library roots/settings commands are registered and implemented.
- Scan/start/status commands are registered and implemented.
- Library list rows are paged when `limit` is provided.
- File detail is lazy and can resolve preview/thumbnail detail separately from row listing.
- Folder tree metadata is implemented from indexed file paths.
- Duplicate pairs are rebuilt after scans.
- Review queue and problem signals exist.
- Update/watch source state exists for Library rows and detail views.
- Explorer reveal/open folder command is implemented using real paths.

## Partial / Weak Systems

- Duplicate detection had only coarse stored types: `exact`, `filename`, and `version`.
- The Duplicates API exposed little evidence about why a pair was classified.
- Version-like matches were still stored under the duplicate table, which can make a version variant look like a cleanup duplicate unless the UI adds context.
- `list_library_folder_files` filters after loading a broader Library list, so it is weak for very large folders.
- True empty disk folders do not appear in folder metadata because the tree is built from file rows.
- The schema mirror `database/schema/simsuite-v1.sql` is stale compared with migrations plus `ensure_schema`.
- Large-library backend stress proof is still missing.

## Stale / Dead / Stubbed Systems

- `list_library_files_for_tree` remains registered but is weak because it removes pagination and can become unbounded. `get_folder_tree_metadata` is the better path for folder trees.
- AI classification code exists in the core tree, but this sprint does not use it and Library should not depend on it for truth claims.
- Some broader Downloads/Staging systems are outside this sprint and were not changed.

## Missing Systems

- True empty disk-folder metadata.
- Large-library synthetic stress proof.
- Live real-library thumbnail validation.
- Rich same-mod-family matching beyond current duplicate/version keys.
- Dependency proof.
- Missing mesh proof.
- Safe-delete proof.
- Duplicate cleanup actions.
- Update provider onboarding and official-source proof.

## Duplicate Detection Before

Before implementation, duplicate detection:

- inserted `exact` pairs for matching non-empty hashes;
- inserted `filename` pairs for case-insensitive filename matches when not already exact;
- inserted `version` pairs when a simple version token strip produced the same canonical key;
- returned pair identity fields, paths, creators, hashes, modified dates, sizes, stored type, and detection method;
- did not return explicit classification labels, confidence labels, evidence labels, or cautions.

## Planned Duplicate Detection Changes

The safest v1 path is to keep the database schema unchanged and compute clearer classifications/evidence at the duplicate pair API boundary.

Planned classifications:

- `exact_duplicate` for same file contents/hash proof.
- `possible_duplicate` for strong same-name comparison without exact-content proof.
- `possible_version_variant` for version-token comparisons.
- `same_mod_family`, `related_only`, `not_duplicate`, and `unknown` remain vocabulary for future backend work, not fully stored pair categories in v1.

Planned evidence labels:

- Same file contents.
- Same filename.
- Similar filename.
- Version clue found.
- Version differs.
- Size matches.
- Size differs.
- Creator matches.
- Creator differs.
- Creator unknown.
- Manual comparison needed.

## Performance Plan

No migration or all-pairs fuzzy matching is planned for v1.

The duplicate detector will continue to use bounded grouping keys:

- hash for exact matches,
- filename for filename matches,
- version-stripped canonical key for version variants.

Richer pair evidence is computed only for returned duplicate pairs, not for every Library row render.

## Implementation Results

- Created `docs/LIBRARY_BACKEND_SYSTEMS_MAP.md` with current command, database, scanner, inspection, folder, detail, signal, relationship, duplicate, update/watch, review queue, performance, and missing-feature maps.
- Kept the `duplicates` table schema unchanged. No migration was needed because v1 classification is computed for returned duplicate pairs.
- Added duplicate pair API fields:
  - `classification`
  - `classificationLabel`
  - `confidenceLabel`
  - `evidence`
  - `cautions`
- Added query-time duplicate classifications:
  - `exact_duplicate` for hash-backed same-file-content evidence;
  - `possible_duplicate` for same-name comparisons without exact-content proof;
  - `possible_version_variant` for version-token comparisons.
- Added evidence labels:
  - Same file contents
  - Content hash differs
  - Same filename
  - Similar filename
  - Version clue found
  - Version differs
  - Size matches / Size differs
  - Creator matches / Creator differs / Creator unknown
  - Detection method
- Added caution labels:
  - Compare before changing anything
  - This is not same-file-content proof
  - This may be another release of the same mod
  - Creator metadata is incomplete
- Fixed `get_file_detail` so `duplicatesCount` counts duplicate pairs, not distinct duplicate types.
- Made duplicate problem signals more cautious:
  - `proofLevel` is now `detected` for generic duplicate candidate signals;
  - label is `Possible duplicate`;
  - evidence is `Duplicate comparison rule matched`.
- Updated the TypeScript duplicate pair type and mock API data.
- Lightly updated Duplicates/Library boundary copy to show classification and evidence without adding cleanup actions or safe-delete claims.

## Validation Results

- `npm run build`: passed. Vite still reports the existing large chunk warning.
- `npx tsc --noEmit`: passed.
- `npm run test:unit`: passed, 22 files / 87 tests.
- `cargo fmt`: passed.
- `cargo check`: passed with existing Rust warnings.
- `cargo test`: passed, 227 tests.
- `cargo build --release`: passed with existing Rust warnings.
- `npm run test:rust`: passed, 227 tests.
- `npm run desktop:smoke:fixtures`: passed.
- `npm run desktop:proof:fixtures`: initially failed when run in parallel with smoke because both builds touched `dist/assets`; rerun sequentially passed with `DESKTOP_LIBRARY_PROOF_OK`.

Desktop proof output:

- `output/desktop/library-proof/2026-05-12T03-32-11-028Z`
- `output/desktop/library-proof/latest-summary.json`

Runtime proof notes:

- Proof summary recorded no `runtimeErrors`.
- It did record known Tauri reload callback console messages during proof navigation; these were non-fatal and the proof completed.

## Duplicate Classifications Now Supported

| Classification | When used | Safe wording |
| --- | --- | --- |
| `exact_duplicate` | Matching non-empty hashes on both files. | Exact duplicate / Same file contents. |
| `possible_duplicate` | Same filename/name comparison without exact-content proof. | Possible duplicate / Compare before changing anything. |
| `possible_version_variant` | Version token comparison or version clues. | Possible version variant / This may be another release of the same mod. |
| `same_mod_family` | Vocabulary only in v1; not persisted or emitted as a duplicate row yet. | Future related hint. |
| `related_only` | Vocabulary only in v1; same pack/folder remain relationship hints outside duplicate detection. | Related hint / Same pack / Same folder. |

## Tests Added or Updated

- Same content/hash returns `exact_duplicate` and `Same file contents` evidence.
- Same filename with different hash returns `possible_duplicate`, not exact proof.
- Version-token pair returns `possible_version_variant`.
- Same folder or same pack alone does not create a duplicate pair.
- Missing creator keeps a filename match at manual-review strength.
- File detail reports actual duplicate pair count instead of distinct duplicate type count.
- Duplicates screen mock/test data now includes classification/evidence fields.
- Preflight and inspector duplicate wording use safer comparison language.

## Performance Considerations

- No new full-library fuzzy matching was added.
- No thumbnail or preview data participates in duplicate classification.
- Classification/evidence is computed only for returned duplicate pairs from `list_duplicate_pairs`.
- Existing duplicate rebuild still groups by hash, filename, and version-stripped keys instead of global all-pairs matching.
- Large same-name/version groups can still grow pair counts, so a future large-library stress sprint should add synthetic performance proof.

## Database / Migration Notes

No migration was added.

Reason: the existing `duplicates` table can continue storing compact pair candidates (`exact`, `filename`, `version`), while the new v1 truth model is computed at query time. This avoids rewriting user data and keeps old duplicate rows loadable.

## What Could Not Be Verified

- Real user Library duplicate behavior on a large live collection was not tested.
- True empty disk-folder metadata remains unsupported because folder metadata is still file-row based.
- Live real-library thumbnail extraction was not part of this backend sprint.
- Same mod family matching beyond filename/version keys remains future work.
- No dependency, missing mesh, or safe-delete proof was attempted.

## Recommended Next Backend Sprints

Draft order:

1. Duplicate Intelligence v2 / real-world fixture hardening.
2. Large-library stress and backend query performance.
3. True empty disk-folder metadata in scanner/indexer.
4. Live real-library thumbnail validation and preview pipeline hardening.
5. Safe Action Preflight backend evidence model, without safe-delete claims.

## Unrelated Worktree Changes

The sprint started with unrelated dirty files:

- `SESSION_HANDOFF.md`
- `docs/IMPLEMENTATION_STATUS.md`
- `src/screens/HomeScreen.tsx`
- `src/styles/globals.css`

Only this sprint's documentation hunks should be staged from the docs. Home and unrelated global CSS work should remain unstaged.

## Commit

- Implementation commit: `89083c9d486edc823e10fea7a5a153e00773ead9`
- Follow-up docs hash commit: pending at the time this report section was written.
