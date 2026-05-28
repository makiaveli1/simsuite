# SimSuite Next Phases Roadmap

> **For Hermes:** Use `subagent-driven-development` to execute this plan task-by-task. Use `test-driven-development` for code changes and `verification-before-completion` before claiming any phase is complete.

**Date:** 2026-05-28

**Product name:** SimSuite. The legacy repo/path may still contain `SimSort`; user-facing copy, docs, reports, and roadmap language should say **SimSuite** unless referring to legacy code paths.

**Goal:** Move SimSuite from the current review/preview/validation/dry-run state toward a first safe beta without breaking the core trust promise: SimSuite helps users understand, review, and plan before it changes files.

**Architecture:** SimSuite is a local-first Tauri desktop app with a React/TypeScript frontend, Rust/Tauri backend, and SQLite persistence. The current ApplyPlan path is intentionally preview-only: generated previews can be saved as draft plans, validated, dry-run rehearsed, and reviewed with backend-owned plan identity/provenance, but real Apply/Restore remains blocked.

**Core product promise:**

> SimSuite scans and explains Sims content, helps users review risky or messy areas, previews safer organization plans, and only later applies a narrow, recoverable, user-confirmed move plan after backend safety gates prove it is safe enough.

---

## 0. Current state audit

### Evidence inspected

- `package.json`: package name is `simsuite`; scripts include `test:unit`, `test:rust`, desktop proof/smoke, Tauri dev/build.
- `docs/IMPLEMENTATION_STATUS.md`: current state is review, preview, validation, dry-run; real Apply/Restore blocked.
- `docs/ARCHITECTURE.md`: active safety pipeline stops at saved-plan review, validation preview, dry-run preview, and immutable plan hash/provenance check.
- `docs/TRUST_BOUNDARIES_AND_AUTOMATION_READINESS.md`: SimSuite must not claim broken mods, missing dependencies, safe delete, official/latest truth, or AI verification without deterministic proof.
- `docs/planning/APPLY_SAFETY_CONTRACT_V1.md`: future Apply requires exact preview, explicit confirmation, backup/restore, path validation, conflict handling, recoverable errors, and per-file result logs.
- `docs/planning/NAVIGATION_WORKFLOW_ARCHITECTURE_V1.md`: Home, Library, Inbox, Organize, Updates, Review, Settings are the durable user workflow surfaces.
- `docs/planning/BACKUP_RESTORE_RESULT_LOG_DESIGN_V1.md`: first visible file-changing workflow should use copy-backup-first plus restore map and backend-observed result logs.

### Current dirty working tree shape

The branch currently has extensive ApplyPlan/Organize safety work in progress, including:

- backend command gating;
- ApplyPlan hash/provenance;
- preview snapshots;
- validation and dry-run surfaces;
- result/restore metadata display;
- frontend Organize/Saved Plans updates;
- docs updates.

**Important:** This roadmap does not claim the current branch is complete. The current branch must still pass focused and full verification before it becomes the baseline for the next phase.

---

## 1. North-star user journey

This is the simple journey SimSuite should optimize around.

### Journey A — First use

1. User opens SimSuite.
2. User chooses Sims folders: Mods, Tray, Downloads/Inbox.
3. User runs a scan.
4. SimSuite builds local inventory and explains what it found.
5. User sees clear safety status: what SimSuite can do now, what it cannot do yet.

### Journey B — Understand my library

1. User opens Library.
2. User browses files/folders.
3. User inspects package/script metadata, hashes, previews, warnings, creator/type clues.
4. User can filter into Needs Review, Duplicates, Updates, Creator/Type lenses.
5. SimSuite explains evidence levels without overclaiming.

### Journey C — Review new or suspicious content

1. User checks Inbox for new downloads/imported files.
2. User reviews Duplicates for exact duplicate evidence and weak review-only cues.
3. User checks Updates for supported/reminder-only sources.
4. User uses Review for ambiguous files.
5. SimSuite routes review work back into Library/Organize when appropriate.

### Journey D — Plan organization

1. User opens Organize.
2. User chooses scope: selected files, folder, Mods/Tray subset, or future Inbox subset.
3. User chooses folder profile: SimSuite default or custom buckets.
4. User generates preview.
5. SimSuite shows source path -> proposed destination, reason, confidence/evidence level, blockers, and review-only status.
6. User saves the preview as a draft plan.

### Journey E — Validate and rehearse

