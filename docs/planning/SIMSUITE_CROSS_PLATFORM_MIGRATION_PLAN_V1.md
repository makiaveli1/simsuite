# SimSuite Cross-Platform Migration and Architecture Plan v1

**Date:** 3 August 2026  
**Canonical source:** `makiaveli1/simsuite` on GitHub  
**Decision:** The GitHub `main` branch is the new canonical baseline. Unpushed Windows-only work is intentionally excluded.  
**Primary development host:** macOS  
**Product target:** Windows, macOS, and Linux desktop environments, with The Sims 4 as the first game adapter.

## 1. Product platform policy

### First-class product targets

- **Windows:** full Sims 4 support, native folder discovery, native file operations, installer, updater, and desktop verification.
- **macOS:** full Sims 4 support, native folder discovery, native file operations, DMG/app packaging, permissions guidance, and desktop verification.
- **Linux:** full SimSuite desktop support. Sims 4 support is provided through explicit user-selected or detected Wine, Proton, Steam, or Lutris environments because The Sims 4 is not a native Linux release.

### Out of scope

- PlayStation and Xbox mod management, because The Sims 4 does not support mods on consoles.
- Pretending a guessed path is authoritative.
- Silently rewriting a player's existing folder structure.
- Platform-specific file changes without a preview, verification, and undo path.

## 2. Architectural objective

SimSuite should not contain Windows assumptions scattered through screens, commands, SQL, and scripts.

The application should have four clear layers:

1. **Shared domain core**
2. **Game adapters**
3. **Platform adapters**
4. **Presentation and experience modes**

This allows the same player journey and safety contract to operate everywhere while keeping genuine operating-system and game differences explicit.

## 3. Shared domain core

The shared core must be operating-system neutral and contain:

- archive inspection
- package and script inspection
- DBPF/resource analysis
- creator and category inference
- duplicate and version evidence
- dependency evidence
- installation planning
- organization planning
- safety validation
- snapshot metadata
- transaction history
- rollback planning
- update comparison
- player preferences
- experience modes
- automation policies
- organization profiles

Shared code must accept paths and capabilities from adapters rather than deriving Windows paths internally.

## 4. Platform adapter

Create a Rust platform contract, implemented separately for Windows, macOS, and Linux.

Illustrative responsibilities:

```rust
trait PlatformAdapter {
    fn platform_id(&self) -> PlatformId;
    fn discover_documents_dirs(&self) -> Result<Vec<PathCandidate>>;
    fn discover_download_dirs(&self) -> Result<Vec<PathCandidate>>;
    fn reveal_in_file_manager(&self, path: &Path) -> Result<()>;
    fn move_to_recoverable_trash(&self, path: &Path) -> Result<TrashReceipt>;
    fn filesystem_capabilities(&self, root: &Path) -> Result<FilesystemCapabilities>;
    fn available_space(&self, root: &Path) -> Result<u64>;
    fn normalize_for_comparison(&self, path: &Path) -> NormalizedPath;
    fn watch_path(&self, path: &Path, options: WatchOptions) -> Result<WatchHandle>;
}
```

Platform-specific behavior includes:

- Documents and Downloads discovery
- Finder, Explorer, or Linux file-manager reveal
- Recycle Bin, Trash, or freedesktop trash behavior
- permission and sandbox guidance
- case sensitivity
- path separators
- symbolic links and aliases
- file locks
- atomic rename guarantees
- long-path handling
- mounted, removable, network, and cloud-synced storage
- native notifications
- app autostart
- game process detection
- installer and updater integration

## 5. Game adapter

Create a game contract separate from the operating system.

```rust
trait GameAdapter {
    fn game_id(&self) -> GameId;
    fn supported_content_types(&self) -> &[ContentType];
    fn discover_profiles(
        &self,
        platform: &dyn PlatformAdapter,
    ) -> Result<Vec<GameProfileCandidate>>;
    fn validate_destination(&self, destination: &Path) -> ValidationReport;
    fn inspect_content(&self, path: &Path) -> Result<ContentEvidence>;
    fn placement_rules(&self) -> &[PlacementRule];
    fn readiness_checks(&self) -> &[ReadinessCheck];
}
```

The first adapter is `Sims4Adapter`.

Later adapters may support other Sims games or life-simulation games without weakening or generalizing away Sims 4-specific intelligence.

## 6. Installation profile model

Do not assume one global Mods folder.

A player may have:

