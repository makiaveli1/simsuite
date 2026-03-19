import type { DownloadQueueLane, UserView } from "../../lib/types";

const DOWNLOADS_LANE_PRIORITY: DownloadQueueLane[] = [
  "waiting_on_you",
  "special_setup",
  "ready_now",
  "blocked",
  "done",
];

export type DownloadsLaneCounts = Record<DownloadQueueLane, number>;

export function pickInitialDownloadsLane(counts: DownloadsLaneCounts) {
  return (
    DOWNLOADS_LANE_PRIORITY.find((lane) => counts[lane] > 0) ?? "ready_now"
  );
}

export function fallbackDownloadsLane(
  preferredLane: DownloadQueueLane,
  counts: DownloadsLaneCounts,
) {
  if (counts[preferredLane] > 0) {
    return preferredLane;
  }

  return DOWNLOADS_LANE_PRIORITY.find((lane) => counts[lane] > 0) ?? preferredLane;
}

export function capRowBadges(labels: string[]) {
  return labels.slice(0, 2);
}

export function viewModeDownloadsFlags(userView: UserView) {
  return {
    showAdvancedFiltersByDefault: userView === "power",
    showCompactPreview: userView !== "beginner",
    showExtraProofBlock: userView === "power",
  };
}
