# Confirmation Design V1

Date: 2026-05-26

## Purpose

Confirmation Design V1 is a read-only product and safety contract for future Organize Apply work. It explains what SimSuite must prove before any file-changing workflow can exist.

No files changed. Apply is not implemented. Restore is not implemented. The confirmation token is not issued in V1.

## Current implementation boundary

The current Organize saved-plan details screen may show a `Confirmation Design V1` section. That section is informational only.

It must not:

- move files;
- copy files;
- create folders;
- delete files;
- quarantine files;
- replace files;
- run backup;
- run restore;
- write result logs;
- write restore-map entries;
- issue a confirmation token;
- make AI-only decisions executable.

The only allowed behavior is rendering saved plan metadata, validation preview status, dry-run classification, and recovery-history metadata that already exists.

## Confirmation contract, future-only

Before real confirmation can exist, SimSuite needs all of the following:

1. **Backend-owned draft plan** — the plan must be created and persisted by backend code, not by frontend-only assembly.
2. **Plan hash/provenance snapshot** — every future confirmation must reference an immutable plan hash.
3. **Folder configuration snapshot** — any default or custom folder configuration must be captured with the plan and invalidated if changed.
4. **Exact operation list** — every operation must show source path, destination path, operation kind, and exclusion reason for skipped rows.
5. **Validation rerun** — validation must be re-run immediately before confirmation.
6. **Canonical root checks** — all source, destination, backup, and restore paths must be canonicalized by backend code.
7. **Backup first** — backup material must exist before any mutation.
8. **Restore map first** — restore map rows must be produced by the backend executor, not the UI.
9. **Backend result logs** — result logs must come from observed backend operations.
10. **Single-use confirmation token** — the token must be backend-issued, single-use, and invalidated by plan/config changes.

## Custom folder configurations

Custom folder configurations are future preview inputs only.

The design direction is to let users define how their Apply plans should organize files, including preferred folder buckets, naming conventions, root-level limits, creator/category folders, and exclusions.

For safety, custom folder configurations must be handled as data that influences preview generation and validation, not as an immediate file-changing instruction.

Future requirements:

- each ApplyPlan stores a folder configuration snapshot;
- the snapshot records allowed roots and proposed folder creations;
- destination changes caused by custom folders are visible before confirmation;
- changing a custom configuration invalidates any pending plan hash and token;
- every configured destination is revalidated before confirmation;
- review-only, duplicate-review, weak-metadata, conflict, or unsupported-cross-root rows stay excluded.

## Cross-system context

The long-term product shape is an organic evidence graph: decisions from one SimSuite area should inform other areas without silently authorizing file changes.

Allowed cross-system context:

| System | May influence | Must not do |
| --- | --- | --- |
| Library | current indexed paths, roots, hashes, folder facts, package evidence | authorize Apply alone |
| Inbox / Downloads | origin, intake context, staged source facts | commit or reject files from Organize confirmation |
| Updates | watch source caveats and possible version context | automatically replace installed files |
| Duplicates | exact duplicate proof and review routing | delete, quarantine, or clean up from confirmation V1 |
| Creator and Category Audit | metadata confidence, creator/category bucket hints | overrule validation blockers |
| Organize | saved draft plan and operation preview | execute file mutation |
| Validation | roots, conflicts, missing paths, blockers | create backup or result logs |
| Dry-run | read-only classification | issue confirmation |
| Recovery | existing result/restore metadata | restore files |

This gives SimSuite the smart, seamless feel the product needs: Library evidence, Inbox origin, Updates caveats, duplicate truth, creator/category confidence, Organize plans, validation, dry-run, and recovery history should all feed the user's decision. The backend safety contract still owns whether an operation is even eligible for future confirmation.

## UI requirements

The V1 UI should show:

- `Confirmation Design V1`;
- `Execution controls disabled`;
- `Confirmation token not issued`;
- `No files changed`;
- future candidate, blocked, and skipped/review-only counts;
- plan provenance;
- custom folder configuration status;
- future operation list;
- cross-system context trail;
- required gates before confirmation can exist;
- a disabled `Confirmation unavailable` control.

Forbidden UI copy:

- `ready to apply`;
- `safe to move`;
- `safe to delete`;
- `safe to replace`;
- `AI verified`;
- `confirmed safe`;
- any enabled Apply, Restore, Backup, Delete, Quarantine, Replace, Cleanup, or Commit control.

## V1 result

Confirmation Design V1 improves clarity without crossing the safety line. It makes future Apply feel coherent and user-configurable while preserving the current boundary: SimSuite may explain and preview, but it still cannot mutate user files.
