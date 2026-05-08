# Desktop Proof Wrapper Portability v1 Report

Date: 2026-05-08
Branch: `codex/updates-workflow-v1`

## Audit Before Coding

### Worktree

- `git status --short` showed one pre-existing unstaged change: `src/styles/globals.css`.
- `git diff -- src/styles/globals.css` shows a Home hero metric styling tweak only:
  - `.home-hero-metric span` receives a text color.
  - `.home-hero-metric small` receives a different text color and line height.
- This CSS change is unrelated to desktop verification wrapper portability and should stay out of this sprint commit.

### What Currently Works

- The actual native PowerShell proof runners work when called directly from native Windows PowerShell:
  - `powershell.exe -NoProfile -ExecutionPolicy Bypass -File scripts\desktop\run-desktop-library-proof.ps1`
  - `powershell.exe -NoProfile -ExecutionPolicy Bypass -File scripts\desktop\run-tauri-smoke.ps1`
- Those scripts derive the repo root from `$PSScriptRoot`, build through `npm run tauri:build`, start `run-tauri-webdriver.ps1` with fixtures, and then run the Node proof/smoke scripts.
- The proof/smoke Node scripts resolve the release exe from the repo root:
  - `scripts/desktop/desktop-library-proof.mjs`
  - `scripts/desktop/desktop-smoke.mjs`

### What Fails

- `npm run desktop:proof:fixtures` fails before app launch in native Windows PowerShell.
- `npm run desktop:smoke:fixtures` has the same wrapper shape and failed in the previous sprint for the same reason.
- Reproduced failure:
  - npm tries to run `/mnt/c/Windows/System32/WindowsPowerShell/v1.0/powershell.exe ...`
  - native Windows PowerShell reports `The system cannot find the path specified.`

### Shell/Context Failure

- Native Windows PowerShell fails because `/mnt/c/...` is a WSL mount path, not a native Windows executable path.
- WSL-style shells can use `/mnt/c/...`, but native Windows npm cannot.
- Git Bash/native Windows Node should be treated as Windows for executable path purposes.

### Wrappers

- `package.json` currently contains direct npm wrappers for the fixture proof lanes:
  - `desktop:proof:fixtures`
  - `desktop:smoke:fixtures`
- Other similar fixture wrappers exist, such as `desktop:driver:fixtures` and `desktop:smoke:apply:fixtures`, but this sprint is focused on the two standard proof commands requested by the user.

### Actual Proof Runners

- `scripts/desktop/run-desktop-library-proof.ps1` is the library proof runner.
- `scripts/desktop/run-tauri-smoke.ps1` is the smoke proof runner.
- `scripts/desktop/run-tauri-webdriver.ps1` starts the Tauri WebDriver fixture app.
- `scripts/desktop/desktop-library-proof.mjs` performs the Library proof assertions.
- `scripts/desktop/desktop-smoke.mjs` performs the smoke assertions.

### Missing/Wrong Path Conversion

- The npm wrappers hardcode the PowerShell executable path as a WSL path.
- There is no wrapper layer that detects native Windows vs WSL.
- There is no conversion from `/mnt/c/...` repo paths to `C:\...` before passing a script path to Windows PowerShell.
- The safest fix is to leave the PowerShell proof runners alone and add a small Node launcher that:
  - resolves the repo root from `process.cwd()`
  - verifies it is running inside the SimSuite repo
  - detects Windows vs WSL-style environments
  - uses `powershell.exe` on Windows
  - uses `powershell.exe` or a WSL-accessible fallback path from WSL
  - converts `/mnt/<drive>/...` paths to Windows paths before passing `-File`
  - fails clearly if the script or PowerShell cannot be found

## Implementation

- Added `scripts/desktop/run-powershell-script.mjs`.
- The launcher:
  - checks that the command is run from the SimSuite repo root
  - detects native Windows vs WSL-style Linux
  - uses `powershell.exe` from native Windows
  - uses `/mnt/c/Windows/System32/WindowsPowerShell/v1.0/powershell.exe` from WSL
  - converts `/mnt/<drive>/...` and Git-Bash-style `/<drive>/...` paths to Windows drive paths before passing `-File`
  - keeps native Windows paths as native Windows paths
  - prints clear errors when the repo root, PowerShell script, shell context, or PowerShell executable is invalid
- Updated the desktop fixture npm wrappers in `package.json` to use the launcher:
  - `desktop:driver:fixtures`
  - `desktop:smoke:fixtures`
  - `desktop:smoke:apply:fixtures`
  - `desktop:proof:fixtures`
- Left the PowerShell proof runners unchanged:
  - `scripts/desktop/run-desktop-library-proof.ps1`
  - `scripts/desktop/run-tauri-smoke.ps1`
  - `scripts/desktop/run-tauri-webdriver.ps1`
- Added `src/lib/runPowershellScript.test.ts` to cover Windows and WSL path behavior.

## Validation

- `npm run desktop:proof:fixtures`
  - reproduced the original native Windows failure before the fix:
    - `/mnt/c/Windows/System32/WindowsPowerShell/v1.0/powershell.exe` was not found
  - passed after the fix
  - reached `DESKTOP_LIBRARY_PROOF_OK`
  - proof summary written to `output\desktop\library-proof\latest-summary.json`
- `npx vitest run src/lib/runPowershellScript.test.ts`
  - passed: 3 tests
- `npx tsc --noEmit`
  - initially failed because the TypeScript test imported a plain `.mjs` tooling module without a declaration file
  - passed after a narrow test import suppression
- `npm run test:unit`
  - passed: 16 files, 52 tests
- `npm run build`
  - passed with the existing Vite chunk-size warning
- `npm run desktop:smoke:fixtures`
  - passed after the fix
  - reached `Desktop smoke passed against ...\src-tauri\target\release\simsuite.exe`
  - reached `TAURI_SMOKE_DONE exit=0`

## Shell Contexts

- Verified:
  - native Windows PowerShell in `C:\Users\likwi\OneDrive\Desktop\PROJS\SimSort`
- Not verified in this session:
  - WSL shell from `/mnt/c/Users/likwi/OneDrive/Desktop/PROJS/SimSort`
  - Git Bash
- WSL behavior is covered by the unit test for path conversion and executable selection, but it was not runtime-executed in WSL.

## Unrelated Files

- `src/styles/globals.css` remains modified and unstaged.
- It is the pre-existing Home hero metric styling tweak from the worktree audit.
- It was not touched for this sprint and should not be included in the wrapper commit.

## Commit

- Pending.
