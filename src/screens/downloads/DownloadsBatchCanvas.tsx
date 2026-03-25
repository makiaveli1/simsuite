import type { ReactNode } from "react";
import { m } from "motion/react";
import { downloadsSelectionTransition } from "../../lib/motion";
import type { DownloadQueueLane, UserView } from "../../lib/types";

interface DownloadsBatchCanvasProps {
  lane: DownloadQueueLane;
  userView: UserView;
  selectionTitle?: string | null;
  summary: string;
  safeCount: number;
  reviewCount: number;
  unchangedCount: number;
  previewItems: string[];
  children?: ReactNode;
}

export function DownloadsBatchCanvas({
  lane,
  userView,
  selectionTitle,
  summary,
  safeCount,
  reviewCount,
  unchangedCount,
  previewItems,
  children,
}: DownloadsBatchCanvasProps) {
  const eyebrow = batchCanvasEyebrow(lane, userView);
  const stats = buildBatchStats({
    userView,
    safeCount,
    reviewCount,
    unchangedCount,
    previewCount: previewItems.length,
  });

  return (
    <m.div
      className={`panel-card downloads-preview-panel downloads-batch-canvas workbench-panel downloads-batch-canvas-lane-${lane}`}
      layout
      transition={downloadsSelectionTransition}
    >
      <m.div
        className="panel-heading downloads-batch-header"
        layout
        transition={downloadsSelectionTransition}
      >
        <div>
          <p className="eyebrow">{eyebrow}</p>
          <h2>{selectionTitle ?? batchCanvasTitle(lane, userView)}</h2>
          <p className="downloads-batch-summary">{summary}</p>
        </div>
        <div
          className={`downloads-batch-stats downloads-batch-stats-${userView}`}
          aria-label="Batch summary"
        >
          {stats.map((stat) => (
            <div
              key={stat.label}
              className={`downloads-batch-stat downloads-batch-stat-${stat.tone}`}
            >
              <span>{stat.label}</span>
              <strong>{stat.value.toLocaleString()}</strong>
            </div>
          ))}
        </div>
      </m.div>

      <m.div
        className="preview-list downloads-preview-list downloads-batch-body"
        layoutScroll
      >
        {children}
      </m.div>
    </m.div>
  );
}

function buildBatchStats({
  userView,
  safeCount,
  reviewCount,
  unchangedCount,
  previewCount,
}: {
  userView: UserView;
  safeCount: number;
  reviewCount: number;
  unchangedCount: number;
  previewCount: number;
}) {
  const filesShownLabel = userView === "beginner"
    ? (previewCount === 1 ? "File shown" : "Files shown")
    : "Preview";
  const allStats = [
    { label: "Safe", value: safeCount, tone: "good" },
    { label: userView === "beginner" ? "Needs care" : "Review", value: reviewCount, tone: "warn" },
    { label: userView === "beginner" ? "Already set" : "Already fine", value: unchangedCount, tone: "neutral" },
    { label: filesShownLabel, value: previewCount, tone: "muted" },
  ] as const;

  if (userView === "power") {
    return allStats;
  }

  if (userView === "standard") {
    return allStats.filter((stat) => stat.value > 0 || stat.label === "Preview").slice(0, 3);
  }

  const beginnerStats = allStats.filter(
    (stat) => stat.label === "Needs care" || stat.label === filesShownLabel || stat.value > 0,
  );

  return beginnerStats.slice(0, 2);
}

function batchCanvasEyebrow(lane: DownloadQueueLane, userView: UserView) {
  switch (lane) {
    case "ready_now":
      return userView === "beginner" ? "Safe hand-off" : "Ready now";
    case "special_setup":
      return "Special setup";
    case "waiting_on_you":
      return userView === "beginner" ? "Waiting on you" : "Decision needed";
    case "blocked":
      return "Blocked";
    case "done":
      return "Done";
    default:
      return "Batch";
  }
}

function batchCanvasTitle(lane: DownloadQueueLane, userView: UserView) {
  switch (lane) {
    case "ready_now":
      return userView === "beginner"
        ? "What can move safely now"
        : "Safe hand-off preview";
    case "special_setup":
      return userView === "beginner"
        ? "How this setup should continue"
        : "Guided setup path";
    case "waiting_on_you":
      return userView === "beginner"
        ? "One more choice is needed"
        : "This batch needs one more choice";
    case "blocked":
      return userView === "beginner"
        ? "Why this batch is blocked"
        : "This batch is still blocked";
    case "done":
      return userView === "beginner"
        ? "What already happened"
        : "Completion story";
    default:
      return "Selected batch";
  }
}
