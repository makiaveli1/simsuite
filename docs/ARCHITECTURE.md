# SimSuite Architecture

This repository currently runs as a Tauri desktop app with:

- `src/`: React + TypeScript desktop UI
- `src-tauri/src/`: Rust backend modules and Tauri commands
- `database/`: SQLite migrations and schema mirror
- `seed/`: starter creators, aliases, taxonomy, keyword dictionaries, and rule presets
- `seed/install_profiles.json`: built-in special mod catalog for guided installs, dependency rules, incompatibility warnings, and review-only download patterns
- `models/`: reserved for shared classification and prompt assets
- `user_data/`: reserved for user-learned data

## Current backend scope

Implemented modules:

- `scanner`
- `filename_parser`
- `bundle_detector`
- `duplicate_detector`
- `downloads_watcher`
- `install_profile_engine`
- `rule_engine`
- `validator`
- `library_index`
- `move_engine`
- `snapshot_manager`

Planned placeholders still present:

- `ai_classifier`

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
- Downloads
- Library
- Settings
- Creator Audit
- Category Audit
- Duplicates
- Organize
- Review

Not yet implemented:

- Tray
- Patch Recovery
- Tools

## Known gaps versus the full product plan

- scanner is incremental and backgrounded now, but deeper prioritization and scheduling controls are still missing
- duplicate detection covers exact, filename, and version cases, but safe duplicate cleanup actions are still missing
- tray bundles are grouped; broader mod-set bundle detection is not implemented
- the special mod catalog covers the first curated wave only; broader curated profile, dependency, and incompatibility coverage is still pending
- option-pack and manual-step archives are detected and routed to review, but SimSuite does not yet guide users through choosing install variants
- AI classification is not wired to Ollama or llama.cpp yet
- patch recovery tooling is still pending
- user-facing Tools and Tray surfaces are still missing
