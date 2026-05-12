# Library Thumbnail Preview Pipeline v1 Report

Date: 2026-05-12
Branch: codex/library-thumbnail-preview-pipeline-v1

## Audit Note Before Implementation

### Worktree state

- Current branch: `codex/library-thumbnail-preview-pipeline-v1`.
- Pre-existing unrelated changes are present in `.cocoindex_code/*`, `src/screens/HomeScreen.tsx`, `src/styles/globals.css`, `SESSION_HANDOFF.md`, and `docs/IMPLEMENTATION_STATUS.md`.
- This sprint will not overwrite or stage unrelated Home/global CSS/generated changes.
- This sprint may add new report/status/handoff notes, but those hunks must be staged selectively.

### 1. Current preview pipeline map

- Preview sources today:
  - `.package` embedded DBPF THUM resources, decoded to base64 PNG.
  - Sims 4 `localthumbcache.package`, parsed once through a `OnceLock` cache, then matched by package instance id and decoded to base64 PNG.
- File types with current preview support:
  - `.package` can have embedded or game-cache previews.
- File types without current preview support:
  - `.ts4script` is inspected as a zip for namespaces and hints, but no preview is extracted.
  - Tray files are indexed and displayed, but there is no committed Tray thumbnail extraction path found in this audit.
  - Unsupported/unknown file types do not have previews.
- Extraction timing:
  - Scan uses `THUMBNAIL_DEFERRED = true`, so package thumbnails are intentionally skipped in the scan hot path.
  - `get_file_detail` runs deferred package thumbnail resolution for the selected file.
  - List, grid, and folder rows only display preview fields already stored in the indexed `insights` JSON.
- Storage:
  - Preview data is stored inside `files.insights` as base64 strings: `thumbnail_preview` and `cached_thumbnail_preview`.
  - The database does not currently store separate preview status fields, failure reasons, attempted-at timestamps, or stale/cache provenance.
- Frontend hydration:
  - List and folder rows use `LibraryRowThumbnail`.
  - Grid cards use `LibraryThumbnailGrid`.
  - Inspector and More Details read `thumbnailPreview` or `cachedThumbnailPreview`.
  - `libraryDisplay.tsx` maps embedded previews to `embedded`, cached previews to `cache`, and missing previews to `fallback`.
- Fallback behavior:
  - Missing preview data renders clean row/folder/grid/detail fallback states.
- Fixture vs live-library verification:
  - Current fixtures mostly verify fallback geometry and preview-field rendering, not real Sims thumbnail payload extraction.
  - No safe real-library thumbnail validation has been run in this sprint yet.

### 2. Current preview state model

- Clearly represented today:
  - Preview available: yes, when either preview field is present.
  - Preview missing in the response: yes, represented by missing preview fields.
- Not clearly represented today:
  - Preview not attempted.
  - Preview unsupported.
  - Preview extraction failed.
  - Preview deferred.
  - Preview stale.
  - Preview cache reused.
- Practical v1 decision:
  - Add a safe diagnostics path and selected-file persistence improvement rather than a broad persistent preview-state migration.
  - Keep user-facing copy simple: `No preview available` remains correct when no indexed preview is available.

### 3. Real-library validation plan

- Do not scan, copy, commit, or screenshot real user Sims files for this sprint.
- Use existing fixture proof for runtime validation.
- Add sanitized diagnostics that report counts only, without paths, filenames, thumbnails, or private folders.
- If real-library validation is done later, it should use configured Library paths through existing scanner behavior, retain no user assets, and report only sanitized counts.

### 4. User experience impact plan

Users should see real thumbnails more often after opening a file detail if SimSuite can extract one, because the selected-file preview can be saved back into the Library index. Files without previews still show a calm fallback, and browsing should stay fast because thumbnail extraction remains selected-file, cached, and not part of ordinary row rendering.

## Implementation Result

### What was audited

- Backend scanner and inspection paths in `scanner`, `file_inspector`, and `library_index`.
- DBPF package thumbnail extraction, localthumbcache lookup, deferred detail hydration, and indexed `files.insights` preview fields.
- Frontend thumbnail surfaces: list rows, folder direct-file rows, grid cards, inspector, and More Details.
- API/types/mock paths for Library preview fields.
- Desktop proof fixture coverage and the newest Library proof summary.

### Current thumbnail behavior

- `.package` files are the only file type with a committed preview extractor today.
- Package previews can come from embedded DBPF THUM resources or from Sims 4 `localthumbcache.package`.
- Scan keeps thumbnail extraction deferred to avoid slowing normal indexing.
- Before this sprint, selected-file detail could hydrate a preview, but the newly found preview was not persisted back into the indexed Library row data.
- `.ts4script`, Tray files, and unsupported file types still use fallback thumbnails because SimSuite does not currently have deterministic preview extraction for them.

### What changed

- Added a sanitized backend/API diagnostic command: `get_library_preview_diagnostics`.
- Added `LibraryPreviewDiagnostics` in Rust and TypeScript.
- Desktop proof now collects preview diagnostics after fixture indexing and after opening details.
- Selected-file detail preview hydration now persists newly found embedded/cache previews back into `files.insights`.
- Later row, folder, and grid queries can reuse a selected-file preview found during detail hydration without reparsing thumbnails during ordinary browsing.
- Removed stale third-party/private-path comments around preview extraction.
- Updated mock API support so frontend tests and preview diagnostics can run without Tauri.

### What this means for the user

When SimSuite can find a real package preview, it is more likely to keep showing that preview after the user opens the file details. Files with no preview still show a clean fallback instead of looking broken. Normal Library browsing stays fast because SimSuite does not scan every file for thumbnails while the user scrolls. No user files or thumbnails were copied into the repo or committed.

