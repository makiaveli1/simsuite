# Responsive Composition Phase — 2026-04-07

## Phase goal
Re-open the app-wide layout pass and fix page composition, vertical space use, and large-canvas behavior beyond basic viewport fill.

## Canonical root
- Windows: `C:\Users\likwi\OneDrive\Desktop\PROJS\SimSort\`
- WSL: `/mnt/c/Users/likwi/OneDrive/Desktop/PROJS/SimSort/`

## Session start status
- Repo verified at canonical root and synced to `main`
- Prior viewport contract work already landed
- Prior content-shortfall pass accepted as incomplete, especially for large dead lower regions
- Agent path issue discovered again: `sessions_spawn` from this chat session is stale after gateway restart
- Workaround in use: isolated `cron` agentTurn jobs for real agent participation

## Running findings log
- 2026-04-07: current session reopened to distinguish viewport-fill from deeper composition failures across Home, Library, Updates, Settings, Inbox, and shared workbench pages.
- 2026-04-07: `openclaw health --json` revealed invalid config at `plugins.entries.memory-core.config.dreaming`; removed the invalid block from `~/.openclaw/openclaw.json` to restore CLI/session health.
- 2026-04-07: gateway itself is healthy, but this chat session's `sessions_spawn` client remains stale after restart; using cron-isolated agent jobs as the reliable path for Ariadne / Scout / Sentinel / Forge participation.

## Shared pattern hypotheses (to confirm or reject during audit)
- Shared-shell pages may be technically full-height while still under-composed because inner content stacks do not own vertical distribution.
- Large canvases likely need stronger max-width shells for readable content pages and stronger bottom-zone ownership for data/workbench pages.
- Workbench pages likely need explicit lower-half composition, not just a scroll region plus a footer.

## To update before finish
- Page-by-page diagnosis
- Root causes per page
- Shared composition system decisions
- Regressions fixed
- Viewport / responsiveness decisions
- Known limitations
- Commit hashes tied to meaningful changes

## Findings after before-state capture
- 35 before-state screenshots captured across 7 viewports × 5 pages via `scripts/layout-audit-capture.mjs`.
- The viewport contract was not the remaining problem. The shared shells already filled the viewport.
- The deeper issue was **inner composition ownership**:
  - `Settings`: `.screen-shell.settings-screen` did not own an `auto + 1fr` row split, so `settings-layout` only sized to content.
  - `Home`: shell filled the viewport, but the module stack did not distribute vertical space on large canvases and the content width shell remained too conservative on 4K.
  - `Updates`: `.updates-stage-body` was not flexing, so the intended `1fr` middle region collapsed and the table barely owned any height.
  - `Library`, `Review`, `Duplicates`, `Organize`, `Downloads`: persisted UI preferences were re-applying fixed pixel heights (`510`, `520`, `320`, etc.) into CSS variables, overriding responsive defaults and reintroducing dead lower space.
- The first attempted fix in `globals.css` was insufficient because `UiPreferencesContext` was writing fixed pixel values directly back into CSS variables on load.

## Shared pattern decisions (implemented)
- Use persisted size preferences as **minimums**, not hard caps, by writing responsive CSS expressions like `max(savedPx, min(vh-based target, ceilingPx))` in `UiPreferencesContext`.
- Non-workbench full-height pages need their own explicit `auto + 1fr` row contract, not just `height: 100%`.
- Workbench content regions need explicit flex/grid ownership (`flex: 1 1 auto`, `minmax(0, 1fr)`) at the level that actually contains the scrollable body.
- Large-canvas home screens need a wider content shell and internal height distribution, not just the same narrow centered island on a taller screen.
- Inspector/actions rails feel more premium when actions anchor to the bottom rather than floating in a short top stack.

## Implemented changes (current session)
- `src/components/UiPreferencesContext.tsx`
  - Responsive height expressions now applied for library/review/duplicates/downloads/organize heights instead of fixed saved px caps.
- `src/styles/globals.css`
  - `screen-shell.settings-screen` now owns `auto + 1fr` rows.
  - `settings-layout` now claims height properly and uses stronger large-canvas columns.
  - `settings-focus-panel` now owns an internal `auto + 1fr` structure and scrolls correctly.
  - `home-hub-shell` widened and given a stronger hero/module split.
  - `home-module-stack` now stretches and beginner/casual bands can actually occupy available height.
  - `updates-stage-body` now flexes to own the middle of the page.
  - shared queue/table height defaults changed from fixed px to responsive clamps.
  - `library-details-panel` and actions now own height better.
  - 4K-only home width rule added.
  - narrow-width Updates footer rule added.
- `src/screens/SettingsScreen.tsx`
  - settings info trigger patched away from nested interactive markup.

## Verification notes
- After the responsive preference-layer fix, the measured composition changed materially:
  - Library list shell on 4K: ~510px → ~1469px.
  - Updates table region on 4K: ~92px → ~1708px.
  - Settings layout on 4K: ~1396px → ~2021px.
- Final screenshots are in `output/layout-audit/`.

## Known limitations / follow-up
- The browser tool path was unreliable during gateway restarts, so stable screenshot generation was moved to Playwright inside the canonical project.
- The existing desktop Selenium audit script is stale against current selectors and needs updating before it can be trusted again.
- Cron agent delivery still tries to announce through the wrong route, so agent summaries are being recovered via cron run history rather than message delivery.

## Post-fix verification snapshot
- Capture script rerun after fixes on a fresh preview server.
- Final measured composition highlights:
  - Library list shell ratio vs workbench height:
    - 1920×1080: ~0.71
    - 2560×1440: ~0.70
    - 3840×2160: ~0.69
  - Updates table region ratio vs stage height:
    - 1920×1080: ~0.61
    - 2560×1440: ~0.71
    - 3840×2160: ~0.81
  - Settings layout ratio vs available shell height:
    - 1920×1080: ~0.91
    - 2560×1440: ~0.94
    - 3840×2160: ~0.96
- Before this pass, Library and Updates were still carrying laptop-sized hard caps into 1440p/4K because persisted UI preference values overwrote responsive defaults.
- After this pass, those persisted heights behave like viewport-aware minimums instead of fixed caps.

## Final decisions from this phase
- Keep Home wider on very large canvases, but still centered and readable rather than edge-to-edge.
- Let data/workbench pages scale vertically much more aggressively than content pages.
- Treat persisted panel sizes as *minimum user preference*, not *absolute lock*.
- Treat non-workbench pages (like Settings) as first-class full-height layouts with explicit `auto + 1fr` composition ownership.
- Use Playwright capture inside the canonical project as the stable visual-audit path when browser-session state is unreliable.
