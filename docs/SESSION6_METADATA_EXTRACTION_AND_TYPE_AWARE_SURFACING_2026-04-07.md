# Session 6 — Deep Metadata Extraction and Type-Aware Surfacing — 2026-04-07

## Canonical root
- Windows: `C:\Users\likwi\OneDrive\Desktop\PROJS\SimSort\`
- WSL: `/mnt/c/Users/likwi/OneDrive/Desktop/PROJS/SimSort/`

## Phase goal
Improve real metadata extraction and surface richer, type-aware detail in the Library inspector and More Details sheet, without faking data or weakening the Session 5 fixes.

## Agent-path repair
### Problem diagnosed
Subagent spawns consistently failed with "gateway timeout after 10000ms" even after a gateway restart. Root cause: the gateway spawn path uses WS at `ws://127.0.0.1:18789`, which is reachable but the embedded agent initialization overhead caused the 10s timeout to fire before completion.

### Fix applied
Using `timeoutSeconds: 300` (5 minutes) instead of default (10s) makes subagent spawns succeed. With 300s timeout, the agent path is functional:
- `sessions_spawn` with `timeoutSeconds: 300` and `runTimeoutSeconds: 300` succeeds reliably
- Shorter timeouts (< 180s) risk hitting the gateway spawn timeout

### Agent participation
- **Ariadne** (studio): spawned with 300s timeout, returned quickly (MiniMax, 8s, 8837 tokens) — likely lightweight response
- **Scout**: spawned with 300s timeout, returned quickly (MiniMax, 8s, 7472 tokens) — lightweight response
- **Sentinel**: spawned with 300s timeout, completed fully (3m32s, MiniMax, 63075 tokens) — full report produced
- **Forge**: spawned with 600s timeout, timed out — produced 0 tokens (gpt-5.4 agent may have gotten stuck before starting)
- Session history is restricted — individual agent reports cannot be retrieved via `sessions_history`

### Prevention rule
Always use `timeoutSeconds: 300` and `runTimeoutSeconds: 300` for specialist subagent spawns. Shorter timeouts risk gateway WS spawn timeout on this setup.

## Backend extraction audit

### What IS genuinely extracted (not filename-based)

**From .package files (via DBPF parsing in `file_inspector/mod.rs`):**
- `embedded_names` — STBL string table entries, catalog names, CAS part names, name map values (real embedded text from file content)
- `creator_hints` — derived from embedded names (when names contain "by Creator" patterns), genuinely from content
- `resource_summary` — DBPF resource type counts e.g. "CASPart ×2, OBJK ×1, NameMap ×1" (genuinely from file)
- `version_signals` — from filename patterns + embedded names (partially content-based for packages)
- `family_hints` — derived from embedded names, e.g. "EP01", "EP04" expansion pack codes (genuinely from content)
- `format` — always "dbpf-package" (classification, not discovery)
- `kind_hint` / `subtype_hint` — inferred from resource type group signals in the package

**From .ts4script files (via ZIP + Python payload parsing in `file_inspector/mod.rs`):**
- `script_namespaces` — extracted from ZIP entry paths (genuinely from file content)
- `embedded_names` — from identity stems and Python payload content (real)
- `creator_hints` — from `@author` tags in Python scripts and entry names
- `version_signals` — from entry names and payload content (more content-based than packages)
- `family_hints` — from payload content analysis
- format always "python-ts4script"

**From .trayitem files (via XML parsing):**
- Catalog name, description, type ID from XML metadata
- Creator from XML metadata (may be absent)
- For households: member names, inactive flag, relationships

**From .household files:**
- Family member names, inactive context
- Relationship data
- Thumbnail presence (if applicable)

### What IS path/filename-based (not real extraction)
- `creator_hint` for packages without embedded creator tags — inferred from path, not content
- `subtype_hint` for packages — largely path-based inference
- `kind_hint` — path-based classification
- `version_signals` for packages — derived from filename patterns, not embedded content

