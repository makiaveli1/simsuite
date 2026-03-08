# SimSuite Architecture

This repository currently runs as a Tauri desktop app with:

- `src/`: React + TypeScript desktop UI
- `src-tauri/src/`: Rust backend modules and Tauri commands
- `database/`: SQLite migrations and schema mirror
- `seed/`: starter creators, aliases, taxonomy, keyword dictionaries, and rule presets
- `models/`: reserved for shared classification and prompt assets
- `user_data/`: reserved for user-learned data

## Current backend scope

Implemented modules:

- `scanner`
- `filename_parser`
- `bundle_detector`
- `duplicate_detector`
- `rule_engine`
- `validator`
- `library_index`
- `move_engine`
- `snapshot_manager`

Planned placeholders still present:

- `ai_classifier`
- `downloads_watcher`

## Active safety pipeline

The current organization flow follows the product safety chain:

1. scanner
2. parser
3. rule engine
4. validator
5. preview
6. user approval
7. move engine

Snapshots are created before approved batch moves, and rollback is exposed through snapshot restore.

## Current UI scope

Implemented screens:

- Home
- Library
- Organize
- Review

Not yet implemented:

- Duplicates
- Tray
- Patch Recovery
- Tools
- Settings

## Known gaps versus the full product plan

- scanner is full-scan only today; incremental scan support is not implemented
- duplicate detection is exact-hash only; filename/version duplicate logic is still missing
- tray bundles are grouped; broader mod-set bundle detection is not implemented
- AI classification is not wired to Ollama or llama.cpp yet
- downloads watcher, archive intake, and patch recovery tooling are still pending
- background worker orchestration is still minimal
