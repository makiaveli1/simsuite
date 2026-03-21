# Non-CurseForge Mod Update Tracking Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build an advanced update-tracking system that handles both structured sources (CurseForge, GitHub) and non-structured sources (Patreon blogs, generic pages, RSS feeds) with confidence scoring and user confirmation.

**Architecture:** This replaces the current single-URL watch system with a multi-layer architecture: local mod inventory → source candidate engine → pluggable source adapters → normalized snapshot store → decision engine → notification layer.

**Tech Stack:** Rust (Tauri backend), SQLite with new tables, reqwest for HTTP, serde for serialization

**PRD Reference:** `non_curseforge_mod_tracking_prd.md`

---

## Key Type Definitions (Single Source of Truth)

All enums and shared types live in `src-tauri/src/models.rs`. Adapters import from there.

```rust
// In models.rs - SourceKind enum (SINGLE DEFINITION)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SourceKind {
    CurseForge,
    GitHub,
    Nexus,
    Feed,
    StructuredPage,
    GenericPage,
}

// In models.rs - TrackingMode enum
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrackingMode {
    Auto,
    Manual,
    DetectedOnly,
    Ignored,
}

// In models.rs - UpdateStatus enum
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UpdateStatus {
    UpToDate,
    ConfirmedUpdate,
    ProbableUpdate,
    SourceActivity,
    SourceUnreachable,
    Untracked,
}

// In models.rs - LocalMod struct (database-backed)
pub struct LocalMod {
    pub id: String,
    pub display_name: String,
    pub normalized_name: String,
    pub creator_name: Option<String>,
    pub category: Option<String>,
    pub local_root_path: String,
    pub tracking_mode: TrackingMode,
    pub source_confidence: f64,
    pub confirmed_source_id: Option<String>,
    pub current_status: UpdateStatus,
    pub last_checked_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

// In models.rs - LocalFile struct (database-backed)
pub struct LocalFile {
    pub id: String,
    pub local_mod_id: String,
    pub file_path: String,
    pub file_name: String,
    pub file_ext: String,
    pub file_size: i64,
    pub sha256: Option<String>,
    pub modified_at: Option<String>,
}

// In models.rs - SourceBinding struct (database-backed)
pub struct SourceBinding {
    pub id: String,
    pub local_mod_id: String,
    pub source_kind: SourceKind,
    pub source_url: String,
    pub provider_mod_id: Option<String>,
    pub provider_file_id: Option<String>,
    pub provider_repo: Option<String>,
    pub bind_method: String,
    pub is_primary: bool,
    pub created_at: String,
    pub updated_at: String,
}
```

---

## Phase 1: Foundation (Scanner + Core Adapters + Snapshot Store)

### Phase 1.1: New Database Schema

**Files:**
- Modify: `src-tauri/src/database/mod.rs` (add new tables inline, following existing pattern)
- Modify: `src-tauri/src/models.rs` (add new types above)

- [ ] **Step 1: Add new types to models.rs**

Add to `src-tauri/src/models.rs`:
- `SourceKind` enum
- `TrackingMode` enum
- `UpdateStatus` enum
- `LocalMod` struct
- `LocalFile` struct
- `SourceBinding` struct

- [ ] **Step 2: Add new tables to database module**

Modify `src-tauri/src/database/mod.rs`:
- Follow existing pattern: add inline SQL in `ensure_schema()` function
- Use `create_table_if_not_exists` helper for each new table
- Add indexes inline with table creation or via `create_index_if_table_exists`
- Add migration version 2 to `schema_migrations`

