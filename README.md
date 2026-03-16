# SimSuite

## An All-in-One Sims Mod Management Suite

SimSuite is a local-first desktop application for managing Sims 4 mods and custom content (CC). Built with Tauri, React, and Rust, it provides a powerful yet intuitive interface for organizing, auditing, and maintaining your Sims 4 mod library.

## Features

### Core Functionality

- **Library Scanning & Indexing** — Automatically scan your mods and tray folders to build a comprehensive index of all installed content
- **Downloads Management** — Monitor your downloads folder with an inbox system for new content
- **Duplicate Detection** — Identify and manage duplicate mods using hash-based comparison
- **Category Auditing** — Review and organize mods by category (CAS, Build/Buy, Script Mods, etc.)
- **Creator Auditing** — Track and manage content by creator

### Advanced Features

- **Install Profile Engine** — Guided installation for supported special mods including:
  - MCCC (More Characters in Create-a-Sim)
  - XML Injector
  - Lot 51 Core Library
  - Sims 4 Community Library
  - Lumpinou Toolbox
  - Smart Core Script
- **Rule Engine** — Automated organization based on customizable rules
- **File Validation** — Validate mod integrity and detect unsafe content
- **Watch System** — Monitor for updates on supported mods
- **Snapshot & Rollback** — Create snapshots before batch operations and restore when needed

### User Experience

- **Multiple Themes** — Choose from themes like Plumbob, Build/Buy, CAS, Neighborhood, Debug Grid, Sunroom, Patch Day, and Night Market
- **Experience Modes** — Casual, Seasoned, or Creator mode depending on your expertise
- **Layout Presets** — Customize views for Browse, Inspect, Catalog, Queue, Balanced, Focus, Sweep, and Compare modes
- **System Tray** — Run in background with tray icon support
- **Desktop Notifications** — Get alerts for updates and important events

## Technology Stack

- **Frontend**: React 19 + TypeScript 5.9 + TailwindCSS
- **Backend**: Rust with Tauri 2.10
- **Database**: SQLite (rusqlite with bundled feature)
- **Build Tool**: Vite 7.3
- **Desktop Framework**: Tauri 2.10 with system tray support

### Project Structure

```
SimSuite/
├── src/                    # React frontend
│   ├── components/         # UI components
│   ├── screens/           # Screen components
│   ├── lib/               # Frontend utilities
│   └── styles/            # Global styles
├── src-tauri/             # Rust backend
│   └── src/
│       ├── commands/      # Tauri command handlers
│       ├── core/          # Core business logic
│       │   ├── scanner/
│       │   ├── filename_parser/
│       │   ├── bundle_detector/
│       │   ├── duplicate_detector/
│       │   ├── downloads_watcher/
│       │   ├── install_profile_engine/
│       │   ├── rule_engine/
│       │   ├── validator/
│       │   ├── library_index/
│       │   ├── move_engine/
│       │   └── snapshot_manager/
│       ├── database/      # Database operations
│       └── seed/          # Seed data
├── database/              # SQL migrations and schema
├── seed/                  # Starter data
│   ├── creators.json
│   ├── heuristics.json
│   ├── keywords.json
│   ├── presets.json
│   ├── taxonomy.json
│   └── install_profiles.json
└── scripts/               # Development and build scripts
```

## Installation

### Prerequisites

- Node.js 18+
- Rust 1.70+
- Windows 10/11 (primary target)

### Development Setup

1. Clone the repository:
```bash
git clone <repository-url>
cd SimSuite
```

2. Install dependencies:
```bash
npm install
```

3. Set up Rust dependencies:
```bash
cd src-tauri
cargo fetch
```

4. Run in development mode:
```bash
npm run tauri:dev
```

### Production Build

Build the application:
```bash
npm run tauri:build
```

The built executable will be located in:
```
src-tauri/target/release/bundle/nsis/
```

## Usage

### Initial Setup

1. Launch SimSuite
2. Configure your library paths:
   - Mods folder (Documents/Electronic Arts/The Sims 4/mod)
   - Tray folder (Documents/Electronic Arts/The Sims 4/Tray)
   - Downloads folder
3. Run an initial scan to index your library

### Screens

| Screen | Purpose |
|--------|---------|
| **Home** | Overview of your library with statistics and quick actions |
| **Downloads** | Manage new downloads with inbox, queue, and processing |
| **Library** | Browse and manage your entire mod collection |
| **Organize** | Preview and apply organization rules |
| **Review** | Review items requiring attention |
| **Duplicates** | Find and resolve duplicate mods |
| **Creator Audit** | Audit and organize mods by creator |
| **Category Audit** | Review and fix mod categories |
| **Settings** | Configure app behavior and preferences |

### Key Workflows

#### Scanning Your Library
1. Go to Home screen
2. Click "Scan Library" to index all mods
3. View statistics and any issues

#### Managing Downloads
1. Downloads are automatically monitored
2. Review new items in the Inbox
3. Apply, ignore, or queue items

#### Organizing Mods
1. Go to Organize screen
2. Preview organization rules
3. Apply changes with optional snapshot

## Configuration

### Library Settings

