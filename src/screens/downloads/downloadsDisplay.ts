import type { DownloadQueueLane, UserView } from "../../lib/types";

const DOWNLOADS_LANE_PRIORITY: DownloadQueueLane[] = [
  "waiting_on_you",
  "special_setup",
  "ready_now",
  "blocked",
  "done",
  "rejected",
];

export type DownloadsLaneCounts = Record<DownloadQueueLane, number>;
export const DOWNLOADS_LANE_SUMMARY_ORDER: DownloadQueueLane[] = [
  "ready_now",
  "special_setup",
  "waiting_on_you",
  "blocked",
  "done",
  "rejected",
];

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
    showAdvancedFiltersByDefault: false,
    showCompactPreview: userView !== "beginner",
    showExtraProofBlock: userView === "power",
  };
}

export function downloadsLaneLabel(lane: DownloadQueueLane, userView: UserView) {
  switch (lane) {
    case "ready_now":
      return "Ready for review";
    case "special_setup":
      return "Special setup";
    case "waiting_on_you":
      return userView === "beginner" ? "Waiting on you" : "Waiting on you";
    case "blocked":
      return "Blocked";
    case "done":
      return "Reviewed";
    case "rejected":
      return "Set aside";
    default:
      return "Inbox";
  }
}

export function downloadsLaneHint(lane: DownloadQueueLane, userView: UserView) {
  switch (lane) {
    case "ready_now":
      return userView === "beginner"
        ? "These batches have enough information for review."
        : "Batches with enough local evidence to inspect before any future action.";
    case "special_setup":
      return userView === "beginner"
        ? "Supported mods with their own install rules."
        : "Supported special mods that need extra setup review.";
    case "waiting_on_you":
      return userView === "beginner"
        ? "These need one more choice from you first."
        : "One more user choice or local review step is still in the way.";
    case "blocked":
      return userView === "beginner"
        ? "SimSuite stopped here for manual review."
        : "Incomplete or unclear intake items that stay in review.";
    case "done":
      return userView === "beginner"
        ? "Already reviewed or handled earlier."
        : "Batches already marked as handled by earlier review.";
    case "rejected":
      return userView === "beginner"
        ? "Set aside from the active intake queue."
        : "Batches set aside from the active intake queue.";
    default:
      return "";
  }
}
