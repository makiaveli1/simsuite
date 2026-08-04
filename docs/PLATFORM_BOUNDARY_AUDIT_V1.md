# SimSuite Platform Boundary Audit V1

**Date:** 4 August 2026  
**Branch:** `platform/cross-platform-foundation`  
**Audited commit:** `9354056873e5847df508a27b7182eec6e66837b1`  
**Scope:** Windows, macOS, and Linux path semantics, installation discovery, platform responsibilities, and verification boundaries.

## 1. Executive conclusion

SimSuite now has a credible portable development foundation, native file-manager reveal dispatch, a three-runner CI preflight, and repaired Windows proof helpers. The next architecture gate is a shared filesystem-aware path model.

Current path-sensitive systems still use several independent normalization rules. Most replace separators and lowercase strings unconditionally. That is intentionally conservative for Windows, but it is not a correct universal identity policy for case-sensitive macOS or Linux volumes. Current destination containment is lexical rather than filesystem-aware and does not prove that an existing symlink or alias remains inside the configured root.

The product must not advance to a fixture executor until these rules are centralized behind explicit platform and filesystem capabilities.

Real Apply, Restore, delete, quarantine, replace, cleanup, and automatic mutation remain blocked.

## 2. Evidence reviewed

The audit inspected the current implementations and call paths in:

- `src-tauri/src/models.rs`
- `src-tauri/src/commands/mod.rs`
- `src-tauri/src/platform/mod.rs`
- `src-tauri/src/core/apply_plan_validation.rs`
- `src-tauri/src/core/duplicate_detector/mod.rs`
- `src-tauri/src/core/scanner/mod.rs`
- `src-tauri/src/core/library_index/mod.rs`
- `src-tauri/src/core/move_engine/mod.rs`
- `src-tauri/src/core/validator/mod.rs`
- `src-tauri/src/core/rule_engine/mod.rs`
- `src-tauri/src/core/rule_engine/sorting_plan.rs`
- `src-tauri/src/core/downloads_watcher/mod.rs`
- Windows desktop and build helpers under `scripts/`

The registered Verify workflow passed on the audited commit:

- `cargo check`
- frontend and wrapper tests
- Rust tests

Hosted CI publication was triggered by pushing the audited commit, but hosted run details require authenticated GitHub inspection and are not claimed in this report.

## 3. Findings

### P1. Universal case folding is being used as filesystem identity

Several systems construct path identity by replacing separators and applying `to_ascii_lowercase()`:

- ApplyPlan source, destination, duplicate-destination, and root-containment checks in `core/apply_plan_validation.rs`
- exact duplicate same-path exclusion in `core/duplicate_detector/mod.rs`, including SQL expressions using `LOWER(REPLACE(...))`
- scanner folder keys in `core/scanner/mod.rs`
- folder scoping queries in `core/library_index/mod.rs`
- replacement matching and hidden move preflight in `core/move_engine/mod.rs`

This can collapse two distinct files on a case-sensitive filesystem. For example, `Creator/Mod.package` and `creator/mod.package` may be separate files on Linux or a case-sensitive macOS volume, but current comparison keys treat them as the same path.

It can also hide a case-only source-path change during stale-plan validation, exclude a genuine exact duplicate pair from duplicate results, merge folder identities, or match the wrong replacement target.

**Required correction:** one path comparison service must derive its comparison key from the confirmed root's filesystem capabilities. Windows roots are normally case-insensitive. Linux roots are normally case-sensitive. macOS must be detected or recorded per root rather than assumed globally.

### P1. Destination containment is lexical, not canonical

`core/apply_plan_validation.rs` currently rejects explicit `..` components and checks whether a normalized destination string equals or starts with the normalized configured root.

This blocks simple traversal but does not prove physical containment. An existing symlink inside the configured root may resolve outside it. A future executor could therefore receive a path that looks lexically safe while resolving beyond the approved fixture or user-data root.

The current validator is read-only and cannot execute the move, so this is not an active user-file mutation vulnerability. It is a release-blocking architecture gap before any fixture or real executor is registered.

**Required correction:** distinguish lexical normalization from physical resolution. Resolve the deepest existing ancestor, inspect symlinks or aliases where supported, reattach missing descendants safely, and prove the resulting destination remains within the canonical root identity. Unreadable metadata must fail closed.

### P1. SimSuite has global path settings, not installation profiles

`LibrarySettings` stores only optional absolute strings for Mods, Tray, Downloads, and the reject folder. `detect_default_library_paths()` checks one conventional `Documents/Electronic Arts/The Sims 4` location plus one Downloads location and returns only existing paths.

The model cannot currently represent:

- multiple Sims 4 user-data folders
- OneDrive or other redirected Documents locations as ranked alternatives
- test profiles
- external or removable drives
- custom Mods and Tray roots
- Steam Proton, Wine, or Lutris environments
- detection evidence or confidence
- explicit player confirmation
- root filesystem capabilities
- stable root identity when drive letters or mount points change

**Required correction:** add Installation Profile V1 with profile identity, game identity, operating environment, root identities, Mods, Tray, Downloads intake roots, capability evidence, detection evidence, and confirmation state. Ambiguous candidates must never be auto-selected.

### P2. Persistent path storage is lossy and tied to absolute display strings

Several command paths are converted with `to_string_lossy()` before being returned or persisted. Database records and settings use UTF-8 strings as the primary file identity.

This is adequate for most Windows and macOS player paths, but it cannot faithfully preserve every valid Unix path. It also makes absolute display paths the durable identity, which is fragile across drive-letter changes, cloud-folder relocation, profile migration, and remounting.

