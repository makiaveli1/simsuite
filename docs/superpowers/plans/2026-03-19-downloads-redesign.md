# Downloads Quiet Staging Desk Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Rebuild the `Downloads` screen into the approved quiet staging desk so new mod batches feel calmer, more focused, and easier to work through in `Casual`, `Seasoned`, and `Creator`.

**Architecture:** Keep `src/screens/DownloadsScreen.tsx` as the orchestration root for data loading, selection, and actions, but move display logic and the main visual sections into focused files under `src/screens/downloads/`. Use one pure display helper module for lane choice, row shaping, and view-mode gating so the new behavior can be tested before the UI is rearranged. Reuse the existing workbench layout and sheet styling patterns instead of inventing a second design system for this page.

**Tech Stack:** React 19, TypeScript, Vite, Motion, Tauri APIs, global CSS, Vitest, Testing Library

---

## Preflight

- If you are still on `main`, create a dedicated branch first:

```bash
git checkout -b codex/downloads-quiet-staging-desk
```

- Keep unrelated dirty worktree changes out of every commit in this plan.
- Update `SESSION_HANDOFF.md` and `docs/IMPLEMENTATION_STATUS.md` after the final verification task.

## File Map

### New files

- `vitest.config.ts`
  - Vitest config for lightweight frontend unit and render tests
- `vitest.setup.ts`
  - Testing Library matchers and shared test setup
- `src/screens/downloads/downloadsDisplay.ts`
  - Pure helpers for lane priority, lane fallback, queue row view models, action priority, and per-view visibility rules
- `src/screens/downloads/downloadsDisplay.test.ts`
  - Unit tests for the pure Downloads display rules
- `src/screens/downloads/DownloadsTopStrip.tsx`
  - Slim strip with totals, last check, and refresh action
- `src/screens/downloads/DownloadsRail.tsx`
  - Watch status, lane switch, search, and optional extra filters
- `src/screens/downloads/DownloadsQueuePanel.tsx`
  - Queue list for the active lane with calmer rows
- `src/screens/downloads/DownloadsBatchCanvas.tsx`
  - Lane-aware main stage for ready, setup, waiting, blocked, and done states
- `src/screens/downloads/DownloadsDecisionPanel.tsx`
  - Short right panel with the one main action and compact summary
- `src/screens/downloads/DownloadsDecisionPanel.test.tsx`
  - Render tests for action priority and proof access
- `src/screens/downloads/DownloadsProofSheet.tsx`
  - Right-side sheet for proof, versions, source details, and full file list
- `src/screens/downloads/DownloadsSetupDialog.tsx`
  - Focused dialog for guided setup and install confirmation
- `src/screens/downloads/DownloadsRail.test.tsx`
  - Render tests for lane picker and view-mode-specific filtering
- `src/screens/downloads/DownloadsBatchCanvas.test.tsx`
  - Render tests for lane-aware center-stage behavior

### Modified files

- `package.json`
  - Add `test:unit` script and test dev dependencies
- `package-lock.json`
  - Lockfile update for new test tooling
- `src/screens/DownloadsScreen.tsx`
  - Shrink into a screen orchestrator that wires the new components together
- `src/lib/motion.ts`
  - Add calmer Downloads-specific timing helpers if the existing motion tokens are not enough
- `src/styles/globals.css`
  - Replace the current Downloads layout rules with the new quiet staging desk styling
- `SESSION_HANDOFF.md`
  - Record implementation work, verification, open gaps, and next step
- `docs/IMPLEMENTATION_STATUS.md`
  - Record the Downloads redesign progress and checks

### Reference files

- `docs/superpowers/specs/2026-03-19-downloads-redesign-design.md`
  - Approved Downloads spec
- `src/screens/HomeScreen.tsx`
  - Existing workbench sheet pattern
- `src/screens/UpdatesScreen.tsx`
  - Existing right-side sheet pattern
- `src/lib/uiLanguage.ts`
  - Existing view-aware language conventions

## Task 1: Add A Small Test Seam For Downloads Rules

**Files:**
- Create: `vitest.config.ts`
- Create: `vitest.setup.ts`
- Create: `src/screens/downloads/downloadsDisplay.ts`
- Create: `src/screens/downloads/downloadsDisplay.test.ts`
- Modify: `package.json`
- Modify: `package-lock.json`

- [ ] **Step 1: Install the lightweight test tooling and add a script**

Run:

```bash
npm install -D vitest jsdom @testing-library/react @testing-library/jest-dom
```

Update `package.json`:

```json
{
  "scripts": {
    "test:unit": "vitest run"
  }
}
```

