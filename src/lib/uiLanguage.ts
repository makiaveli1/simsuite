import type { ExperienceMode, Screen, UserView } from "./types";
import {
  experienceModeToLegacyView,
  normalizeExperienceMode,
} from "./experienceMode";

const SCREEN_LABELS: Record<
  Screen,
  {
    default: string;
    beginner?: string;
    standard?: string;
    power?: string;
  }
> = {
  home: { default: "Home" },
  downloads: { default: "Inbox" },
  library: { default: "Library", beginner: "My CC" },
  updates: { default: "Updates", beginner: "Updates" },
  creatorAudit: { default: "Creators" },
  categoryAudit: { default: "Types" },
  duplicates: { default: "Duplicates", beginner: "Same file?" },
  organize: { default: "Organize", beginner: "Tidy Up" },
  review: { default: "Review" },
  staging: { default: "Staging" },
  settings: { default: "Settings" },
};

const TYPE_LABELS: Record<string, string> = {
  CAS: "CAS",
  BuildBuy: "Build/Buy",
  Gameplay: "Gameplay",
  ScriptMods: "Script Mods",
  OverridesAndDefaults: "Overrides & Defaults",
  PosesAndAnimation: "Poses & Animation",
  PresetsAndSliders: "Presets & Sliders",
  TrayHousehold: "Household",
  TrayLot: "Lot",
  TrayRoom: "Room",
  TrayItem: "Tray Item",
  Unknown: "Unknown",
};

function normalizeUserView(userView: UserView | ExperienceMode): UserView {
  const mode = normalizeExperienceMode(userView);
  if (!mode) {
    return userView as UserView;
  }

  return experienceModeToLegacyView(mode);
}

export function screenLabel(screen: Screen, userView: UserView | ExperienceMode) {
  const labels = SCREEN_LABELS[screen];
  const legacyView = normalizeUserView(userView);
  return labels[legacyView] ?? labels.default;
}

export function backdropScreenLabel(screen: Screen) {
  return SCREEN_LABELS[screen].default;
}

export function friendlyTypeLabel(kind: string) {
  if (TYPE_LABELS[kind]) {
    return TYPE_LABELS[kind];
  }

  return kind
    .replace(/([a-z0-9])([A-Z])/g, "$1 $2")
    .replace(/[._-]+/g, " ")
    .trim();
}

export function reviewLabel(userView: UserView | ExperienceMode) {
  return normalizeUserView(userView) === "beginner" ? "Needs review" : "Review";
}

export function reviewStateLabel(userView: UserView | ExperienceMode) {
  return normalizeUserView(userView) === "beginner" ? "Needs review" : "Review";
}

export function unknownCreatorLabel(userView: UserView | ExperienceMode) {
  return normalizeUserView(userView) === "beginner"
    ? "Unknown creator"
    : "Unknown creator";
}

export function intakeModeLabel(mode: string) {
  switch (mode) {
    case "guided":
      return "Special setup";
    case "needs_review":
      return "Needs review";
    case "blocked":
      return "Blocked";
    default:
      return "Normal";
  }
}

export function riskLevelLabel(level: string) {
  switch (level) {
    case "high":
      return "High care";
    case "medium":
      return "Extra care";
    default:
      return "Low care";
  }
}

export function viewModeLabel(userView: UserView | ExperienceMode) {
  const mode = normalizeExperienceMode(userView);
  switch (mode) {
    case "casual":
      return "Casual";
    case "creator":
      return "Creator";
    default:
      return "Seasoned";
  }
}

export function sampleToggleLabel(
  showingAll: boolean,
) {
  if (showingAll) {
    return "Show examples only";
  }

  return "Show all files";
}

export function sampleCountLabel(
  visibleCount: number,
  totalCount: number,
  showingAll: boolean,
) {
  if (showingAll || visibleCount >= totalCount) {
    return `All ${totalCount.toLocaleString()} files shown`;
  }

  return `${visibleCount.toLocaleString()} shown of ${totalCount.toLocaleString()} checked`;
}

