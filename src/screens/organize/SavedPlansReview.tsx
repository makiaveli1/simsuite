import { useCallback, useEffect, useMemo, useState } from "react";
import {
  AlertCircle,
  ArchiveX,
  CheckCircle2,
  FileCheck2,
  FolderOpen,
  Info,
  ListChecks,
  LoaderCircle,
  ShieldCheck,
} from "lucide-react";
import { api } from "../../lib/api";
import type {
  ApplyPlanListItem,
  ApplyPlanConflictStatus,
  ApplyPlanValidationItem,
  ApplyPlanValidationPreview,
  ApplyPlanValidationPreviewStatus,
  ApplyPlanValidationStatus,
  PersistedApplyPlan,
  PersistedApplyPlanBlocker,
  PersistedApplyPlanItem,
  PersistedApplyPlanSignal,
  PersistedApplyPlanStatus,
} from "../../lib/types";

interface SavedPlansReviewProps {
  refreshVersion: number;
  selectedPlanId: number | null;
  statusMessage: string | null;
  onCreatePlan: () => void;
}

const ITEM_STATUS_LABELS: Record<string, string> = {
  preview_only: "Preview only",
  blocked: "Blocked",
  review_only: "Review-only",
  draft_candidate: "Draft candidate",
};

const PLAN_STATUS_LABELS: Record<PersistedApplyPlanStatus, string> = {
  draft: "Draft preview plan",
  preview_only_source: "Preview-only source",
  blocked: "Blocked",
  cancelled: "Cancelled",
};

const VALIDATION_STATUS_LABELS: Record<ApplyPlanValidationStatus, string> = {
  not_validated: "Needs validation",
  valid_preview_only: "No current blocker found",
  blocked: "Blocked",
  stale_source: "Stale source",
  missing_source: "Missing source",
  missing_destination_root: "Missing destination root",
  unsafe_destination: "Unsafe destination",
  destination_exists: "Destination exists",
  unsupported_cross_root: "Unsupported root move",
  review_only_blocked: "Review-only",
  duplicate_review_blocked: "Duplicate review",
  backup_required: "Backup required",
  error: "Could not validate",
};

const CONFLICT_STATUS_LABELS: Record<ApplyPlanConflictStatus, string> = {
  not_checked: "Not checked",
  none: "No conflict found",
  destination_exists: "Destination exists",
  same_name_conflict: "Same name conflict",
  case_conflict: "Case conflict",
  folder_missing: "Folder missing",
  permission_unknown: "Permission unknown",
  source_missing: "Source missing",
  path_too_long: "Path too long",
  cross_root_blocked: "Cross-root blocked",
  unsupported: "Unsupported",
};

const PLAN_VALIDATION_LABELS: Record<ApplyPlanValidationPreviewStatus, string> = {
  not_validated: "Needs validation",
  valid_preview_only: "No current blocker found, but still preview-only",
  blocked: "Blocked",
};

function formatDateTime(value: string): string {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return new Intl.DateTimeFormat("en-US", {
    dateStyle: "medium",
    timeStyle: "short",
  }).format(date);
}

function formatCount(value: number, singular: string, plural = `${singular}s`) {
  return `${new Intl.NumberFormat("en-US").format(value)} ${
    value === 1 ? singular : plural
  }`;
}

function formatPlanStatus(status: PersistedApplyPlanStatus): string {
  return PLAN_STATUS_LABELS[status] ?? "Draft preview plan";
}

function formatItemStatus(status: string): string {
  return ITEM_STATUS_LABELS[status] ?? status.replace(/_/g, " ");
}

function formatScope(scope: Record<string, unknown> | null): string {
  if (!scope) return "Source scope unavailable";

  if (scope.kind === "selected_files") {
    const fileIds = Array.isArray(scope.fileIds) ? scope.fileIds : [];
    return `${formatCount(fileIds.length, "selected Library file")}`;
  }

  if (scope.kind === "library_folder") {
    const sourceLocation =
      typeof scope.sourceLocation === "string" ? scope.sourceLocation : "Library";
    const folderPath =
      typeof scope.folderPath === "string" && scope.folderPath.trim()
        ? scope.folderPath
        : "root";
    const recursive = scope.recursive === false ? "top level only" : "including nested folders";
    const limit =
      typeof scope.limit === "number" ? `, preview limit ${scope.limit}` : "";
    return `${sourceLocation} ${folderPath} (${recursive}${limit})`;
  }

  return "Saved preview source";
}

