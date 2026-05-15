# Apply Safety Contract v1

Date: 2026-05-15

This document defines what must be true before SimSuite may expose any future
Apply workflow that changes real user files.

Apply is not implemented by this document. This is a safety contract for future
work.

ApplyPlan persistence planning now lives in
`docs/planning/APPLYPLAN_PERSISTENCE_AUDIT_V1.md`. That audit is the
source-of-truth design for how future reviewed plans, blocked items, evidence
snapshots, validation/conflict results, backup/restore references, and per-file
result logs should be stored. It does not add a migration, command, UI, or real
Apply workflow.

Current implementation note: the first DB-only ApplyPlan persistence foundation
now exists for draft/preview records. It can save, list, view, and soft-cancel
preview snapshots, but it still does not expose Apply, move files, create result
logs, or provide restore execution.

Current implementation note: the first backend-owned ApplyPlan builder now
exists. It builds a saved draft ApplyPlan from the existing read-only sorting
preview generator and persistence foundation. It still does not expose Apply,
move files, create result logs, or provide restore execution.

Current implementation note: Organize now has a visible `Saved plans` review
surface for draft preview records. It can save a generated preview plan through
the backend-owned builder, list saved drafts, show item evidence/blockers, and
cancel drafts. Saving or cancelling a draft does not move, copy, delete,
replace, clean up, quarantine, or change user files.

Current planning note: validation/conflict preview design now lives in
`docs/planning/VALIDATION_CONFLICT_PREVIEW_V1.md`. The design defines how
future saved drafts must be checked for stale sources, missing roots, unsafe
destinations, destination conflicts, review-only blockers, and backup/restore
requirements before any future confirmation work. It does not implement Apply
or change files.

## 1. Why Apply Needs A Contract

Sims 4 Mods and Tray folders are user-owned data. A bad file tool can break a
working mod setup, lose user organization, overwrite working files, or make
recovery unclear.

SimSuite must not become an auto-fix tool. It can help users understand files,
generate preview plans, and explain caveats, but real file changes need a
separate safety layer.

Any future Apply workflow must be:

- previewed before anything changes.
- explicitly confirmed by the user.
- limited to items with enough evidence.
- recoverable through backup or restore behavior.
- logged per file.
- safe when interrupted or partially failed.

## 2. Allowed Future Apply Scope

After the safety contract is implemented and proven, a future Apply workflow may
eventually:

- move selected files according to a reviewed organization plan.
- create destination folders only when those folders are shown in the preview.
- leave blocked, review-only, and weak heuristic items untouched.
- write a per-file result log.
- store enough recovery data to restore or undo what this Apply run changed.
- show a result summary after the run.

Apply v1 should be narrow. It should start with a selected, reviewed plan rather
than whole-Library automation.

## 3. Not Allowed

Future Apply must not:

- delete files.
- quarantine files.
- replace files.
- auto-update files.
- decide dependency truth.
- decide missing mesh truth.
- apply review-only suggestions.
- apply weak heuristic suggestions.
- apply AI-only suggestions.
- run without a preview.
- run without explicit user confirmation.
- run without backup or restore design.
- claim a file is safe to move, safe to delete, or safe to replace.

## 4. Required Future Apply Data Model

These are proposed model shapes for future design work. They are examples only
and are not implemented by this sprint.

```ts
type ApplyPlan = {
  id: string;
  sourcePlanId: string;
  createdAt: string;
  status:
    | "draft"
    | "ready_for_confirmation"
    | "blocked"
    | "applied"
    | "failed"
    | "restored";
  wouldTouchFiles: true;
  totalItems: number;
  applyableItems: number;
  blockedItems: number;
  backupRequired: true;
  restoreAvailable: boolean;
  confirmationRequired: true;
  caveats: string[];
  items: ApplyPlanItem[];
};

type ApplyPlanItem = {
  id: string;
  fileId: number;
  currentPath: string;
  destinationPath: string;
  actionKind: "move";
  evidenceLevel: "deterministic" | "evidence_backed";
  blocked: boolean;
  blockedReasons: string[];
  backupPath: string | null;
  result: "pending" | "applied" | "skipped" | "failed" | "restored";
  error: string | null;
};
```

Important rules:

- `ApplyPlan` is different from `StagingPlan`.
- `StagingPlan` remains preview-only with `wouldTouchFiles=false`.
- `ApplyPlan` may have `wouldTouchFiles=true`, but only after safety validation
  passes.
- Review-only and heuristic-only items must be blocked from Apply.

## 5. Required Future Apply Flow

Future Apply must follow this order:

