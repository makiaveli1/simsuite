PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS schema_migrations (
  version INTEGER PRIMARY KEY,
  name TEXT NOT NULL,
  applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS scan_sessions (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  scan_type TEXT NOT NULL CHECK (scan_type IN ('full', 'incremental')),
  started_at TEXT NOT NULL,
  completed_at TEXT,
  files_scanned INTEGER NOT NULL DEFAULT 0,
  errors TEXT
);

CREATE TABLE IF NOT EXISTS bundles (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  bundle_type TEXT NOT NULL,
  bundle_name TEXT NOT NULL,
  file_count INTEGER NOT NULL DEFAULT 0,
  confidence REAL NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS creators (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  canonical_name TEXT NOT NULL UNIQUE,
  notes TEXT,
  locked_by_user INTEGER NOT NULL DEFAULT 0 CHECK (locked_by_user IN (0, 1)),
  created_by_user INTEGER NOT NULL DEFAULT 0 CHECK (created_by_user IN (0, 1)),
  preferred_path TEXT
);

CREATE TABLE IF NOT EXISTS creator_aliases (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  creator_id INTEGER NOT NULL REFERENCES creators (id) ON DELETE CASCADE,
  alias_name TEXT NOT NULL UNIQUE
);

CREATE TABLE IF NOT EXISTS user_creator_aliases (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  creator_id INTEGER NOT NULL REFERENCES creators (id) ON DELETE CASCADE,
  alias_name TEXT NOT NULL UNIQUE,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_user_creator_aliases_creator_id ON user_creator_aliases (creator_id);

CREATE TABLE IF NOT EXISTS user_category_overrides (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  match_path TEXT NOT NULL UNIQUE,
  kind TEXT NOT NULL,
  subtype TEXT,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_user_category_overrides_kind ON user_category_overrides (kind);

CREATE TABLE IF NOT EXISTS files (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  path TEXT NOT NULL UNIQUE,
  filename TEXT NOT NULL,
  extension TEXT NOT NULL,
  hash TEXT,
  size INTEGER NOT NULL DEFAULT 0,
  created_at TEXT,
  modified_at TEXT,
  bundle_id INTEGER REFERENCES bundles (id) ON DELETE SET NULL,
  creator_id INTEGER REFERENCES creators (id) ON DELETE SET NULL,
  kind TEXT NOT NULL DEFAULT 'Unknown',
  subtype TEXT,
  confidence REAL NOT NULL DEFAULT 0,
  source_location TEXT NOT NULL CHECK (source_location IN ('mods', 'tray', 'downloads', 'unknown')),
  download_item_id INTEGER REFERENCES download_items (id) ON DELETE SET NULL,
  source_origin_path TEXT,
  archive_member_path TEXT,
  scan_session_id INTEGER REFERENCES scan_sessions (id) ON DELETE SET NULL,
  relative_depth INTEGER NOT NULL DEFAULT 0,
  safety_notes TEXT NOT NULL DEFAULT '[]',
  parser_warnings TEXT NOT NULL DEFAULT '[]',
  insights TEXT NOT NULL DEFAULT '{}',
  indexed_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_files_hash ON files (hash);
CREATE INDEX IF NOT EXISTS idx_files_filename ON files (filename);
CREATE INDEX IF NOT EXISTS idx_files_creator_id ON files (creator_id);
CREATE INDEX IF NOT EXISTS idx_files_bundle_id ON files (bundle_id);
CREATE INDEX IF NOT EXISTS idx_files_kind ON files (kind);
CREATE INDEX IF NOT EXISTS idx_files_source_location ON files (source_location);
CREATE INDEX IF NOT EXISTS idx_files_download_item_id ON files (download_item_id);

CREATE TABLE IF NOT EXISTS download_items (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  source_path TEXT NOT NULL UNIQUE,
  display_name TEXT NOT NULL,
  source_kind TEXT NOT NULL CHECK (source_kind IN ('file', 'archive')),
  archive_format TEXT,
  staging_path TEXT,
  source_size INTEGER NOT NULL DEFAULT 0,
  source_modified_at TEXT,
  detected_file_count INTEGER NOT NULL DEFAULT 0,
  status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'ready', 'needs_review', 'partial', 'applied', 'ignored', 'error')),
  error_message TEXT,
  notes TEXT NOT NULL DEFAULT '[]',
  first_seen_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  last_seen_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_download_items_status ON download_items (status);

CREATE TABLE IF NOT EXISTS rules (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  rule_name TEXT NOT NULL UNIQUE,
  rule_template TEXT NOT NULL,
  rule_priority INTEGER NOT NULL DEFAULT 100,
  enabled INTEGER NOT NULL DEFAULT 1 CHECK (enabled IN (0, 1)),
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS duplicates (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  file_id_a INTEGER NOT NULL REFERENCES files (id) ON DELETE CASCADE,
  file_id_b INTEGER NOT NULL REFERENCES files (id) ON DELETE CASCADE,
  duplicate_type TEXT NOT NULL CHECK (duplicate_type IN ('exact', 'filename', 'version')),
  detection_method TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS review_queue (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  file_id INTEGER NOT NULL REFERENCES files (id) ON DELETE CASCADE,
  reason TEXT NOT NULL,
  suggested_path TEXT,
  confidence REAL NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  UNIQUE (file_id, reason)
);

CREATE TABLE IF NOT EXISTS snapshots (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  snapshot_name TEXT NOT NULL,
  description TEXT,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS snapshot_items (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  snapshot_id INTEGER NOT NULL REFERENCES snapshots (id) ON DELETE CASCADE,
  file_id INTEGER REFERENCES files (id) ON DELETE SET NULL,
  original_path TEXT NOT NULL,
  original_hash TEXT
);

CREATE TABLE IF NOT EXISTS app_settings (
  key TEXT PRIMARY KEY,
  value TEXT NOT NULL,
  source TEXT NOT NULL DEFAULT 'seed' CHECK (source IN ('seed', 'user')),
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS seed_meta (
  key TEXT PRIMARY KEY,
  value TEXT NOT NULL
);
