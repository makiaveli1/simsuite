import {
  createContext,
  useContext,
  useEffect,
  useState,
  type ReactNode,
} from "react";
import type {
  DuplicatesLayoutPreset,
  LibraryLayoutPreset,
  ReviewLayoutPreset,
  UiDensity,
} from "../lib/types";
import type { UiTheme } from "../lib/types";
import { UI_THEME_IDS } from "../lib/themeMeta";

const STORAGE_KEYS = {
  theme: "simsuite:theme",
  density: "simsuite:density",
  sidebarWidth: "simsuite:sidebar-width",
  homePrimaryWidth: "simsuite:home-primary-width",
  homeSecondaryWidth: "simsuite:home-secondary-width",
  inspectorWidth: "simsuite:inspector-width",
  guideWidth: "simsuite:guide-width",
  scannerWidth: "simsuite:scanner-width",
  auditGroupWidth: "simsuite:audit-group-width",
  auditStageHeight: "simsuite:audit-stage-height",
  organizeRailWidth: "simsuite:organize-rail-width",
  organizePreviewHeight: "simsuite:organize-preview-height",
  downloadsDetailWidth: "simsuite:downloads-detail-width",
  downloadsQueueHeight: "simsuite:downloads-queue-height",
  libraryDetailWidth: "simsuite:library-detail-width",
  libraryTableHeight: "simsuite:library-table-height",
  reviewDetailWidth: "simsuite:review-detail-width",
  reviewQueueHeight: "simsuite:review-queue-height",
  duplicatesDetailWidth: "simsuite:duplicates-detail-width",
  duplicatesQueueHeight: "simsuite:duplicates-queue-height",
  libraryFiltersCollapsed: "simsuite:library-filters-collapsed",
  duplicatesFiltersCollapsed: "simsuite:duplicates-filters-collapsed",
  libraryLayoutPreset: "simsuite:library-layout-preset",
  reviewLayoutPreset: "simsuite:review-layout-preset",
  duplicatesLayoutPreset: "simsuite:duplicates-layout-preset",
  dockLayouts: "simsuite:dock-layouts",
};

interface DockSectionLayout {
  order: string[];
  collapsed: Record<string, boolean>;
}

type DockLayoutStore = Record<string, DockSectionLayout>;

const DEFAULT_THEME: UiTheme = "plumbob";
const DEFAULT_DENSITY: UiDensity = "balanced";
const DEFAULT_SIDEBAR_WIDTH = 96;
const DEFAULT_HOME_PRIMARY_WIDTH = 360;
const DEFAULT_HOME_SECONDARY_WIDTH = 320;
const DEFAULT_INSPECTOR_WIDTH = 380;
const DEFAULT_GUIDE_WIDTH = 820;
const DEFAULT_SCANNER_WIDTH = 560;
const DEFAULT_AUDIT_GROUP_WIDTH = 316;
const DEFAULT_AUDIT_STAGE_HEIGHT = 330;
const DEFAULT_ORGANIZE_RAIL_WIDTH = 304;
const DEFAULT_ORGANIZE_PREVIEW_HEIGHT = 414;
const DEFAULT_DOWNLOADS_DETAIL_WIDTH = 420;
const DEFAULT_DOWNLOADS_QUEUE_HEIGHT = 320;
const DEFAULT_LIBRARY_DETAIL_WIDTH = 392;
const DEFAULT_LIBRARY_TABLE_HEIGHT = 510;
const DEFAULT_REVIEW_DETAIL_WIDTH = 404;
const DEFAULT_REVIEW_QUEUE_HEIGHT = 520;
const DEFAULT_DUPLICATES_DETAIL_WIDTH = 430;
const DEFAULT_DUPLICATES_QUEUE_HEIGHT = 520;
const DEFAULT_LIBRARY_LAYOUT_PRESET: LibraryLayoutPreset = "browse";
const DEFAULT_REVIEW_LAYOUT_PRESET: ReviewLayoutPreset = "balanced";
const DEFAULT_DUPLICATES_LAYOUT_PRESET: DuplicatesLayoutPreset = "balanced";
const VALID_THEMES: UiTheme[] = [...UI_THEME_IDS];
const VALID_LIBRARY_LAYOUT_PRESETS: LibraryLayoutPreset[] = [
  "browse",
  "inspect",
  "catalog",
  "custom",
];
const VALID_REVIEW_LAYOUT_PRESETS: ReviewLayoutPreset[] = [
  "queue",
  "balanced",
  "focus",
  "custom",
];
const VALID_DUPLICATES_LAYOUT_PRESETS: DuplicatesLayoutPreset[] = [
  "sweep",
  "balanced",
  "compare",
  "custom",
];

