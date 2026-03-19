# Library Quiet Catalog Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Rework `Library` into the approved quiet catalog so simmers can scan their collection calmly, understand one file at a time, and only open deeper detail when they need it.

**Architecture:** Keep `src/screens/LibraryScreen.tsx` as the data-loading and navigation root, but move view-specific shaping and the large page sections into focused files under `src/screens/library/`. Use one pure display helper for row shaping, health labels, and per-view defaults so the calm browsing rules are locked with tests before the page layout is rearranged. Reuse the existing workbench shell, motion tokens, and sheet patterns from `Downloads`, `Updates`, and `Home` instead of inventing a second UI system.

**Tech Stack:** React 19, TypeScript, Motion, global CSS, Vitest, Testing Library, existing SimSuite workbench layout

---

## Preflight

- Work on `codex/library-quiet-catalog-design`, not `main`.
- Keep unrelated files out of every commit.
- Update `SESSION_HANDOFF.md` and `docs/IMPLEMENTATION_STATUS.md` after the final verification step.
- Manual review note:
  - the plan-review subagent loop is still not cleanly available in this environment
  - manually review the saved plan against `docs/superpowers/specs/2026-03-19-library-redesign-design.md` before starting Task 1

## File Map

### New files

- `src/screens/library/libraryDisplay.ts`
  - pure helpers for row shaping, per-view column choices, health copy, and detail-sheet availability
- `src/screens/library/libraryDisplay.test.ts`
  - unit tests for the pure Library display rules
- `src/screens/library/LibraryTopStrip.tsx`
  - slim header strip with counts, search, and `More filters`
- `src/screens/library/LibraryFilterRail.tsx`
  - lighter left rail for the default narrowing controls
- `src/screens/library/LibraryCollectionTable.tsx`
  - calmer middle list with view-specific supporting facts
- `src/screens/library/LibraryDetailsPanel.tsx`
  - short right understanding panel with `Snapshot`, `Care`, and `More`
- `src/screens/library/LibraryDetailSheet.tsx`
  - right-side sheet for `Health details`, `Inspect file`, and `Edit details`
- `src/screens/library/LibraryFilterRail.test.tsx`
  - render tests for the lighter rail and `More filters` behavior
- `src/screens/library/LibraryCollectionTable.test.tsx`
  - render tests for row content differences across user views
- `src/screens/library/LibraryDetailsPanel.test.tsx`
  - render tests for the calm inspector and outward actions

### Modified files

- `src/screens/LibraryScreen.tsx`
  - shrink into orchestration for data loading, selection, sheet state, and outward navigation
- `src/styles/globals.css`
  - replace the current Library-specific layout with the calmer quiet-catalog styling
- `SESSION_HANDOFF.md`
  - record implementation progress, checks, open gaps, and next step
- `docs/IMPLEMENTATION_STATUS.md`
  - record the Library redesign progress and checks

### Reference files

- `docs/superpowers/specs/2026-03-19-library-redesign-design.md`
  - approved Library spec
- `src/screens/downloads/downloadsDisplay.ts`
  - pure view-model pattern already used on the Downloads rebuild
- `src/screens/downloads/DownloadsRail.tsx`
  - current calm rail pattern with on-demand controls
- `src/screens/downloads/DownloadsDecisionPanel.tsx`
  - compact right-side summary pattern
- `src/lib/uiLanguage.ts`
  - existing user-view wording helpers
- `src/lib/motion.ts`
  - shared calm motion tokens

## Task 1: Lock The Calm Browsing Rules In Pure Helpers

**Files:**
- Create: `src/screens/library/libraryDisplay.ts`
- Create: `src/screens/library/libraryDisplay.test.ts`

- [ ] **Step 1: Write the failing unit tests**

Create `src/screens/library/libraryDisplay.test.ts`:

```ts
import { describe, expect, it } from "vitest";
import {
  buildLibraryRowModel,
  libraryViewFlags,
  summarizeLibraryCareState,
} from "./libraryDisplay";

describe("libraryViewFlags", () => {
  it("keeps technical detail out of casual mode", () => {
    expect(libraryViewFlags("beginner").showInspectFactsInList).toBe(false);
  });
});

describe("buildLibraryRowModel", () => {
  it("caps the row at two supporting facts", () => {
    const row = buildLibraryRowModel(
      {
        id: 1,
        filename: "MCCmdCenter.package",
        path: "Mods\\\\Scripts\\\\MCCmdCenter.package",
        creator: "Deaderpool",
        kind: "ScriptMods",
        sourceLocation: "mods",
        relativeDepth: 2,
        confidence: 0.91,
      },
      "standard",
    );

    expect(row.supportingFacts).toHaveLength(2);
  });
});

describe("summarizeLibraryCareState", () => {
  it("prefers action wording a casual simmer can understand", () => {
    expect(
      summarizeLibraryCareState({
        installedVersionSummary: null,
        safetyNotes: ["Possible conflict"],
        parserWarnings: [],
      }),
    ).toContain("needs attention");
  });
});
```

