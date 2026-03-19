import type { HomeOverview, UiDensity, UserView } from "../lib/types";

export type HomeHeroFocus = "health" | "watch" | "setup";
export type HomeDetailLevel = "calm" | "balanced" | "detailed";
export type HomeModuleId = "snapshot" | "health" | "watch" | "folders" | "library";

export interface HomeDisplayPrefs {
  focus: HomeHeroFocus;
  detailLevel: HomeDetailLevel;
  ambientMotion: boolean;
  visibleModules: Record<HomeModuleId, boolean>;
}

export const HOME_PREFS_PREFIX = "simsuite:home-display";
export const HOME_MODULE_ORDER: HomeModuleId[] = [
  "snapshot",
  "health",
  "watch",
  "folders",
  "library",
];

export const HOME_MODULE_LABELS: Record<HomeModuleId, string> = {
  snapshot: "Today snapshot",
  health: "System health",
  watch: "Update watch",
  folders: "Folders",
  library: "Library facts",
};

export const HOME_DETAIL_OPTIONS: Array<{
  id: HomeDetailLevel;
  label: string;
  hint: string;
}> = [
  { id: "calm", label: "Calm", hint: "Keep the page softer and shorter." },
  {
    id: "balanced",
    label: "Balanced",
    hint: "Show the best mix of calm and context.",
  },
  { id: "detailed", label: "Detailed", hint: "Keep more facts visible at once." },
];

export const HOME_DENSITY_OPTIONS: Array<{
  id: UiDensity;
  label: string;
  hint: string;
}> = [
  { id: "compact", label: "Snug", hint: "Fit more on screen." },
  { id: "balanced", label: "Normal", hint: "Best default balance." },
  { id: "roomy", label: "Roomy", hint: "Add more breathing room." },
];

export function defaultHomePrefs(userView: UserView): HomeDisplayPrefs {
  if (userView === "beginner") {
    return {
      focus: "setup",
      detailLevel: "calm",
      ambientMotion: true,
      visibleModules: {
        snapshot: true,
        health: true,
        watch: false,
        folders: true,
        library: false,
      },
    };
  }

  if (userView === "power") {
    return {
      focus: "watch",
      detailLevel: "detailed",
      ambientMotion: true,
      visibleModules: {
        snapshot: true,
        health: true,
        watch: true,
        folders: true,
        library: true,
      },
    };
  }

  return {
    focus: "health",
    detailLevel: "balanced",
    ambientMotion: true,
    visibleModules: {
      snapshot: true,
      health: true,
      watch: true,
      folders: true,
      library: false,
    },
  };
}

function normalizeVisibleModules(
  value: Partial<Record<HomeModuleId, boolean>> | undefined,
  defaults: Record<HomeModuleId, boolean>,
) {
  return {
    snapshot: value?.snapshot ?? defaults.snapshot,
    health: value?.health ?? defaults.health,
    watch: value?.watch ?? defaults.watch,
    folders: value?.folders ?? defaults.folders,
    library: value?.library ?? defaults.library,
  };
}

export function normalizeHomePrefs(
  value: Partial<HomeDisplayPrefs> | null | undefined,
  userView: UserView,
): HomeDisplayPrefs {
  const defaults = defaultHomePrefs(userView);
  return {
    focus:
      value?.focus === "health" || value?.focus === "watch" || value?.focus === "setup"
        ? value.focus
        : defaults.focus,
    detailLevel:
      value?.detailLevel === "calm" ||
      value?.detailLevel === "balanced" ||
      value?.detailLevel === "detailed"
        ? value.detailLevel
        : defaults.detailLevel,
    ambientMotion:
      typeof value?.ambientMotion === "boolean"
        ? value.ambientMotion
        : defaults.ambientMotion,
    visibleModules: normalizeVisibleModules(value?.visibleModules, defaults.visibleModules),
  };
}

export function readHomePrefs(userView: UserView) {
  const raw = globalThis.localStorage?.getItem(`${HOME_PREFS_PREFIX}:${userView}`);
  if (!raw) {
    return defaultHomePrefs(userView);
  }

  try {
    return normalizeHomePrefs(JSON.parse(raw) as Partial<HomeDisplayPrefs>, userView);
  } catch {
    return defaultHomePrefs(userView);
  }
}

export function saveHomePrefs(userView: UserView, prefs: HomeDisplayPrefs) {
  globalThis.localStorage?.setItem(
    `${HOME_PREFS_PREFIX}:${userView}`,
    JSON.stringify(prefs),
  );
}

export function describeHomeModule(moduleId: HomeModuleId) {
  switch (moduleId) {
    case "snapshot":
      return "A small read of the active queues.";
    case "health":
      return "Scan truth, safety, and current status.";
    case "watch":
      return "What your tracked pages are telling you.";
    case "folders":
      return "Library roots and setup readiness.";
    case "library":
      return "Counts and receipts about the library shape.";
  }
}

export function getHomeGreeting(date: Date) {
  const hours = date.getHours();
  if (hours < 12) {
    return "Good morning";
  }
  if (hours < 18) {
    return "Good afternoon";
  }
  return "Good evening";
}

