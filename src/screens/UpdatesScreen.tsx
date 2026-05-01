import { useEffect, useState } from "react";
import { AnimatePresence, m } from "motion/react";
import {
  AlertTriangle,
  Bookmark,
  CheckCircle2,
  ExternalLink,
  Eye,
  HelpCircle,
  LibraryBig,
  ListChecks,
  PanelLeftClose,
  PanelLeftOpen,
  RefreshCw,
  ScanSearch,
  Settings2,
  Trash2,
  X,
} from "lucide-react";
import { Workbench } from "../components/layout/Workbench";
import { WorkbenchInspector } from "../components/layout/WorkbenchInspector";
import { WorkbenchRail } from "../components/layout/WorkbenchRail";
import { WorkbenchStage } from "../components/layout/WorkbenchStage";
import { useUiPreferences } from "../components/UiPreferencesContext";
import { api } from "../lib/api";
import {
  overlayTransition,
  panelSpring,
  rowHover,
  rowPress,
  stagedListItem,
} from "../lib/motion";
import {
  friendlyTypeLabel,
  screenHelperLine,
  unknownCreatorLabel,
} from "../lib/uiLanguage";
import type {
  FileDetail,
  LibraryFileRow,
  LibraryWatchListItem,
  LibraryWatchListResponse,
  LibraryWatchReviewItem,
  LibraryWatchReviewResponse,
  LibraryWatchSetupItem,
  LibraryWatchSetupResponse,
  Screen,
  UserView,
  WatchListFilter,
  WatchResult,
  WatchSourceKind,
} from "../lib/types";

const SOURCE_NEEDED_LIMIT = 200;

interface UpdatesScreenProps {
  refreshVersion: number;
  onNavigate: (screen: Screen) => void;
  onDataChanged?: () => void;
  userView: UserView;
  initialMode?: UpdateMode | "tracked" | "review";
  initialFilter?: WatchListFilter;
  initialFileId?: number;
}

type UpdateMode = "watching" | "setup" | "attention" | "reminders";
type WatchingFilter = "all" | "checked_recently" | "check_available";
type AttentionFilter =
  | "all"
  | "update_found"
  | "possible_update"
  | "could_not_check"
  | "provider_required";
type ReminderFilter = "all" | "creator_page" | "exact_page";
type SetupFilter = "all" | "possible_source" | "no_match";

type SourceNeededRow = {
  fileId: number;
  filename: string;
  creator: string | null;
  installedVersion: string | null;
  subjectLabel: string;
  suggestedSourceKind: WatchSourceKind | null;
  setupHint: string;
  suggestionState: "possible_source" | "no_match";
};

type AttentionRow =
  | { kind: "tracked"; item: LibraryWatchListItem }
  | { kind: "review"; item: LibraryWatchReviewItem };

const WATCHING_FILTERS: Array<{
  id: WatchingFilter;
  label: string;
  beginnerLabel: string;
}> = [
  { id: "all", label: "All watched", beginnerLabel: "All watched" },
  {
    id: "checked_recently",
    label: "No new update found",
    beginnerLabel: "No new update found",
  },
  {
    id: "check_available",
    label: "Check available",
    beginnerLabel: "Check available",
  },
];

const ATTENTION_FILTERS: Array<{
  id: AttentionFilter;
  label: string;
  beginnerLabel: string;
}> = [
  { id: "all", label: "All attention items", beginnerLabel: "All attention items" },
  { id: "update_found", label: "Update found", beginnerLabel: "Update found" },
  { id: "possible_update", label: "Possible update", beginnerLabel: "Possible update" },
  { id: "could_not_check", label: "Could not check", beginnerLabel: "Could not check" },
  { id: "provider_required", label: "Provider required", beginnerLabel: "Provider required" },
];

const REMINDER_FILTERS: Array<{
  id: ReminderFilter;
  label: string;
  beginnerLabel: string;
}> = [
  { id: "all", label: "All reminders", beginnerLabel: "All reminders" },
  { id: "creator_page", label: "Creator pages", beginnerLabel: "Creator pages" },
  { id: "exact_page", label: "Saved exact pages", beginnerLabel: "Saved exact pages" },
];

const SETUP_FILTERS: Array<{
  id: SetupFilter;
  label: string;
  beginnerLabel: string;
}> = [
  { id: "all", label: "All needing a source", beginnerLabel: "All needing a source" },
  {
    id: "possible_source",
    label: "Possible source found",
    beginnerLabel: "Possible source found",
  },
  { id: "no_match", label: "No match yet", beginnerLabel: "No match yet" },
];

function watchStatusIcon(status: WatchResult["status"]) {
  switch (status) {
    case "exact_update_available":
      return <CheckCircle2 className="updates-status-icon updates-status-icon-success" />;
    case "current":
      return <CheckCircle2 className="updates-status-icon updates-status-icon-current" />;
    case "possible_update":
      return <AlertTriangle className="updates-status-icon updates-status-icon-warning" />;
    default:
      return <HelpCircle className="updates-status-icon updates-status-icon-muted" />;
  }
}

function watchStatusLabel(status: WatchResult["status"], userView: UserView) {
  const labels: Record<WatchResult["status"], { beginner: string; advanced: string }> = {
    exact_update_available: {
      beginner: "Update found",
      advanced: "Update found",
    },
    possible_update: { beginner: "Possible update", advanced: "Possible update" },
    unknown: { beginner: "Couldn't confirm", advanced: "Couldn't confirm" },
    current: { beginner: "No new update found", advanced: "Checked recently" },
    not_watched: { beginner: "Check available", advanced: "Check available" },
  };

  return userView === "beginner" ? labels[status].beginner : labels[status].advanced;
}

function normalizeUpdateMode(mode?: UpdateMode | "tracked" | "review") {
  if (mode === "tracked") {
    return "watching" as const;
  }

  if (mode === "review") {
    return "attention" as const;
  }

  return mode;
}

function watchSourceKindLabel(kind: WatchSourceKind | null) {
  if (kind === "creator_page") {
    return "Creator page";
  }
  if (kind === "exact_page") {
    return "Exact mod page";
  }
  return "Not set";
}

function watchSourceOriginLabel(origin: WatchResult["sourceOrigin"]) {
  switch (origin) {
    case "built_in_special":
      return "Built-in page";
    case "saved_by_user":
      return "Saved by you";
    default:
      return "Not saved";
  }
}

function watchCapabilityLabel(watchResult: WatchResult | null, userView: UserView) {
  if (!watchResult?.sourceKind) {
    return userView === "beginner" ? "No source saved yet" : "No source saved";
  }

  switch (watchResult.capability) {
    case "can_refresh_now":
      return userView === "beginner"
        ? "Checks for updates automatically"
        : "Automatic checks supported";
    case "provider_required":
      return watchResult.providerName
        ? `${watchResult.providerName} required`
        : "Provider required";
    default:
      return userView === "beginner" ? "Saved for reminders" : "Reminder only";
  }
}

function reviewReasonLabel(
  reason: LibraryWatchReviewItem["reviewReason"],
  userView: UserView,
) {
  switch (reason) {
    case "provider_needed":
      return userView === "beginner" ? "Provider required" : "Provider required";
    case "reference_only":
      return userView === "beginner" ? "Saved for reminders" : "Reminder only";
    case "unknown_result":
      return userView === "beginner" ? "Couldn't confirm" : "Couldn't confirm";
    default:
      return userView === "beginner" ? "Needs review" : "Review";
  }
}

function formatCheckedAt(value: string | null) {
  return value ? new Date(value).toLocaleString() : "Not checked yet";
}

function formatVersion(value: string | null | undefined) {
  return value?.trim() ? value : "Not clear yet";
}

function watchingEmptyMessage(filter: WatchingFilter, userView: UserView) {
  switch (filter) {
    case "checked_recently":
      return userView === "beginner"
        ? "Nothing watched has a clean recent check yet."
        : "No watched items currently show a clean recent check.";
    case "check_available":
      return userView === "beginner"
        ? "Nothing watched is waiting for its first check right now."
        : "No watched items are sitting in a ready-to-check state.";
    default:
      return userView === "beginner"
        ? "No files are in the watched lane yet."
        : "No watched files yet.";
  }
}

function reminderOnlyEmptyMessage(filter: ReminderFilter, userView: UserView) {
  if (filter === "creator_page") {
    return userView === "beginner"
      ? "No creator-page reminders are saved right now."
      : "No creator-page reminders match this filter.";
  }

  if (filter === "exact_page") {
    return userView === "beginner"
      ? "No exact-page reminders are saved right now."
      : "No exact-page reminders match this filter.";
  }

  return userView === "beginner"
    ? "No reminder-only pages are saved right now."
    : "No reminder-only items match this filter.";
}

