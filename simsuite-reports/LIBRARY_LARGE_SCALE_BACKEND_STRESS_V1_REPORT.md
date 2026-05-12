# Library Large-Scale Backend Stress v1 Report

Date: 2026-05-12
Branch: codex/library-large-scale-backend-stress-v1

## Audit Note Before Implementation

### Worktree state

- Started from `codex/library-thumbnail-preview-pipeline-v1`.
- Created branch `codex/library-large-scale-backend-stress-v1`.
- Pre-existing unrelated worktree changes were present in `.cocoindex_code/*`, `src/screens/HomeScreen.tsx`, `src/styles/globals.css`, `SESSION_HANDOFF.md`, and `docs/IMPLEMENTATION_STATUS.md`.
- This sprint will not overwrite or stage unrelated Home/global CSS/generated changes.
- If `SESSION_HANDOFF.md` and `docs/IMPLEMENTATION_STATUS.md` are updated, only this sprint's new notes should be staged.

### 1. Current stress coverage

| Area | Current coverage before this sprint | Classification |
| --- | --- | --- |
| 1,000 rows | Existing folder stress test covers 1,076 rows. | covered |
| 5,000 rows | No committed 5,000 row Library backend stress test found. | missing |
| 10,000 rows | No committed 10,000 row Library backend stress test found. | missing |
| Large folder with many direct files | Existing 1,001 direct-file folder test. | partially covered |
| Deep nested folders | Small direct/recursive tests plus 75 nested files in the 1,076-row test. | partially covered |
| Many empty folders | True empty folder metadata tests exist, but not under large folder counts. | partially covered |
| Mixed Mods and Tray roots | Covered by small root/empty-folder tests, not large scale. | partially covered |
| Long paths | Not meaningfully stressed. | missing |
| Missing metadata | Small cases exist, not large scale. | partially covered |
| Duplicate-heavy same-name groups | Duplicate correctness tests exist, not large same-name stress. | missing |
| Version-heavy groups | Duplicate correctness tests exist, not large version-key stress. | missing |
| Relationship-heavy same-pack groups | Small relationship tests exist. | partially covered |
| List search/filter/sort under load | 1,076-row folder query test, not broad list 5,000+. | partially covered |
| Folder tree counts under load | Not covered beyond small/medium fixture tests. | missing |
| Folder direct file listing under load | 1,076-row folder test. | partially covered |
| Relationship peer counts under load | Not covered at 5,000+. | missing |

### 2. Current query risk map

| Path | Audit finding | Risk classification |
| --- | --- | --- |
| `list_library_files` | SQL-paged for rows, but relationship peer counts load the full filtered row set for count grouping. | needs measurement |
| `list_library_folder_files` | SQL-direct and paged; relies on source/depth/path scope and existing indexes. | safe enough for now, needs large proof |
| `get_folder_tree_metadata` | Loads folder metadata rows and metadata-only file rows, then aggregates tree counts in Rust. | needs measurement |
| `list_library_files_for_tree` | Compatibility endpoint capped at 5,000 rows and preview-light. | safe enough for now |
| Relationship peer counts | Batched rather than per-row N+1, but still broad across filtered rows. | needs measurement |
| Duplicate overview/counts | Exact duplicate SQL guardrails are strict. | safe enough for now |
| Duplicate rebuild grouping | Exact hash insert is SQL-based; filename/version review pair generation is all-pairs per group. | confirmed issue risk |
| File detail | Single-row detail with lazy selected-file preview hydration. | safe enough for now |
| Preview diagnostics | Explicit aggregate diagnostic path, not part of ordinary browsing. | safe enough for now |
| Folder tree count aggregation | Metadata-only but broad by design. | needs measurement |

### 3. Stress harness design

- Use deterministic in-memory SQLite tests.
- Generate synthetic rows only; no real Mods/Tray files, thumbnails, or private paths.
- Required dataset target: 10,000 Library rows if test time remains reasonable.
- Include Mods and Tray rows, large direct folders, nested folders, empty folder metadata, long names/paths, missing metadata, relationship-heavy bundle groups, preview/no-preview rows, packages/scripts/tray-like rows.
- Print timing summaries through `--nocapture`; keep timings informational unless a threshold is extremely conservative.
- Keep generated data inside test transactions and do not commit generated outputs.

### 4. Planned implementation

- Add a 10,000-row Library backend stress fixture in Rust tests.
- Add timing/proof for list, search, filter, sort, folder tree, folder direct/recursive listing, empty folder listing, file detail, duplicate overview, preview diagnostics, and relationship-heavy listing.
- Add a bounded guard for non-exact duplicate review pair generation if the current all-pairs logic is confirmed in code.
- Add a focused duplicate group stress test proving name/version review rows stay bounded and exact duplicate counts remain exact-only.
- Add an npm stress command only if it remains stable and useful.

