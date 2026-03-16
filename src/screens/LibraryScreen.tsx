import { useDeferredValue, useEffect, useRef, useState } from "react";
import { m } from "motion/react";
import { CornerUpLeft, LibraryBig } from "lucide-react";
import { DockSectionStack } from "../components/DockSectionStack";
import { LayoutPresetBar } from "../components/LayoutPresetBar";
import { ResizableEdgeHandle } from "../components/ResizableEdgeHandle";
import { ResizableDetailPanel } from "../components/ResizableDetailPanel";
import { useUiPreferences } from "../components/UiPreferencesContext";
import { api } from "../lib/api";
import { rowHover, rowPress, stagedListItem } from "../lib/motion";
import {
  friendlyTypeLabel,
  screenHelperLine,
  screenLabel,
  unknownCreatorLabel,
} from "../lib/uiLanguage";
import type {
  AppBehaviorSettings,
  CategoryOverrideInfo,
  CreatorLearningInfo,
  FileDetail,
  HomeOverview,
  InstalledVersionSummary,
  LibraryLayoutPreset,
  LibraryFacets,
  LibraryFileRow,
  LibraryWatchFocusRequest,
  LibraryListResponse,
  LibraryWatchListResponse,
  LibraryWatchReviewReason,
  LibraryWatchReviewResponse,
  LibraryWatchSetupItem,
  LibraryWatchSetupResponse,
  SaveLibraryWatchSourceEntry,
  Screen,
  UserView,
  VersionConfidence,
  WatchListFilter,
  WatchResult,
  WatchSourceKind,
  WatchSourceOrigin,
} from "../lib/types";

interface LibraryScreenProps {
  refreshVersion: number;
  onNavigate: (screen: Screen) => void;
  watchFocusRequest?: LibraryWatchFocusRequest | null;
  onConsumeWatchFocus?: () => void;
  userView: UserView;
}

interface PendingWatchIntent {
  fileId: number;
  mode: "setup" | "review";
  sourceKind?: WatchSourceKind;
  sourceLabel?: string;
}

const PAGE_SIZE = 100;
const LIBRARY_LAYOUT_PRESETS: Array<{
  id: LibraryLayoutPreset;
  label: string;
  hint: string;
}> = [
  {
    id: "browse",
    label: "Browse",
    hint: "Balanced list and inspector with filters open.",
  },
  {
    id: "inspect",
    label: "Inspect",
    hint: "Gives the right-hand inspector more room for details.",
  },
  {
    id: "catalog",
    label: "Catalog",
    hint: "Puts more space on the table and hides filters until needed.",
  },
];

const WATCH_LIST_FILTERS: Array<{
  id: WatchListFilter;
  label: string;
  beginnerLabel: string;
}> = [
  { id: "attention", label: "Needs attention", beginnerLabel: "Needs attention" },
  { id: "exact_updates", label: "Confirmed updates", beginnerLabel: "Confirmed updates" },
  { id: "possible_updates", label: "Possible updates", beginnerLabel: "Possible updates" },
  { id: "unclear", label: "Unclear", beginnerLabel: "Unclear" },
  { id: "all", label: "All tracked", beginnerLabel: "All tracked" },
];

