import { RefreshCw, ShieldAlert } from "lucide-react";
import { isNudgeDismissed, setNudgeDismissed, type DownloadQueueLane } from "../../lib/guidedFlowStorage";
import type { DownloadProgress, UserView } from "../../lib/types";

interface DownloadsTopStripProps {
  statusMessage: string | null;
  errorMessage: string | null;
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
  userView: UserView;
  progress?: DownloadProgress | null;
  undoableApply?: { itemId: number; displayName: string } | null;
  onRequestUndo?: () => void;
  isUndoing?: boolean;
}

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
  userView,
  progress,
  undoableApply,
  onRequestUndo,
  isUndoing,
}: DownloadsTopStripProps) {
  const hasProgress = progress != null && progress.totalCount > 0;
  return (
    <div className="slim-strip downloads-top-strip">
      <div className="slim-strip-group">
        {statusMessage ? (
          <span className="health-chip is-warn">
            {statusMessage}
            {undoableApply && onRequestUndo && (
              <button
                type="button"
                className="health-chip-action"
                onClick={onRequestUndo}
                disabled={isUndoing}
              >
                {isUndoing ? "Moving..." : "Undo"}
              </button>
            )}
          </span>
        ) : errorMessage ? (
          <span className="health-chip is-danger">{errorMessage}</span>
        ) : (
          <>
            <span className="health-chip is-good">
              <span className="health-chip-dot"></span>
              {totalItems.toLocaleString()} {totalItems === 1 ? "item" : "items"}
            </span>
            <span className="health-chip">{readyCount.toLocaleString()} ready</span>
            <span className={`health-chip${waitingCount > 0 ? " is-warn" : ""}`}>
              {waitingCount.toLocaleString()} needs review
            </span>
            <span className={`health-chip${blockedCount > 0 ? " is-danger" : ""}`}>
              {blockedCount.toLocaleString()} blocked
            </span>
          </>
        )}
      </div>

      <div className="slim-strip-group">
        {hasProgress && (
          <span className="ghost-chip download-progress-chip">
            {progress.phase}: {progress.currentFile.length > 30 ? `${progress.currentFile.slice(0, 30)}…` : progress.currentFile}
            {progress.totalCount > 0 && (
              <span className="download-progress-count">
                {" "}
                ({progress.processedCount}/{progress.totalCount})
              </span>
            )}
          </span>
        )}
        <span className="ghost-chip">{lastCheckLabel}</span>
        <button
          type="button"
          className="secondary-action"
          onClick={onRefresh}
          disabled={isRefreshing || isLoading}
        >
          <RefreshCw size={14} strokeWidth={2} />
          {isRefreshing ? "Refreshing..." : "Refresh"}
        </button>
        <button
          type="button"
          className="secondary-action"
          onClick={onOpenReview}
        >
          <ShieldAlert size={14} strokeWidth={2} />
          {reviewActionLabel}
        </button>
        {userView === "beginner" && !isNudgeDismissed() && (waitingCount > 0 || specialSetupCount > 0 || blockedCount > 0) && (
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
            aria-label="You have items waiting for review"
          >
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
              <path d="M18 8A6 6 0 0 0 6 8c0 7-3 9-3 9h18s-3-2-3-9"/>
              <path d="M13.73 21a2 2 0 0 1-3.46 0"/>
            </svg>
            <span className="casual-nudge-chip-text">
              {waitingCount > 0
                ? `${waitingCount} item${waitingCount !== 1 ? "s" : ""} waiting for you`
                : specialSetupCount > 0
                  ? `${specialSetupCount} item${specialSetupCount !== 1 ? "s" : ""} need${specialSetupCount === 1 ? "s" : ""} setup`
                  : `${blockedCount} item${blockedCount !== 1 ? "s" : ""} held for safety`}
            </span>
          </button>
        )}
      </div>
    </div>
  );
}
