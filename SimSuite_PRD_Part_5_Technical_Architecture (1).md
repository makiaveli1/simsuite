# Sims Mod Suite --- PRD Part 5: Technical Architecture

## Status

Draft v1

## Purpose

This document defines the technical architecture of the Sims Mod Suite.
It explains how the application should be built internally so that the
features described in previous PRD documents can be implemented safely
and efficiently.

This document is written for engineers and Codex to use as the primary
technical blueprint.

------------------------------------------------------------------------

# Current engineering note (March 11, 2026)

The current app already has a real Rust-backed Downloads Inbox and a
real guided special-mod system. The next engineering focus is not
“invent Inbox” but “stabilize and simplify the Inbox work path.”

Important current truths:

- Downloads is the intake desk
- Organize is the safe sorter for wider library cleanup
- Library and audit screens feed learned data back into future scans
- special-mod updates are now local-first, not website-first
- internal file inspection is already part of special-mod matching and
  version evidence
- official latest checks are helper-only
- community websites are advisory only and are not allowed to decide
  installs automatically

Important current weakness:

- Inbox still does too much repeated work in real desktop use and can
  feel slow or hang during first open, selection, and special-mod detail
  loading

The next engineering work should therefore measure and remove repeated
Inbox work before expanding the special-mod catalog further.

------------------------------------------------------------------------

# 1. Architecture Overview

The system should be built as a **local-first desktop application**.

Major layers:

1.  User Interface Layer
2.  Application Services Layer
3.  Rule Engine Layer
4.  AI Classification Layer
5.  Storage Layer
6.  File System Operations Layer

Each layer must remain loosely coupled so that components can be
replaced or improved without rewriting the entire system.

------------------------------------------------------------------------

# 2. Desktop Application Framework

Recommended stack:

Frontend UI: - Tauri - React - TypeScript

Backend Core: - Rust

Why this architecture:

• Rust handles heavy file operations safely • Tauri keeps the app
lightweight compared to Electron • React provides flexible UI
development • TypeScript improves reliability

------------------------------------------------------------------------

# 3. Major System Components

The system should be divided into the following core modules.

Modules:

scanner rule_engine validator bundle_detector duplicate_detector
ai_classifier library_index downloads_watcher tray_manager patch_manager
move_engine snapshot_manager

Each module should be independently testable.

------------------------------------------------------------------------

# 4. Scanner Service

The scanner indexes all Sims content.

Responsibilities:

• scan Mods folder • scan Tray folder • scan Downloads watcher folder •
extract metadata • calculate file hashes • detect bundles • populate the
database

Scanner must support incremental scans.

Full scan only runs on first launch or when requested.

------------------------------------------------------------------------

# 5. File System Layer

All file operations must be handled through a dedicated module.

Operations:

scan_directory() move_file() copy_file() delete_file() create_folder()
calculate_hash()

Important rule:

**No UI component should ever directly manipulate files.**

All file operations must go through the file system layer.

------------------------------------------------------------------------

# 6. Rule Engine

The rule engine determines suggested file locations.

Input:

metadata classification user_rules detected_structure

Output:

suggested_path confidence reason

The rule engine does not move files. It only calculates suggestions.

------------------------------------------------------------------------

# 7. Safety Validator

The validator checks rule engine results.

Checks include:

script_depth package_depth tray_location bundle_integrity path_collision

If a rule produces an unsafe path, the validator must correct it or send
the item to the Review Queue.

------------------------------------------------------------------------

# 8. AI Classification Service

The AI service is optional but recommended.

Responsibilities:

• classify files • infer creators • infer subtype • explain
classification

The AI service must return strict JSON.

It should never interact with the filesystem directly.

------------------------------------------------------------------------

# 9. Downloads Watcher

The downloads watcher monitors a user-selected folder.

Responsibilities:

• detect new archives • detect loose mod files • extract archives • send
files into the intake pipeline

The watcher should run as a background service.

------------------------------------------------------------------------

# 10. Bundle Detector

Bundle detection groups files that must move together.

Examples:

Tray household bundle Tray lot bundle Gameplay mod bundle

Bundle detection logic should run during scanning.

Bundles should receive a bundle_id.

------------------------------------------------------------------------

# 11. Duplicate Detector

Duplicate detection should run during scans.

Three duplicate types:

exact duplicate name duplicate version duplicate

Duplicates should be stored in a dedicated table.

------------------------------------------------------------------------

# 12. Snapshot Manager

Snapshots allow safe undo.

Snapshot contains:

original paths file hashes timestamps

Snapshots should be created:

• before migrations • before bulk moves • before patch recovery actions

------------------------------------------------------------------------

# 13. Database Layer

Recommended database:

SQLite

Reasons:

• lightweight • local-first • no external service needed

The database stores:

files bundles rules creators aliases duplicates review_items snapshots
scan_sessions

------------------------------------------------------------------------

# 14. Background Workers

Some tasks should run asynchronously.

Background jobs:

scanner jobs downloads watcher duplicate detection AI classification
queue

These should run in worker threads.

------------------------------------------------------------------------

# 15. Error Handling

All critical operations must include error handling.

Example cases:

file locked permission denied invalid archive missing bundle file

Failures must never crash the application.

Instead they should generate user-friendly warnings.

------------------------------------------------------------------------

# 16. Logging System

The system should include structured logging.

Log categories:

scanner filesystem rules ai duplicates patch_recovery

Logs help debugging and user support.

------------------------------------------------------------------------

# 17. Performance Goals

The application should handle large libraries.

Target scale:

10,000+ mod files 500+ tray bundles

Full scans should complete within a few minutes on typical hardware.

Incremental scans should be much faster.

------------------------------------------------------------------------

# 18. Security Principles

The app runs locally and should never upload user files.

Security goals:

• no automatic internet uploads • no external dependency on cloud
services • safe handling of archives • validation before extraction

------------------------------------------------------------------------

# 19. Future Extensibility

The architecture should allow:

support for additional Sims games plugin extensions new classification
models new organization presets

Design decisions should avoid hardcoding Sims 4 assumptions where
possible.

------------------------------------------------------------------------

# 20. Next Document

PRD Part 6 --- Codex Implementation Blueprint