## Final Report

### What was audited

- Library query paths in `src-tauri/src/core/library_index/mod.rs`: paged list, search, filters, sort, folder tree metadata, folder file listing, legacy tree listing, file detail, preview inclusion, preview diagnostics, duplicate counts, and relationship peer counts.
- Duplicate stress paths in `src-tauri/src/core/duplicate_detector/mod.rs`: exact hash duplicate insertion, filename review grouping, version review grouping, existing pair handling, and duplicate overview.
- Database/index posture from current migrations and runtime schema repair: files source/depth/path indexes, `library_folders` indexes, and duplicate pair indexes.
- Desktop proof and smoke scripts to confirm the app still opens Library, folder view, empty-folder proof, duplicate bridge, updates bridge, and thumbnail diagnostics.

### Current stress risks

- Before this sprint, the committed backend stress proof stopped around 1,076 folder rows.
- Folder tree count aggregation was metadata-only but not proven at 5,000 to 10,000 rows.
- Relationship peer counts were batched, not N+1, but still needed proof against a larger filtered result set.
- Duplicate filename/version review generation could grow very large inside one same-name or same-version-key group.
- Real user libraries were still not measured; this sprint uses deterministic synthetic data only.

### Stress harness

- Added an ignored, opt-in Rust stress test with 10,000 synthetic Library rows.
- Dataset shape:
  - 5,000 Mods package rows in one large direct folder.
  - 2,000 nested Mods package rows.
  - 1,500 relationship-heavy Mods rows sharing the same pack/folder context.
  - 1,000 Tray-like rows.
  - 500 script rows with missing creator/subtype metadata.
  - 150 empty folder metadata rows.
  - long filenames/paths, preview/no-preview rows, Mods and Tray roots, and mixed content kinds.
- Added an ignored duplicate stress test with 5,002 rows:
  - 2,500 same-filename rows with different hashes.
  - 2,500 version-token rows with different hashes.
  - 2 exact same-hash rows.
- Added `npm run test:library:stress` so the large synthetic stress checks can be run explicitly without slowing normal test runs.

### What changed

- Added the 10,000-row Library backend stress harness and informational timing output.
- Fixed a real SQL construction bug found by the stress test: `RecentlyModified` and `HasUpdatesFirst` sort SQL could generate `ORDER BYCASE` without a space.
- Wrapped duplicate rebuild work in a transaction.
- Capped non-exact filename and version review pair generation at 2,000 pairs per candidate group.
- Kept exact duplicate truth unchanged: deterministic same-content proof still drives user-facing duplicate counts.
- Updated the backend systems map and sprint report with the new stress coverage and remaining risks.

### What this means for the user

SimSuite now has stronger proof that Library browsing, folder counts, relationship hints, and duplicate review data still behave with thousands of files. This does not change or delete user files. It mostly adds backend stress checks and one safety guard so very large same-name/version groups are less likely to overload duplicate review data.

### Measurements

Command: `npm run test:library:stress`

Dataset: 10,000 synthetic Library rows.

Informational timing summary from the local run:

| Operation | Elapsed |
| --- | ---: |
| Library first page | 103 ms |
| Library search | 18 ms |
| Library filter | 17 ms |
| Library sort | 110 ms |
| Folder tree metadata | 119 ms |
| Large direct folder page | 80 ms |
| Recursive folder page | 67 ms |
| Empty folder listing | 1 ms |
| Relationship-heavy page | 29 ms |
| File detail | 4 ms |
| Duplicate overview | 0 ms |
| Preview diagnostics | 73 ms |

Dataset: 5,002 synthetic duplicate stress rows.

- Duplicate review group stress produced 4,001 stored pairs:
  - `1` exact duplicate pair.
  - `2,000` filename review pairs.
  - `2,000` version review pairs.
- Elapsed time was about `19,301 ms`.
- The duplicate test is intentionally opt-in because it is heavier than normal unit coverage.

All timings are informational because this Windows desktop environment is not a stable performance lab. The assertions focus on bounded paging, correct counts, controlled preview payloads, and exact-only duplicate truth.

### Folder tree findings

- Folder tree metadata handled the 10,000-row synthetic dataset with 150 empty folders.
- The stress test verifies Mods and Tray roots, large direct folders, nested folders, empty folders, and aggregate counts.
- The current folder tree path still performs metadata/count aggregation for the tree, but the measured synthetic run did not show runaway behavior.
- Real-library folder tree behavior with 10,000+ files and unusual path layouts still needs live validation later.

### Relationship count findings

- Relationship counts remained batched and truthful for a 1,500-row same-pack/same-folder filtered page.
- The test confirmed the visible relationship-heavy rows receive same-pack/same-folder peer counts without dependency claims.
- A dedicated SQL aggregation/cache pass remains a good future sprint if real libraries show repeated broad count cost.

