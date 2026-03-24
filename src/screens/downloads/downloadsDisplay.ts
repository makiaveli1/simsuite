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
      return "Ready now";
    case "special_setup":
      return "Special setup";
    case "waiting_on_you":
      return userView === "beginner" ? "Waiting on you" : "Waiting on you";
    case "blocked":
      return "Blocked";
    case "done":
      return "Done";
    case "rejected":
      return "Rejected";
    default:
      return "Inbox";
  }
}

export function downloadsLaneHint(lane: DownloadQueueLane, userView: UserView) {
  switch (lane) {
    case "ready_now":
      return userView === "beginner"
        ? "Safe files can move from here."
        : "Normal batches ready for a safe hand-off.";
    case "special_setup":
      return userView === "beginner"
        ? "Supported mods with their own install rules."
        : "Supported special mods that need the guided install path.";
    case "waiting_on_you":
      return userView === "beginner"
        ? "These need one more choice from you first."
        : "Dependencies, missing files, or a small decision are still in the way.";
    case "blocked":
      return userView === "beginner"
        ? "SimSuite stopped these to stay safe."
        : "Unsafe or incomplete items that cannot move yet.";
    case "done":
      return userView === "beginner"
        ? "Already handled or tucked away."
        : "Applied or hidden batches.";
    case "rejected":
      return userView === "beginner"
        ? "Moved to Reject folder for review."
        : "Moved to SimSuite_Rejected for review.";
    default:
      return "";
  }
}