interface UiPreferencesContextValue {
  theme: UiTheme;
  density: UiDensity;
  sidebarWidth: number;
  homePrimaryWidth: number;
  homeSecondaryWidth: number;
  inspectorWidth: number;
  guideWidth: number;
  scannerWidth: number;
  auditGroupWidth: number;
  auditStageHeight: number;
  organizeRailWidth: number;
  organizePreviewHeight: number;
  downloadsDetailWidth: number;
  downloadsQueueHeight: number;
  libraryDetailWidth: number;
  libraryTableHeight: number;
  reviewDetailWidth: number;
  reviewQueueHeight: number;
  duplicatesDetailWidth: number;
  duplicatesQueueHeight: number;
  libraryFiltersCollapsed: boolean;
  duplicatesFiltersCollapsed: boolean;
  libraryLayoutPreset: LibraryLayoutPreset;
  reviewLayoutPreset: ReviewLayoutPreset;
  duplicatesLayoutPreset: DuplicatesLayoutPreset;
  getDockSectionLayout: (
    layoutId: string,
    sectionIds: string[],
    defaults?: Record<string, boolean>,
  ) => DockSectionLayout;
  setDockSectionOrder: (layoutId: string, order: string[]) => void;
  setDockSectionCollapsed: (
    layoutId: string,
    sectionId: string,
    collapsed: boolean,
  ) => void;
  resetDockSectionLayout: (layoutId: string) => void;
  setTheme: (theme: UiTheme) => void;
  setDensity: (density: UiDensity) => void;
  setSidebarWidth: (width: number) => void;
  setHomePrimaryWidth: (width: number) => void;
  setHomeSecondaryWidth: (width: number) => void;
  setInspectorWidth: (width: number) => void;
  setGuideWidth: (width: number) => void;
  setScannerWidth: (width: number) => void;
  setAuditGroupWidth: (width: number) => void;
  setAuditStageHeight: (height: number) => void;
  setOrganizeRailWidth: (width: number) => void;
  setOrganizePreviewHeight: (height: number) => void;
  setDownloadsDetailWidth: (width: number) => void;
  setDownloadsQueueHeight: (height: number) => void;
  setLibraryDetailWidth: (width: number) => void;
  setLibraryTableHeight: (height: number) => void;
  setReviewDetailWidth: (width: number) => void;
  setReviewQueueHeight: (height: number) => void;
  setDuplicatesDetailWidth: (width: number) => void;
  setDuplicatesQueueHeight: (height: number) => void;
  setLibraryFiltersCollapsed: (collapsed: boolean) => void;
  setDuplicatesFiltersCollapsed: (collapsed: boolean) => void;
  applyLibraryLayoutPreset: (preset: LibraryLayoutPreset) => void;
  applyReviewLayoutPreset: (preset: ReviewLayoutPreset) => void;
  applyDuplicatesLayoutPreset: (preset: DuplicatesLayoutPreset) => void;
  resetPanelSizes: () => void;
}

const UiPreferencesContext = createContext<UiPreferencesContextValue | null>(null);

function clamp(value: number, min: number, max: number) {
  return Math.max(min, Math.min(max, value));
}

function readStoredTheme(): UiTheme {
  const stored = globalThis.localStorage?.getItem(STORAGE_KEYS.theme);
  return VALID_THEMES.includes(stored as UiTheme)
    ? (stored as UiTheme)
    : DEFAULT_THEME;
}

function readStoredDensity(): UiDensity {
  const stored = globalThis.localStorage?.getItem(STORAGE_KEYS.density);
  return stored === "compact" || stored === "balanced" || stored === "roomy"
    ? stored
    : DEFAULT_DENSITY;
}

