import type { ExperienceMode, Screen, UserView, CreatorLearningInfo, VersionConfidence } from "./types";
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
  staging: { default: "Plan Preview" },
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

// ─── Creator disclosure tiers ──────────────────────────────────────────────
//
// Tier 1 — Known: user saved this creator override → "by [Creator]"
// Tier 2 — Strong hint: manifest or embedded name evidence → "likely by [Creator]"
// Tier 3 — Weak inference: path-folder guess only → "probably [Creator]"
// Tier 4 — Unknown: no meaningful clues → "Unknown creator"
//
// creatorHints: string[] from FileInsights — raw embedded name patterns found
// confidence: 0–1 overall file parse confidence (not creator-specific)
// ──────────────────────────────────────────────────────────────────────────
export type CreatorConfidenceTier = "known" | "strong_hint" | "weak_inference" | "unknown";

export function creatorConfidenceTier(
  creator: string | null,
  creatorLearning: CreatorLearningInfo | null,
  creatorHints: string[],
): CreatorConfidenceTier {
  // Tier 1: user locked this creator
  if (creator && creatorLearning?.lockedByUser) {
    return "known";
  }
  // Tier 2: creator present + strong embedded evidence
  if (creator && creatorHints.length > 0) {
    return "strong_hint";
  }
  // Tier 3: creator present but only path inference (no embedded evidence)
  if (creator) {
    return "weak_inference";
  }
  // Tier 4: no creator at all
  return "unknown";
}

export function creatorLabelWithConfidence(
  creator: string | null,
  creatorLearning: CreatorLearningInfo | null,
  creatorHints: string[],
): string {
  const tier = creatorConfidenceTier(creator, creatorLearning, creatorHints);
  switch (tier) {
    case "known":
      return creator ?? "Unknown creator";
    case "strong_hint":
      return `${creator}?`;
    case "weak_inference":
      return `? ${creator}`;
    case "unknown":
    default:
      return "Unknown creator";
  }
}

/** Returns a suffix label describing creator certainty for use in tooltips or secondary labels */
export function creatorConfidenceSuffix(
  creator: string | null,
  creatorLearning: CreatorLearningInfo | null,
  creatorHints: string[],
): string | null {
  const tier = creatorConfidenceTier(creator, creatorLearning, creatorHints);
  switch (tier) {
    case "known":
      return "saved";
    case "strong_hint":
      return "from file";
    case "weak_inference":
      return "from folder";
    case "unknown":
    default:
      return null;
  }
}

// ─── Version confidence ────────────────────────────────────────────────────────
// InstalledVersionSummary.confidence tells us how certain the version signal is.
// ──────────────────────────────────────────────────────────────────────────────
export function versionLabelWithConfidence(
  version: string | null,
  confidence: VersionConfidence | null,
): string | null {
  if (!version) return null;
  switch (confidence) {
    case "exact":
    case "strong":
      return version;
    case "medium":
      return `${version}?`;
    case "weak":
      return `~${version}`;
    case "unknown":
    default:
      return null; // don't show version when confidence is unknown
  }
}

export function versionConfidenceTierLabel(confidence: VersionConfidence | null): string {
  switch (confidence) {
    case "exact":
      return "Confirmed";
    case "strong":
      return "Strong";
    case "medium":
      return "Possible";
    case "weak":
      return "Speculative";
    case "unknown":
    default:
      return "Unconfirmed";
  }
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
    organizeBeginner: "Think of this as Build/Buy for your Mods folder: preview the plan first, then review the messy bits.",
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
      beginner: "Review new downloads and imported batches before they become normal Library work.",
      standard: "Inbox owns new and imported content before it moves into Library or Organize planning.",
      power: "Imported batches, guided review, and blocked oddballs queue here before planning work starts.",
    },
    library: {
      beginner: "Pick a file, check the basics, then save the right Creator or Type once.",
      standard: "Browse the indexed library and fix the details SimSuite should remember next time.",
      power: "Use the full file desk for paths, clues, confidence, and learned overrides.",
    },
    updates: {
      beginner: "See which files need a source, which ones are watched, and which ones need a closer update check.",
      standard: "Manage watch sources, review cautious update states, and check watched pages without pretending everything is automatic.",
      power: "Run the full trust-first updates desk: source setup, cautious review, watched checks, and manual follow-up lanes.",
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
      beginner: "Check exact duplicates and name matches before changing any files.",
      standard: "Compare exact duplicates, name matches, and version reviews before any file-changing actions exist.",
      power: "Inspect exact-content duplicates, filename matches, and version reviews with the fuller compare view.",
    },
    organize: {
      beginner: "Generate a preview plan, check the reasons, then decide what still needs review.",
      standard: "Preview-only organization plans, reasons, and caveats meet here before any future file-changing workflow.",
      power: "Use the full preview plan details, buckets, source signals, and blocked reasons before any future reshuffle.",
    },
    review: {
      beginner: "These files hit a speed bump. Check why, then decide the safest next stop.",
      standard: "Use Review as the hold queue for anything that still needs a human look.",
      power: "Work through the blocked items with fuller reasons, paths, and confidence clues.",
    },
    staging: {
      beginner: "Review pending plans before anything changes.",
      standard: "Use Plan Preview to inspect proposed plans without changing files.",
      power: "Use Plan Preview as the pending-plan checkpoint for future organization workflows.",
    },
    settings: {
      beginner: "Change the app feel here without touching your files or your safety rules.",
      standard: "Tune the workspace look, density, and panel feel from one calm little control lot.",
      power: "Skins, density, and panel resets live here so the work screens can stay sharp instead of chaotic.",
    },
  };

  return copy[screen][legacyView];
}