### Preview state model

- `available`: represented when `thumbnail_preview` or `cached_thumbnail_preview` exists.
- `missing`: represented when no preview fields are present.
- `deferred`: true for package preview extraction as a pipeline behavior; diagnostics report package rows as deferred-or-missing when they lack preview data.
- `unsupported`: query-time classification for scripts, Tray, and other rows without a current extractor.
- `failed`: not persistently tracked yet.
- `stale`: not persistently tracked yet.
- `not attempted`: not persistently tracked per file yet.

No migration was added because v1 can expose honest aggregate diagnostics and persist successful selected-file previews using the existing `files.insights` JSON.

### Real-library validation

- Real user Sims files were not scanned, copied, screenshot, or committed.
- Validation used deterministic desktop fixtures and sanitized diagnostics only.
- Fixture proof confirms the diagnostic command works, but the fixture files do not contain real Sims thumbnail payloads.
- Real-library preview coverage remains future work and should report sanitized counts only unless the user explicitly provides shareable fixture files.

### Diagnostics

Desktop proof wrote sanitized preview diagnostics into:

`output/desktop/library-proof/latest-summary.json`

Latest proof folder:

`output/desktop/library-proof/2026-05-12T19-07-56-936Z`

Fixture diagnostic result:

- `totalRows`: 11
- `rowsWithPreview`: 0
- `rowsWithoutPreview`: 11
- `packageRows`: 5
- `packageRowsWithPreview`: 0
- `packageRowsDeferredOrMissing`: 5
- `scriptRows`: 6
- `unsupportedRows`: 6
- `failureStateTracked`: false
- `staleStateTracked`: false
- `deferredExtractionEnabled`: true

This is expected for the current fixture set and documents the fixture limitation honestly.

### Performance notes

- Normal list/grid/folder browsing still reads indexed preview fields only.
- Thumbnail extraction remains selected-file and lazy through detail hydration.
- Persisted previews avoid repeating successful selected-file extraction on later row queries.
- The new diagnostic command is explicit and aggregate-only; it is not part of ordinary Library browsing.
- No all-library thumbnail backfill, thumbnail render-loop parsing, or unbounded image logging was added.

### Files changed

- `src-tauri/src/core/library_index/mod.rs`: preview diagnostics, selected-file preview persistence, backend tests.
- `src-tauri/src/models.rs`: preview diagnostics response model.
- `src-tauri/src/commands/mod.rs`: Tauri command wrapper.
- `src-tauri/src/lib.rs`: command registration.
- `src-tauri/src/core/file_inspector/mod.rs`: clarified thumbnail extraction comments.
- `src/lib/types.ts`: TypeScript diagnostics type.
- `src/lib/api.ts`: API method and mock diagnostic support.
- `scripts/desktop/desktop-library-proof.mjs`: sanitized preview diagnostic proof.
- `docs/LIBRARY_BACKEND_SYSTEMS_MAP.md`: updated preview pipeline map.

### Tests

Passed:

- `cargo test preview` (`11` focused preview-related tests)
- `cargo fmt`
- `npm run build`
- `npx tsc --noEmit`
- `npm run test:unit` (`22` files, `87` tests)
- `cargo check`
- `cargo test` (`241` tests)
- `cargo build --release`
- `npm run test:rust` (`241` tests)
- `npm run desktop:proof:fixtures`
- `npm run desktop:smoke:fixtures`

Existing warnings remain:

- Rust emits existing unused-code/import warnings in older modules.
- Vite still emits the existing large chunk-size warning for the main bundle.

### Desktop/runtime proof

- `npm run desktop:proof:fixtures` passed with `DESKTOP_LIBRARY_PROOF_OK`.
- `npm run desktop:smoke:fixtures` passed against the release Tauri app.
- Desktop proof captured the existing Library screenshot set and recorded preview diagnostics.
- Runtime proof reported no severe runtime errors. Existing Tauri callback warnings appeared around reload/async timing and were non-fatal.

### What could not be verified

- Real CC/package thumbnail coverage was not verified because this sprint did not scan or retain real user Sims files.
- Tray thumbnail extraction was not implemented or proven.
- Script preview extraction was not implemented or claimed.
- Persistent failed/stale/not-attempted preview state still needs a future schema/state-model pass if the product needs those distinctions per file.

### Next backend sprint recommendation

1. Larger 5,000 to 10,000 row backend stress harness.
2. Relationship count SQL aggregation/cache pass.
3. Real-library thumbnail validation with sanitized counts from user-approved local content.
4. Duplicate Truth Engine v3 scan-time package/script fingerprints.
5. Staging backend cleanup.

### Docs updated

- `simsuite-reports/LIBRARY_THUMBNAIL_PREVIEW_PIPELINE_V1_REPORT.md`
- `docs/LIBRARY_BACKEND_SYSTEMS_MAP.md`
- `SESSION_HANDOFF.md`
- `docs/IMPLEMENTATION_STATUS.md`

### Unrelated worktree changes

Pre-existing unrelated changes in `.cocoindex_code/*`, `src/screens/HomeScreen.tsx`, `src/styles/globals.css`, `SESSION_HANDOFF.md`, and `docs/IMPLEMENTATION_STATUS.md` were not overwritten. Only this sprint's documentation hunks should be staged from the shared status files.

### Commit hashes

- `7248327` - Harden Library thumbnail preview pipeline.

### Final honest verdict

Partially verified: thumbnail pipeline changes passed tests and desktop proof, but real-library preview proof was limited.