export function formatHomeTime(date: Date) {
  return date.toLocaleTimeString([], {
    hour: "numeric",
    minute: "2-digit",
  });
}

export function formatTimestamp(value: string | null | undefined) {
  if (!value) {
    return "Not scanned yet";
  }

  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    return value;
  }

  return date.toLocaleString();
}

export function buildHeroState({
  focus,
  overview,
  userView,
  sourceCount,
  totalWatchCount,
  totalAttentionCount,
}: {
  focus: HomeHeroFocus;
  overview: HomeOverview | null;
  userView: UserView;
  sourceCount: number;
  totalWatchCount: number;
  totalAttentionCount: number;
}) {
  const hasSetupGap = sourceCount < 3;
  const hasRefreshGap = Boolean(overview?.scanNeedsRefresh);
  const riskyCount = overview?.unsafeCount ?? 0;
  const reviewCount = overview?.reviewCount ?? 0;
  const confirmedUpdates = overview?.exactUpdateItems ?? 0;
  const possibleUpdates = overview?.possibleUpdateItems ?? 0;
  const setupItems = overview?.watchSetupItems ?? 0;

  if (hasSetupGap || focus === "setup") {
    return {
      tone: hasSetupGap ? "warn" : "good",
      eyebrow: "Home focus",
      title: hasSetupGap
        ? "Finish folder setup first"
        : "Folder setup already looks steady",
      summary: hasSetupGap
        ? "The app can stay much calmer once the missing library roots are linked, because scans, inbox checks, and tray reads all know where to look."
        : "All three library roots are linked, so SimSuite can keep using one steady desktop flow.",
      footnote: hasSetupGap
        ? "Setup work belongs here because it changes how the rest of the app behaves."
        : "You can still change folders any time, but nothing urgent is missing right now.",
      metrics: [
        {
          label: "Folders ready",
          value: `${sourceCount}/3`,
          note: hasSetupGap ? "One or more roots still missing" : "All roots linked",
        },
        {
          label: "Downloads path",
          value: sourceCount >= 3 ? "Ready" : "Check",
          note: "Inbox watching depends on this path",
        },
        {
          label: "Last scan",
          value: hasRefreshGap ? "Refresh" : "Current",
          note: hasRefreshGap ? "Stored facts are older than the rules" : "Facts match the latest scan",
        },
      ],
    };
  }

  if (focus === "watch") {
    const tone =
      confirmedUpdates > 0 || setupItems > 0 || possibleUpdates > 0 ? "warn" : "good";
    return {
      tone,
      eyebrow: "Home focus",
      title:
        totalWatchCount > 0
          ? "Your tracked pages have a small story today"
          : "Tracked pages look quiet right now",
      summary:
        totalWatchCount > 0
          ? userView === "beginner"
            ? "A few tracked items want a closer look, but the watch lane is still easy to read."
            : "Saved pages, version checks, and setup gaps are in a good place to skim without opening a heavier workspace."
          : "Nothing in the watched page flow is asking for attention right now.",
      footnote:
        totalWatchCount > 0
          ? "This stays as a summary on Home so the page stays calm. The full queue still lives in Updates."
          : "Home only keeps the summary. Deeper proof stays tucked away in Updates.",
      metrics: [
        {
          label: "Confirmed",
          value: String(confirmedUpdates),
          note: "Likely newer versions waiting",
        },
        {
          label: "Need source",
          value: String(setupItems),
          note: "Installed items without a saved page",
        },
        {
          label: "Possible",
          value: String(possibleUpdates),
          note: "Changed pages that still need caution",
        },
      ],
    };
  }

  const healthTone =
    hasRefreshGap || riskyCount > 0 || reviewCount > 0 ? "warn" : "good";

  return {
    tone: healthTone,
    eyebrow: "Home focus",
    title:
      healthTone === "good"
        ? "Your library looks steady"
        : "A few things want a closer look",
    summary:
      healthTone === "good"
        ? "The library, safety checks, and watched-page picture all look aligned, so Home can stay mostly quiet."
        : userView === "beginner"
          ? "Nothing looks chaotic, but there are a few small signs worth keeping in view before you settle in."
          : "The important signals are still readable at a glance: scan freshness, risky files, and follow-up work are the pieces worth noticing first.",
    footnote:
      totalAttentionCount > 0
        ? `${totalAttentionCount.toLocaleString()} items are sitting somewhere in the app, but Home is only surfacing the calm summary.`
        : "No queues are pressing hard enough to turn Home into a workbench.",
    metrics: [
      {
        label: "Scan",
        value: hasRefreshGap ? "Refresh" : "Current",
        note: hasRefreshGap ? "Older than current rules" : "Fresh enough to trust",
      },
      {
        label: "Risky files",
        value: String(riskyCount),
        note: riskyCount > 0 ? "Worth keeping in sight" : "No active risk lane",
      },
      {
        label: "Review",
        value: String(reviewCount),
        note: reviewCount > 0 ? "Human checks still waiting" : "Nothing waiting",
      },
    ],
  };
}