New tables to add:
```rust
// local_mods
"CREATE TABLE IF NOT EXISTS local_mods (
    id TEXT PRIMARY KEY,
    display_name TEXT NOT NULL,
    normalized_name TEXT NOT NULL,
    creator_name TEXT,
    category TEXT,
    local_root_path TEXT NOT NULL,
    tracking_mode TEXT NOT NULL DEFAULT 'detected_only',
    source_confidence REAL DEFAULT 0,
    confirmed_source_id TEXT,
    current_status TEXT NOT NULL DEFAULT 'untracked',
    last_checked_at TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
)",

// local_files
"CREATE TABLE IF NOT EXISTS local_files (
    id TEXT PRIMARY KEY,
    local_mod_id TEXT NOT NULL REFERENCES local_mods(id),
    file_path TEXT NOT NULL,
    file_name TEXT NOT NULL,
    file_ext TEXT NOT NULL,
    file_size INTEGER NOT NULL,
    sha256 TEXT,
    modified_at TEXT
)",

// candidate_sources
"CREATE TABLE IF NOT EXISTS candidate_sources (
    id TEXT PRIMARY KEY,
    local_mod_id TEXT NOT NULL REFERENCES local_mods(id),
    source_kind TEXT NOT NULL,
    source_url TEXT NOT NULL,
    provider_mod_id TEXT,
    provider_file_id TEXT,
    provider_repo TEXT,
    confidence_score REAL NOT NULL,
    reasoning_json TEXT NOT NULL DEFAULT '[]',
    status TEXT NOT NULL DEFAULT 'suggested',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
)",

// source_bindings
"CREATE TABLE IF NOT EXISTS source_bindings (
    id TEXT PRIMARY KEY,
    local_mod_id TEXT NOT NULL REFERENCES local_mods(id),
    source_kind TEXT NOT NULL,
    source_url TEXT NOT NULL,
    provider_mod_id TEXT,
    provider_file_id TEXT,
    provider_repo TEXT,
    bind_method TEXT NOT NULL,
    is_primary INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
)",

// remote_snapshots
"CREATE TABLE IF NOT EXISTS remote_snapshots (
    id TEXT PRIMARY KEY,
    binding_id TEXT NOT NULL REFERENCES source_bindings(id),
    snapshot_hash TEXT NOT NULL,
    title TEXT,
    version_text TEXT,
    published_at TEXT,
    download_url TEXT,
    changelog_url TEXT,
    release_id TEXT,
    asset_names_json TEXT,
    image_hashes_json TEXT,
    raw_summary_json TEXT NOT NULL,
    etag TEXT,
    last_modified TEXT,
    fetched_at TEXT NOT NULL
)",

// update_events
"CREATE TABLE IF NOT EXISTS update_events (
    id TEXT PRIMARY KEY,
    local_mod_id TEXT NOT NULL REFERENCES local_mods(id),
    binding_id TEXT,
    event_type TEXT NOT NULL,
    confidence_score REAL NOT NULL,
    summary TEXT NOT NULL,
    latest_version_text TEXT,
    latest_published_at TEXT,
    is_read INTEGER NOT NULL DEFAULT 0,
    is_dismissed INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL
)",

// user_tracking_prefs
"CREATE TABLE IF NOT EXISTS user_tracking_prefs (
    local_mod_id TEXT PRIMARY KEY REFERENCES local_mods(id),
    ignore_updates INTEGER NOT NULL DEFAULT 0,
    ignore_versions_json TEXT,
    notify_on_probable INTEGER NOT NULL DEFAULT 1,
    notify_on_source_activity INTEGER NOT NULL DEFAULT 0,
    manual_source_url TEXT,
    pinned_source_kind TEXT
)",
```

- [ ] **Step 3: Add indexes**

Add via `create_index_if_table_exists`:
```rust
"CREATE INDEX IF NOT EXISTS idx_candidate_sources_local_mod ON candidate_sources(local_mod_id)",
"CREATE INDEX IF NOT EXISTS idx_source_bindings_local_mod ON source_bindings(local_mod_id)",
"CREATE INDEX IF NOT EXISTS idx_remote_snapshots_binding ON remote_snapshots(binding_id)",
"CREATE INDEX IF NOT EXISTS idx_update_events_local_mod ON update_events(local_mod_id)",
```

- [ ] **Step 4: Write tests for new schema**

Create `src-tauri/src/database/schema_test.rs`:
- Test all new tables exist
- Test indexes exist
- Test foreign keys work

Run: `cargo test --manifest-path src-tauri/Cargo.toml database_schema`
Expected: PASS

- [ ] **Commit**

```bash
git add src-tauri/src/database/mod.rs src-tauri/src/models.rs
git commit -m "feat: add local_mods schema for new tracking system"
```

---

### Phase 1.2: Source Adapter Interface

**Files:**
- Create: `src-tauri/src/adapters/mod.rs` (traits, registry, shared types)
- Create: `src-tauri/src/adapters/curseforge.rs`
- Create: `src-tauri/src/adapters/github.rs`
- Create: `src-tauri/src/adapters/nexus.rs`
- Create: `src-tauri/src/adapters/errors.rs`

**Note:** The `services/` directory doesn't exist yet. Create it as a new module in `src-tauri/src/services/mod.rs`.

- [ ] **Step 1: Create adapter types (imported from models.rs)**

