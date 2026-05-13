# Auto Sorting Rules Audit v1

Date: 2026-05-13

Source docs:

- `docs/planning/simsuite_master_roadmap_linear_plan.md`
- `docs/planning/NAVIGATION_WORKFLOW_ARCHITECTURE_V1.md`
- `docs/TRUST_BOUNDARIES_AND_AUTOMATION_READINESS.md`
- `simsuite-reports/STAGING_PREVIEW_PLAN_FOUNDATION_V1_REPORT.md`

This document defines the safe rule foundation for future Auto Sorting Suggested Plan work. It does not implement a generator, move files, expose Apply, or change app navigation.

## 1. Auto Sorting Product Definition

Auto Sorting v1 means:

> A preview-only organization suggestion system that proposes where files could go, explains why, shows caveats, and changes no files.

Auto Sorting v1 is not:

- automatic file movement.
- cleanup.
- quarantine.
- delete or safe-delete.
- dependency detection.
- missing-mesh detection.
- AI decision-making.
- update replacement.
- proof that a file is safe to move, safe to delete, or safe to replace.

The first implementation must produce `StagingPlan` items with `wouldTouchFiles=false`. Any future Apply workflow is a separate safety-contract phase.

## 2. Evidence Source Inventory

| Signal | Source system | Available today? | Reliability level | Useful for sorting? | Allowed evidence level | Allowed user-facing wording | Forbidden claims | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| File extension | scanner / `files.extension` | Yes | High for format identity | Yes | deterministic | `Script file`, `Package file`, `Tray file` | `safe to move`, `broken` | Extension can identify broad handling, not content intent. |
| Source root | scanner / `source_location` | Yes | High for current root | Yes | deterministic | `Currently in Mods`, `Currently in Tray` | `belongs here forever` | Source root is fact, not a destination decision by itself. |
| Configured Mods/Tray roots | settings | Yes | High when configured | Yes | deterministic | `Configured Mods folder`, `Configured Tray folder` | `safe destination` | Needed to build suggested paths later. |
| Real folder path | scanner / `library_folders`, `files.path` | Yes | High for current location | Yes | deterministic | `Real folder path`, `Current folder` | `safe folder` | Can support leave-in-place and folder-aware caveats. |
| Relative depth | scanner / `relative_depth` | Yes | High for current shape | Yes | deterministic | `Nested this many folders deep` | `broken placement` | Script depth can trigger review, not an auto-move. |
| Filename | scanner / parser | Yes | Medium | Limited | heuristic | `Filename clue`, `Review-only suggestion` | `confirmed category` | Filename-only should never create a strong move suggestion. |
| Creator metadata | parser / creators tables / user learning | Yes | Mixed | Yes with caveat | evidence-backed when learned or strong; heuristic otherwise | `Creator clue`, `Creator grouping suggestion` | `official creator`, `confirmed source` | User-locked creator paths are stronger than parser guesses. |
| Type/category | parser and file inspector | Yes | Mixed | Yes | evidence-backed when confidence is strong; review-only when weak | `Category clue`, `Suggested category` | `definitely this type` | Confidence and warnings must be checked. |
| Subtype | parser and package inspector | Yes | Mixed | Yes | evidence-backed or heuristic | `Subtype clue` | `confirmed subtype` | Good for CAS/BuildBuy buckets when confidence is high enough. |
| Package DBPF resource metadata | file inspector | Yes | Medium-high for resource facts | Yes | evidence-backed | `Package metadata suggests category` | `dependency`, `missing mesh`, `broken CC` | Resource facts can support category, not dependency truth. |
| Script archive metadata | file inspector | Yes | Medium-high for archive facts | Yes | evidence-backed | `Script archive clue`, `Script placement needs review` | `broken script`, `safe to move` | Good for script bucket and placement caveats. |
| Tray extension and bundle metadata | scanner / bundle detector | Yes | High for Tray type; medium for grouping | Yes | deterministic for root/type; evidence-backed for grouping | `Tray item`, `Tray group` | `complete Tray set` | Incomplete Tray groups should go to review. |
| Bundle / same-pack grouping | bundle detector | Yes | Medium | Yes for grouping | evidence-backed | `Same pack`, `Group together for review` | `depends on`, `requires`, `used by` | Grouping hint only. |
| Exact duplicate proof | duplicate detector | Yes | High for same content | Yes, but only to route review | deterministic | `Duplicate`, `Same file/package/script contents` | `safe to delete` | Exact duplicates should route to duplicate review, not auto-move. |
| Name match | duplicate detector | Yes | Low-medium | Only for review | heuristic | `Name match`, `Manual review needed` | `Duplicate` | Review-only. |
| Version review | duplicate detector / version parser | Yes | Low-medium | Only for review | heuristic | `Version review` | `definitely outdated`, `safe to replace` | Review-only. |
| Update source status | content watch | Yes | Mixed | Review only | evidence-backed or review-only | `No update source`, `Could not check` | `official source`, `latest` | Should not drive folder movement. |
| Review queue membership | scanner / rule engine | Yes | High that review was queued | Yes | evidence-backed | `Needs review`, `Leave in place` | `broken` | Review queue should block strong move suggestions. |
| Parser warnings | parser / scanner | Yes | High that warning exists | Yes | evidence-backed | `Manual review needed` | `broken mod`, `missing dependency` | Warnings route to review or leave-in-place. |
| Inspection warnings/failures | file inspector / scanner | Yes | High that inspection failed | Yes | evidence-backed | `Could not inspect fully` | `broken CC` | Block strong move suggestions. |
| Script placement warning | scanner safety notes / validator | Yes | High that depth is risky | Yes | evidence-backed | `Script placement needs review` | `broken script` | Suggest review; do not auto-move in v1. |
| Thumbnail/preview presence | insights / preview diagnostics | Yes | Low for sorting | No, except display | informational | `Preview available` | `content type confirmed` | Thumbnails are not sorting proof. |
| Package/script content fingerprints | scanner / duplicate detector | Yes after scan | High for same-content proof | Only for duplicate review | deterministic | `Same package contents`, `Same script contents` | `safe cleanup` | Duplicate proof, not organization proof. |
| File size/date/hash | scanner | Yes | High for identity facts | Limited | deterministic for facts, review-only for sorting | `Same file contents`, `Modified date` | `safe to replace` | Use for identity and duplicate review. |
| Existing folder structure | folder metadata / rule engine profile | Yes | Medium | Yes with caveats | heuristic or evidence-backed if user-locked | `Keep current folder`, `Folder-aware suggestion` | `best folder` | Useful for Mirror Mode and leave-in-place. |
| User mode | frontend mode | Yes | High for UI preference | Yes for presentation only | informational | `Simpler review`, `Detailed review` | `safer in Creator mode` | Mode changes exposure/detail, not truth. |
| AI classification | none wired | No | Not supported | No | future work | `Not supported yet` | `AI verified` | Must not be used in v1. |
| Dependency graph / missing mesh | none | No | Not supported | No | future work | `Not supported yet` | `depends on`, `missing mesh` | Explicitly out of scope. |