```typescript
interface LibrarySettings {
  modsPath: string | null;      // Path to mods folder
  trayPath: string | null;      // Path to tray folder
  downloadsPath: string | null;  // Path to downloads folder
}
```

### App Behavior Settings

```typescript
interface AppBehaviorSettings {
  keepRunningInBackground: boolean;      // Minimize to tray on close
  automaticWatchChecks: boolean;          // Auto-check for updates
  watchCheckIntervalHours: number;       // Check interval (1-24 hours)
  lastWatchCheckAt: string | null;        // Last check timestamp
  lastWatchCheckError: string | null;     // Last error message
}
```

### UI Preferences

- **Theme**: `plumbob` | `buildbuy` | `cas` | `neighborhood` | `debuggrid` | `sunroom` | `patchday` | `nightmarket`
- **Experience Mode**: `casual` | `seasoned` | `creator`
- **User View**: `beginner` | `standard` | `power`
- **Density**: `compact` | `balanced` | `roomy`

### Layout Presets

- **Library**: `browse` | `inspect` | `catalog` | `custom`
- **Review**: `queue` | `balanced` | `focus` | `custom`
- **Duplicates**: `sweep` | `balanced` | `compare` | `custom`

## API Reference

### Tauri Commands

#### Library Management

| Command | Description |
|---------|-------------|
| `get_library_settings` | Retrieve library path configuration |
| `save_library_paths` | Save library paths |
| `detect_default_library_paths` | Auto-detect Sims 4 folders |
| `scan_library` | Perform full library scan |
| `start_scan` | Start async scan |
| `get_scan_status` | Get current scan progress |

#### Downloads

| Command | Description |
|---------|-------------|
| `refresh_downloads_inbox` | Refresh downloads inbox |
| `get_downloads_inbox` | Get pending downloads |
| `get_downloads_queue` | Get queued items |
| `get_download_item_detail` | Get item details |
| `apply_download_item` | Apply single item |
| `ignore_download_item` | Mark item as ignored |

#### Organization

| Command | Description |
|---------|-------------|
| `preview_download_item` | Preview organization |
| `apply_preview_organization` | Apply organization |
| `list_snapshots` | List available snapshots |
| `restore_snapshot` | Restore from snapshot |

#### Audit

| Command | Description |
|---------|-------------|
| `get_creator_audit` | Get creator audit data |
| `get_category_audit` | Get category audit data |
| `apply_creator_audit` | Apply creator changes |
| `apply_category_audit` | Apply category changes |

#### Watch System

| Command | Description |
|---------|-------------|
| `list_library_watch_items` | List watched items |
| `save_watch_source_for_file` | Set watch source |
| `refresh_watched_sources` | Refresh all sources |

### Data Types

#### HomeOverview

```typescript
interface HomeOverview {
  totalFiles: number;
  modsCount: number;
  trayCount: number;
  downloadsCount: number;
  scriptModsCount: number;
  creatorCount: number;
  bundlesCount: number;
  duplicatesCount: number;
  reviewCount: number;
  unsafeCount: number;
  exactUpdateItems: number;
  possibleUpdateItems: number;
  unknownWatchItems: number;
  watchReviewItems: number;
  watchSetupItems: number;
  lastScanAt: string | null;
  scanNeedsRefresh: boolean;
  readOnlyMode: boolean;
}
```

#### DownloadsWatcherState

```typescript
type DownloadsWatcherState = "idle" | "watching" | "processing" | "error";
```

## Testing

### Rust Tests

Run backend unit tests:
```bash
npm run test:rust
```

### Desktop Testing

#### WebDriver Tests
```bash
npm run desktop:driver
npm run desktop:driver:fixtures
```

#### Smoke Tests
```bash
npm run desktop:smoke
npm run desktop:smoke:apply
npm run desktop:smoke:fixtures
npm run desktop:smoke:apply:fixtures
```

### Development Testing

1. Start development server:
```bash
npm run tauri:dev
```

2. Run in cleanup mode:
```bash
npm run dev:cleanup
```

## Contributing

### Getting Started

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Submit a pull request

### Code Style

- Follow existing code conventions
- Use TypeScript for frontend code
- Use Rust idioms for backend code
- Run formatting before committing

### Development Workflow

1. Read `SESSION_HANDOFF.md` at session start
2. Check `docs/IMPLEMENTATION_STATUS.md` for current status
3. Update `SESSION_HANDOFF.md` at session end
4. Update `docs/ARCHITECTURE.md` only when structure changes

### Architecture Guidelines

- Keep frontend thin; business logic in Rust
- Use SQLite for all persistent data
- Follow the safety pipeline: Scanner → Parser → Rule Engine → Validator → Preview → User Approval → Move Engine
- Maintain backward compatibility for public APIs

## Roadmap

See [`PLAN.md`](./PLAN.md) for detailed development plans and current status.

### Upcoming Features

- [ ] Expanded special mod support
- [ ] Batch watch management
- [ ] Performance optimizations
- [ ] Additional desktop tests
- [ ] Patch Recovery screen
- [ ] Tools screen

## License

MIT License

Copyright (c) 2024-2026 SimSuite Contributors

## Contact

- **Author**: likwi
- **Repository**: https://github.com/likwi/simsuite

---

*SimSuite — Your complete Sims 4 mod management solution*
