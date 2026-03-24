import type { ReactNode } from "react";
import { Check, CheckCircle, Clock, Inbox, Settings, ShieldAlert } from "lucide-react";
import { m } from "motion/react";
import { StatePanel } from "../../components/StatePanel";
import { SkeletonLoader } from "../../components/SkeletonLoader";
import {
  rowHover,
  rowPress,
  stagedListItem,
} from "../../lib/motion";
import { isLaneExplained, setLaneExplained, type DownloadQueueLane } from "../../lib/guidedFlowStorage";
import type { UserView } from "../../lib/types";
import { downloadsLaneHint, downloadsLaneLabel } from "./downloadsDisplay";

const LANE_ACCENTS: Record<string, string> = {
  ready_now: "var(--accent)",
  waiting_on_you: "var(--tone-warn)",
  special_setup: "var(--tone-info)",
  blocked: "var(--tone-danger)",
  done: "var(--text-dim)",
};

const LANE_LABELS: Record<string, string> = {
  ready_now: "Ready Now",
  waiting_on_you: "Waiting on You",
  special_setup: "Special Setup",
  blocked: "Needs Review",
  done: "Done",
};

const LANE_ICONS: Record<string, ReactNode> = {
  ready_now: <Check size={16} />,
  waiting_on_you: <Clock size={16} />,
  special_setup: <Settings size={16} />,
  blocked: <ShieldAlert size={16} />,
  done: <CheckCircle size={16} />,
};

const BANNER_CONTENT: Record<string, { title: string; body: string }> = {
  ready_now: {
    title: "Safe and ready to go",
    body: "These files passed the safety check. Add them to your game whenever you're ready.",
  },
  waiting_on_you: {
    title: "Needs your input",
    body: "Something about these needs your attention. Click one to get started.",
  },
  special_setup: {
    title: "Has special setup steps",
    body: "These need a few extra steps before they can work. SimSuite will walk you through it.",
  },
  blocked: {
    title: "Held for safety",
    body: "SimSuite stopped these to protect your game. Check the warning, then decide.",
  },
  done: {
    title: "Already handled",
    body: "These are already added or set aside. Click one to undo and move it somewhere else.",
  },
};

export interface DownloadsQueueRowModel {
  id: number;
  title: string;
  creatorName?: string | null;
  meta: string;
  summary: string;
  samples?: string | null;
  badges: Array<{
    label: string;
    tone: string;
  }>;
  tone: "good" | "medium" | "low" | "neutral";
  selected: boolean;
  batchSelected: boolean;
  sourcePath: string;
}

interface DownloadsQueuePanelProps {
  lane: DownloadQueueLane;
  userView: UserView;
  rows: DownloadsQueueRowModel[];
  isLoading: boolean;
  hasItems: boolean;
  onSelect: (id: number) => void;
  onToggleBatchSelect: (id: number) => void;
  onSelectAll: () => void;
  onClearSelection: () => void;
  selectedCount: number;
  footer?: ReactNode;
}