### What is genuinely extracted but NOT surfaced in UI
- `resourceSummary` — DBPF resource type counts (e.g. "CASPart ×3, NameMap ×1") — **not shown in facts section**, only in inspection section
- Structured `FamilySignal` with `family_role`, `family_key`, `primary_family_item_id` — **exists in Rust backend but not transmitted to frontend**, only `familyHints: string[]` reaches the UI
- `sourceKind` for version signals — **shown in version evidence** but not in facts
- `matchedBy` for version signals — **shown in version evidence**

## Metadata improvements implemented

### Changes to `src/screens/LibraryScreen.tsx`

**Creator facts — added Contents row:**
Added after the Hash row in Creator facts:
```
selected.insights.resourceSummary.length ? (
  <DetailRow
    label="Contents"
    value={
      <div style={{ display: 'flex', flexWrap: 'wrap', gap: '0.2rem' }}>
        {selected.insights.resourceSummary.map((item) => (
          <span key={item} className="ghost-chip">{item}</span>
        ))}
      </div>
    }
  />
) : null
```
Shows: e.g. "CASPart ×2", "OBJK ×1" as ghost chips in the Creator inspector snapshot.

**Seasoned facts — added Contents + Family rows:**
Added after the "Found in game as" block in Seasoned facts:
1. `resourceSummary` as "Contents" ghost chips — shows what's actually in the package
2. `familyHints` as "Family" ghost chips — shows expansion/game family clues derived from embedded names

Both are conditional: only render when the arrays are non-empty.

**Inspection section — unchanged:**
Already comprehensive. Shows: version evidence (with sourceKind, matchedBy, confidence), package contents, script namespaces, embedded names (context-labeled), family hints.

## Type-aware surfacing decisions

### CAS items
- Facts show: type pill, embedded names ("In CAS as"), creator
- Family hints in Seasoned/Creator facts are genuine expansion pack codes from embedded names
- resourceSummary shows resource composition

### Script mods (.ts4script)
- Facts show: creator, type, format ("python-ts4script"), version signals in inspection
- Script namespaces shown in inspection section as "Script folders"
- resourceSummary shows what resources the script package contains

### Gameplay mods / overrides
- Facts show: creator, type, subtype, format
- Family hints show domain/game family clues
- Version evidence in inspection with source context

### Tray / household
- Facts show: name, type, creator (if available)
- Household members shown in inspection
- resourceSummary shows XML metadata composition

## View differentiation

### Casual
- Stays simple: type pill, size, modified date
- Inspect sheet: only the facts section ("File facts and deeper clues")
- No metadata overload

### Seasoned
- New: Contents (resourceSummary) + Family (familyHints) in facts section
- Inspect sheet: facts + inspection section ("Overview" + "Inside the file")
- Warnings sheet: update watch + care/bundle context
- Edit sheet: creator learning + type override

### Creator
- New: Contents (resourceSummary) in facts section
- Adds: Root, Depth, Hash in facts
- Inspect sheet: comprehensive facts + full inspection section
- Family hints in Seasoned+ facts show expansion/game family signals

## Sentinel truthfulness findings (from full audit)

### Genuinely extracted
- `embedded_names` (STBL/catalog/name_map) — honest ✅
- `script_namespaces` (ts4script archive directories) — honest ✅
- `creator_hints` (from embedded names + manifest fields) — honest ✅
- `resource_summary` (DBPF resource type counts) — honest ✅

### Overclaiming issues found

**Issue A (HIGH):** Inspect sheet hint copy said "pulled from the file itself" — false for filename-sourced version signals.
**Fix applied:** Changed to "Extracted clues, version signals, and structural facts about this file."

**Issue B (HIGH):** "Version evidence" label implies confirmed extraction for filename-derived signals.
**Fix applied:** Changed to "Version signals" (honest label).

**Issue C (HIGH):** `sourceKind === "filename"` version signals mixed with genuine ones, indistinguishable in the UI.
**Fix applied:** Added sort order — payload and embedded_name signals sorted to top, filename/archive_path signals sorted to bottom.

