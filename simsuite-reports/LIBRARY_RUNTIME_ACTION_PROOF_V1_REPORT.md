# Library Runtime Action Proof v1 Report

Date: 2026-05-10

## Audit note before coding

### Worktree

- `git status --short` showed one pre-existing unstaged file: `src/styles/globals.css`.
- The `src/styles/globals.css` diff still only changes Home hero metric text color and line-height.
- Classification: unrelated Home styling tweak, not part of this Library runtime action proof sprint.
- Action: leave it untouched, do not format it, and keep it out of this sprint commit.

### Already working

- Library route, list view, grid view, folder view, inspector, More Details sheet, Safe Action Preflight detail, Needs Review bridge, Updates bridge, and Duplicates screen all exist.
- `reveal_file_in_folder` is registered in Tauri and handles files, directories, and parent-folder fallback on Windows.
- `LibraryDetailsPanel` only shows folder-summary `Open folder` when a real `folderFullPath` exists.
- Existing desktop proof already covers list selection, grid, folder, detail sheet, Safe Action Preflight detail, and Needs Review routing.
- Fixture setup already creates deterministic MCCC and generic watch files in a temp fixture library.

### Missing proof

- Desktop proof did not click the real Library `Open in Updates` action.
- Desktop proof did not click the real Library `Open in Duplicates` action.
- Desktop proof did not inspect runtime errors during the Library action flow.
- Desktop proof did not click `Open folder`, by design, because that can launch Explorer outside the app.

### Issues found

- `LibraryScreen` accepts `onNavigateDuplicates`, but `App` did not pass it. That means the real Library Duplicates bridge could be hidden in the packaged app even though the screen supports it.
- `DuplicatesScreen` did not accept focused file context. It can still load generic duplicates, but the Library bridge could not tell the user which selected file opened the view.

### Open Folder proof choice

- Desktop-click proof would open Windows Explorer. That is useful manually but flaky for automated proof and can leave external windows open.
- For this sprint, the safe proof target is frontend/API proof: verify `Open folder` appears only for real paths in the tested Library contexts and calls `revealFileInFolder` with real fixture-style paths, not virtual Library paths.

## Changes made

- Wired the Library Duplicates bridge through `App` so the packaged app passes selected file context to `DuplicatesScreen`.
- Added focused-file context to `DuplicatesScreen` so Library-opened duplicate review shows `Opened from Library` and identifies the relevant possible duplicate pair.
- Added a unit test for focused Duplicates route context.
- Added a `LibraryDetailsPanel` unit test proving `Open folder` calls the reveal API with the real disk path for a selected file.
- Expanded the desktop Library proof to click the real `Open in Duplicates` and `Open in Updates` actions from Safe Action Preflight.
- Added desktop proof runtime-error capture for `console.error`, uncaught `error`, and `unhandledrejection`.
- Added browser-log inspection when the WebDriver browser log API is available.
- Made the desktop proof wait helpers more resilient to stale WebDriver elements during Tauri route transitions.
- Updated the WebDriver runner to match `msedgedriver` to the installed Edge WebView2 runtime, which is the browser runtime Tauri uses, instead of only matching the standalone Edge browser.

No fixture data was added. The existing fixture library already exposes the MCCC no-source/update cue and duplicate candidate used by the proof.

## Updates bridge proof

- Desktop proof opened Library, selected `mc_cmd_center.package`, opened Safe Action Preflight detail, and clicked the real `Open in Updates` action.
- The proof verified navigation to `#updates`.
- The proof verified the Updates page showed selected context for `mc_cmd_center.package` instead of landing as a generic Updates page.
- Screenshot captured:
  - `output/desktop/library-proof/2026-05-10T12-18-10-658Z/07-updates-bridge-mccc.png`

## Duplicates bridge proof

- Desktop proof opened Library, selected `mc_cmd_center.package`, opened Safe Action Preflight detail, and clicked the real `Open in Duplicates` action.
- The proof verified navigation to `#duplicates`.
- The proof verified `Opened from Library` focused context and visible `mc_cmd_center.package` duplicate context.
- The Duplicates page wording stays cautious: possible duplicate review and compare-first wording, not safe-delete language.
- Screenshot captured:
  - `output/desktop/library-proof/2026-05-10T12-18-10-658Z/06-duplicates-bridge-mccc.png`

## Open Folder proof

- Direct OS-click proof was intentionally not used because it launches Windows Explorer outside the app and can leave external windows open.
- Safe frontend/API proof was added instead.
- The focused test verifies `Open folder` appears for a selected file with a real disk path and calls the reveal handler with `C:\Fixtures\Mods\RuntimeProof\RuntimeProof.package`.
- This proves the action uses a real disk path, not a virtual Library path.

## Console/runtime error proof

- Desktop proof injected runtime error capture before running the Library action flow.
- The proof checked captured runtime errors after the Duplicates bridge and Updates bridge.
- Result: `0` captured runtime errors for both bridge checks.
- Browser log inspection was supported in the run. It recorded warning-level Tauri callback messages during app reload/async cleanup, but no injected runtime errors were captured during the bridge checks.

## Verification

- `npm run test:unit -- src/screens/DuplicatesScreen.test.tsx src/screens/library/LibraryDetailsPanel.test.tsx` passed: 2 files, 9 tests.
- `npx tsc --noEmit` passed.
- `npm run build` passed with the existing Vite chunk-size warning.
- `npm run test:unit` passed: 20 files, 61 tests.
- `npm run desktop:proof:fixtures` passed and reached `DESKTOP_LIBRARY_PROOF_OK`.
- `npm run desktop:smoke:fixtures` passed and reached `Desktop smoke passed`.
- Rust source was not touched, so the dedicated Rust validation commands were not required for this sprint.

## Desktop proof artifacts

- Summary: `output/desktop/library-proof/latest-summary.json`
- Screenshot folder: `output/desktop/library-proof/2026-05-10T12-18-10-658Z`
- Screenshots captured:
  - `01-library-selected-mccc.png`
  - `02-library-grid-view.png`
  - `03-library-folder-view.png`
  - `04-library-detail-sheet.png`
  - `05-library-preflight-detail-mccc.png`
  - `06-duplicates-bridge-mccc.png`
  - `07-updates-bridge-mccc.png`

## What could not be verified

- The proof did not click a real OS `Open folder` button because that opens Windows Explorer outside the app. The proof level is frontend/API coverage.
- WSL runtime execution was not run; this sprint was verified from native Windows PowerShell.
- Truly empty disk folders are still limited by backend scanner metadata before they can appear from real indexed data.
- Existing warning-level Tauri callback log entries remain observable around app lifecycle cleanup; they were not fatal route crashes and the injected runtime-error hook captured no bridge-flow errors.

## Remaining Library readiness work

- Add a safe manual or controlled Explorer-window proof lane for `Open folder` if the harness can close external windows reliably.
- Add natural fixture coverage for truly empty indexed folders if backend scanner metadata starts preserving them.
- Continue keeping Library wording cautious around duplicate, update, dependency, and delete-related surfaces.

## Commit

Pending.
