# Library Redesign Research

## What LibraryFileRow actually provides

**Available at list level:**
- `filename` — primary identifier
- `kind` — PascalCase: `ScriptMods`, `CAS`, `BuildBuy`, `OverridesAndDefaults`, `PosesAndAnimation`, `PresetsAndSliders`, `TrayHousehold`, `TrayLot`, `TrayRoom`, `TrayItem`, `Unknown`
- `subtype` — e.g. "hair", "tops", "pose_pack", freeform
- `source_location` — `"tray"` or `"mods"` (not `"downloads"`)
- `creator` — canonical creator name
- `confidence` — 0.0–1.0
- `watch_status` — `NotWatched | Current | ExactUpdateAvailable | PossibleUpdate | Unknown`
- `has_duplicate` — boolean
- `safety_notes` — `string[]`
- `parser_warnings` — `string[]`
- `bundle_name` / `bundle_type` — for grouped items
- `relative_depth` — folder depth
- `size` — bytes
- `modified_at` — ISO timestamp
- `extension` — file extension (e.g., ".package", ".ts4script")

**NOT available in list (only in FileDetail/inspector):**
- `installed_version` — only resolved at detail level via content_versions
- `hash` — only in FileDetail
- `insights` — only in FileDetail
- `watch_result` (deeper) — only in FileDetail
- `duplicates_count` — only in FileDetail

## kind values (confirmed from backend)
- `ScriptMods` — script .ts4script files
- `CAS` — Create-a-Sim items
- `BuildBuy` — build/buy objects
- `OverridesAndDefaults` — default replacements
- `PosesAndAnimation` — pose/animation packs
- `PresetsAndSliders` — sliders and presets
- `TrayHousehold`, `TrayLot`, `TrayRoom`, `TrayItem` — tray/placed items
- `Unknown` — unclassified

## source_location values
- `"tray"` — placed in world/tray (effectively disabled, not loaded at game start)
- `"mods"` — in Mods folder (loaded at game start)

## key insight
`kind` in the database IS the PascalCase TYPE_LABELS key — so `friendlyTypeLabel(kind)` works directly without case conversion needed.

## What can be shown at LIST level (realistically)
- Type color dot (kind-based)
- Filename + source icon (tray = disabled indicator)
- Watch status pill (colored)
- Duplicate indicator
- At-a-glance facts: creator + primary fact by type
- Safety/warning indicator (has warnings)

## What belongs in INSPECTOR only
- Full watch result with source URL and update info
- Version information (installed + available)
- Full duplicate list
- File hash
- Parser warnings + safety notes detail
- Deep insights

## Type-specific display
| Type | List at-a-glance | List status | Inspector extras |
|---|---|---|---|
| CAS | creator + subtype | tray badge | swatch info, CAS part |
| ScriptMods | creator + confidence | update badge | version, platform |
| BuildBuy | creator | tray badge | lot type, room tags |
| OverridesAndDefaults | creator | update badge | default replacement type |
| PosesAndAnimation | creator + pack | update badge | animation type |
| PresetsAndSliders | creator + area | — | body system |
| TrayHousehold/Lot/Room | creator + tray badge | — | lot/household data |
| Unknown | creator | — | raw file info |
