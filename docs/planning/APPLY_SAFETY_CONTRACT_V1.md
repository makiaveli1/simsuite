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

Current implementation note: the first `preview_apply_plan_validation` command
now exists as a response-only readiness preview for saved draft records. It
reports blockers and conflicts, always returns `canProceedToConfirmation=false`,
does not persist validation state in v1, and does not move, copy, delete,
create folders, create backups, or expose Apply.

Current implementation note: Organize `Saved plans` now shows validation
preview results from the read-only backend command. The UI explains `No files
changed`, shows friendly blocker/conflict labels, and keeps future confirmation
blocked. It does not expose Apply or any file-changing control.

Current planning note: backup/restore/result-log design now lives in
`docs/planning/BACKUP_RESTORE_RESULT_LOG_DESIGN_V1.md`. The design recommends a
future copy-backup-first recovery model with explicit restore maps and per-file
result logs. It does not add migrations, commands, backup execution, restore
execution, Apply UI, or file-changing behavior.

Current implementation note: the DB-only result/restore schema foundation now
exists. `apply_plan_runs`, `apply_plan_results`, and
`apply_plan_restore_entries` can store future run, result-log, and restore-map
metadata only. They do not execute backup, restore, Apply, file movement, file
copying, folder creation, deletion, cleanup, quarantine, replacement, or AI
decisions.

Current implementation note: the fixture-only backup prototype now proves
copy-backup-first mechanics against temporary test files only. It verifies a
backup copy and records safe result/restore metadata, but it is not user-facing
and does not execute real Apply, user-file backup, restore, movement, deletion,
cleanup, quarantine, replacement, or AI decisions.

Current implementation note: the fixture-only restore prototype now proves
restore-copy mechanics from recorded backup references against temporary test
files only. It verifies a restored copy and records safe result/restore
metadata, but it is not user-facing and does not execute real Apply, user-file
backup, user-file Restore, movement, deletion, cleanup, quarantine,
replacement, or AI decisions.

Current implementation note: the fixture-only backup + restore integration
proof now runs the private backup and restore prototypes as one temporary-file
chain. It verifies backup and restored copies, checks restore-map scope for one
run/plan/item/result context, records only safe DB metadata, and still does not
execute real Apply, user-file backup, user-file Restore, movement, deletion,
cleanup, quarantine, replacement, or AI decisions.

Current implementation note: the first hidden fixture transaction coordinator
is now compiled only for Rust tests. It loads one persisted unblocked `move`
ApplyPlan item and its indexed source row, requires `fixtureMode=true`, confines
source, destination, backup, and undo paths to one supplied temporary fixture
root, and requires the live source path, size, and SHA-256 hash to match the
indexed evidence. It then reuses the existing fixture backup prototype, requires
the backup to verify before calling the existing single-file move primitive,
rechecks source and destination state immediately before the move, verifies the
moved destination against the pre-move hash and size, and records only
`pending_log` / `design_only` metadata. Its bounded undo path accepts only the
same recorded plan/run/item/result scope, verifies the moved destination has
not changed, restores the original source from the recorded verified backup,
verifies that restored source, and only then removes the matching temporary
moved copy. Tests cover fixture-mode gating, fixture-root escape, destination
collision, missing or changed source, recovery-scope mismatch, and the full
backup -> move -> verify -> undo chain. The coordinator is not a Tauri command,
is not compiled into normal builds, does not create confirmation tokens, does
not refresh the production Library index, and does not enable real Apply,
Restore, user-file mutation, cleanup, delete, quarantine, or replacement.
Deterministic fixture-only interruption tests now cover the transaction phase
boundaries. A hidden read-only reconciliation classifier uses only the existing
ApplyPlan item, indexed source evidence, run/result/restore records, verified
backup bytes, and current fixture bytes; no new schema was required. It fails
closed unless all plan/run/item paths, hash/size evidence, fixture containment,
and record links agree exactly. The classifier distinguishes a clean start,
verified-backup resume requirement, moved file missing its result log, move
result missing its restore-map entry, fully recorded move awaiting undo,
interrupted undo cleanup, completed undo, and ambiguous/tampered states.

The hidden reconciler remains narrower than a normal transaction executor.
When a verified move already happened but its metadata was interrupted, it can
repair only the missing `pending_log` / `design_only` result or restore-map
record after proving the destination still exactly matches the verified backup;
it never moves the file a second time. When undo already restored the exact
source but cleanup was interrupted, it can re-verify both identical copies and
remove only the moved duplicate; a second reconciliation is then a no-op.
Tampered bytes, conflicting or multiple records, missing evidence, and
cross-scope requests remain blocked or ambiguous. The first classifier run
exposed a macOS `/var` versus `/private/var` path-spelling mismatch; resolved
canonical paths are now compared while keeping the same fixture-root containment
checks.

