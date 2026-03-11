
# Sims Mod Suite — PRD Part 6: Codex Implementation Blueprint

## Status
Draft v1

## Purpose
This document converts the previous PRD documents into a **practical build plan**
that Codex or any engineering team can follow.

It defines:

• repository structure  
• module boundaries  
• database schema outline  
• service interfaces  
• development milestones  
• build order  

This is the **engineering execution guide**.

---

# Current build note (March 11, 2026)

Before starting new work, treat these as already materially in place:

- real Downloads Inbox with queue, selected-item panel, special setup,
  blocked states, and apply flows
- built-in special-mod catalog with guided support for the first curated
  wave
- snapshot-backed guided apply and rollback paths
- local file inspection for `.ts4script` and `.package` content clues
- local-first special-mod version comparison
- targeted workspace refresh events instead of only broad full-screen
  refreshes

Do not restart these systems from scratch.

Current engineering rule for Codex:

1. check the current implementation first
2. trace the real desktop-backed flow
3. measure before optimizing
4. avoid assumptions when the app state or files can be inspected
5. preserve the separation between:
   - Downloads as intake
   - Organize as safe library sorting
   - Review as unresolved hold work
   - Library and audit screens as learning inputs

---

# 1. Repository Structure

Recommended repository layout:

```
sims-mod-suite/
│
├─ apps/
│   └─ desktop/
│       ├─ src/
│       ├─ components/
│       ├─ screens/
│       └─ styles/
│
├─ core/
│   ├─ scanner/
│   ├─ rule_engine/
│   ├─ validator/
│   ├─ bundle_detector/
│   ├─ duplicate_detector/
│   ├─ downloads_watcher/
│   ├─ ai_classifier/
│   └─ snapshot_manager/
│
├─ database/
│   ├─ schema/
│   └─ migrations/
│
├─ services/
│   ├─ filesystem/
│   ├─ archive/
│   ├─ logging/
│   └─ config/
│
├─ models/
│   ├─ classification/
│   └─ prompts/
│
├─ docs/
│
└─ scripts/
```

---

# 2. Core Modules

Each module must have clear responsibilities.

## scanner

Responsibilities:

• scan Mods folder  
• scan Tray folder  
• detect new files  
• extract metadata  
• send results to database  

## rule_engine

Responsibilities:

• calculate suggested file paths  
• apply user rules  
• apply detected structure rules  

## validator

Responsibilities:

• enforce safe script depth  
• enforce tray placement  
• prevent folder collisions  

## bundle_detector

Responsibilities:

• detect tray bundles  
• detect mod sets  
• assign bundle_id  

## duplicate_detector

Responsibilities:

• detect identical files  
• detect version duplicates  

## ai_classifier

Responsibilities:

• classify unknown content  
• infer creators  
• generate structured metadata  

## downloads_watcher

Responsibilities:

• monitor downloads folder  
• detect archives  
• trigger intake pipeline  

---

# 3. Database Schema (Initial Outline)

Tables:

files
bundles
rules
creators
aliases
duplicates
review_queue
snapshots
scan_sessions

Example files table:

```
files
-----
id
path
filename
extension
hash
size
created_at
modified_at
bundle_id
classification_id
```

Example bundles table:

```
bundles
-------
id
bundle_type
bundle_name
file_count
confidence
```

---

# 4. AI Classification Interface

AI should expose a simple interface.

Example service:

```
classify_file(metadata) -> classification_json
```

Metadata example:

```
{
  "filename": "breezyhair.package",
  "path": "downloads/",
  "nearby_files": ["breezytop.package"]
}
```

Output:

```
{
  "kind": "CAS",
  "subtype": "Hair",
  "creator": "Unknown",
  "confidence": 0.82
}
```

---

# 5. File Processing Pipeline

Every file follows the same pipeline.

Step 1
File detected

Step 2
Scanner extracts metadata

Step 3
Bundle detector groups files

Step 4
Rule engine generates destination

Step 5
Validator checks safety

Step 6
Preview created

Step 7
User approval

Step 8
Move engine executes action

---

# 6. Move Engine

The move engine handles file operations safely.

Functions:

move_file()
move_bundle()
rollback_move()
apply_batch_moves()

All operations must log changes for undo.

---

# 7. Snapshot System

Snapshots allow full rollback.

Snapshot record includes:

• original paths  
• timestamps  
• hashes  

Snapshots are created before:

• migrations  
• bulk operations  
• patch recovery  

---

# 8. Background Workers

Workers handle asynchronous tasks.

Worker types:

scanner_worker
classification_worker
duplicate_worker
downloads_worker

Workers should use task queues.

---

# 9. Development Milestones

## Milestone 1 — Scanner Foundation

Deliver:

• Mods folder scanning  
• Tray folder scanning  
• metadata extraction  
• database storage  

## Milestone 2 — Library Viewer

Deliver:

• Library screen  
• basic filtering  
• file metadata display  

## Milestone 3 — Rule Engine

Deliver:

• organization presets  
• mirror mode detection  
• preview suggestions  

## Milestone 4 — Safe Move System

Deliver:

• validator rules  
• move engine  
• snapshot system  

## Milestone 5 — Duplicate Detection

Deliver:

• duplicate scanner  
• duplicate management screen  

## Milestone 6 — Downloads Watcher

Deliver:

• downloads monitoring  
• archive extraction  
• intake pipeline  

## Milestone 7 — AI Classification

Deliver:

• AI service interface  
• JSON classification schema  
• review queue integration  

## Milestone 8 — Patch Recovery

Deliver:

• snapshot comparison  
• creator grouping  
• mod isolation tools  

---

# 10. Development Order

Recommended order:

1 scanner  
2 database schema  
3 library viewer  
4 rule engine  
5 validator  
6 move engine  
7 snapshot system  
8 duplicate detection  
9 downloads watcher  
10 AI classifier  
11 patch recovery  

---

# 11. Testing Strategy

Test types:

unit tests
integration tests
filesystem simulation tests

Critical tests:

• script depth protection  
• bundle integrity  
• undo system reliability  

---

# 12. Performance Targets

Library size support:

10,000+ mod files  
500+ tray bundles  

Scan time target:

under 2 minutes for full scan.

---

# 13. Release Plan

Version 1.0

• scanning
• library viewer
• rule engine
• safe moves
• duplicate detection
• downloads watcher

Version 1.1

• AI classification
• tray bundle tools
• improved rule learning

Version 1.2

• patch recovery
• creator dashboards
• advanced automation options

---

# 14. Final Notes

This blueprint is designed so Codex can begin implementation
without needing to reinterpret the earlier PRD documents.

All safety systems must be implemented before any automatic
file operations are enabled.