export function LibraryScreen({
  refreshVersion,
  onNavigate,
  watchFocusRequest,
  onConsumeWatchFocus,
  userView,
}: LibraryScreenProps) {
  const {
    libraryDetailWidth,
    libraryTableHeight,
    setLibraryDetailWidth,
    setLibraryTableHeight,
    libraryFiltersCollapsed,
    setLibraryFiltersCollapsed,
    libraryLayoutPreset,
    applyLibraryLayoutPreset,
  } = useUiPreferences();
  const [facets, setFacets] = useState<LibraryFacets | null>(null);
  const [rows, setRows] = useState<LibraryListResponse | null>(null);
  const [selected, setSelected] = useState<FileDetail | null>(null);
  const [search, setSearch] = useState("");
  const [kind, setKind] = useState("");
  const [subtype, setSubtype] = useState("");
  const [creator, setCreator] = useState("");
  const [source, setSource] = useState("");
  const [minConfidence, setMinConfidence] = useState("");
  const [page, setPage] = useState(0);
  const [creatorDraft, setCreatorDraft] = useState("");
  const [aliasDraft, setAliasDraft] = useState("");
  const [lockPreference, setLockPreference] = useState(false);
  const [preferredPathDraft, setPreferredPathDraft] = useState("");
  const [creatorMessage, setCreatorMessage] = useState<string | null>(null);
  const [savingCreator, setSavingCreator] = useState(false);
  const [categoryKindDraft, setCategoryKindDraft] = useState("");
  const [categorySubtypeDraft, setCategorySubtypeDraft] = useState("");
  const [categoryMessage, setCategoryMessage] = useState<string | null>(null);
  const [savingCategory, setSavingCategory] = useState(false);
  const [watchEditing, setWatchEditing] = useState(false);
  const [watchSourceKind, setWatchSourceKind] = useState<WatchSourceKind>("exact_page");
  const [watchSourceLabel, setWatchSourceLabel] = useState("");
  const [watchSourceUrl, setWatchSourceUrl] = useState("");
  const [savingWatch, setSavingWatch] = useState(false);
  const [refreshingWatch, setRefreshingWatch] = useState(false);
  const [refreshingAllWatched, setRefreshingAllWatched] = useState(false);
  const [watchMessage, setWatchMessage] = useState<string | null>(null);
  const [watchOverview, setWatchOverview] = useState<HomeOverview | null>(null);
  const [watchList, setWatchList] = useState<LibraryWatchListResponse | null>(null);
  const [watchListFilter, setWatchListFilter] = useState<WatchListFilter>("attention");
  const [loadingWatchList, setLoadingWatchList] = useState(false);
  const [watchReviewList, setWatchReviewList] = useState<LibraryWatchReviewResponse | null>(
    null,
  );
  const [loadingWatchReviewList, setLoadingWatchReviewList] = useState(false);
  const [watchSetupList, setWatchSetupList] = useState<LibraryWatchSetupResponse | null>(null);
  const [loadingWatchSetupList, setLoadingWatchSetupList] = useState(false);
  const [bulkExactWatchUrls, setBulkExactWatchUrls] = useState<Record<number, string>>({});
  const [bulkExactWatchErrors, setBulkExactWatchErrors] = useState<Record<number, string>>(
    {},
  );
  const [bulkExactWatchMessage, setBulkExactWatchMessage] = useState<string | null>(null);
  const [savingBulkExactWatch, setSavingBulkExactWatch] = useState(false);
  const [pendingWatchIntent, setPendingWatchIntent] = useState<PendingWatchIntent | null>(null);
  const [appBehavior, setAppBehavior] = useState<AppBehaviorSettings | null>(null);
  const [watchCenterMessage, setWatchCenterMessage] = useState<string | null>(null);
  const [queuedWatchCenterAction, setQueuedWatchCenterAction] = useState<"review" | null>(
    null,
  );
  const [focusedWatchSection, setFocusedWatchSection] = useState<"tracked" | "setup" | null>(
    null,
  );
  const deferredSearch = useDeferredValue(search);
  const trackedWatchSectionRef = useRef<HTMLDivElement | null>(null);
  const setupWatchSectionRef = useRef<HTMLDivElement | null>(null);
  const watchFocusTimerRef = useRef<number | null>(null);

  useEffect(() => {
    void api.getLibraryFacets().then(setFacets);
  }, [refreshVersion]);

  useEffect(() => {
    void loadWatchCenter();
  }, [refreshVersion]);

  useEffect(() => {
    void loadWatchList();
  }, [refreshVersion, watchListFilter]);

  useEffect(() => {
    void loadWatchReviewList();
  }, [refreshVersion]);

  useEffect(() => {
    void loadWatchSetupList();
  }, [refreshVersion]);

  useEffect(() => {
    const exactItems = watchSetupList?.exactPageItems ?? [];
    setBulkExactWatchUrls((current) => {
      const next: Record<number, string> = {};
      for (const item of exactItems) {
        next[item.fileId] = current[item.fileId] ?? "";
      }
      return next;
    });
    setBulkExactWatchErrors((current) => {
      const next: Record<number, string> = {};
      for (const item of exactItems) {
        if (current[item.fileId]) {
          next[item.fileId] = current[item.fileId];
        }
      }
      return next;
    });
    if (!exactItems.length) {
      setBulkExactWatchMessage(null);
    }
  }, [watchSetupList]);

  useEffect(() => {
    void loadRows();
  }, [
    refreshVersion,
    deferredSearch,
    kind,
    subtype,
    creator,
    source,
    minConfidence,
    page,
  ]);

  useEffect(() => {
    if (!selected) {
      setCreatorDraft("");
      setAliasDraft("");
      setLockPreference(false);
      setPreferredPathDraft("");
      setCreatorMessage(null);
      setCategoryKindDraft("");
      setCategorySubtypeDraft("");
      setCategoryMessage(null);
      setWatchEditing(false);
      setWatchSourceKind("exact_page");
      setWatchSourceLabel("");
      setWatchSourceUrl("");
      setRefreshingWatch(false);
      setWatchMessage(null);
      setWatchCenterMessage(null);
      setPendingWatchIntent(null);
      return;
    }

    setCreatorDraft(selected.creator ?? selected.insights.creatorHints[0] ?? "");
    setAliasDraft("");
    setLockPreference(selected.creatorLearning.lockedByUser);
    setPreferredPathDraft(selected.creatorLearning.preferredPath ?? "");
    setCreatorMessage(null);
    setCategoryKindDraft(selected.categoryOverride.kind ?? selected.kind);
    setCategorySubtypeDraft(selected.categoryOverride.subtype ?? selected.subtype ?? "");
    setCategoryMessage(null);

    const currentWatch = selected.watchResult;
    restoreWatchFields(selected);
    setRefreshingWatch(false);

    if (pendingWatchIntent?.fileId === selected.id) {
      if (pendingWatchIntent.mode === "setup") {
        const sourceKind = pendingWatchIntent.sourceKind ?? "exact_page";
        setWatchSourceKind(sourceKind);
        setWatchSourceLabel(pendingWatchIntent.sourceLabel ?? "");
        setWatchSourceUrl("");
        setWatchEditing(true);
        setWatchMessage(
          sourceKind === "creator_page"
            ? "Add the official creator page URL to finish setup."
            : "Add the official mod page URL to finish setup.",
        );
      } else {
        setWatchSourceKind(currentWatch?.sourceKind ?? pendingWatchIntent.sourceKind ?? "exact_page");
        setWatchSourceLabel(currentWatch?.sourceLabel ?? pendingWatchIntent.sourceLabel ?? "");
        setWatchSourceUrl(currentWatch?.sourceUrl ?? "");
        setWatchEditing(true);
        setWatchMessage(
          currentWatch?.sourceKind
            ? "Review or update this saved watch source."
            : "No watch source is saved yet. Add one here.",
        );
      }
    } else {
      setWatchEditing(false);
      setWatchMessage(null);
    }
  }, [selected, pendingWatchIntent]);

  useEffect(() => {
    return () => {
      if (watchFocusTimerRef.current !== null) {
        globalThis.clearTimeout(watchFocusTimerRef.current);
      }
    };
  }, []);

  function focusWatchSection(section: "tracked" | "setup") {
    setFocusedWatchSection(section);

    if (watchFocusTimerRef.current !== null) {
      globalThis.clearTimeout(watchFocusTimerRef.current);
    }

    globalThis.setTimeout(() => {
      const sectionElement =
        section === "setup"
          ? setupWatchSectionRef.current
          : trackedWatchSectionRef.current;
      sectionElement?.scrollIntoView({ behavior: "smooth", block: "nearest" });
    }, 80);

    watchFocusTimerRef.current = globalThis.setTimeout(() => {
      setFocusedWatchSection(null);
      watchFocusTimerRef.current = null;
    }, 2200);
  }

  useEffect(() => {
    if (!watchFocusRequest) {
      return;
    }

    const focusTarget =
      watchFocusRequest.target === "setup" ? "setup" : "tracked";

    setQueuedWatchCenterAction(null);
    setPendingWatchIntent(null);
    setWatchEditing(false);
    setWatchMessage(null);
    focusWatchSection(focusTarget);

    if (watchFocusRequest.target === "tracked_attention") {
      setWatchListFilter("attention");
      setWatchCenterMessage("Showing watched items that still need attention.");
    } else if (watchFocusRequest.target === "tracked_exact_updates") {
      setWatchListFilter("exact_updates");
      setWatchCenterMessage("Showing watched items with confirmed updates.");
    } else if (watchFocusRequest.target === "tracked_possible_updates") {
      setWatchListFilter("possible_updates");
      setWatchCenterMessage("Showing watched items with possible updates.");
    } else if (watchFocusRequest.target === "tracked_unclear") {
      setWatchListFilter("unclear");
      setWatchCenterMessage("Showing watched items that still look unclear.");
    } else if (watchFocusRequest.target === "tracked_all") {
      setWatchListFilter("all");
      setWatchCenterMessage("Showing every tracked watch item.");
    } else {
      setWatchCenterMessage("Showing the strongest watch setup suggestions.");
    }

    onConsumeWatchFocus?.();
  }, [watchFocusRequest, onConsumeWatchFocus]);

  useEffect(() => {
    if (queuedWatchCenterAction !== "review" || loadingWatchReviewList) {
      return;
    }

    const nextReview = watchReviewList?.items[0] ?? null;

    if (nextReview) {
      setQueuedWatchCenterAction(null);
      void beginWatchReview(nextReview.fileId);
      return;
    }

    if (watchReviewList) {
      setQueuedWatchCenterAction(null);
      setWatchCenterMessage("Nothing needs watch review right now.");
    }
  }, [
    queuedWatchCenterAction,
    loadingWatchReviewList,
    watchReviewList,
  ]);

  async function loadWatchCenter() {
    try {
      const [overview, behavior] = await Promise.all([
        api.getHomeOverview(),
        api.getAppBehaviorSettings(),
      ]);

      setWatchOverview(overview);
      setAppBehavior(behavior);
    } catch {
      setWatchOverview(null);
      setAppBehavior(null);
    }
  }

  async function loadWatchList() {
    setLoadingWatchList(true);

    try {
      const next = await api.listLibraryWatchItems(watchListFilter, 12);
      setWatchList(next);
    } catch {
      setWatchList({
        filter: watchListFilter,
        total: 0,
        items: [],
      });
    } finally {
      setLoadingWatchList(false);
    }
  }

  async function loadWatchReviewList() {
    setLoadingWatchReviewList(true);

    try {
      const next = await api.listLibraryWatchReviewItems(8);
      setWatchReviewList(next);
    } catch {
      setWatchReviewList({
        total: 0,
        providerNeededCount: 0,
        referenceOnlyCount: 0,
        unknownResultCount: 0,
        items: [],
      });
    } finally {
      setLoadingWatchReviewList(false);
    }
  }

  async function loadWatchSetupList() {
    setLoadingWatchSetupList(true);

    try {
      const next = await api.listLibraryWatchSetupItems(6);
      setWatchSetupList(next);
    } catch {
      setWatchSetupList({
        total: 0,
        truncated: false,
        exactPageTotal: 0,
        exactPageTruncated: false,
        exactPageItems: [],
        items: [],
      });
    } finally {
      setLoadingWatchSetupList(false);
    }
  }

  async function loadRows(preferredSelectedId?: number) {
    const result = await api.listLibraryFiles({
      search: deferredSearch || undefined,
      kind: kind || undefined,
      subtype: subtype || undefined,
      creator: creator || undefined,
      source: source || undefined,
      minConfidence: minConfidence ? Number(minConfidence) : undefined,
      limit: PAGE_SIZE,
      offset: page * PAGE_SIZE,
    });

    setRows(result);

    const detailId = preferredSelectedId ?? selected?.id ?? result.items[0]?.id;
    if (detailId) {
      setSelected(await api.getFileDetail(detailId));
    } else {
      setSelected(null);
    }
  }

  async function openFile(row: LibraryFileRow, preserveWatchIntent = false) {
    setQueuedWatchCenterAction(null);
    if (!preserveWatchIntent) {
      setPendingWatchIntent(null);
      setWatchMessage(null);
    }
    setSelected(await api.getFileDetail(row.id));
  }

  async function openFileById(fileId: number, preserveWatchIntent = false) {
    setQueuedWatchCenterAction(null);
    if (!preserveWatchIntent) {
      setPendingWatchIntent(null);
      setWatchMessage(null);
    }
    const detail = await api.getFileDetail(fileId);
    setSelected(detail);
    return detail;
  }

  function restoreWatchFields(file: FileDetail) {
    const currentWatch = file.watchResult;
    if (currentWatch?.sourceKind && currentWatch.sourceUrl) {
      setWatchSourceKind(currentWatch.sourceKind);
      setWatchSourceLabel(currentWatch.sourceLabel ?? "");
      setWatchSourceUrl(currentWatch.sourceUrl);
      return;
    }

    setWatchSourceKind("exact_page");
    setWatchSourceLabel("");
    setWatchSourceUrl("");
  }

  async function openWatchIntent(
    intent: PendingWatchIntent,
    centerMessage: string | null = null,
  ) {
    setQueuedWatchCenterAction(null);
    const detail = await openFileById(intent.fileId, true);
    if (!detail) {
      setPendingWatchIntent(null);
      setWatchMessage("That Library item could not be opened.");
      return;
    }
    setWatchCenterMessage(centerMessage);
    setPendingWatchIntent(intent);
  }

  async function beginWatchSetup(item: LibraryWatchSetupItem) {
    await openWatchIntent({
      fileId: item.fileId,
      mode: "setup",
      sourceKind: item.suggestedSourceKind,
      sourceLabel:
        item.suggestedSourceKind === "creator_page"
          ? item.creator ?? item.subjectLabel
          : item.subjectLabel,
    });
  }

  async function beginWatchReview(fileId: number) {
    const item =
      watchReviewList?.items.find((entry) => entry.fileId === fileId) ??
      watchList?.items.find((entry) => entry.fileId === fileId);
    await openWatchIntent(
      {
        fileId,
        mode: "review",
        sourceKind: item?.watchResult.sourceKind ?? "exact_page",
        sourceLabel: item?.watchResult.sourceLabel ?? item?.subjectLabel,
      },
      null,
    );
  }

  function nextSetupSuggestion(currentFileId: number) {
    return watchSetupList?.items.find((item) => item.fileId !== currentFileId) ?? null;
  }

  function nextReviewItem(currentFileId: number) {
    return watchReviewList?.items.find((item) => item.fileId !== currentFileId) ?? null;
  }

  async function advanceWatchSetupFlow(currentFileId: number, reason: "saved" | "skipped") {
    const nextItem = nextSetupSuggestion(currentFileId);
    if (nextItem) {
      await openWatchIntent(
        {
          fileId: nextItem.fileId,
          mode: "setup",
          sourceKind: nextItem.suggestedSourceKind,
          sourceLabel:
            nextItem.suggestedSourceKind === "creator_page"
              ? nextItem.creator ?? nextItem.subjectLabel
              : nextItem.subjectLabel,
        },
        reason === "saved"
          ? "Watch source saved. Opening the next setup suggestion."
          : "Skipped for now. Opening the next setup suggestion.",
      );
      return;
    }

    setPendingWatchIntent(null);
    setWatchEditing(false);
    setWatchMessage(null);
    setWatchCenterMessage(
      reason === "saved"
        ? "Watch source saved. No more strong setup suggestions are waiting right now."
        : "Skipped for now. No more strong setup suggestions are waiting right now.",
    );
  }

  async function advanceWatchReviewFlow(
    currentFileId: number,
    reason: "saved" | "cleared" | "refreshed" | "skipped",
  ) {
    const nextItem = nextReviewItem(currentFileId);
    if (nextItem) {
      await openWatchIntent(
        {
          fileId: nextItem.fileId,
          mode: "review",
          sourceKind: nextItem.watchResult.sourceKind ?? "exact_page",
          sourceLabel: nextItem.watchResult.sourceLabel ?? nextItem.subjectLabel,
        },
        reason === "saved"
          ? "Watch source saved. Opening the next review item."
          : reason === "cleared"
            ? "Watch source cleared. Opening the next review item."
            : reason === "refreshed"
              ? "Watch result refreshed. Opening the next review item."
              : "Skipped for now. Opening the next review item.",
      );
      return;
    }

    setPendingWatchIntent(null);
    setWatchEditing(false);
    setWatchMessage(null);
    setWatchCenterMessage(
      reason === "saved"
        ? "Watch source saved. No more watched items need review right now."
        : reason === "cleared"
          ? "Watch source cleared. No more watched items need review right now."
          : reason === "refreshed"
            ? "Watch result refreshed. No more watched items need review right now."
            : "Skipped for now. No more watched items need review right now.",
    );
  }

  function closeWatchEditor() {
    if (selected) {
      restoreWatchFields(selected);
    }
    setQueuedWatchCenterAction(null);
    setWatchEditing(false);
    setWatchMessage(null);
    setPendingWatchIntent(null);
  }

  async function skipWatchSetup() {
    if (!selected || pendingWatchIntent?.mode !== "setup" || pendingWatchIntent.fileId !== selected.id) {
      return;
    }

    await advanceWatchSetupFlow(selected.id, "skipped");
  }

  async function skipWatchReview() {
    if (!selected || pendingWatchIntent?.mode !== "review" || pendingWatchIntent.fileId !== selected.id) {
      return;
    }

    await advanceWatchReviewFlow(selected.id, "skipped");
  }

  async function saveCreatorOverride() {
    const trimmedCreator = creatorDraft.trim();
    if (!selected || !trimmedCreator) {
      return;
    }

    setSavingCreator(true);
    setCreatorMessage(null);

    try {
      const updated = await api.saveCreatorLearning(
        selected.id,
        trimmedCreator,
        aliasDraft.trim() || undefined,
        lockPreference,
        preferredPathDraft.trim() || undefined,
      );

      if (!updated) {
        return;
      }

      setSelected(updated);
      setCreatorMessage("Saved for future scans.");
      await Promise.all([loadRows(updated.id), api.getLibraryFacets().then(setFacets)]);
    } finally {
      setSavingCreator(false);
    }
  }

  async function saveCategoryClassification() {
    const trimmedKind = categoryKindDraft.trim();
    if (!selected || !trimmedKind) {
      return;
    }

    setSavingCategory(true);
    setCategoryMessage(null);

    try {
      const updated = await api.saveCategoryOverride(
        selected.id,
        trimmedKind,
        categorySubtypeDraft.trim() || undefined,
      );

      if (!updated) {
        return;
      }

      setSelected(updated);
      setCategoryMessage("Type override saved.");
      await Promise.all([loadRows(updated.id), api.getLibraryFacets().then(setFacets)]);
    } finally {
      setSavingCategory(false);
    }
  }

  const creatorSuggestions = selected
    ? Array.from(
        new Set(
          [selected.creator, ...selected.insights.creatorHints].filter(
            (value): value is string => Boolean(value),
          ),
        ),
      )
    : [];
  const categoryKindOptions = facets?.taxonomyKinds?.length
    ? facets.taxonomyKinds
    : ["CAS", "BuildBuy", "Gameplay", "ScriptMods", "OverridesAndDefaults", "PosesAndAnimation", "PresetsAndSliders", "TrayHousehold", "TrayLot", "TrayRoom", "TrayItem", "Unknown"];
  const totalPages = rows ? Math.max(1, Math.ceil(rows.total / PAGE_SIZE)) : 1;

  const tableColumns =
    userView === "beginner"
      ? ["name", "creator", "kind", "confidence"]
      : userView === "power"
        ? ["name", "creator", "kind", "root", "depth", "confidence"]
        : ["name", "creator", "kind", "root", "confidence"];
  const isPowerView = userView === "power";
  const hasInspectionSignals = Boolean(
    selected &&
      (selected.insights.format ||
        selected.insights.creatorHints.length ||
        selected.insights.familyHints.length ||
        selected.insights.versionHints.length ||
        selected.insights.versionSignals.length ||
        selected.insights.embeddedNames.length ||
        selected.insights.scriptNamespaces.length ||
        selected.insights.resourceSummary.length),
  );
  const hasVersionWatchInfo = Boolean(
    selected?.installedVersionSummary || selected?.watchResult,
  );
  const selectedWatch = selected?.watchResult ?? null;
  const watchSourceOrigin = selectedWatch?.sourceOrigin ?? "none";
  const isBuiltInWatchSource = watchSourceOrigin === "built_in_special";
  const bulkExactSetupItems = watchSetupList?.exactPageItems ?? [];
  const bulkExactSetupItemIds = new Set(bulkExactSetupItems.map((item) => item.fileId));
  const visibleSetupItems =
    watchSetupList?.items.filter((item) => !bulkExactSetupItemIds.has(item.fileId)) ?? [];
  const firstSetupSuggestion = watchSetupList?.items[0] ?? null;
  const firstReviewItem = watchReviewList?.items[0] ?? null;
  const isSetupQueueActive =
    pendingWatchIntent?.mode === "setup" && pendingWatchIntent.fileId === selected?.id;
  const isReviewQueueActive =
    pendingWatchIntent?.mode === "review" && pendingWatchIntent.fileId === selected?.id;
  const playerFacingNames = selected ? collectPlayerFacingNames(selected) : [];
  const showSafetySection = Boolean(
    selected &&
      (isPowerView ||
        selected.bundleName ||
        selected.safetyNotes.length ||
        selected.parserWarnings.length),
  );

  async function saveWatchSource() {
    if (!selected) {
      return;
    }

    const trimmedUrl = watchSourceUrl.trim();
    if (!trimmedUrl) {
      setWatchMessage("Enter a watch URL first.");
      return;
    }

    setSavingWatch(true);
    setWatchMessage(null);

    try {
      const currentIntent = pendingWatchIntent;
      const nextSuggestion =
        currentIntent?.mode === "setup" && currentIntent.fileId === selected.id
          ? nextSetupSuggestion(selected.id)
          : null;
      const reviewResolved =
        currentIntent?.mode === "review" &&
        currentIntent.fileId === selected.id;
      const nextReview =
        reviewResolved ? nextReviewItem(selected.id) : null;
      const updated = await api.saveWatchSourceForFile(
        selected.id,
        watchSourceKind,
        watchSourceLabel.trim() || undefined,
        trimmedUrl,
      );

      if (!updated) {
        return;
      }

      setSelected(updated);
      await Promise.all([
        loadRows(updated.id),
        loadWatchCenter(),
        loadWatchList(),
        loadWatchReviewList(),
        loadWatchSetupList(),
      ]);

      if (currentIntent?.mode === "setup" && currentIntent.fileId === selected.id) {
        if (nextSuggestion) {
          await openWatchIntent(
            {
              fileId: nextSuggestion.fileId,
              mode: "setup",
              sourceKind: nextSuggestion.suggestedSourceKind,
              sourceLabel:
                nextSuggestion.suggestedSourceKind === "creator_page"
                  ? nextSuggestion.creator ?? nextSuggestion.subjectLabel
                  : nextSuggestion.subjectLabel,
            },
            "Watch source saved. Opening the next setup suggestion.",
          );
        } else {
          setPendingWatchIntent(null);
          setWatchEditing(false);
          setWatchMessage("Watch source saved.");
          setWatchCenterMessage(
            "Watch source saved. No more strong setup suggestions are waiting right now.",
          );
        }
      } else if (reviewResolved) {
        if (!updated.watchResult || !shouldShowWatchReviewAction(updated.watchResult)) {
          if (nextReview) {
            await openWatchIntent(
              {
                fileId: nextReview.fileId,
                mode: "review",
                sourceKind: nextReview.watchResult.sourceKind ?? "exact_page",
                sourceLabel:
                  nextReview.watchResult.sourceLabel ?? nextReview.subjectLabel,
              },
              "Watch source saved. Opening the next review item.",
            );
          } else {
            setPendingWatchIntent(null);
            setWatchEditing(false);
            setWatchMessage("Watch source saved.");
            setWatchCenterMessage(
              "Watch source saved. No more watched items need review right now.",
            );
          }
        } else {
          setWatchEditing(true);
          setWatchMessage(
            "Watch source saved. This link still needs review before SimSuite can rely on it.",
          );
        }
      } else {
        setPendingWatchIntent(null);
        setWatchEditing(false);
        setWatchMessage("Watch source saved.");
      }
    } catch (error) {
      setWatchMessage(watchActionError(error, "save the watch source"));
    } finally {
      setSavingWatch(false);
    }
  }

  async function clearWatchSource() {
    if (!selected) {
      return;
    }

    setSavingWatch(true);
    setWatchMessage(null);

    try {
      const currentIntent = pendingWatchIntent;
      const reviewResolved =
        currentIntent?.mode === "review" &&
        currentIntent.fileId === selected.id;
      const nextReview =
        reviewResolved ? nextReviewItem(selected.id) : null;
      const updated = await api.clearWatchSourceForFile(selected.id);
      if (!updated) {
        return;
      }

      setSelected(updated);
      setWatchEditing(false);
      setWatchSourceLabel("");
      setWatchSourceUrl("");
      setWatchMessage("Watch source cleared.");
      setPendingWatchIntent(null);
      await Promise.all([
        loadRows(updated.id),
        loadWatchCenter(),
        loadWatchList(),
        loadWatchReviewList(),
        loadWatchSetupList(),
      ]);

      if (reviewResolved) {
        if (nextReview) {
          await openWatchIntent(
            {
              fileId: nextReview.fileId,
              mode: "review",
              sourceKind: nextReview.watchResult.sourceKind ?? "exact_page",
              sourceLabel:
                nextReview.watchResult.sourceLabel ?? nextReview.subjectLabel,
            },
            "Watch source cleared. Opening the next review item.",
          );
        } else {
          setWatchCenterMessage(
            "Watch source cleared. No more watched items need review right now.",
          );
        }
      }
    } catch (error) {
      setWatchMessage(watchActionError(error, "clear the watch source"));
    } finally {
      setSavingWatch(false);
    }
  }

  async function refreshWatchSource() {
    if (!selected) {
      return;
    }

    setRefreshingWatch(true);
    setWatchMessage(null);

    try {
      const currentIntent = pendingWatchIntent;
      const reviewResolved =
        currentIntent?.mode === "review" &&
        currentIntent.fileId === selected.id;
      const nextReview =
        reviewResolved ? nextReviewItem(selected.id) : null;
      const updated = await api.refreshWatchSourceForFile(selected.id);
      if (!updated) {
        return;
      }

      setSelected(updated);
      setPendingWatchIntent(null);
      setWatchEditing(false);
      setWatchMessage("Watch result refreshed.");
      await Promise.all([
        loadRows(updated.id),
        loadWatchCenter(),
        loadWatchList(),
        loadWatchReviewList(),
        loadWatchSetupList(),
      ]);

      if (reviewResolved) {
        if (!updated.watchResult || !shouldShowWatchReviewAction(updated.watchResult)) {
          if (nextReview) {
            await openWatchIntent(
              {
                fileId: nextReview.fileId,
                mode: "review",
                sourceKind: nextReview.watchResult.sourceKind ?? "exact_page",
                sourceLabel:
                  nextReview.watchResult.sourceLabel ?? nextReview.subjectLabel,
              },
              "Watch result refreshed. Opening the next review item.",
            );
          } else {
            setWatchCenterMessage(
              "Watch result refreshed. No more watched items need review right now.",
            );
          }
        } else {
          setPendingWatchIntent(currentIntent);
          setWatchEditing(true);
          setWatchMessage(
            "Watch result refreshed. This source still needs review before SimSuite can rely on it.",
          );
        }
      }
    } catch (error) {
      setWatchMessage(watchActionError(error, "refresh the watch source"));
    } finally {
      setRefreshingWatch(false);
    }
  }

  async function refreshAllWatchedSources() {
    setRefreshingAllWatched(true);
    setWatchCenterMessage(null);

    try {
      const summary = await api.refreshWatchedSources();
      const checkedLabel =
        summary.checkedSubjects === 1
          ? "Checked 1 watched page."
          : `Checked ${summary.checkedSubjects} watched pages.`;
      const updateLabel =
        summary.exactUpdateItems === 1
          ? "1 confirmed update found."
          : `${summary.exactUpdateItems} confirmed updates found.`;

      setWatchCenterMessage(`${checkedLabel} ${updateLabel}`);
      await Promise.all([
        loadWatchCenter(),
        loadWatchList(),
        loadWatchReviewList(),
        loadWatchSetupList(),
        loadRows(selected?.id),
      ]);
    } catch (error) {
      setWatchCenterMessage(watchActionError(error, "check watched pages"));
    } finally {
      setRefreshingAllWatched(false);
    }
  }

  async function saveBulkExactWatchSources() {
    const entries = (watchSetupList?.exactPageItems ?? [])
      .map((item) => ({
        fileId: item.fileId,
        sourceKind: "exact_page" as const,
        sourceLabel: item.subjectLabel,
        sourceUrl: bulkExactWatchUrls[item.fileId]?.trim() ?? "",
      }))
      .filter((entry) => entry.sourceUrl.length > 0) satisfies SaveLibraryWatchSourceEntry[];

    if (!entries.length) {
      setBulkExactWatchMessage("Paste at least one official exact page URL first.");
      return;
    }

    setSavingBulkExactWatch(true);
    setBulkExactWatchMessage(null);
    setWatchCenterMessage(null);

    try {
      const results: Array<{
        fileId: number;
        saved: boolean;
        message: string;
      }> = [];

      for (const entry of entries) {
        try {
          const updated = await api.saveWatchSourceForFile(
            entry.fileId,
            entry.sourceKind,
            entry.sourceLabel,
            entry.sourceUrl,
          );

          results.push(
            updated
              ? {
                  fileId: entry.fileId,
                  saved: true,
                  message: "Watch source saved.",
                }
              : {
                  fileId: entry.fileId,
                  saved: false,
                  message: "Watch sources can only be saved for installed Library items.",
                },
          );
        } catch (error) {
          results.push({
            fileId: entry.fileId,
            saved: false,
            message: watchActionError(error, "save the watch source"),
          });
        }
      }

      const savedCount = results.filter((item) => item.saved).length;
      const failedCount = results.length - savedCount;
      const result = {
        savedCount,
        failedCount,
        results,
      };

      const nextErrors: Record<number, string> = {};
      for (const item of result.results) {
        if (!item.saved) {
          nextErrors[item.fileId] = item.message;
        }
      }
      setBulkExactWatchErrors(nextErrors);
      setBulkExactWatchUrls((current) => {
        const next = { ...current };
        for (const item of result.results) {
          if (item.saved) {
            delete next[item.fileId];
          }
        }
        return next;
      });

      if (
        selected &&
        result.results.some((item) => item.saved && item.fileId === selected.id)
      ) {
        setPendingWatchIntent(null);
        setWatchEditing(false);
        setWatchMessage(null);
      }

      await Promise.all([
        loadRows(selected?.id),
        loadWatchCenter(),
        loadWatchList(),
        loadWatchReviewList(),
        loadWatchSetupList(),
      ]);

      const singleSavedItem =
        result.savedCount === 1 && result.failedCount === 0
          ? result.results.find((item) => item.saved) ?? null
          : null;

      if (singleSavedItem) {
        const savedDetail = await api.getFileDetail(singleSavedItem.fileId);
        if (savedDetail?.watchResult && shouldShowWatchReviewAction(savedDetail.watchResult)) {
          await openWatchIntent(
            {
              fileId: savedDetail.id,
              mode: "review",
              sourceKind: savedDetail.watchResult.sourceKind ?? "exact_page",
              sourceLabel:
                savedDetail.watchResult.sourceLabel ??
                savedDetail.installedVersionSummary?.subjectLabel ??
                savedDetail.bundleName ??
                savedDetail.filename,
            },
            "Watch source saved. Review this source before SimSuite can rely on it.",
          );
          return;
        }
      }

      if (result.savedCount > 0) {
        setWatchCenterMessage(
          result.failedCount > 0
            ? `Saved ${result.savedCount} exact watch page${
                result.savedCount === 1 ? "" : "s"
              }. ${result.failedCount} still need attention.`
            : `Saved ${result.savedCount} exact watch page${
                result.savedCount === 1 ? "" : "s"
              }.`,
        );
      } else {
        setBulkExactWatchMessage(
          result.results[0]?.message ?? "SimSuite could not save these watch sources.",
        );
      }
    } catch (error) {
      setBulkExactWatchMessage(watchActionError(error, "save the watch sources"));
    } finally {
      setSavingBulkExactWatch(false);
    }
  }

  async function startWatchSetupFlow() {
    focusWatchSection("setup");
    if (firstSetupSuggestion) {
      await beginWatchSetup(firstSetupSuggestion);
      return;
    }

    if (!loadingWatchSetupList) {
      setWatchCenterMessage("Nothing strong enough needs watch setup right now.");
    }
  }

  function startWatchReviewFlow() {
    focusWatchSection("tracked");
    setWatchListFilter("attention");
    setQueuedWatchCenterAction("review");
    setWatchCenterMessage("Opening the next watched item that still needs review.");
  }

  const watchBusy = savingWatch || refreshingWatch;
  const libraryInspectorSections = selected
    ? [
        {
          id: "facts",
          label:
            userView === "beginner"
              ? "What matters"
              : isPowerView
                ? "File details"
                : "Overview",
          hint:
            isPowerView
              ? "Core classification, file metadata, and confidence."
              : "Only the details most simmers usually care about.",
          children: isPowerView ? (
            <div className="detail-list">
              <DetailRow
                label="Creator"
                value={selected.creator ?? unknownCreatorLabel(userView)}
              />
              <DetailRow
                label="Type"
                value={friendlyTypeLabel(selected.kind)}
              />
              <DetailRow
                label="Subtype"
                value={
                  selected.subtype ??
                  "Unspecified"
                }
              />
              <DetailRow label="Format" value={formatLibraryFileFormat(selected)} />
              <DetailRow label="Root" value={selected.sourceLocation} />
              <DetailRow label="Depth" value={`${selected.relativeDepth}`} />
              <DetailRow label="Size" value={formatBytes(selected.size)} />
              <DetailRow
                label="Modified"
                value={
                  selected.modifiedAt
                    ? new Date(selected.modifiedAt).toLocaleString()
                    : "Unknown"
                }
              />
              <DetailRow label="Hash" value={selected.hash ?? "Not available"} mono />
            </div>
          ) : (
            <>
              <div className="detail-list">
                <DetailRow
                  label="Creator"
                  value={selected.creator ?? unknownCreatorLabel(userView)}
                />
                <DetailRow
                  label="Type"
                  value={friendlyTypeLabel(selected.kind)}
                />
                {selected.subtype?.trim() ? (
                  <DetailRow label="Subtype" value={selected.subtype} />
                ) : null}
                <DetailRow label="File format" value={formatLibraryFileFormat(selected)} />
              </div>
              {playerFacingNames.length ? (
                <div className="detail-block">
                  <div className="section-label">Found in game as</div>
                  {playerFacingNames.length === 1 ? (
                    <p>{playerFacingNames[0]}</p>
                  ) : (
                    <div className="tag-list">
                      {playerFacingNames.map((item) => (
                        <span key={item} className="ghost-chip">
                          {item}
                        </span>
                      ))}
                    </div>
                  )}
                </div>
              ) : null}
            </>
          ),
        },
        ...(hasVersionWatchInfo
          ? [
              {
                id: "versionWatch",
                label: userView === "beginner" ? "Updates" : "Version and updates",
                hint:
                  isPowerView
                    ? "Installed version summary, local evidence, and watch status."
                    : "The installed version and any saved update tracking.",
                children: (
                  <>
                    <div className="detail-list">
                      {isPowerView ? (
                        <DetailRow
                          label="Subject"
                          value={
                            selected.installedVersionSummary?.subjectLabel ??
                            selected.filename
                          }
                        />
                      ) : null}
                      <DetailRow
                        label="Installed version"
                        value={
                          isPowerView
                            ? formatInstalledVersionValue(
                                selected.installedVersionSummary?.version ?? null,
                              )
                            : formatPlayerInstalledVersion(
                                selected.installedVersionSummary,
                              )
                        }
                      />
                      {isPowerView ? (
                        <DetailRow
                          label="Confidence"
                          value={versionConfidenceLabel(
                            selected.installedVersionSummary?.confidence ?? "unknown",
                          )}
                        />
                      ) : null}
                      <DetailRow
                        label="Watch status"
                        value={watchStatusLabel(selectedWatch, userView)}
                      />
                      {isPowerView && selectedWatch?.sourceKind ? (
                        <DetailRow
                          label="Watch source"
                          value={watchSourceKindLabel(selectedWatch)}
                        />
                      ) : null}
                      {isPowerView && selectedWatch?.sourceKind ? (
                        <DetailRow
                          label="Source origin"
                          value={watchSourceOriginLabel(selectedWatch.sourceOrigin)}
                        />
                      ) : null}
                      {isPowerView && selectedWatch?.sourceKind ? (
                        <DetailRow
                          label="Check method"
                          value={watchCapabilityLabel(selectedWatch, userView)}
                        />
                      ) : null}
                      {selectedWatch?.latestVersion ? (
                        <DetailRow
                          label="Latest seen"
                          value={selectedWatch.latestVersion}
                        />
                      ) : null}
                      {selectedWatch?.checkedAt ? (
                        <DetailRow
                          label="Last checked"
                          value={new Date(selectedWatch.checkedAt).toLocaleString()}
                        />
                      ) : null}
                    </div>
                    {!isPowerView &&
                    selected.installedVersionSummary?.version &&
                    !hasConfirmedInstalledVersion(selected.installedVersionSummary) ? (
                      <div className="detail-block">
                        <div className="section-label">Version note</div>
                        <p>
                          SimSuite found a possible version clue, but it is not strong enough
                          to trust yet.
                        </p>
                      </div>
                    ) : null}
                    {isPowerView && selected.installedVersionSummary?.evidence.length ? (
                      <div className="detail-block">
                        <div className="section-label">Local version evidence</div>
                        <div className="downloads-evidence-list">
                          {selected.installedVersionSummary.evidence.map((line) => (
                            <div key={line} className="downloads-evidence-row">
                              {line}
                            </div>
                          ))}
                        </div>
                      </div>
                    ) : null}
                    {isPowerView && (selectedWatch?.note || selectedWatch?.evidence.length) ? (
                      <div className="detail-block">
                        <div className="section-label">Watch notes</div>
                        <div className="downloads-evidence-list">
                          {selectedWatch?.note ? (
                            <div className="downloads-evidence-row">
                              {selectedWatch.note}
                            </div>
                          ) : null}
                          {selectedWatch?.evidence.map((line) => (
                            <div key={line} className="downloads-evidence-row">
                              {line}
                            </div>
                          ))}
                        </div>
                      </div>
                    ) : null}
                    <div className="detail-block">
                      <div className="section-label">
                        {isPowerView ? "Watch settings" : "Update check"}
                      </div>
                      {!watchEditing ? (
                        <div className="detail-row-actions">
                          {isBuiltInWatchSource ? (
                            <span className="creator-learning-message">
                              {builtInWatchSourceMessage(selectedWatch, appBehavior)}
                            </span>
                          ) : (
                            <button
                              type="button"
                              className="secondary-action"
                              onClick={() => {
                                setWatchEditing(true);
                                setWatchMessage(null);
                              }}
                            >
                              {selectedWatch?.sourceKind
                                ? "Change watch source"
                                : "Add watch source"}
                            </button>
                          )}
                          {selectedWatch?.sourceKind && selectedWatch.canRefreshNow ? (
                            <button
                              type="button"
                              className="secondary-action"
                              disabled={watchBusy}
                              onClick={() => void refreshWatchSource()}
                            >
                              {refreshingWatch ? "Checking..." : "Check now"}
                            </button>
                          ) : null}
                          {selectedWatch?.sourceKind &&
                          selectedWatch.capability === "provider_required" ? (
                            <span className="creator-learning-message">
                              {selectedWatch.providerName
                                ? `${selectedWatch.providerName} provider support is needed before SimSuite can check this page automatically.`
                                : "A provider setup is needed before SimSuite can check this page automatically."}
                            </span>
                          ) : null}
                          {!isBuiltInWatchSource && selectedWatch?.sourceKind ? (
                            <button
                              type="button"
                              className="ghost-action"
                              disabled={watchBusy}
                              onClick={() => void clearWatchSource()}
                            >
                              {userView === "beginner" ? "Stop watching" : "Clear watch source"}
                            </button>
                          ) : null}
                          {watchMessage ? (
                            <span className="creator-learning-message">{watchMessage}</span>
                          ) : null}
                        </div>
                      ) : (
                        <div className="creator-learning-grid">
                          <label className="field">
                            <span>Source type</span>
                            <select
                              value={watchSourceKind}
                              onChange={(event) =>
                                setWatchSourceKind(event.target.value as WatchSourceKind)
                              }
                            >
                              <option value="exact_page">Exact mod page</option>
                              <option value="creator_page">Creator page</option>
                            </select>
                          </label>
                          <label className="field">
                            <span>URL</span>
                            <input
                              value={watchSourceUrl}
                              onChange={(event) => setWatchSourceUrl(event.target.value)}
                              placeholder="https://example.com/mod-page"
                            />
                          </label>
                          <label className="field">
                            <span>Label (optional)</span>
                            <input
                              value={watchSourceLabel}
                              onChange={(event) => setWatchSourceLabel(event.target.value)}
                              placeholder={
                                watchSourceKind === "creator_page"
                                  ? "Creator name"
                                  : "Mod name on page"
                              }
                            />
                          </label>
                          <div className="learning-intro">
                            <strong>
                              {watchSourceKind === "creator_page"
                                ? "Creator pages are reminder links for now."
                                : "Exact pages work best when one page clearly belongs to one mod."}
                            </strong>
                            <span>
                              {watchSourceKind === "creator_page"
                                ? "SimSuite can save them now, but automatic creator-page checks are not built yet."
                                : "Some sites can be checked right away, while others are saved only as references until a safe official path exists."}
                            </span>
                          </div>
                          <div className="creator-learning-actions">
                            <button
                              type="button"
                              className="primary-action"
                              disabled={watchBusy || !watchSourceUrl.trim()}
                              onClick={() => void saveWatchSource()}
                            >
                              {savingWatch ? "Saving..." : "Save watch"}
                            </button>
                            {isSetupQueueActive ? (
                              <button
                                type="button"
                                className="secondary-action"
                                disabled={watchBusy}
                                onClick={() => void skipWatchSetup()}
                              >
                                {userView === "beginner" ? "Skip for now" : "Skip suggestion"}
                              </button>
                            ) : isReviewQueueActive ? (
                              <button
                                type="button"
                                className="secondary-action"
                                disabled={watchBusy}
                                onClick={() => void skipWatchReview()}
                              >
                                {userView === "beginner" ? "Skip for now" : "Skip review"}
                              </button>
                            ) : null}
                            <button
                              type="button"
                              className="secondary-action"
                              disabled={watchBusy}
                              onClick={() => closeWatchEditor()}
                            >
                              {isSetupQueueActive
                                ? userView === "beginner"
                                  ? "Stop setup"
                                  : "Stop queue"
                                : isReviewQueueActive
                                  ? userView === "beginner"
                                    ? "Done reviewing"
                                    : "Close review"
                                  : "Cancel"}
                            </button>
                            {watchMessage ? (
                              <span className="creator-learning-message">{watchMessage}</span>
                            ) : null}
                          </div>
                        </div>
                      )}
                    </div>
                  </>
                ),
              },
            ]
          : []),
        ...(showSafetySection
          ? [
              {
                id: "safety",
                label:
                  userView === "beginner"
                    ? "Keep together"
                    : isPowerView
                      ? "Bundle and warnings"
                      : "Care and bundle",
                hint: isPowerView
                  ? "Bundle grouping, warnings, and parser notes."
                  : "Shows grouped files and any warning notes that matter for normal play.",
                badge:
                  selected.safetyNotes.length > 0
                    ? `${selected.safetyNotes.length} warning${selected.safetyNotes.length === 1 ? "" : "s"}`
                    : selected.bundleName
                      ? "bundled"
                      : null,
                children: (
                  <>
                    {selected.bundleName ? (
                      <div className="detail-block">
                        <div className="section-label">
                          {userView === "beginner" ? "Keep with" : "Bundle"}
                        </div>
                        <p>
                          {selected.bundleName}
                          {isPowerView && selected.bundleType
                            ? ` (${selected.bundleType})`
                            : ""}
                        </p>
                      </div>
                    ) : null}

                    {selected.safetyNotes.length || isPowerView ? (
                      <div className="detail-block">
                        <div className="section-label">
                          {userView === "beginner" ? "Safety notes" : "Warnings"}
                        </div>
                        {selected.safetyNotes.length ? (
                          <div className="tag-list">
                            {selected.safetyNotes.map((note) => (
                              <span key={note} className="warning-tag">
                                {note}
                              </span>
                            ))}
                          </div>
                        ) : (
                          <p>No safety warnings.</p>
                        )}
                      </div>
                    ) : null}

                    {isPowerView ? (
                      <div className="detail-block">
                        <div className="section-label">Parser</div>
                        {selected.parserWarnings.length ? (
                          <div className="tag-list">
                            {selected.parserWarnings.map((note) => (
                              <span key={note} className="ghost-chip">
                                {note}
                              </span>
                            ))}
                          </div>
                        ) : (
                          <p>No parser warnings.</p>
                        )}
                      </div>
                    ) : null}
                  </>
                ),
              },
            ]
          : []),
        ...(isPowerView && hasInspectionSignals
          ? [
              {
                id: "inspection",
                label: "Inside the file",
                hint: "Signals pulled from package or script contents.",
                defaultCollapsed: false,
                badge: selected.insights.format ?? null,
                children: (
                  <>
                    {selected.insights.format ? (
                      <DetailRow label="Format" value={selected.insights.format} />
                    ) : null}
                    {selected.insights.creatorHints.length ? (
                      <div className="detail-block">
                        <div className="section-label">Creator names found</div>
                        <div className="tag-list">
                          {selected.insights.creatorHints.map((hint) => (
                            <span key={hint} className="ghost-chip">
                              {hint}
                            </span>
                          ))}
                        </div>
                      </div>
                    ) : null}
                    {selected.insights.versionHints.length ? (
                      <div className="detail-block">
                        <div className="section-label">Version numbers found</div>
                        <div className="tag-list">
                          {selected.insights.versionHints.map((item) => (
                            <span key={item} className="ghost-chip">
                              {item}
                            </span>
                          ))}
                        </div>
                      </div>
                    ) : null}
                    {selected.insights.resourceSummary.length ? (
                      <div className="detail-block">
                        <div className="section-label">Package contents</div>
                        <div className="tag-list">
                          {selected.insights.resourceSummary.map((item) => (
                            <span key={item} className="ghost-chip">
                              {item}
                            </span>
                          ))}
                        </div>
                      </div>
                    ) : null}
                    {selected.insights.scriptNamespaces.length ? (
                      <div className="detail-block">
                        <div className="section-label">Script folders</div>
                        <div className="tag-list">
                          {selected.insights.scriptNamespaces.map((item) => (
                            <span key={item} className="ghost-chip">
                              {item}
                            </span>
                          ))}
                        </div>
                      </div>
                    ) : null}
                    {selected.insights.embeddedNames.length ? (
                      <div className="detail-block">
                        <div className="section-label">In-game names</div>
                        <div className="tag-list">
                          {selected.insights.embeddedNames.map((item) => (
                            <span key={item} className="ghost-chip">
                              {item}
                            </span>
                          ))}
                        </div>
                      </div>
                    ) : null}
                    {selected.insights.familyHints.length ? (
                      <div className="detail-block">
                        <div className="section-label">Family hints</div>
                        <div className="tag-list">
                          {selected.insights.familyHints.map((item) => (
                            <span key={item} className="ghost-chip">
                              {item}
                            </span>
                          ))}
                        </div>
                      </div>
                    ) : null}
                  </>
                ),
              },
            ]
          : []),
        ...(isPowerView
          ? [
              {
                id: "creator",
                label: "Creator learning",
                hint: "Save creator matches and optional folder preferences.",
                defaultCollapsed: false,
                children: (
                  <CreatorLearningBlock
                    userView={userView}
                    creatorDraft={creatorDraft}
                    aliasDraft={aliasDraft}
                    lockPreference={lockPreference}
                    preferredPathDraft={preferredPathDraft}
                    savingCreator={savingCreator}
                    creatorMessage={creatorMessage}
                    suggestions={creatorSuggestions}
                    creatorLearning={selected.creatorLearning}
                    onCreatorDraftChange={(value) => {
                      setCreatorDraft(value);
                      setCreatorMessage(null);
                    }}
                    onAliasDraftChange={(value) => {
                      setAliasDraft(value);
                      setCreatorMessage(null);
                    }}
                    onLockPreferenceChange={(value) => {
                      setLockPreference(value);
                      setCreatorMessage(null);
                    }}
                    onPreferredPathChange={(value) => {
                      setPreferredPathDraft(value);
                      setCreatorMessage(null);
                    }}
                    onSelectSuggestion={(value) => {
                      setCreatorDraft(value);
                      setCreatorMessage(null);
                    }}
                    onSave={() => void saveCreatorOverride()}
                    onOpenAudit={() => onNavigate("creatorAudit")}
                  />
                ),
              },
              {
                id: "category",
                label: "Type override",
                hint: "Override type and subtype for later scans and previews.",
                defaultCollapsed: false,
                children: (
                  <CategoryOverrideBlock
                    userView={userView}
                    kindOptions={categoryKindOptions}
                    categoryKindDraft={categoryKindDraft}
                    categorySubtypeDraft={categorySubtypeDraft}
                    categoryMessage={categoryMessage}
                    savingCategory={savingCategory}
                    categoryOverride={selected.categoryOverride}
                    onKindChange={(value) => {
                      setCategoryKindDraft(value);
                      setCategoryMessage(null);
                    }}
                    onSubtypeChange={(value) => {
                      setCategorySubtypeDraft(value);
                      setCategoryMessage(null);
                    }}
                    onSave={() => void saveCategoryClassification()}
                    onOpenAudit={() => onNavigate("categoryAudit")}
                  />
                ),
              },
              {
                id: "path",
                label: "Path",
                hint: "Absolute path for this indexed file.",
                defaultCollapsed: true,
                children: <div className="path-card">{selected.path}</div>,
              },
            ]
          : []),
      ]
    : [];

  return (
    <section className="screen-shell">
      <div className="screen-header-row">
        <div className="screen-heading">
          <p className="eyebrow">{userView === "beginner" ? "Your files" : "Index"}</p>
          <div className="screen-title-row">
            <LibraryBig size={18} strokeWidth={2} />
            <h1>{screenLabel("library", userView)}</h1>
          </div>
          <p className="workspace-toolbar-copy">{screenHelperLine("library", userView)}</p>
        </div>
        <div className="header-actions">
          <button
            type="button"
            className="secondary-action"
            onClick={() => onNavigate("home")}
          >
            <CornerUpLeft size={14} strokeWidth={2} />
            Home
          </button>
        </div>
      </div>

      <LayoutPresetBar
        title={userView === "beginner" ? "Quick filters" : "Library layout"}
        summary={
          userView === "beginner"
            ? "Keep the list calm by hiding or showing the filters when you need them."
            : userView === "power"
              ? "Saved layout presets for denser browse and inspection passes."
              : "Saved layout presets for browsing, inspecting, or catalog-style review."
        }
        presets={userView === "beginner" ? [] : LIBRARY_LAYOUT_PRESETS}
        activePreset={libraryLayoutPreset}
        onApplyPreset={(preset) =>
          applyLibraryLayoutPreset(preset as LibraryLayoutPreset)
        }
        filterToggle={{
          collapsed: libraryFiltersCollapsed,
          onToggle: () => setLibraryFiltersCollapsed(!libraryFiltersCollapsed),
          hiddenLabel: "Show filters",
          shownLabel: "Hide filters",
        }}
      />

      {!libraryFiltersCollapsed ? (
        <div className="panel-card filter-panel">
          <div className="filter-grid">
            <label className="field">
              <span>Search</span>
              <input
                value={search}
                onChange={(event) => {
                  setSearch(event.target.value);
                  setPage(0);
                }}
                placeholder="Name or creator"
              />
            </label>

            <label className="field">
              <span>Type</span>
              <select
                value={kind}
                onChange={(event) => {
                  setKind(event.target.value);
                  setPage(0);
                }}
              >
                <option value="">All</option>
                {facets?.kinds.map((item) => (
                  <option key={item} value={item}>
                    {friendlyTypeLabel(item)}
                  </option>
                ))}
              </select>
            </label>

            {userView === "power" ? (
              <label className="field">
                <span>Subtype</span>
                <select
                  value={subtype}
                  onChange={(event) => {
                    setSubtype(event.target.value);
                    setPage(0);
                  }}
                >
                  <option value="">All</option>
                  {facets?.subtypes.map((item) => (
                    <option key={item} value={item}>
                      {item}
                    </option>
                  ))}
                </select>
              </label>
            ) : null}

            <label className="field">
              <span>Creator</span>
              <select
                value={creator}
                onChange={(event) => {
                  setCreator(event.target.value);
                  setPage(0);
                }}
              >
                <option value="">All</option>
                {facets?.creators.map((item) => (
                  <option key={item} value={item}>
                    {item}
                  </option>
                ))}
              </select>
            </label>

            {userView !== "beginner" ? (
              <label className="field">
                <span>Root</span>
                <select
                  value={source}
                  onChange={(event) => {
                    setSource(event.target.value);
                    setPage(0);
                  }}
                >
                  <option value="">All</option>
                  {facets?.sources.map((item) => (
                    <option key={item} value={item}>
                      {item}
                    </option>
                  ))}
                </select>
              </label>
            ) : null}

            {userView !== "beginner" ? (
              <label className="field">
                <span>Confidence</span>
                <select
                  value={minConfidence}
                  onChange={(event) => {
                    setMinConfidence(event.target.value);
                    setPage(0);
                  }}
                >
                  <option value="">Any</option>
                  <option value="0.35">35%+</option>
                  <option value="0.55">55%+</option>
                  <option value="0.75">75%+</option>
                </select>
              </label>
            ) : null}
          </div>
        </div>
      ) : null}

      <div className="library-layout">
        <div className="panel-card table-panel library-table-panel">
          <div className="table-meta">
            <div>
              <strong>{rows?.total.toLocaleString() ?? "0"}</strong>
              <span>{userView === "beginner" ? "found" : "matches"}</span>
            </div>
            <div>
              <strong>{rows?.items.length.toLocaleString() ?? "0"}</strong>
              <span>{userView === "beginner" ? "on this page" : "visible"}</span>
            </div>
          </div>

          <div className="library-watch-toolbar">
            <div className="library-watch-summary">
              <div className="library-watch-summary-item">
                <strong>
                  {watchOverview
                    ? watchOverview.exactUpdateItems.toLocaleString()
                    : "0"}
                </strong>
                <span>{userView === "beginner" ? "confirmed updates" : "Exact updates"}</span>
              </div>
              <div className="library-watch-summary-item">
                <strong>
                  {watchOverview
                    ? watchOverview.possibleUpdateItems.toLocaleString()
                    : "0"}
                </strong>
                <span>{userView === "beginner" ? "possible updates" : "Possible updates"}</span>
              </div>
              <div className="library-watch-summary-item">
                <strong>
                  {watchOverview
                    ? watchOverview.unknownWatchItems.toLocaleString()
                    : "0"}
                </strong>
                <span>{userView === "beginner" ? "unclear watched items" : "Needs review"}</span>
              </div>
            </div>
            <div className="library-watch-copy">
              <strong>
                {appBehavior?.automaticWatchChecks
                  ? `Automatic checks are on every ${appBehavior.watchCheckIntervalHours} hours.`
                  : "Automatic checks are off."}
              </strong>
              <span>
                {appBehavior?.lastWatchCheckAt
                  ? `Last run ${new Date(appBehavior.lastWatchCheckAt).toLocaleString()}.`
                  : "No automatic watch check has run yet."}
              </span>
              {appBehavior?.lastWatchCheckError ? (
                <span>{`Last error: ${appBehavior.lastWatchCheckError}`}</span>
              ) : null}
            </div>
            <div className="library-watch-actions">
              {firstSetupSuggestion ? (
                <button
                  type="button"
                  className="secondary-action"
                  onClick={() => void startWatchSetupFlow()}
                >
                  {isSetupQueueActive
                    ? userView === "beginner"
                      ? "Resume setup"
                      : "Resume setup"
                    : userView === "beginner"
                      ? "Set up watched pages"
                      : "Work through setup"}
                </button>
              ) : null}
              {loadingWatchReviewList || (watchReviewList?.total ?? 0) > 0 ? (
                <button
                  type="button"
                  className="secondary-action"
                  disabled={loadingWatchReviewList}
                  onClick={() => startWatchReviewFlow()}
                >
                  {isReviewQueueActive
                    ? userView === "beginner"
                      ? "Resume review"
                      : "Resume review"
                    : userView === "beginner"
                      ? "Review watched pages"
                      : "Work through review"}
                </button>
              ) : null}
              <button
                type="button"
                className="secondary-action"
                disabled={refreshingAllWatched}
                onClick={() => void refreshAllWatchedSources()}
              >
                {refreshingAllWatched ? "Checking watched pages..." : "Check watched pages now"}
              </button>
              <button
                type="button"
                className="ghost-action"
                onClick={() => onNavigate("settings")}
              >
                Watch settings
              </button>
            </div>
            {watchCenterMessage ? (
              <div className="library-watch-message">{watchCenterMessage}</div>
            ) : null}
              <div
                ref={trackedWatchSectionRef}
                className={
                  focusedWatchSection === "tracked"
                    ? "library-watch-focus is-focused"
                    : "library-watch-focus"
                }
              >
                {loadingWatchReviewList || (watchReviewList?.total ?? 0) > 0 ? (
                  <div className="library-watch-review-lane">
                    <div className="library-watch-setup-heading">
                      <strong>
                        {userView === "beginner"
                          ? "Review queue"
                          : "Watch review queue"}
                      </strong>
                      <span>
                        {watchReviewList
                          ? `${watchReviewList.total.toLocaleString()} ${
                              userView === "beginner" ? "items" : "saved pages"
                            }`
                          : "Loading..."}
                      </span>
                    </div>

                    {watchReviewList ? (
                      <div className="library-watch-review-summary">
                        {watchReviewList.providerNeededCount > 0 ? (
                          <span className="watch-review-chip">
                            Provider needed{" "}
                            <strong>
                              {watchReviewList.providerNeededCount.toLocaleString()}
                            </strong>
                          </span>
                        ) : null}
                        {watchReviewList.referenceOnlyCount > 0 ? (
                          <span className="watch-review-chip">
                            Reminder only{" "}
                            <strong>
                              {watchReviewList.referenceOnlyCount.toLocaleString()}
                            </strong>
                          </span>
                        ) : null}
                        {watchReviewList.unknownResultCount > 0 ? (
                          <span className="watch-review-chip">
                            Still unclear{" "}
                            <strong>
                              {watchReviewList.unknownResultCount.toLocaleString()}
                            </strong>
                          </span>
                        ) : null}
                      </div>
                    ) : null}

                    <div className="library-watch-list">
                      {loadingWatchReviewList ? (
                        <div className="library-watch-empty">
                          {userView === "beginner"
                            ? "Loading the watch review queue..."
                            : "Loading watch review items..."}
                        </div>
                      ) : watchReviewList?.items.length ? (
                        watchReviewList.items.map((item) => (
                          <div
                            key={`review-${item.fileId}`}
                            className="library-watch-list-entry"
                          >
                            <button
                              type="button"
                              className="library-watch-list-item library-watch-list-main"
                              onClick={() => void openFileById(item.fileId)}
                            >
                              <div className="library-watch-list-copy">
                                <strong>{item.subjectLabel}</strong>
                                <span>
                                  {item.creator
                                    ? `${item.creator} · ${item.filename}`
                                    : item.filename}
                                </span>
                                <span>
                                  {item.installedVersion?.trim()
                                    ? userView === "beginner"
                                      ? `Installed version ${item.installedVersion}.`
                                      : `Installed ${item.installedVersion}.`
                                    : userView === "beginner"
                                      ? "Installed version is still not clear."
                                      : "Installed version is still unclear."}
                                </span>
                              </div>
                              <div className="library-watch-list-state">
                                <strong>
                                  {watchReviewReasonLabel(item.reviewReason, userView)}
                                </strong>
                                <span>{item.reviewHint}</span>
                              </div>
                            </button>
                            <button
                              type="button"
                              className="secondary-action library-watch-setup-action"
                              onClick={() => void beginWatchReview(item.fileId)}
                            >
                              {userView === "beginner" ? "Review" : "Review source"}
                            </button>
                          </div>
                        ))
                      ) : (
                        <div className="library-watch-empty">
                          Nothing needs watch review right now.
                        </div>
                      )}
                    </div>
                  </div>
                ) : null}

                <div className="library-watch-filter-row">
                  {WATCH_LIST_FILTERS.map((filterOption) => (
                    <button
                      key={filterOption.id}
                    type="button"
                    className={
                      watchListFilter === filterOption.id
                        ? "watch-filter-chip is-active"
                        : "watch-filter-chip"
                    }
                    onClick={() => setWatchListFilter(filterOption.id)}
                  >
                    <span>
                      {userView === "beginner"
                        ? filterOption.beginnerLabel
                        : filterOption.label}
                    </span>
                    {watchFilterCount(
                      filterOption.id,
                      watchOverview,
                      watchList,
                    ) !== null ? (
                      <strong>
                        {watchFilterCount(
                          filterOption.id,
                          watchOverview,
                          watchList,
                        )?.toLocaleString()}
                      </strong>
                    ) : null}
                  </button>
                ))}
              </div>

              <div className="library-watch-list">
                {loadingWatchList ? (
                  <div className="library-watch-empty">
                    {userView === "beginner"
                      ? "Loading the tracked watch items..."
                      : "Loading tracked watch items..."}
                  </div>
                ) : watchList?.items.length ? (
                  watchList.items.map((item) => (
                    <div
                      key={item.fileId}
                      className="library-watch-list-entry"
                    >
                      <button
                        type="button"
                        className="library-watch-list-item library-watch-list-main"
                        onClick={() => void openFileById(item.fileId)}
                      >
                        <div className="library-watch-list-copy">
                          <strong>{item.subjectLabel}</strong>
                          <span>
                            {item.creator
                              ? `${item.creator} · ${item.filename}`
                              : item.filename}
                          </span>
                          <span>
                            {watchListVersionLine(
                              item.installedVersion,
                              item.watchResult.latestVersion,
                              userView,
                            )}
                          </span>
                        </div>
                        <div className="library-watch-list-state">
                          <strong>{watchStatusLabel(item.watchResult, userView)}</strong>
                          <span>{watchCapabilityLabel(item.watchResult, userView)}</span>
                        </div>
                      </button>
                      {shouldShowWatchReviewAction(item.watchResult) ? (
                        <button
                          type="button"
                          className="secondary-action library-watch-setup-action"
                          onClick={() => void beginWatchReview(item.fileId)}
                        >
                          {userView === "beginner" ? "Review" : "Review source"}
                        </button>
                      ) : null}
                    </div>
                  ))
                ) : (
                  <div className="library-watch-empty">
                    {watchListEmptyMessage(watchListFilter, userView)}
                  </div>
                )}
              </div>

              <div
                ref={setupWatchSectionRef}
                className={
                  focusedWatchSection === "setup"
                    ? "library-watch-setup is-focused"
                    : "library-watch-setup"
                }
              >
                <div className="library-watch-setup-heading">
                  <strong>
                    {userView === "beginner" ? "Ready to set up" : "Setup suggestions"}
                  </strong>
                  <span>
                    {watchSetupList
                      ? `${watchSetupList.total.toLocaleString()} ${
                          userView === "beginner" ? "items" : "candidates"
                        }`
                      : "Loading..."}
                  </span>
                </div>

                {bulkExactSetupItems.length ? (
                  <div className="library-watch-bulk-panel">
                    <div className="library-watch-bulk-heading">
                      <strong>
                        {userView === "beginner"
                          ? "Save exact pages in one pass"
                          : "Bulk exact-page setup"}
                      </strong>
                      <span>
                        {watchSetupList?.exactPageTotal.toLocaleString() ?? "0"} exact page
                        {watchSetupList?.exactPageTotal === 1 ? "" : "s"}
                      </span>
                    </div>
                    <p className="library-watch-bulk-copy">
                      {userView === "beginner"
                        ? "Paste the official mod pages you already know. SimSuite will save every filled row at once."
                        : "Paste the strongest official exact-page links here and save the filled rows together."}
                    </p>
                    <div className="library-watch-bulk-list">
                      {bulkExactSetupItems.map((item) => (
                        <div
                          key={`bulk-exact-${item.fileId}`}
                          className="library-watch-bulk-row"
                        >
                          <button
                            type="button"
                            className="library-watch-bulk-subject"
                            onClick={() => void openFileById(item.fileId)}
                          >
                            <strong>{item.subjectLabel}</strong>
                            <span>
                              {item.creator
                                ? `${item.creator} · ${item.filename}`
                                : item.filename}
                            </span>
                          </button>
                          <div className="library-watch-bulk-fields">
                            <input
                              type="url"
                              value={bulkExactWatchUrls[item.fileId] ?? ""}
                              onChange={(event) => {
                                const nextValue = event.target.value;
                                setBulkExactWatchUrls((current) => ({
                                  ...current,
                                  [item.fileId]: nextValue,
                                }));
                                setBulkExactWatchErrors((current) => {
                                  if (!current[item.fileId]) {
                                    return current;
                                  }

                                  const next = { ...current };
                                  delete next[item.fileId];
                                  return next;
                                });
                              }}
                              placeholder="https://example.com/mod-page"
                              disabled={savingBulkExactWatch}
                            />
                            {bulkExactWatchErrors[item.fileId] ? (
                              <span className="library-watch-bulk-error">
                                {bulkExactWatchErrors[item.fileId]}
                              </span>
                            ) : null}
                          </div>
                        </div>
                      ))}
                    </div>
                    <div className="library-watch-bulk-actions">
                      <button
                        type="button"
                        className="secondary-action"
                        disabled={savingBulkExactWatch}
                        onClick={() => void saveBulkExactWatchSources()}
                      >
                        {savingBulkExactWatch
                          ? "Saving exact pages..."
                          : "Save filled exact pages"}
                      </button>
                      <span>
                        {watchSetupList?.exactPageTruncated
                          ? "Showing the strongest exact-page candidates first."
                          : "Exact-page suggestions are the easiest batch win right now."}
                      </span>
                    </div>
                    {bulkExactWatchMessage ? (
                      <div className="library-watch-message">
                        {bulkExactWatchMessage}
                      </div>
                    ) : null}
                  </div>
                ) : null}

                <div className="library-watch-list">
                  {loadingWatchSetupList ? (
                    <div className="library-watch-empty">
                      {userView === "beginner"
                        ? "Finding installed items that still need a watch page..."
                        : "Finding watch setup suggestions..."}
                    </div>
                  ) : visibleSetupItems.length ? (
                    visibleSetupItems.map((item) => (
                      <div
                        key={`setup-${item.fileId}`}
                        className="library-watch-list-entry"
                      >
                        <button
                          type="button"
                          className="library-watch-list-item library-watch-list-main"
                          onClick={() => void openFileById(item.fileId)}
                        >
                          <div className="library-watch-list-copy">
                            <strong>{item.subjectLabel}</strong>
                            <span>
                              {item.creator
                                ? `${item.creator} · ${item.filename}`
                                : item.filename}
                            </span>
                            <span>
                              {item.installedVersion?.trim()
                                ? userView === "beginner"
                                  ? `Installed version ${item.installedVersion}.`
                                  : `Installed ${item.installedVersion}.`
                                : userView === "beginner"
                                  ? "Installed version is still not clear."
                                  : "Installed version is still unclear."}
                            </span>
                          </div>
                          <div className="library-watch-list-state">
                            <strong>
                              {item.suggestedSourceKind === "creator_page"
                                ? "Creator page"
                                : "Exact page"}
                            </strong>
                            <span>{item.setupHint}</span>
                          </div>
                        </button>
                        <button
                          type="button"
                          className="secondary-action library-watch-setup-action"
                          onClick={() => void beginWatchSetup(item)}
                        >
                          {userView === "beginner" ? "Set up" : "Start setup"}
                        </button>
                      </div>
                    ))
                  ) : bulkExactSetupItems.length ? (
                    <div className="library-watch-empty">
                      {userView === "beginner"
                        ? "The strongest exact-page items are ready above."
                        : "The strongest exact-page suggestions are ready in the bulk setup strip above."}
                    </div>
                  ) : (
                    <div className="library-watch-empty">
                      {userView === "beginner"
                        ? "Nothing strong enough needs watch setup right now."
                        : "No strong watch setup suggestions right now."}
                    </div>
                  )}
                </div>

                {watchSetupList?.truncated ? (
                  <div className="library-watch-empty">
                    {userView === "beginner"
                      ? "This is the strongest shortlist for now. Open Library items to set up more watch pages."
                      : "Showing the strongest local setup suggestions first."}
                  </div>
                ) : null}
              </div>
            </div>
          </div>

          <div className="vertical-dock library-table-dock">
            <div className="table-scroll library-table-scroll">
              <table className="library-table">
                <thead>
                  <tr>
                    <th>{userView === "beginner" ? "File" : "Name"}</th>
                    <th>Creator</th>
                    <th>Type</th>
                    {tableColumns.includes("root") ? (
                      <th>{userView === "beginner" ? "Folder" : "Root"}</th>
                    ) : null}
                    {tableColumns.includes("depth") ? <th>Depth</th> : null}
                    <th>{userView === "beginner" ? "Match" : "Confidence"}</th>
                  </tr>
                </thead>
                <tbody>
                  {rows?.items.length ? (
                    rows.items.map((row, index) => (
                      <m.tr
                        key={row.id}
                        className={selected?.id === row.id ? "is-selected" : ""}
                        onClick={() => void openFile(row)}
                        role="button"
                        tabIndex={0}
                        onKeyDown={(event) => {
                          if (event.key === "Enter" || event.key === " ") {
                            event.preventDefault();
                            void openFile(row);
                          }
                        }}
                        whileHover={rowHover}
                        whileTap={rowPress}
                        {...stagedListItem(index)}
                      >
                        <td>
                          <div className="file-title">{row.filename}</div>
                          <div className="file-path">{row.path}</div>
                        </td>
                        <td>{row.creator ?? unknownCreatorLabel(userView)}</td>
                        <td>
                          <span className={`kind-pill kind-${kindSlug(row.kind)}`}>
                            {friendlyTypeLabel(row.kind)}
                          </span>
                        </td>
                        {tableColumns.includes("root") ? (
                          <td>
                            <span className="source-pill">{row.sourceLocation}</span>
                          </td>
                        ) : null}
                        {tableColumns.includes("depth") ? (
                          <td>{row.relativeDepth}</td>
                        ) : null}
                        <td>
                          <span
                            className={`confidence-badge ${confidenceTone(
                              row.confidence,
                            )}`}
                          >
                            {Math.round(row.confidence * 100)}%
                          </span>
                        </td>
                      </m.tr>
                    ))
                  ) : (
                    <tr>
                      <td colSpan={tableColumns.length} className="empty-row">
                        {userView === "beginner"
                          ? "Nothing matches these filters."
                          : "No indexed files match the current filters."}
                      </td>
                    </tr>
                  )}
                </tbody>
              </table>
            </div>
            <ResizableEdgeHandle
              label="Resize library table height"
              value={libraryTableHeight}
              min={260}
              max={860}
              onChange={setLibraryTableHeight}
              side="bottom"
              className="dock-resize-handle library-table-height-handle"
            />
          </div>

          <div className="table-footer">
            <button
              type="button"
              className="secondary-action"
              onClick={() => setPage((current) => Math.max(current - 1, 0))}
              disabled={page === 0}
            >
              Previous
            </button>
            <div className="table-page-label">
              Page {Math.min(page + 1, totalPages)} of {totalPages}
            </div>
            <button
              type="button"
              className="secondary-action"
              onClick={() => setPage((current) => current + 1)}
              disabled={!rows || page + 1 >= totalPages}
            >
              Next
            </button>
          </div>
        </div>

        <ResizableDetailPanel
          ariaLabel="Library inspector"
          width={libraryDetailWidth}
          onWidthChange={setLibraryDetailWidth}
          maxWidth={760}
        >
          {selected ? (
            <>
              <div className="detail-header">
                <div>
                  <p className="eyebrow">{userView === "beginner" ? "Selected file" : "Inspector"}</p>
                  <h2>{selected.filename}</h2>
                </div>
                <span className={`confidence-badge ${confidenceTone(selected.confidence)}`}>
                  {Math.round(selected.confidence * 100)}%
                </span>
              </div>

              <DockSectionStack
                layoutId="libraryInspector"
                sections={libraryInspectorSections}
                intro={
                  userView === "beginner"
                    ? "Open the parts you need, hide the rest, and move sections up or down."
                    : "Collapse, reorder, and reset inspector sections to fit your workflow."
                }
              />
            </>
          ) : (
            <div className="detail-empty">
              <p className="eyebrow">{userView === "beginner" ? "Selected file" : "Inspector"}</p>
              <h2>Select a file</h2>
            </div>
          )}
        </ResizableDetailPanel>
      </div>
    </section>
  );
}

