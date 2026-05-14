import { useCallback, useEffect, useMemo, useState } from "react";
import { m } from "motion/react";
import {
  ArrowRight,
  Eye,
  EyeOff,
  Inbox,
  Layers3,
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
} from "../../lib/types";

interface PendingPlansPreviewProps {
  onNavigate?: (screen: Screen) => void;
  onCreatePlan?: () => void;
  showOrganizeLink?: boolean;
}

interface PendingBatchRowProps {
  area: StagingArea;
  batchIndex: number;
  showTechnicalDetails: boolean;
}

interface PendingPlanSummaryPanelProps {
  plan: StagingPlan | null;
}

const VISIBLE_BATCH_LIMIT = 5;
const TECHNICAL_FOLDER_LIMIT = 5;
const LARGE_PENDING_FILE_COUNT = 10_000;
const LARGE_PENDING_BYTES = 50 * 1024 * 1024 * 1024;

function formatBytes(bytes: number): string {
  if (bytes === 0) return "0 B";
  const k = 1024;
  const sizes = ["B", "KB", "MB", "GB", "TB"];
  const i = Math.min(Math.floor(Math.log(bytes) / Math.log(k)), sizes.length - 1);
  return `${parseFloat((bytes / Math.pow(k, i)).toFixed(1))} ${sizes[i]}`;
}

function formatNumber(value: number): string {
  return new Intl.NumberFormat("en-US").format(value);
}

function pluralize(value: number, singular: string, plural = `${singular}s`) {
  return `${formatNumber(value)} ${value === 1 ? singular : plural}`;
}

function userFacingPlanText(value: string): string {
  return value
    .replace(/\bStaging preview plan\b/gi, "Plan Preview")
    .replace(/\bCurrent Staging data\b/gi, "Current pending batch data")
    .replace(/\bstaged folders\b/gi, "pending batch folders")
    .replace(/\bstaged folder\b/gi, "pending batch folder")
    .replace(/\bstaged data\b/gi, "pending batch data")
    .replace(/\bFolder-level staging data\b/gi, "Folder-level pending batch data")
    .replace(/\bStaging\b/g, "Plan Preview")
    .replace(/\bstaging\b/g, "pending batch");
}

function summarizeArea(area: StagingArea) {
  const fileCount = area.subdirectories.reduce(
    (sum, subdirectory) => sum + subdirectory.fileCount,
    0,
  );
  const totalBytes = area.subdirectories.reduce(
    (sum, subdirectory) => sum + subdirectory.totalBytes,
    0,
  );

  return {
    fileCount,
    totalBytes,
    folderCount: area.subdirectories.length,
  };
}

function planStatusLabel(plan: StagingPlan): string {
  if (plan.status === "blocked") return "Not ready to apply yet";
  if (plan.status === "ready_for_review") return "Ready for review";
  return "Preview only";
}

function dedupeText(values: string[]): string[] {
  return [...new Set(values.map(userFacingPlanText).filter(Boolean))];
}

function EmptyPendingPlans({ onCreatePlan }: { onCreatePlan: () => void }) {
  return (
    <div className="staging-empty pending-plans-empty">
      <Inbox size={42} className="staging-empty-icon" />
      <h3 className="staging-empty-title">No saved organization plans yet</h3>
      <p className="staging-empty-body">
        Generated plans are not saved yet. Downloaded or imported batches waiting
        for review can be handled from Inbox, and new organization suggestions
        can be created from the Create plan tab.
      </p>
      <div className="pending-plans-actions">
        <button type="button" className="primary-action" onClick={onCreatePlan}>
          <ListChecks size={16} />
          Create preview plan
        </button>
      </div>
    </div>
  );
}