function readStoredBoolean(key: string, fallback: boolean) {
  const raw = globalThis.localStorage?.getItem(key);
  if (raw === "true") {
    return true;
  }

  if (raw === "false") {
    return false;
  }

  return fallback;
}

function readStoredSize(key: string, fallback: number, min: number, max: number) {
  const raw = globalThis.localStorage?.getItem(key);
  const parsed = raw ? Number(raw) : Number.NaN;
  return Number.isFinite(parsed) ? clamp(parsed, min, max) : fallback;
}

function readStoredLibraryLayoutPreset(): LibraryLayoutPreset {
  const stored = globalThis.localStorage?.getItem(STORAGE_KEYS.libraryLayoutPreset);
  return VALID_LIBRARY_LAYOUT_PRESETS.includes(stored as LibraryLayoutPreset)
    ? (stored as LibraryLayoutPreset)
    : DEFAULT_LIBRARY_LAYOUT_PRESET;
}

function readStoredReviewLayoutPreset(): ReviewLayoutPreset {
  const stored = globalThis.localStorage?.getItem(STORAGE_KEYS.reviewLayoutPreset);
  return VALID_REVIEW_LAYOUT_PRESETS.includes(stored as ReviewLayoutPreset)
    ? (stored as ReviewLayoutPreset)
    : DEFAULT_REVIEW_LAYOUT_PRESET;
}

function readStoredDuplicatesLayoutPreset(): DuplicatesLayoutPreset {
  const stored = globalThis.localStorage?.getItem(
    STORAGE_KEYS.duplicatesLayoutPreset,
  );
  return VALID_DUPLICATES_LAYOUT_PRESETS.includes(
    stored as DuplicatesLayoutPreset,
  )
    ? (stored as DuplicatesLayoutPreset)
    : DEFAULT_DUPLICATES_LAYOUT_PRESET;
}

function readStoredDockLayouts(): DockLayoutStore {
  const raw = globalThis.localStorage?.getItem(STORAGE_KEYS.dockLayouts);
  if (!raw) {
    return {};
  }

  try {
    const parsed = JSON.parse(raw) as DockLayoutStore;
    if (!parsed || typeof parsed !== "object") {
      return {};
    }

    const cleaned: DockLayoutStore = {};

    for (const [layoutId, layout] of Object.entries(parsed)) {
      if (
        !layout ||
        typeof layout !== "object" ||
        !Array.isArray(layout.order) ||
        !layout.collapsed ||
        typeof layout.collapsed !== "object"
      ) {
        continue;
      }

      cleaned[layoutId] = {
        order: layout.order.filter((value): value is string => typeof value === "string"),
        collapsed: Object.fromEntries(
          Object.entries(layout.collapsed).filter(
            (entry): entry is [string, boolean] =>
              typeof entry[0] === "string" && typeof entry[1] === "boolean",
          ),
        ),
      };
    }

    return cleaned;
  } catch {
    return {};
  }
}

function normalizeDockSectionLayout(
  layout: DockSectionLayout | undefined,
  sectionIds: string[],
  defaults: Record<string, boolean> = {},
): DockSectionLayout {
  const order = [
    ...(layout?.order.filter((id) => sectionIds.includes(id)) ?? []),
    ...sectionIds.filter((id) => !(layout?.order ?? []).includes(id)),
  ];

  const collapsed = Object.fromEntries(
    sectionIds.map((id) => [id, layout?.collapsed[id] ?? defaults[id] ?? false]),
  );

  return {
    order,
    collapsed,
  };
}