## 3. Sorting Destination Buckets

| Bucket name | When to suggest | Evidence required | Caveats | When not to suggest | Example user-facing reason |
| --- | --- | --- | --- | --- | --- |
| Script Mods | `.ts4script` extension, script archive inspection, or `ScriptMods` kind. | Deterministic extension plus evidence-backed script inspection or existing kind. | Script placement can be sensitive; keep shallow and review if nested. | Parser failure, conflicting source, unknown target root. | `This is a script file, so SimSuite suggests reviewing it in a Script Mods plan.` |
| CAS | Strong CAS kind/subtype from parser or package resources. | Evidence-backed type with confidence at or above the generator threshold and no review blockers. | Filename or creator clue alone is not enough. | Low confidence, warnings, exact duplicate, or only filename tokens. | `Package metadata suggests CAS content, so this can be previewed under CAS with caveats.` |
| Build/Buy | Strong BuildBuy kind/subtype from package resources or trusted user override. | Evidence-backed category or user override. | Mixed packages may need review. | Weak category, package inspection failure, current clear folder is better. | `Package metadata suggests Build/Buy content, so SimSuite can preview a Build/Buy grouping.` |
| Gameplay | Strong Gameplay kind, script-support package clue, or user override. | Evidence-backed category plus no review blockers. | Gameplay packages often have support files; grouping should stay review-first. | Dependency-like assumptions, parser warnings, filename-only clue. | `SimSuite found gameplay-style metadata, but this remains a suggested destination.` |
| Presets & Sliders | Strong PresetsAndSliders kind or subtype. | Evidence-backed category/subtype. | Creator/subtype may be missing; show caveat. | Weak metadata or conflicting clues. | `Subtype metadata points toward presets or sliders, so this can be reviewed there.` |
| Overrides & Defaults | Strong OverridesAndDefaults kind or explicit user override. | Evidence-backed category or user override. | Overrides can intentionally replace behavior; use review copy. | Filename contains override/default only with no supporting metadata. | `SimSuite has an override/defaults clue, so this should be reviewed before any move.` |
| Tray | Source root is Tray or file extension is a known Tray extension. | Deterministic source/extension; bundle grouping is evidence-backed. | Tray sets can span multiple files; incomplete groups need review. | Tray-like file found in Mods without review, missing Tray root. | `This is Tray content, so SimSuite keeps it in a Tray review plan.` |
| Needs Review | Any warning, low confidence, exact duplicate, conflict, unsupported type, or unclear destination. | Evidence-backed warning or review-only gap. | No destination should be treated as ready. | None; this is the fallback for risky items. | `SimSuite has limited information here, so this item should stay in review.` |
| Unknown / Leave in place | Existing clear folder, no strong improvement, unsupported file, weak metadata, or filename-only clue. | Deterministic current path plus weak or missing sorting evidence. | Leave-in-place is still a preview decision, not an Apply action. | Strong category evidence with no blockers may use a specific bucket. | `The current folder is known, but SimSuite does not have enough evidence to suggest a better destination.` |

