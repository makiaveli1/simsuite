# Session 10 — Tray / Household / Lot Intelligence Pass

Date: 2026-04-08
Canonical root: `/mnt/c/Users/likwi/OneDrive/Desktop/PROJS/SimSort/`

## Goal
Give tray content real product dignity in Library without faking metadata.

## Agent participation
Required agents were actually used.

Observed runtime status:
- Ariadne: completed
- Scout: completed
- Sentinel: completed
- Forge: initial timeout, retried after gateway cooldown and accepted

### Retrieval constraint
Cross-agent history retrieval was blocked by session-visibility policy during this session. Real completion status was confirmed via subagent runtime status, but report bodies were not accessible from the main session. No quotes or detailed conclusions were fabricated.

## Honest tray extraction audit
Current tray intelligence is materially weaker than package/script extraction.

### Real signals available now
- `kind` classification distinguishes tray-oriented content in the UI layer:
  - `TrayHousehold`
  - `TrayLot`
  - `TrayRoom`
  - `TrayItem`
  - live-path compatibility widened to also handle plain `Household` / `Lot` / `Room`
- `sourceLocation` distinguishes `tray` vs `mods`
- `bundleName` and `bundleType` provide grouping clues when available
- `safetyNotes` already flag misplaced tray content, e.g. tray files sitting in Mods
- `hash`, file size, modified date, extension, and path exist for diagnostics

### Weak / underused signals before Session 10
- tray rows were basically `🔖 Tray` + creator
- inspector used the same generic structure as package/script content
- More Details did not visibly explain tray identity or placement context
- tray content often had empty `insights`, so generic “What’s Inside” sections added little value

### Not actually available
These should not be overstated:
- real extracted household member identity
- real extracted lot name from deep binary parsing
- strong creator identity for many tray files
- script-like namespaces / embedded-name richness for tray files

## Trust-model decisions
Tray metadata is now treated like this:

### Extracted
- file size
- modified date
- hash / fingerprint
- raw file path / extension

### Derived
- `bundleName`
- `bundleType`
- grouping hints surfaced as “Grouped as” / related tray hints

### Inferred
- tray kind labels when driven by classification (`Household`, `Lot`, `Room`, `Tray Item`)
- storage interpretation (`Stored in Tray`, `Stored in Mods`)
- load behavior messaging

### Unknown / unavailable
- household members
- lot title if not already represented by bundle/group clues
- creator where none exists

## UI decisions implemented

### Row surfacing
Tray rows now surface tray-specific clues instead of feeling like weak generic leftovers.

Implemented behavior:
- tray rows still use the tray type color
- tray supporting facts now prefer:
  1. `bundleName` when present
  2. placement clue (`🔖 Tray` or `Misplaced tray`)
  3. creator only when actually available

Examples verified live:
- `OakHousehold_0x00ABCDEF.trayitem` → stronger household identity in row context
- `LooseBlueprint.blueprint` → `Misplaced tray` surfaced clearly

### Inspector surfacing
Inspector now uses tray identity directly.

Implemented behavior:
- tray items get explicit `Tray type`
- tray items get explicit `Stored` context
- tray items get `Grouped as` when bundle/group data exists
- tray care text is tray-aware:
  - in Tray → library object, not an active mod
  - in Mods → tray content outside Tray, review needed

### More Details surfacing
The inspect sheet now uses tray-aware file facts and tray-specific inspect routing.

Implemented behavior:
- inspect routing widened so tray items are included even when classic extraction signals are sparse
- inspect-mode section filtering updated to allow tray-aware inspect content instead of stale section ids
- file-facts content widened for tray items so sheet-level detail can show storage/grouping context instead of acting like a script/package-only surface

## Live verification
Verified against real live app state in Creator view with browser automation.

### Examples checked
1. `OakHousehold_0x00ABCDEF.trayitem`
2. `LooseBlueprint.blueprint`

### What was visually confirmed
- tray rows visibly read as tray/library objects, not active mods
- lot example clearly surfaces review-needed placement context
- inspector now shows:
  - `Tray type`
  - `Stored`
  - `Grouped as` (when available)
- lot example clearly shows `Stored in Mods · review needed`
- no footer clipping regression observed

## Smoke tests
Added/updated coverage for:
- tray row model surfacing (`bundleName`, misplaced tray clue)
- tray-aware care summary wording
- tray inspector identity/storage/grouping display

## Build / test status
- `npm run build` — clean during Session 10
- library-focused tests updated and passing during Session 10
- full `vitest` run completed with default reporter after invalid `basic` reporter attempt

## Known limits after Session 10
- tray intelligence is still honest-but-shallow compared with script/package extraction
- no deep household member parsing was added
- no swatch/preview extraction was started
- no fake creator identity was introduced for unknown tray files

## Focused cleanup — tray grouping path leak

### Root cause
The leak was frontend formatting, not a backend requirement.

Primary UI tray grouping was trusting raw grouping inputs too early:
- row clues trusted `bundleName`
- inspector tray summary trusted `bundleName`
- tray More Details related hints were built from raw `bundleName` + `familyHints`

If any of those values were path-like, the UI would surface a full local machine path as if it were useful grouping information.

### Formatting rule adopted
Primary tray UI now follows this rule:
- show short human-readable grouping only
- suppress path-like grouping values from row clues, inspector summaries, and tray More Details summary rows
- use tray storage labels like `Stored in Tray` and `Stored in Mods · review needed`
- allow raw paths only in deep technical/file-facts areas where the user explicitly expects diagnostics

### Fix applied
Added helper-layer sanitization in `libraryDisplay.tsx`:
- `isPathLikeValue(...)`
- `usefulTrayGroupingValue(...)`

Then rewired tray surfaces to use the sanitized value:
- row clues
- inspector tray summary
- tray More Details grouping

If the grouping signal is only a raw path, it is now hidden from primary UI.

## Session 10 outcome
Tray content is materially clearer in rows and inspector, with explicit storage and grouping context, and the trust model stays honest about what is inferred versus actually known.