function CreatorLearningBlock({
  userView,
  creatorDraft,
  aliasDraft,
  lockPreference,
  preferredPathDraft,
  savingCreator,
  creatorMessage,
  suggestions,
  creatorLearning,
  onCreatorDraftChange,
  onAliasDraftChange,
  onLockPreferenceChange,
  onPreferredPathChange,
  onSelectSuggestion,
  onSave,
  onOpenAudit,
}: {
  userView: UserView;
  creatorDraft: string;
  aliasDraft: string;
  lockPreference: boolean;
  preferredPathDraft: string;
  savingCreator: boolean;
  creatorMessage: string | null;
  suggestions: string[];
  creatorLearning: CreatorLearningInfo;
  onCreatorDraftChange: (value: string) => void;
  onAliasDraftChange: (value: string) => void;
  onLockPreferenceChange: (value: boolean) => void;
  onPreferredPathChange: (value: string) => void;
  onSelectSuggestion: (value: string) => void;
  onSave: () => void;
  onOpenAudit: () => void;
}) {
  const showAdvanced = userView !== "beginner";

  return (
    <div className="creator-learning-block">
      <div className="creator-learning-header">
        <div className="section-label">
          {userView === "beginner" ? "Set creator" : "Creator learning"}
        </div>
        {creatorLearning.lockedByUser ? (
          <span className="ghost-chip">
            {userView === "beginner" ? "saved folder" : "locked route"}
          </span>
        ) : null}
      </div>

      <div className="learning-intro">
        <strong>
          {userView === "beginner"
            ? "Use this when the creator is wrong or missing."
            : "Save the creator for this file."}
        </strong>
        <span>
          {userView === "beginner"
            ? "Saving here helps future scans recognize this creator again."
            : "This saves a creator name or extra clue and reuses it in later scans."}
        </span>
      </div>

      {suggestions.length ? (
        <div className="creator-suggestion-strip">
          {suggestions.map((value) => (
            <button
              key={value}
              type="button"
              className={`creator-suggestion ${creatorDraft === value ? "is-active" : ""}`}
              onClick={() => onSelectSuggestion(value)}
              title={`Use ${value}`}
            >
              {value}
            </button>
          ))}
        </div>
      ) : null}

      <div className={`creator-learning-grid ${showAdvanced ? "" : "is-compact"}`}>
        <label className="field">
          <span>Creator</span>
          <input
            value={creatorDraft}
            onChange={(event) => onCreatorDraftChange(event.target.value)}
            placeholder={userView === "beginner" ? "Creator name" : "Creator"}
          />
        </label>

        {showAdvanced ? (
          <label className="field">
            <span>Also save this clue</span>
            <input
              value={aliasDraft}
              onChange={(event) => onAliasDraftChange(event.target.value)}
              placeholder="[creator] / file tag"
            />
          </label>
        ) : null}
      </div>

      {showAdvanced ? (
        <>
          <label className="creator-toggle">
            <input
              type="checkbox"
              checked={lockPreference}
              onChange={(event) => onLockPreferenceChange(event.target.checked)}
            />
            <span>Lock this creator to one preview folder</span>
          </label>

          {lockPreference ? (
            <label className="field">
              <span>Preferred path</span>
              <input
                value={preferredPathDraft}
                onChange={(event) => onPreferredPathChange(event.target.value)}
                placeholder="Relative to Mods or Tray root"
              />
            </label>
          ) : null}
        </>
      ) : null}

      {creatorLearning.learnedAliases.length ? (
        <div className="creator-known-strip">
          {creatorLearning.learnedAliases.map((alias) => (
            <span key={alias} className="ghost-chip">
              {alias}
            </span>
          ))}
        </div>
      ) : null}

      <div className="creator-learning-actions">
        <button
          type="button"
          className="primary-action"
          disabled={!creatorDraft.trim() || savingCreator}
          onClick={onSave}
        >
          {savingCreator
            ? "Saving..."
            : userView === "beginner"
              ? "Save creator"
              : "Save creator"}
        </button>
        <button type="button" className="secondary-action" onClick={onOpenAudit}>
          {userView === "beginner" ? "Fix many creators" : "Open creator batches"}
        </button>
        {creatorMessage ? <span className="creator-learning-message">{creatorMessage}</span> : null}
      </div>
    </div>
  );
}