- a normal Windows installation
- OneDrive-redirection
- a macOS Documents folder
- a custom Documents location
- multiple Sims user folders
- a test profile
- an external drive
- a Steam Proton prefix
- a Lutris or Wine prefix
- a manually selected library

Represent each setup as an explicit installation profile:

```text
Game profile
├── game
├── operating environment
├── game install location, optional
├── user-data root
├── Mods root
├── Tray root
├── Downloads intake roots
├── save/cache roots, optional
├── capabilities
├── detection evidence
└── user confirmation state
```

Detection results should be ranked:

- Confirmed by player
- Strongly detected
- Possible
- Manually configured
- Unavailable

SimSuite must never silently select an ambiguous profile.

## 7. Portable path identity

Absolute Windows or macOS paths must not be the permanent identity of a file.

Store:

- `profile_id`
- `root_id`
- normalized relative path
- display path
- filesystem identity when available
- comparison key based on filesystem capabilities

Use Rust `Path` and `PathBuf` internally. Avoid slash manipulation and path parsing with raw strings.

This permits the same database and transaction model to survive:

- Windows to Mac migration
- drive-letter changes
- Documents relocation
- cloud-folder relocation
- Wine-prefix changes
- external drive remounting

## 8. Filesystem capability model

Before performing a transaction, SimSuite should discover or record:

- case-sensitive or case-insensitive behavior
- Unicode normalization behavior
- symbolic-link support
- atomic rename support
- trash availability
- available space
- read/write permissions
- cloud or network storage hints
- removable-volume status
- path length constraints
- file locking behavior

Actions should use capabilities rather than assumptions such as “Windows behaves this way.”

## 9. Cross-platform transaction safety

All file-changing operations should use one transaction engine:

1. Resolve the confirmed game profile.
2. Normalize and validate every source and destination.
3. Detect collisions using the target filesystem's comparison rules.
4. Confirm sufficient space.
5. create a snapshot or recoverable transaction record.
6. stage changes where possible.
7. apply changes.
8. verify hashes, sizes, destinations, and placement rules.
9. refresh only affected index entries.
10. expose undo or recovery.

Platform adapters provide primitives. They must not independently invent product behavior.

## 10. Development-script migration

The current repository contains Windows-bound npm scripts, including direct calls to:

- `powershell.exe`
- `/mnt/c/Windows/System32/...`
- `.ps1` development wrappers
- Windows-only desktop proof lanes

Replace this with a portable command surface.

### Preferred structure

```text
scripts/
├── dev/
│   ├── run-tauri-dev.mjs
│   └── cleanup-dev-port.mjs
├── test/
│   ├── run-unit.mjs
│   ├── run-rust.mjs
│   └── run-desktop-smoke.mjs
└── platform/
    ├── windows/
    ├── macos/
    └── linux/
```

Top-level package scripts call Node or a Rust `xtask`. Those dispatch to an OS-specific helper only where native behavior is genuinely required.

Examples:

```json
{
  "scripts": {
    "dev": "vite",
    "tauri:dev": "tauri dev",
    "build": "tsc && vite build",
    "test:unit": "node scripts/test/run-unit.mjs",
    "test:rust": "node scripts/test/run-rust.mjs",
    "desktop:smoke": "node scripts/test/run-desktop-smoke.mjs"
  }
}
```

Keep Windows PowerShell scripts as implementation details for Windows-specific proof tasks, not as the universal entry point.

Audit the explicit `@rollup/rollup-linux-x64-gnu` dependency. A native Linux x64 Rollup binary should not be a universal direct dependency for Apple Silicon, Windows, and other Linux architectures unless a documented build reason requires it.

## 11. Low-spec performance contract

Cross-platform support must preserve the low-spec requirement.

Use:

- streaming directory traversal
- bounded worker pools
- configurable scan intensity
- adaptive concurrency
- deferred thumbnail generation
- thumbnail disk and memory budgets
- incremental hashes
- resumable scans
- cancellation
- per-file inspection timeouts
- database write batching
- WAL mode where appropriate
- virtualized UI collections
- affected-path refresh instead of full rescans
- idle-time deep inspection
- battery-aware background work where practical

Create at least three performance profiles:

- **Light:** older laptops and very large libraries
- **Balanced:** default
- **Fast:** powerful systems

These profiles change resource budgets, not safety or truth.

## 12. UI platform neutrality

The UI should generally say:

- `Show in folder`