function PendingPlanSummaryPanel({ plan }: PendingPlanSummaryPanelProps) {
  if (!plan) {
    return (
      <section className="pending-plans-meaning-panel" aria-label="Preview plan status">
        <div className="pending-plans-meaning-icon">
          <ListChecks size={18} />
        </div>
        <div>
          <h3>Preview plan status</h3>
          <p>
            SimSuite could not load the pending preview summary. No files
            changed.
          </p>
        </div>
      </section>
    );
  }

  const caveats = dedupeText(plan.caveats).filter(
    (caveat) => !/per-file organization suggestions are future work/i.test(caveat),
  );

  return (
    <section className="pending-plans-meaning-panel" aria-label="Preview plan status">
      <div className="pending-plans-meaning-icon">
        <ListChecks size={18} />
      </div>
      <div>
        <div className="pending-plans-meaning-heading">
          <h3>Pending data is folder-level</h3>
          <span className="organize-plan-status-chip">{planStatusLabel(plan)}</span>
        </div>
        <p>
          {plan.items.length > 0
            ? "SimSuite can see imported/downloaded batch folders, but these are not saved organization plans and do not contain per-file destination suggestions yet."
            : userFacingPlanText(plan.summary)}
        </p>
        <ul className="pending-plans-meaning-list">
          <li>No files changed.</li>
          <li>
            Folder-level batch data is summarized once here instead of repeated
            as review rows.
          </li>
          {caveats.slice(0, 2).map((caveat) => (
            <li key={caveat}>{caveat}</li>
          ))}
        </ul>
      </div>
    </section>
  );
}

function PendingBatchRow({
  area,
  batchIndex,
  showTechnicalDetails,
}: PendingBatchRowProps) {
  const summary = summarizeArea(area);
  const visibleTechnicalFolders = area.subdirectories.slice(0, TECHNICAL_FOLDER_LIMIT);
  const hiddenTechnicalFolders = Math.max(
    0,
    area.subdirectories.length - visibleTechnicalFolders.length,
  );

  return (
    <m.article
      className="pending-batch-row"
      initial={{ opacity: 0, y: 6 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.14 }}
    >
      <div className="pending-batch-main">
        <div className="pending-batch-heading">
          <Layers3 size={16} />
          <div>
            <h3>Pending batch {batchIndex + 1}</h3>
            <p>
              Imported or downloaded content waiting for review. This is not a
              saved organization plan yet.
            </p>
          </div>
        </div>
        <span className="staging-card-badge staging-card-badge--pending">
          Preview only
        </span>
      </div>

      <div className="pending-batch-metrics" aria-label={`Pending batch ${batchIndex + 1} summary`}>
        <span>
          <strong>{formatNumber(summary.folderCount)}</strong>
          <small>Folder group{summary.folderCount !== 1 ? "s" : ""}</small>
        </span>
        <span>
          <strong>{formatNumber(summary.fileCount)}</strong>
          <small>File{summary.fileCount !== 1 ? "s" : ""} found</small>
        </span>
        <span>
          <strong>{formatBytes(summary.totalBytes)}</strong>
          <small>Total size</small>
        </span>
      </div>

      <div className="pending-batch-next">
        <strong>Next step</strong>
        <span>
          Review this batch in Inbox, or create a bounded preview plan from
          Library files when you want organization suggestions.
        </span>
      </div>

      {showTechnicalDetails ? (
        <div className="pending-batch-technical" aria-label={`Pending batch ${batchIndex + 1} technical details`}>
          <div>
            <strong>Internal folder ID</strong>
            <code>{area.itemId}</code>
          </div>
          {visibleTechnicalFolders.map((subdirectory, index) => (
            <div key={subdirectory.path}>
              <strong>Internal folder {index + 1}</strong>
              <code>{subdirectory.name}</code>
              <span>
                {pluralize(subdirectory.fileCount, "file")} |{" "}
                {formatBytes(subdirectory.totalBytes)}
              </span>
            </div>
          ))}
          {hiddenTechnicalFolders > 0 ? (
            <p>{pluralize(hiddenTechnicalFolders, "technical folder")} hidden.</p>
          ) : null}
        </div>
      ) : null}
    </m.article>
  );
}