function CategoryOverrideBlock({
  userView,
  kindOptions,
  categoryKindDraft,
  categorySubtypeDraft,
  categoryMessage,
  savingCategory,
  categoryOverride,
  onKindChange,
  onSubtypeChange,
  onSave,
  onOpenAudit,
}: {
  userView: UserView;
  kindOptions: string[];
  categoryKindDraft: string;
  categorySubtypeDraft: string;
  categoryMessage: string | null;
  savingCategory: boolean;
  categoryOverride: CategoryOverrideInfo;
  onKindChange: (value: string) => void;
  onSubtypeChange: (value: string) => void;
  onSave: () => void;
  onOpenAudit: () => void;
}) {
  const showSubtype = userView !== "beginner";

  return (
    <div className="category-override-block">
      <div className="creator-learning-header">
        <div className="section-label">
          {userView === "beginner" ? "Set type" : "Type override"}
        </div>
        {categoryOverride.savedByUser ? (
          <span className="ghost-chip">
            {userView === "beginner" ? "saved" : "manual"}
          </span>
        ) : null}
      </div>

      <div className="learning-intro">
        <strong>
          {userView === "beginner"
            ? "Use this when the type looks wrong."
            : "Set the type and subtype when the automatic guess is off."}
        </strong>
        <span>
          {userView === "beginner"
            ? "Saving here changes the label SimSuite uses later. It does not move the file yet."
            : "This changes the library label and future guesses. Moves still happen only from Organize."}
        </span>
      </div>

      <div className={`creator-learning-grid ${showSubtype ? "" : "is-compact"}`}>
        <label className="field">
          <span>Type</span>
          <select
            value={categoryKindDraft}
            onChange={(event) => onKindChange(event.target.value)}
          >
            <option value="">{userView === "beginner" ? "Choose a type" : "Select type"}</option>
            {kindOptions.map((item) => (
              <option key={item} value={item}>
                {friendlyTypeLabel(item)}
              </option>
            ))}
          </select>
        </label>

        {showSubtype ? (
          <label className="field">
            <span>Subtype</span>
            <input
              value={categorySubtypeDraft}
              onChange={(event) => onSubtypeChange(event.target.value)}
              placeholder="Optional subtype"
            />
          </label>
        ) : null}
      </div>

      <div className="creator-learning-actions">
        <button
          type="button"
          className="primary-action"
          disabled={!categoryKindDraft.trim() || savingCategory}
          onClick={onSave}
        >
          {savingCategory
            ? "Saving..."
            : userView === "beginner"
              ? "Save type"
              : "Save type"}
        </button>
        <button type="button" className="secondary-action" onClick={onOpenAudit}>
          {userView === "beginner" ? "Fix many types" : "Open type batches"}
        </button>
        {categoryMessage ? <span className="creator-learning-message">{categoryMessage}</span> : null}
      </div>
    </div>
  );
}