It may display the native action name in supporting text:

- Explorer on Windows
- Finder on macOS
- File Manager on Linux

Avoid Windows-only terminology in shared screens.

Permissions screens should be native:

- Windows controlled-folder access or antivirus interference
- macOS Files and Folders or Full Disk Access guidance
- Linux file ownership, mount, Flatpak portal, or sandbox guidance

## 13. Verification matrix

### Shared automated lane

Run on all three systems:

- TypeScript checks
- React/Vitest tests
- Rust unit tests
- SQLite migrations
- query-level integration tests
- transaction planning tests
- path normalization tests
- archive/package fixture tests

### Platform contract lane

Run platform-specific tests for:

- path discovery
- reveal in file manager
- trash behavior
- watcher behavior
- permission failures
- case-collision handling
- symlink handling
- cloud/mounted storage behavior where fixtures permit

### Native desktop lane

- Windows Tauri smoke test
- macOS Tauri smoke test
- Linux Tauri smoke test

### CI matrix

Use GitHub Actions with:

- `windows-latest`
- `macos-latest`
- `ubuntu-latest`

Build and test each platform on that platform. Do not cross-compile and assume native behavior is correct.

## 14. Migration execution order

### Phase 0: Establish the Mac baseline

1. Clone canonical GitHub `main`.
2. Inspect repository state and recent history.
3. Create `platform/cross-platform-foundation`.
4. Install dependencies without changing lockfiles unexpectedly.
5. record Node, npm, Rust, Cargo, and Tauri versions.
6. Run frontend, TypeScript, unit, Rust, and build baselines.
7. Attempt a native macOS Tauri launch.
8. Record every failure before fixing anything.

### Phase 1: Portable development commands

1. Remove Windows-only assumptions from universal npm scripts.
2. Introduce portable Node or Rust command dispatchers.
3. retain Windows helpers behind platform-specific dispatch.
4. add macOS development and smoke commands.
5. add Linux development and smoke commands.
6. verify lockfile and native package handling.

### Phase 2: Platform-boundary audit

Search for and classify:

- drive letters
- backslashes
- `USERPROFILE`
- `APPDATA`
- `LOCALAPPDATA`
- OneDrive assumptions
- `explorer.exe`
- PowerShell calls
- Windows Registry calls
- Windows-only process names
- path comparisons using lowercase strings
- absolute Sims 4 paths
- filesystem operations embedded in screens or domain services

Move them behind platform contracts.

### Phase 3: macOS Sims 4 vertical slice

Prove:

```text
Select or detect profile
→ scan Mods and Tray fixtures
→ inspect content
→ prepare install plan
→ preview
→ snapshot
→ apply to a safe fixture library
→ verify
→ show in Finder
→ undo
```

Do not test destructive behavior against the player's real Sims folders until fixture-based proof passes.

### Phase 4: Windows parity

When Windows access returns:

1. clone canonical main and the cross-platform branch
2. run the full Windows baseline
3. adapt native Windows behavior to the platform contract
4. verify OneDrive and custom Documents paths
5. restore Windows installer and desktop-proof parity
6. confirm no regression against large real-world fixtures

### Phase 5: Linux compatibility

Support:

- manual profile selection first
- Steam Proton prefixes
- Wine prefixes
- Lutris-managed prefixes
- custom locations
- common Linux file managers
- AppImage and one additional package format initially

Do not claim official native Sims 4 Linux support. Present Linux game locations as compatibility environments.

### Phase 6: Release gates

A feature is not complete until:

- shared tests pass
- relevant platform contract tests pass
- native smoke tests pass on the target OS
- low-spec budgets pass
- player-facing certainty and undo behavior are correct

## 15. First cross-platform milestone

The first meaningful cross-platform milestone is:

> On Windows and macOS, and through an explicitly configured Linux compatibility profile, a player can select a Sims 4 setup, inspect a downloaded mod, preview where it will go, install it into a safe fixture library, verify the result, see it in the Library, and undo the transaction.

This milestone proves the product journey and the architecture at the same time.

## 16. Immediate next actions

1. Clone `makiaveli1/simsuite` into `/Users/gracebabalola/Downloads/SimSuite`.
2. Let PC Bridge inspect and set it as the active project.
3. create the cross-platform foundation branch.
4. capture an untouched Mac baseline.
5. perform the platform-boundary audit.
6. fix universal development commands before changing product behavior.
7. begin the macOS fixture-based vertical slice.