`src-tauri/src/adapters/mod.rs`:
```rust
// Types imported from models.rs:
// - SourceKind
// - LocalMod
// - LocalFile  
// - SourceBinding
// - UpdateStatus

use crate::error::AppResult;
use crate::models::{SourceKind, LocalMod, LocalFile, SourceBinding, UpdateStatus};

pub struct DiscoverInput {
    pub local_mod_id: String,
    pub display_name: String,
    pub normalized_name: String,
    pub creator_name: Option<String>,
    pub category: Option<String>,
    pub files: Vec<FileInfo>,
}

pub struct FileInfo {
    pub file_name: String,
    pub sha256: Option<String>,
    pub size: i64,
}

pub struct CandidateSource {
    pub source_kind: SourceKind,
    pub source_url: String,
    pub provider_mod_id: Option<String>,
    pub provider_file_id: Option<String>,
    pub provider_repo: Option<String>,
    pub confidence_score: f64,
    pub reasoning: Vec<String>,
}

pub struct RemoteSnapshot {
    pub binding_id: String,
    pub title: Option<String>,
    pub version_text: Option<String>,
    pub published_at: Option<String>,
    pub download_url: Option<String>,
    pub changelog_url: Option<String>,
    pub release_id: Option<String>,
    pub release_asset_names: Vec<String>,
    pub image_hashes: Vec<String>,
    pub etag: Option<String>,
    pub last_modified: Option<String>,
    pub evidence: SnapshotEvidence,
    pub confidence: f64,
    pub raw: serde_json::Value,
}

pub struct SnapshotEvidence {
    pub version_changed: bool,
    pub download_changed: bool,
    pub title_changed: bool,
    pub asset_list_changed: bool,
    pub feed_guid_changed: bool,
}

pub struct UpdateDecision {
    pub status: UpdateStatus,
    pub confidence: f64,
    pub summary: Option<String>,
}

pub trait SourceAdapter: Send + Sync {
    fn kind(&self) -> SourceKind;
    fn discover_candidates(&self, input: &DiscoverInput) -> AppResult<Vec<CandidateSource>>;
    fn refresh_snapshot(&self, binding: &SourceBinding) -> AppResult<RemoteSnapshot>;
    fn detect_update(
        &self,
        local_mod: &LocalMod,
        previous: Option<&RemoteSnapshot>,
        current: &RemoteSnapshot,
    ) -> UpdateDecision;
}

// Registry for all adapters
pub struct AdapterRegistry {
    adapters: Vec<Box<dyn SourceAdapter>>,
}

impl AdapterRegistry {
    pub fn new() -> Self { /* register all adapters */ }
    pub fn for_kind(&self, kind: SourceKind) -> Option<&dyn SourceAdapter> { /* ... */ }
    pub fn discover_all(&self, input: &DiscoverInput) -> AppResult<Vec<CandidateSource>> { /* ... */ }
}
```

- [ ] **Step 2: Create CurseForge adapter**

`src-tauri/src/adapters/curseforge.rs`:
- Implement `SourceAdapter` trait
- Use CF API for mod search and file lookup
- Return `CandidateSource` with confidence based on name matching
- Implement `refresh_snapshot` to fetch latest mod metadata
- Implement `detect_update` using release ID comparison

- [ ] **Step 3: Create GitHub adapter**

`src-tauri/src/adapters/github.rs`:
- Use GitHub API for releases
- Parse owner/repo from URL
- Track release IDs and asset names
- Implement full adapter trait

- [ ] **Step 4: Create Nexus adapter**

`src-tauri/src/adapters/nexus.rs`:
- Use Nexus Mods API if available
- Fall back to structured page parsing
- Track file update dates
- Implement full adapter trait

- [ ] **Step 5: Write adapter tests**

Create `src-tauri/src/adapters/adapters_test.rs`:
- Test GitHub release URL parsing
- Test adapter returns expected structure

Run: `cargo test --manifest-path src-tauri/Cargo.toml adapters`
Expected: PASS

- [ ] **Commit**

```bash
git add src-tauri/src/adapters/
git commit -m "feat: add source adapter interface and registry"
```

---

### Phase 1.3: Local Scanner Enhancement

**Files:**
- Create: `src-tauri/src/services/mod.rs` (new module entry point)
- Create: `src-tauri/src/services/local_inventory.rs`
- Modify: `src-tauri/src/core/scanner/mod.rs` (add local_mods creation)
- Modify: `src-tauri/src/core/mod.rs` (add services module)

- [ ] **Step 1: Create services module**

Create `src-tauri/src/services/mod.rs`:
```rust
pub mod local_inventory;
pub mod snapshot_store;
pub mod update_decision;
pub mod candidate_scorer;
pub mod candidate_discovery;
pub mod update_events;
pub mod rate_limiter;
pub mod scheduler;
pub mod source_learning;
```

- [ ] **Step 2: Create local inventory service**

`src-tauri/src/services/local_inventory.rs`:
```rust
pub struct LocalInventory;

impl LocalInventory {
    pub fn scan_and_update_local_mods(conn: &Connection) -> AppResult<LocalModScanResult> {
        // 1. Scan mods folder for .package and .ts4script
        // 2. Group files by folder/family into local_mod records
        // 3. Compute SHA256 for fingerprint matching
        // 4. Store in local_mods and local_files tables
        // 5. Return scan statistics
    }
    
    pub fn get_or_create_local_mod(
        conn: &Connection,
        display_name: &str,
        folder_path: &str,
    ) -> AppResult<String> {
        // Check if exists by normalized_name + folder
        // Create if not exists
        // Return mod id
    }
}
```

- [ ] **Step 3: Integrate into scanner**

Modify `src-tauri/src/core/scanner/mod.rs`:
- After file discovery, call `LocalInventory::scan_and_update_local_mods()`
- Pass file groups to inventory service

- [ ] **Step 4: Write local inventory tests**

Create `src-tauri/src/services/local_inventory_test.rs`:
- Test mod grouping by folder
- Test SHA256 computation
- Test create/get

Run: `cargo test --manifest-path src-tauri/Cargo.toml local_inventory`
Expected: PASS

