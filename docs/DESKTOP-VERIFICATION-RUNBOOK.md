# SimSuite Desktop Verification Runbook

This is the practical Level 3 lane for the real Windows Tauri app.

## What this lane is for

Use this when Level 2 browser/Vite proof is not enough and you need evidence from the real desktop app window.

It is intentionally narrower than the full desktop smoke suite.

The goal is to prove:
1. the Windows app launches
2. the Library screen becomes usable
3. a known fixture row can be selected
4. the detail/preflight path can be exercised
5. screenshots and a JSON summary are saved as evidence

## Recommended command

From the SimSort repo root in WSL or Windows PowerShell:

```bash
npm run desktop:proof:fixtures -- --SkipBuild
```

If you want it to rebuild the desktop binary first, omit `--SkipBuild`.

## What it does

`desktop:proof:fixtures` runs a Windows-side PowerShell wrapper that:

1. optionally builds the release Tauri app
2. launches `tauri-driver` with isolated smoke fixtures
3. starts the real Windows app against those fixture paths
4. runs a focused Selenium proof script
5. captures screenshots
6. writes a JSON summary to `output/desktop/library-proof/latest-summary.json`
7. cleans up `tauri-driver`, `msedgedriver`, and the app process

## Output

Evidence is written to:

```text
output/desktop/library-proof/
```

Each run creates a timestamped folder plus:

```text
output/desktop/library-proof/latest-summary.json
```

Typical screenshots:
- `01-library-selected-mccc.png`
- `02-library-preflight-detail-mccc.png`
- `03-review-route.png`
- `04-updates-route.png`

## Why this is the recommended Level 3 path

This lane is more reliable than launching `tauri-driver` separately and then attaching later.
It also prefers the built release executable over the dev-oriented debug binary, so it does not depend on a localhost dev server being alive.

### Good
- Windows-side lifecycle stays in one wrapper run
- Uses deterministic smoke fixtures
- Produces saved evidence
- Focused assertions, so failures are easier to trust

### Not recommended for routine proof
- Detached `desktop:driver` + ad-hoc follow-up commands from WSL
  - readiness can look good briefly while the later session still dies
- Full `desktop:smoke:fixtures` as the first proof step for a narrow UI change
  - it covers much more surface area and can fail for unrelated reasons

## Existing desktop verification paths

### 1. `desktop:proof:fixtures` — recommended for focused Level 3 proof
- real Windows app
- deterministic fixtures
- screenshots + summary
- narrow, trustworthy checks

### 2. `desktop:smoke:fixtures` — broader regression lane
- real Windows app
- deterministic fixtures
- wider assertions across Inbox/Home/Updates/Library watch flows
- more brittle when copy or unrelated UI shifts

### 3. Level 2 Vite/Playwright
- fastest verification path
- good for most UI work
- not sufficient when you specifically need real desktop proof

## Manual fallback

If webdriver is misbehaving on the machine:

1. run `npm run tauri:dev` in Windows PowerShell
2. open Library
3. select the target file
4. capture screenshots manually
5. record the exact route exercised and whether the app stayed stable

Use manual fallback when the desktop app itself looks healthy but the automation bridge is the thing breaking.

## Notes

- `scripts/desktop/run-tauri-smoke.ps1` was tightened so it runs from the repo root, preserves exit codes properly, and prefers a release build path.
- The focused proof lane is meant to be the default Level 3 spot-check before using the broader smoke suite.
