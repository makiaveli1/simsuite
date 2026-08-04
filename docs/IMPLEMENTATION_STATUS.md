# SimSuite implementation status

Last updated: 2026-08-04

## Current product state

SimSuite is in a **review, preview, validation, and dry-run** phase. The app has substantial scanning, indexing, inspection, duplicate review, Downloads/Inbox, Updates, Creator/Category Audit, and Organize draft-plan infrastructure.

The product is **not** ready to execute real user-file Apply or Restore operations. That boundary is intentional.

## Implemented

- Library path setup, scanning, indexing, folder metadata, and detail views.
- Package/script inspection and evidence extraction.
- Duplicate review using exact hashes and content fingerprints.
- Downloads/Inbox intake and review state, with visible file-changing actions blocked.
- Watch/update review for supported and manually tracked sources.
- Creator and category metadata audits.
- Organize sorting preview generation, including custom folder profile preview inputs.
- Saved draft ApplyPlan records with folder-configuration and cross-system context snapshots.
- Read-only ApplyPlan validation preview.
- Read-only Recovery history metadata display.
- Read-only dry-run preview in saved plan details.
- Read-only Confirmation Design V1 in saved plan details, including persisted custom folder configuration and cross-system context trail snapshots.
- Fixture-only backend proof work for future backup/restore logic.
- Windows desktop proof/smoke scripts.
- Validation wrappers for WSL/Windows test reliability.
- Cross-platform development foundation with pnpm 10.34.5 pinned as the canonical package manager.
- Portable Tauri development and build wrappers that preserve the existing Windows/WSL PowerShell lane and use native Node/Tauri execution on macOS and Linux.
- Tested native file-manager dispatch for Windows Explorer, macOS Finder, and Linux desktop openers.
- Apple Silicon macOS production verification, including a launchable `.app` bundle and generated DMG.
- Project-owned cross-platform CI preflight for locked dependency installation, TypeScript, unit tests, Rust tests, frontend builds, and native Tauri compilation on Windows, macOS, and Linux runners.
- Command-surface Apply safety audit.
- Backend Command Gating V1 for externally callable legacy file-changing commands and client-forged ApplyPlan run/result/restore writes.
- Plan Hash / Provenance V1 for newly saved ApplyPlans: backend-computed SHA-256 preview identity, immutable provenance storage, and validation mismatch blocking.
- Platform Path Semantics V1 foundation with absolute canonical root identity, read-only root-local case-sensitivity probing, root-derived comparison keys, canonical containment, metadata states, and fail-closed handling when capabilities cannot be proven.
- Canonical Destination Validation V2 in the read-only ApplyPlan validator: source drift and destination conflicts now follow confirmed root semantics; missing or relative roots, parent traversal, physical root escape, external symlink escape, unreadable metadata, and duplicate destinations block without enabling execution. Internal symlinks that remain inside the confirmed root are accepted as preview-only paths.
- Phase 0 Home clarity baseline: first-run journey cards (`Scan Library` -> `Review Inbox` -> `Check Duplicates` -> `Review Updates` -> `Create Organization Preview`), a Home safety/status panel, consistent evidence labels, and explicit locked-action copy (`No files changed`, `Preview only`, `Apply not ready yet`, `Restore not ready yet`).

## Cross-platform verification remaining

- Inspect and record the hosted Windows, macOS, and Linux preflight triggered by commit `9354056`. The branch push is verified, but hosted job details require authenticated GitHub access and are not yet claimed.
- Run the existing Windows fixture desktop proof and smoke lanes on a real Windows host. The CI Windows runner verifies dependency installation, tests, builds, and native compilation, but it does not exercise the webdriver player journey.
- Verify Platform Path Semantics V1 and Canonical Destination Validation V2 on native Windows, including root-local case probing, Windows path forms, case-only source drift, duplicate destination detection, and symlink or reparse-point containment.
- Verify native app launch, file-manager behavior, and filesystem-aware ApplyPlan validation on a real Linux desktop host. The CI Linux runner provides compile/test coverage only.
- Add native macOS and Linux fixture desktop automation rather than treating the Windows webdriver lane as universal.

## Still blocked

- Real Organize Apply.
- Real Restore.
- User-file backup/restore execution.
- Delete, quarantine, replace, or cleanup flows on user files.
- Automatic duplicate cleanup.
- AI-only file decisions.
- Automatic update replacement.
- Any frontend-created result-log or restore-entry writes for real execution.

## Current safety contract

