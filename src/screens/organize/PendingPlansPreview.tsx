import { useCallback, useEffect, useState } from "react";
import { AnimatePresence, m } from "motion/react";
import {
  Archive,
  ArrowRight,
  Inbox,
  ListChecks,
  LoaderCircle,
  ShieldCheck,
} from "lucide-react";
import { api } from "../../lib/api";
import type {
  Screen,
  StagingArea,
  StagingAreasSummary,
  StagingPlan,
  StagingPlanItem,
} from "../../lib/types";

interface PendingPlansPreviewProps {
  onNavigate?: (screen: Screen) => void;
  showOrganizeLink?: boolean;
}

interface StagingAreaCardProps {
  area: StagingArea;
}

interface StagingPlanPanelProps {
  plan: StagingPlan | null;
}

function formatBytes(bytes: number): string {
  if (bytes === 0) return "0 B";
  const k = 1024;
  const sizes = ["B", "KB", "MB", "GB"];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return `${parseFloat((bytes / Math.pow(k, i)).toFixed(1))} ${sizes[i]}`;
}

function userFacingPlanText(value: string): string {
  return value
    .replace(/\bStaging preview plan\b/gi, "Plan Preview")
    .replace(/\bCurrent Staging data\b/g, "Current Plan Preview data")
    .replace(/\bstaged folders\b/gi, "pending folders")
    .replace(/\bstaged folder\b/gi, "pending folder")
    .replace(/\bstaged data\b/gi, "pending plan data")
    .replace(/\bStaging\b/g, "Plan Preview")
    .replace(/\bstaging\b/g, "Plan Preview");
}

function StagingAreaCard({ area }: StagingAreaCardProps) {
  const isNumeric = /^\d+$/.test(area.itemId);
  const totalFiles = area.subdirectories.reduce(
    (sum, subdirectory) => sum + subdirectory.fileCount,
    0,
  );
  const totalBytes = area.subdirectories.reduce(
    (sum, subdirectory) => sum + subdirectory.totalBytes,
    0,
  );

  return (
    <m.div
      className="staging-card"
      initial={{ opacity: 0, y: 8 }}
      animate={{ opacity: 1, y: 0 }}
      exit={{ opacity: 0, y: -8 }}
      transition={{ duration: 0.15 }}
    >
      <div className="staging-card-header">
        <div className="staging-card-meta">
          <Archive size={16} className="staging-card-icon" />
          <span className="staging-card-item-id">
            {isNumeric ? `Item #${area.itemId}` : `Pending plan (${area.itemId})`}
          </span>
          {!isNumeric && (
            <span className="staging-card-badge staging-card-badge--pending">
              Pending
            </span>
          )}
        </div>
        <div className="staging-card-stats">
          <span>
            {totalFiles} file{totalFiles !== 1 ? "s" : ""}
          </span>
          <span className="staging-card-sep">|</span>
          <span>{formatBytes(totalBytes)}</span>
        </div>
      </div>

      <div className="staging-card-subs">
        {area.subdirectories.map((subdirectory) => (
          <div key={subdirectory.path} className="staging-sub-row">
            <span className="staging-sub-name">{subdirectory.name}</span>
            <span className="staging-sub-info">
              {subdirectory.fileCount} file
              {subdirectory.fileCount !== 1 ? "s" : ""} |{" "}
              {formatBytes(subdirectory.totalBytes)}
            </span>
          </div>
        ))}
      </div>

      <div className="staging-card-actions" aria-label="Plan preview readiness">
        <button
          type="button"
          className="staging-btn staging-btn--disabled"
          disabled
        >
          <ShieldCheck size={14} />
          Preview only
        </button>
        <span className="staging-sub-info">
          Review before applying. No files can be changed from this view yet.
        </span>
      </div>
    </m.div>
  );
}

