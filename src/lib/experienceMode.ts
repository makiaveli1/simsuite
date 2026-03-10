import type {
  DuplicatesLayoutPreset,
  ExperienceMode,
  LibraryLayoutPreset,
  ReviewLayoutPreset,
  Screen,
  UserView,
} from "./types";

export interface ExperienceModeProfile {
  id: ExperienceMode;
  label: string;
  badge: string;
  summary: string;
  workspaceSummary: string;
  primaryScreens: Screen[];
  toolScreens: Screen[];
  defaults: {
    libraryLayoutPreset: LibraryLayoutPreset;
    reviewLayoutPreset: ReviewLayoutPreset;
    duplicatesLayoutPreset: DuplicatesLayoutPreset;
    libraryFiltersCollapsed: boolean;
    duplicatesFiltersCollapsed: boolean;
  };
}

export const EXPERIENCE_MODE_ORDER: ExperienceMode[] = [
  "casual",
  "seasoned",
  "creator",
];

export const EXPERIENCE_MODE_PROFILES: Record<
  ExperienceMode,
  ExperienceModeProfile
> = {
  casual: {
    id: "casual",
    label: "Casual",
    badge: "Easygoing",
    summary: "Simple, guided, and calm for day-to-day play sessions.",
    workspaceSummary: "Straightforward flow with the safest next step up front.",
    primaryScreens: ["home", "downloads", "organize", "review", "library", "settings"],
    toolScreens: ["creatorAudit", "categoryAudit", "duplicates"],
    defaults: {
      libraryLayoutPreset: "browse",
      reviewLayoutPreset: "focus",
      duplicatesLayoutPreset: "sweep",
      libraryFiltersCollapsed: false,
      duplicatesFiltersCollapsed: true,
    },
  },
  seasoned: {
    id: "seasoned",
    label: "Seasoned",
    badge: "Balanced",
    summary: "Balanced workflow with enough proof to stay confident while you sort.",
    workspaceSummary: "A steady desktop workbench with guidance and detail in balance.",
    primaryScreens: [
      "home",
      "downloads",
      "library",
      "creatorAudit",
      "categoryAudit",
      "duplicates",
      "organize",
      "review",
      "settings",
    ],
    toolScreens: [],
    defaults: {
      libraryLayoutPreset: "browse",
      reviewLayoutPreset: "balanced",
      duplicatesLayoutPreset: "balanced",
      libraryFiltersCollapsed: false,
      duplicatesFiltersCollapsed: false,
    },
  },
  creator: {
    id: "creator",
    label: "Creator",
    badge: "Full receipts",
    summary: "Dense, tool-forward, and ready for deeper cleanup or authoring passes.",
    workspaceSummary: "More evidence, more control, and more of the system visible at once.",
    primaryScreens: [
      "home",
      "downloads",
      "organize",
      "library",
      "creatorAudit",
      "categoryAudit",
      "review",
      "duplicates",
      "settings",
    ],
    toolScreens: [],
    defaults: {
      libraryLayoutPreset: "inspect",
      reviewLayoutPreset: "queue",
      duplicatesLayoutPreset: "compare",
      libraryFiltersCollapsed: false,
      duplicatesFiltersCollapsed: false,
    },
  },
};

export function normalizeExperienceMode(
  value: string | null | undefined,
): ExperienceMode | null {
  switch (value) {
    case "casual":
    case "seasoned":
    case "creator":
      return value;
    case "beginner":
      return "casual";
    case "standard":
      return "seasoned";
    case "power":
      return "creator";
    default:
      return null;
  }
}

export function experienceModeToLegacyView(mode: ExperienceMode): UserView {
  switch (mode) {
    case "casual":
      return "beginner";
    case "creator":
      return "power";
    default:
      return "standard";
  }
}

export function legacyViewToExperienceMode(view: UserView): ExperienceMode {
  switch (view) {
    case "beginner":
      return "casual";
    case "power":
      return "creator";
    default:
      return "seasoned";
  }
}

export function getExperienceModeProfile(
  mode: ExperienceMode | UserView,
): ExperienceModeProfile {
  const normalized =
    normalizeExperienceMode(mode) ?? legacyViewToExperienceMode(mode as UserView);
  return EXPERIENCE_MODE_PROFILES[normalized];
}

export function isToolScreen(
  mode: ExperienceMode,
  screen: Screen,
): boolean {
  return EXPERIENCE_MODE_PROFILES[mode].toolScreens.includes(screen);
}
