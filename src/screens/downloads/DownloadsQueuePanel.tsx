import type { ReactNode } from "react";
import { Inbox } from "lucide-react";
import { m } from "motion/react";
import { StatePanel } from "../../components/StatePanel";
import { rowHover, rowPress, stagedListItem } from "../../lib/motion";
import type { DownloadQueueLane, UserView } from "../../lib/types";
import { downloadsLaneHint, downloadsLaneLabel } from "./downloadsDisplay";

export interface DownloadsQueueRowModel {
  id: number;
  title: string;
  meta: string;
  summary: string;
  samples?: string | null;
  badges: Array<{
    label: string;
    tone: string;
  }>;
  tone: "good" | "medium" | "low" | "neutral";
  selected: boolean;
  sourcePath: string;
}

interface DownloadsQueuePanelProps {
  lane: DownloadQueueLane;
  userView: UserView;
  rows: DownloadsQueueRowModel[];
  isLoading: boolean;
  hasItems: boolean;
  onSelect: (id: number) => void;
  footer?: ReactNode;
}

export function DownloadsQueuePanel({
  lane,
  userView,
  rows,
  isLoading,
  hasItems,
  onSelect,
  footer,
}: DownloadsQueuePanelProps) {
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
        <div className="queue-list downloads-queue-list">
          {hasItems ? (
            rows.length ? (
              <div className="downloads-lane-group">
                <div className="downloads-lane-header">
                  <div>
                    <strong>{downloadsLaneLabel(lane, userView)}</strong>
                    <span>{downloadsLaneHint(lane, userView)}</span>
                  </div>
                  <span className="ghost-chip">{rows.length.toLocaleString()}</span>
                </div>

                <div className="downloads-lane-list">
                  {rows.map((row, index) => (
                    <m.button
                      key={row.id}
                      type="button"
                      className={`downloads-item-row ${
                        row.selected ? "is-selected" : ""
                      } downloads-item-row-${row.tone}`}
                      onClick={() => onSelect(row.id)}
                      title={row.sourcePath}
                      whileHover={rowHover}
                      whileTap={rowPress}
                      {...stagedListItem(index)}
                    >
                      <div className="downloads-item-main">
                        <strong>{row.title}</strong>
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
                  ))}
                </div>
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
        </div>
        {footer}
      </div>
    </div>
  );
}