1. User opens Saved Plans.
2. SimSuite shows hash/provenance identity and safe summary metadata.
3. User runs validation preview.
4. User runs dry-run preview.
5. SimSuite explains what would move later, what would skip, what blocks confirmation, and why Apply is still locked.

### Journey F — First future beta Apply

1. User opens a saved backend-generated plan.
2. SimSuite reruns validation and builds an exact operation set.
3. SimSuite shows exact eligible move list, skipped list, backup/restore readiness, and per-file risk summary.
4. User explicitly confirms.
5. Backend issues a short-lived confirmation token.
6. Backend backs up each eligible file before move.
7. Backend moves only eligible files.
8. Backend writes observed result logs.
9. User gets receipt and restore option for that run.

---

## 2. Hard safety boundaries

These are non-negotiable through all phases.

### Still blocked until explicitly released

- Real Organize Apply.
- Real Restore.
- User-file backup/restore execution.
- Delete.
- Quarantine.
- Replace.
- Cleanup.
- Automatic duplicate cleanup.
- Automatic update replacement.
- AI-only file decisions.
- Frontend-created result logs.
- Frontend-created restore entries.
- Legacy file-changing Tauri command paths.

### Permanent invariants

1. Plan hash is identity/provenance, not authorization.
2. Backend-generated preview snapshots are the only acceptable future confirmation source.
3. `client_supplied_preview` plans are review/audit-only forever.
4. Frontend may pass IDs and expected hashes, not executable path arrays.
5. Validation must be fresh before confirmation and execution.
6. Canonical path/root checks must run at validation, token issuance, and execution.
7. Backup/restore material must exist before mutation.
8. Result/restore logs must be backend-observed only.
9. Restore is scoped to one Apply run, not arbitrary filesystem rollback.
10. Legacy mutating commands stay fail-closed.

---

## 3. Phase -1 — Stabilize the current working tree

**Purpose:** Finish the current ApplyPlan Plan Hash / Provenance / Preview Snapshot work before starting new features.

### User-facing outcome

The current Organize/Saved Plans flow becomes a trustworthy preview-only baseline:

- Generate preview.
- Save as draft from backend-owned snapshot.
- Review saved plan.
- See hash metadata.
- Run validation preview.
- Run dry-run preview.
- See recovery metadata only.
- No file changes.

### Implementation focus

- Complete current review findings.
- Fix stale preview snapshot mismatch.
- Distinguish backend-generated vs client-supplied preview provenance.
- Keep full provenance server-side.
- Fail closed on missing/empty provenance.
- Fail closed on malformed persisted JSON.
- Make hash deterministic across identical logical plans.
- Include active rule snapshot/hash, not just rule count.
- Verify DB migration/index/triggers/startup repair.

### Required focused tests

- Same generated preview saved twice has same hash.
- Tampered provenance/context/destination fails validation.
- Missing/empty provenance fails validation.
- Malformed persisted JSON fails closed.
- Rule edit with same active-rule count changes hash/provenance.
- Migration creates columns, indexes, triggers.
- Saving after changing UI state uses preview-time snapshot, not current mutable state.
- Saved plan UI shows hash metadata but not full provenance.

### Required verification lane

```bash
cd "/mnt/c/Users/likwi/OneDrive/Desktop/PROJS/SimSort"

cargo fmt --all --manifest-path src-tauri/Cargo.toml
cargo test apply_plan --lib --manifest-path src-tauri/Cargo.toml
cargo test initialize_creates_apply_plan_hash_provenance_columns_and_guards --lib --manifest-path src-tauri/Cargo.toml
npx tsc --noEmit
npm run test:unit -- src/screens/OrganizeScreen.test.tsx src/trustBoundaryCopy.test.ts
```

Then full lane:

```bash
npm run test:unit
npm run build
npm run test:rust
cargo check --manifest-path src-tauri/Cargo.toml
cargo fmt --all --check --manifest-path src-tauri/Cargo.toml
cargo clippy --all-targets --all-features --manifest-path src-tauri/Cargo.toml
git diff --check
```

### Gate type

**Pre-flight gate:** no next phase starts until current branch has fresh verification evidence or explicit documented blockers.

### Exit criteria

- Current branch is verified as preview-only and non-mutating.
- Any unrelated failures are identified by exact test name and failure signature.
- No success claim relies on older test output.

---

## 4. Phase 0 — Product clarity and onboarding baseline

**Purpose:** Make SimSuite understandable as an app, not just a pile of screens.

### User-facing outcome

A user can open the app and understand:

- what to do first;
- what SimSuite knows;
- what it only suspects;
- what is blocked;
- how to get from scan -> review -> organize preview.

### UX deliverables

- Home safety/status panel:
  - Library configured/not configured.
  - Last scan status.
  - Review queues.
  - Preview-only status.
  - Locked future actions.
- Journey cards:
  1. Scan Library.
  2. Review Inbox.
  3. Check Duplicates.
  4. Review Updates.
  5. Create Organization Preview.
- Consistent evidence labels:
  - Exact match.
  - Evidence-backed cue.
  - Heuristic hint.
  - Manual review needed.
  - Not supported yet.
- Consistent blocked action language:
  - No files changed.
  - Apply not ready yet.
  - Restore not ready yet.
  - Preview only.

### Engineering deliverables

- Copy audit tests for banned overclaims:
  - safe to delete;
  - broken mod;
  - official/latest unless supported;
  - AI verified;
  - automatically fixed;
  - cleaned up.
- Update docs to define user journey in product language.
- Keep Settings/Guide/help language aligned with trust policy.

### Suggested implementation agents

- Ariadne/Studio: UX journey clarity and copy review.
- Argus/Sentinel: trust-boundary copy audit.
- Hephaestus/Forge: small UI/test implementation.

### Gate type

**Revision gate:** UX/copy review loops until no misleading user-facing safety claims remain.

### Exit criteria

- A new user can tell how to use SimSuite without reading implementation docs.
- No visible route implies real Apply/Restore exists.

---

## 5. Phase 1 — Review workflow consolidation

**Purpose:** Strengthen the workflows users use before organizing files.

### User-facing outcome

SimSuite becomes a useful daily tool even before Apply exists:

- Library is the source of truth.
- Inbox owns new content intake.
- Duplicates owns comparison, not cleanup.
- Updates owns source/watch review, not replacement.
- Review owns ambiguous/manual items.
- Creator/Type audits gradually become Library lenses.

### UX deliverables

- Home shows review queues by user task, not backend module.
- Library links into Duplicates/Updates/Review with clear context.
- Inbox clarifies “new files waiting here” and does not imply install/apply.
- Duplicates clarifies exact duplicate vs review-only similarity.
- Updates clarifies supported checker vs reminder-only vs unknown.
- Creator/Type audits get Library-lens migration notes or initial UI entry points.

### Engineering deliverables

- Route ownership cleanup.
- Shared evidence-label component or mapping.
- Shared empty/loading/error states for review queues.
- Regression tests for navigation handoff.

### Gate type

**Pre-flight gate:** before deleting or hiding any screen, prove equivalent user path exists.

### Exit criteria

- User can resolve or inspect review work before creating an organize plan.
- No review workflow exposes cleanup/replacement/quarantine.

---

## 6. Phase 2 — Organization planning UX v2

**Purpose:** Make Organize feel like a product workflow: choose scope, choose profile, preview tree, review, save.

### User-facing outcome

The user can answer:

- What files are included?
- What folder scheme is used?
- What would each file do later?
- Why was that destination suggested?
- Which items are blocked or manual-review?
- What changed between preview and saved draft?

### UX deliverables

- Organize stepper:
  1. Choose scope.
  2. Choose folder profile.
  3. Generate preview.
  4. Review proposed destinations.
  5. Save draft plan.
- Preview destination tree.
- Better item rows:
  - current path;
  - proposed destination;
  - bucket;
  - evidence strength;
  - reason;
  - blocker/review-only status.
- Saved plan summary:
  - total proposed;
  - blocked;
  - review-only;
  - eligible later;
  - hash metadata;
  - source kind.

### Backend deliverables

- Backend-issued preview snapshot remains the authoritative saved source.
- Snapshot id/hash required for preferred save path.
- Direct `save_apply_plan_preview` remains client-supplied and non-confirmable.
- Plan hash/provenance remains immutable.

### Tests

- Saving from old preview after UI config changes saves preview-time snapshot.
- Frontend cannot forge backend-generated source kind.
- Saved plan details include safe hash metadata but not full provenance JSON.
- Mock API mirrors backend semantics.

### Gate type

**Revision gate:** product review must confirm the plan is understandable before deeper safety engineering starts.

### Exit criteria

- Organize is usable as planning software even without Apply.
- The saved draft is clearly a frozen reviewed proposal, not a pending file action.

---

## 7. Phase 3 — Canonical Destination Validation V1

**Purpose:** Build the real backend path-safety layer before confirmation tokens or executors.