function sourceBehaviorSummary(watchResult: WatchResult | null, userView: UserView) {
  if (!watchResult?.sourceKind) {
    return userView === "beginner" ? "No source is saved yet." : "No source is saved yet.";
  }

  if (watchResult.capability === "can_refresh_now") {
    return userView === "beginner"
      ? "SimSuite can check this source automatically when you ask."
      : "This source supports explicit checks.";
  }

  if (watchResult.capability === "provider_required") {
    return userView === "beginner"
      ? "This source is saved, but SimSuite cannot check it automatically yet."
      : "This source is saved, but provider support is still required.";
  }

  return userView === "beginner"
    ? "This source is saved for manual checks and reminders."
    : "This source is reminder-only today.";
}

function deriveSourceClues(detail: FileDetail | null) {
  if (!detail) {
    return [] as string[];
  }

  const clues: string[] = [];
  if (detail.creator?.trim()) clues.push("Creator clue");
  if (detail.installedVersionSummary?.version?.trim()) clues.push("Version clue");
  if (detail.insights.scriptNamespaces.length) clues.push("Script clue");
  if (detail.insights.familyHints.length) clues.push("Family clue");
  if (detail.insights.embeddedNames.length) clues.push("Name clue");
  return clues.slice(0, 4);
}

function sourceSuggestionStrength(detail: FileDetail | null, row: SourceNeededRow | null) {
  if (!row || row.suggestionState === "no_match") {
    return "No strong match yet";
  }

  const clueCount = deriveSourceClues(detail).length;
  if (row.suggestedSourceKind === "exact_page" && clueCount >= 2) {
    return "Strong match";
  }

  return "Possible match";
}

function sourceConfidenceLabel(detail: FileDetail | null, row: SourceNeededRow | null) {
  if (row) {
    return sourceSuggestionStrength(detail, row);
  }

  const watchResult = detail?.watchResult;
  if (!watchResult?.sourceKind) {
    return "No source saved";
  }

  if (watchResult.sourceOrigin === "built_in_special") {
    return "Built-in supported source";
  }

  if (watchResult.capability === "can_refresh_now") {
    return "Saved page with live checks";
  }

  if (watchResult.capability === "provider_required") {
    return "Saved page, provider-limited";
  }

  return "Saved page, manual review only";
}

function updatesRouteForDetail(detail: FileDetail): {
  mode: UpdateMode;
} {
  const watchResult = detail.watchResult;

  if (!watchResult?.sourceKind) {
    return { mode: "setup" };
  }

  if (watchResult.capability === "saved_reference_only") {
    return { mode: "reminders" };
  }

  if (watchResult.capability === "provider_required") {
    return { mode: "attention" };
  }

  switch (watchResult.status) {
    case "exact_update_available":
    case "possible_update":
    case "unknown":
      return { mode: "attention" };
    case "current":
    case "not_watched":
      return { mode: "watching" };
    default:
      return { mode: "attention" };
  }
}

function isRefreshableWatch(item: LibraryWatchListItem) {
  return item.watchResult.canRefreshNow;
}

function isWatchingLaneItem(item: LibraryWatchListItem) {
  return (
    isRefreshableWatch(item) &&
    (item.watchResult.status === "current" || item.watchResult.status === "not_watched")
  );
}

function isAttentionTrackedItem(item: LibraryWatchListItem) {
  return (
    isRefreshableWatch(item) &&
    (item.watchResult.status === "exact_update_available" ||
      item.watchResult.status === "possible_update" ||
      item.watchResult.status === "unknown")
  );
}

function matchesWatchingFilter(item: LibraryWatchListItem, filter: WatchingFilter) {
  if (filter === "checked_recently") {
    return item.watchResult.status === "current";
  }

  if (filter === "check_available") {
    return item.watchResult.status === "not_watched";
  }

  return true;
}

function matchesAttentionFilter(row: AttentionRow, filter: AttentionFilter) {
  if (filter === "all") {
    return true;
  }

  if (row.kind === "review") {
    return filter === "provider_required" && row.item.reviewReason === "provider_needed";
  }

  if (filter === "update_found") {
    return row.item.watchResult.status === "exact_update_available";
  }

  if (filter === "possible_update") {
    return row.item.watchResult.status === "possible_update";
  }

  if (filter === "could_not_check") {
    return row.item.watchResult.status === "unknown";
  }

  return false;
}

function matchesReminderFilter(item: LibraryWatchReviewItem, filter: ReminderFilter) {
  if (filter === "all") {
    return true;
  }

  return item.watchResult.sourceKind === filter;
}

function attentionStatusLabel(row: AttentionRow, userView: UserView) {
  if (row.kind === "review") {
    return reviewReasonLabel(row.item.reviewReason, userView);
  }

  return watchStatusLabel(row.item.watchResult.status, userView);
}

function attentionHint(row: AttentionRow) {
  if (row.kind === "review") {
    return row.item.reviewHint;
  }

  return row.item.watchResult.note ?? "This watched source needs a closer manual look.";
}

function updatesRowFileId(row: LibraryWatchListItem | SourceNeededRow | AttentionRow | LibraryWatchReviewItem) {
  return "kind" in row ? row.item.fileId : row.fileId;
}

function sourceNeededStatusLabel(row: SourceNeededRow) {
  return row.suggestionState === "possible_source" ? "Possible source found" : "No match yet";
}

function sourceNeededEmptyMessage(userView: UserView) {
  return userView === "beginner"
    ? "No files currently need a source."
    : "No files currently need source setup.";
}