The main safety and platform-boundary docs are:

- `docs/PLATFORM_BOUNDARY_AUDIT_V1.md`
- `docs/TRUST_BOUNDARIES_AND_AUTOMATION_READINESS.md`
- `docs/planning/APPLY_SAFETY_CONTRACT_V1.md`
- `docs/planning/VALIDATION_CONFLICT_PREVIEW_V1.md`
- `docs/planning/DRY_RUN_APPLY_DESIGN_V1.md`
- `docs/planning/CONFIRMATION_DESIGN_V1.md`
- `docs/COMMAND_SURFACE_APPLY_SAFETY_AUDIT_V1_REPORT.md`

Required direction before real Apply:

1. Keep legacy file-changing Tauri commands out of current user flows.
2. Require backend-owned ApplyPlan identity and immutable plan hash. ✅ V1 implemented for newly saved plans; future confirmation token work must still consume it.
3. Require backend-issued confirmation token.
4. Re-run validation immediately before execution.
5. Enforce canonical root checks at execution time.
6. Revalidate the saved folder-configuration snapshot and context trail before any confirmation token.
7. Create backup/restore material before any user-file mutation.
8. Write per-file result logs only from backend-observed operations.
9. Keep delete/quarantine/replace out of the first visible Apply release.

## Current validation lane

Run before claiming a branch is ready:

```bash
pnpm exec tsc --noEmit
pnpm run test:unit
pnpm run build
pnpm run test:rust
```

For focused Windows desktop proof:

```bash
pnpm run desktop:proof:fixtures -- --SkipBuild
```

For broader Windows desktop regression proof:

```bash
pnpm run desktop:smoke:fixtures
```

For a native package on the current operating system:

```bash
pnpm run tauri:build
```

## Recommended next sprint

Complete native proof for **Platform Path Semantics V1** and **Canonical Destination Validation V2**, then extend the shared identity model incrementally. Do not begin executor work from Mac-only evidence.

The read-only ApplyPlan validator now derives source drift, destination containment, destination occupancy, and duplicate destination identity from the confirmed root's filesystem behavior. It rejects missing or relative roots, parent traversal, physical and symlink escape, cross-root movement, unreadable metadata, and unprovable case behavior. Apply and Restore remain unavailable.

Next work:

- run the final path-semantics and ApplyPlan validation suite on native Windows, including drive paths, UNC or supported extended path forms, case-only collisions, and reparse-point containment;
- run the same validation on a native Linux desktop and record case-sensitive behavior plus file-manager smoke evidence;
- inspect authenticated hosted Windows, macOS, and Linux workflow results before claiming hosted CI proof;
- integrate the shared root-derived comparison key into duplicate identity, scanner folder keys, Library folder queries, Downloads intake, and hidden move preflight in small verified batches;
- add Installation Profile V1 and the Sims 4 game-adapter boundary so root identity is profile-relative rather than one global Mods/Tray assumption;
- preserve Backend Command Gating V1: legacy file-changing commands and client-forged ApplyPlan run/result/restore writes must keep failing closed;
- keep Apply, Restore, backup execution, result logs, restore logs, delete, quarantine, replace, cleanup, and automatic mutation unavailable.

The detailed findings and required tests are in `docs/PLATFORM_BOUNDARY_AUDIT_V1.md`. A hidden fixture-only executor and undo prototype remain later work, after native path and profile proof.

## Recent backend safety gate

Backend Command Gating V1 is now implemented at the Tauri command boundary. Current safe review commands remain callable, but externally callable legacy file-changing commands and client-forged ApplyPlan run/result/restore writes fail closed before doing work. This gate does not enable Apply or Restore; it only makes the previous UI-only boundary harder to bypass.

## Recent backend provenance gate

Plan Hash / Provenance V1 is now implemented for newly saved ApplyPlans. The backend computes a SHA-256 hash over a canonical preview identity that includes source scope, preview-time folder configuration, context trail, saved item/source/destination data, item signals/blockers, source file snapshots, and active rule/seed/settings version evidence. Provenance distinguishes `backend_generated_sorting_preview` from `client_supplied_preview`; future confirmation must accept only backend-generated provenance. Full provenance stays server-side; normal frontend responses expose only hash metadata. The hash/provenance columns are immutable once set. Validation blocks future confirmation if the stored hash/provenance is missing or no longer matches the persisted preview snapshot. This does not enable Apply, Restore, backup execution, confirmation tokens, result logs, restore logs, folder creation, or file mutation.
