# SimSuite Next Steps Execution Order

Date: 2026-05-31

## Chosen order

The chosen order is:

1. Stabilize and commit the current safety baseline.
2. Build Operation-set Preview / Confirmation Readiness V1.
3. Push toward real Apply by the fastest safe route: confirmation token, fixture-only executor, hidden move-only beta.
4. Do the UX/product beta polish pass after the backend path is clearer.

This order keeps SimSuite moving toward the feature users actually want — organized files — without pretending a preview hash or UI button is enough to safely change real Sims folders.

## Current baseline after stabilization

SimSuite is currently a review, preview, validation, and dry-run app.

Users can:

- scan and inspect their Library;
- review Inbox, duplicates, updates, and manual-review signals;
- generate organization previews;
- save backend-generated draft ApplyPlans;
- see plan hash/provenance metadata;
- run validation preview;
- run dry-run preview;
- see read-only recovery/result metadata.

Still blocked:

- real Apply;
- real Restore;
- user-file backup execution;
- moving, copying, deleting, quarantining, replacing, or cleaning real files;
- automatic duplicate cleanup;
- automatic update replacement;
- AI-only file decisions;
- frontend-created result logs or restore entries.

Plain-English safety rule:

> SimSuite may explain and rehearse a plan. It may not change user files until the backend can prove the exact operation set, issue a single-use authorization, back up first, log observed results, and restore only what that run changed.

---

# Phase 1 — Operation-set Preview / Confirmation Readiness V1

## User-facing goal

When a user opens a saved plan, SimSuite should show the exact list of future operations the backend would be willing to discuss later.

Not execute. Discuss.

The screen should answer:

- Which files are candidates for a future move?
- Which files would be skipped?
- Which files are blocked?
- Which files need manual review?
- What safety gates are still missing before Apply can exist?

## Why this comes next

Plan Hash / Provenance V1 answers:

> Is this the same reviewed plan?

Canonical Destination Validation V1 answers:

> Are the paths structurally safe enough to discuss?

Operation-set Preview V1 answers:

> What exact operations would the backend derive from that reviewed, validated plan?

Without this layer, a confirmation token would be vague authorization theatre.

## Backend scope

Create a focused backend module:

```text
src-tauri/src/core/apply_plan_operation_preview.rs
```

Add it to:

```text
src-tauri/src/core/mod.rs
```

Add a read-only Tauri command:

```text
preview_apply_plan_operations
```

Likely command wiring files:

```text
src-tauri/src/commands/mod.rs
src-tauri/src/lib.rs
```

## Request shape

```ts
type PreviewApplyPlanOperationsRequest = {
  planId: number;
  expectedPlanHash?: string;
};
```

The optional expected hash lets the frontend say, “I am looking at this exact plan hash,” but the backend still owns truth.

## Response shape

```ts
type ApplyPlanOperationPreview = {
  planId: number;
  planHash: string;
  sourceKind: "backend_generated_sorting_preview" | "client_supplied_preview" | string;
  checkedAt: string;
  canProceedToConfirmation: false;
  canProceedToApply: false;
  summary: {
    totalItems: number;
    candidateMoveItems: number;
    skippedItems: number;
    blockedItems: number;
    reviewOnlyItems: number;
    destinationConflictItems: number;
    backupRequiredItems: number;
  };
  requiredBeforeConfirmation: string[];
  requiredBeforeApply: string[];
  operations: ApplyPlanOperationPreviewItem[];
};

type ApplyPlanOperationPreviewItem = {
  itemId: number;
  fileId: number | null;
  fileName: string;
  sourcePath: string | null;
  destinationPath: string | null;
  operationKind: "move_later" | "skip" | "no_action";
  operationStatus:
    | "candidate_after_future_safety_gates"
    | "blocked"
    | "would_skip"
    | "would_require_review"
    | "would_require_destination_review"
    | "would_require_backup"
    | "not_eligible";
  evidenceLevel: string;
  reasons: string[];
  blockers: string[];
  requiredBeforeApply: string[];
  canApply: false;
};
```

Forbidden status names:

- `ready_to_apply`
- `safe_to_move`
- `approved`
- `confirmed_safe`

## Backend rules

The command must:

1. Load the saved ApplyPlan by ID.
2. Verify the plan hash/provenance before doing anything else.
3. Reject missing, empty, malformed, or mismatched provenance.
4. Reject `client_supplied_preview` as future-confirmation material.
5. Reuse the existing validation preview logic.
6. Derive operation candidates only from validation-clean, direct move suggestions.
7. Treat review-only, heuristic-only, unsupported, blocked, missing-source, unsafe-source, unsafe-destination, destination-conflict, and cross-root items as not eligible.
8. Return `canProceedToConfirmation=false`.
9. Return `canProceedToApply=false`.
10. Write no database rows.
11. Create no folders.
12. Create no backups.
13. Move/copy/delete/quarantine/replace nothing.
14. Never call legacy mutating commands or move-engine paths.

## Frontend scope

Update:

```text
src/lib/types.ts
src/lib/api.ts
src/screens/organize/SavedPlansReview.tsx
src/screens/OrganizeScreen.test.tsx
src/lib/api.test.ts
src/trustBoundaryCopy.test.ts
```

Saved Plans should show a new section:

```text
Operation preview
```

It should group rows as:

- Candidate later
- Skipped
- Blocked
- Needs review
- Destination review needed

Visible copy should say:

- `No files changed`
- `Apply is still locked`
- `Backup, confirmation, execution-time validation, and result logs are still required`

It must not show enabled Apply, Restore, Backup, Delete, Quarantine, Replace, Cleanup, or Commit controls.

## Tests to write first

Rust tests:

1. backend-generated saved plan returns operation preview.
2. client-supplied preview is rejected for operation preview.
3. mismatched expected plan hash is rejected.
4. tampered persisted plan hash/provenance blocks operation preview.
5. review-only item becomes `would_require_review` / not eligible.
6. heuristic-only item is not eligible.
7. unsupported file type is not eligible.
8. destination conflict is not eligible.
9. missing/unsafe source is not eligible.
10. no result-log or restore-entry rows are created.
11. operation preview production code avoids file-mutating calls.

Frontend tests:

1. Saved Plans renders operation preview groups.
2. operation preview shows locked safety gates.
3. no enabled Apply/Restore controls appear.
4. trust copy test rejects `safe to move`, `ready to apply`, `approved`, and similar overclaims.

## Verification lane

Focused:

```bash
cargo fmt --all --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml apply_plan_operation_preview --lib
cargo test --manifest-path src-tauri/Cargo.toml apply_plan --lib
npx tsc --noEmit
npm run test:unit -- src/screens/OrganizeScreen.test.tsx src/trustBoundaryCopy.test.ts src/lib/api.test.ts
```

Full:

```bash
npm run test:unit
npm run build
npm run test:rust
cargo check --manifest-path src-tauri/Cargo.toml
cargo fmt --all --check --manifest-path src-tauri/Cargo.toml
cargo clippy --all-targets --all-features --manifest-path src-tauri/Cargo.toml
git diff --check
```

## Definition of done

Phase 1 is done only when:

- `preview_apply_plan_operations` exists and is read-only.
- operation candidates come from backend-owned saved plans only.
- `client_supplied_preview` plans are rejected for future operation discussion.
- operation preview exposes exact candidate/skipped/blocked reasons.
- frontend displays the operation preview clearly.
- no Apply/Restore/file-changing controls are enabled.
- focused and full verification lanes pass fresh.

---

# Phase 2 — Fastest safe route toward real Apply

This phase is about moving quickly, not recklessly.

The fastest safe route is:

1. Backend-issued confirmation token.
2. Fixture-only executor.
3. Hidden real move-only executor behind a feature flag.
4. Narrow beta Apply.

Do not skip from operation preview to real file movement.

## Phase 2A — Confirmation Token V1

### User-facing goal

The user still cannot Apply yet, but SimSuite can explain the final locked gate:

> Confirmation will only become available for this exact operation set after validation, backup readiness, and execution proof exist.

### Backend goal

Create a backend authorization object, not a UI button state.

A token must bind to:

- plan ID;
- plan hash;
- operation-set hash;
- validation timestamp;
- source kind;
- allowed operation count;
- issuing time;
- expiry;
- single-use state.

### Required behavior

The token must:

- only issue for backend-generated preview provenance;
- reject client-supplied previews;
- reject stale/tampered/mismatched hashes;
- reject changed operation sets;
- expire quickly;
- be single-use;
- not execute anything;
- not create backup material by itself;
- not mark a plan applied.