export function UpdatesScreen({
  refreshVersion,
  onNavigate,
  onDataChanged,
  userView,
  initialMode,
  initialFilter,
  initialFileId,
}: UpdatesScreenProps) {
  const {
    updatesFiltersCollapsed,
    setUpdatesFiltersCollapsed,
  } = useUiPreferences();
  const [mode, setMode] = useState<UpdateMode>(normalizeUpdateMode(initialMode) ?? "watching");
  const [watchingFilter, setWatchingFilter] = useState<WatchingFilter>("all");
  const [attentionFilter, setAttentionFilter] = useState<AttentionFilter>("all");
  const [reminderFilter, setReminderFilter] = useState<ReminderFilter>("all");
  const [setupFilter, setSetupFilter] = useState<SetupFilter>("all");
  const [trackedList, setTrackedList] = useState<LibraryWatchListResponse | null>(null);
  const [setupList, setSetupList] = useState<LibraryWatchSetupResponse | null>(null);
  const [reviewList, setReviewList] = useState<LibraryWatchReviewResponse | null>(null);
  const [sourceNeededRows, setSourceNeededRows] = useState<SourceNeededRow[]>([]);
  const [selectedItem, setSelectedItem] = useState<FileDetail | null>(null);
  const [loadingTracked, setLoadingTracked] = useState(false);
  const [loadingSetup, setLoadingSetup] = useState(false);
  const [loadingReview, setLoadingReview] = useState(false);
  const [focusedFileId, setFocusedFileId] = useState<number | null>(initialFileId ?? null);
  const [refreshingAll, setRefreshingAll] = useState(false);
  const [watchEditing, setWatchEditing] = useState(false);
  const [watchSourceKind, setWatchSourceKind] = useState<WatchSourceKind>("exact_page");
  const [watchSourceLabel, setWatchSourceLabel] = useState("");
  const [watchSourceUrl, setWatchSourceUrl] = useState("");
  const [savingWatch, setSavingWatch] = useState(false);
  const [clearingWatch, setClearingWatch] = useState(false);
  const [refreshingWatch, setRefreshingWatch] = useState(false);
  const [message, setMessage] = useState<string | null>(null);

  useEffect(() => {
    const nextMode = normalizeUpdateMode(initialMode);
    if (nextMode) {
      setMode(nextMode);
      setWatchEditing(nextMode === "setup");
    }
  }, [initialMode]);

  useEffect(() => {
    if (!initialFilter) {
      return;
    }

    if (initialFilter === "exact_updates") {
      setMode("attention");
      setAttentionFilter("update_found");
      return;
    }

    if (initialFilter === "possible_updates") {
      setMode("attention");
      setAttentionFilter("possible_update");
      return;
    }

    if (initialFilter === "unclear") {
      setMode("attention");
      setAttentionFilter("could_not_check");
      return;
    }

    if (initialFilter === "all") {
      setMode("watching");
      setWatchingFilter("all");
    }
  }, [initialFilter]);

  useEffect(() => {
    setFocusedFileId(initialFileId ?? null);
  }, [initialFileId]);

  useEffect(() => {
    void refreshLists();
  }, [refreshVersion]);

  useEffect(() => {
    if (!initialFileId) {
      return;
    }

    void loadSelectedFile(initialFileId, { edit: normalizeUpdateMode(initialMode) === "setup" });
  }, [initialFileId, initialMode]);

  async function loadTrackedList(skipLoading = false) {
    if (!skipLoading) {
      setLoadingTracked(true);
    }

    try {
      const result = await api.listLibraryWatchItems("all", 48);
      setTrackedList(result);
    } catch (error) {
      console.error("Could not load watched update items.", error);
    } finally {
      if (!skipLoading) {
        setLoadingTracked(false);
      }
    }
  }

  async function loadSetupList(skipLoading = false) {
    if (!skipLoading) {
      setLoadingSetup(true);
    }

    try {
      const [result, libraryResult] = await Promise.all([
        api.listLibraryWatchSetupItems(),
        api.listLibraryFiles({
          watchFilter: "not_tracked",
          limit: SOURCE_NEEDED_LIMIT,
          includePreviews: false,
        }),
      ]);
      setSetupList(result);

      const suggestionMap = new Map(result.items.map((item) => [item.fileId, item]));
      const merged = libraryResult.items
        .filter(
          (row) =>
            row.sourceLocation !== "tray" &&
            ["package", "ts4script"].includes(row.extension.replace(/^\./, "").toLowerCase()),
        )
        .map((row) => {
          const suggestion = suggestionMap.get(row.id);
          return {
            fileId: row.id,
            filename: row.filename,
            creator: row.creator,
            installedVersion: row.installedVersion ?? null,
            subjectLabel: suggestion?.subjectLabel ?? row.filename.replace(/\.[^.]+$/, ""),
            suggestedSourceKind: suggestion?.suggestedSourceKind ?? null,
            setupHint:
              suggestion?.setupHint ??
              "SimSuite does not have a strong source suggestion yet. Add a manual page or keep this as a reminder.",
            suggestionState: suggestion ? "possible_source" : "no_match",
          } satisfies SourceNeededRow;
        });
      setSourceNeededRows(merged);
    } catch (error) {
      console.error("Could not load setup items.", error);
    } finally {
      if (!skipLoading) {
        setLoadingSetup(false);
      }
    }
  }

  async function loadReviewList(skipLoading = false) {
    if (!skipLoading) {
      setLoadingReview(true);
    }

    try {
      const result = await api.listLibraryWatchReviewItems(48);
      setReviewList(result);
    } catch (error) {
      console.error("Could not load review items.", error);
    } finally {
      if (!skipLoading) {
        setLoadingReview(false);
      }
    }
  }

  async function refreshLists() {
    await Promise.all([loadTrackedList(true), loadSetupList(true), loadReviewList(true)]);
  }

  function syncWatchFields(
    detail: FileDetail | null,
    draft?: {
      sourceKind?: WatchSourceKind;
      sourceLabel?: string;
      sourceUrl?: string;
    },
  ) {
    setWatchSourceKind(draft?.sourceKind ?? detail?.watchResult?.sourceKind ?? "exact_page");
    setWatchSourceLabel(draft?.sourceLabel ?? detail?.watchResult?.sourceLabel ?? "");
    setWatchSourceUrl(draft?.sourceUrl ?? detail?.watchResult?.sourceUrl ?? "");
  }

  async function loadSelectedFile(
    fileId: number,
    options?: {
      edit?: boolean;
      sourceKind?: WatchSourceKind;
      sourceLabel?: string;
      sourceUrl?: string;
    },
  ) {
    const detail = await api.getFileDetail(fileId);
    setSelectedItem(detail);
    syncWatchFields(detail, options);
    setWatchEditing(Boolean(options?.edit));
  }

  async function handleSelectTrackedItem(item: LibraryWatchListItem) {
    setFocusedFileId(null);
    await loadSelectedFile(item.fileId);
  }

  async function handleSelectSetupItem(item: SourceNeededRow) {
    setFocusedFileId(null);
    await loadSelectedFile(item.fileId, {
      edit: true,
      sourceKind: item.suggestedSourceKind ?? "exact_page",
      sourceLabel:
        item.suggestedSourceKind === "creator_page"
          ? item.creator ?? item.subjectLabel
          : item.suggestedSourceKind
            ? item.subjectLabel
            : item.creator ?? item.subjectLabel,
      sourceUrl: "",
    });
  }

  async function handleSelectReviewItem(item: LibraryWatchReviewItem) {
    setFocusedFileId(null);
    await loadSelectedFile(item.fileId);
  }

  async function handleSelectAttentionRow(row: AttentionRow) {
    if (row.kind === "review") {
      await handleSelectReviewItem(row.item);
      return;
    }

    await handleSelectTrackedItem(row.item);
  }

  function closeWatchEditor() {
    syncWatchFields(selectedItem);
    setWatchEditing(false);
  }

  async function handleSaveWatchSource() {
    if (!selectedItem) {
      return;
    }

    setSavingWatch(true);
    setMessage(null);

    try {
      const saved = await api.saveWatchSourceForFile(
        selectedItem.id,
        watchSourceKind,
        watchSourceLabel.trim() || undefined,
        watchSourceUrl.trim(),
      );

      if (!saved) {
        setMessage("Could not save a source for this item.");
        return;
      }

      const detail = await api.getFileDetail(selectedItem.id);
      if (!detail) {
        setMessage("Could not reload this item after saving.");
        return;
      }

      const nextRoute = updatesRouteForDetail(detail);
      setSelectedItem(detail);
      syncWatchFields(detail);
      setWatchEditing(false);
      setMode(nextRoute.mode);
      await refreshLists();
      onDataChanged?.();
      setMessage("Source saved.");
    } catch (error) {
      setMessage(error instanceof Error ? error.message : "Could not save this source.");
    } finally {
      setSavingWatch(false);
    }
  }

  async function handleClearWatchSource() {
    if (!selectedItem) {
      return;
    }

    const confirmed = globalThis.confirm(
      userView === "beginner"
        ? `Stop tracking "${selectedItem.filename}" from this source? SimSuite will no longer watch this source for updates to this file.`
        : `Clear the tracked source for "${selectedItem.filename}"? SimSuite will stop watching this source path for updates.`,
    );
    if (!confirmed) return;

    setClearingWatch(true);
    setMessage(null);

    try {
      const detail = await api.clearWatchSourceForFile(selectedItem.id);
      const nextRoute = detail ? updatesRouteForDetail(detail) : { mode: "setup" as const };
      setSelectedItem(detail);
      syncWatchFields(detail);
      setWatchEditing(false);
      setMode(nextRoute.mode);
      await refreshLists();
      onDataChanged?.();
      setMessage("Source cleared.");
    } catch (error) {
      setMessage(error instanceof Error ? error.message : "Could not clear this source.");
    } finally {
      setClearingWatch(false);
    }
  }

  async function handleRefreshWatchSource() {
    if (!selectedItem) {
      return;
    }

    setRefreshingWatch(true);
    setMessage(null);

    try {
      const detail = await api.refreshWatchSourceForFile(selectedItem.id);

      if (!detail) {
        setMessage("Could not refresh this source.");
        return;
      }

      const nextRoute = updatesRouteForDetail(detail);
      setSelectedItem(detail);
      syncWatchFields(detail);
      setMode(nextRoute.mode);
      await refreshLists();
      onDataChanged?.();
      setMessage("Checked the selected source.");
    } catch (error) {
      setMessage(error instanceof Error ? error.message : "Could not refresh this source.");
    } finally {
      setRefreshingWatch(false);
    }
  }

  async function handleRefreshAll() {
    setRefreshingAll(true);
    setMessage(null);

    try {
      const summary = await api.refreshWatchedSources();
      await refreshLists();
      onDataChanged?.();
      const checkedLabel =
        summary.checkedSubjects === 1
          ? "Checked 1 watched source."
          : `Checked ${summary.checkedSubjects} watched sources.`;
      const updateLabel =
        summary.exactUpdateItems === 1
          ? "1 update found."
          : `${summary.exactUpdateItems} updates found.`;
      setMessage(`${checkedLabel} ${updateLabel}`);
    } catch (error) {
      setMessage(error instanceof Error ? error.message : "Could not refresh tracked pages.");
    } finally {
      setRefreshingAll(false);
    }
  }

  const trackedItems = trackedList?.items ?? [];
  const reviewItems = reviewList?.items ?? [];
  const filteredSetupRows = sourceNeededRows.filter((item) => {
    if (setupFilter === "all") {
      return true;
    }
    return item.suggestionState === setupFilter;
  });
  const watchingItems = trackedItems.filter(isWatchingLaneItem);
  const filteredWatchingItems = watchingItems.filter((item) =>
    matchesWatchingFilter(item, watchingFilter),
  );
  const attentionTrackedItems = trackedItems.filter(isAttentionTrackedItem);
  const attentionSeenFileIds = new Set(attentionTrackedItems.map((item) => item.fileId));
  const providerNeededItems = reviewItems.filter((item) => item.reviewReason === "provider_needed");
  const reminderOnlyItems = reviewItems.filter((item) => item.reviewReason === "reference_only");
  const attentionRows: AttentionRow[] = [
    ...attentionTrackedItems.map((item) => ({ kind: "tracked" as const, item })),
    ...providerNeededItems
      .filter((item) => !attentionSeenFileIds.has(item.fileId))
      .map((item) => ({ kind: "review" as const, item })),
  ];
  const filteredAttentionRows = attentionRows.filter((row) =>
    matchesAttentionFilter(row, attentionFilter),
  );
  const filteredReminderItems = reminderOnlyItems.filter((item) =>
    matchesReminderFilter(item, reminderFilter),
  );
  const trackedTotal = trackedList?.total ?? 0;
  const setupTotal = sourceNeededRows.length;
  const attentionTotal = attentionRows.length;
  const reminderTotal = reminderOnlyItems.length;
  const currentModeCount =
    mode === "watching"
      ? filteredWatchingItems.length
      : mode === "setup"
        ? filteredSetupRows.length
        : mode === "attention"
          ? filteredAttentionRows.length
          : filteredReminderItems.length;
  const visibleRows =
    mode === "watching"
      ? filteredWatchingItems
      : mode === "setup"
        ? filteredSetupRows
        : mode === "attention"
          ? filteredAttentionRows
          : filteredReminderItems;
  const canEditSelectedSource =
    selectedItem?.watchResult?.sourceOrigin !== "built_in_special";
  const canClearSelectedSource =
    selectedItem?.watchResult?.sourceOrigin === "saved_by_user";
  const canRefreshSelectedSource = Boolean(
    selectedItem?.watchResult?.sourceUrl && selectedItem.watchResult.canRefreshNow,
  );
  const currentFilterLabel =
    mode === "watching"
      ? WATCHING_FILTERS.find((item) => item.id === watchingFilter)?.label ?? "All watched"
      : mode === "setup"
        ? SETUP_FILTERS.find((item) => item.id === setupFilter)?.label ?? "Needs source"
        : mode === "attention"
          ? ATTENTION_FILTERS.find((item) => item.id === attentionFilter)?.label ?? "Attention"
          : REMINDER_FILTERS.find((item) => item.id === reminderFilter)?.label ?? "Reminder only";
  const modeCopy: Record<UpdateMode, { title: string; body: string }> = {
    watching: {
      title: "Watched",
      body:
        "These files already have a checkable source. Keep the calm stuff here, and push real follow-up into Attention needed.",
    },
    setup: {
      title: "Needs source",
      body:
        "These files are not being watched yet. Save one source, or leave a reminder if SimSuite cannot check the page safely.",
    },
    attention: {
      title: "Attention needed",
      body:
        "This lane holds update leads, unclear checks, and provider-blocked pages so they do not get buried inside normal watched items.",
    },
    reminders: {
      title: "Reminder only",
      body:
        "These pages are saved for manual follow-up. SimSuite keeps them separate so bookmarks do not pretend to be live update checks.",
    },
  };
  const watchingCheckedCount = watchingItems.filter((item) => item.watchResult.status === "current").length;
  const watchingCheckAvailableCount = watchingItems.filter((item) => item.watchResult.status === "not_watched").length;
  const attentionUpdateFoundCount = attentionTrackedItems.filter(
    (item) => item.watchResult.status === "exact_update_available",
  ).length;
  const attentionPossibleCount = attentionTrackedItems.filter(
    (item) => item.watchResult.status === "possible_update",
  ).length;
  const attentionCouldNotCheckCount = attentionTrackedItems.filter(
    (item) => item.watchResult.status === "unknown",
  ).length;
  const setupPossibleSourceCount = sourceNeededRows.filter(
    (item) => item.suggestionState === "possible_source",
  ).length;
  const setupNoMatchCount = Math.max(0, setupTotal - setupPossibleSourceCount);
  const reminderCreatorCount = reminderOnlyItems.filter(
    (item) => item.watchResult.sourceKind === "creator_page",
  ).length;
  const reminderExactCount = reminderOnlyItems.filter(
    (item) => item.watchResult.sourceKind === "exact_page",
  ).length;
  const selectedSetupRow =
    mode === "setup" && selectedItem
      ? sourceNeededRows.find((item) => item.fileId === selectedItem.id) ?? null
      : null;
  const stageSummaryCards =
    mode === "watching"
      ? [
          {
            label: "No new update found",
            value: watchingCheckedCount,
            tone: "good" as const,
            note:
              userView === "beginner"
                ? "These watched files already have a recent clean result."
                : "These watched files currently look steady after a real check.",
          },
          {
            label: "Check available",
            value: watchingCheckAvailableCount,
            tone: "muted" as const,
            note:
              userView === "beginner"
                ? "These files are watched, but still waiting for a manual check."
                : "These sources are ready, but have not produced a fresh result yet.",
          },
          {
            label: "Watched total",
            value: watchingItems.length,
            tone: "low" as const,
            note:
              userView === "beginner"
                ? "This is the clean watch lane only."
                : "Refreshable, low-drama watch sources stay here.",
          },
        ]
      : mode === "setup"
        ? [
            {
              label: "Possible source found",
              value: setupPossibleSourceCount,
              tone: "good" as const,
              note:
                userView === "beginner"
                  ? "SimSuite has enough clues to suggest a next source."
                  : "These items have a clue-backed source suggestion.",
            },
            {
              label: "No match yet",
              value: setupNoMatchCount,
              tone: "muted" as const,
              note:
                userView === "beginner"
                  ? "These still need a manual page or later follow-up."
                  : "These items still need a manual source decision.",
            },
            {
              label: "Needs a source",
              value: setupTotal,
              tone: "warn" as const,
              note:
                userView === "beginner"
                  ? "Everything here still needs one saved source."
                  : "These items stay unwatched until a source is saved.",
            },
          ]
        : mode === "attention"
          ? [
              {
                label: "Update found",
                value: attentionUpdateFoundCount,
                tone: "good" as const,
                note:
                  userView === "beginner"
                    ? "These watched files have a clearer newer version waiting."
                    : "These watched files have a strong update lead.",
              },
              {
                label: "Possible update",
                value: attentionPossibleCount,
                tone: "warn" as const,
                note:
                  userView === "beginner"
                    ? "These changed, but still need a manual look."
                    : "The page changed, but the version clue is still cautious.",
              },
              {
                label: "Could not check",
                value: attentionCouldNotCheckCount,
                tone: "muted" as const,
                note:
                  userView === "beginner"
                    ? "These checks came back too unclear to trust yet."
                    : "These refreshable checks still need a safer result.",
              },
              {
                label: "Provider required",
                value: providerNeededItems.length,
                tone: "low" as const,
                note:
                  userView === "beginner"
                    ? "These saved pages still need provider support before SimSuite can check them."
                    : "These exact pages are blocked on an approved provider path.",
              },
            ]
          : [
              {
                label: "Creator pages",
                value: reminderCreatorCount,
                tone: "muted" as const,
                note:
                  userView === "beginner"
                    ? "These stay as reminders because one page can cover many files."
                    : "These reminders point at broader creator feeds instead of one-to-one releases.",
              },
              {
                label: "Saved exact pages",
                value: reminderExactCount,
                tone: "low" as const,
                note:
                  userView === "beginner"
                    ? "These exact links are still bookmark-only today."
                    : "These exact pages are saved, but still not safe to treat as live checks.",
              },
              {
                label: "Reminder-only total",
                value: reminderOnlyItems.length,
                tone: "warn" as const,
                note:
                  userView === "beginner"
                    ? "These are reminders, not automatic checks."
                    : "Reminder-only pages stay separate on purpose.",
              },
            ];
  const stageDetailRows = selectedItem
    ? [
        {
          label: "Status",
          value: selectedItem.watchResult
            ? selectedItem.watchResult.capability === "saved_reference_only"
              ? "Reminder only"
              : selectedItem.watchResult.capability === "provider_required"
                ? "Provider required"
                : watchStatusLabel(selectedItem.watchResult.status, userView)
            : "No source saved",
        },
        {
          label: mode === "setup" ? "Suggested source" : mode === "reminders" ? "Saved page" : "Watching",
          value:
            mode === "setup"
              ? selectedSetupRow?.suggestedSourceKind
                ? watchSourceKindLabel(selectedSetupRow.suggestedSourceKind)
                : "No strong match yet"
              : selectedItem.watchResult?.sourceLabel ?? "No source saved",
        },
        {
          label: "Installed",
          value: formatVersion(selectedItem.installedVersionSummary?.version),
        },
      ]
    : [];

  useEffect(() => {
    if (!watchEditing) {
      return;
    }

    function handleKeyDown(event: KeyboardEvent) {
      if (event.key !== "Escape") {
        return;
      }

      event.preventDefault();
      closeWatchEditor();
    }

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [watchEditing, selectedItem]);

  useEffect(() => {
    if (!visibleRows.length) {
      if (!focusedFileId) {
        setSelectedItem(null);
        setWatchEditing(false);
      }
      return;
    }

    if (
      selectedItem &&
      (focusedFileId === selectedItem.id ||
        visibleRows.some((item) => updatesRowFileId(item) === selectedItem.id))
    ) {
      return;
    }

    if (mode === "watching") {
      void handleSelectTrackedItem(visibleRows[0] as LibraryWatchListItem);
      return;
    }

    if (mode === "setup") {
      void handleSelectSetupItem(visibleRows[0] as SourceNeededRow);
      return;
    }

    if (mode === "attention") {
      void handleSelectAttentionRow(visibleRows[0] as AttentionRow);
      return;
    }

    void handleSelectReviewItem(visibleRows[0] as LibraryWatchReviewItem);
  }, [focusedFileId, mode, selectedItem, visibleRows]);

  const selectionOutsideCurrentList = Boolean(
    selectedItem && !visibleRows.some((item) => updatesRowFileId(item) === selectedItem.id),
  );

  return (
    <Workbench threePanel fullHeight className="updates-workbench">
      <WorkbenchRail
        ariaLabel="Updates controls"
        className={`updates-rail-shell ${updatesFiltersCollapsed ? "is-collapsed" : ""}`}
        noBorder
        noPadding={updatesFiltersCollapsed}
        hideHandle
      >
        {!updatesFiltersCollapsed ? (
          <div className="updates-rail">
            <div className="workbench-header">
              <div>
                <p className="eyebrow">Workspace</p>
                <h1 className="updates-rail-title">Updates</h1>
                <p className="updates-rail-copy">
                  {screenHelperLine("updates", userView)}
                </p>
              </div>
              <button
                type="button"
                className="workspace-toggle"
                onClick={() => setUpdatesFiltersCollapsed(true)}
                aria-label="Hide updates controls"
              >
                <PanelLeftClose size={14} strokeWidth={2} />
              </button>
            </div>

            <div className="updates-rail-section">
              <div className="section-label">Modes</div>
              <div className="updates-mode-list">
                <button
                  type="button"
                  className={`action-item ${mode === "setup" ? "is-active" : ""}`}
                  onClick={() => setMode("setup")}
                >
                  <ScanSearch size={14} strokeWidth={2} className="action-item-icon" />
                  <span className="action-item-label">Needs source</span>
                  <span className="action-item-badge">{setupTotal}</span>
                </button>
                <button
                  type="button"
                  className={`action-item ${mode === "watching" ? "is-active" : ""}`}
                  onClick={() => setMode("watching")}
                >
                  <Eye size={14} strokeWidth={2} className="action-item-icon" />
                  <span className="action-item-label">Watched</span>
                  <span className="action-item-badge">{watchingItems.length}</span>
                </button>
                <button
                  type="button"
                  className={`action-item ${mode === "attention" ? "is-active" : ""}`}
                  onClick={() => setMode("attention")}
                >
                  <ListChecks size={14} strokeWidth={2} className="action-item-icon" />
                  <span className="action-item-label">Attention needed</span>
                  <span className="action-item-badge">{attentionTotal}</span>
                </button>
                <button
                  type="button"
                  className={`action-item ${mode === "reminders" ? "is-active" : ""}`}
                  onClick={() => setMode("reminders")}
                >
                  <Bookmark size={14} strokeWidth={2} className="action-item-icon" />
                  <span className="action-item-label">Reminder only</span>
                  <span className="action-item-badge">{reminderTotal}</span>
                </button>
              </div>
            </div>

            <div className="updates-rail-section">
              <div className="section-label">
                {mode === "setup"
                  ? "Source queue"
                  : mode === "watching"
                    ? "Watch filter"
                    : mode === "attention"
                      ? "Attention filter"
                      : "Reminder filter"}
              </div>
              <div className="updates-filter-list">
                {(mode === "setup"
                  ? SETUP_FILTERS
                  : mode === "watching"
                    ? WATCHING_FILTERS
                    : mode === "attention"
                      ? ATTENTION_FILTERS
                      : REMINDER_FILTERS
                ).map((item) => (
                  <button
                    key={item.id}
                    type="button"
                    className={`workspace-toggle ${
                      (mode === "setup"
                        ? setupFilter
                        : mode === "watching"
                          ? watchingFilter
                          : mode === "attention"
                            ? attentionFilter
                            : reminderFilter) === item.id
                        ? "is-active"
                        : ""
                    }`}
                    onClick={() => {
                      if (mode === "setup") {
                        setSetupFilter(item.id as SetupFilter);
                      } else if (mode === "watching") {
                        setWatchingFilter(item.id as WatchingFilter);
                      } else if (mode === "attention") {
                        setAttentionFilter(item.id as AttentionFilter);
                      } else {
                        setReminderFilter(item.id as ReminderFilter);
                      }
                    }}
                  >
                    {userView === "beginner" ? item.beginnerLabel : item.label}
                  </button>
                ))}
              </div>
            </div>

            <div className="updates-rail-section">
              <div className="section-label">Quick actions</div>
              <div className="updates-rail-actions">
                <button
                  type="button"
                  className="primary-action"
                  onClick={() => void handleRefreshAll()}
                  disabled={refreshingAll}
                >
                  <RefreshCw
                    size={14}
                    strokeWidth={2}
                    className={refreshingAll ? "spin" : undefined}
                  />
                  {refreshingAll ? "Checking..." : "Check watched now"}
                </button>
                <button
                  type="button"
                  className="secondary-action"
                  onClick={() => onNavigate("library")}
                >
                  <LibraryBig size={14} strokeWidth={2} />
                  Browse library
                </button>
              </div>
            </div>
          </div>
        ) : null}
      </WorkbenchRail>

      <WorkbenchStage className="updates-stage">
        <div className="table-header updates-stage-header">
          <div className="updates-stage-copy">
            <div>
              <p className="eyebrow">Workspace</p>
              <h2>{modeCopy[mode].title}</h2>
              <p className="text-muted">{modeCopy[mode].body}</p>
            </div>
            <div className="health-chip-group updates-stage-metrics">
              <span className={`health-chip ${mode === "setup" ? "is-warn" : ""}`}>
                {setupTotal} need source
              </span>
              <span className={`health-chip ${mode === "watching" ? "is-good" : ""}`}>
                {watchingItems.length} watched
              </span>
              <span className={`health-chip ${mode === "attention" ? "is-warn" : ""}`}>
                {attentionTotal} attention
              </span>
              <span className={`health-chip ${mode === "reminders" ? "is-low" : ""}`}>
                {reminderTotal} reminders
              </span>
            </div>
          </div>
          <div className="workspace-toggles">
            <button
              type="button"
              className="workspace-toggle"
              onClick={() => setUpdatesFiltersCollapsed(!updatesFiltersCollapsed)}
            >
              {updatesFiltersCollapsed ? (
                <PanelLeftOpen size={14} strokeWidth={2} />
              ) : (
                <PanelLeftClose size={14} strokeWidth={2} />
              )}
              {updatesFiltersCollapsed ? "Show controls" : "Hide controls"}
            </button>
          </div>
        </div>

        <div className="updates-stage-toolbar">
          <span className="ghost-chip">{currentModeCount.toLocaleString()} in view</span>
          <span className="ghost-chip">{currentFilterLabel}</span>
          <span className="ghost-chip">
            {selectedItem ? `Selected: ${selectedItem.filename}` : "Nothing selected"}
          </span>
        </div>

        {message ? <div className="updates-inline-message">{message}</div> : null}

        {selectionOutsideCurrentList && selectedItem ? (
          <div className="updates-inline-message">
            Focused on <strong>{selectedItem.filename}</strong>. It is not in the current list, but you can still review or save a source below.
          </div>
        ) : null}

        <div className="updates-stage-body">
          <div className="workbench-panel updates-stage-focus-band">
            <div className="updates-stage-focus">
              <p className="eyebrow">
                {mode === "setup"
                  ? userView === "beginner"
                    ? "Next source"
                    : "Source focus"
                  : selectedItem
                    ? userView === "beginner"
                      ? "Selected file"
                      : "Selection focus"
                    : mode === "attention"
                      ? "Attention focus"
                      : mode === "reminders"
                        ? "Reminder focus"
                        : "Queue focus"}
              </p>
              <h3>{selectedItem ? selectedItem.filename : modeCopy[mode].title}</h3>
              <p className="updates-stage-note">
                {selectedItem
                  ? describeUpdatesStageFocus(selectedItem, mode, userView)
                  : mode === "setup"
                    ? "Save one good page here, and SimSuite will reuse it the next time this file is checked."
                    : mode === "attention"
                      ? "Keep live update leads, provider blocks, and unclear checks here so watched results stay honest."
                      : mode === "reminders"
                        ? "Reminder-only pages stay visible here without pretending SimSuite can check them automatically."
                        : "Watched sources stay together here so calm checks and ready-to-check items are easy to compare."}
              </p>
            </div>

            {stageDetailRows.length ? (
              <div className="detail-list updates-stage-detail-list">
                {stageDetailRows.map((row) => (
                  <DetailRow key={row.label} label={row.label} value={row.value} />
                ))}
              </div>
            ) : null}
          </div>

          <div className="table-scroll workbench-panel updates-table-scroll">
            {mode === "watching" ? (
              <UpdatesWatchingTable
                items={filteredWatchingItems}
                loading={loadingTracked}
                selectedId={selectedItem?.id ?? null}
                onSelect={handleSelectTrackedItem}
                userView={userView}
                filter={watchingFilter}
              />
            ) : null}

            {mode === "setup" ? (
              <UpdatesSetupTable
                items={filteredSetupRows}
                loading={loadingSetup}
                selectedId={selectedItem?.id ?? null}
                onSelect={handleSelectSetupItem}
                userView={userView}
              />
            ) : null}

            {mode === "attention" ? (
              <UpdatesAttentionTable
                items={filteredAttentionRows}
                loading={loadingTracked || loadingReview}
                selectedId={selectedItem?.id ?? null}
                onSelect={handleSelectAttentionRow}
                userView={userView}
                filter={attentionFilter}
              />
            ) : null}

            {mode === "reminders" ? (
              <UpdatesReminderTable
                items={filteredReminderItems}
                loading={loadingReview}
                selectedId={selectedItem?.id ?? null}
                onSelect={handleSelectReviewItem}
                userView={userView}
                filter={reminderFilter}
              />
            ) : null}
          </div>

          <div className="updates-stage-footer">
            <div className="updates-stage-summary-grid">
              {stageSummaryCards.map((card) => (
                <UpdatesStageStatCard
                  key={card.label}
                  label={card.label}
                  value={card.value}
                  tone={card.tone}
                  note={card.note}
                />
              ))}
            </div>

            <div className="workbench-panel updates-stage-guidance-card">
              <div className="updates-stage-guidance">
                <strong>{updatesGuidanceTitle(mode, userView)}</strong>
                <p>{updatesGuidanceBody(mode, userView)}</p>
              </div>
            </div>
          </div>
        </div>
      </WorkbenchStage>

      <WorkbenchInspector
        ariaLabel="Update details"
        className="updates-inspector-shell"
      >
        {selectedItem ? (
          <div className="updates-inspector">
            <div className="detail-header">
              <div>
                <p className="eyebrow">{userView === "beginner" ? "Selected item" : "Inspector"}</p>
                <h2>{selectedItem.filename}</h2>
              </div>
              {selectedItem.watchResult ? (
                <span className="ghost-chip">
                  {selectedItem.watchResult.capability === "saved_reference_only"
                    ? "Reminder only"
                    : selectedItem.watchResult.capability === "provider_required"
                      ? "Provider required"
                      : watchStatusLabel(selectedItem.watchResult.status, userView)}
                </span>
              ) : null}
            </div>

            <div className="detail-block">
              <div className="section-label">At a glance</div>
              <div className="detail-list">
                <DetailRow
                  label="Creator"
                  value={selectedItem.creator ?? unknownCreatorLabel(userView)}
                />
                <DetailRow label="Type" value={friendlyTypeLabel(selectedItem.kind)} />
                <DetailRow
                  label="Installed version"
                  value={formatVersion(selectedItem.installedVersionSummary?.version)}
                />
                <DetailRow
                  label="Latest helper version"
                  value={formatVersion(selectedItem.watchResult?.latestVersion)}
                />
              </div>
            </div>

            {selectedItem.watchResult ? (
              <div className="detail-block">
                <div className="section-label">Tracking</div>
                <div className="updates-status-card">
                  <div className="updates-status-row">
                    {watchStatusIcon(selectedItem.watchResult.status)}
                    <div>
                      <strong>
                        {watchStatusLabel(selectedItem.watchResult.status, userView)}
                      </strong>
                      <p className="text-muted">
                        {watchCapabilityLabel(selectedItem.watchResult, userView)}
                      </p>
                    </div>
                  </div>
                  <div className="detail-list">
                    <DetailRow
                      label="Source type"
                      value={watchSourceKindLabel(selectedItem.watchResult.sourceKind)}
                    />
                    <DetailRow
                      label="Source origin"
                      value={watchSourceOriginLabel(selectedItem.watchResult.sourceOrigin)}
                    />
                    <DetailRow
                      label="What this source does"
                      value={sourceBehaviorSummary(selectedItem.watchResult, userView)}
                    />
                    <DetailRow
                      label="Last checked"
                      value={formatCheckedAt(selectedItem.watchResult.checkedAt)}
                    />
                  </div>
                  {selectedItem.watchResult.sourceLabel ? (
                    <p className="text-muted">
                      Watching <strong>{selectedItem.watchResult.sourceLabel}</strong>
                    </p>
                  ) : null}
                  {selectedItem.watchResult.note ? (
                    <p className="text-muted">{selectedItem.watchResult.note}</p>
                  ) : null}
                  {selectedItem.watchResult.evidence.length ? (
                    <div className="tag-list">
                      {selectedItem.watchResult.evidence.map((entry) => (
                        <span key={entry} className="ghost-chip">
                          {entry}
                        </span>
                      ))}
                    </div>
                  ) : null}
                  {selectedItem.watchResult.sourceUrl ? (
                    <a
                      href={selectedItem.watchResult.sourceUrl}
                      target="_blank"
                      rel="noreferrer noopener"
                      className="updates-source-link"
                    >
                      <ExternalLink size={12} strokeWidth={2} />
                      Open source page
                    </a>
                  ) : null}
                </div>
              </div>
            ) : null}

            <div className="detail-block">
              <div className="section-label">Source review</div>
              <div className="detail-list">
                <DetailRow
                  label="Source confidence"
                  value={sourceConfidenceLabel(selectedItem, selectedSetupRow)}
                />
                <DetailRow
                  label="Next step"
                  value={
                    mode === "setup"
                      ? selectedSetupRow?.suggestionState === "possible_source"
                        ? "Review the suggested page, then save it if it looks right."
                        : "Add a manual page or keep this as a reminder for later."
                      : mode === "reminders"
                        ? "Use this as a manual bookmark, or replace it with a page SimSuite can actually check."
                        : mode === "attention"
                          ? "Review the result, then decide whether the saved page is good enough or needs replacing."
                          : "Run a check when needed, then keep or replace the source based on the result."
                  }
                />
              </div>
              {deriveSourceClues(selectedItem).length ? (
                <div className="tag-list">
                  {deriveSourceClues(selectedItem).map((entry) => (
                    <span key={entry} className="ghost-chip">
                      {entry}
                    </span>
                  ))}
                </div>
              ) : null}
              {mode === "setup" && selectedSetupRow ? (
                <p className="text-muted">{selectedSetupRow.setupHint}</p>
              ) : null}
            </div>

            <div className="detail-block">
              <div className="section-label">Actions</div>
              <div className="updates-action-grid">
                {canEditSelectedSource ? (
                  <button
                    type="button"
                    className="secondary-action"
                    onClick={() => setWatchEditing(true)}
                  >
                    <Settings2 size={14} strokeWidth={2} />
                    {selectedItem.watchResult?.sourceUrl || selectedItem.watchResult?.sourceLabel
                      ? "Edit source"
                      : "Set source"}
                  </button>
                ) : (
                  <div className="updates-built-in-note">
                    SimSuite is using its built-in page for this item.
                  </div>
                )}

                {selectedItem.watchResult ? (
                  <button
                    type="button"
                    className="secondary-action"
                    onClick={() => void handleRefreshWatchSource()}
                    disabled={refreshingWatch || !canRefreshSelectedSource}
                  >
                    <RefreshCw
                      size={14}
                      strokeWidth={2}
                      className={refreshingWatch ? "spin" : undefined}
                    />
                    {refreshingWatch ? "Checking..." : "Check selected"}
                  </button>
                ) : null}
              </div>
              <p className="updates-action-note">
                {mode === "setup"
                  ? "Source editing opens in a side sheet so you can keep the queue and status details in view while you save the page."
                  : "Keep the item details open here, then slide in the source editor only when you need to change or save a page."}
              </p>
            </div>
          </div>
        ) : (
          <div className="detail-empty updates-empty-state">
            <p className="eyebrow">{userView === "beginner" ? "Selected item" : "Inspector"}</p>
            <h2>Select an item</h2>
            <p>
              {userView === "beginner"
                ? "Pick something from the list to set a source or check its update state."
                : "Select a row to inspect its current source, status, and next action."}
            </p>
          </div>
        )}
      </WorkbenchInspector>

      <AnimatePresence>
        {selectedItem && watchEditing ? (
          <m.div
            className="workbench-sheet-shell"
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            transition={overlayTransition}
            onClick={closeWatchEditor}
          >
            <m.aside
              className="workbench-sheet updates-watch-sheet"
              role="dialog"
              aria-modal="true"
              aria-labelledby="updates-watch-sheet-title"
              initial={{ opacity: 0, x: 52 }}
              animate={{ opacity: 1, x: 0 }}
              exit={{ opacity: 0, x: 58 }}
              transition={panelSpring}
              onClick={(event) => event.stopPropagation()}
            >
              <div className="workbench-sheet-header">
                <div>
                  <p className="eyebrow">
                    {mode === "setup" ? "Save source" : "Source editor"}
                  </p>
                  <h2 id="updates-watch-sheet-title">
                    {selectedItem.watchResult?.sourceLabel
                      ? "Update this saved page"
                      : "Save a watch page"}
                  </h2>
                  <p className="workbench-sheet-copy">
                    {mode === "setup"
                      ? "Pick the page here, save it once, and let SimSuite reuse it on the next check."
                      : "Change the saved page here without losing sight of the item details behind it."}
                  </p>
                </div>
                <button
                  type="button"
                  className="workspace-toggle"
                  onClick={closeWatchEditor}
                  aria-label="Close watch source editor"
                >
                  <X size={14} strokeWidth={2} />
                </button>
              </div>

              <div className="workbench-sheet-body">
                <div className="updates-watch-sheet-lead">
                  <div>
                    <span className="section-label">Editing</span>
                    <strong>{selectedItem.filename}</strong>
                    <p className="workspace-toolbar-copy">
                      {selectedItem.creator ?? unknownCreatorLabel(userView)} ·{" "}
                      {friendlyTypeLabel(selectedItem.kind)}
                    </p>
                  </div>

                  <div className="updates-watch-sheet-meta">
                    <span className="ghost-chip">
                      {watchSourceKindLabel(watchSourceKind)}
                    </span>
                    <span className="ghost-chip">
                      {selectedItem.watchResult?.sourceLabel
                        ? "Saved source"
                        : "Needs source"}
                    </span>
                  </div>
                </div>

                <div className="updates-watch-sheet-guidance">
                  <strong>
                    {userView === "beginner"
                      ? "Pick the clearest page you trust"
                      : "Save the source that gives the cleanest follow-up"}
                  </strong>
                  <p>
                    {userView === "beginner"
                      ? "Exact pages are best when one file clearly maps to one download page. Creator pages work better when one page covers a whole creator set."
                      : "Use exact pages for one-to-one release tracking. Use creator pages when several related files share one release stream."}
                  </p>
                </div>

                <div className="updates-form-grid">
                  <label className="field">
                    <span>Source type</span>
                    <select
                      value={watchSourceKind}
                      onChange={(event) =>
                        setWatchSourceKind(event.target.value as WatchSourceKind)
                      }
                    >
                      <option value="exact_page">Exact page</option>
                      <option value="creator_page">Creator page</option>
                    </select>
                  </label>

                  <label className="field">
                    <span>Label</span>
                    <input
                      type="text"
                      value={watchSourceLabel}
                      onChange={(event) => setWatchSourceLabel(event.target.value)}
                      placeholder="What page is this?"
                    />
                  </label>

                  <label className="field">
                    <span>URL</span>
                    <input
                      type="url"
                      value={watchSourceUrl}
                      onChange={(event) => setWatchSourceUrl(event.target.value)}
                      placeholder="https://..."
                    />
                  </label>
                </div>

                {canClearSelectedSource ? (
                  <div className="updates-watch-sheet-danger">
                    <strong>Remove the saved page</strong>
                    <p>
                      Clear it if this page is wrong and you want the item back in setup.
                    </p>
                    <button
                      type="button"
                      className="secondary-action"
                      onClick={() => void handleClearWatchSource()}
                      disabled={clearingWatch}
                    >
                      <Trash2 size={14} strokeWidth={2} />
                      {clearingWatch ? "Clearing..." : "Clear source"}
                    </button>
                  </div>
                ) : null}
              </div>

              <div className="workbench-sheet-footer">
                <button
                  type="button"
                  className="secondary-action"
                  onClick={closeWatchEditor}
                >
                  Cancel
                </button>
                <button
                  type="button"
                  className="primary-action"
                  onClick={() => void handleSaveWatchSource()}
                  disabled={savingWatch || !watchSourceUrl.trim()}
                >
                  {savingWatch ? "Saving..." : "Save source"}
                </button>
              </div>
            </m.aside>
          </m.div>
        ) : null}
      </AnimatePresence>
    </Workbench>
  );
}

function UpdatesStageStatCard({
  label,
  value,
  tone,
  note,
}: {
  label: string;
  value: number;
  tone: "good" | "warn" | "low" | "muted";
  note: string;
}) {
  return (
    <div className={`updates-stage-stat updates-stage-stat-${tone}`}>
      <span>{label}</span>
      <strong>{value.toLocaleString()}</strong>
      <p>{note}</p>
    </div>
  );
}

function describeUpdatesStageFocus(
  item: FileDetail,
  mode: UpdateMode,
  userView: UserView,
) {
  if (mode === "setup") {
    return userView === "beginner"
      ? "This file still needs one saved page before SimSuite can keep checking it for you."
      : "Save one exact or creator page here so this file can move from setup into the watched lane.";
  }

  if (mode === "attention") {
    return item.watchResult?.note?.trim()
      ? item.watchResult.note
      : userView === "beginner"
        ? "This file needs a closer look before you trust the update story."
        : "This file stays in the attention lane because the result still needs human judgment.";
  }

  if (mode === "reminders") {
    return item.watchResult?.note?.trim()
      ? item.watchResult.note
      : userView === "beginner"
        ? "This saved page is acting like a bookmark, not a live update check."
        : "This source is saved for manual follow-up only, so it stays out of the watched lane.";
  }

  return item.watchResult?.note?.trim()
    ? item.watchResult.note
    : userView === "beginner"
      ? "This watched file has the clearest next update story in the current view."
      : "This watched file is the clearest current follow-up in the selected watch lane.";
}

function updatesGuidanceTitle(mode: UpdateMode, userView: UserView) {
  if (mode === "setup") {
    return userView === "beginner" ? "How to pick the page" : "Choosing the source";
  }

  if (mode === "attention") {
    return userView === "beginner" ? "Why it needs attention" : "Why this lane exists";
  }

  if (mode === "reminders") {
    return userView === "beginner" ? "Why this is reminder only" : "Why reminders stay separate";
  }

  return userView === "beginner" ? "How watched checks work" : "Reading watched results";
}

function updatesGuidanceBody(mode: UpdateMode, userView: UserView) {
  if (mode === "setup") {
    return userView === "beginner"
      ? "Use an exact page when one file clearly belongs to one download page. Use a creator page when that source covers a whole set from the same creator."
      : "Exact pages work best for one-to-one matches. Creator pages are better when several related files share one release stream.";
  }

  if (mode === "attention") {
    return userView === "beginner"
      ? "Attention keeps possible updates, provider blocks, and unclear checks in one place so they do not get mixed up with calm watched items."
      : "This lane holds update leads and blocked/unclear checks without pretending those results are already settled.";
  }

  if (mode === "reminders") {
    return userView === "beginner"
      ? "Reminder-only pages are saved so you can come back later, but SimSuite is not checking them automatically."
      : "Bookmarks and manual reference pages stay isolated here so they do not impersonate live watch sources.";
  }

  return userView === "beginner"
    ? "Built-in exact pages can often be checked directly. Creator pages and first-time checks stay more careful because one source may cover several files or versions."
    : "Watched results stay together here so clean checks and ready-to-check items can be compared without burying the action lanes.";
}

function UpdatesWatchingTable({
  items,
  loading,
  selectedId,
  onSelect,
  userView,
  filter,
}: {
  items: LibraryWatchListItem[];
  loading: boolean;
  selectedId: number | null;
  onSelect: (item: LibraryWatchListItem) => Promise<void>;
  userView: UserView;
  filter: WatchingFilter;
}) {
  return (
    <table className="library-table updates-table">
      <thead>
        <tr>
          <th>Status</th>
          <th>File</th>
          <th>Watching</th>
          <th>Installed</th>
        </tr>
      </thead>
      <tbody>
        {loading ? (
          <tr>
            <td colSpan={4} className="empty-row">
              Loading watched items...
            </td>
          </tr>
        ) : items.length ? (
          items.map((item, index) => (
            <m.tr
              key={item.fileId}
              className={selectedId === item.fileId ? "is-selected" : ""}
              onClick={() => void onSelect(item)}
              role="button"
              tabIndex={0}
              onKeyDown={(event) => {
                if (event.key === "Enter" || event.key === " ") {
                  event.preventDefault();
                  void onSelect(item);
                }
              }}
              whileHover={rowHover}
              whileTap={rowPress}
              {...stagedListItem(index)}
            >
              <td>{watchStatusIcon(item.watchResult.status)}</td>
              <td>
                <div className="file-title">{item.filename}</div>
                <div className="updates-table-meta">
                  {item.creator ?? unknownCreatorLabel(userView)}
                </div>
              </td>
              <td>
                <div className="file-title">{item.subjectLabel}</div>
                <div className="updates-table-meta">
                  {watchStatusLabel(item.watchResult.status, userView)}
                </div>
              </td>
              <td>
                <div className="file-title">{formatVersion(item.installedVersion)}</div>
                <div className="updates-table-meta">
                  {watchSourceKindLabel(item.watchResult.sourceKind)}
                </div>
              </td>
            </m.tr>
          ))
        ) : (
          <tr>
            <td colSpan={4} className="empty-row">
              {watchingEmptyMessage(filter, userView)}
            </td>
          </tr>
        )}
      </tbody>
    </table>
  );
}

function UpdatesSetupTable({
  items,
  loading,
  selectedId,
  onSelect,
  userView,
}: {
  items: SourceNeededRow[];
  loading: boolean;
  selectedId: number | null;
  onSelect: (item: SourceNeededRow) => Promise<void>;
  userView: UserView;
}) {
  return (
    <table className="library-table updates-table">
      <thead>
        <tr>
          <th>File</th>
          <th>Status</th>
          <th>Suggested source</th>
          <th>Next step</th>
        </tr>
      </thead>
      <tbody>
        {loading ? (
          <tr>
            <td colSpan={4} className="empty-row">
              Loading source items...
            </td>
          </tr>
        ) : items.length ? (
          items.map((item, index) => (
            <m.tr
              key={item.fileId}
              className={selectedId === item.fileId ? "is-selected" : ""}
              onClick={() => void onSelect(item)}
              role="button"
              tabIndex={0}
              onKeyDown={(event) => {
                if (event.key === "Enter" || event.key === " ") {
                  event.preventDefault();
                  void onSelect(item);
                }
              }}
              whileHover={rowHover}
              whileTap={rowPress}
              {...stagedListItem(index)}
            >
              <td>
                <div className="file-title">{item.filename}</div>
                <div className="updates-table-meta">
                  {item.creator ?? unknownCreatorLabel(userView)}
                </div>
              </td>
              <td>
                <span className="warning-tag">{sourceNeededStatusLabel(item)}</span>
                <div className="updates-table-meta">
                  Installed {formatVersion(item.installedVersion)}
                </div>
              </td>
              <td>
                <div className="file-title">
                  <span className="ghost-chip">
                    {item.suggestedSourceKind
                      ? watchSourceKindLabel(item.suggestedSourceKind)
                      : "Manual source needed"}
                  </span>
                </div>
                <div className="updates-table-meta">{item.subjectLabel}</div>
              </td>
              <td>
                <div className="updates-table-note">{item.setupHint}</div>
              </td>
            </m.tr>
          ))
        ) : (
          <tr>
            <td colSpan={4} className="empty-row">
              {sourceNeededEmptyMessage(userView)}
            </td>
          </tr>
        )}
      </tbody>
    </table>
  );
}

function UpdatesAttentionTable({
  items,
  loading,
  selectedId,
  onSelect,
  userView,
  filter,
}: {
  items: AttentionRow[];
  loading: boolean;
  selectedId: number | null;
  onSelect: (row: AttentionRow) => Promise<void>;
  userView: UserView;
  filter: AttentionFilter;
}) {
  return (
    <table className="library-table updates-table">
      <thead>
        <tr>
          <th>Status</th>
          <th>File</th>
          <th>Watching</th>
          <th>Next step</th>
        </tr>
      </thead>
      <tbody>
        {loading ? (
          <tr>
            <td colSpan={4} className="empty-row">
              Loading attention items...
            </td>
          </tr>
        ) : items.length ? (
          items.map((row, index) => {
            const item = row.item;
            return (
              <m.tr
                key={`${row.kind}-${item.fileId}`}
                className={selectedId === item.fileId ? "is-selected" : ""}
                onClick={() => void onSelect(row)}
                role="button"
                tabIndex={0}
                onKeyDown={(event) => {
                  if (event.key === "Enter" || event.key === " ") {
                    event.preventDefault();
                    void onSelect(row);
                  }
                }}
                whileHover={rowHover}
                whileTap={rowPress}
                {...stagedListItem(index)}
              >
                <td>
                  {row.kind === "review" ? (
                    <AlertTriangle className="updates-status-icon updates-status-icon-warning" />
                  ) : (
                    watchStatusIcon(item.watchResult.status)
                  )}
                </td>
                <td>
                  <div className="file-title">{item.filename}</div>
                  <div className="updates-table-meta">
                    {item.creator ?? unknownCreatorLabel(userView)}
                  </div>
                </td>
                <td>
                  <div className="file-title">{item.subjectLabel}</div>
                  <div className="updates-table-meta">
                    {attentionStatusLabel(row, userView)}
                  </div>
                </td>
                <td>
                  <span className="warning-tag">{attentionStatusLabel(row, userView)}</span>
                  <div className="updates-table-meta">{attentionHint(row)}</div>
                </td>
              </m.tr>
            );
          })
        ) : (
          <tr>
            <td colSpan={4} className="empty-row">
              {filter === "provider_required"
                ? userView === "beginner"
                  ? "Nothing is blocked on provider support right now."
                  : "No provider-required items match this filter."
                : userView === "beginner"
                  ? "Nothing needs attention right now."
                  : "No attention items match this filter."}
            </td>
          </tr>
        )}
      </tbody>
    </table>
  );
}

function UpdatesReminderTable({
  items,
  loading,
  selectedId,
  onSelect,
  userView,
  filter,
}: {
  items: LibraryWatchReviewItem[];
  loading: boolean;
  selectedId: number | null;
  onSelect: (item: LibraryWatchReviewItem) => Promise<void>;
  userView: UserView;
  filter: ReminderFilter;
}) {
  return (
    <table className="library-table updates-table">
      <thead>
        <tr>
          <th>Status</th>
          <th>File</th>
          <th>Saved page</th>
          <th>Reminder</th>
        </tr>
      </thead>
      <tbody>
        {loading ? (
          <tr>
            <td colSpan={4} className="empty-row">
              Loading reminder items...
            </td>
          </tr>
        ) : items.length ? (
          items.map((item, index) => (
            <m.tr
              key={item.fileId}
              className={selectedId === item.fileId ? "is-selected" : ""}
              onClick={() => void onSelect(item)}
              role="button"
              tabIndex={0}
              onKeyDown={(event) => {
                if (event.key === "Enter" || event.key === " ") {
                  event.preventDefault();
                  void onSelect(item);
                }
              }}
              whileHover={rowHover}
              whileTap={rowPress}
              {...stagedListItem(index)}
            >
              <td>
                <Bookmark className="updates-status-icon updates-status-icon-muted" />
              </td>
              <td>
                <div className="file-title">{item.filename}</div>
                <div className="updates-table-meta">
                  {item.creator ?? unknownCreatorLabel(userView)}
                </div>
              </td>
              <td>
                <div className="file-title">{item.subjectLabel}</div>
                <div className="updates-table-meta">
                  {watchSourceKindLabel(item.watchResult.sourceKind)}
                </div>
              </td>
              <td>
                <span className="warning-tag">Reminder only</span>
                <div className="updates-table-meta">{item.reviewHint}</div>
              </td>
            </m.tr>
          ))
        ) : (
          <tr>
            <td colSpan={4} className="empty-row">
              {reminderOnlyEmptyMessage(filter, userView)}
            </td>
          </tr>
        )}
      </tbody>
    </table>
  );
}

function DetailRow({ label, value }: { label: string; value: string }) {
  return (
    <div className="detail-row">
      <span>{label}</span>
      <strong>{value}</strong>
    </div>
  );
}
