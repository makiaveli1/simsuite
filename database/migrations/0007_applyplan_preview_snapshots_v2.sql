-- ApplyPlan Preview Snapshot Identity V2
-- Stores backend-issued immutable preview snapshots so saved ApplyPlans can be
-- persisted from the exact backend preview the user reviewed instead of a later
-- regeneration.

CREATE TABLE IF NOT EXISTS apply_plan_preview_snapshots (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    snapshot_id TEXT NOT NULL UNIQUE,
    source_plan_kind TEXT NOT NULL DEFAULT 'backend_generated_sorting_preview',
    source_plan_json TEXT NOT NULL,
    source_scope_json TEXT,
    folder_config_json TEXT,
    context_trail_json TEXT NOT NULL DEFAULT '[]',
    preview_snapshot_hash TEXT NOT NULL,
    preview_snapshot_hash_version TEXT NOT NULL DEFAULT 'apply_plan_preview_snapshot_v2',
    preview_snapshot_hash_algorithm TEXT NOT NULL DEFAULT 'sha256',
    preview_snapshot_provenance_json TEXT NOT NULL,
    scan_session_id INTEGER REFERENCES scan_sessions(id) ON DELETE SET NULL,
    consumed_apply_plan_id INTEGER REFERENCES apply_plans(id) ON DELETE SET NULL,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    expires_at TEXT
);

CREATE INDEX IF NOT EXISTS idx_apply_plan_preview_snapshots_hash
    ON apply_plan_preview_snapshots(preview_snapshot_hash);

CREATE INDEX IF NOT EXISTS idx_apply_plan_preview_snapshots_created_at
    ON apply_plan_preview_snapshots(created_at);

ALTER TABLE apply_plans ADD COLUMN preview_snapshot_id INTEGER REFERENCES apply_plan_preview_snapshots(id) ON DELETE SET NULL;
ALTER TABLE apply_plans ADD COLUMN preview_snapshot_hash TEXT;

CREATE UNIQUE INDEX IF NOT EXISTS idx_apply_plans_preview_snapshot_id_unique
    ON apply_plans(preview_snapshot_id)
    WHERE preview_snapshot_id IS NOT NULL;

CREATE TRIGGER IF NOT EXISTS trg_apply_plan_preview_snapshots_identity_immutable
BEFORE UPDATE OF snapshot_id, source_plan_kind, source_plan_json, source_scope_json, folder_config_json, context_trail_json, preview_snapshot_hash, preview_snapshot_hash_version, preview_snapshot_hash_algorithm, preview_snapshot_provenance_json, scan_session_id, created_at, expires_at
ON apply_plan_preview_snapshots
BEGIN
    SELECT RAISE(ABORT, 'apply_plan_preview_snapshots identity/provenance is immutable');
END;

CREATE TRIGGER IF NOT EXISTS trg_apply_plans_preview_snapshot_id_immutable
BEFORE UPDATE OF preview_snapshot_id ON apply_plans
WHEN OLD.preview_snapshot_id IS NOT NULL AND OLD.preview_snapshot_id IS NOT NEW.preview_snapshot_id
BEGIN
    SELECT RAISE(ABORT, 'apply_plans.preview_snapshot_id is immutable once set');
END;

CREATE TRIGGER IF NOT EXISTS trg_apply_plans_preview_snapshot_hash_immutable
BEFORE UPDATE OF preview_snapshot_hash ON apply_plans
WHEN OLD.preview_snapshot_hash IS NOT NULL AND OLD.preview_snapshot_hash IS NOT NEW.preview_snapshot_hash
BEGIN
    SELECT RAISE(ABORT, 'apply_plans.preview_snapshot_hash is immutable once set');
END;
