import { useCallback, useEffect, useState } from "react";
import { AnimatePresence, m } from "motion/react";
import {
  Archive,
  Inbox,
  ListChecks,
  LoaderCircle,
  ShieldCheck,
} from "lucide-react";
import { api } from "../lib/api";
import type {
  Screen,
  StagingArea,
  StagingAreasSummary,
  StagingPlan,
  StagingPlanItem,
  UserView,
} from "../lib/types";

interface StagingScreenProps {
  onNavigate: (screen: Screen) => void;
  userView?: UserView;
}

interface StagingAreaCardProps {
  area: StagingArea;
}

function formatBytes(bytes: number): string {
  if (bytes === 0) return "0 B";
  const k = 1024;
  const sizes = ["B", "KB", "MB", "GB"];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return `${parseFloat((bytes / Math.pow(k, i)).toFixed(1))} ${sizes[i]}`;
}

function StagingAreaCard({ area }: StagingAreaCardProps) {
  const isNumeric = /^\d+$/.test(area.itemId);
  const totalFiles = area.subdirectories.reduce((sum, s) => sum + s.fileCount, 0);
  const totalBytes = area.subdirectories.reduce((sum, s) => sum + s.totalBytes, 0);

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
            {isNumeric ? `Item #${area.itemId}` : `Uncommitted (${area.itemId})`}
          </span>
          {!isNumeric && (
            <span className="staging-card-badge staging-card-badge--pending">
              Pending
            </span>
          )}
        </div>
        <div className="staging-card-stats">
          <span>{totalFiles} file{totalFiles !== 1 ? "s" : ""}</span>
          <span className="staging-card-sep">|</span>
          <span>{formatBytes(totalBytes)}</span>
        </div>
      </div>

      <div className="staging-card-subs">
        {area.subdirectories.map((sub) => (
          <div key={sub.path} className="staging-sub-row">
            <span className="staging-sub-name">{sub.name}</span>
            <span className="staging-sub-info">
              {sub.fileCount} file{sub.fileCount !== 1 ? "s" : ""} |{" "}
              {formatBytes(sub.totalBytes)}
            </span>
          </div>
        ))}
      </div>

      <div className="staging-card-actions" aria-label="Staging readiness">
        <button
          type="button"
          className="staging-btn staging-btn--disabled"
          disabled
        >
          <ShieldCheck size={14} />
          Preview only
        </button>
        <span className="staging-sub-info">
          Review before applying. No files can be changed from this screen yet.
        </span>
      </div>
    </m.div>
  );
}

function EmptyStaging() {
  return (
    <div className="staging-empty">
      <Inbox size={48} className="staging-empty-icon" />
      <h3 className="staging-empty-title">No staged content</h3>
      <p className="staging-empty-body">
        Staging will show preview-only plans here when app-managed downloads
        are ready for review. No files are changed from this screen.
      </p>
    </div>
  );
}

interface StagingPlanPanelProps {
  plan: StagingPlan | null;
}

function planStatusLabel(plan: StagingPlan): string {
  if (plan.status === "blocked") return "Not ready to apply yet";
  if (plan.status === "ready_for_review") return "Ready for review";
  return "Preview only";
}

function actionLabel(item: StagingPlanItem): string {
  switch (item.actionKind) {
    case "suggest_move":
      return "Suggested move";
    case "suggest_group":
      return "Suggested group";
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
          <span className="staging-card-item-id">{plan.title}</span>
          <span className="staging-card-badge staging-card-badge--pending">
            {planStatusLabel(plan)}
          </span>
        </div>
        <div className="staging-card-stats">
          <span>{plan.itemCount} plan item{plan.itemCount !== 1 ? "s" : ""}</span>
          <span className="staging-card-sep">|</span>
          <span>No files changed</span>
        </div>
      </div>

      <div className="staging-card-subs">
        <div className="staging-sub-row">
          <span className="staging-sub-name">Summary</span>
          <span className="staging-sub-info">{plan.summary}</span>
        </div>
        {plan.caveats.map((caveat) => (
          <div key={caveat} className="staging-sub-row">
            <span className="staging-sub-name">Caveat</span>
            <span className="staging-sub-info">{caveat}</span>
          </div>
        ))}
        {plan.items.length === 0 ? (
          <div className="staging-sub-row">
            <span className="staging-sub-name">Plan items</span>
            <span className="staging-sub-info">
              No preview items yet. Staging needs more plan data before review.
            </span>
          </div>
        ) : (
          plan.items.map((item) => (
            <div key={item.id} className="staging-sub-row">
              <span className="staging-sub-name">{item.fileName}</span>
              <span className="staging-sub-info">
                {actionLabel(item)} | {evidenceLabel(item)} | {item.reason}
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

export function StagingScreen(_props: StagingScreenProps) {
  const [summary, setSummary] = useState<StagingAreasSummary | null>(null);
  const [plan, setPlan] = useState<StagingPlan | null>(null);
  const [loading, setLoading] = useState(true);

  const loadStagingAreas = useCallback(async () => {
    try {
      const [areasResult, planResult] = await Promise.allSettled([
        api.getStagingAreas(),
        api.getStagingPreviewPlan(),
      ]);

      if (areasResult.status === "fulfilled") {
        setSummary(areasResult.value);
      } else {
        console.error(
          "[StagingScreen] failed to load staging areas:",
          areasResult.reason,
        );
      }

      if (planResult.status === "fulfilled") {
        setPlan(planResult.value);
      } else {
        console.error(
          "[StagingScreen] failed to load staging preview plan:",
          planResult.reason,
        );
      }
    } catch (err) {
      console.error("[StagingScreen] failed to load staging data:", err);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void loadStagingAreas();
  }, [loadStagingAreas]);

  if (loading) {
    return (
      <div className="screen-loading">
        <LoaderCircle size={24} className="spin" />
        <span>Loading staging area...</span>
      </div>
    );
  }

  const areas = summary?.areas ?? [];
  const totalFiles = summary?.totalFileCount ?? 0;
  const totalBytes = summary?.totalBytes ?? 0;

  return (
    <div className="staging-screen">
      <div className="staging-header">
        <div className="staging-header-left">
          <h2 className="staging-title">
            Staging
            {areas.length > 0 && (
              <span className="staging-count"> ({areas.length})</span>
            )}
          </h2>
          {areas.length > 0 && (
            <span className="staging-summary">
              {totalFiles} file{totalFiles !== 1 ? "s" : ""} |{" "}
              {formatBytes(totalBytes)}
            </span>
          )}
        </div>

        <div className="staging-header-actions">
          <button
            type="button"
            className="staging-btn staging-btn--disabled"
            disabled
          >
            <ShieldCheck size={14} />
            User confirmation required
          </button>
        </div>
      </div>

      <m.div
        className="staging-result staging-result--warn"
        initial={{ opacity: 0, y: -4 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.14 }}
      >
        Staging is preview-only right now. No files will be changed from this
        screen yet. Future file-changing workflows need preview, user
        confirmation, backup and restore support, and recoverable errors.
      </m.div>

      <StagingPlanPanel plan={plan} />

      {areas.length === 0 ? (
        <EmptyStaging />
      ) : (
        <div className="staging-list">
          <AnimatePresence mode="popLayout">
            {areas.map((area) => (
              <StagingAreaCard key={area.itemId} area={area} />
            ))}
          </AnimatePresence>
        </div>
      )}
    </div>
  );
}