export function simsFlavorLine(
  key:
    | "homeBeginner"
    | "inboxBeginner"
    | "organizeBeginner"
    | "reviewBeginner"
    | "libraryBeginner"
    | "settingsBeginner"
    | "guideBeginner",
) {
  const lines: Record<typeof key, string> = {
    homeBeginner: "Your SimSuite home lot: folders ready, next steps up front.",
    inboxBeginner: "Fresh downloads land here first, so nothing sneaky reaches your game unchecked.",
    organizeBeginner: "Think of this as Build/Buy for your Mods folder: safe moves first, messy bits parked for later.",
    reviewBeginner: "These files need a quick second look before they join the household.",
    libraryBeginner: "Browse your CC like a calmer catalog instead of a maze of folders.",
    settingsBeginner: "Tune the look and feel without touching your files or save-safe rules.",
    guideBeginner: "Short answers, safe steps, and no plumbob panic.",
  };

  return lines[key];
}

export function screenHelperLine(
  screen: Screen,
  userView: UserView | ExperienceMode,
) {
  const legacyView = normalizeUserView(userView);
  const copy: Record<Screen, Record<UserView, string>> = {
    home: {
      beginner: "Set your folders, run a scan, and follow the big next step cards.",
      standard: "This is your control lot: folders, totals, and the quickest next stop.",
      power: "Roots, counts, and jump points stay here so the deeper work screens can stay lean.",
    },
    downloads: {
      beginner: "Fresh downloads land here first, so nothing sneaks into your game like an uninvited townie.",
      standard: "Inbox is the plumbob checkpoint before anything reaches Mods or Tray.",
      power: "Staged batches, guided installs, and blocked oddballs all queue here first.",
    },
    library: {
      beginner: "Pick a file, check the basics, then save the right Creator or Type once.",
      standard: "Browse the indexed library and fix the details SimSuite should remember next time.",
      power: "Use the full file desk for paths, clues, confidence, and learned overrides.",
    },
    updates: {
      beginner: "Check your tracked mods for updates and set up new ones.",
      standard: "Review tracked mods, set up new watch sources, and check for updates.",
      power: "Manage watch sources, review uncertain sources, and refresh all tracked mods.",
    },
    creatorAudit: {
      beginner: "Fix creator names in groups so you do not have to sort one file at a time.",
      standard: "Batch-save creator names over unresolved groups with a quick proof check first.",
      power: "Audit clue groups, inspect the raw reasons, and lock creator decisions in batches.",
    },
    categoryAudit: {
      beginner: "Fix mod Types in groups so future scans stop guessing the same files.",
      standard: "Batch-save Type choices over uncertain groups with clear examples first.",
      power: "Use grouped clues, keyword signals, and raw file reasons to lock Type decisions.",
    },
    duplicates: {
      beginner: "These look like twins. Check the pairs before you clean anything up later.",
      standard: "Use this desk to compare likely repeats before safe duplicate actions arrive.",
      power: "Inspect exact, filename, and version-style matches with the fuller compare view.",
    },
    organize: {
      beginner: "Pick a tidy style, check a sample, then let the ready files scoot into place.",
      standard: "Rule previews, safe counts, and restore points all meet here before any file does the Sims shuffle.",
      power: "Use the full validator pass, route details, and restore history before you reshuffle the whole household.",
    },
    review: {
      beginner: "These files hit a speed bump. Check why, then decide the safest next stop.",
      standard: "Use Review as the hold queue for anything that still needs a human look.",
      power: "Work through the blocked items with fuller reasons, paths, and confidence clues.",
    },
    staging: {
      beginner: "Review staged files before they go to your Library.",
      standard: "Commit or reject staged items in batches.",
      power: "Use the staging desk as a pre-Library checkpoint for bulk operations.",
    },
    settings: {
      beginner: "Change the app feel here without touching your files or your safety rules.",
      standard: "Tune the workspace look, density, and panel feel from one calm little control lot.",
      power: "Skins, density, and panel resets live here so the work screens can stay sharp instead of chaotic.",
    },
  };

  return copy[screen][legacyView];
}
