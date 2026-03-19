# Library Redesign Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Rebuild `Library` into a top-filter, list-first browser with a steady right sidebar and no unnecessary page scrolling.

**Architecture:** Replace the current left-filter workbench layout with a flatter page shell: one filter strip at the top, a two-column lower area for the list and sidebar, and on-demand detail layers for deeper reading or editing. Keep the current file-loading logic and reuse the row/detail models where they still fit, but move the layout responsibility into smaller Library-specific components.

**Tech Stack:** React, TypeScript, Motion for calm transitions, Vitest + Testing Library, shared app CSS in `src/styles/globals.css`

---

### Task 1: Lock the new top filter strip behavior in tests

**Files:**
- Create: `src/screens/library/LibraryTopStrip.test.tsx`
- Modify: `src/screens/library/LibraryTopStrip.tsx`
- Test: `src/screens/library/LibraryTopStrip.test.tsx`

- [ ] **Step 1: Write the failing test**

Add tests that prove:
- the top strip shows the common filters directly
- the extra filter row appears only when `moreFiltersOpen` is true
- the quick count chips still render

- [ ] **Step 2: Run test to verify it fails**

Run: `npm run test:unit -- LibraryTopStrip`

Expected: FAIL because the current top strip only shows search, counts, and a button row.

- [ ] **Step 3: Write minimal implementation**

Refactor `LibraryTopStrip.tsx` so it owns:
- search
- visible filters
- inline summary chips
- the on-demand extra filter row

Keep its API small and focused around:
- current filter values
- facets
- change handlers
- `moreFiltersOpen`

- [ ] **Step 4: Run test to verify it passes**

Run: `npm run test:unit -- LibraryTopStrip`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/screens/library/LibraryTopStrip.tsx src/screens/library/LibraryTopStrip.test.tsx
git commit -m "test: lock library top filter strip"
```

### Task 2: Replace the left-heavy Library page layout

**Files:**
- Modify: `src/screens/LibraryScreen.tsx`
- Modify: `src/screens/library/LibraryCollectionTable.tsx`
- Modify: `src/screens/library/LibraryDetailsPanel.tsx`
- Delete: `src/screens/library/LibraryFilterRail.tsx`
- Delete: `src/screens/library/LibraryFilterRail.test.tsx`

- [ ] **Step 1: Write the failing test**

Add or update component tests so they prove:
- the list remains the main surface
- the right sidebar still supports quick actions
- no code path depends on the old left filter rail

- [ ] **Step 2: Run test to verify it fails**

Run: `npm run test:unit -- LibraryCollectionTable LibraryDetailsPanel`

Expected: FAIL after the layout contract changes.

- [ ] **Step 3: Write minimal implementation**

Refactor `LibraryScreen.tsx` to use:
- a top filter strip
- a lower two-column content area
- the list in the main column
- the sidebar in the right column

Keep row click behavior stable so users can move through mods while the sidebar stays open.

Remove the left filter rail from the live layout.

- [ ] **Step 4: Run test to verify it passes**

Run: `npm run test:unit -- LibraryCollectionTable LibraryDetailsPanel`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/screens/LibraryScreen.tsx src/screens/library/LibraryCollectionTable.tsx src/screens/library/LibraryDetailsPanel.tsx src/screens/library/LibraryFilterRail.tsx src/screens/library/LibraryFilterRail.test.tsx
git commit -m "feat: rebuild library around list and sidebar"
```

### Task 3: Rework deeper detail layers

**Files:**
- Modify: `src/screens/library/LibraryDetailSheet.tsx`
- Create: `src/screens/library/LibraryEditDialog.tsx`
- Modify: `src/screens/LibraryScreen.tsx`

- [ ] **Step 1: Write the failing test**

Add focused tests that prove:
- browse actions still open deeper health or inspect detail on demand
- edit details opens in a task-focused layer instead of crowding the main sidebar

- [ ] **Step 2: Run test to verify it fails**

Run: `npm run test:unit -- LibraryDetailsPanel`

Expected: FAIL because the current edit action still routes into the older detail-layer behavior.

- [ ] **Step 3: Write minimal implementation**

Keep:
- side sheet for deeper reading

Add:
- modal dialog for edit details

Make sure:
- the sidebar remains summary-first
- deeper layers stay fully reachable inside the viewport

- [ ] **Step 4: Run test to verify it passes**

Run: `npm run test:unit -- LibraryDetailsPanel`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/screens/library/LibraryDetailSheet.tsx src/screens/library/LibraryEditDialog.tsx src/screens/LibraryScreen.tsx
git commit -m "feat: split library detail layers"
```

### Task 4: Tighten the Library styling and fit rules

**Files:**
- Modify: `src/styles/globals.css`

- [ ] **Step 1: Write the failing check**

Document the intended visual checks:
- no page-level scroll on normal desktop size
- only the list and sidebar scroll when needed
- filter strip stays compact
- lower content stays within the window

- [ ] **Step 2: Run current checks to expose the gap**

Run:
- `npm run build`
- screenshot the page at the normal desktop size

Expected: current Library still shows the older left-heavy styling and does not match the new top-strip design.

- [ ] **Step 3: Write minimal implementation**

Update `globals.css` so the Library page uses:
- a compact horizontal filter strip
- a list-first lower layout
- a calmer right sidebar
- no unnecessary nested scrolling
- smooth but restrained row, sidebar, and layer transitions

- [ ] **Step 4: Run checks to verify the redesign holds**

Run:
- `npm run build`
- `npm run test:unit -- LibraryTopStrip LibraryCollectionTable LibraryDetailsPanel libraryDisplay`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/styles/globals.css
git commit -m "style: reshape library into top-strip browser"
```

### Task 5: Visual verification and repo memory

**Files:**
- Modify: `SESSION_HANDOFF.md`
- Modify: `docs/IMPLEMENTATION_STATUS.md`
- Capture: `output/playwright/library-*.png`

- [ ] **Step 1: Run the live verification loop**

Use the local app to verify:
- `Casual`
- `Seasoned`
- `Creator`
- details sidebar
- health sheet
- edit dialog

- [ ] **Step 2: Save screenshots**

Capture the updated Library page and the deeper layers.

- [ ] **Step 3: Update repo memory**

Record:
- what changed
- what was verified
- remaining polish gaps

- [ ] **Step 4: Commit**

```bash
git add SESSION_HANDOFF.md docs/IMPLEMENTATION_STATUS.md output/playwright/library-*.png
git commit -m "docs: record library redesign verification"
```
