import { RefreshCw, ShieldAlert } from "lucide-react";

interface DownloadsTopStripProps {
  statusMessage: string | null;
  errorMessage: string | null;
  totalItems: number;
  readyCount: number;
  waitingCount: number;
  blockedCount: number;
  lastCheckLabel: string;
  isRefreshing: boolean;
  isLoading: boolean;
  reviewActionLabel: string;
  onRefresh: () => void;
  onOpenReview: () => void;
}

export function DownloadsTopStrip({
  statusMessage,
  errorMessage,
  totalItems,
  readyCount,
  waitingCount,
  blockedCount,
  lastCheckLabel,
  isRefreshing,
  isLoading,
  reviewActionLabel,
  onRefresh,
  onOpenReview,
}: DownloadsTopStripProps) {
  return (
    <div className="slim-strip downloads-top-strip">
      <div className="slim-strip-group">
        {statusMessage ? (
          <span className="health-chip is-warn">{statusMessage}</span>
        ) : errorMessage ? (
          <span className="health-chip is-danger">{errorMessage}</span>
        ) : (
          <>
            <span className="health-chip is-good">
              <span className="health-chip-dot"></span>
              {totalItems.toLocaleString()} items
            </span>
            <span className="health-chip">{readyCount.toLocaleString()} ready</span>
            <span className={`health-chip${waitingCount > 0 ? " is-warn" : ""}`}>
              {waitingCount.toLocaleString()} waiting
            </span>
            <span className={`health-chip${blockedCount > 0 ? " is-danger" : ""}`}>
              {blockedCount.toLocaleString()} blocked
            </span>
          </>
        )}
      </div>

      <div className="slim-strip-group">
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
      </div>
    </div>
  );
}
