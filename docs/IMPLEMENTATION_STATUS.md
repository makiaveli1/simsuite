# SimSuite implementation status

Last updated: 2026-08-07

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
- Fixture-only backend proof work for future transaction safety: verified copy-backup and restore prototypes plus a Rust-test-only one-file transaction coordinator that loads persisted ApplyPlan/run/item scope, requires an isolated fixture root and indexed source hash/size match, verifies backup before reusing the existing move primitive, verifies the moved bytes, records only non-execution result/restore metadata, and proves scoped undo restores the exact previous temporary-file state before removing the matching moved fixture copy. It is not compiled into normal builds and does not enable real Apply or Restore.
- Windows desktop proof/smoke scripts.
- Native Windows game-profile candidate proof wrapper that runs the real read-only detector and candidate test module, emits ignored local JSON evidence, and includes a temporary NTFS junction/reparse-point test. GitHub Actions run `30989399771` passed on Windows, macOS, and Ubuntu for exact commit `4412f76e2a86694b13145498310bfb56a9d3ab66`. Its Windows receipt artifact was downloaded and passed the standalone reviewer for schema, UTC timestamp, exact Git revision, Windows host and known-folder evidence, detector environment, and read-only candidate guarantees. This is hosted-runner baseline evidence only and explicitly does not unlock representative player-machine proof, candidate refresh, or file mutation.
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
- Game Installation Profile V1 and Sims4Adapter foundation: versioned profile/root tables, stable root IDs, idempotent migration of previously saved Mods/Tray/Downloads paths, one explicit active-profile setting, fail-closed enum and JSON parsing, schema self-repair, and a compatibility `LibrarySettings` view for existing callers. Read-only Tauri and TypeScript APIs list complete profiles, return the selected active profile, and produce transient validation reports off the Tauri command thread. The generic layer checks absolute paths, metadata state, canonical directory identity, root-local case sensitivity, and symlink observations. The Sims4Adapter consumes that canonical evidence without probing the filesystem again, owns Mods/Tray content extensions and placement-depth constants, and evaluates explicit or inferred user-data coherence using stable root IDs rather than folder names. Separate coherent Mods and Tray roots can now reach `valid`; overlapping roles, split user-data trees, missing required roots, or roots outside an explicit user-data root fail closed. Foreign-native and Wine/Proton/Lutris paths remain unguessed, no result is persisted, and the old settings command can update only the reserved legacy-migration profile.
- Guarded Settings game-profile setup, selection, candidate review, and evidence: the Game setup section lazy-loads only when opened, lists saved profiles, requires an explicit choice and activation click, and shows the active profile, host compatibility, configured and canonical roots, generic filesystem evidence, Sims4Adapter rules and evidence strength, blockers, review notes, and truthful degraded states. Players can add a current-host Windows or macOS Sims 4 setup manually by naming it and choosing existing Mods and Tray folders plus optional user-data and Downloads roots. On macOS, SimSuite checks the resolved Documents directory and a home-Documents fallback. On Windows, it checks the operating-system-resolved configured Documents directory, explicit absolute OneDrive roots exposed through `OneDrive`, `OneDriveConsumer`, or `OneDriveCommercial`, and a home-Documents fallback. Both native paths look only for existing `Electronic Arts/The Sims 4` layouts, deduplicate equivalent roots, and rank transient candidates as strong or possible according to whether existing Mods and Tray directories were found. Windows candidate identity is case-insensitive, and relative OneDrive roots are ignored. Candidate results are read-only DTOs and are never stored. Choosing `Review this suggestion` only copies the proposed paths into the unsaved manual form. The player must still explicitly save an unconfirmed draft, review fresh validation evidence, confirm it, and activate it as separate actions. Linux remains manual-only; drafts are stored with an unknown environment and remain unconfirmable until Wine/Proton/Lutris environment support can identify them safely. Confirmation reruns a fresh read-only current-host validation off the Tauri command thread and succeeds only for a fully valid, blocker-free manual profile whose stored roots still match the report. Detailed filesystem evidence remains transient; only the confirmation state, profile status, and confirmation/validation timestamps are persisted. Only player-confirmed profiles whose operating environment matches the current host can become active. Foreign-native, Wine/Proton/Lutris, unknown-environment, and unconfirmed profiles remain reviewable but cannot be activated. A successful switch updates the active preference, restarts Downloads monitoring, and refreshes folder-dependent workspaces without running Apply or Restore or moving installed Mods or Tray files. Confirmed-profile editing, profile deletion, automatic saving, and silent activation remain unavailable.
- Product direction execution plan that defines SimSuite as a local-first lifecycle companion across intake, understanding, planning, installation, verification, Library, maintenance, troubleshooting, and recovery. The next golden milestone remains fixture-only and does not enable real user-file execution. Indexed File Identity V1 gives newly scanned, active-profile-matched Mods and Tray rows nullable scan-owned profile ID, root ID, and slash-normalized root-relative path metadata. Duplicate Same-File Identity V1 also stores a versioned root-policy-aware relative-path comparison key when case sensitivity can be proven. Exact SHA-256 and package/script fingerprint duplicate proof, Duplicates pair classification, and Library exact-duplicate counts use one shared fail-closed rule: complete profile/root/key tuples decide same-file identity; legacy rows may prove separation only when their paths differ by more than case or slash style; mixed, partial, case-only, or same-key identity remains ambiguous. Portable Destination Identity V1 now gives Organize and guided-install preview planning one read-only destination-reservation context per batch. It probes configured Mods and Tray roots once, reuses the existing root-local case policy, keeps proven case-distinct paths separate on sensitive roots, treats case-only aliases as collisions on insensitive or unknown roots, and keeps distinct physical Mods and Tray roots separate while detecting shared-root collisions. Reservations use hash-based sensitive and folded keys for bounded low-spec lookup cost, while existing physical occupancy, unreadable-metadata, and current-file ownership checks remain unchanged. Indexed Folder Identity V1 adds nullable scan-owned profile ID, root ID, root-relative path, and case-policy-aware comparison key metadata to newly scanned `library_folders` rows. It reuses the scan root's existing evidence without another filesystem probe and leaves legacy folder rows unassigned until rescan. Library Folder Content Identity V1 uses that identity for read-only selected-folder file queries: current-profile rows scope by profile, root, and component-safe comparison key; unknown-key rows use the exact observed relative path; stale or incomplete identity fails closed; and genuinely legacy rows retain the configured-root compatibility query. Library Folder Tree Profile Identity V1 now makes each requested Mods or Tray tree choose a current-profile, legacy, or blocked mode. Current-profile trees use scan-owned root-relative folder identity, count only files from the same profile and root, and derive nesting from each file's root-relative identity rather than its absolute path. Stale-profile, incomplete, or unsafe identity is omitted instead of merged, while genuinely legacy all-null folders and files retain the previous path/depth behaviour. The public tree shape and stored full-path evidence remain unchanged. Library Folder Identity Uniqueness V2 replaces the old global lowercase constraint with three guarded uniqueness lanes. Proven case-sensitive comparison keys can preserve case-only siblings such as `Creator` and `creator`; folded insensitive keys still reject aliases; and unknown-policy or genuinely legacy rows remain conservatively unique by their lowercase compatibility path. Existing folder IDs, rows, timestamps, indexes, and identity triggers survive the transactional migration, schema self-repair does not depend only on the migration ledger, and scanner folder writes are strict inserts after their source snapshot is cleared. Indexed File Parent Identity and Library Folder Tree SQL Counts V1 adds nullable scan-owned parent root-relative path and optional case-policy-aware parent comparison key metadata to newly scanned Mods and Tray file rows. Root-direct files use an empty parent path with no parent key. Folder-tree queries now let SQLite return grouped direct counts for rows with parent identity, while Rust preserves the existing nodes and recursive totals. Legacy and pre-v13 rows whose parent identity is null retain the exact metadata-row compatibility path until a valid rescan. Existing indexed file rows gain comparison keys and parent identity only after a valid rescan, Downloads-only rows remain unassigned, and no candidate refresh, Apply, Restore, cleanup, confirmation-token, or user-file mutation capability is enabled.
- Phase 0 Home clarity baseline: first-run journey cards (`Scan Library` -> `Review Inbox` -> `Check Duplicates` -> `Review Updates` -> `Create Organization Preview`), a Home safety/status panel, consistent evidence labels, and explicit locked-action copy (`No files changed`, `Preview only`, `Apply not ready yet`, `Restore not ready yet`).