1. Generate a preview plan.
2. User reviews the plan.
3. SimSuite filters out blocked and review-only items.
4. SimSuite builds an `ApplyPlan`.
5. SimSuite validates source and destination paths.
6. SimSuite checks destination conflicts.
7. SimSuite shows exact changes.
8. User confirms.
9. SimSuite creates backup or restore records.
10. SimSuite applies changes one by one.
11. SimSuite logs each result.
12. SimSuite shows a result summary.
13. SimSuite offers restore or undo if supported.

No step may silently skip preview, confirmation, backup/restore, path
validation, conflict handling, or result logging.

## 6. Required Path Validation

Future Apply path validation must require:

- source path is under a configured safe root or known app-local intake root.
- destination path is under a configured Mods or Tray root.
- no path traversal.
- no unreviewed cross-root moves.
- no cross-drive surprises unless explicitly supported and tested.
- no overwriting without conflict handling.
- no moving unsupported files unless a future design explicitly supports them.
- no moving review-only or blocked items.
- no unsafe symlink or reparse-point traversal unless explicitly supported and
  tested.

If path validation cannot prove a path is within the allowed boundary, Apply
must block that item.

## 7. Required Backup / Restore Behavior

Minimum future behavior:

- record the original path before each move.
- store a restore map for every changed file.
- verify backup data exists before moving if a copy-backup model is used.
- handle partial failures.
- restore only files changed by the same Apply run.
- never delete backups automatically in v1.
- keep result logs readable by the user.

Existing snapshot and restore code is useful prior art, but it is not by itself
enough to expose broad Apply. The future workflow still needs a user-facing
contract, path validation, conflict handling, and proof.

## 8. Required Conflict Handling

Future Apply must define behavior for:

- destination already exists.
- duplicate filename.
- destination directory missing.
- read-only file.
- permission denied.
- file in use.
- source missing.
- disk full.
- interrupted Apply.
- partial success.

The default v1 behavior should block or skip conflicted items and report the
reason, not guess.

## 9. Required UI Rules

Future Apply UI must show:

- `No files changed yet` before confirmation.
- exact source and destination paths.
- blocked items.
- review-only items excluded from Apply.
- backup/restore status.
- explicit confirmation step.
- result log after a run.
- restore option if supported.

Future Apply UI must not say:

- `safe to delete`
- `safe to replace`
- `fixed`
- `auto-fixed`
- `AI verified`
- `broken mod`
- `missing dependency`
- `missing mesh`

## 10. Required Tests And Proof

Future Apply implementation must add tests for:

- path validation.
- blocked items never applying.
- review-only items never applying.
- weak heuristic suggestions never applying.
- backup failure blocking Apply.
- destination conflicts.
- partial failure logging.
- restore map correctness.
- no mutating command called from preview UI.
- no unconfirmed file changes.

Future desktop proof must show:

- preview.
- confirmation.
- Apply blocked states.
- per-file result logs.
- restore behavior where supported.
- no enabled delete, quarantine, cleanup, or replacement workflow.

## 11. Existing Systems Required Before Apply

Future Apply work must follow `docs/planning/EXISTING_SYSTEMS_INTEGRATION_CONTRACT_V1.md`.

An ApplyPlan must reuse existing SimSuite evidence before adding any new logic:

- `StagingPlan` or sorting preview plan items for the reviewed source plan.
- Library file identity, source roots, current paths, size/date, and indexed hash or fingerprint evidence where available.
- duplicate detector truth for exact duplicate blockers or duplicate-review context.
- review queue, parser warning, inspection warning, and weak-metadata signals as blockers or caveats.
- Updates/watch state as review context only, not replacement proof.
- scanner-owned folder metadata and configured Mods/Tray roots for path validation.
- Inbox intake state only when the source content is a reviewed app-local intake batch.

ApplyPlan work must not create new unreviewed file lists, duplicate proof,
folder truth, update-source truth, or direct UI move actions. If a future Apply
sprint needs new data, its final report must explain the existing systems
reused and why the new data or logic was necessary.

## 12. Future Implementation Phases

Phase A - Apply Safety Contract Design

- Current sprint.
- Define the safety rules and audit existing file-changing paths.

Phase B - ApplyPlan Persistence Audit

- Design saved plan and result-log persistence without moving files.
- Decide whether a migration is needed.

Phase C - ApplyPlan Builder

- Build `ApplyPlan` from `StagingPlan`.
- Still no file movement.
- Validate evidence levels, blocked reasons, paths, and conflicts.

Phase D - Backup / Restore Prototype

- Backend-only fixture prototype.
- Test backups, restore maps, partial failures, and interruption behavior.

Phase E - User-Confirmed Apply Prototype

- Hidden or fixture-only.
- Small scope.
- Requires explicit confirmation.

Phase F - Visible Apply v1

- Only after proof.
- Strict scope.
- No delete, quarantine, replacement, update replacement, AI-decided actions, or
  safe-delete claims.
