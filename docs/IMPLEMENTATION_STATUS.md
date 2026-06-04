# SimSuite implementation status

Last updated: 2026-06-04

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
- Read-only operation-set preview in saved plan details, including backend-owned saved-plan/hash checks, candidate operation summaries, and explicit `canApply=false` / confirmation-blocked safety copy.
- Backend-issued confirmation token V1 in saved plan details: token receipts are bound to backend-owned plan/hash/operation-set identity, remain single-use metadata, and keep `canExecute=false` / `canProceedToApply=false`.
- Hidden fixture-only Apply executor prototype: token-gated, backup-first, temp-root-only move proof with backend-observed prototype result/restore metadata; no public Tauri command and no user-file Apply.
- Hidden feature-gated real move executor design spike: compiled only with `apply-executor-real-move-spike`, additionally requires a runtime gate, revalidates backend-owned plan/hash/operation-set/token identity immediately before execution, accepts move-only operation previews, creates/verifies backup material before `rename`, records recovery metadata if a move fails after verified backup, includes a hidden run-scoped restore proof from backend restore entries only, writes backend-observed result/restore metadata plus backend-owned run status/counter transitions, and remains unregistered from the Tauri invoke handler.
- Fixture-only backend proof work for future backup/restore logic.
- Windows desktop proof/smoke scripts.
- Validation wrappers for WSL/Windows test reliability.
- Command-surface Apply safety audit.
- Backend Command Gating V1 for externally callable legacy file-changing commands and client-forged ApplyPlan run/result/restore writes.
- Plan Hash / Provenance V1 for newly saved ApplyPlans: backend-computed SHA-256 preview identity, immutable provenance storage, and validation mismatch blocking.
- Canonical Destination Validation V1 for saved ApplyPlans: read-only source/destination root checks, hostile-path rejection, duplicate/case-conflict detection, symlink/reparse-style blocking where detectable, and review-only gating for unsupported/heuristic items.
- Phase 0 Home clarity baseline: first-run journey cards (`Scan Library` -> `Review Inbox` -> `Check Duplicates` -> `Review Updates` -> `Create Organization Preview`), a Home safety/status panel, consistent evidence labels, and explicit locked-action copy (`No files changed`, `Preview only`, `Apply not ready yet`, `Restore not ready yet`).
- ApplyPlan persistence boundary cleanup through Phase E review checkpoint: strict JSON parsing, row mapping, preview snapshot repository, item persistence, hash persistence, record lifecycle, full-plan loading, draft saving, snapshot creation, snapshot consumption, backend builder orchestration, provenance hashing, save rejection, malformed JSON, and record lifecycle boundary tests are split/colocated behind focused backend modules; the remaining public persistence façade tests have been reviewed and retained as compatibility/integration coverage while public APIs remain stable.

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

The main safety docs are:

- `docs/TRUST_BOUNDARIES_AND_AUTOMATION_READINESS.md`
- `docs/planning/APPLY_SAFETY_CONTRACT_V1.md`
- `docs/planning/VALIDATION_CONFLICT_PREVIEW_V1.md`
- `docs/planning/DRY_RUN_APPLY_DESIGN_V1.md`
- `docs/planning/CONFIRMATION_DESIGN_V1.md`
- `docs/COMMAND_SURFACE_APPLY_SAFETY_AUDIT_V1_REPORT.md`

Required direction before real Apply:

1. Keep legacy file-changing Tauri commands out of current user flows.
2. Require backend-owned ApplyPlan identity and immutable plan hash. ✅ V1 implemented for newly saved plans; future confirmation token work must still consume it.
3. Re-run validation immediately before execution. ✅ Preview-time validation now includes Canonical Destination Validation V1, but execution-time validation still must re-run it.
4. Enforce canonical root checks at execution time. ✅ Read-only V1 implemented for preview validation; future executor must treat it as a prerequisite, not authorization.
5. Revalidate the saved folder-configuration snapshot and context trail before any confirmation token. ✅ V1 implemented through backend-owned plan/hash/provenance and operation-set checks.
6. Require backend-issued confirmation token. ✅ V1 implemented as locked, read-only receipt metadata; real Apply still unavailable.
7. Create backup/restore material before any user-file mutation. ✅ Fixture-only prototype proves backup-first behavior inside temp roots; hidden real-move spike proves backup-first behavior behind compile-time and runtime gates only, and records recovery metadata if `rename` fails after backup verification.
8. Write per-file result logs only from backend-observed operations. ✅ Fixture-only prototype and hidden real-move spike write backend-observed metadata; hidden restore proof reads paths from backend restore entries instead of caller-supplied arbitrary rollback paths; frontend-forged writes remain blocked.
9. Keep delete/quarantine/replace out of the first visible Apply release.

## Current validation lane

Run before claiming a branch is ready:

```bash
npx tsc --noEmit
npm run test:unit
npm run build
npm run test:rust
npm run test:rust:warnings
```

For focused desktop proof:

```bash
npm run desktop:proof:fixtures -- --SkipBuild
```

For broader desktop regression proof:

```bash
npm run desktop:smoke:fixtures
```

## Recommended next sprint

Move from hidden executor proof to a **throwaway-profile beta lane** before any visible Apply button:

- keep both fixture and hidden real-move executors hidden and unregistered as public Tauri commands;
- preserve token/hash/operation-set revalidation at execution time;
- keep backend-observed run counters/status transitions private to the hidden executor lane until a throwaway-profile beta proves the boundary;
- keep delete, quarantine, replace, update replacement, folder creation, and AI-only actions outside this lane;
- keep `canProceedToApply=false` in the current UI until the real executor, restore path, feature gate, and throwaway-profile beta lane are proven.

## Recent hidden feature-gated real move executor spike

The hidden real move executor spike is implemented in backend Rust behind the `apply-executor-real-move-spike` Cargo feature and a separate runtime gate. It is not registered in the Tauri invoke handler, so there is still no public Apply command. The spike requires a backend-issued confirmation token, rejects token reuse once backend result rows exist, re-runs the current operation-set preview with the token-bound plan hash, verifies the operation-set hash and allowed operation count, accepts only `WouldMoveLater` move previews, requires source/destination paths to stay inside the configured Mods/Tray root, rejects destination overwrites and cross-root moves, copies and hash/verifies backup material before `fs::rename`, verifies the destination bytes after the move, writes backend-observed result/restore rows, and transitions only the backend-owned run record (`applying` -> `applied` or `apply_failed`) with counters derived from observed operations. If a move fails after backup proof, it records a backend-observed failed-move result and recovery restore entry that preserves the original source path, destination path, backup path, hash, and size for audit/retry safety.

## Recent hidden run-scoped restore proof

The hidden run-scoped restore proof is also compiled only with `apply-executor-real-move-spike` and requires the separate runtime gate. It is not registered in the Tauri invoke handler, so there is still no public Restore command. The proof accepts a backend ApplyPlan run id, its matching confirmation token, and backend restore entry ids; it refuses arbitrary source/destination path input, rejects non-real-move restore rows, requires `not_restored` entries, revalidates destination bytes and backup bytes against the backend-recorded hash/size, requires paths to remain inside configured roots and backup root, then moves the recorded destination back to the recorded original source path. After byte verification it records a backend-observed restore result, marks the backend restore entry `restored`, and transitions only the backend-owned run record to `restored` while preserving apply counters. It does not enable visible Apply, visible Restore, delete, quarantine, replace, update replacement, folder creation, AI-only actions, arbitrary rollback, or frontend-created result/restore writes.

## Recent hidden fixture executor prototype

The hidden fixture-only Apply executor prototype is implemented in backend Rust tests only. It accepts only `fixtureMode=true`, requires a backend-issued confirmation token, revalidates the current plan hash and operation-set hash, refuses token reuse after prototype result rows exist, creates a backup before moving, moves only inside a caller-provided temporary fixture root, verifies destination and backup bytes, records backend-observed prototype result/restore metadata, and remains unregistered from the Tauri invoke handler. It does not enable real Apply, Restore, public backup execution, delete, quarantine, replace, cleanup, or user-file mutation.

## Recent backend confirmation token gate

Backend-issued Confirmation Token V1 is implemented as read-only locked receipt metadata. Tokens are bound to the backend-owned saved plan, plan hash, current operation-set hash, source plan kind, allowed operation count, issue/expiry timestamps, and single-use state metadata. The UI can display the receipt, but `canProceedToApply=false`, `canExecute=false`, and legacy/client-forged result/restore command gates remain in force.

## Recent backend safety gate

Backend Command Gating V1 is now implemented at the Tauri command boundary. Current safe review commands remain callable, but externally callable legacy file-changing commands and client-forged ApplyPlan run/result/restore writes fail closed before doing work. This gate does not enable Apply or Restore; it only makes the previous UI-only boundary harder to bypass.

## Recent backend canonical destination gate

Canonical Destination Validation V1 is now implemented inside the read-only `preview_apply_plan_validation` flow. The backend rejects unsupported/malformed source and destination paths, parent-directory traversal (including Windows backslash variants), root-prefix spoofing, destinations outside configured Mods/Tray roots, missing destination parents, cross-root moves, duplicate destinations, case-only destination conflicts, existing destinations, symlink/reparse-style source escapes where detectable, symlink/reparse-style destination-parent escapes where detectable, unsupported grouped/non-move action kinds, and heuristic-only rows. This does not create folders, issue confirmation tokens, create backups, write result logs, or mutate files; it only reports readiness blockers while keeping `canProceedToConfirmation=false`.

## Recent backend provenance gate

Plan Hash / Provenance V1 is now implemented for newly saved ApplyPlans. The backend computes a SHA-256 hash over a canonical preview identity that includes source scope, preview-time folder configuration, context trail, saved item/source/destination data, item signals/blockers, source file snapshots, and active rule/seed/settings version evidence. Provenance distinguishes `backend_generated_sorting_preview` from `client_supplied_preview`; future confirmation must accept only backend-generated provenance. Full provenance stays server-side; normal frontend responses expose only hash metadata. The hash/provenance columns are immutable once set. Validation blocks future confirmation if the stored hash/provenance is missing or no longer matches the persisted preview snapshot. This does not enable Apply, Restore, backup execution, confirmation tokens, result logs, restore logs, folder creation, or file mutation.