function shortenPath(path: string | null): string {
  if (!path) return "Path unavailable";
  if (path.length <= 76) return path;

  const parts = path.split(/[\\/]+/).filter(Boolean);
  if (parts.length <= 3) return path;
  return `...\\${parts.slice(-3).join("\\")}`;
}

function groupSavedItems(items: PersistedApplyPlanItem[]) {
  const grouped = new Map<string, PersistedApplyPlanItem[]>();
  for (const item of items) {
    const key = item.bucket || item.itemStatus;
    grouped.set(key, [...(grouped.get(key) ?? []), item]);
  }
  return [...grouped.entries()].map(([key, groupItems]) => ({
    key,
    label: key.replace(/_/g, " "),
    items: groupItems,
  }));
}

function signalLabel(signal: PersistedApplyPlanSignal): string {
  return signal.signalLabel || signal.signalValue || signal.signalKind;
}

function blockerLabel(blocker: PersistedApplyPlanBlocker): string {
  return blocker.message || blocker.reasonCode || blocker.blockerKind;
}

function formatValidationStatus(status: ApplyPlanValidationStatus): string {
  return VALIDATION_STATUS_LABELS[status] ?? status.replace(/_/g, " ");
}

function formatConflictStatus(status: ApplyPlanConflictStatus): string {
  return CONFLICT_STATUS_LABELS[status] ?? status.replace(/_/g, " ");
}

function formatPlanValidationStatus(status: ApplyPlanValidationPreviewStatus): string {
  return PLAN_VALIDATION_LABELS[status] ?? status.replace(/_/g, " ");
}

function SavedPlanSummaryRow({
  plan,
  selected,
  onReview,
}: {
  plan: ApplyPlanListItem;
  selected: boolean;
  onReview: () => void;
}) {
  return (
    <article className="pending-batch-row saved-plan-summary-row">
      <div className="pending-batch-main">
        <div className="pending-batch-heading">
          <FileCheck2 size={16} />
          <div>
            <h3>{plan.title || `Draft preview plan ${plan.id}`}</h3>
            <p>{plan.summary}</p>
          </div>
        </div>
        <span className="staging-card-badge staging-card-badge--pending">
          Preview only
        </span>
      </div>

      <div className="pending-batch-metrics" aria-label={`${plan.title} summary`}>
        <span>
          <strong>{formatPlanStatus(plan.status)}</strong>
          <small>Status</small>
        </span>
        <span>
          <strong>{plan.totalItems}</strong>
          <small>Items</small>
        </span>
        <span>
          <strong>{plan.blockedItems}</strong>
          <small>Blocked</small>
        </span>
      </div>

      <div className="pending-batch-next">
        <strong>No files changed</strong>
        <span>
          Updated {formatDateTime(plan.updatedAt)}. This is a saved draft
          preview record, not a file-changing plan.
        </span>
      </div>

      <div className="pending-plans-actions">
        <button
          type="button"
          className={selected ? "primary-action" : "secondary-action"}
          onClick={onReview}
        >
          <ListChecks size={16} />
          {selected ? "Reviewing details" : "Review details"}
        </button>
      </div>
    </article>
  );
}