- [ ] **Step 2: Run the new unit tests and confirm they fail**

Run:

```bash
npm run test:unit -- libraryDisplay
```

Expected: FAIL because the helper module does not exist yet.

- [ ] **Step 3: Implement the minimal display helpers**

Create `src/screens/library/libraryDisplay.ts` with helpers like:

```ts
import { friendlyTypeLabel } from "../../lib/uiLanguage";
import type { FileDetail, LibraryFileRow, UserView } from "../../lib/types";

export function libraryViewFlags(userView: UserView) {
  return {
    showCreatorInList: userView !== "beginner",
    showInspectFactsInList: userView === "power",
    showAdvancedFilters: userView === "power",
    showCareSummaryByDefault: true,
  };
}

export function buildLibraryRowModel(row: LibraryFileRow, userView: UserView) {
  const supportingFacts = [
    friendlyTypeLabel(row.kind),
    row.creator ?? "Creator unknown",
    row.sourceLocation === "tray" ? "Tray" : "Mods",
    `${Math.round(row.confidence * 100)}% match`,
  ].slice(userView === "power" ? 3 : 2);

  return {
    id: row.id,
    title: row.filename,
    typeLabel: friendlyTypeLabel(row.kind),
    supportingFacts,
  };
}

export function summarizeLibraryCareState(detail: Pick<FileDetail, "installedVersionSummary" | "safetyNotes" | "parserWarnings">) {
  if (detail.safetyNotes.length || detail.parserWarnings.length) {
    return "This file needs attention before you forget about it.";
  }

  if (detail.installedVersionSummary) {
    return "This file has update tracking ready if you want to check it.";
  }

  return "Nothing stands out right now.";
}
```

- [ ] **Step 4: Run the tests again**

Run:

```bash
npm run test:unit -- libraryDisplay
```

Expected: PASS with the new Library display tests green.

- [ ] **Step 5: Commit the display-rule helper**

```bash
git add src/screens/library/libraryDisplay.ts src/screens/library/libraryDisplay.test.ts
git commit -m "test: add library display rules"
```

## Task 2: Rebuild The Left Rail And Top Strip Around Calm Narrowing

**Files:**
- Create: `src/screens/library/LibraryTopStrip.tsx`
- Create: `src/screens/library/LibraryFilterRail.tsx`
- Create: `src/screens/library/LibraryFilterRail.test.tsx`
- Modify: `src/screens/LibraryScreen.tsx`
- Modify: `src/screens/library/libraryDisplay.ts`
- Modify: `src/styles/globals.css`

- [ ] **Step 1: Write the failing render test for the lighter rail**

Create `src/screens/library/LibraryFilterRail.test.tsx`:

```tsx
import { render, screen } from "@testing-library/react";
import { LibraryFilterRail } from "./LibraryFilterRail";

it("keeps only the core filters visible in casual mode", () => {
  render(
    <LibraryFilterRail
      userView="beginner"
      facets={{
        creators: ["Lumpinou"],
        kinds: ["CAS"],
        sources: ["mods"],
        subtypes: ["Hair"],
        taxonomyKinds: ["CAS"],
      }}
      filters={{
        kind: "",
        creator: "",
        source: "",
        subtype: "",
        minConfidence: "",
      }}
      search="hair"
      activeFilterCount={1}
      isCollapsed={false}
      onToggleCollapsed={() => {}}
      onSearchChange={() => {}}
      onFilterChange={() => {}}
      onReset={() => {}}
      onOpenMoreFilters={() => {}}
    />,
  );

  expect(screen.getByLabelText(/search/i)).toBeVisible();
  expect(screen.queryByLabelText(/confidence/i)).not.toBeInTheDocument();
});
```

- [ ] **Step 2: Run the rail test and confirm it fails**

Run:

```bash
npm run test:unit -- LibraryFilterRail
```

Expected: FAIL because the component does not exist yet.

- [ ] **Step 3: Build the quiet top strip and lighter rail**

Create `src/screens/library/LibraryTopStrip.tsx` with:

- total shown
- total in library
- active filters
- search field
- `More filters` button

Create `src/screens/library/LibraryFilterRail.tsx` with:

- core filters only:
  - type
  - creator
  - root/profile
  - health/update state
- compact count summary
- reset action
- collapsed behavior

Wire both into `src/screens/LibraryScreen.tsx` and remove the current heavy filter header block from the screen file.

- [ ] **Step 4: Add the advanced-filter popover state in the screen**

In `src/screens/LibraryScreen.tsx`, add state for:

- `isMoreFiltersOpen`