- [ ] **Commit**

```bash
git add src-tauri/src/services/mod.rs src-tauri/src/services/local_inventory.rs src-tauri/src/core/scanner/mod.rs src-tauri/src/core/mod.rs
git commit -m "feat: add local mod inventory service"
```

---

### Phase 1.4: Snapshot Store and Decision Engine

**Files:**
- Create: `src-tauri/src/services/snapshot_store.rs`
- Create: `src-tauri/src/services/update_decision.rs`
- Create: `src-tauri/src/services/candidate_scorer.rs`

- [ ] **Step 1: Create snapshot store service**

`src-tauri/src/services/snapshot_store.rs`:
```rust
pub struct SnapshotStore;

impl SnapshotStore {
    pub fn store_snapshot(
        conn: &Connection,
        binding_id: &str,
        snapshot: &RemoteSnapshot,
    ) -> AppResult<String> {
        // Generate ID
        // Compute snapshot_hash from title + version + download_url
        // Store in remote_snapshots
        // Return snapshot ID
    }
    
    pub fn get_latest_snapshot(
        conn: &Connection,
        binding_id: &str,
    ) -> AppResult<Option<RemoteSnapshot>> {
        // Query latest row for binding_id
        // Parse JSON back to RemoteSnapshot
    }
    
    pub fn should_refetch(
        conn: &Connection,
        binding_id: &str,
        etag: Option<&str>,
        last_modified: Option<&str>,
    ) -> bool {
        // If no etag/last_modified, always refetch
        // If different from stored, refetch
        // Otherwise skip
    }
}
```

- [ ] **Step 2: Create update decision engine**

`src-tauri/src/services/update_decision.rs`:
```rust
pub fn detect_update(
    local_mod: &LocalMod,
    previous: Option<&RemoteSnapshot>,
    current: &RemoteSnapshot,
) -> UpdateDecision {
    if previous.is_none() {
        return UpdateDecision {
            status: UpdateStatus::SourceActivity,
            confidence: current.confidence,
            summary: Some("First time checking this source".into()),
        };
    }
    
    let prev = previous.unwrap();
    
    // Rule 1: Release ID changed
    if current.release_id.is_some() 
        && prev.release_id.is_some() 
        && current.release_id != prev.release_id {
        return UpdateDecision {
            status: UpdateStatus::ConfirmedUpdate,
            confidence: 0.98,
            summary: Some("New release detected".into()),
        };
    }
    
    // Rule 2: Version text changed
    if current.version_text.is_some() 
        && prev.version_text.is_some() 
        && current.version_text != prev.version_text {
        return UpdateDecision {
            status: UpdateStatus::ConfirmedUpdate,
            confidence: 0.92,
            summary: Some("Version changed".into()),
        };
    }
    
    // Rule 3: Download URL changed
    if current.download_url.is_some() 
        && prev.download_url.is_some() 
        && current.download_url != prev.download_url {
        return UpdateDecision {
            status: UpdateStatus::ProbableUpdate,
            confidence: 0.78,
            summary: Some("Download target changed".into()),
        };
    }
    
    // Rule 4: Asset list or feed GUID changed
    if current.evidence.asset_list_changed || current.evidence.feed_guid_changed {
        return UpdateDecision {
            status: UpdateStatus::ProbableUpdate,
            confidence: 0.72,
            summary: Some("Source assets changed".into()),
        };
    }
    
    // Rule 5: Title changed
    if current.evidence.title_changed {
        return UpdateDecision {
            status: UpdateStatus::SourceActivity,
            confidence: 0.55,
            summary: Some("Page title changed".into()),
        };
    }
    
    UpdateDecision {
        status: UpdateStatus::UpToDate,
        confidence: 0.99,
        summary: Some("No meaningful change".into()),
    }
}
```

- [ ] **Step 3: Create candidate scorer**

`src-tauri/src/services/candidate_scorer.rs`:
```rust
pub struct MatchSignals {
    pub stored_binding: bool,
    pub fingerprint_match: bool,
    pub exact_title_match: bool,
    pub fuzzy_title_score: f64,
    pub exact_creator_match: bool,
    pub fuzzy_creator_score: f64,
    pub file_name_similarity: f64,
    pub category_match: bool,
    pub image_similarity: f64,
    pub download_name_similarity: f64,
    pub user_confirmed_source: bool,
}

pub fn score_match(signals: &MatchSignals) -> f64 {
    let mut score = 0.0;
    if signals.user_confirmed_source { score += 50.0; }
    if signals.stored_binding { score += 40.0; }
    if signals.fingerprint_match { score += 40.0; }
    if signals.exact_title_match { score += 20.0; }
    score += signals.fuzzy_title_score * 15.0;
    if signals.exact_creator_match { score += 20.0; }
    score += signals.fuzzy_creator_score * 10.0;
    score += signals.file_name_similarity * 15.0;
    if signals.category_match { score += 10.0; }
    score += signals.image_similarity * 10.0;
    score += signals.download_name_similarity * 15.0;
    score.min(100.0)
}

pub fn confidence_level(score: f64) -> &str {
    match score {
        s if s >= 90.0 => "confirmed",
        s if s >= 70.0 => "probable",
        s if s >= 50.0 => "weak",
        _ => "rejected",
    }
}
```

