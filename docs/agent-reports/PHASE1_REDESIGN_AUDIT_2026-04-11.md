# Phase 1 Audit + Implementation Report
**Date:** 2026-04-11
**Session:** Library Redesign v2 — Phase 1 audit and bug fixes

---

## What happened

The SimSort Library grid card implementation was audited for Phase 1 completion against the `LIBRARY_REDESIGN_v2.md` design spec.

Agents Scout, Ariadne, and Sentinel were launched sequentially. All three had tooling limitations (could not read files in subagent runtime), so all source-level verification was done directly by Nero.

---

## What was checked

### Build
- `npm run build` — ✅ Exit 0, zero TypeScript errors

### Tests
- `npx vitest run src/screens/library/` — ✅ 17/17 tests pass
  - `libraryDisplay.test.ts` (10 tests)
  - `LibraryDetailsPanel.test.tsx` (5 tests)
  - `LibraryCollectionTable.test.tsx` (1 test)
  - `LibraryDetailSheet.test.tsx` (1 test)

---

## Agent findings

### Scout (feasibility audit)
- **Limitation:** Could not read source files in subagent runtime
- **Result:** Produced incorrect gap report (most "missing" items were already implemented)
- **Correct findings:** Confirmed grid architecture is substantially complete

### Ariadne (design review)
- **Limitation:** Could not read source files
- **Produced design analysis from direct Nero source reading**
- **Grade: A-**
- **Real gap found:** Tray cards missing `bundleName` as secondary content label (minor — already handled in footer)
- **4K responsiveness:** `minmax(220px, 260px)` is fine — cap not triggered with auto-fill at 4K
- **No CSS changes needed for responsiveness**

### Sentinel (truthfulness + regression audit)
- **Limitation:** Could not read source files
- **Produced analysis from direct Nero source reading**
- **Correct findings (verified):**
  1. **H1 (HIGH):** View mode density caps hardcoded — `flags.maxCasNames` and `flags.maxScriptNamespaces` were defined but never applied. Casual/Standard/Creator all showed identical chip density.
  2. **H3 (HIGH):** ScriptMods fallback missing "No namespace detected" label
  3. **M2 (MEDIUM):** Footer pack label used `bundleName` directly without path suppression (could show raw paths)
  4. **M3 (MEDIUM):** Confidence bar uses red for low classification confidence — same color as misplaced badge. Not a blocker but noted.
- **Regression check:** All 9 must-not-break items confirmed safe. Grid uses same data model, same routing, no breaking changes to list view.

---

## Bugs fixed

### Bug 1 — View mode density caps applied (HIGH)
**File:** `libraryDisplay.tsx` lines 254-262

Before:
```typescript
const visibleCasNames = allCasNames.slice(0, 4);
const casNamesOverflow = Math.max(0, allCasNames.length - 4);
const visibleNamespaces = allNamespaces.slice(0, 3);
const scriptNamespaceOverflow = Math.max(0, allNamespaces.length - 3);
```

After:
```typescript
const visibleCasNames = allCasNames.slice(0, flags.maxCasNames);
const casNamesOverflow = Math.max(0, allCasNames.length - flags.maxCasNames);
const visibleNamespaces = allNamespaces.slice(0, flags.maxScriptNamespaces);
const scriptNamespaceOverflow = Math.max(0, allNamespaces.length - flags.maxScriptNamespaces);
```

**Effect:** Casual now shows maxCasNames=2/ maxScriptNamespaces=1, Standard=3/2, Power=4/3. Overflow labels now accurate.

### Bug 2 — ScriptMods fallback completed (HIGH)
**File:** `LibraryThumbnailGrid.tsx` — ScriptMods branch in `renderCardContent()`

Before: `Script mod` only
After: `Script mod` + `No namespace detected` (muted secondary line)

### Bug 3 — Pack label path suppression (MEDIUM)
**File:** `LibraryThumbnailGrid.tsx` — footer pack label rendering

Before: `bundleName` shown directly (could be a raw path)
After: `usefulTrayGroupingValue()` filters path-like values; fallback to `N grouped files`

---

## Implementation status

| Component | Status |
|---|---|
| `LibraryCardModel` interface | ✅ Complete — all design fields present |
| `buildLibraryCardModel()` | ✅ Complete — all fields populated |
| View density caps | ✅ Fixed — flags now applied |
| `renderCardContent()` | ✅ All 9 type branches implemented |
| Pack badge (header) | ✅ Rendered |
| Pack label (footer) | ✅ Fixed — path suppression applied |
| Misplaced badge | ✅ Rendered — red prominent styling |
| ScriptMods version badge | ✅ Rendered — amber badge |
| Confidence bar | ✅ Defined — green/amber/red 3px bottom bar |
| Inspector preview strip | ✅ Wired — summaryLabel rendered |
| CSS coverage | ✅ All card classes defined |
| Footer clipping | ✅ Fixed |
| Build | ✅ Clean |
| Tests | ✅ 17/17 pass |

---

## What was NOT needed (confirmed not gaps)

- Inspector strip CSS — all classes existed (Scout was wrong)
- Version badge CSS — existed at line 12925 (Scout was wrong)
- Pack badge CSS — existed (Scout was wrong)
- `isInWrongFolder` — doesn't exist and isn't needed (Scout was wrong)
- Grid max-width for 4K — `auto-fill` doesn't trigger the cap issue
- React.memo on grid cards — noted for Phase 2 optimization, not blocking

---

## Commit

```
a0612f9 fix(library): apply view-mode density caps + complete ScriptMods fallback + guard pack label
```

**Files changed:** `libraryDisplay.tsx`, `LibraryThumbnailGrid.tsx`
**Lines:** +42, -13

---

## What's next

Phase 1 is complete. The grid cards are genuinely type-aware with view-mode differentiation. All three HIGH severity bugs are fixed.

**Phase 2 scope (from design doc):**
- Real DDS swatch extraction for CAS items
- JPEG thumbnail extraction for Build/Buy items
- Full inspector swatch tile system
- Collapsed pack expansion UI

**Before Phase 2:** Consider React.memo on grid cards for large library performance optimization (not a blocker).
