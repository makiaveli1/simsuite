# Dry-run Apply Design v1

Date: 2026-05-24

Dry-run Apply is a read-only rehearsal. It explains what SimSuite would try
later, what it would skip, and why Apply is still not allowed.

No files changed. Apply is not ready yet. Restore is not ready yet. The first
backend/API-only `preview_apply_plan_dry_run` command now exists and remains
read-only. It does not add a migration, UI, confirmation workflow, Apply,
Restore, backup execution, restore execution, DB writes, result-log creation,
restore-entry creation, file movement, file copying, folder creation, cleanup,
delete, quarantine, replacement, auto-sort execution, fixture helper exposure,
or AI decision.

## 1. Purpose

Dry-run Apply is a future explanation layer between validation preview and any
later confirmation design.

It should answer:

- what would be attempted later if all future safety gates existed.
- what would be skipped.
- what remains blocked.
- what backup, restore-map, result-log, and confirmation data would still be
  required.
- why the saved draft still cannot change files.

Dry-run is not Apply. Dry-run is not confirmation.

## 2. Current State

Current SimSuite state:

- saved draft ApplyPlans exist.
- saved draft items, source signals, and blockers exist.
- Organize can create, list, inspect, validate, and cancel saved drafts.
- validation preview exists and remains read-only.
- validation preview keeps `canProceedToConfirmation=false`.
- `preview_apply_plan_dry_run` exists as a backend/API-only read-only command.
- dry-run v1 returns `canProceedToApply=false` and
  `canProceedToConfirmation=false`.
- dry-run v1 writes no DB rows and creates no result logs, restore entries,
  backups, folders, or file changes.
- Organize `Saved plans` now includes a read-only `Dry-run preview` UI for
  selected saved drafts. It calls `previewApplyPlanDryRun` only from the
  user-triggered preview control, displays cautious classification groups, and
  does not create dry-run state, result logs, restore entries, backups,
  folders, Apply, Restore, or confirmation.
- recovery history exists and remains read-only.
- recovery history displays existing DB-only run, result, and restore-map
  metadata.
- result/restore metadata tables exist.
- fixture-only backup proof exists for temporary test files only.
- fixture-only restore proof exists for temporary test files only.
- fixture-only backup + restore integration proof exists for temporary test
  files only.
- fixture-only recovery helpers are private backend test/prototype code.
- real Apply does not exist.
- real Restore does not exist.
- confirmation workflow does not exist.
- user-file backup execution does not exist.
- user-file restore execution does not exist.

## 3. Existing systems reused

| System | What it already provides | How dry-run Apply should use it | What not to duplicate |
| --- | --- | --- | --- |
| Saved ApplyPlan records | Draft metadata, counts, caveats, source scope, and safety flags | Parent record for a future dry-run preview | Do not create a second saved-plan model. |
| ApplyPlan items | Saved file id, source snapshot, destination preview string, evidence level, blocked/review-only state | One dry-run item should map to one saved item | Do not build items from frontend-only arrays. |
| ApplyPlan item signals | Evidence snapshots explaining why a suggestion exists | Explain why a future action is only a candidate | Do not reclassify files during dry-run. |
| ApplyPlan item blockers | Saved blocked/review-only/source-plan reasons | Force skipped or blocked dry-run statuses | Do not silently drop blockers. |
| Validation preview output | Stale source, missing source, root, destination, conflict, review-only, and backup blockers | Dry-run must depend on validation output before classifying possible future actions | Do not replace validation with dry-run. |
| Result/restore schema | DB-only run, result-log, and restore-map metadata foundation | Dry-run can reference required logging/recovery data | Do not create result or restore rows during dry-run v1. |
| Recovery history UI | Read-only display of existing run/result/restore metadata | Future dry-run UI should sit near it without creating history | Do not create a frontend-only result log or restore map. |
| Fixture-only backup/restore proof | Temporary-file-only backup and restore safety proof | Inform future recovery requirements | Do not expose fixture helpers or call them from dry-run. |
| Library identity/current paths | Indexed `files.id`, current path, source root, size/date/hash where available | Future dry-run can reference validation's current evidence | Do not rescan or reparse files. |
| Scanner roots/settings | Configured Mods/Tray roots and scan-owned folder facts | Bound future source/destination explanations through validation | Do not infer roots from raw strings. |
| Apply Safety Contract | Required preview, validation, confirmation, backup/restore, result logs, and proof | Gate dry-run wording and prevent Apply readiness claims | Do not skip safety steps. |
| Existing Systems Integration Contract | Reuse rules and anti-duplication boundaries | Keep dry-run backend-owned and evidence-backed | Do not add duplicate truth or UI-owned validation. |

## 4. Dry-run Scope

Future dry-run may check and explain:

- the source item is still in the saved plan.
- validation preview has no unresolved validation blocker for the item.
- the item is not review-only.
- the item has a destination preview string.
- the item has no destination conflict in validation preview.
- backup/restore is still required.
- result-log schema exists.
- restore-map schema exists.
- confirmation is still missing.
- Apply remains blocked.

Future dry-run must not:

- move files.
- copy files.
- create folders.
- create backups.
- create restore entries.
- modify result logs.
- update Library file rows.
- mark plans Apply-ready.
- call AI.
- call move-engine Apply paths.
- call fixture backup or restore helpers.
- create a confirmation token.

## 5. Dry-run Status Model