- [ ] **Step 4: Write service tests**

Create `src-tauri/src/services/update_decision_test.rs`:
- Test confirmed update when release ID changes
- Test confirmed update when version text changes
- Test probable update when download URL changes
- Test source activity when only title changes
- Test up_to_date when nothing changes

Run: `cargo test --manifest-path src-tauri/Cargo.toml update_decision`
Expected: PASS

- [ ] **Commit**

```bash
git add src-tauri/src/services/
git commit -m "feat: add snapshot store and update decision engine"
```

---

### Phase 1.5: Update Events and Notification Layer

**Files:**
- Create: `src-tauri/src/services/update_events.rs`
- Modify: `src-tauri/src/commands/mod.rs` (add new commands)
- Modify: `src-tauri/src/core/watch_polling/mod.rs` (integrate new system)

**Migration Note:** The new system runs alongside the old `content_watch_sources` table. During initial rollout:
1. New tables are created but old system continues working
2. `scan_local_mods` command populates new tables from existing library files
3. Watch polling gradually migrates as bindings are confirmed
4. Old tables can be deprecated in a future migration

- [ ] **Step 1: Create update events service**

`src-tauri/src/services/update_events.rs`:
```rust
pub struct UpdateEvents;

impl UpdateEvents {
    pub fn create_event(
        conn: &Connection,
        local_mod_id: &str,
        binding_id: Option<&str>,
        decision: &UpdateDecision,
        latest_version: Option<&str>,
        latest_published_at: Option<&str>,
    ) -> AppResult<String> {
        // Generate ID
        // Insert into update_events
        // Return event ID
    }
    
    pub fn get_unread_events(
        conn: &Connection,
        limit: i64,
    ) -> AppResult<Vec<UpdateEvent>> {
        // Query where is_read = 0 and is_dismissed = 0
        // Order by created_at desc
        // Return events
    }
    
    pub fn mark_read(conn: &Connection, event_id: &str) -> AppResult<()> {
        // Update is_read = 1
    }
    
    pub fn dismiss_event(conn: &Connection, event_id: &str) -> AppResult<()> {
        // Update is_dismissed = 1
    }
    
    pub fn get_update_counts(conn: &Connection) -> AppResult<UpdateCounts> {
        // Count by status
    }
}
```

- [ ] **Step 2: Add new Tauri commands**

Modify `src-tauri/src/commands/mod.rs`:
- `scan_local_mods` - trigger full local mod scan
- `get_local_mods` - list mods with filters
- `get_candidates_for_mod` - list source candidates
- `confirm_candidate_source` - promote candidate to binding
- `reject_candidate_source` - mark candidate as rejected
- `get_update_events` - get unread events
- `mark_event_read` - mark event as read
- `dismiss_event` - dismiss event
- `refresh_source_now` - manually trigger refresh

- [ ] **Step 3: Integrate into watch polling (gradual migration)**

Modify `src-tauri/src/core/watch_polling/mod.rs`:
1. Keep existing `content_watch_sources` polling working
2. Add new `AdapterRegistry::refresh()` call for bindings in `source_bindings` table
3. Use `SnapshotStore` for caching with ETag/Last-Modified
4. Call `UpdateDecision` engine for new bindings
5. Create `UpdateEvents` for audit trail
6. Run both systems in parallel until new system is proven

- [ ] **Step 4: Write integration tests**

Create `src-tauri/src/services/update_events_test.rs`:
- Test event creation
- Test get unread
- Test mark read/dismiss

Run: `cargo test --manifest-path src-tauri/Cargo.toml update_events`
Expected: PASS

- [ ] **Commit**

```bash
git add src-tauri/src/services/update_events.rs src-tauri/src/commands/mod.rs src-tauri/src/core/watch_polling/mod.rs
git commit -m "feat: add update events and integrate into polling"
```

---

## Phase 2: Expanded Coverage (Manual Sources + Feed + Structured Pages + UI)

### Phase 2.1: Feed Adapter

**Files:**
- Create: `src-tauri/src/adapters/feed.rs`

- [ ] **Step 1: Create RSS/Atom adapter**

`src-tauri/src/adapters/feed.rs`:
- Implement `SourceAdapter` for `SourceKind::Feed`
- Parse RSS 2.0 and Atom formats
- Extract: title, link, guid, pubDate
- Detect new entries by GUID
- Implement `refresh_snapshot` with conditional requests

- [ ] **Step 2: Test feed adapter**

Create `src-tauri/src/adapters/feed_test.rs`:
- Test RSS 2.0 parsing
- Test Atom parsing
- Test new entry detection

Run: `cargo test --manifest-path src-tauri/Cargo.toml feed_adapter`
Expected: PASS

- [ ] **Commit**