function EmptyPendingPlans() {
  return (
    <div className="staging-empty">
      <Inbox size={48} className="staging-empty-icon" />
      <h3 className="staging-empty-title">No pending plans</h3>
      <p className="staging-empty-body">
        Pending plans will appear here when SimSuite has preview-only plan data
        to review. No files are changed from this section.
      </p>
    </div>
  );
}

function planStatusLabel(plan: StagingPlan): string {
  if (plan.status === "blocked") return "Not ready to apply yet";
  if (plan.status === "ready_for_review") return "Ready for review";
  return "Preview only";
}

function actionLabel(item: StagingPlanItem): string {
  switch (item.actionKind) {
    case "suggest_move":
      return "Suggested destination";
    case "suggest_group":
      return "Suggested group";
    case "leave_in_place":
      return "Leave in place";
    case "no_action":
      return "No action";
    case "suggest_review":
    default:
      return "Review item";
  }
}

function evidenceLabel(item: StagingPlanItem): string {
  switch (item.evidenceLevel) {
    case "deterministic":
      return "Deterministic fact";
    case "evidence_backed":
      return "Evidence-backed cue";
    case "heuristic":
      return "Heuristic hint";
    case "review_only":
    default:
      return "Manual review needed";
  }
}

function StagingPlanPanel({ plan }: StagingPlanPanelProps) {
  if (!plan) {
    return (
      <m.section
        className="staging-card"
        aria-label="Preview plan"
        initial={{ opacity: 0, y: 8 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.15 }}
      >
        <div className="staging-card-header">
          <div className="staging-card-meta">
            <ListChecks size={16} className="staging-card-icon" />
            <span className="staging-card-item-id">Preview plan</span>
            <span className="staging-card-badge staging-card-badge--pending">
              Not ready to apply yet
            </span>
          </div>
        </div>
        <div className="staging-card-actions" aria-label="Preview plan safety">
          <ShieldCheck size={14} />
          <span className="staging-sub-info">
            Preview plan could not be loaded. No files changed.
          </span>
        </div>
      </m.section>
    );
  }

  return (
    <m.section
      className="staging-card"
      aria-label="Preview plan"
      initial={{ opacity: 0, y: 8 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.15 }}
    >
      <div className="staging-card-header">
        <div className="staging-card-meta">
          <ListChecks size={16} className="staging-card-icon" />
          <span className="staging-card-item-id">
            {userFacingPlanText(plan.title)}
          </span>
          <span className="staging-card-badge staging-card-badge--pending">
            {planStatusLabel(plan)}
          </span>
        </div>
        <div className="staging-card-stats">
          <span>
            {plan.itemCount} plan item{plan.itemCount !== 1 ? "s" : ""}
          </span>
          <span className="staging-card-sep">|</span>
          <span>No files changed</span>
        </div>
      </div>

      <div className="staging-card-subs">
        <div className="staging-sub-row">
          <span className="staging-sub-name">Summary</span>
          <span className="staging-sub-info">
            {userFacingPlanText(plan.summary)}
          </span>
        </div>
        {plan.caveats.map((caveat) => (
          <div key={caveat} className="staging-sub-row">
            <span className="staging-sub-name">Caveat</span>
            <span className="staging-sub-info">
              {userFacingPlanText(caveat)}
            </span>
          </div>
        ))}
        {plan.items.length === 0 ? (
          <div className="staging-sub-row">
            <span className="staging-sub-name">Plan items</span>
            <span className="staging-sub-info">
              No preview items yet. SimSuite needs more plan data before
              review.
            </span>
          </div>
        ) : (
          plan.items.map((item) => (
            <div key={item.id} className="staging-sub-row">
              <span className="staging-sub-name">{item.fileName}</span>
              <span className="staging-sub-info">
                {actionLabel(item)} | {evidenceLabel(item)} |{" "}
                {userFacingPlanText(item.reason)}
              </span>
            </div>
          ))
        )}
      </div>

      <div className="staging-card-actions" aria-label="Preview plan safety">
        <button
          type="button"
          className="staging-btn staging-btn--disabled"
          disabled
        >
          <ShieldCheck size={14} />
          Preview plan only
        </button>
        <span className="staging-sub-info">
          Review before applying. Backup and restore support are still required.
        </span>
      </div>
    </m.section>
  );
}

