import { RefreshCw, ShieldAlert, X } from "lucide-react";
import { m } from "motion/react";
import { isNudgeDismissed, setNudgeDismissed, type DownloadQueueLane } from "../../lib/guidedFlowStorage";
import type { DownloadProgress, UserView } from "../../lib/types";

interface DownloadsTopStripProps {
  statusMessage: string | null;
  errorMessage: string | null;
  /** Total items visible after the current filter is applied */
  totalItems: number;
  readyCount: number;
  waitingCount: number;
  specialSetupCount: number;
  blockedCount: number;
  lastCheckLabel: string;
  isRefreshing: boolean;
  isLoading: boolean;
  reviewActionLabel: string;
  onRefresh: () => void;
  onOpenReview: () => void;
  onLaneChange: (lane: DownloadQueueLane) => void;
  onClearFilter?: () => void;
  userView: UserView;
  statusFilter: string;
  progress?: DownloadProgress | null;
  undoableApply?: { itemId: number; displayName: string } | null;
  onRequestUndo?: () => void;
  isUndoing?: boolean;
}

const STATUS_FILTER_LABELS: Record<string, { label: string; tone: string }> = {
  ready:       { label: "Ready",       tone: "is-good"   },
  partial:     { label: "Partial",     tone: ""          },
  needs_review:{ label: "Needs review",tone: "is-warn"   },
  applied:     { label: "Applied",     tone: "is-good"   },
  error:       { label: "Error",       tone: "is-danger" },
  ignored:     { label: "Ignored",     tone: ""          },
};

export function DownloadsTopStrip({
  statusMessage,
  errorMessage,
  totalItems,
  readyCount,
  waitingCount,
  specialSetupCount,
  blockedCount,
  lastCheckLabel,
  isRefreshing,
  isLoading,
  reviewActionLabel,
  onRefresh,
  onOpenReview,
  onLaneChange,
  onClearFilter,
  userView,
  statusFilter,
  progress,
  undoableApply,
  onRequestUndo,
  isUndoing,
}: DownloadsTopStripProps) {
  const hasProgress = progress != null && progress.totalCount > 0;
  const showAlertRow = Boolean(errorMessage) || Boolean(statusMessage);
  const isError = Boolean(errorMessage);
  const isFiltered = statusFilter !== "";
  const filterMeta = isFiltered ? STATUS_FILTER_LABELS[statusFilter] : null;

  return (
    <div className="downloads-top-strip">
      {/* Row 1: Alert / status message — only visible when there's something to say */}
      {showAlertRow && (
        <div className={`downloads-top-strip-alert downloads-top-strip-alert-${isError ? "error" : "warn"}`}>
          <span className="health-chip">
            {isError ? errorMessage : statusMessage}
          </span>
          {!isError && undoableApply && onRequestUndo && (
            <button
              type="button"
              className="health-chip-action"
              onClick={onRequestUndo}
              disabled={isUndoing}
            >
              {isUndoing ? "Moving..." : "Undo"}
            </button>
          )}
        </div>
      )}

      {/* Row 2: Always-visible data strip */}
      <div className="slim-strip downloads-top-strip-data">
        {/* Counter chips */}
        <div className="slim-strip-group downloads-top-strip-counters">
          {isFiltered && filterMeta ? (
            /* Filtered view: one prominent count + active filter chip + clear button */
            <>
              <span className={`health-chip ${filterMeta.tone}`}>
                {totalItems.toLocaleString()}{" "}
                {totalItems === 1 ? "item" : "items"}
              </span>
              <span className="health-chip downloads-top-strip-filter-chip">
                {filterMeta.label}
              </span>
              {onClearFilter && (
                <button
                  type="button"
                  className="health-chip health-chip-clear"
                  onClick={onClearFilter}
                  title="Clear filter and show all"
                >
                  <X size={12} strokeWidth={2.5} style={{ flexShrink: 0 }} />
                  Clear filter
                </button>
              )}
            </>
          ) : (
            /* All view: four counters */
            <>
              <span className="health-chip is-good">
                <span className="health-chip-dot" />
                {totalItems.toLocaleString()}{" "}
                {totalItems === 1 ? "item" : "items"}
              </span>
              <span className="health-chip">
                {readyCount.toLocaleString()} ready
              </span>
              <span className={`health-chip${waitingCount > 0 ? " is-warn" : ""}`}>
                {waitingCount.toLocaleString()} needs review
              </span>
              <span className={`health-chip${blockedCount > 0 ? " is-danger" : ""}`}>
                {blockedCount.toLocaleString()} blocked
              </span>
            </>
          )}
        </div>

        {/* Progress + actions */}
        <div className="slim-strip-group downloads-top-strip-actions">
          {hasProgress && (
            <m.span
              className="ghost-chip download-progress-chip"
              key={`progress-${progress.phase}`}
              initial={{ opacity: 0.6 }}
              animate={{ opacity: 1 }}
              transition={{ duration: 0.3 }}
            >
              {progress.phase}:{" "}
              {progress.currentFile.length > 30
                ? `${progress.currentFile.slice(0, 30)}…`
                : progress.currentFile}
              {progress.totalCount > 0 && (
                <span className="download-progress-count">
                  {" "}({progress.processedCount}/{progress.totalCount})
                </span>
              )}
            </m.span>
          )}
          <span className="ghost-chip downloads-top-strip-lastcheck">
            {lastCheckLabel}
          </span>
          <button
            type="button"
            className="secondary-action"
            onClick={onRefresh}
            disabled={isRefreshing || isLoading}
          >
            <RefreshCw size={14} strokeWidth={2} />
            {isRefreshing ? "Refreshing…" : "Refresh"}
          </button>
          <button
            type="button"
            className="secondary-action"
            onClick={onOpenReview}
          >
            <ShieldAlert size={14} strokeWidth={2} />
            {reviewActionLabel}
          </button>
          {/* Nudge chip only shown in "All" view for beginners */
          !isFiltered &&
          userView === "beginner" &&
          !isNudgeDismissed() &&
          (waitingCount > 0 || specialSetupCount > 0 || blockedCount > 0) && (
            <button
              className="casual-nudge-chip"
              onClick={() => {
                setNudgeDismissed();
                const targetLane: DownloadQueueLane =
                  waitingCount > 0 ? "waiting_on_you" :
                  specialSetupCount > 0 ? "special_setup" :
                  blockedCount > 0 ? "blocked" : "ready_now";
                onLaneChange(targetLane);
              }}
              aria-label="Jump to items needing your attention"
            >
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" aria-hidden="true">
                <path d="M18 8A6 6 0 0 0 6 8c0 7-3 9-3 9h18s-3-2-3-9"/>
                <path d="M13.73 21a2 2 0 0 1-3.46 0"/>
              </svg>
              <span className="casual-nudge-chip-text">
                {waitingCount > 0
                  ? `${waitingCount} item${waitingCount !== 1 ? "s" : ""} need${waitingCount === 1 ? "s" : ""} review`
                  : specialSetupCount > 0
                    ? `${specialSetupCount} item${specialSetupCount !== 1 ? "s" : ""} need${specialSetupCount === 1 ? "s" : ""} setup`
                    : `${blockedCount} item${blockedCount !== 1 ? "s" : ""} held for safety`}
              </span>
            </button>
          )}
        </div>
      </div>
    </div>
  );
}
