import { useCallback, useEffect, useState } from "react";
import { AnimatePresence, m } from "motion/react";
import {
  Archive,
  Check,
  CheckCheck,
  Inbox,
  LoaderCircle,
  Trash2,
  X,
} from "lucide-react";
import { api } from "../lib/api";
import { hoverLift, tapPress } from "../lib/motion";
import type { Screen, StagingArea, StagingAreasSummary, StagingCommitResult, UserView } from "../lib/types";

interface StagingScreenProps {
  onNavigate: (screen: Screen) => void;
  userView?: UserView;
}

interface StagingAreaCardProps {
  area: StagingArea;
  onCommit: (itemId: string) => void;
  onReject: (itemId: string, paths: string[]) => void;
  committing: boolean;
  rejecting: boolean;
}

function formatBytes(bytes: number): string {
  if (bytes === 0) return "0 B";
  const k = 1024;
  const sizes = ["B", "KB", "MB", "GB"];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return `${parseFloat((bytes / Math.pow(k, i)).toFixed(1))} ${sizes[i]}`;
}

function StagingAreaCard({
  area,
  onCommit,
  onReject,
  committing,
  rejecting,
}: StagingAreaCardProps) {
  const isNumeric = /^\d+$/.test(area.itemId);
  const totalFiles = area.subdirectories.reduce((sum, s) => sum + s.fileCount, 0);
  const totalBytes = area.subdirectories.reduce((sum, s) => sum + s.totalBytes, 0);
  const allPaths = area.subdirectories.map((s) => s.path);

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
          <span className="staging-card-sep">·</span>
          <span>{formatBytes(totalBytes)}</span>
        </div>
      </div>

      <div className="staging-card-subs">
        {area.subdirectories.map((sub) => (
          <div key={sub.path} className="staging-sub-row">
            <span className="staging-sub-name">{sub.name}</span>
            <span className="staging-sub-info">
              {sub.fileCount} file{sub.fileCount !== 1 ? "s" : ""} ·{" "}
              {formatBytes(sub.totalBytes)}
            </span>
          </div>
        ))}
      </div>

      <div className="staging-card-actions">
        <m.button
          className={`staging-btn staging-btn--commit${!isNumeric || committing ? " staging-btn--disabled" : ""}`}
          onClick={() => isNumeric && onCommit(area.itemId)}
          disabled={!isNumeric || committing}
          whileHover={isNumeric && !committing ? hoverLift : undefined}
          whileTap={isNumeric && !committing ? tapPress : undefined}
        >
          {committing ? (
            <LoaderCircle size={14} className="spin" />
          ) : (
            <Check size={14} />
          )}
          {isNumeric ? "Commit to Library" : "Cannot auto-commit"}
        </m.button>
        <m.button
          className={`staging-btn staging-btn--reject${rejecting ? " staging-btn--disabled" : ""}`}
          onClick={() => onReject(area.itemId, allPaths)}
          disabled={rejecting}
          whileHover={!rejecting ? hoverLift : undefined}
          whileTap={!rejecting ? tapPress : undefined}
        >
          {rejecting ? (
            <LoaderCircle size={14} className="spin" />
          ) : (
            <Trash2 size={14} />
          )}
          Reject
        </m.button>
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
        Files that have been extracted or reviewed but not yet committed will
        appear here.
      </p>
    </div>
  );
}