### Likely files

```text
src-tauri/src/core/apply_plan_confirmation.rs
src-tauri/src/core/mod.rs
src-tauri/src/commands/mod.rs
src-tauri/src/lib.rs
src-tauri/src/models.rs
src/lib/types.ts
src/lib/api.ts
```

A small migration may be needed if tokens are persisted instead of purely derived/ephemeral.

## Phase 2B — Fixture-only Executor

### User-facing goal

No public UI yet.

This is developer proof that the executor can safely handle files before it touches user Sims folders.

### Backend goal

Create a private executor that only runs in temp fixture roots.

It must prove:

- backup happens before move;
- move happens only for eligible operation-set items;
- result logs are backend-observed;
- restore map is scoped to one run;
- restore only restores what that run changed;
- partial failure is explainable and recoverable;
- token reuse fails;
- path/root validation reruns at execution time.

### Forbidden

Still no user-file execution.

No delete, quarantine, replace, update replacement, cleanup, or AI-only decisions.

## Phase 2C — Hidden real move-only executor

### User-facing goal

Still hidden unless deliberately enabled for developer/throwaway-profile testing.

### Backend goal

Use the proven fixture executor pattern on real paths, but only behind a feature flag and only for move operations.

Required gates:

- backend-generated plan;
- matching hash/provenance;
- fresh validation;
- operation-set preview;
- confirmation token;
- backup first;
- restore map;
- backend-observed result logs;
- receipt;
- run-scoped restore.

## Phase 2D — First beta Apply

### Narrow beta promise

> SimSuite can move selected, reviewed, eligible files from one saved organization plan. It backs them up first, gives a receipt, and can restore that specific run.

Still excluded from beta:

- delete;
- quarantine;
- replace;
- cleanup;
- automatic duplicate cleanup;
- automatic update replacement;
- AI-only actions;
- review-only suggestions;
- heuristic-only suggestions;
- dependency/missing-mesh claims.

---

# Phase 3 — UX/product beta polish

This comes after Operation-set Preview and the Apply path are shaped, because then the UI has a real story to tell.

## User-facing goal

A non-technical Sims player should understand:

- where to start;
- what each page is for;
- what SimSuite knows;
- what SimSuite only suspects;
- what is blocked;
- how to get from scan to saved plan to operation preview;
- why Apply is locked until safety gates exist.

## UX work

Focus areas:

1. Home
   - clearer journey cards;
   - current safety state;
   - next best action.

2. Library
   - stronger distinction between facts, cues, and review-only hints;
   - clearer handoff to Duplicates, Updates, Review, and Organize.

3. Organize
   - make the flow obvious:
     - Create preview;
     - Save draft;
     - Validate;
     - Dry-run;
     - Operation preview;
     - Apply locked.

4. Saved Plans
   - reduce technical overload;
   - group blockers/reasons clearly;
   - keep hash/provenance visible as identity, not authorization.

5. Navigation
   - keep durable top-level pages:
     - Home;
     - Scan;
     - Library;
     - Inbox;
     - Organize;
     - Updates;
     - Review;
     - Settings.
   - later fold Creators/Types into Library lenses.
   - later fold Duplicates into Library/Review once equivalent workflow exists.

## UX tests/checks

- copy audit forbids overclaims;
- no enabled file-changing controls before safety gates;
- empty states explain the next action;
- keyboard and screen-reader labels stay intact;
- desktop layout remains sparse and squared, not bubbly dashboard sludge.

---

# Recommended immediate next implementation command

After this plan is accepted, start Phase 1 with TDD:

```bash
cargo test --manifest-path src-tauri/Cargo.toml apply_plan_operation_preview --lib
```

Expected first result: fail, because the module/tests do not exist yet.

Then implement Operation-set Preview V1 in small commits.

## Stop/go checkpoints

Stop and reassess if any of these happen:

- operation preview needs to mutate state;
- frontend has to pass raw executable path arrays;
- client-supplied previews look confirmable;
- validation can be bypassed;
- a test needs broad mocking to hide a safety gap;
- UI copy starts saying `safe`, `ready`, `approved`, or `confirmed` before the backend has earned it.

Go forward when:

- backend owns the operation set;
- every skipped item has a clear reason;
- every candidate still says future safety gates are required;
- no file-changing command is exposed;
- fresh focused and full verification lanes pass.