### User-facing outcome

Saved plan validation can explain concrete risks:

- source missing;
- source stale;
- destination outside safe root;
- destination exists;
- destination conflict;
- path traversal;
- missing root;
- backup/restore not ready;
- manual-review items excluded.

### Backend deliverables

- Canonical source/destination normalization.
- Root containment checks for Windows and WSL path semantics.
- Parent traversal rejection.
- Root prefix spoofing rejection.
- Case-insensitive conflict detection.
- Existing destination detection.
- Symlink/reparse-point behavior: block by default unless explicitly supported and tested.
- Cross-root movement: block by default for v1.
- Unsupported file types blocked.
- Review-only, weak heuristic, duplicate cleanup, update replacement, AI-only, delete/quarantine/replace items excluded.

### Possible DB deliverables

Introduce persisted validation proof only if needed for future token binding:

- `apply_plan_validation_runs`.
- `apply_plan_validation_items`.
- validation hash/version.
- root/config/context fingerprints.
- expiry.

If this is too much for one sprint, keep V1 response-only but design the persisted proof schema before token work.

### Tests

- parent traversal blocked;
- root escape blocked;
- root prefix spoofing blocked;
- missing source blocked;
- stale source blocked;
- destination exists blocked;
- duplicate destination blocked;
- case conflict blocked;
- symlink/reparse escape blocked or explicitly unsupported;
- cross-root blocked;
- malformed paths fail closed.

### Gate type

**Pre-flight gate:** confirmation token work cannot start until canonical validation is proven.

### Exit criteria

- Validation can safely say whether a saved plan is structurally eligible for future confirmation.
- Validation still does not create folders, backups, result logs, restore entries, or file changes.

---

## 8. Phase 4 — Confirmation readiness and operation-set preview

**Purpose:** Build the exact backend-derived operation list a user would confirm later, still read-only.

### User-facing outcome

Saved plan detail can show:

- eligible future moves;
- skipped items;
- blocked items;
- why each item is included or excluded;
- exact operation count;
- exact destination list;
- backup/restore readiness state;
- “Apply still locked” if token/executor not released.

### Backend deliverables

- Derive operation set from backend data only:
  - saved ApplyPlan;
  - backend preview snapshot;
  - plan hash/provenance;
  - latest validation proof;
  - folder config/context snapshots;
  - current root settings.
- No raw executable paths from frontend.
- Operation set hash.
- Expiry/invalidation when plan/root/config/validation changes.

### Possible command

```text
preview_apply_plan_confirmation_readiness
```

Read-only. No token. No file change.

### Gate type

**Revision gate:** safety reviewer verifies operation filtering before token phase.

### Exit criteria

- Operation set includes only move-only, backend-generated, validation-passing, non-review-only items.
- Everything else is explicitly skipped or blocked.

---

## 9. Phase 5 — Backend-issued confirmation token

**Purpose:** Create the first real authorization object without yet exposing execution.

### User-facing outcome

The UI can eventually show “ready for confirmation,” but only after backend token issuance conditions are satisfied.

### Backend deliverables

- `issue_apply_plan_confirmation_token`.
- Token is short-lived, single-use, hash-stored in DB.
- Token bound to:
  - apply plan id;
  - preview snapshot id/hash;
  - plan hash;
  - validation hash;
  - operation set hash;
  - token scope, e.g. `apply_plan_move_v1`.
- Token cannot be issued for:
  - client-supplied preview;
  - missing/mismatched plan hash;
  - stale validation;
  - unsafe destinations;
  - blocked/review-only/weak/AI-only/update/delete/quarantine/replace items;
  - missing backup/restore readiness.

### Tests

- no token for client-supplied preview;
- no token for stale validation;
- no token for changed plan hash;
- no token for changed operation set;
- token replay rejected;
- expired token rejected;
- cancelled plan invalidates token;
- root/settings change invalidates token.

### Gate type

**Pre-flight gate:** executor work cannot start until token semantics are tested.

### Exit criteria

- Confirmation exists as backend safety state, not a UI button flag.

---

## 10. Phase 6 — Fixture-only guarded executor

**Purpose:** Prove the executor in isolated temporary roots before touching user files.

### User-facing outcome

None yet. This is internal proof only.

### Backend deliverables

- Internal/test-only executor consumes:
  - apply plan id;
  - confirmation token;
  - expected hashes.