export function PendingPlansPreview({
  onNavigate,
  showOrganizeLink = false,
}: PendingPlansPreviewProps) {
  const [summary, setSummary] = useState<StagingAreasSummary | null>(null);
  const [plan, setPlan] = useState<StagingPlan | null>(null);
  const [loading, setLoading] = useState(true);
  const [loadErrors, setLoadErrors] = useState<string[]>([]);

  const loadPendingPlans = useCallback(async () => {
    setLoading(true);
    setLoadErrors([]);

    const [areasResult, planResult] = await Promise.allSettled([
      api.getStagingAreas(),
      api.getStagingPreviewPlan(),
    ]);

    const nextErrors: string[] = [];

    if (areasResult.status === "fulfilled") {
      setSummary(areasResult.value);
    } else {
      setSummary(null);
      nextErrors.push("Pending plan folders could not be loaded.");
      console.error(
        "[PendingPlansPreview] failed to load pending plan folders:",
        areasResult.reason,
      );
    }

    if (planResult.status === "fulfilled") {
      setPlan(planResult.value);
    } else {
      setPlan(null);
      nextErrors.push("Preview plan summary could not be loaded.");
      console.error(
        "[PendingPlansPreview] failed to load preview plan:",
        planResult.reason,
      );
    }

    setLoadErrors(nextErrors);
    setLoading(false);
  }, []);

  useEffect(() => {
    void loadPendingPlans();
  }, [loadPendingPlans]);

  const areas = summary?.areas ?? [];
  const totalFiles = summary?.totalFileCount ?? 0;
  const totalBytes = summary?.totalBytes ?? 0;

  if (loading) {
    return (
      <section className="organize-plan-empty" aria-live="polite">
        <LoaderCircle size={24} className="spin" />
        <h3>Loading pending plans</h3>
        <p>SimSuite is checking preview-only plan data. No files changed.</p>
      </section>
    );
  }

  return (
    <section className="pending-plans-preview" aria-label="Pending plans">
      <div className="pending-plans-header">
        <div>
          <span className="eyebrow">Plan Preview</span>
          <h2>Pending plans</h2>
          <p>
            Review proposed plans before anything changes. This is the same
            preview-only checkpoint, now inside Organize.
          </p>
        </div>
        {showOrganizeLink && onNavigate ? (
          <button
            type="button"
            className="secondary-action"
            onClick={() => onNavigate("organize")}
          >
            <ArrowRight size={16} />
            Open Organize
          </button>
        ) : null}
      </div>

      <div className="staging-result staging-result--warn">
        Preview only. No files changed. Review before applying.
      </div>

      {loadErrors.length > 0 ? (
        <div className="staging-result staging-result--warn" role="status">
          {loadErrors.join(" ")} No files changed.
        </div>
      ) : null}

      {areas.length > 0 ? (
        <div className="pending-plans-stats" aria-label="Pending plan totals">
          <span>
            <strong>{areas.length}</strong>
            <small>Pending plan{areas.length !== 1 ? "s" : ""}</small>
          </span>
          <span>
            <strong>{totalFiles}</strong>
            <small>File{totalFiles !== 1 ? "s" : ""}</small>
          </span>
          <span>
            <strong>{formatBytes(totalBytes)}</strong>
            <small>Total size</small>
          </span>
        </div>
      ) : null}

      <StagingPlanPanel plan={plan} />

      {areas.length === 0 ? (
        <EmptyPendingPlans />
      ) : (
        <div className="staging-list">
          <AnimatePresence mode="popLayout">
            {areas.map((area) => (
              <StagingAreaCard key={area.itemId} area={area} />
            ))}
          </AnimatePresence>
        </div>
      )}
    </section>
  );
}