export function UiPreferencesProvider({
  children,
}: {
  children: ReactNode;
}) {
  const [theme, setTheme] = useState<UiTheme>(readStoredTheme);
  const [density, setDensity] = useState<UiDensity>(readStoredDensity);
  const [sidebarWidth, setSidebarWidth] = useState(() =>
    readStoredSize(STORAGE_KEYS.sidebarWidth, DEFAULT_SIDEBAR_WIDTH, 84, 180),
  );
  const [homePrimaryWidth, setHomePrimaryWidth] = useState(() =>
    readStoredSize(
      STORAGE_KEYS.homePrimaryWidth,
      DEFAULT_HOME_PRIMARY_WIDTH,
      280,
      520,
    ),
  );
  const [homeSecondaryWidth, setHomeSecondaryWidth] = useState(() =>
    readStoredSize(
      STORAGE_KEYS.homeSecondaryWidth,
      DEFAULT_HOME_SECONDARY_WIDTH,
      240,
      460,
    ),
  );
  const [inspectorWidth, setInspectorWidth] = useState(() =>
    readStoredSize(STORAGE_KEYS.inspectorWidth, DEFAULT_INSPECTOR_WIDTH, 300, 720),
  );
  const [guideWidth, setGuideWidth] = useState(() =>
    readStoredSize(STORAGE_KEYS.guideWidth, DEFAULT_GUIDE_WIDTH, 460, 1040),
  );
  const [scannerWidth, setScannerWidth] = useState(() =>
    readStoredSize(STORAGE_KEYS.scannerWidth, DEFAULT_SCANNER_WIDTH, 420, 860),
  );
  const [auditGroupWidth, setAuditGroupWidth] = useState(() =>
    readStoredSize(
      STORAGE_KEYS.auditGroupWidth,
      DEFAULT_AUDIT_GROUP_WIDTH,
      240,
      520,
    ),
  );
  const [auditStageHeight, setAuditStageHeight] = useState(() =>
    readStoredSize(
      STORAGE_KEYS.auditStageHeight,
      DEFAULT_AUDIT_STAGE_HEIGHT,
      220,
      620,
    ),
  );
  const [organizeRailWidth, setOrganizeRailWidth] = useState(() =>
    readStoredSize(
      STORAGE_KEYS.organizeRailWidth,
      DEFAULT_ORGANIZE_RAIL_WIDTH,
      240,
      480,
    ),
  );
  const [organizePreviewHeight, setOrganizePreviewHeight] = useState(() =>
    readStoredSize(
      STORAGE_KEYS.organizePreviewHeight,
      DEFAULT_ORGANIZE_PREVIEW_HEIGHT,
      280,
      720,
    ),
  );
  const [downloadsDetailWidth, setDownloadsDetailWidthState] = useState(() =>
    readStoredSize(
      STORAGE_KEYS.downloadsDetailWidth,
      DEFAULT_DOWNLOADS_DETAIL_WIDTH,
      320,
      780,
    ),
  );
  const [downloadsQueueHeight, setDownloadsQueueHeightState] = useState(() =>
    readStoredSize(
      STORAGE_KEYS.downloadsQueueHeight,
      DEFAULT_DOWNLOADS_QUEUE_HEIGHT,
      220,
      720,
    ),
  );
  const [libraryDetailWidth, setLibraryDetailWidthState] = useState(() =>
    readStoredSize(
      STORAGE_KEYS.libraryDetailWidth,
      DEFAULT_LIBRARY_DETAIL_WIDTH,
      300,
      760,
    ),
  );
  const [libraryTableHeight, setLibraryTableHeightState] = useState(() =>
    readStoredSize(
      STORAGE_KEYS.libraryTableHeight,
      DEFAULT_LIBRARY_TABLE_HEIGHT,
      260,
      860,
    ),
  );
  const [reviewDetailWidth, setReviewDetailWidthState] = useState(() =>
    readStoredSize(
      STORAGE_KEYS.reviewDetailWidth,
      DEFAULT_REVIEW_DETAIL_WIDTH,
      300,
      760,
    ),
  );
  const [reviewQueueHeight, setReviewQueueHeightState] = useState(() =>
    readStoredSize(
      STORAGE_KEYS.reviewQueueHeight,
      DEFAULT_REVIEW_QUEUE_HEIGHT,
      260,
      860,
    ),
  );
  const [duplicatesDetailWidth, setDuplicatesDetailWidthState] = useState(() =>
    readStoredSize(
      STORAGE_KEYS.duplicatesDetailWidth,
      DEFAULT_DUPLICATES_DETAIL_WIDTH,
      320,
      780,
    ),
  );
  const [duplicatesQueueHeight, setDuplicatesQueueHeightState] = useState(() =>
    readStoredSize(
      STORAGE_KEYS.duplicatesQueueHeight,
      DEFAULT_DUPLICATES_QUEUE_HEIGHT,
      260,
      860,
    ),
  );
  const [libraryFiltersCollapsed, setLibraryFiltersCollapsedState] = useState(() =>
    readStoredBoolean(STORAGE_KEYS.libraryFiltersCollapsed, false),
  );
  const [duplicatesFiltersCollapsed, setDuplicatesFiltersCollapsedState] =
    useState(() =>
      readStoredBoolean(STORAGE_KEYS.duplicatesFiltersCollapsed, false),
    );
  const [libraryLayoutPreset, setLibraryLayoutPresetState] =
    useState<LibraryLayoutPreset>(readStoredLibraryLayoutPreset);
  const [reviewLayoutPreset, setReviewLayoutPresetState] =
    useState<ReviewLayoutPreset>(readStoredReviewLayoutPreset);
  const [duplicatesLayoutPreset, setDuplicatesLayoutPresetState] =
    useState<DuplicatesLayoutPreset>(readStoredDuplicatesLayoutPreset);
  const [dockLayouts, setDockLayouts] = useState<DockLayoutStore>(
    readStoredDockLayouts,
  );

  function applyLibraryLayoutPreset(preset: LibraryLayoutPreset) {
    setLibraryLayoutPresetState(preset);

    if (preset === "browse") {
      setLibraryDetailWidthState(392);
      setLibraryTableHeightState(510);
      setLibraryFiltersCollapsedState(false);
      return;
    }

    if (preset === "inspect") {
      setLibraryDetailWidthState(520);
      setLibraryTableHeightState(430);
      setLibraryFiltersCollapsedState(false);
      return;
    }

    if (preset === "catalog") {
      setLibraryDetailWidthState(320);
      setLibraryTableHeightState(640);
      setLibraryFiltersCollapsedState(true);
    }
  }

  function applyReviewLayoutPreset(preset: ReviewLayoutPreset) {
    setReviewLayoutPresetState(preset);

    if (preset === "queue") {
      setReviewDetailWidthState(340);
      setReviewQueueHeightState(620);
      return;
    }

    if (preset === "balanced") {
      setReviewDetailWidthState(404);
      setReviewQueueHeightState(520);
      return;
    }

    if (preset === "focus") {
      setReviewDetailWidthState(540);
      setReviewQueueHeightState(400);
    }
  }

  function applyDuplicatesLayoutPreset(preset: DuplicatesLayoutPreset) {
    setDuplicatesLayoutPresetState(preset);

    if (preset === "sweep") {
      setDuplicatesDetailWidthState(360);
      setDuplicatesQueueHeightState(620);
      setDuplicatesFiltersCollapsedState(false);
      return;
    }

    if (preset === "balanced") {
      setDuplicatesDetailWidthState(430);
      setDuplicatesQueueHeightState(520);
      setDuplicatesFiltersCollapsedState(false);
      return;
    }

    if (preset === "compare") {
      setDuplicatesDetailWidthState(560);
      setDuplicatesQueueHeightState(400);
      setDuplicatesFiltersCollapsedState(true);
    }
  }

  function setLibraryDetailWidth(width: number) {
    setLibraryDetailWidthState(clamp(width, 300, 760));
    setLibraryLayoutPresetState("custom");
  }

  function setDownloadsDetailWidth(width: number) {
    setDownloadsDetailWidthState(clamp(width, 320, 780));
  }

  function setDownloadsQueueHeight(height: number) {
    setDownloadsQueueHeightState(clamp(height, 220, 720));
  }

  function setLibraryTableHeight(height: number) {
    setLibraryTableHeightState(clamp(height, 260, 860));
    setLibraryLayoutPresetState("custom");
  }

  function setReviewDetailWidth(width: number) {
    setReviewDetailWidthState(clamp(width, 300, 760));
    setReviewLayoutPresetState("custom");
  }

  function setReviewQueueHeight(height: number) {
    setReviewQueueHeightState(clamp(height, 260, 860));
    setReviewLayoutPresetState("custom");
  }

  function setDuplicatesDetailWidth(width: number) {
    setDuplicatesDetailWidthState(clamp(width, 320, 780));
    setDuplicatesLayoutPresetState("custom");
  }

  function setDuplicatesQueueHeight(height: number) {
    setDuplicatesQueueHeightState(clamp(height, 260, 860));
    setDuplicatesLayoutPresetState("custom");
  }

  function setLibraryFiltersCollapsed(collapsed: boolean) {
    setLibraryFiltersCollapsedState(collapsed);
    setLibraryLayoutPresetState("custom");
  }

  function setDuplicatesFiltersCollapsed(collapsed: boolean) {
    setDuplicatesFiltersCollapsedState(collapsed);
    setDuplicatesLayoutPresetState("custom");
  }

  function getDockSectionLayout(
    layoutId: string,
    sectionIds: string[],
    defaults: Record<string, boolean> = {},
  ) {
    return normalizeDockSectionLayout(dockLayouts[layoutId], sectionIds, defaults);
  }

  function setDockSectionOrder(layoutId: string, order: string[]) {
    setDockLayouts((current) => {
      const next = normalizeDockSectionLayout(current[layoutId], order);
      return {
        ...current,
        [layoutId]: {
          order: next.order,
          collapsed: {
            ...current[layoutId]?.collapsed,
            ...next.collapsed,
          },
        },
      };
    });
  }

  function setDockSectionCollapsed(
    layoutId: string,
    sectionId: string,
    collapsed: boolean,
  ) {
    setDockLayouts((current) => ({
      ...current,
      [layoutId]: {
        order: current[layoutId]?.order ?? [sectionId],
        collapsed: {
          ...current[layoutId]?.collapsed,
          [sectionId]: collapsed,
        },
      },
    }));
  }

  function resetDockSectionLayout(layoutId: string) {
    setDockLayouts((current) => {
      if (!(layoutId in current)) {
        return current;
      }

      const next = { ...current };
      delete next[layoutId];
      return next;
    });
  }

  useEffect(() => {
    const root = document.documentElement;

    root.dataset.theme = theme;
    root.dataset.density = density;
    root.style.setProperty("--sidebar-width", `${sidebarWidth}px`);
    root.style.setProperty("--home-primary-width", `${homePrimaryWidth}px`);
    root.style.setProperty("--home-secondary-width", `${homeSecondaryWidth}px`);
    root.style.setProperty("--detail-panel-width", `${inspectorWidth}px`);
    root.style.setProperty("--guide-width", `${guideWidth}px`);
    root.style.setProperty("--scanner-width", `${scannerWidth}px`);
    root.style.setProperty("--audit-left-width", `${auditGroupWidth}px`);
    root.style.setProperty("--audit-stage-top-height", `${auditStageHeight}px`);
    root.style.setProperty("--organize-rail-width", `${organizeRailWidth}px`);
    root.style.setProperty(
      "--organize-preview-height",
      `${organizePreviewHeight}px`,
    );
    root.style.setProperty(
      "--downloads-detail-width",
      `${downloadsDetailWidth}px`,
    );
    root.style.setProperty(
      "--downloads-queue-height",
      `${downloadsQueueHeight}px`,
    );
    root.style.setProperty("--library-detail-width", `${libraryDetailWidth}px`);
    root.style.setProperty("--library-table-height", `${libraryTableHeight}px`);
    root.style.setProperty("--review-detail-width", `${reviewDetailWidth}px`);
    root.style.setProperty("--review-queue-height", `${reviewQueueHeight}px`);
    root.style.setProperty(
      "--duplicates-detail-width",
      `${duplicatesDetailWidth}px`,
    );
    root.style.setProperty(
      "--duplicates-queue-height",
      `${duplicatesQueueHeight}px`,
    );

    globalThis.localStorage?.setItem(STORAGE_KEYS.theme, theme);
    globalThis.localStorage?.setItem(STORAGE_KEYS.density, density);
    globalThis.localStorage?.setItem(
      STORAGE_KEYS.sidebarWidth,
      String(sidebarWidth),
    );
    globalThis.localStorage?.setItem(
      STORAGE_KEYS.homePrimaryWidth,
      String(homePrimaryWidth),
    );
    globalThis.localStorage?.setItem(
      STORAGE_KEYS.homeSecondaryWidth,
      String(homeSecondaryWidth),
    );
    globalThis.localStorage?.setItem(
      STORAGE_KEYS.inspectorWidth,
      String(inspectorWidth),
    );
    globalThis.localStorage?.setItem(STORAGE_KEYS.guideWidth, String(guideWidth));
    globalThis.localStorage?.setItem(
      STORAGE_KEYS.scannerWidth,
      String(scannerWidth),
    );
    globalThis.localStorage?.setItem(
      STORAGE_KEYS.auditGroupWidth,
      String(auditGroupWidth),
    );
    globalThis.localStorage?.setItem(
      STORAGE_KEYS.auditStageHeight,
      String(auditStageHeight),
    );
    globalThis.localStorage?.setItem(
      STORAGE_KEYS.organizeRailWidth,
      String(organizeRailWidth),
    );
    globalThis.localStorage?.setItem(
      STORAGE_KEYS.organizePreviewHeight,
      String(organizePreviewHeight),
    );
    globalThis.localStorage?.setItem(
      STORAGE_KEYS.downloadsDetailWidth,
      String(downloadsDetailWidth),
    );
    globalThis.localStorage?.setItem(
      STORAGE_KEYS.downloadsQueueHeight,
      String(downloadsQueueHeight),
    );
    globalThis.localStorage?.setItem(
      STORAGE_KEYS.libraryDetailWidth,
      String(libraryDetailWidth),
    );
    globalThis.localStorage?.setItem(
      STORAGE_KEYS.libraryTableHeight,
      String(libraryTableHeight),
    );
    globalThis.localStorage?.setItem(
      STORAGE_KEYS.reviewDetailWidth,
      String(reviewDetailWidth),
    );
    globalThis.localStorage?.setItem(
      STORAGE_KEYS.reviewQueueHeight,
      String(reviewQueueHeight),
    );
    globalThis.localStorage?.setItem(
      STORAGE_KEYS.duplicatesDetailWidth,
      String(duplicatesDetailWidth),
    );
    globalThis.localStorage?.setItem(
      STORAGE_KEYS.duplicatesQueueHeight,
      String(duplicatesQueueHeight),
    );
    globalThis.localStorage?.setItem(
      STORAGE_KEYS.libraryFiltersCollapsed,
      String(libraryFiltersCollapsed),
    );
    globalThis.localStorage?.setItem(
      STORAGE_KEYS.duplicatesFiltersCollapsed,
      String(duplicatesFiltersCollapsed),
    );
    globalThis.localStorage?.setItem(
      STORAGE_KEYS.libraryLayoutPreset,
      libraryLayoutPreset,
    );
    globalThis.localStorage?.setItem(
      STORAGE_KEYS.reviewLayoutPreset,
      reviewLayoutPreset,
    );
    globalThis.localStorage?.setItem(
      STORAGE_KEYS.duplicatesLayoutPreset,
      duplicatesLayoutPreset,
    );
    globalThis.localStorage?.setItem(
      STORAGE_KEYS.dockLayouts,
      JSON.stringify(dockLayouts),
    );
  }, [
    theme,
    density,
    sidebarWidth,
    homePrimaryWidth,
    homeSecondaryWidth,
    inspectorWidth,
    guideWidth,
    scannerWidth,
    auditGroupWidth,
    auditStageHeight,
    organizeRailWidth,
    organizePreviewHeight,
    downloadsDetailWidth,
    downloadsQueueHeight,
    libraryDetailWidth,
    libraryTableHeight,
    reviewDetailWidth,
    reviewQueueHeight,
    duplicatesDetailWidth,
    duplicatesQueueHeight,
    libraryFiltersCollapsed,
    duplicatesFiltersCollapsed,
    libraryLayoutPreset,
    reviewLayoutPreset,
    duplicatesLayoutPreset,
    dockLayouts,
  ]);

  return (
    <UiPreferencesContext.Provider
      value={{
        theme,
        density,
        sidebarWidth,
        homePrimaryWidth,
        homeSecondaryWidth,
        inspectorWidth,
        guideWidth,
        scannerWidth,
        auditGroupWidth,
        auditStageHeight,
        organizeRailWidth,
        organizePreviewHeight,
        downloadsDetailWidth,
        downloadsQueueHeight,
        libraryDetailWidth,
        libraryTableHeight,
        reviewDetailWidth,
        reviewQueueHeight,
        duplicatesDetailWidth,
        duplicatesQueueHeight,
        libraryFiltersCollapsed,
        duplicatesFiltersCollapsed,
        libraryLayoutPreset,
        reviewLayoutPreset,
        duplicatesLayoutPreset,
        getDockSectionLayout,
        setDockSectionOrder,
        setDockSectionCollapsed,
        resetDockSectionLayout,
        setTheme,
        setDensity,
        setSidebarWidth,
        setHomePrimaryWidth,
        setHomeSecondaryWidth,
        setInspectorWidth,
        setGuideWidth,
        setScannerWidth,
        setAuditGroupWidth,
        setAuditStageHeight,
        setOrganizeRailWidth,
        setOrganizePreviewHeight,
        setDownloadsDetailWidth,
        setDownloadsQueueHeight,
        setLibraryDetailWidth,
        setLibraryTableHeight,
        setReviewDetailWidth,
        setReviewQueueHeight,
        setDuplicatesDetailWidth,
        setDuplicatesQueueHeight,
        setLibraryFiltersCollapsed,
        setDuplicatesFiltersCollapsed,
        applyLibraryLayoutPreset,
        applyReviewLayoutPreset,
        applyDuplicatesLayoutPreset,
        resetPanelSizes: () => {
          setSidebarWidth(DEFAULT_SIDEBAR_WIDTH);
          setHomePrimaryWidth(DEFAULT_HOME_PRIMARY_WIDTH);
          setHomeSecondaryWidth(DEFAULT_HOME_SECONDARY_WIDTH);
          setInspectorWidth(DEFAULT_INSPECTOR_WIDTH);
          setGuideWidth(DEFAULT_GUIDE_WIDTH);
          setScannerWidth(DEFAULT_SCANNER_WIDTH);
          setAuditGroupWidth(DEFAULT_AUDIT_GROUP_WIDTH);
          setAuditStageHeight(DEFAULT_AUDIT_STAGE_HEIGHT);
          setOrganizeRailWidth(DEFAULT_ORGANIZE_RAIL_WIDTH);
          setOrganizePreviewHeight(DEFAULT_ORGANIZE_PREVIEW_HEIGHT);
          setDownloadsDetailWidthState(DEFAULT_DOWNLOADS_DETAIL_WIDTH);
          setDownloadsQueueHeightState(DEFAULT_DOWNLOADS_QUEUE_HEIGHT);
          setLibraryDetailWidthState(DEFAULT_LIBRARY_DETAIL_WIDTH);
          setLibraryTableHeightState(DEFAULT_LIBRARY_TABLE_HEIGHT);
          setReviewDetailWidthState(DEFAULT_REVIEW_DETAIL_WIDTH);
          setReviewQueueHeightState(DEFAULT_REVIEW_QUEUE_HEIGHT);
          setDuplicatesDetailWidthState(DEFAULT_DUPLICATES_DETAIL_WIDTH);
          setDuplicatesQueueHeightState(DEFAULT_DUPLICATES_QUEUE_HEIGHT);
          setLibraryFiltersCollapsedState(false);
          setDuplicatesFiltersCollapsedState(false);
          setLibraryLayoutPresetState(DEFAULT_LIBRARY_LAYOUT_PRESET);
          setReviewLayoutPresetState(DEFAULT_REVIEW_LAYOUT_PRESET);
          setDuplicatesLayoutPresetState(DEFAULT_DUPLICATES_LAYOUT_PRESET);
          setDockLayouts({});
        },
      }}
    >
      {children}
    </UiPreferencesContext.Provider>
  );
}

export function useUiPreferences() {
  const context = useContext(UiPreferencesContext);
  if (!context) {
    throw new Error("useUiPreferences must be used within UiPreferencesProvider");
  }

  return context;
}