function DetailRow({
  label,
  value,
  mono,
}: {
  label: string;
  value: string;
  mono?: boolean;
}) {
  return (
    <div className="detail-row">
      <span>{label}</span>
      <strong className={mono ? "mono-text" : ""}>{value}</strong>
    </div>
  );
}

function formatLibraryFileFormat(detail: FileDetail) {
  if (detail.insights.format === "ts4script-zip" || detail.extension === ".ts4script") {
    return "Script archive (.ts4script)";
  }
  if (detail.insights.format === "dbpf-package" || detail.extension === ".package") {
    return "Package file (.package)";
  }
  return detail.insights.format ?? detail.extension ?? "Unknown";
}

function collectPlayerFacingNames(detail: FileDetail) {
  const normalizedFilename = normalizeForNameComparison(detail.filename);

  return detail.insights.embeddedNames
    .map((value) => value.trim())
    .filter((value) => {
      if (!value || value.length < 3 || value.length > 72) {
        return false;
      }
      if (/^0x[0-9a-f]+$/i.test(value) || /[\\/]/.test(value)) {
        return false;
      }

      const normalizedValue = normalizeForNameComparison(value);
      if (!normalizedValue || normalizedValue === normalizedFilename) {
        return false;
      }

      const letters = value.match(/[a-z]/gi)?.length ?? 0;
      return letters >= 3;
    })
    .filter((value, index, values) =>
      values.findIndex((candidate) => candidate.toLowerCase() === value.toLowerCase()) === index,
    )
    .slice(0, 4);
}

