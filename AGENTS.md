---
description:
alwaysApply: true
---

# Repo agents

## Guaranteed skills

- `anthropic-frontend-design` is a guaranteed skill for this repository.
  Path: `C:/Users/likwi/.codex/skills/anthropic-frontend-design/SKILL.md`
- For frontend, layout, typography, styling, and UI redesign work, open and follow `anthropic-frontend-design` together with `frontend-design`.
- If the user explicitly names `anthropic-frontend-design`, treat that as an instruction to use it for the turn.

## Frontend direction

- Prefer flat, squared, desktop-oriented interfaces over bubbly web-dashboard styling.
- Keep primary work surfaces sparse: show actions and state first, move guidance into contextual help, drawers, tooltips, or dedicated help views.
- Favor data density, strong hierarchy, restrained motion, and stable hover states.

## Documentation discipline

- Treat `README.md`, `docs/IMPLEMENTATION_STATUS.md`, `docs/ARCHITECTURE.md`, `docs/TRUST_BOUNDARIES_AND_AUTOMATION_READINESS.md`, and `docs/planning/` as the main repo memory.
- At the start of a meaningful session, read `docs/IMPLEMENTATION_STATUS.md` and the main docs that match the task.
- Update `docs/IMPLEMENTATION_STATUS.md` only for durable product state, safety gates, validation lanes, and next-sprint direction.
- Update `docs/ARCHITECTURE.md` only when real app structure, flow ownership, or system behavior changed.
- Do not commit session logs, one-off agent reports, generated desktop screenshots, local index databases, or `.cocoindex_code` artifacts.
- If a real bug is found but not fixed, record it in `docs/IMPLEMENTATION_STATUS.md` with reproduction and recommended next diagnostic.