## 4. Action Kinds

The current `StagingPlanItem.actionKind` supports `suggest_move`, `suggest_group`, `suggest_review`, and `no_action`. The generator should add or explicitly map `leave_in_place` before implementation.

| Action kind | Meaning | Allowed evidence levels | Destination path allowed? | Can touch files in v1? | User-facing copy | Forbidden copy |
| --- | --- | --- | --- | --- | --- | --- |
| `suggest_move` | Preview a possible destination for one file. | deterministic or evidence-backed only; never filename-only | Yes, but as suggested path only | No | `Suggested destination`, `Review before applying` | `Ready to move`, `safe move`, `sorted automatically` |
| `suggest_group` | Preview grouping related files together without dependency claims. | evidence-backed or heuristic | Optional | No | `Suggested group`, `Related hint` | `depends on`, `requires`, `used by` |
| `suggest_review` | Route item to manual review. | any level | Optional, usually null | No | `Manual review needed`, `SimSuite has limited information here` | `broken`, `bad file` |
| `leave_in_place` | Explicitly recommend no move because current location is clear or evidence is weak. | deterministic current path plus evidence-backed, heuristic, or review-only reason | No new destination | No | `Leave in place`, `No files changed` | `safe as-is`, `confirmed safe` |
| `no_action` | No useful suggestion is available. | review-only or future work | No | No | `No action suggested`, `Not ready to apply yet` | `ignored safely`, `fixed` |

All v1 action kinds must keep `wouldTouchFiles=false`.

## 5. Evidence-Level Rules

### Deterministic

Examples:

- source root is Mods or Tray.
- extension is `.ts4script`.
- exact duplicate proof exists.
- real current folder path exists.
- configured Mods/Tray root exists.

Allowed use:

- explain current location.
- route exact duplicates to duplicate review.
- route scripts to script review/bucket with caveats.
- suggest leave-in-place.
- build folder-aware suggested paths only when no review blockers exist.

Not allowed:

- safe-delete or safe-replace claims.
- dependency claims.
- automatic moves.

### Evidence-backed

Examples:

- package metadata category/subtype.
- script archive metadata.
- user-learned creator or category override.
- review queue reason.
- script placement caution.
- update source state as review context.

Allowed use:

- suggest category with caveats.
- suggest group.
- suggest review.

Not allowed:

- claim a destination is correct without user review.
- use support-file clues as dependency proof.

### Heuristic

Examples:

- filename tokens.
- bracketed creator names without user lock.
- version-like strings.
- same folder.
- same pack.
- same mod family clue.

Allowed use:

- add caveat.
- suggest review-only.
- support an evidence-backed suggestion as secondary context.

Not allowed:

- filename-only strong move suggestions.
- duplicate claims.
- update replacement claims.

### Review-only

Examples:

- missing metadata.
- parser warning.
- inspection failed.
- weak confidence.
- unsupported file type.
- conflicting clues.

Allowed use:

- route to review.
- leave in place.
- show blocked/no-action plan items.

Not allowed:

- destination path that looks ready to apply.
- file movement.

## 6. Do-Not-Move Rules

The v1 generator must not emit a strong `suggest_move` for:

- exact duplicates; route to duplicate review instead.
- files already in Needs Review.
- parser warnings.
- inspection failures.
- low confidence or unknown kind.
- unsupported file types.
- unknown source root.
- `.ts4script` files with placement risk.
- Tray groups with incomplete or conflicting metadata.
- files already in a clear existing folder when moving gives no clear benefit.
- any item where the suggestion is based only on filename, version, same folder, same pack, or family hints.
- files with missing configured target root.
- files with destination collision or preview path collision.
- files from Downloads/Inbox unless the scope is an explicit preview plan for that staged batch.

These items may receive `suggest_review`, `suggest_group`, `leave_in_place`, or `no_action`.

## 7. Per-File Plan Item Requirements

Future generator items must include the current `StagingPlanItem` fields:

- `id`
- `fileId`
- `fileName`
- `currentPath`
- `suggestedDestinationPath`
- `actionKind`
- `evidenceLevel`
- `reason`
- `caveats`
- `wouldTouchFiles=false`

The next implementation should add, or otherwise carry in a compatible response shape:

- `sourceSignals[]`: exact signal names used by the rule.
- `blockedReasons[]`: reasons a stronger suggestion was blocked.
- `bucket`: one of the approved destination buckets.
- `confidenceLabel`: `deterministic`, `evidence-backed`, `heuristic`, or `review-only` display label.
- `currentRoot`: `mods`, `tray`, `downloads`, `inbox`, or `unknown`.

Do not add these fields until the generator sprint unless the API contract is updated with tests.

## 8. Scope Rules For V1 Generator

Recommended first scopes:

- current staged batch.
- selected Library files.
- current Library folder.

Avoid in v1:

- whole-Library auto plan.
- file movement.
- Apply.
- destination folder creation.
- cross-root moves.
- update replacement.
- AI categorization.
- dependency or missing-mesh reasoning.

The generator should be bounded, paged or capped, and preview-only. It should not parse package thumbnails or perform heavy disk work during normal browsing.

## 9. UI Expectations For Future Plan Review

The future plan review UI should:

- group items by suggested destination bucket.
- show reason and caveats for every item.
- mark weak suggestions as review-only.
- keep `No files changed` visible.
- show `Review before applying` copy.
- keep Apply hidden or disabled until the Apply Safety Contract exists.
- provide empty, loading, error, and blocked states.
- wrap long filenames and paths.
- be keyboard accessible.
- work in Casual, Seasoned, and Creator modes with different density, not different truth.
- avoid words like `safe move`, `auto-sort completed`, `quarantine`, `broken`, or `missing dependency`.

## 10. Test Strategy For Future Generator

The generator sprint must add tests that verify:

- deterministic signals produce expected buckets.
- evidence-backed package/script metadata can produce suggested buckets when no blockers exist.
- weak filename-only suggestions become review-only.
- exact duplicates route to duplicate review, not move.
- parser warnings route to review, not move.
- script placement caution blocks strong move.
- unknown type/source leaves item in review or in place.
- every plan and item has `wouldTouchFiles=false`.
- no file operation helper is called.
- no forbidden wording appears in touched UI.
- desktop proof shows the plan and no enabled Apply/move/delete/cleanup/quarantine control.

## Implementation Decision For Next Sprint

Build `Suggested plan generator v1` against `StagingPlan`, not the legacy `OrganizationPreview` apply path.

The legacy Organize preview/apply system still exists and can call file-moving commands after approval. It should be treated as an older implementation surface that needs safety gating before future use. The new generator should remain read-only and should not call:

- `apply_preview_organization`
- `apply_preview_moves`
- `apply_preview_moves_for_files`
- `commit_staging_area`
- `commit_all_staging_areas`
- `cleanup_staging_areas`
- any move-engine apply path

## What This Means For The User

Future organization suggestions should be easier to understand because SimSuite will explain why it suggests a bucket and what it is unsure about. Nothing changes in the app from this audit alone, and no files are moved.

## Trust / Safety Boundary

Auto Sorting remains future preview-only suggested-plan work. SimSuite still does not move, delete, quarantine, clean up, replace, or auto-sort files from this rules audit. AI does not decide file actions.