export function DownloadsQueuePanel({
  lane,
  userView,
  rows,
  isLoading,
  hasItems,
  onSelect,
  onToggleBatchSelect,
  onSelectAll,
  onClearSelection,
  selectedCount,
  footer,
}: DownloadsQueuePanelProps) {
  const allSelected = hasItems && rows.length > 0 && selectedCount === rows.length;
  const someSelected = selectedCount > 0;

  return (
    <div className="panel-card downloads-queue-panel workbench-panel">
      <div className="panel-heading">
        <div>
          <p className="eyebrow">Inbox queue</p>
          <h2>{downloadsLaneLabel(lane, userView)}</h2>
          <p className="downloads-queue-subcopy">{downloadsLaneHint(lane, userView)}</p>
        </div>
        <span className="ghost-chip">
          {isLoading ? "Loading..." : `${rows.length.toLocaleString()} shown`}
        </span>
      </div>

      <div className="vertical-dock downloads-queue-dock">
        <m.div className="queue-list downloads-queue-list" layoutScroll>
          {isLoading ? (
            <SkeletonLoader rows={6} height={56} />
          ) : hasItems ? (
            rows.length ? (
              <div className="downloads-lane-group">
                {userView === "beginner" && !isLaneExplained(lane) && (
                  <div className="casual-lane-banner" role="note">
                    <div className="casual-lane-banner-icon">
                      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                        <circle cx="12" cy="12" r="10"/>
                        <line x1="12" y1="16" x2="12" y2="12"/>
                        <line x1="12" y1="8" x2="12.01" y2="8"/>
                      </svg>
                    </div>
                    <div className="casual-lane-banner-content">
                      <p className="casual-lane-banner-title">{BANNER_CONTENT[lane]?.title}</p>
                      <p className="casual-lane-banner-body">{BANNER_CONTENT[lane]?.body}</p>
                    </div>
                    <button
                      className="casual-lane-banner-dismiss"
                      onClick={() => setLaneExplained(lane)}
                      aria-label="Dismiss"
                    >
                      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                        <line x1="18" y1="6" x2="6" y2="18"/>
                        <line x1="6" y1="6" x2="18" y2="18"/>
                      </svg>
                    </button>
                  </div>
                )}
                {userView === "beginner" && (
                  <div
                    className="downloads-lane-sticky-header"
                    style={{
                      position: "sticky",
                      top: 0,
                      zIndex: 10,
                      background: `color-mix(in srgb, ${LANE_ACCENTS[lane]} 12%, var(--surface-2))`,
                      borderLeft: `3px solid ${LANE_ACCENTS[lane]}`,
                      padding: "6px 12px",
                      display: "flex",
                      alignItems: "center",
                      gap: "8px",
                      borderRadius: "6px 6px 0 0",
                    }}
                  >
                    <span style={{ color: LANE_ACCENTS[lane], display: "flex", alignItems: "center" }}>
                      {LANE_ICONS[lane]}
                    </span>
                    <span style={{ fontSize: "12px", fontWeight: 600, color: "var(--text)" }}>
                      {LANE_LABELS[lane]}
                    </span>
                    <span style={{ fontSize: "11px", color: "var(--text-dim)", marginLeft: "auto" }}>
                      {rows.length} item{rows.length !== 1 ? "s" : ""}
                    </span>
                  </div>
                )}
                <div className="downloads-lane-header">
                  <div>
                    <input
                      type="checkbox"
                      className="downloads-batch-checkbox"
                      checked={allSelected}
                      ref={(el) => {
                        if (el) el.indeterminate = someSelected && !allSelected;
                      }}
                      onChange={() => {
                        if (allSelected) {
                          onClearSelection();
                        } else {
                          onSelectAll();
                        }
                      }}
                      aria-label={allSelected ? "Deselect all" : "Select all"}
                    />
                    <strong>{downloadsLaneLabel(lane, userView)}</strong>
                    <span>{downloadsLaneHint(lane, userView)}</span>
                  </div>
                  <span className="ghost-chip">{rows.length.toLocaleString()}</span>
                </div>

                <m.div
                  className="downloads-lane-list"
                  layout
                >
                  {rows.map((row, index) => (
                    <div
                      key={row.id}
                      className={`downloads-item-row-wrapper ${
                        row.selected ? "is-selected" : ""
                      } ${row.batchSelected ? "is-batch-selected" : ""} downloads-item-row-${row.tone}`}
                      {...stagedListItem(index)}
                    >
                      <input
                        type="checkbox"
                        className="downloads-batch-checkbox"
                        checked={row.batchSelected}
                        onChange={() => onToggleBatchSelect(row.id)}
                        aria-label={`Select ${row.title}`}
                        onClick={(e) => e.stopPropagation()}
                      />
                      <m.button
                        type="button"
                        className="downloads-item-row"
                        onClick={() => onSelect(row.id)}
                        title={row.sourcePath}
                        layout
                        whileHover={rowHover}
                        whileTap={rowPress}
                      >
                        <div className="downloads-item-main">
                          <strong>{row.title}</strong>
                          {row.creatorName ? (
                            <span className="downloads-item-creator">{row.creatorName}</span>
                          ) : null}
                          <span>{row.meta}</span>
                          <div className="downloads-item-samples">{row.summary}</div>
                          {row.samples ? (
                            <div className="downloads-item-samples downloads-item-samples-muted">
                              {row.samples}
                            </div>
                          ) : null}
                        </div>

                        <div className="downloads-item-meta">
                          {row.badges.map((badge) => (
                            <span
                              key={`${row.id}-${badge.label}`}
                              className={`confidence-badge ${badge.tone}`}
                            >
                              {badge.label}
                            </span>
                          ))}
                        </div>
                      </m.button>
                    </div>
                  ))}
                </m.div>
              </div>
            ) : (
              <StatePanel
                eyebrow="Downloads lane"
                title={`Nothing is in ${downloadsLaneLabel(lane, userView).toLowerCase()} right now`}
                body={downloadsLaneHint(lane, userView)}
                icon={Inbox}
                compact
                badge="Lane clear"
              />
            )
          ) : (
            <StatePanel
              eyebrow="Downloads inbox"
              title={
                userView === "beginner"
                  ? "No inbox items match this view"
                  : "No download items match the current filter"
              }
              body={
                userView === "beginner"
                  ? "Try clearing the search, changing the filter, or refresh the inbox after a new download lands."
                  : "Clear the search, adjust status filters, or refresh the inbox to pull in newly detected downloads."
              }
              icon={Inbox}
              compact
              badge="Queue clear"
              meta={["Filters stay local to this workspace"]}
            />
          )}
        </m.div>
        {footer}
      </div>
    </div>
  );
}