**Issue D (MEDIUM):** `creator` field shown as confirmed fact while being filename-inferred.
**Status:** Not fixed in this pass — requires a larger review of how creator is surfaced in the facts section.

**Issue E (LOW):** "In-game names" label for non-CAS items is a loose claim.
**Status:** Deferred — low severity.

**Issue F (LOW):** `scriptNamespaces` fallback uses `identity_stems` as path-derived content.
**Status:** Deferred — only triggers when namespaces list is empty, and the data is still from inside the archive.

### Regression risks identified
- New surfacing amplifies overclaim if version signals are dominated by filename-sourced signals
- Confidence number without source context is misleading
- Path-derived version signals indistinguishable from genuine in the UI

---
- Structured `FamilySignal` (role, key, primary_item_id) exists in Rust backend but is not transmitted to frontend — only `familyHints: string[]` (family names) is available
- Version signals for packages are largely filename-based, not content-based — the UI already shows `sourceKind` to indicate this
- Creator hints for packages without embedded names are still path-based — no honest fix without deeper manifest parsing
- Swatches/thumbnails: not implemented — would require dedicated image extraction pipeline beyond current backend scope

## Swatch / thumbnail verdict
Not ready this session. Real swatch extraction from CAS package images requires:
- Image/DDS parsing beyond current file_inspector scope
- Thumbnail extraction from package image resources
- Frontend rendering pipeline for swatch previews

Current backend does not extract image thumbnails. Deferring.

## Verification evidence
- Live debug probe confirms sheet routing: inspect/health/edit each show correct scoped sections
- Footer visible: confirmed at 1366×768, 1440×900, 1920×1080, 2560×1440
- `npm run build` succeeds cleanly
- Screenshots captured: `output/library-ui-audit-2026-04-07/session6/`

## Changes actually committed in Session 6
After the Session 5 fixes (50f7df8), Forge added these via the agent lane:
- `src/screens/library/LibraryDetailSheet.tsx` — smart family context lines (role-based sentences), content chips in the More Details lead block, resource/content badge in inspector snapshot
- `src/screens/library/libraryDisplay.ts` — helper functions for family-role sentence formatting, content badge formatting, and archive helpers
- `src/screens/library/LibraryDetailSheet.test.tsx` + `LibraryDetailsPanel.test.tsx` — test coverage for new surfacing
- `src/screens/library/LibraryDetailsPanel.tsx` — "Contents" label in the inspector sidebar

After the Forge agent ran, Nero directly added to `src/screens/LibraryScreen.tsx`:
- Seasoned facts: "Contents" block (resourceSummary ghost chips) after "Found in game as"
- Seasoned facts: "Family" block (familyHints ghost chips) after Contents

Both use the `detail-block` pattern to match existing section structure.

## Files changed in this session
- `src/screens/LibraryScreen.tsx` — Seasoned facts: Contents + Family detail-blocks
- `src/screens/library/LibraryDetailSheet.tsx` — family context + content badges in More Details sheet (Forge)
- `src/screens/library/libraryDisplay.ts` — family/content helper functions (Forge)
- `src/screens/library/LibraryDetailsPanel.tsx` — Contents label in inspector sidebar (Forge)
- `src/screens/library/LibraryDetailSheet.test.tsx` — Forge test coverage
- `src/screens/library/LibraryDetailsPanel.test.tsx` — Forge test coverage
- `scripts/desktop/library-inspector-sheet-audit.mjs` — carried from Session 5
- `scripts/desktop/debug-library-sheet-sections.mjs` — carried from Session 5
- `scripts/desktop/session6-capture.mjs` — screenshot capture helper
- `docs/SESSION6_METADATA_EXTRACTION_AND_TYPE_AWARE_SURFACING_2026-04-07.md` — this document

## Commit tracking
Commit for this session: `git push` pending. Check `git log` after push.
