# SimSuite implementation status

Last updated: 2026-05-26

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
- Organize sorting preview generation.
- Saved draft ApplyPlan records.
- Read-only ApplyPlan validation preview.
- Read-only Recovery history metadata display.
- Read-only dry-run preview in saved plan details.
- Fixture-only backend proof work for future backup/restore logic.
- Windows desktop proof/smoke scripts.
- Validation wrappers for WSL/Windows test reliability.
- Command-surface Apply safety audit.

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
- `docs/COMMAND_SURFACE_APPLY_SAFETY_AUDIT_V1_REPORT.md`

Required direction before real Apply:

1. Keep legacy file-changing Tauri commands out of current user flows.
2. Require backend-owned ApplyPlan identity and immutable plan hash.
3. Require backend-issued confirmation token.
4. Re-run validation immediately before execution.
5. Enforce canonical root checks at execution time.
6. Create backup/restore material before any user-file mutation.
7. Write per-file result logs only from backend-observed operations.
8. Keep delete/quarantine/replace out of the first visible Apply release.

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

Build **Confirmation Design V1** as a read-only UI/product contract:

- show exact future operations;
- show blockers and skipped items;
- show backup and restore expectations;
- introduce plan hash/provenance language;
- keep confirmation disabled;
- do not execute Apply, Restore, backup, result logs, or restore logs.

After that, move to backend command gating and fixture-only executor work.