- Executor:
  1. loads token;
  2. checks token unexpired/unused;
  3. revalidates plan/snapshot/operation hashes;
  4. reruns execution-time path checks;
  5. creates verified backup copy;
  6. records restore map;
  7. moves one fixture file at a time;
  8. writes backend-observed result rows;
  9. handles partial failure;
  10. marks token used.

### Strict limits

- Temp fixtures only.
- No Tauri command exposure.
- No real Mods/Tray/Downloads.
- No delete/quarantine/replace/update.

### Tests

- no token means no changes;
- invalid token means no changes;
- backup failure prevents move;
- backup copy verified by size/hash;
- result row for every attempted/skipped item;
- restore entry created by backend only;
- blocked/review-only items skipped;
- partial failure recorded;
- interruption recoverable/reconcilable;
- replay with used token rejected.

### Gate type

**Abort gate:** any file access outside fixture roots stops the phase.

### Exit criteria

- End-to-end fixture Apply works and fails safely.
- No public UI/Tauri exposure exists.

---

## 11. Phase 7 — Hidden real-root executor behind feature gate

**Purpose:** Implement the real backend executor but keep it hidden and disabled by default.

### User-facing outcome

None for normal users. Internal closed testing only.

### Backend deliverables

- `apply_confirmed_apply_plan` command exists only behind strict command gate and feature/runtime flag.
- Input contains IDs and token, not raw paths.
- Move-only v1.
- Same-root reorganization by default.
- Destination folders only if previewed and validated.
- No overwrite.
- No delete/quarantine/replace/update/AI-only.
- Backup-first.
- Restore-map and result logs created by backend only.
- Partial failure handling.

### Gate type

**Escalation gate:** any policy choice around partial failure, cross-root moves, existing destinations, occupied restore paths, or symlink behavior must be explicitly decided before release.

### Exit criteria

- Executor is technically real but still not user-visible.
- Throwaway-profile proof can begin.

---

## 12. Phase 8 — Guarded Restore path

**Purpose:** Let users recover only files changed by a specific Apply run.

### User-facing outcome

A future beta user can preview and confirm restore for one Apply run.

### Backend deliverables

- `preview_apply_plan_restore`.
- `issue_apply_plan_restore_confirmation_token`.
- `restore_apply_plan_run`.
- Restore scoped to one Apply run and backend-created restore entries.
- No arbitrary restore paths.
- Backup path must be under app-owned backup root.
- Verify backup size/hash.
- Block if original path occupied unless safe policy exists.
- Block if destination changed unexpectedly.
- Backend-observed restore result rows.

### Tests

- restore refuses arbitrary path;
- restore refuses entries from another run;
- restore verifies backup;
- restore blocks occupied original path;
- restore blocks changed destination;
- double restore is idempotent or clearly blocked;
- missing backup blocks restore;
- restore logs per-file results.

### Gate type

**Pre-flight gate:** visible Apply beta cannot open without proven restore or a deliberate product decision that beta is fixture/throwaway-only.

### Exit criteria

- Restore is run-scoped recovery, not generic filesystem rollback.

---

## 13. Phase 9 — Closed beta on throwaway Sims profile

**Purpose:** Test real Apply/Restore only on disposable user-created test roots.

### User-facing outcome

A beta tester can use SimSuite on a throwaway Mods/Tray profile and apply one reviewed, eligible, move-only plan with backup/restore.

### Beta scope

Allowed:

- selected saved backend-generated plan;
- move-only;
- explicit confirmation;
- backup-first;
- restore-map;
- per-file receipt;
- restore for the same run.

Still blocked:

- delete;
- quarantine;
- replace;
- cleanup;
- auto-update;
- AI-only decisions;
- whole-library automation;
- safe-delete claims;
- broken-mod claims;
- missing mesh/dependency claims.

### QA deliverables

- throwaway profile setup guide;
- beta risk copy;
- manual QA script;
- screenshot evidence;
- logs/artifact paths;
- restore proof;
- known limitations.

### Gate type

**Pre-flight gate:** no real personal Mods/Tray folders until throwaway profile repeated clean passes.

### Exit criteria

- A human can recover every changed file from the beta run.
- No unresolved P0/P1 data-loss or command-boundary risk remains.

---

## 14. Phase 10 — Limited real-user beta

**Purpose:** Carefully expose the narrow move-only Apply/Restore path to opt-in users.

### User-facing outcome

The beta promise becomes:

> SimSuite can apply one reviewed, eligible, move-only organization plan with backup, restore, exact confirmation, and a per-file receipt.