function normalizeForNameComparison(value: string) {
  return value
    .replace(/\.[^.]+$/, "")
    .replace(/[_\-\s]+/g, "")
    .toLowerCase()
    .trim();
}

function formatPlayerInstalledVersion(summary: InstalledVersionSummary | null) {
  if (!summary?.version?.trim()) {
    return "Not confirmed yet";
  }

  return hasConfirmedInstalledVersion(summary)
    ? summary.version
    : "Not confirmed yet";
}

function hasConfirmedInstalledVersion(summary: InstalledVersionSummary | null) {
  return Boolean(
    summary?.version?.trim() &&
      (summary.confidence === "exact" || summary.confidence === "strong"),
  );
}

function formatInstalledVersionValue(value: string | null) {
  return value?.trim() ? value : "Not clear yet";
}

function versionConfidenceLabel(confidence: VersionConfidence) {
  switch (confidence) {
    case "exact":
      return "Exact";
    case "strong":
      return "Strong";
    case "medium":
      return "Medium";
    case "weak":
      return "Weak";
    default:
      return "Unknown";
  }
}

function watchFilterCount(
  filter: WatchListFilter,
  watchOverview: HomeOverview | null,
  watchList: LibraryWatchListResponse | null,
) {
  if (!watchOverview) {
    return filter === "all" && watchList?.filter === "all" ? watchList.total : null;
  }

  switch (filter) {
    case "attention":
      return (
        watchOverview.exactUpdateItems +
        watchOverview.possibleUpdateItems +
        watchOverview.unknownWatchItems
      );
    case "exact_updates":
      return watchOverview.exactUpdateItems;
    case "possible_updates":
      return watchOverview.possibleUpdateItems;
    case "unclear":
      return watchOverview.unknownWatchItems;
    case "all":
      return watchList?.filter === "all" ? watchList.total : null;
  }
}

