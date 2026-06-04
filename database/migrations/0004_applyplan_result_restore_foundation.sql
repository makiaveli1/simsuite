CREATE TABLE IF NOT EXISTS apply_plan_runs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    apply_plan_id INTEGER NOT NULL REFERENCES apply_plans (id) ON DELETE CASCADE,
    status TEXT NOT NULL DEFAULT 'draft_log' CHECK (status IN ('draft_log', 'confirmed', 'applying', 'applied', 'apply_failed', 'restored', 'restore_failed', 'blocked', 'cancelled')),
    backup_strategy TEXT NOT NULL DEFAULT 'copy_backup_first' CHECK (backup_strategy IN ('copy_backup_first', 'design_only')),
    confirmation_token TEXT,
    confirmed_at TEXT,
    started_at TEXT,
    finished_at TEXT,
    total_items INTEGER NOT NULL DEFAULT 0 CHECK (total_items >= 0),
    skipped_items INTEGER NOT NULL DEFAULT 0 CHECK (skipped_items >= 0),
    applied_items INTEGER NOT NULL DEFAULT 0 CHECK (applied_items >= 0),
    failed_items INTEGER NOT NULL DEFAULT 0 CHECK (failed_items >= 0),
    restored_items INTEGER NOT NULL DEFAULT 0 CHECK (restored_items >= 0),
    summary TEXT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_apply_plan_runs_plan_id ON apply_plan_runs (apply_plan_id);
CREATE INDEX IF NOT EXISTS idx_apply_plan_runs_status_created_at ON apply_plan_runs (status, created_at);

CREATE TABLE IF NOT EXISTS apply_plan_results (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    apply_plan_run_id INTEGER NOT NULL REFERENCES apply_plan_runs (id) ON DELETE CASCADE,
    apply_plan_id INTEGER NOT NULL REFERENCES apply_plans (id) ON DELETE CASCADE,
    apply_plan_item_id INTEGER REFERENCES apply_plan_items (id) ON DELETE SET NULL,
    operation_kind TEXT NOT NULL,
    result_status TEXT NOT NULL CHECK (result_status IN ('pending_log', 'skipped', 'blocked', 'failed_before_change', 'applied', 'failed_after_change', 'restored')),
    source_path_at_execution TEXT,
    destination_path_at_execution TEXT,
    backup_path TEXT,
    error_code TEXT,
    error_message TEXT,
    user_summary TEXT NOT NULL,
    started_at TEXT,
    finished_at TEXT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_apply_plan_results_run_id ON apply_plan_results (apply_plan_run_id);
CREATE INDEX IF NOT EXISTS idx_apply_plan_results_plan_item_id ON apply_plan_results (apply_plan_item_id);
CREATE INDEX IF NOT EXISTS idx_apply_plan_results_status ON apply_plan_results (result_status);

CREATE TABLE IF NOT EXISTS apply_plan_restore_entries (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    apply_plan_run_id INTEGER NOT NULL REFERENCES apply_plan_runs (id) ON DELETE CASCADE,
    apply_plan_result_id INTEGER REFERENCES apply_plan_results (id) ON DELETE SET NULL,
    apply_plan_id INTEGER NOT NULL REFERENCES apply_plans (id) ON DELETE CASCADE,
    apply_plan_item_id INTEGER REFERENCES apply_plan_items (id) ON DELETE SET NULL,
    original_source_path TEXT NOT NULL,
    destination_path_at_execution TEXT,
    backup_path TEXT,
    file_hash_before TEXT,
    file_size_before INTEGER CHECK (file_size_before IS NULL OR file_size_before >= 0),
    operation_kind TEXT NOT NULL,
    operation_result_status TEXT NOT NULL CHECK (operation_result_status IN ('pending_log', 'skipped', 'blocked', 'failed_before_change', 'applied', 'failed_after_change', 'restored')),
    restore_status TEXT NOT NULL CHECK (restore_status IN ('not_available', 'design_only', 'not_restored', 'restored', 'restore_failed')),
    restore_error_code TEXT,
    restore_error_message TEXT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    restored_at TEXT,
    failed_at TEXT
);

CREATE INDEX IF NOT EXISTS idx_apply_plan_restore_entries_run_id ON apply_plan_restore_entries (apply_plan_run_id);
CREATE INDEX IF NOT EXISTS idx_apply_plan_restore_entries_result_id ON apply_plan_restore_entries (apply_plan_result_id);
CREATE INDEX IF NOT EXISTS idx_apply_plan_restore_entries_item_id ON apply_plan_restore_entries (apply_plan_item_id);
CREATE INDEX IF NOT EXISTS idx_apply_plan_restore_entries_status ON apply_plan_restore_entries (restore_status);