- [ ] **Step 2: Add the test config and setup files**

Create `vitest.config.ts`:

```ts
import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  test: {
    environment: "jsdom",
    setupFiles: ["./vitest.setup.ts"],
    include: ["src/**/*.test.ts", "src/**/*.test.tsx"],
  },
});
```

Create `vitest.setup.ts`:

```ts
import "@testing-library/jest-dom/vitest";
```

- [ ] **Step 3: Write failing unit tests for the new pure Downloads rules**

Create `src/screens/downloads/downloadsDisplay.test.ts`:

```ts
import { describe, expect, it } from "vitest";
import {
  capRowBadges,
  fallbackDownloadsLane,
  pickInitialDownloadsLane,
  viewModeDownloadsFlags,
} from "./downloadsDisplay";

describe("pickInitialDownloadsLane", () => {
  it("prefers waiting_on_you over ready_now", () => {
    expect(
      pickInitialDownloadsLane({
        ready_now: 4,
        special_setup: 0,
        waiting_on_you: 1,
        blocked: 0,
        done: 0,
      }),
    ).toBe("waiting_on_you");
  });
});

describe("fallbackDownloadsLane", () => {
  it("moves to the next useful non-empty lane when the preferred lane empties", () => {
    expect(
      fallbackDownloadsLane("special_setup", {
        ready_now: 2,
        special_setup: 0,
        waiting_on_you: 0,
        blocked: 1,
        done: 0,
      }),
    ).toBe("ready_now");
  });
});

describe("capRowBadges", () => {
  it("never returns more than two visible badges", () => {
    expect(capRowBadges(["Ready", "Newer", "Supported"])).toEqual(["Ready", "Newer"]);
  });
});

describe("viewModeDownloadsFlags", () => {
  it("keeps advanced filters hidden in casual mode", () => {
    expect(viewModeDownloadsFlags("beginner").showAdvancedFiltersByDefault).toBe(false);
  });
});
```

- [ ] **Step 4: Run the tests to verify they fail first**

Run:

```bash
npm run test:unit -- downloadsDisplay
```

Expected: FAIL with missing exports or missing module errors.

- [ ] **Step 5: Implement the minimal pure helpers**

Create `src/screens/downloads/downloadsDisplay.ts` with focused helpers like:

```ts
import type { DownloadQueueLane, UserView } from "../../lib/types";

const LANE_PRIORITY: DownloadQueueLane[] = [
  "waiting_on_you",
  "special_setup",
  "ready_now",
  "blocked",
  "done",
];

export function pickInitialDownloadsLane(counts: Record<DownloadQueueLane, number>) {
  return LANE_PRIORITY.find((lane) => counts[lane] > 0) ?? "ready_now";
}

export function fallbackDownloadsLane(
  preferred: DownloadQueueLane,
  counts: Record<DownloadQueueLane, number>,
) {
  if (counts[preferred] > 0) {
    return preferred;
  }

  return LANE_PRIORITY.find((lane) => counts[lane] > 0) ?? preferred;
}

export function capRowBadges(labels: string[]) {
  return labels.slice(0, 2);
}

export function viewModeDownloadsFlags(userView: UserView) {
  return {
    showAdvancedFiltersByDefault: userView === "power",
    showCompactPreview: userView !== "beginner",
    showExtraProofBlock: userView === "power",
  };
}
```

- [ ] **Step 6: Run the unit tests again**

Run:

```bash
npm run test:unit -- downloadsDisplay
```

Expected: PASS with all Downloads display tests green.

- [ ] **Step 7: Commit the test seam**

```bash
git add package.json package-lock.json vitest.config.ts vitest.setup.ts src/screens/downloads/downloadsDisplay.ts src/screens/downloads/downloadsDisplay.test.ts
git commit -m "test: add downloads display rules"
```

## Task 2: Split The Current Screen Shell And Lane Controls

**Files:**
- Create: `src/screens/downloads/DownloadsTopStrip.tsx`
- Create: `src/screens/downloads/DownloadsRail.tsx`
- Create: `src/screens/downloads/DownloadsRail.test.tsx`
- Modify: `src/screens/DownloadsScreen.tsx`
- Modify: `src/screens/downloads/downloadsDisplay.ts`
- Modify: `src/styles/globals.css`

- [ ] **Step 1: Write a failing render test for the left rail behavior**

Create `src/screens/downloads/DownloadsRail.test.tsx`:

```tsx
import { render, screen } from "@testing-library/react";
import { DownloadsRail } from "./DownloadsRail";

it("shows compact lane counts and hides advanced filters in casual mode", () => {
  render(
    <DownloadsRail
      userView="beginner"
      watcherLabel="Watching"
      watchedPath="C:\\Users\\Player\\Downloads"
      lastCheckLabel="Last check 3/8/2026, 4:12 AM"
      activeLane="ready_now"
      laneCounts={{
        ready_now: 1,
        special_setup: 2,
        waiting_on_you: 1,
        blocked: 3,
        done: 0,
      }}
      search=""
      statusFilter=""
      selectedPreset="Category First"
      onLaneChange={() => {}}
      onSearchChange={() => {}}
      onOpenFilters={() => {}}
      onRefresh={() => {}}
    />,
  );

  expect(screen.getByRole("button", { name: /ready now/i })).toBeVisible();
  expect(screen.queryByText(/more filters/i)).not.toBeInTheDocument();
});
```

- [ ] **Step 2: Run the rail test and confirm it fails**

Run:

```bash
npm run test:unit -- DownloadsRail
```

Expected: FAIL because `DownloadsRail` does not exist yet.

- [ ] **Step 3: Build the new top strip and rail components**

Create `src/screens/downloads/DownloadsTopStrip.tsx` with:
- totals
- waiting count
- blocked count
- last check
- refresh action

Create `src/screens/downloads/DownloadsRail.tsx` with:
- watch folder block
- compact lane picker
- search
- small "More filters" trigger

Keep the props focused. The rail should not fetch data or own business rules.

- [ ] **Step 4: Wire the new shell into `DownloadsScreen.tsx`**

Update `src/screens/DownloadsScreen.tsx` so it now owns:
- data loading
- selected lane state
- remembered lane fallback behavior
- selection state
- action handlers

But it should stop rendering the left rail markup inline. Replace that markup with:

```tsx
<DownloadsTopStrip ... />
<Workbench>
  <WorkbenchRail ...>
    <DownloadsRail ... />
  </WorkbenchRail>
  ...
</Workbench>
```

- [ ] **Step 5: Update the Downloads shell styles**

Move the left side away from stacked heavy cards:
- compact lane buttons
- calmer watch block
- smaller filter footprint
- quieter visual weight than the center

Modify `src/styles/globals.css` instead of adding a second CSS file.

- [ ] **Step 6: Run the targeted rail test**

Run:

```bash
npm run test:unit -- DownloadsRail
```

Expected: PASS.

- [ ] **Step 7: Run the full build**

Run:

```bash
npm run build
```

Expected: PASS.

- [ ] **Step 8: Commit the shell split**

```bash
git add src/screens/DownloadsScreen.tsx src/screens/downloads/DownloadsTopStrip.tsx src/screens/downloads/DownloadsRail.tsx src/screens/downloads/DownloadsRail.test.tsx src/screens/downloads/downloadsDisplay.ts src/styles/globals.css
git commit -m "refactor: split downloads shell"
```

## Task 3: Rebuild The Queue And Lane-Aware Batch Canvas

**Files:**
- Create: `src/screens/downloads/DownloadsQueuePanel.tsx`
- Create: `src/screens/downloads/DownloadsBatchCanvas.tsx`
- Create: `src/screens/downloads/DownloadsBatchCanvas.test.tsx`
- Modify: `src/screens/DownloadsScreen.tsx`
- Modify: `src/screens/downloads/downloadsDisplay.ts`
- Modify: `src/styles/globals.css`

- [ ] **Step 1: Write failing tests for the lane-aware center stage**

Create `src/screens/downloads/DownloadsBatchCanvas.test.tsx`:

```tsx
import { render, screen } from "@testing-library/react";
import { DownloadsBatchCanvas } from "./DownloadsBatchCanvas";

it("shows the waiting-on-you decision story for that lane", () => {
  render(
    <DownloadsBatchCanvas
      lane="waiting_on_you"
      userView="standard"
      selectionTitle="Adeepindigo_Healthcare_Redux_Addon.zip"
      summary="This batch needs one more choice from you before it can move."
      safeCount={0}
      reviewCount={1}
      unchangedCount={0}
      previewItems={[]}
    />,
  );

  expect(screen.getByText(/needs one more choice/i)).toBeVisible();
});
```

- [ ] **Step 2: Run the batch-canvas test and confirm it fails**

Run:

```bash
npm run test:unit -- DownloadsBatchCanvas
```

Expected: FAIL because `DownloadsBatchCanvas` does not exist yet.

- [ ] **Step 3: Create the queue panel and calmer row model**