function watchListVersionLine(
  installedVersion: string | null,
  latestVersion: string | null,
  userView: UserView,
) {
  const installedLabel = installedVersion?.trim() ? installedVersion : "not clear";
  if (latestVersion?.trim()) {
    return userView === "beginner"
      ? `Installed ${installedLabel}. Latest helper check ${latestVersion}.`
      : `Installed ${installedLabel} · Latest helper ${latestVersion}`;
  }

  return userView === "beginner"
    ? `Installed ${installedLabel}. No helper version recorded yet.`
    : `Installed ${installedLabel} · No helper version yet`;
}

function watchListEmptyMessage(filter: WatchListFilter, userView: UserView) {
  switch (filter) {
    case "attention":
      return userView === "beginner"
        ? "Nothing in the tracked watch list needs attention right now."
        : "No tracked watch items need attention right now.";
    case "exact_updates":
      return userView === "beginner"
        ? "No confirmed updates are tracked right now."
        : "No tracked exact updates right now.";
    case "possible_updates":
      return userView === "beginner"
        ? "No possible updates are waiting right now."
        : "No tracked possible updates right now.";
    case "unclear":
      return userView === "beginner"
        ? "No tracked items are unclear right now."
        : "No tracked unclear watch results right now.";
    case "all":
      return userView === "beginner"
        ? "No tracked watch items are set up yet."
        : "No tracked watch items yet.";
  }
}

