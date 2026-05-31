# SimSuite implementation status

Last updated: 2026-05-31

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
- Command-surface Apply safety audit.
- Backend Command Gating V1 for externally callable legacy file-changing commands and client-forged ApplyPlan run/result/restore writes.
- Plan Hash / Provenance V1 for newly saved ApplyPlans: backend-computed SHA-256 preview identity, immutable provenance storage, and validation mismatch blocking.
- Canonical Destination Validation V1 for saved ApplyPlans: read-only source/destination root checks, hostile-path rejection, duplicate/case-conflict detection, symlink/reparse-style blocking where detectable, and review-only gating for unsupported/heuristic items.
- Phase 0 Home clarity baseline: first-run journey cards (`Scan Library` -> `Review Inbox` -> `Check Duplicates` -> `Review Updates` -> `Create Organization Preview`), a Home safety/status panel, consistent evidence labels, and explicit locked-action copy (`No files changed`, `Preview only`, `Apply not ready yet`, `Restore not ready yet`).

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
5. Revalidate the saved folder-configuration snapshot and context trail before any confirmation token.
6. Require backend-issued confirmation token.
7. Create backup/restore material before any user-file mutation.
8. Write per-file result logs only from backend-observed operations.
9. Keep delete/quarantine/replace out of the first visible Apply release.

## Current validation lane

Run before claiming a branch is ready:

```bash
npx tsc --noEmit
npm run test:unit
npm run build
npm run test:rust
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

Build **Operation-set preview / confirmation readiness V1** before any executor work:

- consume the existing Plan Hash / Provenance V1 identity and Canonical Destination Validation V1 results;
- define the exact backend operation set that would be needed for a future Apply without executing it;
- keep `canProceedToConfirmation=false` until a backend-issued single-use token, backup/restore proof, execution-time validation, and result logging all exist;
- keep Backend Command Gating V1 in place: legacy file-changing commands and client-forged result/restore writes must keep failing closed;
- keep Apply, Restore, backup execution, result logs, restore logs, delete, quarantine, replace, and cleanup unavailable.

After that, move to backend-issued confirmation token design and then a hidden fixture-only executor prototype.

## Recent backend safety gate

Backend Command Gating V1 is now implemented at the Tauri command boundary. Current safe review commands remain callable, but externally callable legacy file-changing commands and client-forged ApplyPlan run/result/restore writes fail closed before doing work. This gate does not enable Apply or Restore; it only makes the previous UI-only boundary harder to bypass.

## Recent backend canonical destination gate

Canonical Destination Validation V1 is now implemented inside the read-only `preview_apply_plan_validation` flow. The backend rejects unsupported/malformed source and destination paths, parent-directory traversal (including Windows backslash variants), root-prefix spoofing, destinations outside configured Mods/Tray roots, missing destination parents, cross-root moves, duplicate destinations, case-only destination conflicts, existing destinations, symlink/reparse-style source escapes where detectable, symlink/reparse-style destination-parent escapes where detectable, unsupported grouped/non-move action kinds, and heuristic-only rows. This does not create folders, issue confirmation tokens, create backups, write result logs, or mutate files; it only reports readiness blockers while keeping `canProceedToConfirmation=false`.

## Recent backend provenance gate

Plan Hash / Provenance V1 is now implemented for newly saved ApplyPlans. The backend computes a SHA-256 hash over a canonical preview identity that includes source scope, preview-time folder configuration, context trail, saved item/source/destination data, item signals/blockers, source file snapshots, and active rule/seed/settings version evidence. Provenance distinguishes `backend_generated_sorting_preview` from `client_supplied_preview`; future confirmation must accept only backend-generated provenance. Full provenance stays server-side; normal frontend responses expose only hash metadata. The hash/provenance columns are immutable once set. Validation blocks future confirmation if the stored hash/provenance is missing or no longer matches the persisted preview snapshot. This does not enable Apply, Restore, backup execution, confirmation tokens, result logs, restore logs, folder creation, or file mutation.