export function StagingScreen({ onNavigate, userView }: StagingScreenProps) {
  const [summary, setSummary] = useState<StagingAreasSummary | null>(null);
  const [loading, setLoading] = useState(true);
  const [committingAll, setCommittingAll] = useState(false);
  const [rejectingAll, setRejectingAll] = useState(false);
  const [committingId, setCommittingId] = useState<string | null>(null);
  const [rejectingId, setRejectingId] = useState<string | null>(null);
  const [result, setResult] = useState<StagingCommitResult | null>(null);

  const loadStagingAreas = useCallback(async () => {
    try {
      const data = await api.getStagingAreas();
      setSummary(data);
    } catch (err) {
      console.error("[StagingScreen] failed to load staging areas:", err);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void loadStagingAreas();
  }, [loadStagingAreas]);

  const handleCommit = async (itemId: string) => {
    setCommittingId(itemId);
    try {
      const res = await api.commitStagingArea(itemId);
      setResult(res);
      if (res.committedCount > 0) {
        await loadStagingAreas();
      }
    } catch (err) {
      console.error("[StagingScreen] commit failed:", err);
    } finally {
      setCommittingId(null);
    }
  };

  const handleReject = async (itemId: string, paths: string[]) => {
    setRejectingId(itemId);
    try {
      await api.cleanupStagingAreas(paths);
      await loadStagingAreas();
    } catch (err) {
      console.error("[StagingScreen] reject failed:", err);
    } finally {
      setRejectingId(null);
    }
  };

  const handleCommitAll = async () => {
    setCommittingAll(true);
    try {
      const res = await api.commitAllStagingAreas();
      setResult(res);
      if (res.committedCount > 0) {
        await loadStagingAreas();
      }
    } catch (err) {
      console.error("[StagingScreen] commit all failed:", err);
    } finally {
      setCommittingAll(false);
    }
  };

  const handleRejectAll = async () => {
    if (!summary || summary.areas.length === 0) return;
    setRejectingAll(true);
    try {
      const allPaths = summary.areas.flatMap((a) =>
        a.subdirectories.map((s) => s.path),
      );
      await api.cleanupStagingAreas(allPaths);
      await loadStagingAreas();
    } catch (err) {
      console.error("[StagingScreen] reject all failed:", err);
    } finally {
      setRejectingAll(false);
    }
  };

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
              {totalFiles} file{totalFiles !== 1 ? "s" : ""} ·{" "}
              {formatBytes(totalBytes)}
            </span>
          )}
        </div>

        {areas.length > 0 && (
          <div className="staging-header-actions">
            <m.button
              className={`staging-btn staging-btn--commit-all${committingAll ? " staging-btn--disabled" : ""}`}
              onClick={() => void handleCommitAll()}
              disabled={committingAll}
              whileHover={!committingAll ? hoverLift : undefined}
              whileTap={!committingAll ? tapPress : undefined}
            >
              {committingAll ? (
                <LoaderCircle size={14} className="spin" />
              ) : (
                <CheckCheck size={14} />
              )}
              Commit all
            </m.button>
            <m.button
              className={`staging-btn staging-btn--reject-all${rejectingAll ? " staging-btn--disabled" : ""}`}
              onClick={() => void handleRejectAll()}
              disabled={rejectingAll}
              whileHover={!rejectingAll ? hoverLift : undefined}
              whileTap={!rejectingAll ? tapPress : undefined}
            >
              {rejectingAll ? (
                <LoaderCircle size={14} className="spin" />
              ) : (
                <Trash2 size={14} />
              )}
              Reject all
            </m.button>
          </div>
        )}
      </div>

      <AnimatePresence mode="wait">
        {result && (
          <m.div
            className={`staging-result staging-result--${result.failedCount > 0 ? "warn" : "ok"}`}
            initial={{ opacity: 0, height: 0 }}
            animate={{ opacity: 1, height: "auto" }}
            exit={{ opacity: 0, height: 0 }}
          >
            {result.errors.length > 0
              ? result.errors[0]
              : `${result.committedCount} item${result.committedCount !== 1 ? "s" : ""} committed`}
            <button
              className="staging-result-close"
              onClick={() => setResult(null)}
            >
              <X size={12} />
            </button>
          </m.div>
        )}
      </AnimatePresence>

      {areas.length === 0 ? (
        <EmptyStaging />
      ) : (
        <div className="staging-list">
          <AnimatePresence mode="popLayout">
            {areas.map((area) => (
              <StagingAreaCard
                key={area.itemId}
                area={area}
                onCommit={handleCommit}
                onReject={handleReject}
                committing={committingId === area.itemId}
                rejecting={rejectingId === area.itemId}
              />
            ))}
          </AnimatePresence>
        </div>
      )}
    </div>
  );
}
