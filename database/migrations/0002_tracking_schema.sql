-- Migration 0002: Non-CurseForge Mod Update Tracking Schema
-- Adds tables for discovering, tracking, and monitoring updates from various mod sources

PRAGMA foreign_keys = ON;

-- =============================================================================
-- LOCAL MODS: Tracks discovered mod folders in the user's Mods directory
-- =============================================================================
CREATE TABLE IF NOT EXISTS local_mods (
    id TEXT PRIMARY KEY,
    display_name TEXT NOT NULL,
    normalized_name TEXT NOT NULL,
    creator_name TEXT,
    category TEXT,
    local_root_path TEXT NOT NULL UNIQUE,
    tracking_mode TEXT NOT NULL DEFAULT 'detected_only' CHECK (tracking_mode IN ('detected_only', 'auto', 'manual', 'ignored')),
    source_confidence REAL NOT NULL DEFAULT 0.0,
    confirmed_source_id TEXT,
    current_status TEXT NOT NULL DEFAULT 'untracked' CHECK (current_status IN ('untracked', 'up_to_date', 'confirmed_update', 'probable_update', 'source_activity', 'source_unreachable')),
    last_checked_at TEXT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_local_mods_normalized_name ON local_mods (normalized_name);
CREATE INDEX IF NOT EXISTS idx_local_mods_tracking_mode ON local_mods (tracking_mode);
CREATE INDEX IF NOT EXISTS idx_local_mods_current_status ON local_mods (current_status);
CREATE INDEX IF NOT EXISTS idx_local_mods_confirmed_source_id ON local_mods (confirmed_source_id);

-- =============================================================================
-- LOCAL FILES: Tracks individual .package and .ts4script files within mod folders
-- =============================================================================
CREATE TABLE IF NOT EXISTS local_files (
    id TEXT PRIMARY KEY,
    local_mod_id TEXT NOT NULL REFERENCES local_mods(id) ON DELETE CASCADE,
    file_path TEXT NOT NULL UNIQUE,
    file_name TEXT NOT NULL,
    file_ext TEXT NOT NULL,
    file_size INTEGER NOT NULL DEFAULT 0,
    sha256 TEXT,
    modified_at TEXT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_local_files_local_mod_id ON local_files (local_mod_id);
CREATE INDEX IF NOT EXISTS idx_local_files_sha256 ON local_files (sha256);
CREATE INDEX IF NOT EXISTS idx_local_files_file_name ON local_files (file_name);

-- =============================================================================
-- SOURCE BINDINGS: Confirmed source URLs linked to local mods
-- =============================================================================
CREATE TABLE IF NOT EXISTS source_bindings (
    id TEXT PRIMARY KEY,
    local_mod_id TEXT NOT NULL REFERENCES local_mods(id) ON DELETE CASCADE,
    source_kind TEXT NOT NULL CHECK (source_kind IN ('curseforge', 'github', 'nexus', 'feed', 'structured_page', 'generic_page')),
    source_url TEXT NOT NULL,
    provider_mod_id TEXT,
    provider_file_id TEXT,
    provider_repo TEXT,
    bind_method TEXT NOT NULL DEFAULT 'manual',
    is_primary INTEGER NOT NULL DEFAULT 1 CHECK (is_primary IN (0, 1)),
    custom_headers_json TEXT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_source_bindings_local_mod_id ON source_bindings (local_mod_id);
CREATE INDEX IF NOT EXISTS idx_source_bindings_source_kind ON source_bindings (source_kind);
CREATE INDEX IF NOT EXISTS idx_source_bindings_provider_mod_id ON source_bindings (provider_mod_id);
CREATE INDEX IF NOT EXISTS idx_source_bindings_provider_repo ON source_bindings (provider_repo);

-- =============================================================================
-- CANDIDATE SOURCES: Discovered but not yet confirmed source candidates
-- =============================================================================
CREATE TABLE IF NOT EXISTS candidate_sources (
    id TEXT PRIMARY KEY,
    local_mod_id TEXT NOT NULL REFERENCES local_mods(id) ON DELETE CASCADE,
    source_kind TEXT NOT NULL CHECK (source_kind IN ('curseforge', 'github', 'nexus', 'feed', 'structured_page', 'generic_page')),
    source_url TEXT NOT NULL,
    provider_mod_id TEXT,
    provider_file_id TEXT,
    provider_repo TEXT,
    confidence_score REAL NOT NULL DEFAULT 0.0,
    reasoning_json TEXT NOT NULL DEFAULT '[]',
    status TEXT NOT NULL DEFAULT 'suggested' CHECK (status IN ('suggested', 'confirmed', 'rejected')),
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_candidate_sources_local_mod_id ON candidate_sources (local_mod_id);
CREATE INDEX IF NOT EXISTS idx_candidate_sources_status ON candidate_sources (status);
CREATE INDEX IF NOT EXISTS idx_candidate_sources_confidence ON candidate_sources (confidence_score DESC);
CREATE INDEX IF NOT EXISTS idx_candidate_sources_provider_mod_id ON candidate_sources (provider_mod_id);

-- =============================================================================
-- REMOTE SNAPSHOTS: Stores historical state of remote sources for change detection
-- This enables detecting updates by comparing current vs previous snapshots
-- =============================================================================
CREATE TABLE IF NOT EXISTS remote_snapshots (
    id TEXT PRIMARY KEY,
    binding_id TEXT NOT NULL REFERENCES source_bindings(id) ON DELETE CASCADE,
    snapshot_hash TEXT NOT NULL,
    title TEXT,
    version_text TEXT,
    published_at TEXT,
    download_url TEXT,
    changelog_url TEXT,
    release_id TEXT,
    asset_names_json TEXT NOT NULL DEFAULT '[]',
    image_hashes_json TEXT NOT NULL DEFAULT '[]',
    file_fingerprints_json TEXT NOT NULL DEFAULT '{}',
    raw_summary_json TEXT NOT NULL,
    etag TEXT,
    last_modified TEXT,
    fetched_at TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_remote_snapshots_binding ON remote_snapshots(binding_id);
CREATE INDEX IF NOT EXISTS idx_remote_snapshots_binding_fetched ON remote_snapshots(binding_id, fetched_at DESC);

-- =============================================================================
-- UPDATE EVENTS: Tracks update detection events for display to users
-- =============================================================================
CREATE TABLE IF NOT EXISTS update_events (
    id TEXT PRIMARY KEY,
    local_mod_id TEXT NOT NULL REFERENCES local_mods(id) ON DELETE CASCADE,
    binding_id TEXT REFERENCES source_bindings(id) ON DELETE SET NULL,
    event_type TEXT NOT NULL CHECK (event_type IN ('confirmed_update', 'probable_update', 'source_activity', 'source_unreachable', 'up_to_date', 'untracked')),
    confidence_score REAL NOT NULL DEFAULT 0.0,
    summary TEXT,
    latest_version_text TEXT,
    latest_published_at TEXT,
    is_read INTEGER NOT NULL DEFAULT 0 CHECK (is_read IN (0, 1)),
    is_dismissed INTEGER NOT NULL DEFAULT 0 CHECK (is_dismissed IN (0, 1)),
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_update_events_local_mod_id ON update_events (local_mod_id);
CREATE INDEX IF NOT EXISTS idx_update_events_is_read ON update_events (is_read);
CREATE INDEX IF NOT EXISTS idx_update_events_is_dismissed ON update_events (is_dismissed);
CREATE INDEX IF NOT EXISTS idx_update_events_created_at ON update_events (created_at DESC);
CREATE INDEX IF NOT EXISTS idx_update_events_unread ON update_events (is_read, is_dismissed, created_at DESC);

-- =============================================================================
-- USER TRACKING PREFS: Per-mod user preferences for tracking behavior
-- =============================================================================
CREATE TABLE IF NOT EXISTS user_tracking_prefs (
    local_mod_id TEXT PRIMARY KEY REFERENCES local_mods(id) ON DELETE CASCADE,
    ignore_updates INTEGER NOT NULL DEFAULT 0,
    ignore_versions_json TEXT NOT NULL DEFAULT '[]',
    notify_on_probable INTEGER NOT NULL DEFAULT 0,
    notify_on_source_activity INTEGER NOT NULL DEFAULT 0,
    manual_source_url TEXT,
    pinned_source_kind TEXT,
    custom_check_interval_hours INTEGER,
    fingerprint_enabled INTEGER NOT NULL DEFAULT 0,
    ea_broken_mods_enabled INTEGER NOT NULL DEFAULT 1,
    ea_broken_mods_custom_url TEXT,
    custom_headers_json TEXT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