Use it to keep:

- subtype
- confidence
- future narrow filters

out of the default left rail.

- [ ] **Step 5: Run the new rail test**

Run:

```bash
npm run test:unit -- LibraryFilterRail
```

Expected: PASS.

- [ ] **Step 6: Commit the shell narrowing changes**

```bash
git add src/screens/LibraryScreen.tsx src/screens/library/LibraryTopStrip.tsx src/screens/library/LibraryFilterRail.tsx src/screens/library/LibraryFilterRail.test.tsx src/screens/library/libraryDisplay.ts src/styles/globals.css
git commit -m "feat: calm library filter shell"
```

## Task 3: Turn The Center Into The Real Hero

**Files:**
- Create: `src/screens/library/LibraryCollectionTable.tsx`
- Create: `src/screens/library/LibraryCollectionTable.test.tsx`
- Modify: `src/screens/LibraryScreen.tsx`
- Modify: `src/screens/library/libraryDisplay.ts`
- Modify: `src/styles/globals.css`

- [ ] **Step 1: Write the failing render test for the calmer rows**

Create `src/screens/library/LibraryCollectionTable.test.tsx`:

```tsx
import { render, screen } from "@testing-library/react";
import { LibraryCollectionTable } from "./LibraryCollectionTable";

it("shows a calm two-line row in casual mode", () => {
  render(
    <LibraryCollectionTable
      userView="beginner"
      rows={[
        {
          id: 1,
          filename: "BetterBuildBuy.package",
          path: "Mods\\\\BuildBuy\\\\BetterBuildBuy.package",
          creator: "TwistedMexi",
          kind: "Gameplay",
          sourceLocation: "mods",
          relativeDepth: 2,
          confidence: 0.92,
        },
      ]}
      selectedId={1}
      page={0}
      totalPages={1}
      onSelect={() => {}}
      onPrevPage={() => {}}
      onNextPage={() => {}}
    />,
  );

  expect(screen.getByText(/betterbuildbuy\.package/i)).toBeVisible();
  expect(screen.queryByText(/mods\\\\buildbuy/i)).not.toBeInTheDocument();
});
```

- [ ] **Step 2: Run the table test and confirm it fails**

Run:

```bash
npm run test:unit -- LibraryCollectionTable
```

Expected: FAIL because the component does not exist yet.

- [ ] **Step 3: Build the new list-first table component**

Create `src/screens/library/LibraryCollectionTable.tsx` with:

- calmer row titles
- one health marker
- one or two supporting facts
- stable keyboard selection
- quiet empty state
- footer pagination

Move the current table markup out of `src/screens/LibraryScreen.tsx`.

- [ ] **Step 4: Update the screen to use the new list-first center stage**

In `src/screens/LibraryScreen.tsx`:

- replace the current `library-stage-focus` block with a smaller empty-or-selection hint
- keep the list as the widest area
- remove path-first row copy from casual and seasoned rows

- [ ] **Step 5: Run the new table test**

Run:

```bash
npm run test:unit -- LibraryCollectionTable
```

Expected: PASS.

- [ ] **Step 6: Commit the list-first rebuild**

```bash
git add src/screens/LibraryScreen.tsx src/screens/library/LibraryCollectionTable.tsx src/screens/library/LibraryCollectionTable.test.tsx src/screens/library/libraryDisplay.ts src/styles/globals.css
git commit -m "feat: make library list the hero"
```

## Task 4: Replace The Right Wall With A Short Understanding Panel

**Files:**
- Create: `src/screens/library/LibraryDetailsPanel.tsx`
- Create: `src/screens/library/LibraryDetailsPanel.test.tsx`
- Modify: `src/screens/LibraryScreen.tsx`
- Modify: `src/screens/library/libraryDisplay.ts`
- Modify: `src/styles/globals.css`

- [ ] **Step 1: Write the failing render test for the calmer details panel**

Create `src/screens/library/LibraryDetailsPanel.test.tsx`:

```tsx
import { render, screen } from "@testing-library/react";
import { LibraryDetailsPanel } from "./LibraryDetailsPanel";

it("shows snapshot, care, and more instead of inline edit forms", () => {
  render(
    <LibraryDetailsPanel
      userView="standard"
      selectedFile={{
        id: 1,
        filename: "WonderfulWhims.package",
      } as never}
      onOpenHealthDetails={() => {}}
      onOpenInspectFile={() => {}}
      onOpenEditDetails={() => {}}
      onOpenUpdates={() => {}}
    />,
  );

  expect(screen.getByText(/snapshot/i)).toBeVisible();
  expect(screen.getByText(/care/i)).toBeVisible();
  expect(screen.getByRole("button", { name: /inspect file/i })).toBeVisible();
});
```

- [ ] **Step 2: Run the details-panel test and confirm it fails**

Run:

```bash
npm run test:unit -- LibraryDetailsPanel
```

Expected: FAIL because the component does not exist yet.

- [ ] **Step 3: Build the short understanding panel**

Create `src/screens/library/LibraryDetailsPanel.tsx` with:

- `Snapshot`
  - name
  - creator
  - type
  - status
  - update hint
- `Care`
  - short warning/dependency/compatibility summary
  - plain-language next step
- `More`
  - outward actions:
    - `Health details`
    - `Inspect file`
    - `Edit details`
    - `Open in Updates` when available

In `src/screens/LibraryScreen.tsx`, replace the current `DockSectionStack` usage and remove the always-open inline editor blocks from the main inspector.

- [ ] **Step 4: Keep the selection-empty state calm**

When there is no selected file, show:

- one short explanation
- one gentle instruction to select a row

Do not show a second heavy empty card.

- [ ] **Step 5: Run the new details-panel test**

Run:

```bash
npm run test:unit -- LibraryDetailsPanel
```

Expected: PASS.

- [ ] **Step 6: Commit the calmer right panel**

```bash
git add src/screens/LibraryScreen.tsx src/screens/library/LibraryDetailsPanel.tsx src/screens/library/LibraryDetailsPanel.test.tsx src/screens/library/libraryDisplay.ts src/styles/globals.css
git commit -m "feat: simplify library details panel"
```

## Task 5: Move Deep Detail And Editing Behind One Focused Sheet

**Files:**
- Create: `src/screens/library/LibraryDetailSheet.tsx`
- Modify: `src/screens/LibraryScreen.tsx`
- Modify: `src/styles/globals.css`

- [ ] **Step 1: Add sheet state to the screen**

In `src/screens/LibraryScreen.tsx`, add:

- `activeLibrarySheet`
  - `"health" | "inspect" | "edit" | null`

Add open and close handlers tied to the new right-panel buttons.

- [ ] **Step 2: Build the shared Library detail sheet**

Create `src/screens/library/LibraryDetailSheet.tsx` with three modes:

- `health`
  - fuller safety notes
  - parser warnings
  - compatibility or dependency notes
- `inspect`
  - full path
  - hash
  - inside-file clues
  - creator-mode receipts
- `edit`
  - creator save/fix
  - type override
  - related metadata correction

Reuse the existing creator-learning and type-override form logic from `src/screens/LibraryScreen.tsx` instead of rewriting that behavior.

- [ ] **Step 3: Add the calmer sheet transitions and polish**

In `src/styles/globals.css`:

- give the sheet the same quiet slide/fade feel used on `Home`, `Updates`, and `Downloads`
- keep form spacing short and readable
- prevent the sheet from feeling like another giant settings page

- [ ] **Step 4: Verify the main page now stays calmer**

Manually check that:

- the default page no longer shows inline edit forms
- the right panel stays short
- deep detail only appears after button clicks

- [ ] **Step 5: Commit the details-on-demand layer**

```bash
git add src/screens/LibraryScreen.tsx src/screens/library/LibraryDetailSheet.tsx src/styles/globals.css
git commit -m "feat: add library detail sheet"
```

## Task 6: Final Polish, Verification, And Docs

**Files:**
- Modify: `src/screens/LibraryScreen.tsx`
- Modify: `src/styles/globals.css`
- Modify: `SESSION_HANDOFF.md`
- Modify: `docs/IMPLEMENTATION_STATUS.md`

- [ ] **Step 1: Do the visual polish pass**

Tighten:

- row spacing
- status chip weight
- hover and selection feel
- empty states
- top strip balance
- creator-mode detail density

Keep the motion restrained and desktop-like.

- [ ] **Step 2: Capture real UI checks**

Run the app and capture fresh Library screenshots for:

- `Casual`
- `Seasoned`
- `Creator`

Save them under `output/playwright/` with a new `library-pass` prefix.

- [ ] **Step 3: Run the focused tests**

Run:

```bash
npm run test:unit -- libraryDisplay LibraryFilterRail LibraryCollectionTable LibraryDetailsPanel
```

Expected: PASS.

- [ ] **Step 4: Run the full build**

Run:

```bash
npm run build
```

Expected: PASS.

- [ ] **Step 5: Update the repo memory**

Update:

- `SESSION_HANDOFF.md`
- `docs/IMPLEMENTATION_STATUS.md`

Include:

- what changed
- what was visually checked
- what tests passed
- any remaining gaps
- next best step

- [ ] **Step 6: Commit and push the finished Library slice**

```bash
git add src/screens/LibraryScreen.tsx src/screens/library src/styles/globals.css SESSION_HANDOFF.md docs/IMPLEMENTATION_STATUS.md
git commit -m "feat: rework library into quiet catalog"
git push origin codex/library-quiet-catalog-design
```