function SavedPlanItemCard({
  item,
  index,
}: {
  item: PersistedApplyPlanItem;
  index: number;
}) {
  const hasSignals = item.signals.length > 0;
  const hasBlockers = item.blockers.length > 0;

  return (
    <article className="organize-plan-item" aria-label={`Saved plan item ${index + 1}`}>
      <div className="organize-plan-item-header">
        <div className="organize-plan-item-title">
          <span>{item.fileName}</span>
          <small>{item.currentRoot || "Unknown"} source</small>
        </div>
        <div className="organize-plan-status-row" aria-label="Saved plan item labels">
          <span className="organize-plan-status-chip">
            {formatItemStatus(item.itemStatus)}
          </span>
          <span className="organize-plan-status-chip">{item.evidenceLevel}</span>
          {item.confidenceLabel ? (
            <span className="organize-plan-status-chip">{item.confidenceLabel}</span>
          ) : null}
        </div>
      </div>

      <div className="organize-plan-path-grid">
        <div className="organize-plan-path-card">
          <span>Current path</span>
          <code title={item.currentPath ?? undefined}>{shortenPath(item.currentPath)}</code>
        </div>
        <div className="organize-plan-path-card">
          <span>Suggested destination</span>
          <code title={item.destinationPath ?? undefined}>
            {shortenPath(item.destinationPath)}
          </code>
        </div>
      </div>

      {hasSignals ? (
        <div className="organize-plan-detail-block">
          <h4>Source signals</h4>
          <div className="organize-plan-tags">
            {item.signals.map((signal) => (
              <span key={signal.id} className="organize-plan-tag">
                {signalLabel(signal)}
              </span>
            ))}
          </div>
        </div>
      ) : null}

      {hasBlockers ? (
        <div className="organize-plan-detail-block">
          <h4>Blockers</h4>
          <div className="organize-plan-tags">
            {item.blockers.map((blocker) => (
              <span
                key={blocker.id}
                className="organize-plan-tag organize-plan-tag--blocked"
              >
                {blockerLabel(blocker)}
              </span>
            ))}
          </div>
        </div>
      ) : null}

      <details>
        <summary>Technical details</summary>
        <div className="pending-batch-technical">
          <div>
            <strong>Current path</strong>
            <code>{item.currentPath ?? "Path unavailable"}</code>
          </div>
          <div>
            <strong>Destination preview</strong>
            <code>{item.destinationPath ?? "No destination preview"}</code>
          </div>
          <div>
            <strong>Source item</strong>
            <code>{item.sourceItemId ?? "No source item id"}</code>
          </div>
        </div>
      </details>
    </article>
  );
}

function ValidationSummaryGrid({
  preview,
}: {
  preview: ApplyPlanValidationPreview;
}) {
  const summary = preview.summary;
  const counts = [
    ["Total items", summary.totalItems],
    ["Blocked", summary.blockedItems],
    ["Needs review", summary.reviewOnlyItems],
    ["Conflicts", summary.conflictItems],
    ["Stale paths", summary.staleItems],
    ["Missing files", summary.missingSourceItems],
    ["Destination conflicts", summary.destinationConflictItems],
    ["Backup required", summary.backupBlockedItems],
  ] as const;

  return (
    <div className="validation-preview-grid" aria-label="Validation summary counts">
      {counts.map(([label, value]) => (
        <span key={label}>
          <strong>{value}</strong>
          <small>{label}</small>
        </span>
      ))}
    </div>
  );
}

function ValidationItemCard({
  item,
}: {
  item: ApplyPlanValidationItem;
}) {
  return (
    <article className="validation-preview-item" aria-label={`${item.fileName} validation result`}>
      <div className="validation-preview-item-header">
        <div className="organize-plan-item-title">
          <span>{item.fileName}</span>
          <small>Future confirmation blocked</small>
        </div>
        <div className="organize-plan-status-row" aria-label="Validation result labels">
          <span className="organize-plan-status-chip">
            {formatValidationStatus(item.validationStatus)}
          </span>
          <span className="organize-plan-status-chip">
            {formatConflictStatus(item.conflictStatus)}
          </span>
          {item.reviewOnly ? (
            <span className="organize-plan-tag organize-plan-tag--blocked">
              Manual review needed
            </span>
          ) : null}
          {item.blocked ? (
            <span className="organize-plan-tag organize-plan-tag--blocked">
              Blocked
            </span>
          ) : null}
        </div>
      </div>

      <div className="validation-preview-reasons">
        <div>
          <h5>Reasons</h5>
          {item.reasons.length > 0 ? (
            <ul>
              {item.reasons.map((reason) => (
                <li key={reason}>{reason}</li>
              ))}
            </ul>
          ) : (
            <p>No item-level validation reason was returned.</p>
          )}
        </div>
        <div>
          <h5>Required next steps</h5>
          {item.requiredNextSteps.length > 0 ? (
            <ul>
              {item.requiredNextSteps.map((step) => (
                <li key={step}>{step}</li>
              ))}
            </ul>
          ) : (
            <p>Future confirmation remains blocked.</p>
          )}
        </div>
      </div>
    </article>
  );
}