A separate hidden fixture-only resume proof now consumes only the exact
`ResumeFromVerifiedBackupRequired` state. It requires the same plan/run/item,
the same verified backup result and restore-entry identities, the same indexed
source hash/size, an exact source and backup, an absent destination, and no move
or undo metadata. After the first classification it deliberately re-reads the
complete database and filesystem evidence, then re-hashes backup/source and
rechecks destination immediately beside the existing single-file move call.
Deterministic tests prove that a destination, source change, backup change, or
competing move metadata appearing after the first classification blocks the
resume without moving or overwriting the observed fixture bytes. A successful
resume moves exactly once, verifies destination bytes before recording the same
`pending_log` / `design_only` metadata, becomes `MoveRecordedAwaitingUndo`, is a
no-op on repeated resume, and remains undoable through the existing verified
fixture undo path.

This is destination-race hardening, not an atomic no-overwrite guarantee. The
existing production `move_single_file` helper still has a small operating-system
check-to-rename window; on platforms where rename can replace a destination, a
file appearing after the last explicit check could still race the primitive.

A separate standalone fixture proof now explores one no-replace candidate
without routing any transaction through it: `std::fs::hard_link` claims the
planned destination name first, the claimed destination is verified against the
expected hash/size, and only then is the source name unlinked. On the current
macOS fixture filesystem, an already-existing destination is refused without
changing either file, and a 16-contender simultaneous claim test produces exactly
one winner while preserving the source/destination bytes. A successful proof
verifies the destination before source unlink. A forced verification failure
removes only the newly-created destination link after confirming both names still
refer to the same physical file, leaving the source intact.

The hard-link proof also exposes a new interruption state rather than hiding it.
If execution stops after the destination link is created but before the source
name is removed, source and destination are two names for the same physical file.
On the current macOS run, the fixture classifier proves that identity using Unix
device/inode metadata and reports `ExactHardLinkPairNeedsReview`; the reconciler
deliberately does nothing. An ordinary byte-for-byte copy uses a different
physical identity, and a symlink alias is also rejected from this special state;
both remain `Ambiguous`. Equal content alone is therefore not treated as proof
of an interrupted hard-link claim. This distinction is important because the current
records still do not prove that SimSuite, rather than another process/user,
created the hard-link pair.

This is a macOS runtime proof of an exclusive destination-claim technique, not a
chosen cross-platform production move primitive. Hard links require filesystem
support and normally require both names on the same filesystem. Windows and
Linux have not received equivalent native runtime proof in this development
environment. The candidate also has a deliberate two-name interval and still
needs durable transaction-intent evidence before automatic recovery could safely
remove either name. Cross-platform capability gating/native proof, concurrent
content-write handling, incremental only-affected index refresh, multi-file
transactions, golden content fixtures, and native player-environment proof
remain future gates before any real Apply or Restore surface is considered.

Current implementation note: Organize `Saved plans` now shows read-only
`Recovery history` metadata from DB-only ApplyPlan run logs, result logs, and
restore-map records. It keeps `No files changed`, `Apply is not ready yet`, and
`Restore is not ready yet` visible. It does not expose Apply, Restore, backup
execution, restore execution, fixture proof helpers, or file-changing controls.

Current planning note: dry-run Apply design now lives in
`docs/planning/DRY_RUN_APPLY_DESIGN_V1.md`. It defines a future read-only
rehearsal layer that explains what a saved draft would attempt later, what
would be skipped, and which safety gates are still missing. It does not add a
command, API, UI, confirmation workflow, Apply, Restore, backup execution,
restore execution, fixture helper exposure, or file-changing behavior.

Current implementation note: `preview_apply_plan_dry_run` now exists as a
backend/API-only read-only command. It reuses saved ApplyPlan records and the
validation preview, returns `canProceedToApply=false` and
`canProceedToConfirmation=false`, writes no DB rows, creates no result logs or
restore entries, creates no backups or folders, and does not move, copy,
delete, clean up, quarantine, replace, or auto-sort files.

Current implementation note: Organize `Saved plans` now shows a read-only
`Dry-run preview` section for selected saved drafts. It calls
`previewApplyPlanDryRun`, groups item classifications with cautious wording,
keeps `No files changed`, `Apply is not ready yet`, and `Future confirmation
blocked` visible, and exposes no Apply, Restore, Backup, confirmation,
file-changing, result-log write, or restore-entry write control.

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