```bash
git add src-tauri/src/adapters/feed.rs
git commit -m "feat: add RSS/Atom feed adapter"
```

---

### Phase 2.2: Structured Page Adapter

**Files:**
- Create: `src-tauri/src/adapters/structured_page.rs`

- [ ] **Step 1: Create structured page adapter**

`src-tauri/src/adapters/structured_page.rs`:
- Implement `SourceAdapter` for `SourceKind::StructuredPage`
- Parse HTML for known patterns:
  - Title from `<title>` or `<h1>`
  - Version from common patterns (v1.2.3, version: 1.2.3)
  - Download link from `<a>` with common text
  - Published date from `<time>` or meta tags
- Implement ETag/Last-Modified support

- [ ] **Step 2: Write tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml structured_page`
Expected: PASS

- [ ] **Commit**

```bash
git add src-tauri/src/adapters/structured_page.rs
git commit -m "feat: add structured page adapter"
```

---

### Phase 2.3: Generic Page Adapter

**Files:**
- Create: `src-tauri/src/adapters/generic_page.rs`

- [ ] **Step 1: Create generic page adapter**

`src-tauri/src/adapters/generic_page.rs`:
- Implement `SourceAdapter` for `SourceKind::GenericPage`
- Fallback when no other adapter matches
- Fetch page with conditional requests
- Extract meaningful text content
- Compute content hash for change detection
- Store minimal snapshot

- [ ] **Step 2: Write tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml generic_page`
Expected: PASS

- [ ] **Commit**

```bash
git add src-tauri/src/adapters/generic_page.rs
git commit -m "feat: add generic page adapter"
```

---

### Phase 2.4: Candidate Discovery Engine

**Files:**
- Create: `src-tauri/src/services/candidate_discovery.rs`
- Modify: `src-tauri/src/services/candidate_scorer.rs`

- [ ] **Step 1: Create candidate discovery service**

`src-tauri/src/services/candidate_discovery.rs`:
```rust
pub struct CandidateDiscovery;

impl CandidateDiscovery {
    pub fn discover_for_mod(
        adapters: &AdapterRegistry,
        local_mod: &LocalMod,
        files: &[LocalFile],
    ) -> AppResult<Vec<CandidateSource>> {
        let input = DiscoverInput {
            local_mod_id: local_mod.id.clone(),
            display_name: local_mod.display_name.clone(),
            normalized_name: local_mod.normalized_name.clone(),
            creator_name: local_mod.creator_name.clone(),
            category: local_mod.category.clone(),
            files: files.iter().map(|f| FileInfo {
                file_name: f.file_name.clone(),
                sha256: f.sha256.clone(),
                size: f.file_size,
            }).collect(),
        };
        
        // Run discover on all adapters
        // Collect and sort by confidence
        // Return top candidates
    }
    
    pub fn auto_bind_if_confident(
        conn: &Connection,
        candidates: &[CandidateSource],
    ) -> AppResult<Option<String>> {
        // If top candidate score >= 90, auto-promote to binding
        // Return binding_id if auto-bound
    }
}
```

- [ ] **Step 2: Integrate discovery into commands**

Modify commands to call discovery:
- When scanning, run discovery for each new local mod
- Store candidates in `candidate_sources` table

- [ ] **Step 3: Write tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml candidate_discovery`
Expected: PASS

- [ ] **Commit**

```bash
git add src-tauri/src/services/candidate_discovery.rs
git commit -m "feat: add candidate discovery engine"
```

---

### Phase 2.5: User Confirmation UI (Backend)

**Files:**
- Modify: `src-tauri/src/commands/mod.rs`
- Modify: `src-tauri/src/models.rs`

- [ ] **Step 1: Add confirmation/rejection commands**

```rust
#[tauri::command]
pub fn confirm_candidate_source(
    state: State<AppState>,
    candidate_id: String,
) -> Result<Binding, String> {
    // 1. Get candidate from DB
    // 2. Verify it's still 'suggested' status
    // 3. Create binding from candidate
    // 4. Update candidate status to 'confirmed'
    // 5. Return binding
}

#[tauri::command]
pub fn reject_candidate_source(
    state: State<AppState>,
    candidate_id: String,
) -> Result<(), String> {
    // 1. Get candidate from DB
    // 2. Update status to 'rejected'
    // 3. Optionally boost/down future scoring
}
```

- [ ] **Step 2: Write tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml confirm_candidate`
Expected: PASS

- [ ] **Commit**

```bash
git add src-tauri/src/commands/mod.rs
git commit -m "feat: add candidate confirmation/rejection commands"
```

---

## Phase 3: Advanced Features (Generic Watcher + Learning)

### Phase 3.1: Generic Page Watcher with Better Diffing

**Files:**
- Modify: `src-tauri/src/adapters/generic_page.rs`

- [ ] **Step 1: Improve generic page adapter**

- Add better text extraction (remove scripts, styles, nav)
- Compute semantic hash instead of raw content hash
- Detect version-like strings in content
- Compare normalized URLs