export function PendingPlansPreview({
  onNavigate,
  onCreatePlan,
  showOrganizeLink = false,
}: PendingPlansPreviewProps) {
  const [summary, setSummary] = useState<StagingAreasSummary | null>(null);
  const [plan, setPlan] = useState<StagingPlan | null>(null);
  const [loading, setLoading] = useState(true);
  const [loadErrors, setLoadErrors] = useState<string[]>([]);
  const [showAllBatches, setShowAllBatches] = useState(false);
  const [showTechnicalDetails, setShowTechnicalDetails] = useState(false);

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
      nextErrors.push("Pending batch data could not be loaded.");
      console.error(
        "[PendingPlansPreview] failed to load pending batch data:",
        areasResult.reason,
      );
    }

    if (planResult.status === "fulfilled") {
      setPlan(planResult.value);
    } else {
      setPlan(null);
      nextErrors.push("Preview summary could not be loaded.");
      console.error(
        "[PendingPlansPreview] failed to load preview summary:",
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
  const totalFolderGroups = useMemo(
    () =>
      areas.reduce((sum, area) => sum + area.subdirectories.length, 0),
      [areas],
  );
  const visibleAreas = showAllBatches ? areas : areas.slice(0, VISIBLE_BATCH_LIMIT);
  const hiddenBatchCount = Math.max(0, areas.length - visibleAreas.length);
  const hasPendingBatches = areas.length > 0;
  const hasLargeCounts =
    totalFiles >= LARGE_PENDING_FILE_COUNT || totalBytes >= LARGE_PENDING_BYTES;

  const handleCreatePlan = () => {
    if (onCreatePlan) {
      onCreatePlan();
      return;
    }
    onNavigate?.("organize");
  };

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
            Review proposed plans before anything changes. Generated organization
            plans are not saved yet, so this tab currently summarizes pending
            imported/downloaded batches when they exist.
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

      <div className="pending-plans-safety-strip" aria-label="Pending plans safety">
        <ShieldCheck size={16} />
        <strong>No files changed</strong>
        <span>
          This area is preview-only. SimSuite is not moving, deleting, or
          changing files from Pending plans.
        </span>
      </div>

      {loadErrors.length > 0 ? (
        <div className="staging-result staging-result--warn" role="status">
          {loadErrors.join(" ")} No files changed.
        </div>
      ) : null}

      {hasPendingBatches ? (
        <div className="pending-plans-stats" aria-label="Pending batch totals">
          <span>
            <strong>{formatNumber(areas.length)}</strong>
            <small>Pending batch{areas.length !== 1 ? "es" : ""}</small>
          </span>
          <span>
            <strong>{formatNumber(totalFolderGroups)}</strong>
            <small>Folder group{totalFolderGroups !== 1 ? "s" : ""}</small>
          </span>
          <span>
            <strong>{formatNumber(totalFiles)}</strong>
            <small>File{totalFiles !== 1 ? "s" : ""} found</small>
          </span>
          <span>
            <strong>{formatBytes(totalBytes)}</strong>
            <small>Total size</small>
          </span>
        </div>
      ) : null}

      {hasLargeCounts ? (
        <div className="pending-plans-note pending-plans-note--warn">
          This count comes from current pending batch data. Review before
          trusting it.
        </div>
      ) : null}

      <PendingPlanSummaryPanel plan={plan} />

      {hasPendingBatches ? (
        <section className="pending-batches-section" aria-label="Pending batches">
          <div className="pending-batches-heading">
            <div>
              <h3>Pending batches</h3>
              <p>
                These are app-local imported/downloaded batches, not saved
                organization plans. Raw internal IDs are hidden unless you open
                technical details.
              </p>
            </div>
            <div className="pending-plans-actions">
              <button type="button" className="primary-action" onClick={handleCreatePlan}>
                <ListChecks size={16} />
                Create preview plan
              </button>
              {onNavigate ? (
                <button
                  type="button"
                  className="secondary-action"
                  onClick={() => onNavigate("downloads")}
                >
                  <Inbox size={16} />
                  Open Inbox
                </button>
              ) : null}
              <button
                type="button"
                className="secondary-action"
                onClick={() => setShowTechnicalDetails((value) => !value)}
              >
                {showTechnicalDetails ? <EyeOff size={16} /> : <Eye size={16} />}
                {showTechnicalDetails
                  ? "Hide technical details"
                  : "Show technical details"}
              </button>
            </div>
          </div>

          <div className="pending-batch-list">
            {visibleAreas.map((area, index) => (
              <PendingBatchRow
                key={area.itemId}
                area={area}
                batchIndex={index}
                showTechnicalDetails={showTechnicalDetails}
              />
            ))}
          </div>

          {hiddenBatchCount > 0 || showAllBatches ? (
            <button
              type="button"
              className="secondary-action pending-batches-toggle"
              onClick={() => setShowAllBatches((value) => !value)}
            >
              {showAllBatches
                ? "Show fewer batches"
                : `Show ${formatNumber(hiddenBatchCount)} more batch${
                    hiddenBatchCount !== 1 ? "es" : ""
                  }`}
            </button>
          ) : null}
        </section>
      ) : (
        <EmptyPendingPlans onCreatePlan={handleCreatePlan} />
      )}
    </section>
  );
}
