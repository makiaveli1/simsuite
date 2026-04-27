# Sprint 1 — Library Truth and Testability

## 1. Executive summary

This pass tightened trust language, verified the core Library SQL paths actually execute, confirmed folder metadata no longer depends on unsupported SQLite functions, confirmed Staging is wired into Tauri, and re-ran the main frontend + desktop verification checks.

What changed in code this pass:
- softened misleading **"Not tracked"** wording to **"No update source"** across Library/Updates surfaces
- renamed **"Loose files"** UI language to **"Direct files"** so folder views describe placement more truthfully
- removed a few overreaching folder relationship phrases like **"often downloaded together"** and replaced them with literal placement wording
- softened tray status wording from **"Disabled (in tray)"** to **"Stored in Tray"**

What was verified rather than newly fixed:
- `list_library_files` paged and unpaged paths prepare and execute
- same-folder counts use actual parent-folder identity, not only depth
- same-pack counts ignore null `bundle_id`
- folder metadata is computed in Rust path logic, not `REVERSE()` SQL
- Staging commands are registered with Tauri and the Staging screen ships in the built app

## 2. What was broken

### Confirmed in the current pass
1. **Trust language was still too soft / misleading in a few places**
   - "Not tracked" implied a hidden tracking state rather than the simpler truth: no watch source is saved.
   - "Loose files" sounded messy/problematic when the real fact is just "stored directly in this folder."
   - "Disabled (in tray)" added interpretation where literal placement wording is safer.
   - Some same-folder copy drifted toward relationship storytelling instead of plain storage truth.

### Confirmed from verification, but already fixed in the repo before this pass
1. **Library SQL trailing-comma risk**
   - The current `list_library_files` implementation is valid in both paged and unpaged paths.
2. **Folder metadata fragility**
   - Current folder metadata does not use SQLite `REVERSE()`.
3. **Staging registration gap**
   - Current `src-tauri/src/lib.rs` registers Staging commands and includes a regression test.

### Still failing
1. **`cargo test` is not fully green**
   - Exact failure:
     - `core::install_profile_engine::tests::stale_indexed_special_files_do_not_block_fresh_guided_install`
   - Exact error:
     - `assertion failed: plan.apply_ready`
   - This is **not** the old `inspect_file` signature problem from the audit. Tests now run; this is a separate existing failure in install-profile logic.

## 3. What was fixed

### A. Update-watch wording
Changed user-visible "not watched/tracked" wording to **"No update source"** in:
- `src/screens/library/LibraryTopStrip.tsx`
- `src/screens/LibraryScreen.tsx`
- `src/screens/library/LibraryDetailsPanel.tsx`
- `src/screens/library/LibraryDetailSheet.tsx`
- `src/screens/library/LibraryThumbnailGrid.tsx`
- `src/screens/library/libraryDisplay.tsx`
- `src/screens/UpdatesScreen.tsx`

Reason: this describes the actual state better. The app does not know a source to check yet.

### B. Folder view wording
Changed visible folder copy from **"Loose files"** to **"Direct files"** in:
- `src/screens/library/FolderContentPane.tsx`

Reason: these files are not necessarily messy or wrong; they are simply stored directly in the selected folder rather than a child folder.

### C. Tray wording
Changed tray-status wording from **"Disabled (in tray)"** / **"Disabled"** to **"Stored in Tray"** in:
- `src/screens/library/libraryDisplay.tsx`

Reason: tray placement is a literal fact. Whether the user thinks of that as "disabled" is secondary.

### D. Relationship copy
Adjusted same-folder language to stay literal:
- "Same folder set" → "Same folder"
- removed "often downloaded together"
- changed copy to "stored in" / "shared placement"

File:
- `src/screens/library/libraryDisplay.tsx`

## 4. Library SQL results

## Current result
`list_library_files` is valid in the current codebase.

### Paged path
- Function: `src-tauri/src/core/library_index/mod.rs`
- Query branch: `query.limit.is_some()`
- Status: valid and executable
- Relationship count placeholders in the SQL select list are valid:
  - `0 AS same_folder_peer_count`
  - `0 AS same_pack_peer_count`
- No trailing comma before `FROM files f`

### Unpaged path
- Same file, `limit.is_none()` branch
- Status: valid and executable
- No malformed select-list tail

### Preview inclusion
- `include_previews` does **not** change SQL shape
- It only affects post-query compaction through `compact_library_row_insights(...)`

### Filters and sort modes
- Still compatible with the query shape
- Filters are assembled via `build_filters(&query)`
- Sort is assembled via `build_order_by(query.sort_by)`

### Relationship fields
- Peer counts are loaded outside the main row SQL via `load_relationship_peer_counts(...)`
- This avoids malformed inline window SQL and keeps query preparation straightforward