### Release requirements

- versioned artifact;
- release notes;
- rollback instructions;
- diagnostic collection guide;
- privacy-aware logs;
- known limitations;
- explicit opt-in;
- support process;
- no unsafe automation promises.

### Gate type

**Revision gate:** release manager aggregates safety, UX, backend, migration, desktop proof. Any blocking issue loops back to the owning phase.

### Exit criteria

- Real-user beta can ship with honest limitations.
- Broader automation remains blocked.

---

## 15. Later phases — broader automation only after trust is earned

Do not plan these until the narrow Apply/Restore loop is boringly reliable.

Possible later work:

- selective duplicate cleanup with deterministic exact duplicates only;
- provider-safe update replacement;
- richer Library lenses;
- AI-assisted explanations/summaries;
- backup retention management;
- batch operations;
- dependency/mesh analysis if deterministic evidence model exists.

Still forbidden without new contracts:

- broad auto-delete;
- quarantine by guess;
- AI-decided file mutation;
- unverified official-source scraping;
- “broken mod” claims without deterministic proof.

---

## 16. Required validation lanes by phase

### Baseline full lane

```bash
cd "/mnt/c/Users/likwi/OneDrive/Desktop/PROJS/SimSort"

npx tsc --noEmit
npm run test:unit
npm run build
npm run test:rust
cargo check --manifest-path src-tauri/Cargo.toml
cargo fmt --all --check --manifest-path src-tauri/Cargo.toml
cargo clippy --all-targets --all-features --manifest-path src-tauri/Cargo.toml
git diff --check
```

### Desktop proof

```bash
npm run desktop:proof:fixtures -- --SkipBuild
npm run desktop:smoke:fixtures
```

### Release build proof

```bash
npm run tauri:build
```

### ApplyPlan focused lane

```bash
npm run test:unit -- src/screens/OrganizeScreen.test.tsx src/trustBoundaryCopy.test.ts src/lib/api.test.ts
npm run test:rust -- apply_plan
npm run test:rust -- command
npm run test:rust -- database
```

### Path validation focused lane

```bash
npm run test:rust -- apply_plan_validation
cargo check --manifest-path src-tauri/Cargo.toml
cargo clippy --all-targets --all-features --manifest-path src-tauri/Cargo.toml
```

### Backup/restore fixture lane

```bash
npm run test:rust -- backup
npm run test:rust -- restore
npm run test:rust -- apply_plan_results
```

---

## 17. Specialist-agent operating model

Use agents by lane, not one mega-agent.

### Product / Journey agent

Owns:

- user stories;
- screen flow;
- onboarding;
- copy clarity;
- “does the user know how to use this?”

### Forge / Backend agent

Owns:

- Rust modules;
- Tauri command surface;
- DB migrations;
- validation/token/executor architecture;
- tests.

### Sentinel / Security QA agent

Owns:

- command bypasses;
- unsafe path cases;
- frontend-forged writes;
- provenance/hash misuse;
- result/restore ownership.

### Studio / UX QA agent

Owns:

- screenshots;
- hierarchy;
- disabled action language;
- evidence labels;
- whether UI implies file mutation.

### Desktop Proof agent

Owns:

- Windows/Tauri proof;
- fixture scripts;
- screenshots;
- release build sanity.

### Release Manager agent

Owns:

- final gate matrix;
- validation evidence;
- known failures;
- ship/hold decision.

---

## 18. Standard report template for every future phase

Every phase report should include:

```markdown
### What this means for the user

### Modules touched

### Tauri command surface

### DB schema / migrations

### Implemented workflows

### Partial / stubbed areas

### Trust / safety boundary

### Existing systems reused

### New data or logic added

### Tests and verification evidence

### Code-quality / security / spec issues fixed

### Remaining risks

### Next gate
```

No phase is complete until fresh verification evidence is attached.

---

## 19. Immediate next action

Do **not** start a new feature phase yet.

First finish **Phase -1: Stabilize current working tree**:

1. Inspect current diffs.
2. Fix the known review findings.
3. Run focused ApplyPlan validation.
4. Run full frontend/Rust/build/lint lanes.
5. Produce a concise evidence report.
6. Only then choose whether Phase 0, Phase 2 UX, or Phase 3 canonical validation is the next implementation sprint.

Recommended next sprint after current branch is verified:

> **Canonical Destination Validation V1** should come before confirmation tokens or executor work. UX clarity can run in parallel, but no backend execution path should advance until canonical validation is proven.
