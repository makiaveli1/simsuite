CREATE TABLE IF NOT EXISTS apply_plans (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    source_staging_plan_id TEXT,
    source_plan_kind TEXT NOT NULL,
    title TEXT NOT NULL,
    summary TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'draft' CHECK (status IN ('draft', 'preview_only_source', 'blocked', 'cancelled')),
    would_touch_files INTEGER NOT NULL DEFAULT 0 CHECK (would_touch_files IN (0, 1)),
    confirmation_required INTEGER NOT NULL DEFAULT 1 CHECK (confirmation_required IN (0, 1)),
    backup_required INTEGER NOT NULL DEFAULT 1 CHECK (backup_required IN (0, 1)),
    restore_available INTEGER NOT NULL DEFAULT 0 CHECK (restore_available IN (0, 1)),
    total_items INTEGER NOT NULL DEFAULT 0,
    applyable_items INTEGER NOT NULL DEFAULT 0,
    blocked_items INTEGER NOT NULL DEFAULT 0,
    review_only_items INTEGER NOT NULL DEFAULT 0,
    caveats_json TEXT NOT NULL DEFAULT '[]',
    source_scope_json TEXT,
    scan_session_id INTEGER REFERENCES scan_sessions (id) ON DELETE SET NULL,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_apply_plans_status_created_at ON apply_plans (status, created_at);
CREATE INDEX IF NOT EXISTS idx_apply_plans_source_staging_plan_id ON apply_plans (source_staging_plan_id);

CREATE TABLE IF NOT EXISTS apply_plan_items (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    apply_plan_id INTEGER NOT NULL REFERENCES apply_plans (id) ON DELETE CASCADE,
    source_item_id TEXT,
  file_id INTEGER,
    file_name TEXT NOT NULL,
    current_path TEXT NOT NULL,
    current_root TEXT NOT NULL,
    destination_path TEXT,
    destination_root TEXT,
    action_kind TEXT NOT NULL,
    evidence_level TEXT NOT NULL,
    bucket TEXT,
    confidence_label TEXT,
    item_status TEXT NOT NULL DEFAULT 'preview_only' CHECK (item_status IN ('preview_only', 'blocked', 'review_only', 'draft_candidate')),
    blocked INTEGER NOT NULL DEFAULT 0 CHECK (blocked IN (0, 1)),
    review_only INTEGER NOT NULL DEFAULT 0 CHECK (review_only IN (0, 1)),
    validation_status TEXT,
    conflict_status TEXT,
    path_privacy_level TEXT NOT NULL DEFAULT 'local_full_path_required',
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_apply_plan_items_plan_id ON apply_plan_items (apply_plan_id);
CREATE INDEX IF NOT EXISTS idx_apply_plan_items_file_id ON apply_plan_items (file_id);
CREATE INDEX IF NOT EXISTS idx_apply_plan_items_status ON apply_plan_items (item_status);

CREATE TABLE IF NOT EXISTS apply_plan_item_signals (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    apply_plan_item_id INTEGER NOT NULL REFERENCES apply_plan_items (id) ON DELETE CASCADE,
    signal_kind TEXT NOT NULL,
    signal_label TEXT NOT NULL,
    signal_value TEXT,
    evidence_level TEXT,
    source_system TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_apply_plan_item_signals_item_id ON apply_plan_item_signals (apply_plan_item_id);
CREATE INDEX IF NOT EXISTS idx_apply_plan_item_signals_kind ON apply_plan_item_signals (signal_kind);

CREATE TABLE IF NOT EXISTS apply_plan_item_blockers (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    apply_plan_item_id INTEGER NOT NULL REFERENCES apply_plan_items (id) ON DELETE CASCADE,
    blocker_kind TEXT NOT NULL,
    reason_code TEXT NOT NULL,
    message TEXT NOT NULL,
    source_system TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_apply_plan_item_blockers_item_id ON apply_plan_item_blockers (apply_plan_item_id);
CREATE INDEX IF NOT EXISTS idx_apply_plan_item_blockers_reason_code ON apply_plan_item_blockers (reason_code);
