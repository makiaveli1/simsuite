---
description: 
alwaysApply: true
---

# Repo Agents

## Guaranteed Skills

- `anthropic-frontend-design` is a guaranteed skill for this repository.
  Path: `C:/Users/likwi/.codex/skills/anthropic-frontend-design/SKILL.md`
- For frontend, layout, typography, styling, and UI redesign work, open and follow `anthropic-frontend-design` together with `frontend-design`.
- If the user explicitly names `anthropic-frontend-design`, treat that as an instruction to use it for the turn.

## Frontend Direction

- Prefer flat, squared, desktop-oriented interfaces over bubbly web-dashboard styling.
- Keep primary work surfaces sparse: show actions and state first, move guidance into contextual help, drawers, tooltips, or dedicated help views.
- Favor data density, strong hierarchy, restrained motion, and stable hover states.

## Session Memory

- Treat the local repo docs as the main memory between sessions.
- At the start of a meaningful session, read `SESSION_HANDOFF.md` first, then the main docs that match the task.
- At the end of every meaningful session, update the repo memory docs before finishing.
- Put session-by-session baton-pass notes in `SESSION_HANDOFF.md`.
- Put broader progress updates in `docs/IMPLEMENTATION_STATUS.md`.
- Update `docs/ARCHITECTURE.md` only when real app structure, flow ownership, or system behavior changed.
- Keep `docs/Extensive Index of Sims 4 Mods & Resources.md` as reference material only, not as the session log.
- Record important tests run, real desktop checks, findings, open gaps, and the next best step in simple language.
- If a real bug is found but not fixed, record it in both `SESSION_HANDOFF.md` and `docs/IMPLEMENTATION_STATUS.md`.