function shouldShowWatchReviewAction(watchResult: WatchResult) {
  return (
    watchResult.sourceOrigin === "saved_by_user" &&
    (watchResult.capability !== "can_refresh_now" || watchResult.status === "unknown")
  );
}

function watchReviewReasonLabel(
  reason: LibraryWatchReviewReason,
  userView: UserView,
) {
  switch (reason) {
    case "provider_needed":
      return userView === "beginner" ? "Provider needed" : "Provider needed";
    case "reference_only":
      return userView === "beginner" ? "Reminder only" : "Reference only";
    case "unknown_result":
      return userView === "beginner" ? "Still unclear" : "Still unclear";
    default:
      return userView === "beginner" ? "Needs review" : "Needs review";
  }
}

function watchStatusLabel(watchResult: WatchResult | null, userView: UserView) {
  if (!watchResult) {
    return userView === "beginner" ? "Not watched yet" : "Not watched";
  }

  const isBuiltIn = watchResult.sourceOrigin === "built_in_special";

  switch (watchResult.status) {
    case "exact_update_available":
      return userView === "beginner"
        ? "Confirmed update found"
        : "Exact update available";
    case "possible_update":
      return userView === "beginner"
        ? "Possible update spotted"
        : "Possible update";
    case "current":
      return userView === "beginner" ? "Looks current" : "Looks current";
    case "not_watched":
      if (watchResult.sourceKind) {
        if (watchResult.capability === "provider_required") {
          return isBuiltIn
            ? userView === "beginner"
              ? "Built-in page needs provider setup"
              : "Built-in page needs provider"
            : userView === "beginner"
              ? "Saved, provider setup needed"
              : "Saved, provider needed";
        }
        return watchResult.canRefreshNow
          ? isBuiltIn
            ? userView === "beginner"
              ? "Built-in page ready to check"
              : "Built-in page ready"
            : userView === "beginner"
              ? "Saved and ready to check"
              : "Saved and ready"
          : isBuiltIn
            ? userView === "beginner"
              ? "Built-in page is reference only"
              : "Built-in reference only"
            : userView === "beginner"
              ? "Saved as a reference"
              : "Saved as reference";
      }
      return userView === "beginner" ? "Not watched yet" : "Not watched";
    default:
      if (watchResult.capability === "provider_required") {
        return userView === "beginner"
          ? "Provider setup still needed"
          : "Provider needed";
      }
      return userView === "beginner"
        ? "Watch result is still unclear"
        : "Unknown";
  }
}

function watchSourceKindLabel(watchResult: WatchResult | null) {
  if (!watchResult?.sourceKind) {
    return "Not set";
  }

  switch (watchResult.sourceKind) {
    case "exact_page":
      return "Exact mod page";
    case "creator_page":
      return "Creator page";
    default:
      return "Saved source";
  }
}

function watchSourceOriginLabel(origin: WatchSourceOrigin) {
  switch (origin) {
    case "built_in_special":
      return "Built-in official page";
    case "saved_by_user":
      return "Saved by you";
    default:
      return "Not saved";
  }
}

function watchCapabilityLabel(watchResult: WatchResult | null, userView: UserView) {
  if (!watchResult?.sourceKind) {
    return userView === "beginner" ? "No watch source yet" : "Not set";
  }

  const isBuiltIn = watchResult.sourceOrigin === "built_in_special";

  switch (watchResult.capability) {
    case "can_refresh_now":
      return isBuiltIn
        ? userView === "beginner"
          ? "Built-in page can be checked now"
          : "Built-in check now supported"
        : userView === "beginner"
          ? "SimSuite can check this now"
          : "Check now supported";
    case "provider_required":
      return watchResult.providerName
        ? `${watchResult.providerName} provider needed`
        : "Provider support needed";
    default:
      return isBuiltIn
        ? userView === "beginner"
          ? "Built-in page is reference only"
          : "Built-in reference only"
        : userView === "beginner"
          ? "Saved as a reminder only"
          : "Reference only";
  }
}

function builtInWatchSourceMessage(
  watchResult: WatchResult | null,
  appBehavior: AppBehaviorSettings | null,
) {
  if (!watchResult) {
    return "SimSuite is using the built-in official page for this supported mod.";
  }

  if (watchResult.capability === "provider_required") {
    return watchResult.providerName
      ? `SimSuite is using the built-in official page for this supported mod. ${watchResult.providerName} support is still needed before automatic checks can work here.`
      : "SimSuite is using the built-in official page for this supported mod, but automatic checks are not available here yet.";
  }

  if (!watchResult.canRefreshNow) {
    return "SimSuite is using the built-in official page for this supported mod. This page is reference-only right now.";
  }

  if (appBehavior?.automaticWatchChecks) {
    return `SimSuite is using the built-in official page for this supported mod. Automatic checks are on every ${appBehavior.watchCheckIntervalHours} hours.`;
  }

  return "SimSuite is using the built-in official page for this supported mod. Automatic checks are off, but you can still use Check now here.";
}

function formatBytes(size: number) {
  if (size < 1024) {
    return `${size} B`;
  }

  if (size < 1024 * 1024) {
    return `${(size / 1024).toFixed(1)} KB`;
  }

  return `${(size / (1024 * 1024)).toFixed(1)} MB`;
}

function watchActionError(error: unknown, action: string) {
  if (error instanceof Error && error.message.trim()) {
    return error.message;
  }
  if (typeof error === "string" && error.trim()) {
    return error;
  }
  return `SimSuite could not ${action}.`;
}

function kindSlug(kind: string) {
  return kind.toLowerCase().replace(/[^a-z0-9]+/g, "-");
}

function confidenceTone(confidence: number) {
  if (confidence >= 0.85) {
    return "good";
  }

  if (confidence >= 0.6) {
    return "medium";
  }

  return "low";
}