function ValidationPreviewSection({
  planId,
  preview,
  loading,
  error,
  onCheck,
}: {
  planId: number;
  preview: ApplyPlanValidationPreview | null;
  loading: boolean;
  error: string | null;
  onCheck: (planId: number) => void;
}) {
  return (
    <section className="validation-preview-panel" aria-label="Validation preview">
      <div className="validation-preview-header">
        <div>
          <div className="eyebrow">Validation preview</div>
          <h4>Check saved plan</h4>
          <p>
            Check whether this saved draft has missing files, stale paths,
            conflicts, or review-only blockers. No files changed.
          </p>
        </div>
        <button
          type="button"
          className="secondary-action"
          disabled={loading}
          onClick={() => onCheck(planId)}
        >
          {loading ? (
            <>
              <LoaderCircle size={16} className="spin" />
              Checking saved plan
            </>
          ) : (
            <>
              <ListChecks size={16} />
              Check saved plan
            </>
          )}
        </button>
      </div>

      <div className="validation-preview-safety-strip">
        <ShieldCheck size={16} />
        <strong>No files changed</strong>
        <span>
          This is a read-only validation preview. Future confirmation is blocked
          until backup/restore, confirmation, result logs, and proof exist.
        </span>
      </div>

      {!preview && !loading && !error ? (
        <div className="validation-preview-empty">
          <Info size={20} />
          <div>
            <strong>Needs validation</strong>
            <span>
              Run a preview check to see blockers, stale sources, missing files,
              destination conflicts, and backup requirements.
            </span>
          </div>
        </div>
      ) : null}

      {error ? (
        <div className="staging-result staging-result--warn" role="status">
          <AlertCircle size={16} />
          <span>
            Could not load validation preview: {error}. No files changed.
          </span>
        </div>
      ) : null}

      {preview ? (
        <div className="validation-preview-results">
          <div className="validation-preview-status-row">
            <span className="organize-plan-status-chip">
              {formatPlanValidationStatus(preview.status)}
            </span>
            <span className="organize-plan-tag organize-plan-tag--blocked">
              Future confirmation blocked
            </span>
            <span className="organize-plan-status-chip">
              Checked {formatDateTime(preview.checkedAt)}
            </span>
          </div>

          {preview.status === "valid_preview_only" ? (
            <p className="validation-preview-note">
              No current validation blocker was found, but Apply is still not
              available.
            </p>
          ) : null}

          <ValidationSummaryGrid preview={preview} />

          {preview.caveats.length > 0 ? (
            <div className="organize-plan-detail-block">
              <h4>Validation caveats</h4>
              <ul className="organize-plan-caveats">
                {preview.caveats.map((caveat) => (
                  <li key={caveat}>{caveat}</li>
                ))}
              </ul>
            </div>
          ) : null}

          <div className="validation-preview-item-list">
            {preview.items.map((item) => (
              <ValidationItemCard key={item.itemId} item={item} />
            ))}
          </div>

          <details>
            <summary>Technical details</summary>
            <div className="pending-batch-technical">
              <div>
                <strong>Plan id</strong>
                <code>{preview.planId}</code>
              </div>
              <div>
                <strong>Confirmation status</strong>
                <code>No. Future confirmation blocked.</code>
              </div>
            </div>
          </details>
        </div>
      ) : null}
    </section>
  );
}

