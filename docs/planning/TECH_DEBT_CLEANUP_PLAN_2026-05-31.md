# SimSuite Technical Debt Cleanup Plan

Date: 2026-05-31 20:42 IST
Repo: `/mnt/c/Users/likwi/OneDrive/Desktop/PROJS/SimSort`
Baseline branch: `main`
Baseline commit: `8dbd196` — `harden ApplyPlan preview snapshot identity`

## Purpose

Clean up warning and maintainability debt without weakening the current preview-only safety boundary.

This plan is deliberately conservative. SimSuite is now carrying enough ApplyPlan safety logic that random cleanup is risky. The right order is:

1. remove mechanical warning noise;
2. mark intentional future-facing schema/API fields explicitly;
3. tighten lint gates only after the warning budget reaches zero;
4. then refactor module boundaries.

## Non-negotiable boundaries

This cleanup must not enable or expose:

- real Apply;
- Restore;
- backup execution against user files;
- file movement/copy/delete/quarantine/replacement;
- result-log writes from the frontend;
- restore-entry writes from the frontend;
- confirmation-token issuance;
- legacy mutating Tauri commands.

Any cleanup that touches command registration, command gating, ApplyPlan validation, path validation, backup/restore prototype code, or move-engine code must prove that the boundary stays closed.

## Current measured warning baseline

Fresh measurements from this working tree:

```bash
cargo check --manifest-path src-tauri/Cargo.toml --message-format=json
# exit 0; 15 rustc warnings, all dead_code

cargo clippy --all-targets --all-features --manifest-path src-tauri/Cargo.toml
# exit 0; simsuite lib generated 80 warnings; lib test generated 82 warnings, 74 duplicates

cargo test --manifest-path src-tauri/Cargo.toml apply_plan --lib
# exit 0; 104 passed; 30 warning lines emitted by lib-test compile
```

### Warning concentration

`cargo check` warnings:

- `src/models.rs`: 6
- `src/core/special_mod_versions/mod.rs`: 3
- `src/seed/mod.rs`: 3
- `src/core/install_profile_engine/mod.rs`: 2
- `src/core/scanner/mod.rs`: 1

`cargo test apply_plan --lib` warning lines:

- `src/core/file_inspector/mod.rs`: 13
- `src/models.rs`: 6
- `src/core/downloads_watcher/mod.rs`: 3
- `src/core/special_mod_versions/mod.rs`: 3
- `src/seed/mod.rs`: 3
- `src/commands/mod.rs`: 1
- `src/core/apply_plan_persistence.rs`: 1

`cargo clippy --all-targets --all-features` warning headers, counting lib plus lib-test additions:

- `src/core/file_inspector/mod.rs`: 21
- `src/models.rs`: 14
- `src/core/downloads_watcher/mod.rs`: 10
- `src/core/install_profile_engine/mod.rs`: 7
- `src/core/rule_engine/sorting_plan.rs`: 6
- `src/core/library_index/mod.rs`: 5
- `src/core/special_mod_versions/mod.rs`: 4
- `src/seed/mod.rs`: 4
- `src/commands/mod.rs`: 3
- `src/core/scanner/mod.rs`: 3
- `src/core/validator/mod.rs`: 3
- remaining one-off files: `command_gate.rs`, `apply_plan_persistence.rs`, `content_versions`, `move_engine`, `lib.rs`, `apply_plan_backup_prototype`, `filename_parser`

Main Clippy categories:

- behavior-free mechanical lints: derivable `Default`, needless borrow, redundant closure, `repeat_n`, `clamp`, `?`, `is_empty`, needless lifetimes, unnecessary casts, string replace collapse;
- API-shape lints: too-many-arguments helpers;
- dead-code/unused fields that may represent future contracts and need explicit decisions, not blind deletion.

## Phase A — mechanical Clippy cleanup

Goal: remove behavior-free Clippy warnings first.

Likely edits:

- Replace manual `Default` impls with `#[derive(Default)]` where Clippy identified derivable implementations.
- Remove the needless borrow in `src/lib.rs` without changing command-gate behavior.

Files likely touched:

- `src-tauri/src/models.rs`
- `src-tauri/src/seed/mod.rs`
- `src-tauri/src/lib.rs`

Verification gate:

```bash
cargo fmt --all --manifest-path src-tauri/Cargo.toml
cargo clippy --all-targets --all-features --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml apply_plan --lib
npm run test:rust -- apply_plan --lib
git diff --check
```

Commit shape:

```text
chore: remove mechanical Rust clippy warnings
```

## Phase A landing result

Phase A removed the targeted derivable-default and `src/lib.rs` needless-borrow Clippy findings without changing serialized field names, command registration, ApplyPlan validation, command gating, or file-operation boundaries.

Fresh post-Phase-A Clippy summary:

```bash
cargo clippy --all-targets --all-features --manifest-path src-tauri/Cargo.toml
# exit 0; simsuite lib generated 70 warnings; lib test generated 72 warnings, 64 duplicates
```

The remaining warning debt is Phase B/C/D territory, not part of this mechanical landing slice.

## Phase B — test-build warning cleanup

Goal: remove unused imports, unused locals, and stale parser scratch values that appear only in lib-test builds.

Likely edits:

- `src/core/file_inspector/mod.rs`
  - remove unused imports such as `time::Duration` and `BufReader` if no longer needed;
  - remove or underscore unused locals such as `group_id`, `pos_before`, `start_pos`, and `first_entry_values`;
  - remove dead parser scaffolding only if tests prove no behavior changed.
- `src/core/downloads_watcher/mod.rs`
  - remove unnecessary `mut`;
  - rename truly intentional ignored values with `_` prefix.
- `src/core/apply_plan_persistence.rs`
  - either remove unread fields from `PreviewSnapshotRow` or keep them with explicit intent if they are needed for near-term expiry/audit work.
- `src/commands/mod.rs`
  - decide whether `emit_downloads_progress` is dead or future event plumbing. Delete only if no command/event path needs it.

Verification gate:

```bash
cargo fmt --all --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml apply_plan --lib
cargo test --manifest-path src-tauri/Cargo.toml file_inspector --lib
cargo test --manifest-path src-tauri/Cargo.toml downloads_watcher --lib
npm run test:rust
cargo check --manifest-path src-tauri/Cargo.toml
git diff --check
```

Commit shape:

```text
chore: clean Rust test-build warning noise
```

## Phase B landing result

Phase B removed low-risk test-build warning noise in `file_inspector` and `downloads_watcher`: unused imports, stale scratch locals, an unused local parser constant, a no-op Windows timeout scaffold, an intentionally ignored ZIP count binding, and an unnecessary mutable closure binding.

Fresh post-Phase-B Clippy summary:

```bash
cargo clippy --all-targets --all-features --manifest-path src-tauri/Cargo.toml
# exit 0; simsuite lib generated 60 warnings; lib test generated 62 warnings, 54 duplicates
```

The remaining warning debt is mostly future-contract/dead-code decisions and broader mechanical lints that belong to Phase C/D, not this narrow test-build cleanup slice.

## Phase C — intentional future-contract fields

Goal: distinguish real dead code from fields intentionally kept for future Apply/Restore/API contracts.

Do not blindly delete future-facing fields just to make Rust quiet. Some warning sources are schema/API contract placeholders and should either be used, documented, or explicitly allowed.

Review carefully:

- `PersistedApplyPlanRun.confirmation_token`
- `LibraryFileRow.installed_version`
- `McccUpdateInfo`
- `ApplyMcccUpdateResult`
- seed structs with fields used for external seed shape rather than internal reads
- `ProblemSignalSeverity::Severe`
- `ProblemSignalProofLevel::{Inferred, Unknown}`

Preferred outcomes, in order:

1. if field is truly obsolete: delete it and update tests/API types;
2. if field is part of a serialized or future DB/API contract: add a narrow `#[allow(dead_code)]` with a comment explaining the contract;
3. if field should be displayed or consumed soon: add a test-backed usage path instead of an allow.

Verification gate:

```bash
cargo fmt --all --manifest-path src-tauri/Cargo.toml
cargo check --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml apply_plan --lib
npm run test:unit
npm run test:rust
git diff --check
```

Commit shape:

```text
chore: document intentional future Rust contracts
```

## Phase D — warning budget gate

Goal: make new warnings harder to introduce after the backlog is cleaned.

Only do this when `cargo check`, `cargo test apply_plan --lib`, and `cargo clippy` are clean or have an intentionally documented tiny budget.

Options:

- add a repo script that counts warnings and fails above the current budget;
- later upgrade to `-D warnings` for selected modules;
- keep full `-D warnings` for the whole crate as a final step, not the first step.

Verification gate:

```bash
cargo check --manifest-path src-tauri/Cargo.toml
cargo clippy --all-targets --all-features --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml apply_plan --lib
npm run test:rust
```

Commit shape:

```text
chore: add Rust warning budget gate
```

## Phase E — ApplyPlan module boundary cleanup

Only start this after warning cleanup. This is where risk increases.

Likely extraction targets:

- persistence row mapping / strict JSON parsing helpers;
- preview snapshot repository functions;
- ApplyPlan item insert/load helpers;
- hash/provenance helpers that are currently stable enough to isolate;
- command wrappers vs core functions.

Rules:

- one extraction per commit;
- no behavior change without a failing test first;
- keep command-gate tests green after every extraction;
- do not rename externally serialized fields unless frontend/API tests prove the contract.

Verification gate:

```bash
cargo fmt --all --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml apply_plan --lib
cargo test --manifest-path src-tauri/Cargo.toml command_gate --lib
npm run test:unit -- src/screens/OrganizeScreen.test.tsx src/lib/api.test.ts src/trustBoundaryCopy.test.ts
npm run test:rust
cargo clippy --all-targets --all-features --manifest-path src-tauri/Cargo.toml
git diff --check
```

Commit shape:

```text
refactor: split ApplyPlan persistence boundaries
```

## Recommended immediate next step

Start with Phase A. It is low-risk, verifies quickly, and removes Clippy noise before touching file-inspector or future-contract fields.

Do not jump straight to Operation-set Preview V1 until the local warning noise is reduced enough that safety-review output is not buried under unrelated warnings. Tiny cleanup before sharper knives. Boring, but boring is how file tools avoid becoming horror stories.