### Null bundle IDs
- Current same-pack grouping only inserts into `bundle_groups` when `bundle_id` is `Some(...)`
- Null bundle IDs are **not** grouped together

### Regression proof
Verified by execution, not guesswork:
- `library_queries_focus_on_installed_content`
- `library_listing_supports_paged_queries`
- folder listing tests also depend on this flow working

## 5. Rust test results

### Old audit claim: outdated `inspect_file` signatures
Current result: **not reproducible now**.

Current `inspect_file` signature:
```rust
pub fn inspect_file(
    path: &Path,
    extension: &str,
    seed_pack: &SeedPack,
    defer_thumbnails: bool,
) -> AppResult<InspectionOutcome>
```

Current search result shows callers already updated.

### `cargo test`
Command run:
```bash
cd src-tauri
cargo test
```

Result:
- tests run successfully
- suite is **not fully green**
- failure:
  - `core::install_profile_engine::tests::stale_indexed_special_files_do_not_block_fresh_guided_install`
- error:
  - `assertion failed: plan.apply_ready`

Assessment:
- this is a **pre-existing or separate** failure from the Library sprint brief
- it does **not** indicate the old `inspect_file` signature mismatch
- it does **not** block Library SQL truth work directly
- it should be fixed separately unless Sprint 1 is broadened to include install-profile engine stability

## 6. Folder metadata truth

Current result: folder metadata is using Rust path segmentation, not unsupported SQLite tricks.

Relevant functions:
- `get_folder_tree_metadata`
- `build_folder_metadata_from_rows`
- `folder_segments_for_file`
- `folder_root_name`

### Verified truths
- Mods root is explicit
- Tray root is explicit
- nested folder segments are derived from normalized file paths
- direct-folder identity uses actual parent path segments
- selected-folder content and descendant-folder content are separated by different UI sections

### Important distinction now reflected in UI wording
- **Direct files** = stored directly in the selected folder
- **Files in subfolders** = stored below the selected folder in child folders
- **Descendant files** = umbrella concept, not mixed into direct-file wording

## 7. Relationship count truth

### Same-folder
Current implementation uses:
- normalized file path
- source root
- actual parent-folder identity from `folder_segments_for_file(...)`

It does **not** group only by:
- `source_location + relative_depth`

That overclaim risk appears already fixed in current code.

### Same-pack
Current implementation only groups when `bundle_id` is present.
Null `bundle_id` values are excluded.

### UI proof language
This pass made same-folder copy more literal:
- shared placement is described as storage truth
- wording avoids implying dependency or pack membership

## 8. Staging exposure

Current result: **Staging is wired, not dead.**

Evidence:
- `src-tauri/src/lib.rs` registers:
  - `commands::get_staging_areas`
  - `commands::cleanup_staging_areas`
  - `commands::commit_staging_area`
  - `commands::commit_all_staging_areas`
- `src-tauri/src/lib.rs` also includes regression test:
  - `staging_commands_are_registered_with_tauri`
- built frontend still includes `StagingScreen-*.js`
- `src/screens/StagingScreen.test.tsx` passed

Conclusion:
- Sprint 1 does **not** need to hide Staging right now
- the current path is effectively **Option C — register commands properly**, and that part is already in place

## 9. Verification run

### Frontend / TypeScript
Passed:
```bash
npx tsc --noEmit
npm run test:unit
npm run build
```

Results:
- `npx tsc --noEmit` ✅
- `npm run test:unit` ✅ — 13 files / 41 tests passed
- `npm run build` ✅

### Rust / Tauri
Passed:
```bash
cargo check
cargo build --release
```

Result:
- `cargo check` ✅
- `cargo build --release` ✅

Not fully green:
```bash
cargo test
```
- `cargo test` ❌ — 1 failing test, listed above

### Real app / desktop runtime
Passed:
```bash
npm run desktop:smoke:fixtures
```

Result:
- `Desktop smoke passed against ...\src-tauri\target\debug\simsuite.exe`

What this proves:
- real Tauri app built and launched
- desktop smoke flow passed in the Windows runtime

What it does **not** fully prove by itself:
- manual coverage of every folder-view path
- explicit Staging navigation coverage

## 10. Remaining gaps

1. **One unrelated Rust test still fails**
   - install-profile engine, not Library query syntax
2. **Folder/list/grid/staging manual route-by-route desktop verification is still thinner than ideal**
   - desktop smoke passed, which is strong evidence
   - but Sprint 1 final sign-off would still benefit from explicit folder-view/manual-path checks if we want maximum certainty

## 11. Recommendation

Sprint 1 is materially healthier now.

If continuing immediately, the next best step is:
1. do a focused real-app route verification for Library list/grid/folders/Staging
2. decide whether to pull the unrelated install-profile test failure into this sprint or leave it as a separate stabilization ticket