function SavedPlanDetails({
  plan,
  loading,
  error,
  validationPreview,
  validationLoading,
  validationError,
  confirmingCancel,
  cancelling,
  onCheckValidation,
  onCancelDraft,
  onConfirmCancel,
  onKeepDraft,
}: {
  plan: PersistedApplyPlan | null;
  loading: boolean;
  error: string | null;
  validationPreview: ApplyPlanValidationPreview | null;
  validationLoading: boolean;
  validationError: string | null;
  confirmingCancel: boolean;
  cancelling: boolean;
  onCheckValidation: (planId: number) => void;
  onCancelDraft: () => void;
  onConfirmCancel: () => void;
  onKeepDraft: () => void;
}) {
  const groupedItems = useMemo(() => groupSavedItems(plan?.items ?? []), [plan]);

  if (loading) {
    return (
      <section className="organize-plan-empty" aria-live="polite">
        <LoaderCircle size={24} className="spin" />
        <h3>Loading saved plan</h3>
        <p>SimSuite is loading the draft preview record. No files changed.</p>
      </section>
    );
  }

  if (error) {
    return (
      <section className="organize-plan-empty organize-plan-empty--error">
        <AlertCircle size={24} />
        <h3>Could not load saved plan</h3>
        <p>{error} No files changed.</p>
      </section>
    );
  }

  if (!plan) {
    return (
      <section className="organize-plan-empty">
        <Info size={24} />
        <h3>Select a saved preview plan</h3>
        <p>
          Choose a draft from the list to review its evidence, blockers, and
          caveats. No files changed.
        </p>
      </section>
    );
  }

  return (
    <section className="organize-plan-results" aria-label="Saved plan details">
      <div className="organize-plan-summary">
        <div>
          <div className="eyebrow">Plan details</div>
          <h3>{plan.title}</h3>
          <p>{plan.summary}</p>
        </div>
        <div className="organize-plan-summary-grid">
          <span>
            <strong>{formatPlanStatus(plan.status)}</strong>
            <small>Status</small>
          </span>
          <span>
            <strong>{plan.totalItems}</strong>
            <small>Items</small>
          </span>
          <span>
            <strong>No</strong>
            <small>Files changed</small>
          </span>
        </div>
      </div>

      <div className="pending-plans-safety-strip" aria-label="Saved plan safety">
        <ShieldCheck size={16} />
        <strong>No files changed</strong>
        <span>
          This saved draft is for review only. Future validation, confirmation,
          backup, and result logging are still required before any file-changing
          workflow can exist.
        </span>
      </div>

      <div className="pending-plans-stats" aria-label="Saved plan counts">
        <span>
          <strong>{plan.applyableItems}</strong>
          <small>Future candidates</small>
        </span>
        <span>
          <strong>{plan.blockedItems}</strong>
          <small>Blocked</small>
        </span>
        <span>
          <strong>{plan.reviewOnlyItems}</strong>
          <small>Review-only</small>
        </span>
        <span>
          <strong>Draft</strong>
          <small>Record type</small>
        </span>
      </div>

      <div className="organize-plan-detail-block">
        <h4>Source scope</h4>
        <p>{formatScope(plan.sourceScope)}</p>
      </div>

      {plan.caveats.length > 0 ? (
        <div className="organize-plan-detail-block">
          <h4>Caveats</h4>
          <ul className="organize-plan-caveats">
            {plan.caveats.map((caveat) => (
              <li key={caveat}>{caveat}</li>
            ))}
          </ul>
        </div>
      ) : null}

      <ValidationPreviewSection
        planId={plan.id}
        preview={validationPreview}
        loading={validationLoading}
        error={validationError}
        onCheck={onCheckValidation}
      />

      <div className="pending-plans-actions">
        <button type="button" className="secondary-action" onClick={onCancelDraft}>
          <ArchiveX size={16} />
          Cancel draft
        </button>
      </div>

      {confirmingCancel ? (
        <div className="staging-result staging-result--warn" role="status">
          <span>
            This only cancels the saved draft record. It does not touch files.
          </span>
          <button
            type="button"
            className="secondary-action"
            disabled={cancelling}
            onClick={onKeepDraft}
          >
            Keep draft
          </button>
          <button
            type="button"
            className="secondary-action"
            disabled={cancelling}
            onClick={onConfirmCancel}
          >
            {cancelling ? (
              <>
                <LoaderCircle size={16} className="spin" />
                Cancelling draft
              </>
            ) : (
              <>
                <ArchiveX size={16} />
                Confirm cancel draft
              </>
            )}
          </button>
        </div>
      ) : null}

      {groupedItems.length === 0 ? (
        <section className="organize-plan-empty">
          <Info size={24} />
          <h3>No saved item details</h3>
          <p>This saved draft has no item rows. No files changed.</p>
        </section>
      ) : (
        <div className="organize-plan-bucket-list">
          {groupedItems.map((group) => (
            <section
              key={group.key}
              className="organize-plan-bucket"
              aria-label={`${group.label} saved items`}
            >
              <div className="organize-plan-bucket-heading">
                <h3>{group.label}</h3>
                <span>{formatCount(group.items.length, "item")}</span>
              </div>
              <div className="organize-plan-item-list">
                {group.items.map((item, index) => (
                  <SavedPlanItemCard key={item.id} item={item} index={index} />
                ))}
              </div>
            </section>
          ))}
        </div>
      )}
    </section>
  );
}

