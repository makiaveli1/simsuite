# SimSuite

SimSuite is a local-first Windows desktop app for reviewing and maintaining a Sims 4 Mods/Tray library. It is built with Tauri, Rust, React, TypeScript, Vite, and SQLite.

The current product is intentionally safety-first: it can scan, inspect, classify, preview, validate, and dry-run organization plans, but real user-file Apply/Restore workflows are still blocked until the backend safety contract is complete.

## Current capability

Implemented and usable today:

- Library path setup for Mods, Tray, and Downloads folders.
- Library scanning and SQLite indexing.
- Package/script inspection with Sims-specific evidence extraction.
- Library browsing, folder views, facets, detail panels, and preview diagnostics.
- Duplicate detection based on exact hashes and content fingerprints.
- Creator and category audit workflows for metadata review.
- Downloads/Inbox review flows with file-changing actions currently blocked in the visible UI.
- Watch/update review for supported and manually tracked sources.
- Organize preview plans, saved draft ApplyPlans, validation preview, recovery-history metadata, and dry-run preview.

Not implemented as user-file-changing workflows yet:

- Real Organize Apply.
- Real Restore.
- User-file backup/restore execution.
- Delete/quarantine/replace flows.
- Automatic duplicate cleanup.
- AI-only file decisions.
- Automatic update replacement.

For the safety contract, see:

- `docs/TRUST_BOUNDARIES_AND_AUTOMATION_READINESS.md`
- `docs/planning/APPLY_SAFETY_CONTRACT_V1.md`
- `docs/COMMAND_SURFACE_APPLY_SAFETY_AUDIT_V1_REPORT.md`

## Tech stack

- Frontend: React 19, TypeScript 5.9, Vite 7.
- Desktop shell: Tauri 2.
- Backend: Rust.
- Database: SQLite through `rusqlite`.
- Tests: Vitest, Rust unit tests, and Windows desktop proof/smoke scripts.

## Project structure

```text
src/                 React frontend
src-tauri/           Tauri/Rust backend
database/            SQLite migrations
seed/                Seed catalogs and rules
scripts/             Build, test, and desktop verification scripts
docs/                Architecture, status, safety, and planning docs
models/              Local model/prompt placeholders
```

## Development setup

Prerequisites:

- Node.js 24+ recommended for this repo state.
- Rust stable with the Windows MSVC toolchain for desktop builds.
- Windows 10/11 as the primary runtime target.
- WSL is supported for agent/development orchestration.

Install dependencies:

```bash
npm install
```

Run the frontend dev server:

```bash
npm run dev
```

Run the Tauri dev app from Windows/WSL:

```bash
npm run tauri:dev
```

Build the frontend:

```bash
npm run build
```

Build the desktop app:

```bash
npm run tauri:build
```

## Verification

Baseline lane before review:

```bash
npx tsc --noEmit
npm run test:unit
npm run build
npm run test:rust
```

Notes:

- `npm run test:unit` forces `NODE_ENV=test` through `scripts/test/run-vitest.mjs` so inherited production environments do not poison Vitest/React tests.
- `npm run test:rust` runs Cargo through Windows PowerShell when called from WSL so path-sensitive Tauri tests match the Windows target.

Desktop proof/smoke lanes:

```bash
npm run desktop:proof:fixtures -- --SkipBuild
npm run desktop:smoke:fixtures
```

Generated proof output belongs under `output/`, which is ignored.

## Documentation map

Keep the repo docs lean:

- `docs/ARCHITECTURE.md` — architecture overview.
- `docs/IMPLEMENTATION_STATUS.md` — current product state and next gates.
- `docs/DESKTOP-VERIFICATION-RUNBOOK.md` — desktop proof/testing instructions.
- `docs/TRUST_BOUNDARIES_AND_AUTOMATION_READINESS.md` — safety boundaries.
- `docs/planning/CONFIRMATION_DESIGN_V1.md` — read-only confirmation contract, custom folder configuration direction, and cross-system context trail.
- `docs/planning/` — active safety and ApplyPlan design contracts.
- `docs/product-specs/` — longer product specification archive.

Do not commit generated reports, session logs, desktop screenshots, local indexes, or `.cocoindex_code` artifacts.

## License

Proprietary / private project unless a license is added later.