**Required correction:** use a portable identity made from `profile_id`, `root_id`, normalized relative components, and an optional native filesystem identity. Keep display paths as presentation data. Where the UI or database cannot represent a native path losslessly, report the limitation and fail closed rather than silently changing identity.

### P2. Root membership is inferred from labels and path text

The shared core frequently uses string labels such as `mods`, `tray`, and `downloads`, and some move logic additionally infers Tray membership by searching path components or path text for `Tray`.

This can misclassify a custom root whose parent folders happen to contain that word, and it does not survive renamed roots or future game adapters cleanly.

**Required correction:** use explicit profile root IDs and game-adapter placement rules. Path text may be display evidence, not root authority.

### P2. The current platform adapter covers reveal only

`src-tauri/src/platform/mod.rs` provides tested Finder, Explorer, and Linux file-manager dispatch. This is a useful first slice, but the architecture plan requires additional platform-owned capabilities:

- Documents and Downloads candidate discovery
- filesystem capabilities
- available space
- recoverable trash
- watcher behavior
- permission guidance
- path comparison policy
- file-lock behavior
- mounted, removable, network, and cloud-storage hints
- native game-process detection

**Required correction:** expand the adapter in bounded stages. Path semantics and discovery should come first. Trash, watchers, process detection, and release integration can follow after the fixture journey is safe.

### P2. Native desktop proof is still asymmetric

The repository has a Windows Tauri WebDriver fixture lane and hosted compile/test coverage for all three systems. It does not yet have equivalent native macOS or Linux desktop automation.

Hosted CI compilation cannot prove Finder behavior, Linux desktop opener behavior, permissions, platform discovery, or a real player journey.

**Required correction:** record hosted runner evidence separately, add macOS fixture automation, retain the real Windows fixture proof, and add Linux native launch, file-manager, and manual-profile proof.

## 4. Boundary classification

### Shared domain core

Keep operating-system neutral:

- content inspection
- duplicate evidence
- version evidence
- placement planning
- ApplyPlan provenance
- preview validation policy
- transaction records
- rollback planning
- performance profiles

The shared core should consume root and path identities supplied by adapters. It should not lowercase paths or infer platform behavior itself.

### Platform adapter

Own:

- platform ID
- root candidate discovery
- filesystem capability probing
- native path comparison keys
- canonical containment evidence
- Finder, Explorer, or Linux file-manager reveal
- permission and sandbox evidence
- space and mount evidence
- recoverable trash primitives
- watcher primitives

### Sims 4 game adapter

Own:

- expected user-data layout
- Mods and Tray semantics
- supported file types
- script-depth rules
- placement rules
- profile validation
- game-specific readiness checks

It must not own Windows, macOS, Wine, or Proton mechanics directly. Those come from the platform and operating-environment layer.

### Presentation layer

Show:

- confirmed profile
- detected candidates and evidence
- native permission guidance
- certainty and blockers
- preview-only status
- native `Show in folder` supporting text

It must not construct authoritative destination paths independently.

### Host-only proof tooling

Windows PowerShell, WebDriver, package rebuild helpers, and platform-specific diagnostics remain implementation details behind portable package scripts. They are not shared application architecture.

## 5. Platform Path Semantics V1 contract

The first implementation batch should introduce types equivalent to:

```text
PlatformId
OperatingEnvironment
FilesystemCapabilities
RootIdentity
PortablePathIdentity
CanonicalContainmentResult
PathMetadataState
```

Minimum capability fields:

```text
case_sensitivity
unicode_normalization
symlink_support
atomic_rename_support
trash_availability
readable
writable
available_space
cloud_or_network_hint
removable_hint
path_length_constraints
```

Minimum portable path identity:

```text
profile_id
root_id
relative_components
comparison_key
native_identity, optional
display_path
```

The comparison key must be derived from the confirmed root's capabilities, not from a universal operating-system guess.

## 6. Required validation tests

Platform Path Semantics V1 and its ApplyPlan integration must cover:

- Windows separator equivalence
- Windows case-insensitive collision
- case-sensitive Linux distinction
- case-sensitive macOS volume distinction
- case-insensitive macOS volume collision
- textual-prefix false positives such as `/Mods` versus `/ModsBackup`
- `.` and `..` components
- symlink escape from an approved root
- symlink that remains inside an approved root
- missing destination descendants under a real canonical ancestor
- unreadable source metadata
- unreadable destination metadata
- missing configured root
- destination equal to the root
- cross-root movement
- duplicate destinations within one plan
- current source path drift
- root relocation with stable profile-relative identity
- non-Unicode native paths where the platform permits them

## 7. Execution order

1. Record hosted CI evidence when authenticated access is available.
2. Correct authoritative status documentation.
3. Add the shared path and filesystem capability types without changing product behavior.
4. Implement read-only platform path probing and canonical containment.
5. Integrate ApplyPlan validation first.
6. Integrate duplicate, scanner, Library query, Downloads, and hidden move-preflight path identity incrementally.
7. Add Installation Profile V1 and Sims4Adapter discovery and validation.
8. Build the isolated macOS fixture journey.
9. Add native macOS automation and run real Windows and Linux proof.
10. Define and verify Light, Balanced, and Fast resource budgets.

## 8. Safety gates

Until the fixture journey and recovery proof pass:

- no public Apply command
- no public Restore command
- no real user-file executor
- no confirmation token issuance
- no delete, quarantine, replace, or cleanup path
- no automatic file decisions
- no frontend-authored result or restore records
- no testing against the player's live Sims folders

The existing command gate and preview-only copy must remain intact throughout the migration.