export function SavedPlansReview({
  refreshVersion,
  selectedPlanId: selectedPlanIdFromParent,
  statusMessage,
  onCreatePlan,
}: SavedPlansReviewProps) {
  const [plans, setPlans] = useState<ApplyPlanListItem[]>([]);
  const [selectedPlanId, setSelectedPlanId] = useState<number | null>(null);
  const [selectedPlan, setSelectedPlan] = useState<PersistedApplyPlan | null>(null);
  const [loadingList, setLoadingList] = useState(true);
  const [loadingDetails, setLoadingDetails] = useState(false);
  const [listError, setListError] = useState<string | null>(null);
  const [detailError, setDetailError] = useState<string | null>(null);
  const [validationPreview, setValidationPreview] =
    useState<ApplyPlanValidationPreview | null>(null);
  const [validationLoading, setValidationLoading] = useState(false);
  const [validationError, setValidationError] = useState<string | null>(null);
  const [confirmingCancel, setConfirmingCancel] = useState(false);
  const [cancelling, setCancelling] = useState(false);
  const [localStatusMessage, setLocalStatusMessage] = useState<string | null>(null);

  const loadSavedPlans = useCallback(async () => {
    setLoadingList(true);
    setListError(null);
    try {
      const nextPlans = await api.listSavedApplyPlans();
      setPlans(nextPlans);
    } catch (error) {
      setPlans([]);
      setListError(error instanceof Error ? error.message : String(error));
    } finally {
      setLoadingList(false);
    }
  }, []);

  useEffect(() => {
    void loadSavedPlans();
  }, [loadSavedPlans, refreshVersion]);

  useEffect(() => {
    if (selectedPlanIdFromParent !== null) {
      setSelectedPlanId(selectedPlanIdFromParent);
    }
  }, [selectedPlanIdFromParent]);

  useEffect(() => {
    if (selectedPlanId === null) {
      setSelectedPlan(null);
      setDetailError(null);
      setValidationPreview(null);
      setValidationError(null);
      setValidationLoading(false);
      setConfirmingCancel(false);
      return;
    }

    let cancelled = false;
    const loadDetails = async () => {
      setLoadingDetails(true);
      setDetailError(null);
      setValidationPreview(null);
      setValidationError(null);
      setValidationLoading(false);
      setConfirmingCancel(false);
      try {
        const plan = await api.getApplyPlan(selectedPlanId);
        if (cancelled) return;
        if (!plan) {
          setSelectedPlan(null);
          setDetailError("Saved preview plan was not found.");
          return;
        }
        setSelectedPlan(plan);
      } catch (error) {
        if (!cancelled) {
          setSelectedPlan(null);
          setDetailError(error instanceof Error ? error.message : String(error));
        }
      } finally {
        if (!cancelled) {
          setLoadingDetails(false);
        }
      }
    };

    void loadDetails();
    return () => {
      cancelled = true;
    };
  }, [selectedPlanId]);

  const handleCheckValidation = async (planId: number) => {
    setValidationLoading(true);
    setValidationError(null);
    try {
      const preview = await api.previewApplyPlanValidation({ planId });
      setValidationPreview(preview);
    } catch (error) {
      setValidationPreview(null);
      setValidationError(error instanceof Error ? error.message : String(error));
    } finally {
      setValidationLoading(false);
    }
  };

  const handleCancelDraft = async () => {
    if (!selectedPlanId) return;
    setCancelling(true);
    setLocalStatusMessage(null);
    try {
      await api.deleteDraftApplyPlan(selectedPlanId);
      setLocalStatusMessage("Draft cancelled. No files changed.");
      setSelectedPlanId(null);
      setSelectedPlan(null);
      setConfirmingCancel(false);
      await loadSavedPlans();
    } catch (error) {
      setDetailError(error instanceof Error ? error.message : String(error));
    } finally {
      setCancelling(false);
    }
  };

  return (
    <section className="pending-plans-preview saved-plans-review" aria-label="Saved plans">
      <div className="pending-plans-header">
        <div>
          <span className="eyebrow">Saved preview work</span>
          <h2>Saved plans</h2>
          <p>
            Review saved draft preview plans, including item evidence, caveats,
            and blockers. These records are not ready for file changes and do not change files.
          </p>
        </div>
        <button type="button" className="secondary-action" onClick={onCreatePlan}>
          <FolderOpen size={16} />
          Create plan
        </button>
      </div>

      <div className="pending-plans-safety-strip" aria-label="Saved plans safety">
        <ShieldCheck size={16} />
        <strong>No files changed</strong>
        <span>
          Saved plans are draft preview records. SimSuite is not moving,
          deleting, replacing, or changing files from this section.
        </span>
      </div>

      {statusMessage ? (
        <div className="staging-result staging-result--ok" role="status">
          <CheckCircle2 size={16} />
          <span>{statusMessage}</span>
        </div>
      ) : null}

      {localStatusMessage ? (
        <div className="staging-result staging-result--ok" role="status">
          <CheckCircle2 size={16} />
          <span>{localStatusMessage}</span>
        </div>
      ) : null}

      {listError ? (
        <div className="staging-result staging-result--warn" role="status">
          <AlertCircle size={16} />
          <span>{listError} No files changed.</span>
        </div>
      ) : null}

      {loadingList ? (
        <section className="organize-plan-empty" aria-live="polite">
          <LoaderCircle size={24} className="spin" />
          <h3>Loading saved plans</h3>
          <p>SimSuite is checking draft preview records. No files changed.</p>
        </section>
      ) : plans.length === 0 ? (
        <section className="organize-plan-empty">
          <FileCheck2 size={24} />
          <h3>No saved preview plans yet</h3>
          <p>
            Generate a preview plan from the Create plan tab, then save it as a
            draft for later review. No files changed.
          </p>
          <button type="button" className="primary-action" onClick={onCreatePlan}>
            <ListChecks size={16} />
            Create preview plan
          </button>
        </section>
      ) : (
        <div className="organize-plan-layout saved-plans-layout">
          <section className="pending-batches-section" aria-label="Saved preview plans">
            <div className="pending-batches-heading">
              <div>
                <h3>Draft preview plans</h3>
                <p>
                  Summaries stay compact here. Open details to review evidence,
                  blockers, and technical path information.
                </p>
              </div>
            </div>
            <div className="pending-batch-list">
              {plans.map((plan) => (
                <SavedPlanSummaryRow
                  key={plan.id}
                  plan={plan}
                  selected={plan.id === selectedPlanId}
                  onReview={() => setSelectedPlanId(plan.id)}
                />
              ))}
            </div>
          </section>

          <div className="panel-card organize-plan-panel">
            <SavedPlanDetails
              plan={selectedPlan}
              loading={loadingDetails}
              error={detailError}
              validationPreview={validationPreview}
              validationLoading={validationLoading}
              validationError={validationError}
              confirmingCancel={confirmingCancel}
              cancelling={cancelling}
              onCheckValidation={handleCheckValidation}
              onCancelDraft={() => setConfirmingCancel(true)}
              onConfirmCancel={handleCancelDraft}
              onKeepDraft={() => setConfirmingCancel(false)}
            />
          </div>
        </div>
      )}
    </section>
  );
}