### Duplicate group findings

- Exact duplicate counts remained exact-only under stress.
- Same-name rows and version-token rows remained review/comparison data, not duplicates.
- Non-exact review pair generation is now bounded per candidate group.
- Package/script fingerprints were not added.
- Exact same-hash groups can still grow if a user has many byte-identical files with the same hash; that is truthful duplicate data, but very large exact groups may need a future presentation/performance pass.

### Preview behavior under stress

- The Library stress test verifies `include_previews=false` keeps list and folder rows preview-light.
- Folder tree metadata does not load preview payloads.
- Preview diagnostics remain explicit and aggregate-only.
- No thumbnail parsing or real user preview data is part of the stress harness.

### Database / migration notes

- No migration was added.
- Existing files, folder metadata, and duplicate indexes were enough for this synthetic stress pass.
- The only runtime query fix was the sort SQL spacing bug.
- The duplicate cap is an in-code guard for generated review pairs, not a schema change.

### Files changed

- `package.json`: added `npm run test:library:stress`.
- `src-tauri/src/core/library_index/mod.rs`: added the 10,000-row Library stress test and fixed sort SQL spacing.
- `src-tauri/src/core/duplicate_detector/mod.rs`: added transaction-wrapped rebuild, bounded non-exact review pair generation, and duplicate stress coverage.
- `docs/LIBRARY_BACKEND_SYSTEMS_MAP.md`: recorded the new large-scale stress proof and remaining backend risks.
- `simsuite-reports/LIBRARY_LARGE_SCALE_BACKEND_STRESS_V1_REPORT.md`: created this sprint report.
- `SESSION_HANDOFF.md` and `docs/IMPLEMENTATION_STATUS.md`: updated with this sprint note; only this sprint's hunks should be staged.

### Tests

Passed:

- `npm run test:library:stress`
- `cargo fmt`
- `npm run build`
- `npx tsc --noEmit`
- `npm run test:unit` (`22` files, `87` tests)
- `cargo check`
- `cargo test` (`241` passed, `2` ignored stress tests)
- `cargo build --release`
- `npm run test:rust` (`241` passed, `2` ignored stress tests)

Existing Rust warnings remain unchanged. The existing Vite chunk-size warning remains unchanged.

### Desktop/runtime proof

Passed:

- `npm run desktop:proof:fixtures`
- `npm run desktop:smoke:fixtures`

Latest Library proof summary:

- `C:\Users\likwi\OneDrive\Desktop\PROJS\SimSort\output\desktop\library-proof\latest-summary.json`
- Screenshot folder:
  `C:\Users\likwi\OneDrive\Desktop\PROJS\SimSort\output\desktop\library-proof\2026-05-12T20-14-57-200Z`

The proof summary search did not show failed `ok: false` entries, severe/error markers, or geometry failures. Desktop proof still reports fixture preview diagnostics honestly: 11 fixture rows, 0 preview rows.

### What could not be verified

- No real user Mods/Tray library was scanned or benchmarked.
- No real 10,000+ file Windows library with mixed CC/mod/Tray content was measured.
- The stress harness does not test actual disk scanning throughput.
- The stress harness does not implement or measure package/script duplicate fingerprints.
- Relationship count behavior is proven for a large synthetic filtered page, not every real query shape.

### Next backend sprint recommendation

Next recommended backend focus:

1. Relationship count SQL aggregation/cache v2 if real-library traces show repeated broad filtered-set cost.
2. Duplicate Truth Engine v3 scan-time package/script fingerprints.
3. Real-library thumbnail validation with sanitized user-approved data.
4. Staging backend cleanup.
5. Updates provider onboarding architecture.

### Docs updated

- `simsuite-reports/LIBRARY_LARGE_SCALE_BACKEND_STRESS_V1_REPORT.md`
- `docs/LIBRARY_BACKEND_SYSTEMS_MAP.md`
- `SESSION_HANDOFF.md`
- `docs/IMPLEMENTATION_STATUS.md`

### Unrelated worktree changes

- Pre-existing `.cocoindex_code/*`, `src/screens/HomeScreen.tsx`, and `src/styles/globals.css` changes were left alone.
- Pre-existing unrelated notes in `SESSION_HANDOFF.md` and `docs/IMPLEMENTATION_STATUS.md` were preserved; only this sprint's new notes should be staged.

### Commit

- Pending before commit.

### Final honest verdict

Verified: Library large-scale backend stress v1 is working for the tested paths.

This does not prove every real-world 10,000+ file collection is production-ready. It proves the covered synthetic backend paths, keeps duplicate truth strict, and identifies relationship count and real-library validation as the next areas to watch.