| Status | Meaning |
| --- | --- |
| `not_run` | No dry-run preview has been produced. |
| `blocked` | The plan or item has a blocker that prevents future action discussion. |
| `would_skip` | A future Apply would skip the item unless the saved plan changes. |
| `would_require_review` | Manual review or stronger evidence is required before future confirmation. |
| `would_require_backup` | Copy-backup-first recovery remains required before any future file change. |
| `would_require_confirmation` | The item could only be discussed after a future explicit confirmation design exists. |
| `would_require_destination_review` | Destination path, root, parent, or conflict state still needs review. |
| `candidate_after_future_safety_gates` | No current dry-run blocker is known, but the item is only a candidate after validation, backup, restore map, result log, confirmation, and proof exist. |
| `error` | Dry-run preview could not complete and must surface the error as a blocker. |

Forbidden status names:

- `ready_to_apply`
- `safe_to_move`
- `safe`
- `approved`

## 6. Dry-run Item Model

These model shapes are design-only.

```ts
type ApplyPlanDryRunPreview = {
  planId: number;
  status: "not_run" | "blocked" | "preview_only";
  canProceedToApply: false;
  canProceedToConfirmation: false;
  checkedAt: string;
  summary: {
    totalItems: number;
    candidateItems: number;
    skippedItems: number;
    blockedItems: number;
    reviewOnlyItems: number;
    conflictItems: number;
    backupRequiredItems: number;
  };
  caveats: string[];
  items: ApplyPlanDryRunItem[];
};

type ApplyPlanDryRunItem = {
  itemId: number;
  fileId: number | null;
  fileName: string;
  dryRunStatus: string;
  actionPreview: "would_move_later" | "would_skip" | "no_action";
  sourcePath: string | null;
  destinationPath: string | null;
  reasons: string[];
  blockers: string[];
  requiredBeforeApply: string[];
  canApply: false;
};
```

`candidate_after_future_safety_gates` must never be displayed as ready, safe,
approved, or Apply-ready.

## 7. Implemented Command v1

Implemented first command:

```text
preview_apply_plan_dry_run
```

Input:

```ts
type PreviewApplyPlanDryRunRequest = {
  planId: number;
};
```

Output:

```ts
type PreviewApplyPlanDryRunResponse = ApplyPlanDryRunPreview;
```

The command is:

- read-only.
- response-only in v1.
- backend-owned.
- based on saved ApplyPlan records and validation preview output.
- covered by tests proving `canProceedToApply=false`.
- covered by tests proving `canProceedToConfirmation=false`.
- covered by tests proving item `canApply=false`.
- covered by tests proving no result-log or restore-entry rows are created.

The command must not:

- write DB rows in v1.
- create result logs.
- create restore entries.
- create backups.
- create folders.
- move files.
- call Apply.
- call confirmation.
- call fixture backup or restore helpers.

Future UI remains separate work.

## 8. Relationship to Validation Preview

Validation preview checks whether saved paths, roots, conflicts, and blockers
are safe enough to discuss.

Dry-run preview explains how a future Apply would classify items after
validation.

Dry-run must depend on validation output. It must not replace validation. It
must keep `canProceedToConfirmation=false` until a future confirmation design,
backup/restore execution, result logging, and proof exist.

## 9. Relationship to Recovery History

Recovery history shows existing DB-only run, result, and restore-map metadata.

Dry-run must not create recovery history records. It may mention that future
confirmed Apply would need result logs and restore-map rows, but it must not
write them.

Future confirmed Apply may create result/restore records only after a separate
confirmation and executor contract exists.

## 10. Relationship to Confirmation

Dry-run happens before confirmation.

Confirmation design remains future work. Dry-run must not produce a
confirmation token, enable Apply, or mark a plan as ready.

Dry-run may list what confirmation would need later:

- exact operation preview.
- validation proof.
- destination conflict proof.
- backup strategy.
- restore-map requirement.
- result-log requirement.
- recoverable error behavior.
- explicit user confirmation copy.

## 11. UI Recommendation

Future UI should show:

- `Dry-run preview`.
- `No files changed`.
- `Apply is not ready yet`.
- `Future confirmation blocked`.
- grouped items:
  - `would be skipped`.
  - `blocked`.
  - `needs review`.
  - `candidate after future safety gates`.
- required safety steps:
  - validation.
  - backup.
  - restore map.
  - confirmation.
  - result log.

Future UI must not show an Apply, Restore, Backup, move, delete, cleanup,
quarantine, replacement, or auto-sort execution control.

## 12. Test Strategy

Future implementation tests must prove:

- dry-run never calls mutating commands.
- dry-run never calls fixture backup/restore helpers.
- dry-run returns `canProceedToApply=false`.
- dry-run returns `canProceedToConfirmation=false`.
- blocked items stay blocked.
- review-only items are skipped.
- missing destination items are skipped.
- destination conflicts are skipped.
- backup required remains a blocker.
- result-log missing remains a blocker.
- no UI shows Apply, Restore, Backup, move, delete, cleanup, quarantine, or
  auto-sort controls.

## 13. Recommended Implementation Phases

Phase A - dry-run design:

- completed before the command sprint.
- docs and guard test only.

Phase B - read-only dry-run command:

- implemented as backend/API only.
- no UI.
- no DB writes in v1.
- no Apply or confirmation.

Phase C - dry-run UI inside Organize:

- read-only.
- no Apply button.
- no result/restore writes.

Phase D - confirmation design:

- design-only.
- no real Apply.

Phase E - fixture-only confirmed Apply prototype:

- hidden/test-only.
- temporary fixtures only.

Phase F - visible Apply v1:

- only after validation, dry-run, confirmation, backup/restore execution,
  result logs, recoverable errors, restore limitations, tests, and desktop
  proof are complete.