Create `src/screens/downloads/DownloadsQueuePanel.tsx`:
- render only the active lane
- keep one strong selected row
- use one short reason line
- cap visible badges at two by using `capRowBadges`

Use `downloadsDisplay.ts` to build row view models instead of re-deriving row labels inside JSX.

- [ ] **Step 4: Create the lane-aware batch canvas**

Create `src/screens/downloads/DownloadsBatchCanvas.tsx`:
- `ready_now` shows safe hand-off
- `special_setup` shows short setup story
- `waiting_on_you` shows the missing decision
- `blocked` shows the stop reason
- `done` shows the completion story

Keep this component presentation-only. It should receive already-shaped values from the screen or helper module.

- [ ] **Step 5: Replace the current queue + preview area in `DownloadsScreen.tsx`**

Replace the current middle-stage inline blocks with:

```tsx
<DownloadsQueuePanel ... />
<DownloadsBatchCanvas ... />
```

Keep the old data-loading and action handlers alive, but stop letting the screen inline-render every queue row and preview card.

- [ ] **Step 6: Update the center-stage styling**

Modify `src/styles/globals.css` so the center stage:
- gets more breathing room than the rail
- reads as the visual anchor
- uses calmer rows
- avoids large empty black gaps when the queue is short

- [ ] **Step 7: Run the batch-canvas test**

Run:

```bash
npm run test:unit -- DownloadsBatchCanvas
```

Expected: PASS.

- [ ] **Step 8: Run the full test and build pass**

Run:

```bash
npm run test:unit
npm run build
```

Expected: PASS for both.

- [ ] **Step 9: Commit the center-stage rebuild**

```bash
git add src/screens/DownloadsScreen.tsx src/screens/downloads/DownloadsQueuePanel.tsx src/screens/downloads/DownloadsBatchCanvas.tsx src/screens/downloads/DownloadsBatchCanvas.test.tsx src/screens/downloads/downloadsDisplay.ts src/styles/globals.css
git commit -m "feat: rebuild downloads queue stage"
```

## Task 4: Add The Short Decision Panel And Details-On-Demand Layers

**Files:**
- Create: `src/screens/downloads/DownloadsDecisionPanel.tsx`
- Create: `src/screens/downloads/DownloadsDecisionPanel.test.tsx`
- Create: `src/screens/downloads/DownloadsProofSheet.tsx`
- Create: `src/screens/downloads/DownloadsSetupDialog.tsx`
- Modify: `src/screens/DownloadsScreen.tsx`
- Modify: `src/styles/globals.css`
- Modify: `src/lib/motion.ts`

- [ ] **Step 1: Write a failing render test for the right-side action priority**

Create `src/screens/downloads/DownloadsDecisionPanel.test.tsx`:

```tsx
import { render, screen } from "@testing-library/react";
import { DownloadsDecisionPanel } from "./DownloadsDecisionPanel";

it("shows one primary action and keeps proof behind a secondary action", () => {
  render(
    <DownloadsDecisionPanel
      title="SpringRefreshPack.zip"
      statusLabel="Ready now"
      summary="No safe hand-off is ready yet."
      primaryActionLabel="Ignore"
      onPrimaryAction={() => {}}
      onOpenProof={() => {}}
    />,
  );

  expect(screen.getByRole("button", { name: /ignore/i })).toBeVisible();
  expect(screen.getByRole("button", { name: /open proof/i })).toBeVisible();
});
```

- [ ] **Step 2: Run the targeted test and confirm it fails**

Run:

```bash
npm run test:unit -- DownloadsDecisionPanel
```

Expected: FAIL because the component does not exist yet.

- [ ] **Step 3: Create the short decision panel**

Create `src/screens/downloads/DownloadsDecisionPanel.tsx` with:
- selected batch title
- status label
- one primary action
- one or two secondary actions
- compact summary block

Do not mount the old full receipt stack in this panel.

- [ ] **Step 4: Create the proof sheet**

Create `src/screens/downloads/DownloadsProofSheet.tsx` using the existing `workbench-sheet` markup pattern already used by `Home` and `Updates`.

The sheet should hold:
- proof
- versions
- source details
- full file list
- full "why blocked" story

- [ ] **Step 5: Create the setup dialog**

Create `src/screens/downloads/DownloadsSetupDialog.tsx` for:
- guided setup flow
- install confirmation
- bigger irreversible choices

Use a focused centered dialog instead of a full-page panel.

- [ ] **Step 6: Wire the new panel, sheet, and dialog into the screen**

`DownloadsScreen.tsx` should own only the open/close state:

```tsx
const [proofSheetOpen, setProofSheetOpen] = useState(false);
const [setupDialogOpen, setSetupDialogOpen] = useState(false);
```

Then render:

```tsx
<DownloadsDecisionPanel ... />
{proofSheetOpen ? <DownloadsProofSheet ... /> : null}
{setupDialogOpen ? <DownloadsSetupDialog ... /> : null}
```

- [ ] **Step 7: Add the calmer motion tokens**

If the current motion tokens are not enough, extend `src/lib/motion.ts` with a small Downloads-specific set such as:

```ts
export const downloadsSelectionTransition = {
  duration: 0.18,
  ease: easeStandard,
};
```

Keep the new motion within the spec range:
- row and lane updates roughly `160ms` to `220ms`
- sheets and dialogs roughly `180ms` to `240ms`

- [ ] **Step 8: Run tests and build**

Run:

```bash
npm run test:unit
npm run build
```

Expected: PASS for both.

- [ ] **Step 9: Commit the details-on-demand layer**

```bash
git add src/screens/DownloadsScreen.tsx src/screens/downloads/DownloadsDecisionPanel.tsx src/screens/downloads/DownloadsDecisionPanel.test.tsx src/screens/downloads/DownloadsProofSheet.tsx src/screens/downloads/DownloadsSetupDialog.tsx src/styles/globals.css src/lib/motion.ts
git commit -m "feat: add downloads detail layers"
```

## Task 5: Polish Motion, Empty States, And Cross-View Behavior

**Files:**
- Modify: `src/screens/DownloadsScreen.tsx`
- Modify: `src/screens/downloads/downloadsDisplay.ts`
- Modify: `src/screens/downloads/DownloadsRail.tsx`
- Modify: `src/screens/downloads/DownloadsQueuePanel.tsx`
- Modify: `src/screens/downloads/DownloadsBatchCanvas.tsx`
- Modify: `src/screens/downloads/DownloadsDecisionPanel.tsx`
- Modify: `src/styles/globals.css`
- Modify: `SESSION_HANDOFF.md`
- Modify: `docs/IMPLEMENTATION_STATUS.md`

- [ ] **Step 1: Tighten the empty and small states**

Make sure:
- empty lane views do not leave dead boxes
- empty inbox has a centered calm state
- short queues still feel balanced

- [ ] **Step 2: Verify the three user views feel meaningfully different**

Check and adjust:
- `Casual` keeps advanced filters and deep proof out of the way
- `Seasoned` keeps the balanced preview visible
- `Creator` gets faster access to proof without turning the page back into a wall of receipts

- [ ] **Step 3: Tune the final motion and hover behavior**

Ensure:
- row selection is smooth but not floaty
- lane changes are quick and readable
- sheets and dialogs settle softly
- reduced-motion mode still feels stable

- [ ] **Step 4: Run the automated checks**

Run:

```bash
npm run test:unit
npm run build
```

Expected: PASS for both.

- [ ] **Step 5: Run live visual checks**

Start the dev server if needed:

```bash
npm run dev -- --host 127.0.0.1 --port 1420
```

Check `Downloads` in:
- `Casual`
- `Seasoned`
- `Creator`

Capture fresh screenshots for at least:
- mixed-lane queue
- empty lane
- active guided setup state if fixture data allows it

If the live fixture data does not expose guided setup during this pass:
- verify the dialog with component tests and code review
- note the missing live state clearly in both repo memory docs

- [ ] **Step 6: Update repo memory**

Update:
- `SESSION_HANDOFF.md`
- `docs/IMPLEMENTATION_STATUS.md`

Record:
- tests run
- visual checks
- what changed
- open gaps
- next best page to redesign

- [ ] **Step 7: Commit the final polish**

```bash
git add src/screens/DownloadsScreen.tsx src/screens/downloads src/styles/globals.css src/lib/motion.ts SESSION_HANDOFF.md docs/IMPLEMENTATION_STATUS.md
git commit -m "refactor: polish downloads workspace"
```

## Plan Review Notes

Manual review checklist to run before implementation starts:
- no TODO or placeholder text left in the plan
- every spec requirement is covered by at least one task
- no task requires guessing a missing file path
- test steps use commands that can actually run in this repo
- commits stay scoped so unrelated dirty files are not swept in

## Completion Gate

Do not call the Downloads redesign done until all of these are true:
- the new Downloads layout is live
- `Casual`, `Seasoned`, and `Creator` all feel different in the intended way
- proof and guided setup are off the main page and open on demand
- `npm run test:unit` passes
- `npm run build` passes
- repo memory docs are updated
