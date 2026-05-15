import type { ReactNode } from "react";
import { ArrowUp, Check, CheckCircle, Clock, Inbox, Settings, ShieldAlert } from "lucide-react";
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

export interface LibraryVersionInfo {
  libraryLabel: string | null;
  installedVersion: string | null;
  incomingVersion: string | null;
}

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
  libraryVersionInfo?: LibraryVersionInfo;
}

const LANE_ACCENTS: Record<string, string> = {
  ready_now: "var(--accent)",
  waiting_on_you: "var(--tone-warn)",
  special_setup: "var(--tone-info)",
  blocked: "var(--tone-danger)",
  done: "var(--text-dim)",
  rejected: "var(--text-dim)",
};

const LANE_LABELS: Record<string, string> = {
  ready_now: "Ready for Review",
  waiting_on_you: "Waiting on You",
  special_setup: "Special Setup",
  blocked: "Needs Review",
  done: "Reviewed",
  rejected: "Set Aside",
};

const LANE_ICONS: Record<string, ReactNode> = {
  ready_now: <Check size={16} />,
  waiting_on_you: <Clock size={16} />,
  special_setup: <Settings size={16} />,
  blocked: <ShieldAlert size={16} />,
  done: <CheckCircle size={16} />,
  rejected: <CheckCircle size={16} />,
};

const BANNER_CONTENT: Record<string, { title: string; body: string }> = {
  ready_now: {
    title: "Ready for review",
    body: "These batches have enough local information to inspect. No files are changed from this view.",
  },
  waiting_on_you: {
    title: "Needs your input",
    body: "Something about these needs your attention. Select one to inspect the intake details.",
  },
  special_setup: {
    title: "Has special setup steps",
    body: "These need extra review steps before any future setup workflow can be considered.",
  },
  blocked: {
    title: "Held for review",
    body: "SimSuite stopped here because the local evidence is incomplete or unclear.",
  },
  done: {
    title: "Already reviewed",
    body: "These batches were handled earlier and stay here for reference.",
  },
  rejected: {
    title: "Set aside",
    body: "These batches are outside the active intake queue and remain visible for review.",
  },
};

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
  onOpenInLibrary?: () => void;
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
  onOpenInLibrary,
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
                    className={`downloads-lane-sticky-header downloads-lane-sticky-header-lane-${lane}`}
                  >
                    <span className="downloads-lane-sticky-icon">
                      {LANE_ICONS[lane]}
                    </span>
                    <span className="downloads-lane-sticky-title">
                      {LANE_LABELS[lane]}
                    </span>
                    <span className="downloads-lane-sticky-count">
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
                      <m.div
                        role="button"
                        tabIndex={0}
                        className="downloads-item-row"
                        onClick={() => onSelect(row.id)}
                        onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') onSelect(row.id); }}
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
                          {row.libraryVersionInfo ? (
                            <div className="version-compare-chip">
                              <ArrowUp size={11} strokeWidth={2.5} />
                              <span>
                                {row.libraryVersionInfo.installedVersion
                                  ? `v${row.libraryVersionInfo.installedVersion} in Library → `
                                  : "Library → "}
                                v{row.libraryVersionInfo.incomingVersion ?? "?"} available
                              </span>
                              {onOpenInLibrary && (
                                <button
                                  className="open-in-library-btn"
                                  onClick={(e) => {
                                    e.stopPropagation();
                                    onOpenInLibrary();
                                  }}
                                >
                                  Open in Library
                                </button>
                              )}
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
                      </m.div>
                    </div>
                  ))}
                </m.div>
              </div>
            ) : (
              <StatePanel
                eyebrow="Inbox lane"
                title={`Nothing is in ${downloadsLaneLabel(lane, userView).toLowerCase()} right now`}
                body={downloadsLaneHint(lane, userView)}
                icon={Inbox}
                compact
                badge="Lane clear"
              />
            )
          ) : (
            <StatePanel
              eyebrow="Inbox"
              title={
                userView === "beginner"
                  ? "No inbox items match this view"
                  : "No inbox items match the current filter"
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
