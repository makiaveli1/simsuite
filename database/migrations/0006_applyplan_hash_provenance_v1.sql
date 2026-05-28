ALTER TABLE apply_plans ADD COLUMN plan_hash TEXT;
ALTER TABLE apply_plans ADD COLUMN plan_hash_version TEXT NOT NULL DEFAULT 'apply_plan_hash_v1';
ALTER TABLE apply_plans ADD COLUMN plan_hash_algorithm TEXT NOT NULL DEFAULT 'sha256';
ALTER TABLE apply_plans ADD COLUMN plan_hash_created_at TEXT;
ALTER TABLE apply_plans ADD COLUMN plan_provenance_json TEXT NOT NULL DEFAULT '{}';

CREATE INDEX IF NOT EXISTS idx_apply_plans_plan_hash ON apply_plans (plan_hash);

CREATE TRIGGER IF NOT EXISTS trg_apply_plans_plan_hash_immutable
BEFORE UPDATE OF plan_hash ON apply_plans
WHEN OLD.plan_hash IS NOT NULL AND OLD.plan_hash IS NOT NEW.plan_hash
BEGIN
    SELECT RAISE(ABORT, 'apply_plans.plan_hash is immutable once set');
END;

CREATE TRIGGER IF NOT EXISTS trg_apply_plans_plan_hash_version_immutable
BEFORE UPDATE OF plan_hash_version ON apply_plans
WHEN OLD.plan_hash_version IS NOT NEW.plan_hash_version
BEGIN
    SELECT RAISE(ABORT, 'apply_plans.plan_hash_version is immutable');
END;

CREATE TRIGGER IF NOT EXISTS trg_apply_plans_plan_hash_algorithm_immutable
BEFORE UPDATE OF plan_hash_algorithm ON apply_plans
WHEN OLD.plan_hash_algorithm IS NOT NEW.plan_hash_algorithm
BEGIN
    SELECT RAISE(ABORT, 'apply_plans.plan_hash_algorithm is immutable');
END;

CREATE TRIGGER IF NOT EXISTS trg_apply_plans_plan_hash_created_at_immutable
BEFORE UPDATE OF plan_hash_created_at ON apply_plans
WHEN OLD.plan_hash_created_at IS NOT NULL AND OLD.plan_hash_created_at IS NOT NEW.plan_hash_created_at
BEGIN
    SELECT RAISE(ABORT, 'apply_plans.plan_hash_created_at is immutable once set');
END;

CREATE TRIGGER IF NOT EXISTS trg_apply_plans_plan_provenance_immutable
BEFORE UPDATE OF plan_provenance_json ON apply_plans
WHEN OLD.plan_provenance_json != '{}' AND OLD.plan_provenance_json IS NOT NEW.plan_provenance_json
BEGIN
    SELECT RAISE(ABORT, 'apply_plans.plan_provenance_json is immutable once set');
END;