## Cross-platform verification remaining

- Preserve the successful hosted Windows, macOS, and Linux preflight receipt as baseline evidence while keeping it separate from representative player-machine and native desktop proof.
- Run the existing Windows fixture desktop proof and smoke lanes on a real Windows host. The CI Windows runner verifies dependency installation, tests, builds, native compilation, and the read-only game-profile detector baseline, but it does not exercise the webdriver player journey or a player's real Sims folders.
- Run `pnpm run desktop:proof:game-profile:windows` on representative Windows hosts and retain the generated JSON receipt for normal Documents, redirected Documents, personal and commercial OneDrive, duplicate aliases, inaccessible folders, case variants, and reparse points. A hosted-runner receipt is only baseline evidence and does not satisfy these player-environment scenarios.
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
- `docs/planning/SIMSUITE_APP_DIRECTION_EXECUTION_PLAN_V1.md`
- `docs/planning/GAME_INSTALLATION_PROFILE_V1_DESIGN.md`
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

For native Windows game-profile candidate evidence:

```bash
pnpm run desktop:proof:game-profile:windows
```

To review a downloaded or local Windows game-profile receipt without changing it:

```bash
pnpm run desktop:review:game-profile:windows -- --summary <path-to-latest-summary.json> --expected-git <full-40-character-sha>
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

Advance **Game Installation Profile V1** from the verified macOS and code-level Windows candidate proofs through the prepared hosted and representative Windows evidence lanes, while keeping silent activation, confirmed-profile editing/deletion, Linux environment guessing, and executor work behind later safety gates. The hosted runner should establish a repeatable Windows/NTFS baseline after an approved push, but only representative player-machine receipts can prove real known-folder, OneDrive, case, permission, and redirection behaviour.

The profile foundation preserves existing users' effective Mods, Tray, and Downloads paths through an idempotent compatibility migration. Complete profiles, guarded manual draft creation, fresh validation-backed confirmation, guarded active-profile selection, transient generic root evidence, Sims4Adapter readiness reports, and ranked read-only macOS and Windows candidates are available through backend and TypeScript APIs. Settings presents candidates as temporary suggestions and permits only confirmed, current-host-compatible saved profiles to become active; foreign-native and adapter-dependent environments remain review-only. Candidate review and manual setup never create folders or activate a profile silently. The adapter proves coherent Mods/Tray relationships for matching native profiles and shares its content-extension and placement-depth contract with scanner and validation code.

Next work:
- inspect the published Windows preflight result, archive the hosted artifact, and run `pnpm run desktop:review:game-profile:windows` against the exact workflow Git revision without treating a hosted pass as player-environment proof;
- run `pnpm run desktop:proof:game-profile:windows` on representative Windows hosts and review each generated receipt for normal Documents, redirected Documents, personal and commercial OneDrive, duplicate aliases, inaccessible folders, case variants, and reparse points;
- add a safe explicit candidate refresh action only after the hosted baseline and representative native Windows proof pass;
- keep old `LibrarySettings` callers on the compatibility view while migrating one subsystem at a time;
- design Wine/Proton/Lutris environment identification separately without interpreting native Linux paths as a Sims 4 setup;
- run path-semantics and profile-validation proof on native Windows and Linux, separately from hosted CI evidence;
- integrate profile/root-relative identity into duplicate identity, scanner folder keys, Library queries, Downloads intake, and hidden move preflight in small verified batches;
- preserve Backend Command Gating V1: legacy file-changing commands and client-forged ApplyPlan run/result/restore writes must keep failing closed;
- keep Apply, Restore, backup execution, result logs, restore logs, delete, quarantine, replace, cleanup, and automatic mutation unavailable.

The product hierarchy and golden fixture journey are defined in `docs/planning/SIMSUITE_APP_DIRECTION_EXECUTION_PLAN_V1.md`. The profile migration and API contract are defined in `docs/planning/GAME_INSTALLATION_PROFILE_V1_DESIGN.md`. The first hidden transaction proof now exists only inside Rust tests for one temporary file. The next Phase D work should remain fixture-only and add interruption/failure injection, destination-race hardening, only-affected index refresh, multi-file rollback behavior, and the golden `.package`, `.ts4script`, archive, Tray, and curated special-mod fixtures before any real Apply or Restore surface is considered.

## Recent backend safety gate

Backend Command Gating V1 is now implemented at the Tauri command boundary. Current safe review commands remain callable, but externally callable legacy file-changing commands and client-forged ApplyPlan run/result/restore writes fail closed before doing work. This gate does not enable Apply or Restore; it only makes the previous UI-only boundary harder to bypass.

## Recent backend provenance gate

Plan Hash / Provenance V1 is now implemented for newly saved ApplyPlans. The backend computes a SHA-256 hash over a canonical preview identity that includes source scope, preview-time folder configuration, context trail, saved item/source/destination data, item signals/blockers, source file snapshots, and active rule/seed/settings version evidence. Provenance distinguishes `backend_generated_sorting_preview` from `client_supplied_preview`; future confirmation must accept only backend-generated provenance. Full provenance stays server-side; normal frontend responses expose only hash metadata. The hash/provenance columns are immutable once set. Validation blocks future confirmation if the stored hash/provenance is missing or no longer matches the persisted preview snapshot. This does not enable Apply, Restore, backup execution, confirmation tokens, result logs, restore logs, folder creation, or file mutation.
