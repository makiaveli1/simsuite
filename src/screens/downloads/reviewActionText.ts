import type { ReviewPlanAction, UserView } from "../../lib/types";

function stripLeadingVerb(label: string, verb: string) {
  const prefix = `${verb} `;
  return label.toLowerCase().startsWith(prefix.toLowerCase())
    ? label.slice(prefix.length)
    : label;
}

export function reviewActionCardTitle(action: ReviewPlanAction) {
  switch (action.kind) {
    case "open_related_item":
      return action.relatedItemName ?? stripLeadingVerb(action.label, "Use");
    case "install_dependency":
      return action.relatedItemName ?? stripLeadingVerb(action.label, "Install");
    case "open_dependency":
      return action.relatedItemName ?? stripLeadingVerb(action.label, "Open");
    case "open_official_source":
      return action.relatedItemName ?? "Official download page";
    case "download_missing_files":
      return "Missing files";
    case "separate_supported_files":
      return "Supported files";
    case "repair_special":
      return "Repair special setup";
    default:
      return action.label;
  }
}

export function reviewActionButtonLabel(
  action: ReviewPlanAction,
  userView: UserView,
  isApplying: boolean,
) {
  if (isApplying) {
    switch (action.kind) {
      case "repair_special":
        return userView === "beginner" ? "Fixing old setup..." : "Repairing setup...";
      case "install_dependency":
        return userView === "beginner" ? "Installing helper..." : "Installing dependency...";
      case "open_related_item":
        return userView === "beginner" ? "Opening better pack..." : "Opening fuller pack...";
      case "download_missing_files":
        return userView === "beginner" ? "Downloading files..." : "Downloading missing files...";
      case "separate_supported_files":
        return userView === "beginner" ? "Splitting files..." : "Separating supported files...";
      case "open_dependency":
        return userView === "beginner" ? "Opening dependency..." : "Opening dependency...";
      case "open_official_source":
        return userView === "beginner" ? "Opening page..." : "Opening official page...";
      default:
        return action.label;
    }
  }

  switch (action.kind) {
    case "open_related_item":
      return "Use this pack";
    case "install_dependency":
      return userView === "power" ? "Install dependency" : "Install helper";
    case "open_dependency":
      return userView === "beginner" ? "Open helper" : "Open dependency";
    case "open_official_source":
      return userView === "beginner" ? "Open page" : "Open source";
    case "download_missing_files":
      return userView === "beginner" ? "Get files" : "Download files";
    case "separate_supported_files":
      return userView === "beginner" ? "Split files" : "Separate files";
    case "repair_special":
      return userView === "beginner" ? "Fix setup" : "Repair setup";
    default:
      return action.label;
  }
}