- [ ] **Step 2: Write improved tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml generic_page`
Expected: PASS

- [ ] **Commit**

```bash
git add src-tauri/src/adapters/generic_page.rs
git commit -m "feat: improve generic page adapter with semantic diffing"
```

---

### Phase 3.2: Creator/Domain Learning

**Files:**
- Create: `src-tauri/src/services/source_learning.rs`

- [ ] **Step 1: Create source learning service**

`src-tauri/src/services/source_learning.rs`:
```rust
pub struct SourceLearning;

impl SourceLearning {
    pub fn record_confirmation(
        conn: &Connection,
        local_mod_id: &str,
        source_url: &str,
        source_kind: SourceKind,
    ) -> AppResult<()> {
        // Track (creator_domain, source_kind) -> confirmed
        // Store in a learning table
    }
    
    pub fn record_rejection(
        conn: &Connection,
        source_url: &str,
    ) -> AppResult<()> {
        // Track rejected source URLs
    }
    
    pub fn get_learned_domains(
        conn: &Connection,
    ) -> AppResult<HashMap<String, SourceKind>> {
        // Return known creator domains and their source kind
    }
    
    pub fn boost_candidate_score(
        candidate: &mut CandidateSource,
        learned_domains: &HashMap<String, SourceKind>,
    ) {
        // If source URL domain is known good, boost score
    }
}
```

- [ ] **Step 2: Add learning tables to schema**

Create `database/migrations/0003_source_learning.sql`:
```sql
CREATE TABLE source_learning (
  id TEXT PRIMARY KEY,
  domain TEXT NOT NULL UNIQUE,
  source_kind TEXT NOT NULL,
  confirm_count INTEGER NOT NULL DEFAULT 0,
  reject_count INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
```

- [ ] **Step 3: Write tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml source_learning`
Expected: PASS

- [ ] **Commit**

```bash
git add src-tauri/src/services/source_learning.rs database/migrations/0003_source_learning.sql
git commit -m "feat: add creator/domain learning for better candidate scoring"
```

---

### Phase 3.3: Rate Limiter and Scheduler

**Files:**
- Create: `src-tauri/src/services/rate_limiter.rs`
- Create: `src-tauri/src/services/scheduler.rs`
- Modify: `src-tauri/src/core/watch_polling/mod.rs`

- [ ] **Step 1: Create rate limiter**

`src-tauri/src/services/rate_limiter.rs`:
```rust
pub struct DomainRateLimiter {
    domains: HashMap<String, Instant>,
    min_interval: Duration,
}

impl DomainRateLimiter {
    pub fn can_fetch(&self, url: &Url) -> bool {
        // Check if enough time passed for this domain
    }
    
    pub fn record_fetch(&mut self, url: &Url) {
        // Record fetch time for domain
    }
    
    pub fn wait_time(&self, url: &Url) -> Duration {
        // Return how long until next fetch allowed
    }
}
```

- [ ] **Step 2: Create scheduler**

`src-tauri/src/services/scheduler.rs`:
```rust
pub struct UpdateScheduler {
    source_intervals: HashMap<SourceKind, Duration>,
}

impl UpdateScheduler {
    pub fn get_interval(&self, kind: SourceKind) -> Duration {
        match kind {
            SourceKind::CurseForge => Duration::hours(6),
            SourceKind::GitHub => Duration::hours(6),
            SourceKind::Nexus => Duration::hours(6),
            SourceKind::Feed => Duration::hours(12),
            SourceKind::StructuredPage => Duration::hours(24),
            SourceKind::GenericPage => Duration::hours(48),
        }
    }
    
    pub fn is_due(&self, binding: &SourceBinding, last_check: Option<&str>) -> bool {
        // Check if enough time passed since last_check
    }
}
```

- [ ] **Step 3: Integrate into polling**

Modify watch_polling to use new rate limiter and scheduler

- [ ] **Step 4: Write tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml rate_limiter`
Expected: PASS

- [ ] **Commit**

```bash
git add src-tauri/src/services/rate_limiter.rs src-tauri/src/services/scheduler.rs
git commit -m "feat: add domain rate limiter and per-source scheduler"
```

---

## Frontend Integration (Updates Screen)

**Files:**
- Modify: `src/lib/api.ts` (extend existing)
- Modify: `src/lib/types.ts` (add new types)
- Modify: `src/screens/UpdatesScreen.tsx`

### Updates to Frontend

- [ ] **Step 1: Add new types to types.ts**

Add to `src/lib/types.ts`:
```typescript
export interface LocalMod {
  id: string;
  display_name: string;
  normalized_name: string;
  creator_name?: string;
  category?: string;
  tracking_mode: 'auto' | 'manual' | 'detected_only' | 'ignored';
  source_confidence: number;
  current_status: 'up_to_date' | 'confirmed_update' | 'probable_update' | 'source_activity' | 'source_unreachable' | 'untracked';
  last_checked_at?: string;
}

export interface CandidateSource {
  id: string;
  local_mod_id: string;
  source_kind: 'curseforge' | 'github' | 'nexus' | 'feed' | 'structured_page' | 'generic_page';
  source_url: string;
  confidence_score: number;
  reasoning: string[];
  status: 'suggested' | 'confirmed' | 'rejected';
}

export interface UpdateEvent {
  id: string;
  local_mod_id: string;
  event_type: 'confirmed_update' | 'probable_update' | 'source_activity';
  confidence_score: number;
  summary: string;
  latest_version_text?: string;
  is_read: boolean;
  created_at: string;
}
```

- [ ] **Step 2: Extend API layer**

In `src/lib/api.ts`, add to the `api` object (following existing pattern around line 5609):
```typescript
scanLocalMods: () => invoke<ScanSummary>("scan_local_mods"),
getLocalMods: (filters?: LibraryQuery) => invoke<LocalMod[]>("get_local_mods", { filters }),
getCandidatesForMod: (modId: string) => invoke<CandidateSource[]>("get_candidates_for_mod", { modId }),
confirmCandidateSource: (candidateId: string) => invoke<SourceBinding>("confirm_candidate_source", { candidateId }),
rejectCandidateSource: (candidateId: string) => invoke<void>("reject_candidate_source", { candidateId }),
getUpdateEvents: () => invoke<UpdateEvent[]>("get_update_events"),
markEventRead: (eventId: string) => invoke<void>("mark_event_read", { eventId }),
dismissEvent: (eventId: string) => invoke<void>("dismiss_event", { eventId }),
```

- [ ] **Step 3: Update UpdatesScreen**

Modify `src/screens/UpdatesScreen.tsx`:
- Add "Candidate" lane for unconfirmed sources
- Show confidence scores in UI
- Add confirm/reject buttons
- Show "probable update" vs "confirmed update" badges
- Implement new notification preferences

- [ ] **Step 4: Write frontend tests**

Run: `npm run test:unit -- UpdatesScreen`
Expected: PASS

- [ ] **Commit**

```bash
git add src/lib/api.ts src/lib/types.ts src/screens/UpdatesScreen.tsx
git commit -m "feat: add candidate confirmation UI to Updates screen"
```

---

## Testing Strategy

### Backend Tests
```bash
# Unit tests per module
cargo test --manifest-path src-tauri/Cargo.toml adapters
cargo test --manifest-path src-tauri/Cargo.toml services
cargo test --manifest-path src-tauri/Cargo.toml update_decision

# Integration test
cargo test --manifest-path src-tauri/Cargo.toml --test watch_integration
```

### Frontend Tests
```bash
npm run test:unit -- UpdatesScreen
npm run build
```

### Smoke Tests
```bash
pwsh -NoProfile -File scripts/desktop/run-tauri-smoke.ps1
```

---

## File Summary

| New Files | Purpose |
|-----------|---------|
| `src-tauri/src/adapters/mod.rs` | Adapter trait + registry |
| `src-tauri/src/adapters/curseforge.rs` | CurseForge adapter |
| `src-tauri/src/adapters/github.rs` | GitHub adapter |
| `src-tauri/src/adapters/nexus.rs` | Nexus adapter |
| `src-tauri/src/adapters/feed.rs` | RSS/Atom feed adapter |
| `src-tauri/src/adapters/structured_page.rs` | Structured page adapter |
| `src-tauri/src/adapters/generic_page.rs` | Generic page fallback |
| `src-tauri/src/adapters/errors.rs` | Adapter error types |
| `src-tauri/src/services/mod.rs` | Services module entry |
| `src-tauri/src/services/local_inventory.rs` | Local mod inventory |
| `src-tauri/src/services/snapshot_store.rs` | Remote snapshot storage |
| `src-tauri/src/services/update_decision.rs` | Update detection logic |
| `src-tauri/src/services/candidate_scorer.rs` | Confidence scoring |
| `src-tauri/src/services/candidate_discovery.rs` | Multi-adapter discovery |
| `src-tauri/src/services/update_events.rs` | Event audit trail |
| `src-tauri/src/services/source_learning.rs` | Domain/creator learning |
| `src-tauri/src/services/rate_limiter.rs` | Per-domain rate limiting |
| `src-tauri/src/services/scheduler.rs` | Refresh interval scheduler |

| Modified Files | Change |
|----------------|--------|
| `src-tauri/src/database/mod.rs` | Add new tables inline |
| `src-tauri/src/models.rs` | Add SourceKind, TrackingMode, UpdateStatus, LocalMod, LocalFile, SourceBinding |
| `src-tauri/src/commands/mod.rs` | Add new commands |
| `src-tauri/src/core/scanner/mod.rs` | Integrate inventory |
| `src-tauri/src/core/watch_polling/mod.rs` | Use new adapters alongside old system |
| `src-tauri/src/core/mod.rs` | Add services module |
| `src/lib/types.ts` | Add LocalMod, CandidateSource, UpdateEvent types |
| `src/lib/api.ts` | Add new API calls |
| `src/screens/UpdatesScreen.tsx` | Add candidate UI |