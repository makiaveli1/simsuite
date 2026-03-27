import {
  startTransition,
  type ReactNode,
  useCallback,
  useDeferredValue,
  useEffect,
  useRef,
  useState,
} from "react";
import { AnimatePresence, LayoutGroup, m } from "motion/react";
import {
  AlertTriangle,
  Download,
  FolderSearch,
  Inbox,
  LoaderCircle,
  RefreshCw,
  ShieldAlert,
  Workflow,
  X,
} from "lucide-react";
import { CasualGuidedTours } from "../components/CasualGuidedTours";
import { KeyboardShortcutsDialog } from "../components/KeyboardShortcutsDialog";
import {
  type DockSectionDefinition,
} from "../components/DockSectionStack";
import { Workbench } from "../components/layout/Workbench";
import { WorkbenchInspector } from "../components/layout/WorkbenchInspector";
import { WorkbenchRail } from "../components/layout/WorkbenchRail";
import { WorkbenchStage } from "../components/layout/WorkbenchStage";
import { ResizableEdgeHandle } from "../components/ResizableEdgeHandle";
import { StatePanel } from "../components/StatePanel";
import { useUiPreferences } from "../components/UiPreferencesContext";
import { api, hasTauriRuntime } from "../lib/api";
import {
  isDownloadsTourDismissed,
  isKeyboardHintDismissed,
  setKeyboardHintDismissed,
} from "../lib/guidedFlowStorage";
import {
  downloadsSelectionTransition,
  rowHover,
  rowPress,
  stagedListItem,
} from "../lib/motion";
import {
  friendlyTypeLabel,
  intakeModeLabel,
  reviewLabel,
  riskLevelLabel,
  sampleCountLabel,
  sampleToggleLabel,
} from "../lib/uiLanguage";
import type {
  DependencyStatus,
  DownloadInboxDetail,
  DownloadIntakeMode,
  DownloadProgress,
  DownloadQueueLane,
  DownloadsInboxItem,
  DownloadsInboxResponse,
  DownloadsWatcherStatus,
  GuidedInstallFileEntry,
  GuidedInstallPlan,
  OrganizationPreview,
  ReviewPlanAction,
  RulePreset,
  Screen,
  SpecialModDecision,
  SpecialReviewPlan,
  UserView,
  VersionConfidence,
  VersionResolution,
} from "../lib/types";
import { DownloadsRail } from "./downloads/DownloadsRail";
import { DownloadsBatchCanvas } from "./downloads/DownloadsBatchCanvas";
import {
  DownloadsDecisionPanel,
  type DownloadsDecisionBadge,
  type DownloadsDecisionSignal,
} from "./downloads/DownloadsDecisionPanel";
import { DownloadsProofSheet } from "./downloads/DownloadsProofSheet";
import {
  DownloadsQueuePanel,
  type DownloadsQueueRowModel,
} from "./downloads/DownloadsQueuePanel";
import { DownloadsSetupDialog } from "./downloads/DownloadsSetupDialog";
import { DownloadsTopStrip } from "./downloads/DownloadsTopStrip";
import {
  reviewActionButtonLabel,
  reviewActionCardTitle,
} from "./downloads/reviewActionText";
import {
  capRowBadges,
  fallbackDownloadsLane,
  pickInitialDownloadsLane,
  viewModeDownloadsFlags,
} from "./downloads/downloadsDisplay";

interface DownloadsScreenProps {
  refreshVersion: number;
  onNavigate: (screen: Screen) => void;
  onDataChanged: () => void;
  userView: UserView;
}

interface DownloadsSelectionState {
  itemId: number | null;
  requestId: number;
  detail: DownloadInboxDetail | null;
  preview: OrganizationPreview | null;
  guidedPlan: GuidedInstallPlan | null;
  reviewPlan: SpecialReviewPlan | null;
}

interface DownloadsScreenCache {
  refreshVersion: number;
  watcherStatus: DownloadsWatcherStatus | null;
  inbox: DownloadsInboxResponse | null;
  activeLane: DownloadQueueLane | null;
  selectedItemId: number | null;
  selectedPreset: string;
  search: string;
  statusFilter: string;
}

type DownloadsDialogRequest =
  | { kind: "guided_apply" }
  | { kind: "safe_move" }
  | { kind: "reject" }
  | { kind: "review_action"; action: ReviewPlanAction }
  | { kind: "undo"; itemId: number; displayName: string }
  | { kind: "batch_apply"; count: number }
  | { kind: "batch_reject"; count: number };

const AUTO_RECHECK_NOTE_PREFIX = "Rechecked with newer SimSuite rules";
const DEFAULT_DOWNLOADS_PRESET = "Category First";
const WORKSPACE_RELOAD_GRACE_MS = 1200;
const downloadsScreenCache: DownloadsScreenCache = {
  refreshVersion: -1,
  watcherStatus: null,
  inbox: null,
  activeLane: null,
  selectedItemId: null,
  selectedPreset: DEFAULT_DOWNLOADS_PRESET,
  search: "",
  statusFilter: "",
};

function createEmptySelectionState(): DownloadsSelectionState {
  return {
    itemId: null,
    requestId: 0,
    detail: null,
    preview: null,
    guidedPlan: null,
    reviewPlan: null,
  };
}

export function DownloadsScreen({
  refreshVersion,
  onNavigate,
  onDataChanged,
  userView,
}: DownloadsScreenProps) {
  const {
    downloadsDetailWidth,
    downloadsQueueHeight,
    setDownloadsDetailWidth,
    setDownloadsQueueHeight,
  } = useUiPreferences();
  const skipInitialBootstrap = useRef(
    downloadsScreenCache.refreshVersion === refreshVersion &&
      (downloadsScreenCache.watcherStatus !== null ||
        downloadsScreenCache.inbox !== null),
  );
  const [watcherStatus, setWatcherStatus] = useState<DownloadsWatcherStatus | null>(
    downloadsScreenCache.watcherStatus,
  );
  const [progress, setProgress] = useState<DownloadProgress | null>(null);
  const [inbox, setInbox] = useState<DownloadsInboxResponse | null>(
    downloadsScreenCache.inbox,
  );
  const [presets, setPresets] = useState<RulePreset[]>([]);
  const [selectedPreset, setSelectedPreset] = useState(
    downloadsScreenCache.selectedPreset,
  );
  const [activeLane, setActiveLane] = useState<DownloadQueueLane | null>(
    downloadsScreenCache.activeLane,
  );
  const [selectedItemId, setSelectedItemId] = useState<number | null>(
    downloadsScreenCache.selectedItemId,
  );
  const [selectionState, setSelectionState] = useState<DownloadsSelectionState>(
    createEmptySelectionState,
  );
  const [search, setSearch] = useState(downloadsScreenCache.search);
  const [statusFilter, setStatusFilter] = useState(downloadsScreenCache.statusFilter);
  const [statusMessage, setStatusMessage] = useState<string | null>(null);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [filtersOpen, setFiltersOpen] = useState(
    viewModeDownloadsFlags(userView).showAdvancedFiltersByDefault,
  );
  const [proofSheetOpen, setProofSheetOpen] = useState(false);
  const [pendingDialog, setPendingDialog] = useState<DownloadsDialogRequest | null>(
    null,
  );
  const [isLoadingInbox, setIsLoadingInbox] = useState(false);
  const [isRefreshing, setIsRefreshing] = useState(false);
  const [isLoadingSelection, setIsLoadingSelection] = useState(false);
  const [isApplying, setIsApplying] = useState(false);
  const [isRejecting, setIsRejecting] = useState(false);
  const [isUndoing, setIsUndoing] = useState(false);
  const [undoableApply, setUndoableApply] = useState<{ itemId: number; displayName: string } | null>(null);
  const [batchSelectedIds, setBatchSelectedIds] = useState<Set<number>>(new Set());
  const [showTour, setShowTour] = useState(false);
  const [showShortcutsHelp, setShowShortcutsHelp] = useState(false);
  const [showKeyboardHint, setShowKeyboardHint] = useState(
    () => !isKeyboardHintDismissed() && userView !== "power",
  );
  const latestWatcherStatus = useRef<DownloadsWatcherStatus | null>(
    downloadsScreenCache.watcherStatus,
  );
  const latestInbox = useRef<DownloadsInboxResponse | null>(
    downloadsScreenCache.inbox,
  );
  const latestBusyState = useRef({
    isLoadingInbox: false,
    isApplying: false,
    isRejecting: false,
    isRefreshing: false,
  });
  const inboxRetryTimer = useRef<number | null>(null);
  const watcherPollTimer = useRef<number | null>(null);
  const workspaceReloadTimer = useRef<number | null>(null);
  const queueRequestId = useRef(0);
  const selectionRequestId = useRef(0);
  const pendingWorkspaceReload = useRef(false);
  const pendingPreferredSelectionId = useRef<number | null>(null);
  const tourShownRef = useRef(false);
  const previousRefreshVersion = useRef(refreshVersion);
  const latestRefreshVersion = useRef(refreshVersion);
  const skipWorkspaceReloadUntil = useRef(0);
  const deferredSearch = useDeferredValue(search);
  const groupedItems = groupDownloadItems(inbox?.items ?? []);
  const visibleLaneCounts = Object.fromEntries(
    DOWNLOAD_LANE_ORDER.map((lane) => [
      lane,
      groupedItems.find((group) => group.lane === lane)?.items.length ?? 0,
    ]),
  ) as Record<DownloadQueueLane, number>;
  const resolvedActiveLane = activeLane ?? pickInitialDownloadsLane(visibleLaneCounts);
  const activeLaneItems =
    groupedItems.find((group) => group.lane === resolvedActiveLane)?.items ?? [];

  useEffect(() => {
    latestRefreshVersion.current = refreshVersion;
    latestWatcherStatus.current = watcherStatus;
    latestInbox.current = inbox;
    latestBusyState.current = {
      isLoadingInbox,
      isApplying,
      isRejecting,
      isRefreshing,
    };
    downloadsScreenCache.refreshVersion = refreshVersion;
    downloadsScreenCache.watcherStatus = watcherStatus;
    downloadsScreenCache.inbox = inbox;
    downloadsScreenCache.activeLane = activeLane;
    downloadsScreenCache.selectedItemId = selectedItemId;
    downloadsScreenCache.selectedPreset = selectedPreset;
    downloadsScreenCache.search = search;
    downloadsScreenCache.statusFilter = statusFilter;
  }, [
    activeLane,
    inbox,
    isApplying,
    isRejecting,
    isLoadingInbox,
    isRefreshing,
    refreshVersion,
    search,
    selectedItemId,
    selectedPreset,
    statusFilter,
    watcherStatus,
  ]);

  useEffect(() => {
    if (userView === "beginner" && filtersOpen) {
      setFiltersOpen(false);
    }
  }, [filtersOpen, userView]);

  useEffect(() => {
    void api
      .listRulePresets()
      .then((items) => {
        startTransition(() => {
          setPresets(items);
          if (items.length > 0) {
            setSelectedPreset((current) =>
              items.some((item) => item.name === current)
                ? current
                : items[0].name,
            );
          }
        });
      })
      .catch((error) => setErrorMessage(toErrorMessage(error)));
  }, []);

  useEffect(() => {
    const unlisten = api.listenToDownloadsStatus((status) => {
      startTransition(() => {
        setWatcherStatus(status);
      });
    });

    return () => {
      void unlisten.then((dispose) => dispose());
    };
  }, []);

  useEffect(() => {
    const unlisten = api.listenToDownloadsProgress((dlProgress) => {
      startTransition(() => {
        setProgress(dlProgress);
      });
    });

    return () => {
      void unlisten.then((dispose) => dispose());
    };
  }, []);

  useEffect(() => {
    return () => {
      if (inboxRetryTimer.current !== null) {
        globalThis.clearTimeout(inboxRetryTimer.current);
      }
      if (watcherPollTimer.current !== null) {
        globalThis.clearTimeout(watcherPollTimer.current);
      }
      if (workspaceReloadTimer.current !== null) {
        globalThis.clearTimeout(workspaceReloadTimer.current);
      }
    };
  }, []);

  useEffect(() => {
    if (!watcherStatus?.configured || watcherStatus.state !== "processing") {
      clearScheduledWatcherPoll();
      return;
    }

    scheduleWatcherPoll(inbox === null ? 180 : 320);

    return () => {
      clearScheduledWatcherPoll();
    };
  }, [inbox, watcherStatus?.configured, watcherStatus?.state]);

  useEffect(() => {
    if (!isRefreshing || !watcherStatus) {
      return;
    }

    if (watcherStatus.state === "processing") {
      return;
    }

    let cancelled = false;

    void (async () => {
      if (watcherStatus.state === "error") {
        if (!cancelled && watcherStatus.lastError) {
          setErrorMessage(watcherStatus.lastError);
        }
        if (!cancelled) {
          setIsRefreshing(false);
        }
        return;
      }

      await loadInbox();
      markRecentLocalInboxReload();
      if (cancelled) {
        return;
      }

      setStatusMessage(
        userView === "beginner"
          ? "Inbox checked again."
          : "Inbox refreshed and checked again.",
      );
      setIsRefreshing(false);
    })();

    return () => {
      cancelled = true;
    };
  }, [isRefreshing, userView, watcherStatus]);

  useEffect(() => {
    if (skipInitialBootstrap.current) {
      skipInitialBootstrap.current = false;
      return;
    }

    void loadVisibleInbox();
  }, [deferredSearch, statusFilter]);

  useEffect(() => {
    if (previousRefreshVersion.current === refreshVersion) {
      return;
    }

    previousRefreshVersion.current = refreshVersion;
    if (Date.now() < skipWorkspaceReloadUntil.current) {
      return;
    }

    if (shouldPauseWorkspaceReload()) {
      pendingWorkspaceReload.current = true;
      scheduleWorkspaceReload();
      return;
    }

    void loadVisibleInbox();
  }, [
    isApplying,
    isRejecting,
    isLoadingInbox,
    isRefreshing,
    refreshVersion,
    watcherStatus?.configured,
    watcherStatus?.state,
  ]);

  useEffect(() => {
    if (!pendingWorkspaceReload.current) {
      clearScheduledWorkspaceReload();
      return;
    }

    if (Date.now() < skipWorkspaceReloadUntil.current || shouldPauseWorkspaceReload()) {
      scheduleWorkspaceReload();
      return;
    }

    pendingWorkspaceReload.current = false;
    clearScheduledWorkspaceReload();
    void loadVisibleInbox();
  }, [
    isApplying,
    isRejecting,
    isLoadingInbox,
    isRefreshing,
    watcherStatus?.configured,
    watcherStatus?.state,
  ]);

  useEffect(() => {
    if (!watcherStatus?.configured) {
      return;
    }

    if (
      watcherStatus.state === "processing" ||
      watcherStatus.state === "error" ||
      isLoadingInbox ||
      isRefreshing
    ) {
      return;
    }

    if (inbox === null) {
      void loadInbox();
    }
  }, [
    inbox,
    isLoadingInbox,
    isRefreshing,
    watcherStatus?.configured,
    watcherStatus?.state,
  ]);

  useEffect(() => {
    const hasItems = DOWNLOAD_LANE_ORDER.some((lane) => visibleLaneCounts[lane] > 0);
    if (!hasItems) {
      if (activeLane !== null) {
        setActiveLane(null);
      }
      return;
    }

    setActiveLane((current) =>
      current
        ? fallbackDownloadsLane(current, visibleLaneCounts)
        : pickInitialDownloadsLane(visibleLaneCounts),
    );
  }, [
    activeLane,
    visibleLaneCounts.blocked,
    visibleLaneCounts.done,
    visibleLaneCounts.ready_now,
    visibleLaneCounts.special_setup,
    visibleLaneCounts.waiting_on_you,
  ]);

  useEffect(() => {
    const items = activeLaneItems;
    if (!items.length) {
      pendingPreferredSelectionId.current = null;
      setSelectedItemId(null);
      return;
    }

    const preferredItemId = pendingPreferredSelectionId.current;
    if (
      preferredItemId !== null &&
      items.some((item) => item.id === preferredItemId)
    ) {
      pendingPreferredSelectionId.current = null;
      setSelectedItemId(preferredItemId);
      return;
    }

    if (selectedItemId === null) {
      return;
    }

    if (!items.some((item) => item.id === selectedItemId)) {
      setSelectedItemId(items[0].id);
    }
  }, [activeLaneItems, selectedItemId]);

  const selectedItem =
    activeLaneItems.find((item) => item.id === selectedItemId) ?? null;

  // Show keyboard shortcut hint on first item selection (beginner only)
  useEffect(() => {
    if (selectedItem && showKeyboardHint) {
      const timer = setTimeout(() => {
        setShowKeyboardHint(false);
        setKeyboardHintDismissed();
      }, 8000);
      return () => clearTimeout(timer);
    }
  }, [selectedItem, showKeyboardHint]);

  useEffect(() => {
    if (!selectedItem) {
      const requestId = ++selectionRequestId.current;
      setIsLoadingSelection(false);
      setSelectionState({
        ...createEmptySelectionState(),
        requestId,
      });
      return;
    }

    void loadSelectedItem(selectedItem);
  }, [selectedItem?.id, selectedItem?.updatedAt, selectedPreset]);

  // Show the downloads tour once when inbox data first loads in Casual mode
  useEffect(() => {
    if (
      userView === "beginner" &&
      inbox !== null &&
      !isDownloadsTourDismissed() &&
      !tourShownRef.current
    ) {
      tourShownRef.current = true;
      setShowTour(true);
    }
  }, [userView, inbox]);

  function clearScheduledInboxRetry() {
    if (inboxRetryTimer.current !== null) {
      globalThis.clearTimeout(inboxRetryTimer.current);
      inboxRetryTimer.current = null;
    }
  }

  function scheduleInboxRetry(callback: () => void, delayMs = 320) {
    if (inboxRetryTimer.current !== null) {
      return;
    }

    inboxRetryTimer.current = globalThis.setTimeout(() => {
      inboxRetryTimer.current = null;
      callback();
    }, delayMs);
  }

  function clearScheduledWatcherPoll() {
    if (watcherPollTimer.current !== null) {
      globalThis.clearTimeout(watcherPollTimer.current);
      watcherPollTimer.current = null;
    }
  }

  function clearScheduledWorkspaceReload() {
    if (workspaceReloadTimer.current !== null) {
      globalThis.clearTimeout(workspaceReloadTimer.current);
      workspaceReloadTimer.current = null;
    }
  }

  function scheduleWatcherPoll(delayMs = 320) {
    if (watcherPollTimer.current !== null) {
      return;
    }

    watcherPollTimer.current = globalThis.setTimeout(() => {
      watcherPollTimer.current = null;
      void refreshWatcherStatus();
    }, delayMs);
  }

  function scheduleWorkspaceReload(delayMs = 320) {
    if (workspaceReloadTimer.current !== null) {
      return;
    }

    workspaceReloadTimer.current = globalThis.setTimeout(() => {
      workspaceReloadTimer.current = null;
      if (!pendingWorkspaceReload.current) {
        return;
      }
      if (Date.now() < skipWorkspaceReloadUntil.current || shouldPauseWorkspaceReload()) {
        scheduleWorkspaceReload(320);
        return;
      }
      pendingWorkspaceReload.current = false;
      void loadVisibleInbox();
    }, delayMs);
  }

  function shouldPauseWorkspaceReload() {
    const busyState = latestBusyState.current;
    const currentWatcherStatus = latestWatcherStatus.current;
    return (
      busyState.isLoadingInbox ||
      busyState.isApplying ||
      busyState.isRejecting ||
      busyState.isRefreshing ||
      currentWatcherStatus?.state === "processing"
    );
  }

  function markRecentLocalInboxReload() {
    pendingWorkspaceReload.current = false;
    clearScheduledWorkspaceReload();
    skipWorkspaceReloadUntil.current = Date.now() + WORKSPACE_RELOAD_GRACE_MS;
    previousRefreshVersion.current = latestRefreshVersion.current;
  }

  function handleLockedInboxRead(error: unknown, retry: () => void) {
    const message = toErrorMessage(error);
    if (!isLockedDatabaseError(message)) {
      setErrorMessage(message);
      return;
    }

    setErrorMessage(null);
    setStatusMessage(
      inbox?.items?.length
        ? userView === "beginner"
          ? "Inbox is still catching up. Trying again."
          : "Inbox is still finishing another check. Trying again."
        : userView === "beginner"
          ? "Checking your Downloads inbox again."
          : "Inbox is still checking your Downloads folder. Trying again.",
    );
    scheduleInboxRetry(retry);
  }

  async function refreshWatcherStatus() {
    try {
      const status = await api.getDownloadsWatcherStatus();
      startTransition(() => {
        setWatcherStatus(status);
      });
    } catch (error) {
      setErrorMessage(toErrorMessage(error));
    }
  }

  async function loadVisibleInbox() {
    const currentWatcherStatus = latestWatcherStatus.current;
    const currentInbox = latestInbox.current;
    if (
      currentWatcherStatus?.configured &&
      currentWatcherStatus.state !== "processing" &&
      currentWatcherStatus.state !== "error" &&
      currentInbox !== null
    ) {
      await loadInbox();
      return;
    }

    await loadBootstrap();
  }

  async function reloadInboxAfterMutation(preferredItemId?: number | null) {
    pendingPreferredSelectionId.current = preferredItemId ?? null;
    await loadInbox();
    markRecentLocalInboxReload();
    void refreshWatcherStatus();
  }

  async function loadBootstrap() {
    const requestId = ++queueRequestId.current;
    setIsLoadingInbox(true);
    setErrorMessage(null);

    try {
      clearScheduledInboxRetry();
      const response = await api.getDownloadsBootstrap({
        search: deferredSearch || undefined,
        status: statusFilter || undefined,
        limit: 120,
      });

      if (requestId !== queueRequestId.current) {
        return;
      }

      startTransition(() => {
        setWatcherStatus(response.watcherStatus);
        if (response.queue) {
          setInbox(response.queue);
          return;
        }

        if (!response.watcherStatus.configured) {
          setInbox(null);
        }
      });
    } catch (error) {
      handleLockedInboxRead(error, () => {
        void loadBootstrap();
      });
    } finally {
      if (requestId === queueRequestId.current) {
        setIsLoadingInbox(false);
      }
    }
  }

  async function loadInbox() {
    const requestId = ++queueRequestId.current;
    setIsLoadingInbox(true);
    setErrorMessage(null);

    try {
      clearScheduledInboxRetry();
      const response = await api.getDownloadsQueue({
        search: deferredSearch || undefined,
        status: statusFilter || undefined,
        limit: 120,
      });
      if (requestId !== queueRequestId.current) {
        return;
      }
      startTransition(() => {
        setInbox(response);
      });
    } catch (error) {
      handleLockedInboxRead(error, () => {
        void loadInbox();
      });
    } finally {
      if (requestId === queueRequestId.current) {
        setIsLoadingInbox(false);
      }
    }
  }

  async function loadSelectedItem(item: DownloadsInboxItem) {
    const requestId = ++selectionRequestId.current;
    setIsLoadingSelection(true);
    setErrorMessage(null);
    startTransition(() => {
      setSelectionState({
        itemId: item.id,
        requestId,
        detail: null,
        preview: null,
        guidedPlan: null,
        reviewPlan: null,
      });
    });

    try {
      const selection = await api.getDownloadsSelection(item.id, selectedPreset);
      if (requestId !== selectionRequestId.current) {
        return;
      }

      startTransition(() => {
        setSelectionState({
          itemId: item.id,
          requestId,
          detail: selection.detail,
          preview: selection.preview,
          guidedPlan: selection.guidedPlan,
          reviewPlan: selection.reviewPlan,
        });
      });
    } catch (error) {
      if (requestId !== selectionRequestId.current) {
        return;
      }
      startTransition(() => {
        setSelectionState({
          itemId: item.id,
          requestId,
          detail: null,
          preview: null,
          guidedPlan: null,
          reviewPlan: null,
        });
      });
      setErrorMessage(toErrorMessage(error));
    } finally {
      if (requestId === selectionRequestId.current) {
        setIsLoadingSelection(false);
      }
    }
  }

  async function handleRefresh() {
    setStatusMessage(null);
    setErrorMessage(null);

    try {
      const nextStatus = await api.refreshDownloadsInbox();
      setWatcherStatus(nextStatus);
      if (nextStatus.state === "processing") {
        setIsRefreshing(true);
        setStatusMessage(
          userView === "beginner"
            ? "Inbox check started."
            : "Inbox refresh started in the background.",
        );
        return;
      }

      await loadInbox();
      markRecentLocalInboxReload();
      setStatusMessage(
        userView === "beginner"
          ? "Inbox checked again."
          : "Inbox refreshed and checked again.",
      );
    } catch (error) {
      setErrorMessage(toErrorMessage(error));
    }
  }

  async function handleReviewAction(
    action: ReviewPlanAction,
    skipApproval = false,
  ) {
    if (!selectedItem) {
      return;
    }

    const needsApproval = reviewActionNeedsApproval(action.kind);
    if (needsApproval && !skipApproval) {
      setPendingDialog({ kind: "review_action", action });
      return;
    }

    setIsApplying(true);
    setStatusMessage(null);
    setErrorMessage(null);

    try {
      const mutatesInbox = reviewActionUpdatesInbox(action.kind);
      const result = await api.applyReviewPlanAction(
        selectedItem.id,
        action.kind,
        action.relatedItemId,
        action.url,
        needsApproval,
      );

      if (result.openedUrl && !hasTauriRuntime) {
        globalThis.open?.(result.openedUrl, "_blank", "noopener,noreferrer");
      }

      const shouldReload = mutatesInbox;

      if (shouldReload) {
        await reloadInboxAfterMutation(result.focusItemId);
      }

      setSelectedItemId(result.focusItemId);

      if (
        result.snapshotId !== null ||
        result.createdItemId !== null ||
        result.installedCount > 0 ||
        result.repairedCount > 0
      ) {
        onDataChanged();
      }

      setStatusMessage(result.message);
    } catch (error) {
      setErrorMessage(toErrorMessage(error));
    } finally {
      setIsApplying(false);
    }
  }

  async function handleApply(skipConfirm = false) {
    if (!selectedItem) {
      return;
    }

    const guidedApplyReady = Boolean(
      selectedSpecialDecision?.applyReady || selectedGuidedPlan?.applyReady,
    );
    const effectiveGuided =
      selectedResolvedItem?.intakeMode === "guided" ||
      selectedItem.intakeMode === "guided" ||
      guidedApplyReady;

    if (effectiveGuided) {
      if (!guidedApplyReady || !selectedGuidedPlan) {
        return;
      }

      const isSameVersion = selectedSpecialDecision?.sameVersion ?? false;
      if (!skipConfirm) {
        setPendingDialog({ kind: "guided_apply" });
        return;
      }

      setIsApplying(true);
      setStatusMessage(null);
      setErrorMessage(null);

      try {
        const result = await api.applyGuidedDownloadItem(selectedItem.id, true);
        setStatusMessage(
          isSameVersion
            ? `${selectedGuidedPlan.profileName} was reinstalled safely. ${result.replacedCount} current file(s) were refreshed and ${result.preservedCount} settings file(s) were kept.`
            : `${selectedGuidedPlan.profileName} installed safely. ${result.installedCount} new file(s) moved, ${result.replacedCount} old file(s) replaced, and ${result.preservedCount} settings file(s) kept.`,
        );
        onDataChanged();
        await reloadInboxAfterMutation();
      } catch (error) {
        setErrorMessage(toErrorMessage(error));
      } finally {
        setIsApplying(false);
      }

      return;
    }

    const safeCount = actionableCount(selectedPreview);
    if (selectedItem.intakeMode !== "standard" || safeCount === 0) {
      return;
    }

    if (!skipConfirm) {
      setPendingDialog({ kind: "safe_move" });
      return;
    }

    setIsApplying(true);
    setStatusMessage(null);
    setErrorMessage(null);

    try {
      const result = await api.applyDownloadItem(
        selectedItem.id,
        selectedPreset,
        true,
      );
      setStatusMessage(
        `Moved ${result.movedCount} safe file(s) from ${selectedItem.displayName}. ${result.deferredReviewCount} file(s) stayed in the inbox.`,
      );
      setUndoableApply({ itemId: selectedItem.id, displayName: selectedItem.displayName });
      onDataChanged();
      await reloadInboxAfterMutation();
    } catch (error) {
      setErrorMessage(toErrorMessage(error));
    } finally {
      setIsApplying(false);
    }
  }

  async function handleReject(skipConfirm = false) {
    if (!selectedItem) {
      return;
    }

    if (!skipConfirm) {
      setPendingDialog({ kind: "reject" });
      return;
    }

    setIsRejecting(true);
    setStatusMessage(null);
    setErrorMessage(null);

    try {
      const result = await api.rejectDownloadItem(selectedItem.id);
      setStatusMessage(`${selectedItem.displayName} was moved to SimSuite_Rejected.`);
      onDataChanged();
      await reloadInboxAfterMutation();
    } catch (error) {
      setErrorMessage(toErrorMessage(error));
    } finally {
      setIsRejecting(false);
    }
  }

  async function handleSnooze(durationSeconds: number) {
    if (!selectedItem) return;
    setStatusMessage(null);
    setErrorMessage(null);
    try {
      await api.snoozeDownloadItem(selectedItem.id, durationSeconds);
      setStatusMessage(`${selectedItem.displayName} snoozed. It will reappear later.`);
      onDataChanged();
      await reloadInboxAfterMutation();
    } catch (error) {
      setErrorMessage(toErrorMessage(error));
    }
  }

  async function handleBatchApply(skipConfirm = false) {
    if (batchSelectedIds.size === 0) return;
    if (!skipConfirm) {
      setPendingDialog({ kind: "batch_apply", count: batchSelectedIds.size });
      return;
    }
    setIsApplying(true);
    setStatusMessage(null);
    setErrorMessage(null);
    try {
      const itemIds = Array.from(batchSelectedIds);
      const result = await api.applyDownloadItems(itemIds, selectedPreset, true);
      const parts: string[] = [];
      if (result.appliedCount > 0) parts.push(`${result.appliedCount} applied`);
      if (result.skippedCount > 0) parts.push(`${result.skippedCount} skipped`);
      if (result.failedCount > 0) parts.push(`${result.failedCount} failed`);
      setStatusMessage(`Batch apply: ${parts.join(", ")}.`);
      if (result.errors.length > 0) {
        setErrorMessage(result.errors.slice(0, 3).join("; "));
      }
      setBatchSelectedIds(new Set());
      onDataChanged();
      await reloadInboxAfterMutation();
    } catch (error) {
      setErrorMessage(toErrorMessage(error));
    } finally {
      setIsApplying(false);
    }
  }

  async function handleBatchReject(skipConfirm = false) {
    if (batchSelectedIds.size === 0) return;
    if (!skipConfirm) {
      setPendingDialog({ kind: "batch_reject", count: batchSelectedIds.size });
      return;
    }
    setIsRejecting(true);
    setStatusMessage(null);
    setErrorMessage(null);
    try {
      const itemIds = Array.from(batchSelectedIds);
      const result = await api.rejectDownloadItems(itemIds);
      const parts: string[] = [];
      if (result.ignoredCount > 0) parts.push(`${result.ignoredCount} rejected`);
      if (result.failedCount > 0) parts.push(`${result.failedCount} failed`);
      setStatusMessage(`Batch reject: ${parts.join(", ")}.`);
      if (result.errors.length > 0) {
        setErrorMessage(result.errors.slice(0, 3).join("; "));
      }
      setBatchSelectedIds(new Set());
      onDataChanged();
      await reloadInboxAfterMutation();
    } catch (error) {
      setErrorMessage(toErrorMessage(error));
    } finally {
      setIsRejecting(false);
    }
  }

  function handlePrimaryAction() {
    if (!selectedResolvedItem) {
      return;
    }

    if (primaryReviewAction) {
      if (reviewActionNeedsApproval(primaryReviewAction.kind)) {
        setPendingDialog({
          kind: "review_action",
          action: primaryReviewAction,
        });
        return;
      }

      void handleReviewAction(primaryReviewAction);
      return;
    }

    if (effectiveSelectedIntakeMode === "guided") {
      if (!selectedGuidedPlan) {
        void handleApply();
        return;
      }

      setPendingDialog({ kind: "guided_apply" });
      return;
    }

    if (effectiveSelectedIntakeMode === "standard" && safeCount > 0) {
      setPendingDialog({ kind: "safe_move" });
    }
  }

  async function handleConfirmDialog() {
    if (!pendingDialog) {
      return;
    }

    switch (pendingDialog.kind) {
      case "guided_apply":
      case "safe_move":
        await handleApply(true);
        break;
      case "reject":
        await handleReject(true);
        break;
      case "review_action":
        await handleReviewAction(pendingDialog.action, true);
        break;
      case "undo":
        await handleUndo(pendingDialog.itemId, pendingDialog.displayName);
        break;
      case "batch_apply":
        await handleBatchApply(true);
        break;
      case "batch_reject":
        await handleBatchReject(true);
        break;
      default:
        break;
    }

    setPendingDialog(null);
  }

  async function handleUndo(itemId: number, displayName: string) {
    setIsUndoing(true);
    setStatusMessage(null);
    setErrorMessage(null);
    try {
      await api.undoAppliedItem(itemId);
      setStatusMessage(`"${displayName}" was moved back to Downloads.`);
      setUndoableApply(null);
      onDataChanged();
      await reloadInboxAfterMutation();
    } catch (error) {
      setErrorMessage(toErrorMessage(error));
    } finally {
      setIsUndoing(false);
    }
  }

  const overview = inbox?.overview ?? null;
  const activeSelection =
    selectedItem && selectionState.itemId === selectedItem.id ? selectionState : null;
  const selectedResolvedItem = activeSelection?.detail?.item ?? selectedItem;
  const selectedFiles = activeSelection?.detail?.files ?? [];
  const selectedSpecialDecision =
    activeSelection?.detail?.item.specialDecision ?? selectedItem?.specialDecision ?? null;
  const selectedVersionResolution =
    activeSelection?.detail?.item.versionResolution ??
    selectedItem?.versionResolution ??
    null;
  const selectedStateBadge = selectedResolvedItem
    ? primaryInboxStateBadge(selectedResolvedItem, userView)
    : null;
  const selectedPreview = activeSelection?.preview ?? null;
  const selectedGuidedPlan = activeSelection?.guidedPlan ?? null;
  const selectedReviewPlan = activeSelection?.reviewPlan ?? null;
  const previewSuggestions = selectedPreview?.suggestions ?? [];
  const safeCount = actionableCount(selectedPreview);
  const reviewCount =
    selectedResolvedItem?.intakeMode === "guided"
      ? selectedGuidedPlan?.reviewFiles.length ??
        selectedReviewPlan?.reviewFiles.length ??
        selectedResolvedItem?.reviewFileCount ??
        0
      : selectedResolvedItem?.intakeMode === "needs_review" ||
          selectedResolvedItem?.intakeMode === "blocked"
        ? selectedReviewPlan?.reviewFiles.length ?? selectedResolvedItem?.reviewFileCount ?? 0
      : selectedPreview?.reviewCount ?? selectedResolvedItem?.reviewFileCount ?? 0;
  const unchangedCount = alignedCount(selectedPreview);
  const activeQueueRows: DownloadsQueueRowModel[] = activeLaneItems.map((item) => {
    const primaryBadge = primaryInboxStateBadge(item, userView);
    const rawBadges = [
      findAutoRecheckNote(item.notes)
        ? { label: "Rechecked", tone: "neutral" }
        : null,
      item.relatedItemIds?.length
        ? {
            label: `Linked ${item.relatedItemIds.length + 1}`,
            tone: "neutral",
          }
        : null,
      ...(primaryBadge
        ? [primaryBadge]
        : [
            {
              label: intakeModeLabel(item.intakeMode),
              tone: intakeModeTone(item.intakeMode),
            },
            {
              label: friendlyItemStatus(item.status),
              tone: itemStatusTone(item.status),
            },
          ]),
    ].filter((badge): badge is { label: string; tone: string } => badge !== null);
    const visibleLabels = capRowBadges(rawBadges.map((badge) => badge.label));
    const badges = rawBadges.filter(
      (badge, index) =>
        visibleLabels.includes(badge.label) &&
        rawBadges.findIndex((candidate) => candidate.label === badge.label) === index,
    );

    return {
      id: item.id,
      title: item.displayName,
      creatorName: item.creatorName,
      meta: `${item.sourceKind === "archive" ? "Archive" : "Direct file"} · ${item.detectedFileCount.toLocaleString()} file(s)${
        userView === "power" && item.archiveFormat
          ? ` · ${item.archiveFormat.toUpperCase()}`
          : ""
      }`,
      summary: item.queueSummary ?? fallbackQueueSummary(item),
      samples: item.sampleFiles.length ? item.sampleFiles.slice(0, 3).join(" · ") : null,
      badges,
      tone: inboxItemTone(item),
      selected: selectedItemId === item.id,
      batchSelected: batchSelectedIds.has(item.id),
      sourcePath: item.sourcePath,
      libraryVersionInfo:
        item.versionResolution?.status === "incoming_newer"
          ? {
              libraryLabel: item.versionResolution.matchedSubjectLabel,
              installedVersion: item.versionResolution.installedVersion,
              incomingVersion: item.versionResolution.incomingVersion,
            }
          : undefined,
    };
  });
  const batchCanvasPreviewItems = previewSuggestions.length
    ? previewSuggestions.slice(0, 4).map((item) => item.filename)
    : selectedFiles.slice(0, 4).map((file) => file.filename);
  const reviewActions = selectedSpecialDecision
    ? buildDecisionActions(selectedSpecialDecision, selectedReviewPlan)
    : selectedReviewPlan
      ? buildReviewActions(selectedReviewPlan)
      : [];
  const primaryReviewAction =
    selectedSpecialDecision?.primaryAction ?? reviewActions[0] ?? null;
  const batchCanvasSummary = selectedResolvedItem
    ? downloadsNextStepDescription(
        selectedResolvedItem,
        selectedGuidedPlan,
        selectedSpecialDecision,
        selectedVersionResolution,
        primaryReviewAction,
        safeCount,
        userView,
      )
    : queueLaneHint(resolvedActiveLane, userView);
  const guidedNeedsReview = Boolean(
    selectedResolvedItem?.intakeMode === "guided" &&
      (selectedSpecialDecision
        ? !selectedSpecialDecision.applyReady
        : selectedGuidedPlan && !selectedGuidedPlan.applyReady),
  );
  const incomingOlder =
    selectedSpecialDecision?.versionStatus === "incoming_older" ||
    selectedVersionResolution?.status === "incoming_older";
  const guidedActionReady = Boolean(
    selectedSpecialDecision?.applyReady || selectedGuidedPlan?.applyReady,
  );
  const effectiveSelectedIntakeMode: DownloadIntakeMode | undefined = selectedResolvedItem
    ? selectedResolvedItem.intakeMode === "guided" || guidedActionReady
      ? "guided"
      : selectedResolvedItem.intakeMode
    : selectedItem
      ? selectedItem.intakeMode === "guided" || guidedActionReady
        ? "guided"
        : selectedItem.intakeMode
      : undefined;
  const canApply =
    effectiveSelectedIntakeMode === "guided"
      ? guidedActionReady
      : effectiveSelectedIntakeMode === "standard" && safeCount > 0;
  const showPrimaryAction =
    Boolean(selectedResolvedItem) &&
    !incomingOlder &&
    (canApply || Boolean(primaryReviewAction));
  const applyLabel = selectedResolvedItem
    ? primaryReviewAction
      ? reviewActionLabel(primaryReviewAction, userView, isApplying)
      : applyButtonLabel(
          effectiveSelectedIntakeMode ?? selectedResolvedItem.intakeMode,
          selectedGuidedPlan,
          selectedSpecialDecision,
          userView,
          isApplying,
          selectedReviewPlan,
        )
    : userView === "beginner"
      ? "Move safe files"
      : "Apply safe batch";
  const selectedAutoRecheckNote = selectedResolvedItem
    ? findAutoRecheckNote(selectedResolvedItem.notes)
    : null;
  const primaryActionDisabled = primaryReviewAction
    ? isApplying
    : !canApply || isApplying;
  const nextStepTitle = selectedResolvedItem
    ? downloadsNextStepTitle(
        selectedResolvedItem,
        selectedGuidedPlan,
        selectedSpecialDecision,
        selectedVersionResolution,
        primaryReviewAction,
        canApply,
        safeCount,
        userView,
      )
    : null;
  const nextStepDescription = selectedResolvedItem
    ? downloadsNextStepDescription(
        selectedResolvedItem,
        selectedGuidedPlan,
        selectedSpecialDecision,
        selectedVersionResolution,
        primaryReviewAction,
        safeCount,
        userView,
      )
    : null;
  const inspectorSections = selectedResolvedItem
    ? buildInspectorSections({
        selectedItem: selectedResolvedItem,
        selectedFiles,
        selectedPreview,
        selectedGuidedPlan,
        selectedSpecialDecision,
        selectedVersionResolution,
        selectedReviewPlan,
        safeCount,
        reviewCount,
        unchangedCount,
        userView,
      })
    : [];
  const decisionBadges: DownloadsDecisionBadge[] = selectedResolvedItem
    ? selectedStateBadge
      ? [selectedStateBadge]
      : [
          {
            label: intakeModeLabel(selectedResolvedItem.intakeMode),
            tone: intakeModeTone(selectedResolvedItem.intakeMode),
          },
          {
            label: friendlyItemStatus(selectedResolvedItem.status),
            tone: itemStatusTone(selectedResolvedItem.status),
          },
        ]
    : [];
  const splitStage = userView !== "beginner";
  const inspectorSignals: DownloadsDecisionSignal[] = selectedResolvedItem
    ? buildDownloadInspectorSignals(
        selectedResolvedItem,
        selectedSpecialDecision,
        selectedVersionResolution,
        selectedReviewPlan,
        selectedAutoRecheckNote,
      )
    : [];
  const visibleInspectorSignals =
    userView === "beginner" ? inspectorSignals.slice(0, 2) : inspectorSignals;
  const decisionLaneLabel = selectedResolvedItem
    ? queueLaneLabel(
        selectedResolvedItem.queueLane ?? deriveQueueLane(selectedResolvedItem),
        userView,
      )
    : queueLaneLabel(resolvedActiveLane, userView);
  const decisionResolvedLane = selectedResolvedItem
    ? (selectedResolvedItem.queueLane ?? deriveQueueLane(selectedResolvedItem))
    : resolvedActiveLane;
  const proofSummary = selectedResolvedItem
    ? userView === "beginner"
      ? `Open ${inspectorSections.length.toLocaleString()} detail section(s) for files, versions, and the fuller safety notes.`
      : `See ${inspectorSections.length.toLocaleString()} proof section(s) for files, versions, source history, and the full evidence trail.`
    : userView === "beginner"
      ? "Pick a batch first, then open the fuller details here."
      : "Pick a batch first, then open the calmer proof sheet.";
  const dialogBusy =
    pendingDialog?.kind === "reject"
      ? isRejecting
      : pendingDialog?.kind === "undo"
        ? isUndoing
        : pendingDialog
          ? isApplying
          : false;
  // Batch dialogs don't need a selectedResolvedItem; use a dummy for the config builder
  const isBatchDialog =
    pendingDialog?.kind === "batch_apply" ||
    pendingDialog?.kind === "batch_reject";
  const dialogConfig =
    pendingDialog && (isBatchDialog || selectedResolvedItem)
      ? buildDownloadsDialogConfig({
          request: pendingDialog,
          item: selectedResolvedItem ?? ({
            id: 0,
            displayName: "",
            queueLane: null,
            detectedFileCount: 0,
          } as unknown as DownloadsInboxItem),
          guidedPlan: selectedGuidedPlan,
          reviewPlan: selectedReviewPlan,
          specialDecision: selectedSpecialDecision,
          safeCount,
          reviewCount,
          unchangedCount,
          userView,
          selectedPreset,
        })
      : null;
  const hasWatcherStatus = watcherStatus !== null;
  const showWatcherSetup = hasWatcherStatus && !watcherStatus.configured;
  const showWatcherBootstrap = !hasWatcherStatus
    ? true
    : watcherStatus.configured &&
      watcherStatus.state === "processing" &&
      inbox === null;
  const activeWatcherStatus = watcherStatus ?? {
    state: "idle",
    watchedPath: null,
    configured: false,
    currentItem: null,
    lastRunAt: null,
    lastChangeAt: null,
    lastError: null,
    readyItems: 0,
    needsReviewItems: 0,
    activeItems: 0,
    progress: null,
  };
  const stageStatusMessage =
    activeWatcherStatus.state === "processing"
      ? activeWatcherStatus.currentItem
        ? `Checking ${activeWatcherStatus.currentItem}`
        : "Checking the Downloads folder"
      : activeWatcherStatus.lastError
        ? activeWatcherStatus.lastError
        : activeWatcherStatus.lastRunAt
          ? `Last check ${formatDate(activeWatcherStatus.lastRunAt)}`
          : "Watcher ready";

  useEffect(() => {
    if (!selectedItem) {
      setProofSheetOpen(false);
      setPendingDialog(null);
      setUndoableApply(null);
    }
  }, [selectedItem]);

  useEffect(() => {
    function handleKeyDown(event: KeyboardEvent) {
      // Don't fire shortcuts when typing in an input
      const target = event.target as HTMLElement;
      if (
        target.tagName === "INPUT" ||
        target.tagName === "TEXTAREA" ||
        target.isContentEditable
      ) {
        return;
      }

      switch (event.key) {
        case "j":
        case "J":
        case "ArrowDown": {
          event.preventDefault();
          const currentIndex = activeLaneItems.findIndex(
            (item) => item.id === selectedItemId,
          );
          const nextIndex =
            currentIndex < activeLaneItems.length - 1
              ? currentIndex + 1
              : 0;
          setSelectedItemId(activeLaneItems[nextIndex]?.id ?? null);
          break;
        }
        case "k":
        case "K":
        case "ArrowUp": {
          event.preventDefault();
          const currentIndex = activeLaneItems.findIndex(
            (item) => item.id === selectedItemId,
          );
          const prevIndex =
            currentIndex > 0 ? currentIndex - 1 : activeLaneItems.length - 1;
          setSelectedItemId(activeLaneItems[prevIndex]?.id ?? null);
          break;
        }
        case "Enter": {
          // Toggle inspector open/closed for the selected item
          if (selectedItem) {
            if (!proofSheetOpen) {
              setProofSheetOpen(true);
            }
          }
          break;
        }
        case "a":
        case "A": {
          event.preventDefault();
          if (selectedItem) {
            void handleApply();
          }
          break;
        }
        case "i":
        case "I": {
          event.preventDefault();
          if (selectedItem) {
            void handleReject();
          }
          break;
        }
        case "r":
        case "R": {
          event.preventDefault();
          void handleRefresh();
          break;
        }
        case "?": {
          event.preventDefault();
          setShowShortcutsHelp(true);
          break;
        }
        case "Escape": {
          if (pendingDialog) {
            if (!dialogBusy) {
              setPendingDialog(null);
            }
            return;
          }
          if (proofSheetOpen) {
            setProofSheetOpen(false);
            return;
          }
          if (filtersOpen) {
            setFiltersOpen(false);
            return;
          }
          if (showShortcutsHelp) {
            setShowShortcutsHelp(false);
          }
          break;
        }
      }
    }

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [
    activeLaneItems,
    dialogBusy,
    filtersOpen,
    pendingDialog,
    proofSheetOpen,
    selectedItem,
    selectedItemId,
    showShortcutsHelp,
  ]);

  return (
    <section className={`screen-shell downloads-shell${batchSelectedIds.size > 0 ? " downloads-shell-batch-active" : ""}`}>
      <DownloadsTopStrip
        statusMessage={statusMessage}
        errorMessage={errorMessage}
        totalItems={statusFilter ? (inbox?.items.length ?? 0) : (overview?.totalItems ?? 0)}
        readyCount={overview?.readyNowItems ?? overview?.readyItems ?? 0}
        waitingCount={overview?.waitingOnYouItems ?? overview?.needsReviewItems ?? 0}
        specialSetupCount={overview?.specialSetupItems ?? 0}
        blockedCount={overview?.blockedItems ?? overview?.errorItems ?? 0}
        lastCheckLabel={stageStatusMessage}
        isRefreshing={isRefreshing}
        isLoading={isLoadingInbox}
        reviewActionLabel={reviewLabel(userView)}
        onRefresh={() => void handleRefresh()}
        onOpenReview={() => onNavigate("review")}
        onLaneChange={(lane) => {
          setStatusMessage(null);
          setActiveLane(lane);
        }}
        userView={userView}
        statusFilter={statusFilter}
        onClearFilter={statusFilter ? () => setStatusFilter("") : undefined}
        progress={progress}
        undoableApply={undoableApply}
        onRequestUndo={() => {
          if (undoableApply) {
            setPendingDialog({
              kind: "undo",
              itemId: undoableApply.itemId,
              displayName: undoableApply.displayName,
            });
          }
        }}
        isUndoing={isUndoing}
      />

      {showWatcherBootstrap ? (
        <StatePanel
          eyebrow="Downloads inbox"
          title="Checking your Downloads inbox..."
          body={
            userView === "beginner"
              ? "SimSuite is checking your Downloads folder and lining up the latest items."
              : "SimSuite is checking the watcher state and loading the latest inbox queue."
          }
          icon={LoaderCircle}
          tone="info"
          actions={
            <button
              type="button"
              className="secondary-action"
              onClick={() => void handleRefresh()}
              disabled={isRefreshing || isLoadingInbox}
            >
              <RefreshCw size={14} strokeWidth={2} />
              Check again
            </button>
          }
          meta={["Inbox stays read-only until the first check finishes"]}
        />
      ) : showWatcherSetup ? (
        <StatePanel
          eyebrow="Downloads folder"
          title={
            userView === "beginner"
              ? "Choose a Downloads folder first"
              : "Downloads watcher is not configured"
          }
          body={
            userView === "beginner"
              ? "Set one inbox folder on Home so SimSuite can check new files safely before they touch your game."
              : "Point SimSuite at a Downloads inbox before using archive intake, guided setup, or safe hand-off previews."
          }
          icon={FolderSearch}
          tone="warn"
          actions={
            <button
              type="button"
              className="primary-action"
              onClick={() => onNavigate("home")}
            >
              Go to Home
            </button>
          }
          meta={["No watcher path", "Nothing moves from this screen automatically"]}
        />
      ) : (
        <Workbench threePanel className="downloads-workbench">
          <WorkbenchRail
            ariaLabel="Downloads controls"
            className="downloads-rail-shell"
            noBorder
          >
            <DownloadsRail
              userView={userView}
              watcherLabel={friendlyWatcherLabel(activeWatcherStatus.state)}
              watchedPath={activeWatcherStatus.watchedPath}
              lastCheckLabel={
                activeWatcherStatus.lastRunAt
                  ? `Last check ${formatDate(activeWatcherStatus.lastRunAt)}`
                  : "Watcher ready"
              }
              activeItemsLabel={`${activeWatcherStatus.activeItems.toLocaleString()} active item(s)`}
              currentItemLabel={activeWatcherStatus.currentItem}
              activeLane={resolvedActiveLane}
              laneCounts={visibleLaneCounts}
              search={search}
              statusFilter={statusFilter}
              selectedPreset={selectedPreset}
              presetOptions={presets.map((preset) => preset.name)}
              filtersOpen={filtersOpen}
              filterLocked={selectedItem?.intakeMode === "guided"}
              onLaneChange={(lane) => {
                setStatusMessage(null);
                setActiveLane(lane);
              }}
              onSearchChange={setSearch}
              onStatusFilterChange={setStatusFilter}
              onPresetChange={setSelectedPreset}
              onToggleFilters={() => {
                setFiltersOpen((current) => !current);
              }}
            />
          </WorkbenchRail>

          <WorkbenchStage className="downloads-stage-panel">
            <div className="table-header downloads-stage-header">
              <div className="health-chip-group downloads-stage-metrics">
                <span className="health-chip">
                  {queueLaneLabel(resolvedActiveLane, userView)}
                </span>
                {selectedItem ? (
                  <span className="health-chip is-good">
                    1 selected
                  </span>
                ) : (
                  <span className="health-chip downloads-stage-hint" title="Select a row below to inspect">
                    ← Select an item
                  </span>
                )}
                {userView !== "beginner" ? (
                  <span className="health-chip">
                    {userView === "power" ? "Rule" : "Tidy"}: {selectedPreset}
                  </span>
                ) : null}
              </div>
              <div className="downloads-stage-status">
                {activeWatcherStatus.state === "processing" ? (
                  <span className="health-chip is-warn downloads-stage-processing">
                    {activeWatcherStatus.currentItem
                      ? `Checking ${activeWatcherStatus.currentItem}`
                      : "Checking the Downloads folder"}
                  </span>
                ) : activeWatcherStatus.lastError ? (
                  <span className="health-chip is-danger">{activeWatcherStatus.lastError}</span>
                ) : (
                  <span className="ghost-chip downloads-stage-idle">
                    {activeWatcherStatus.lastRunAt
                      ? `Last check ${formatDate(activeWatcherStatus.lastRunAt)}`
                      : "Watcher ready"}
                  </span>
                )}
              </div>
            </div>

            <div className={`downloads-stage${splitStage ? " downloads-stage-split" : ""}`}>
              <DownloadsQueuePanel
                lane={resolvedActiveLane}
                userView={userView}
                rows={activeQueueRows}
                isLoading={isLoadingInbox}
                hasItems={Boolean(inbox?.items.length)}
                onSelect={(itemId) => {
                  setStatusMessage(null);
                  setSelectedItemId(itemId);
                }}
                onToggleBatchSelect={(id) => {
                  setBatchSelectedIds((prev) => {
                    const next = new Set(prev);
                    if (next.has(id)) {
                      next.delete(id);
                    } else {
                      next.add(id);
                    }
                    return next;
                  });
                }}
                onSelectAll={() => {
                  setBatchSelectedIds(new Set(activeLaneItems.map((item) => item.id)));
                }}
                onClearSelection={() => setBatchSelectedIds(new Set())}
                selectedCount={batchSelectedIds.size}
                onOpenInLibrary={() => onNavigate("library")}
                footer={
                  splitStage ? (
                    <div className="downloads-queue-footer-card">
                      <p className="eyebrow">
                        {userView === "power" ? "Desk rhythm" : "Keep the queue simple"}
                      </p>
                      <strong>
                        {userView === "power"
                          ? "Queue first, receipts second"
                          : "Scan here, act in the middle"}
                      </strong>
                      <p>
                        {userView === "power"
                          ? "Rows stay short on purpose. Keep the action in the inspector and open proof only when you want the full receipt trail."
                          : "Select an item and decide in the preview. Open proof only when you want the full receipt trail."}
                      </p>
                    </div>
                  ) : (
                    <ResizableEdgeHandle
                      label="Resize download queue height"
                      value={downloadsQueueHeight}
                      min={220}
                      max={720}
                      onChange={setDownloadsQueueHeight}
                      side="bottom"
                      className="dock-resize-handle downloads-queue-height-handle"
                    />
                  )
                }
              />
              {/* In split mode (standard/power), show batch canvas inline. */}
              {/* In casual mode, batch canvas is hidden from stage flow and */}
              {/* shown as a right-side drawer instead (see below). */}
              {splitStage && (
                <DownloadsBatchCanvas
                  lane={resolvedActiveLane}
                  userView={userView}
                  selectionTitle={selectedItem?.displayName ?? null}
                  summary={batchCanvasSummary}
                  safeCount={safeCount}
                  reviewCount={reviewCount}
                  unchangedCount={unchangedCount}
                  previewItems={batchCanvasPreviewItems}
                >
                  <AnimatePresence mode="wait" initial={false}>
                    <m.div
                      key={`${resolvedActiveLane}-${selectedItem?.id ?? "empty"}-${isLoadingSelection ? "loading" : "ready"}`}
                      className="downloads-batch-stage"
                      initial={{ opacity: 0, y: 8 }}
                      animate={{ opacity: 1, y: 0 }}
                      exit={{ opacity: 0, y: -6 }}
                      transition={downloadsSelectionTransition}
                    >
                    {isLoadingSelection ? (
                      <StatePanel
                        eyebrow="Preview"
                        title="Loading batch details"
                        body="SimSuite is checking the selected download and preparing the safest next step."
                        icon={LoaderCircle}
                        tone="info"
                        compact
                        badge="Working"
                      />
                    ) : selectedItem?.intakeMode === "guided" ? (
                      selectedGuidedPlan ? (
                        guidedNeedsReview && selectedReviewPlan ? (
                          <SpecialReviewPanel
                            item={selectedItem}
                            reviewPlan={selectedReviewPlan}
                            files={selectedFiles}
                            userView={userView}
                            reviewActions={reviewActions}
                            onResolveAction={handleReviewAction}
                            isApplying={isApplying}
                          />
                        ) : (
                        <GuidedPreviewPanel plan={selectedGuidedPlan} userView={userView} />
                        )
                      ) : (
                        <StatePanel
                          eyebrow="Special setup"
                          title="Guided plan not ready yet"
                          body="SimSuite recognized a special setup item, but the install plan is not ready. Refresh the inbox and try again."
                          icon={AlertTriangle}
                          tone="warn"
                          compact
                        />
                      )
                    ) : selectedItem &&
                      (selectedItem.intakeMode === "needs_review" ||
                        selectedItem.intakeMode === "blocked") ? (
                      selectedReviewPlan ? (
                        <SpecialReviewPanel
                          item={selectedItem}
                          reviewPlan={selectedReviewPlan}
                          files={selectedFiles}
                          userView={userView}
                          reviewActions={reviewActions}
                          onResolveAction={handleReviewAction}
                          isApplying={isApplying}
                        />
                      ) : (
                        <StatePanel
                          eyebrow={intakeModeLabel(selectedItem.intakeMode)}
                          title={
                            selectedItem.intakeMode === "blocked"
                              ? "Blocked details are not ready yet"
                              : "Review details are not ready yet"
                          }
                          body="SimSuite recognized a special case, but the review plan is not ready. Refresh the inbox and try again."
                          icon={AlertTriangle}
                          tone="warn"
                          compact
                        />
                      )
                    ) : previewSuggestions.length ? (
                      <StandardPreviewPanel
                        suggestions={previewSuggestions}
                        safeCount={safeCount}
                        reviewCount={reviewCount}
                        unchangedCount={unchangedCount}
                        userView={userView}
                      />
                    ) : selectedFiles.length ? (
                      <TrackedFilesPanel files={selectedFiles} userView={userView} />
                    ) : (
                      <StatePanel
                        eyebrow="Preview"
                        title="Select a download item to inspect"
                        body="Select a staged archive or file batch to load the correct inbox preview."
                        icon={Download}
                        compact
                        meta={["Normal", "Special setup", "Needs review", "Blocked"]}
                      />
                    )}
                    </m.div>
                  </AnimatePresence>
                </DownloadsBatchCanvas>
              )}
            </div>
          </WorkbenchStage>

          {/* Only show the inspector in standard/power mode. */}
          {/* In casual mode, the decision drawer (above) handles item decisions.
              In seasoned mode, the preview panel (right) is the decision surface.
              In creator mode, the inspector stays visible for audit depth. */}
          {userView === "power" && (
            <WorkbenchInspector
              ariaLabel="Downloads inbox details"
              width={downloadsDetailWidth}
              onWidthChange={setDownloadsDetailWidth}
              minWidth={320}
              maxWidth={780}
              className="downloads-inspector-shell"
              noBorder
            >
              {selectedItem ? (
                <DownloadsDecisionPanel
                  userView={userView}
                  title={selectedItem.displayName}
                  summary={selectedItem.queueSummary ?? fallbackQueueSummary(selectedItem)}
                  laneLabel={decisionLaneLabel}
                  resolvedLane={decisionResolvedLane}
                  badges={decisionBadges}
                  signals={visibleInspectorSignals}
                  nextStepTitle={nextStepTitle}
                  nextStepDescription={nextStepDescription}
                  primaryActionLabel={showPrimaryAction ? applyLabel : null}
                  primaryActionDisabled={primaryActionDisabled}
                  onPrimaryAction={showPrimaryAction ? handlePrimaryAction : undefined}
                  secondaryActionLabel={isRejecting ? "Rejecting..." : "Reject"}
                  secondaryActionDisabled={isRejecting}
                  onSecondaryAction={() => setPendingDialog({ kind: "reject" })}
                  onOpenProof={() => setProofSheetOpen(true)}
                  proofSummary={proofSummary}
                  onSnooze={handleSnooze}
                  idleNote={
                    !showPrimaryAction
                      ? downloadsInspectorIdleNote(
                          effectiveSelectedIntakeMode ?? selectedItem.intakeMode,
                          userView,
                          safeCount,
                          selectedGuidedPlan,
                          selectedSpecialDecision,
                          selectedVersionResolution,
                          selectedReviewPlan,
                        )
                      : null
                  }
                />
              ) : (
                <StatePanel
                  eyebrow="Inbox"
                  title="Select an inbox item to inspect"
                  body="The inspector shows intake mode, evidence, and the file set for the selected batch."
                  icon={Download}
                  meta={["Approval first", "Snapshots happen before moves"]}
                />
              )}
            </WorkbenchInspector>
          )}

          {/* Casual mode: decision drawer — slides in from right, overlay on top of the stage.
              Unlike the split layout (which divides space between queue and detail),
              this overlay keeps the queue fully visible and focused. */}
          <AnimatePresence>
            {userView === "beginner" && selectedItem && (
              <m.div
                className="downloads-casual-drawer"
                initial={{ x: "100%" }}
                animate={{ x: 0 }}
                exit={{ x: "100%" }}
                transition={{ type: "spring", stiffness: 320, damping: 34 }}
                aria-label="Item decision panel"
              >
                <div className="downloads-casual-drawer-header">
                  <div className="downloads-casual-drawer-title-group">
                    <p className="eyebrow">Inbox item</p>
                    <h2 className="downloads-casual-drawer-title">
                      {selectedItem.displayName}
                    </h2>
                  </div>
                  <button
                    type="button"
                    className="downloads-casual-drawer-close"
                    onClick={() => setSelectedItemId(null)}
                    aria-label="Close and return to queue"
                  >
                    <X size={16} strokeWidth={2} />
                  </button>
                </div>

                <div className="downloads-casual-drawer-content">
                  <DownloadsDecisionPanel
                    userView={userView}
                    title={selectedItem.displayName}
                    summary={selectedItem.queueSummary ?? fallbackQueueSummary(selectedItem)}
                    laneLabel={decisionLaneLabel}
                    resolvedLane={decisionResolvedLane}
                    badges={decisionBadges}
                    signals={visibleInspectorSignals}
                    nextStepTitle={nextStepTitle}
                    nextStepDescription={nextStepDescription}
                    primaryActionLabel={showPrimaryAction ? applyLabel : null}
                    primaryActionDisabled={primaryActionDisabled}
                    onPrimaryAction={showPrimaryAction ? handlePrimaryAction : undefined}
                    secondaryActionLabel={isRejecting ? "Rejecting..." : "Reject"}
                    secondaryActionDisabled={isRejecting}
                    onSecondaryAction={() => setPendingDialog({ kind: "reject" })}
                    onOpenProof={() => setProofSheetOpen(true)}
                    proofSummary={proofSummary}
                    onSnooze={handleSnooze}
                    idleNote={
                      !showPrimaryAction
                        ? downloadsInspectorIdleNote(
                            effectiveSelectedIntakeMode ?? selectedItem.intakeMode,
                            userView,
                            safeCount,
                            selectedGuidedPlan,
                            selectedSpecialDecision,
                            selectedVersionResolution,
                            selectedReviewPlan,
                          )
                        : null
                    }
                  />
                </div>
              </m.div>
            )}
          </AnimatePresence>

          {/* Casual mode: dark backdrop when drawer is open — keeps focus on the queue context */}
          <AnimatePresence>
            {userView === "beginner" && selectedItem && (
              <m.div
                className="downloads-casual-backdrop"
                initial={{ opacity: 0 }}
                animate={{ opacity: 1 }}
                exit={{ opacity: 0 }}
                transition={{ duration: 0.2 }}
                onClick={() => setSelectedItemId(null) }
                aria-hidden="true"
              />
            )}
          </AnimatePresence>
        </Workbench>
      )}

      {selectedItem ? (
        <DownloadsProofSheet
          open={proofSheetOpen}
          onClose={() => setProofSheetOpen(false)}
          title={selectedItem.displayName}
          summary={selectedItem.queueSummary ?? fallbackQueueSummary(selectedItem)}
          laneLabel={decisionLaneLabel}
          badges={decisionBadges}
          signals={visibleInspectorSignals}
          sections={inspectorSections}
          userView={userView}
        />
      ) : null}

      <DownloadsSetupDialog
        open={Boolean(dialogConfig)}
        onClose={() => setPendingDialog(null)}
        onConfirm={() => void handleConfirmDialog()}
        eyebrow={dialogConfig?.eyebrow ?? "Confirm"}
        title={dialogConfig?.title ?? "Continue?"}
        description={dialogConfig?.description ?? ""}
        confirmLabel={dialogConfig?.confirmLabel ?? "Continue"}
        tone={dialogConfig?.tone ?? "accent"}
        metrics={dialogConfig?.metrics ?? []}
        notes={dialogConfig?.notes ?? []}
        isWorking={dialogBusy}
      />

      {batchSelectedIds.size > 0 && (
        <div className="downloads-batch-action-bar">
          <span className="downloads-batch-count">
            {batchSelectedIds.size} selected
          </span>
          <button
            type="button"
            className="primary-action"
            onClick={() => void handleBatchApply()}
            disabled={isApplying}
          >
            {isApplying ? "Applying..." : `Apply ${batchSelectedIds.size}`}
          </button>
          <button
            type="button"
            className="secondary-action"
            onClick={() => void handleBatchReject()}
            disabled={isRejecting}
          >
            {isRejecting ? "Rejecting..." : "Reject"}
          </button>
          <button
            type="button"
            className="ghost-chip"
            onClick={() => setBatchSelectedIds(new Set())}
          >
            Clear
          </button>
        </div>
      )}

      {showTour && (
        <CasualGuidedTours onComplete={() => setShowTour(false)} />
      )}
      {showShortcutsHelp && (
        <KeyboardShortcutsDialog
          isOpen={showShortcutsHelp}
          onClose={() => setShowShortcutsHelp(false)}
        />
      )}

      {/* First-time keyboard shortcut hint — shown once, auto-dismissed after 8s */}
      <AnimatePresence>
        {showKeyboardHint && (
          <m.div
            className="downloads-keyboard-hint"
            role="note"
            initial={{ opacity: 0, y: 12, scale: 0.97 }}
            animate={{ opacity: 1, y: 0, scale: 1 }}
            exit={{ opacity: 0, y: 8, scale: 0.97 }}
            transition={{ duration: 0.25, ease: "easeOut" }}
          >
            <span className="downloads-keyboard-hint-text">
              {userView === "beginner"
                ? <>Press <kbd>?</kbd> for keyboard shortcuts</>
                : <>Press <kbd>?</kbd> or <kbd>⌘K</kbd> for shortcuts</>}
            </span>
            <button
              type="button"
              className="downloads-keyboard-hint-close"
              onClick={() => {
                setShowKeyboardHint(false);
                setKeyboardHintDismissed();
              }}
              aria-label="Dismiss shortcut hint"
            >
              <X size={12} strokeWidth={2.5} />
            </button>
          </m.div>
        )}
      </AnimatePresence>
    </section>
  );
}

function buildDownloadsDialogConfig({
  request,
  item,
  guidedPlan,
  reviewPlan,
  specialDecision,
  safeCount,
  reviewCount,
  unchangedCount,
  userView,
  selectedPreset,
}: {
  request: DownloadsDialogRequest;
  item: DownloadsInboxItem;
  guidedPlan: GuidedInstallPlan | null;
  reviewPlan: SpecialReviewPlan | null;
  specialDecision: SpecialModDecision | null;
  safeCount: number;
  reviewCount: number;
  unchangedCount: number;
  userView: UserView;
  selectedPreset: string;
}) {
  if (request.kind === "guided_apply" && guidedPlan) {
    const isSameVersion = specialDecision?.sameVersion ?? false;

    return {
      eyebrow: userView === "beginner" ? "Safe setup" : "Guided setup",
      title: isSameVersion
        ? `Reinstall ${guidedPlan.profileName}?`
        : `Install ${guidedPlan.profileName} safely?`,
      description: isSameVersion
        ? "SimSuite matched this download against the installed copy and found the same version. Reinstall only if you want to refresh the current setup cleanly."
        : "SimSuite has a guided install plan ready for this special mod. It will follow the matched setup path instead of dropping files in blind.",
      confirmLabel: isSameVersion ? "Reinstall safely" : "Start safe install",
      tone: "accent" as const,
      metrics: [
        { label: "Replace", value: guidedPlan.replaceFiles.length.toLocaleString() },
        { label: "Keep", value: guidedPlan.preserveFiles.length.toLocaleString() },
        { label: "Review later", value: guidedPlan.reviewFiles.length.toLocaleString() },
      ],
      notes: [
        "A restore point is created first.",
        "Saved settings and side files stay in place when they are meant to be kept.",
      ],
    };
  }

  if (request.kind === "safe_move") {
    return {
      eyebrow: userView === "beginner" ? "Safe hand-off" : "Move safe files",
      title: `Move the safe files from ${item.displayName}?`,
      description:
        userView === "beginner"
          ? "Only the part SimSuite already trusts will move now. Anything that still needs review stays in the inbox."
          : "This keeps the queue calm by moving only the validated files and leaving the uncertain rows behind for later review.",
      confirmLabel: userView === "beginner" ? "Move safe files" : "Apply safe batch",
      tone: "accent" as const,
      metrics: [
        { label: "Safe now", value: safeCount.toLocaleString() },
        { label: "Held for review", value: reviewCount.toLocaleString() },
        { label: "Already fine", value: unchangedCount.toLocaleString() },
      ],
      notes: ["A restore point is created before files move."],
    };
  }

  if (request.kind === "reject") {
    return {
      eyebrow: "Queue cleanup",
      title: `Reject ${item.displayName}?`,
      description:
        userView === "beginner"
          ? "This removes the batch from the active inbox for now. It can come back later if the download changes."
          : "Use this to quiet a batch without deleting it. SimSuite can surface it again if the source changes later.",
      confirmLabel: "Reject",
      tone: "warn" as const,
      metrics: [
        { label: "Files in batch", value: item.detectedFileCount.toLocaleString() },
        {
          label: "Current lane",
          value: queueLaneLabel(item.queueLane ?? deriveQueueLane(item), userView),
        },
      ],
      notes: ["This only hides the batch from the active queue."],
    };
  }

  if (request.kind === "undo") {
    return {
      eyebrow: "Undo apply",
      title: `Move "${request.displayName}" back to Downloads?`,
      description:
        userView === "beginner"
          ? "This will remove the files from your Library and put them back in the Downloads queue. You can apply them again later."
          : "This will move the files from the Library back to the staging area and reset the item to the waiting queue.",
      confirmLabel: "Yes, move back",
      tone: "warn" as const,
      metrics: [],
      notes: [
        "The files will be removed from your Library.",
        "The item will go back to the waiting queue.",
      ],
    };
  }

  if (request.kind === "batch_apply") {
    return {
      eyebrow: "Batch apply",
      title: `Apply ${request.count} item${request.count === 1 ? "" : "s"}?`,
      description:
        userView === "beginner"
          ? "All selected items will be applied using the current preset. This may replace some installed mods."
          : "All selected items will be applied. Review the per-item results after completion.",
      confirmLabel: `Apply ${request.count}`,
      tone: "accent" as const,
      metrics: [
        { label: "Items", value: request.count.toLocaleString() },
        { label: "Preset", value: selectedPreset },
      ],
      notes: ["A restore point is created before files are moved."],
    };
  }

  if (request.kind === "batch_reject") {
    return {
      eyebrow: "Batch reject",
      title: `Reject ${request.count} item${request.count === 1 ? "" : "s"}?`,
      description:
        userView === "beginner"
          ? "All selected items will be moved to the rejected queue. They can be recovered later if needed."
          : "All selected items will be moved to the rejected queue and hidden from the active inbox.",
      confirmLabel: `Reject ${request.count}`,
      tone: "warn" as const,
      metrics: [{ label: "Items", value: request.count.toLocaleString() }],
      notes: ["Rejected items can be recovered from the Rejected lane."],
    };
  }

  if (request.kind !== "review_action") {
    return {
      eyebrow: "Confirm step",
      title: "Continue?",
      description: "SimSuite is ready to continue with the selected batch.",
      confirmLabel: "Continue",
      tone: "accent" as const,
      metrics: [],
      notes: [],
    };
  }

  const action = request.action;

  if (action.kind === "repair_special" && reviewPlan) {
    return {
      eyebrow: "Repair first",
      title: action.label,
      description:
        userView === "beginner"
          ? "SimSuite can clear the older setup out of the way, keep your saved files, and then continue the update safely."
          : "A safe repair path is ready. SimSuite will move the older layout aside, keep the settings files, and then continue the update if the batch still checks out.",
      confirmLabel: action.label,
      tone: "warn" as const,
      metrics: [
        { label: "Move aside", value: reviewPlan.repairMoveFiles.length.toLocaleString() },
        { label: "Keep", value: reviewPlan.repairKeepFiles.length.toLocaleString() },
        { label: "Replace", value: reviewPlan.repairReplaceFiles.length.toLocaleString() },
      ],
      notes: [
        "A restore point is created first.",
        reviewPlan.repairCanContinueInstall
          ? "If the pack is complete, SimSuite can continue the install in the same safe run."
          : "SimSuite will repair first, then check the batch again.",
      ],
    };
  }

  return {
    eyebrow: "Confirm step",
    title: action.label,
    description: reviewActionDescription(action),
    confirmLabel: action.label,
    tone: "warn" as const,
    metrics: [
      {
        label: "Matched setup",
        value: item.matchedProfileName ?? reviewPlan?.profileName ?? "Not matched",
      },
      { label: "Review files", value: reviewCount.toLocaleString() },
    ],
    notes: ["SimSuite keeps the batch in the inbox until the safe next step is complete."],
  };
}

interface DownloadsStageTab {
  id: string;
  label: string;
  count?: number;
  content: ReactNode;
}

function DownloadsStageDeck({
  deckId,
  eyebrow,
  title,
  summary,
  defaultTabId,
  tabs,
}: {
  deckId: string;
  eyebrow: string;
  title: string;
  summary?: string;
  defaultTabId?: string;
  tabs: DownloadsStageTab[];
}) {
  const availableTabs = tabs.filter((tab) => Boolean(tab.content));
  const fallbackTabId = defaultTabId ?? availableTabs[0]?.id ?? null;
  const [activeTabId, setActiveTabId] = useState<string | null>(fallbackTabId);
  const resolvedActiveTab =
    availableTabs.find((tab) => tab.id === activeTabId) ?? availableTabs[0] ?? null;

  if (!resolvedActiveTab) {
    return null;
  }

  return (
    <div className="downloads-stage-deck">
      <div className="downloads-stage-deck-header">
        <div className="downloads-stage-deck-copy">
          <p className="eyebrow">{eyebrow}</p>
          <strong>{title}</strong>
          {summary ? <p>{summary}</p> : null}
        </div>

        <LayoutGroup id={`${deckId}-tabs`}>
          <div
            className="downloads-stage-tabs"
            role="tablist"
            aria-label={title}
          >
            {availableTabs.map((tab) => {
              const isActive = tab.id === resolvedActiveTab.id;

              return (
                <button
                  key={tab.id}
                  type="button"
                  role="tab"
                  aria-selected={isActive}
                  className={`downloads-stage-tab${isActive ? " is-active" : ""}`}
                  onClick={() => setActiveTabId(tab.id)}
                >
                  {isActive ? (
                    <m.span
                      className="downloads-stage-tab-highlight"
                      layoutId={`${deckId}-highlight`}
                      transition={downloadsSelectionTransition}
                    />
                  ) : null}
                  <span className="downloads-stage-tab-label">{tab.label}</span>
                  {typeof tab.count === "number" ? (
                    <span className="downloads-stage-tab-count">
                      {tab.count.toLocaleString()}
                    </span>
                  ) : null}
                </button>
              );
            })}
          </div>
        </LayoutGroup>
      </div>

      <div className="downloads-stage-tab-panels">
        <AnimatePresence mode="wait" initial={false}>
          <m.div
            key={resolvedActiveTab.id}
            className="downloads-stage-tab-panel"
            initial={{ opacity: 0, y: 8 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0, y: -6 }}
            transition={downloadsSelectionTransition}
          >
            {resolvedActiveTab.content}
          </m.div>
        </AnimatePresence>
      </div>
    </div>
  );
}

function StandardPreviewPanel({
  suggestions,
  safeCount,
  reviewCount,
  unchangedCount,
  userView,
}: {
  suggestions: OrganizationPreview["suggestions"];
  safeCount: number;
  reviewCount: number;
  unchangedCount: number;
  userView: UserView;
}) {
  const [showingAll, setShowingAll] = useState(userView === "power");
  const visibleSuggestions = showingAll ? suggestions : suggestions.slice(0, 12);

  return (
    <div className="downloads-preview-stack">
      <div className="downloads-preview-summary">
        <div className="downloads-preview-summary-topline">
          <strong>
            {userView === "beginner"
              ? "What will head into your Mods folder"
              : "Validated hand-off preview"}
          </strong>
          <button
            type="button"
            className="secondary-action compact-action"
            onClick={() => setShowingAll((current) => !current)}
            disabled={suggestions.length === 0}
          >
            {sampleToggleLabel(showingAll)}
          </button>
        </div>
        <span>
          {userView === "beginner"
            ? "Peek at a few files first, or open the whole batch if you want the full story."
            : "The rows below start as a sample so you can skim the batch fast, then open the full list when needed."}
        </span>
        <span className="downloads-preview-count">
          {sampleCountLabel(visibleSuggestions.length, suggestions.length, showingAll)}
        </span>
        <div className="downloads-preview-summary-grid">
          <SummaryStat label="Safe" value={safeCount} tone="good" />
          <SummaryStat label="Needs review" value={reviewCount} tone="low" />
          <SummaryStat label="Already fine" value={unchangedCount} tone="neutral" />
        </div>
      </div>

      <div className="downloads-preview-rows">
        {visibleSuggestions.map((item, index) => {
          const state =
            item.reviewRequired
              ? "review"
              : item.finalAbsolutePath === item.currentPath
                ? "aligned"
                : "safe";

          return (
            <m.div
              key={item.fileId}
              className={`preview-row preview-row-state-${state}`}
              whileHover={rowHover}
              {...stagedListItem(index)}
            >
              <div className="preview-row-main">
                <strong>{item.filename}</strong>
                <span>
                  {item.creator ?? "Unknown"} · {friendlyTypeLabel(item.kind)}
                </span>
              </div>
              <div className="downloads-preview-route">
                <div className="section-label">
                  {userView === "beginner" ? "Safe folder" : "Safe route"}
                </div>
                <code>{formatPreviewPath(item.finalRelativePath, userView)}</code>
                <strong className="downloads-preview-route-headline">
                  {state === "safe"
                    ? userView === "beginner"
                      ? "Ready to scoot into place"
                      : "Ready for the safe hand-off"
                    : state === "aligned"
                      ? userView === "beginner"
                        ? "Already tucked away safely"
                        : "Already in a safe spot"
                      : "Held for review"}
                </strong>
                {item.validatorNotes.length ? (
                  <span className="downloads-preview-route-note">
                    {item.validatorNotes[0]}
                  </span>
                ) : state === "safe" ? (
                  <span className="downloads-preview-route-note">
                    {userView === "beginner"
                      ? "Only the ready part of this batch will move."
                      : "This row can move in the approved batch."}
                  </span>
                ) : null}
              </div>
              <div className="preview-row-meta">
                <span className={`confidence-badge ${previewStateTone(state)}`}>
                  {previewStateLabel(state)}
                </span>
              </div>
            </m.div>
          );
        })}
      </div>
    </div>
  );
}

function GuidedPreviewPanel({
  plan,
  userView,
}: {
  plan: GuidedInstallPlan;
  userView: UserView;
}) {
  const dependencySummary = summarizeDependencies(plan.dependencies);
  const detailTabs: DownloadsStageTab[] = [
    {
      id: "plan",
      label: userView === "beginner" ? "What moves" : "Plan",
      count:
        plan.installFiles.length +
        plan.replaceFiles.length +
        plan.preserveFiles.length,
      content: (
        <div className="downloads-stage-tab-panel-stack">
          <div className="downloads-guided-columns">
            <GuidedListCard
              title={userView === "beginner" ? "What will move" : "Install files"}
              badge={plan.installFiles.length.toString()}
              tone="good"
              files={plan.installFiles}
              userView={userView}
              showPaths={userView === "power"}
            />
            <GuidedListCard
              title={userView === "beginner" ? "What will be replaced" : "Replace files"}
              badge={plan.replaceFiles.length.toString()}
              tone="medium"
              files={plan.replaceFiles}
              userView={userView}
              showPaths={userView === "power"}
            />
            <GuidedListCard
              title={userView === "beginner" ? "What will be kept" : "Keep files"}
              badge={plan.preserveFiles.length.toString()}
              tone="neutral"
              files={plan.preserveFiles}
              userView={userView}
              showPaths={userView === "power"}
            />
          </div>
        </div>
      ),
    },
    ...(plan.dependencies.length
      ? [
          {
            id: "dependencies",
            label: userView === "beginner" ? "Depends on" : "Dependencies",
            count: plan.dependencies.length,
            content: (
              <div className="downloads-stage-tab-panel-stack">
                <div className="downloads-guided-card downloads-guided-card-neutral">
                  <div className="downloads-guided-card-header">
                    <strong>
                      {userView === "beginner" ? "What it depends on" : "Dependencies"}
                    </strong>
                    <span className="ghost-chip">{plan.dependencies.length}</span>
                  </div>
                  <div className="downloads-evidence-list">
                    {plan.dependencies.map((dependency) => (
                      <div key={dependency.key} className="downloads-evidence-row">
                        <strong>{dependency.displayName}</strong>
                        <span>{friendlyDependencyState(dependency.status)}</span>
                        <span>{dependency.summary}</span>
                      </div>
                    ))}
                  </div>
                </div>
              </div>
            ),
          },
        ]
      : []),
    ...((plan.postInstallNotes.length > 0 || plan.warnings.length > 0)
      ? [
          {
            id: "notes",
            label: userView === "beginner" ? "Remember" : "Notes",
            count: plan.postInstallNotes.length + plan.warnings.length,
            content: (
              <div className="downloads-stage-tab-panel-stack">
                {plan.postInstallNotes.length ? (
                  <div className="downloads-guided-card downloads-guided-card-neutral">
                    <div className="downloads-guided-card-header">
                      <strong>
                        {userView === "beginner"
                          ? "What to remember"
                          : "Post-install notes"}
                      </strong>
                    </div>
                    <div className="downloads-evidence-list">
                      {plan.postInstallNotes.map((note) => (
                        <div key={note} className="downloads-evidence-row">
                          {note}
                        </div>
                      ))}
                    </div>
                  </div>
                ) : null}
                {plan.warnings.length ? (
                  <div className="downloads-guided-warnings">
                    {plan.warnings.map((warning) => (
                      <div key={warning} className="status-banner">
                        {warning}
                      </div>
                    ))}
                  </div>
                ) : null}
              </div>
            ),
          },
        ]
      : []),
    ...(userView !== "beginner"
      ? [
          {
            id: "reason",
            label: userView === "power" ? "Why this path" : "Why",
            count:
              plan.evidence.length +
              plan.incompatibilityWarnings.length +
              plan.existingLayoutFindings.length,
            content: (
              <div className="downloads-stage-tab-panel-stack">
                <div className="downloads-guided-card downloads-guided-card-neutral">
                  <div className="downloads-guided-card-header">
                    <strong>
                      {userView === "power"
                        ? "Matched evidence"
                        : "Why SimSuite chose this path"}
                    </strong>
                  </div>
                  <div className="downloads-evidence-list">
                    {plan.evidence.map((reason) => (
                      <div key={reason} className="downloads-evidence-row">
                        {reason}
                      </div>
                    ))}
                    {plan.incompatibilityWarnings.map((warning) => (
                      <div key={warning} className="downloads-evidence-row">
                        {warning}
                      </div>
                    ))}
                    {plan.existingLayoutFindings.map((finding) => (
                      <div key={finding} className="downloads-evidence-row">
                        {finding}
                      </div>
                    ))}
                    {userView === "power" && plan.catalogSource ? (
                      <div className="downloads-evidence-row">
                        Catalog reviewed {plan.catalogSource.reviewedAt ?? "recently"}.
                      </div>
                    ) : null}
                  </div>
                </div>
              </div>
            ),
          },
        ]
      : []),
  ];

  return (
    <div className="downloads-guided-layout">
      <div className="downloads-guided-focus">
        <div className="downloads-guided-title">
          <div>
            <p className="eyebrow">Special setup</p>
            <h3>{plan.profileName}</h3>
          </div>
          <span className={`confidence-badge ${plan.applyReady ? "good" : "medium"}`}>
            {plan.applyReady ? "Ready to install" : "Needs review"}
          </span>
        </div>
        <p>{plan.explanation}</p>
        <div className="detail-list">
          <DetailRow label="Family" value={plan.specialFamily ?? "Special mod"} />
          <DetailRow label="Dependency" value={dependencySummary} />
          <DetailRow
            label="Existing install"
            value={plan.existingInstallDetected ? "Found" : "Not found"}
          />
        </div>
        <div className="path-card">{plan.installTargetFolder}</div>
        <div className="summary-matrix">
          <SummaryStat label="Move" value={plan.installFiles.length} tone="good" />
          <SummaryStat label="Replace" value={plan.replaceFiles.length} tone="neutral" />
          <SummaryStat label="Keep" value={plan.preserveFiles.length} tone="neutral" />
          <SummaryStat label="Needs review" value={plan.reviewFiles.length} tone="low" />
        </div>
      </div>

      <DownloadsStageDeck
        deckId="downloads-guided-deck"
        eyebrow={userView === "beginner" ? "More detail" : "Keep the stage calm"}
        title={
          userView === "beginner"
            ? "Open only the part you need"
            : "More setup detail, only when you want it"
        }
        summary={
          userView === "power"
            ? "The main stage stays focused while the deeper setup detail moves into tabs."
            : undefined
        }
        defaultTabId="plan"
        tabs={detailTabs}
      />
    </div>
  );
}

function SpecialReviewPanel({
  item,
  reviewPlan,
  files,
  userView,
  reviewActions,
  onResolveAction,
  isApplying,
}: {
  item: DownloadsInboxItem;
  reviewPlan: SpecialReviewPlan;
  files: DownloadInboxDetail["files"];
  userView: UserView;
  reviewActions: ReviewPlanAction[];
  onResolveAction: (action: ReviewPlanAction) => Promise<void>;
  isApplying: boolean;
}) {
  const modeEyebrow =
    item.intakeMode === "guided"
      ? "Special setup"
      : item.intakeMode === "blocked"
        ? "Blocked"
        : "Needs review";
  const trackedFiles =
    reviewPlan.reviewFiles.length > 0
      ? reviewPlan.reviewFiles
      : files.map((file) => ({
          fileId: file.fileId,
          filename: file.filename,
          currentPath: file.currentPath,
          targetPath: null,
          archiveMemberPath: file.archiveMemberPath,
          kind: file.kind,
          subtype: file.subtype,
          creator: file.creator,
          notes: file.safetyNotes,
        }));
  const repairAction =
    reviewActions.find((action) => action.kind === "repair_special") ?? null;
  const secondaryActions = reviewActions.filter(
    (action) => action.kind !== "repair_special",
  );
  const repairBuckets = [
    {
      key: "move",
      title:
        userView === "power" ? "Older files to clear out" : "Older files to clear out",
      description:
        userView === "power"
          ? "Older suite files that SimSuite will move out of the way before the update."
          : "Older suite files that SimSuite will clear out before the update.",
      badge: reviewPlan.repairMoveFiles.length.toString(),
      tone: "good" as const,
      files: reviewPlan.repairMoveFiles,
    },
    {
      key: "replace",
      title:
        userView === "power" ? "Incoming replacement files" : "New files to swap in",
      description:
        userView === "power"
          ? "Fresh files from the new download that will replace the old suite."
          : "Fresh files from this download that will replace the older suite files.",
      badge: reviewPlan.repairReplaceFiles.length.toString(),
      tone: "medium" as const,
      files: reviewPlan.repairReplaceFiles,
    },
    {
      key: "keep",
      title:
        userView === "power" ? "Settings and sidecars to keep" : "Settings to keep",
      description:
        userView === "power"
          ? "Saved settings and side files that stay safe during the repair."
          : "Saved settings and side files that will stay safe during the repair.",
      badge: reviewPlan.repairKeepFiles.length.toString(),
      tone: "neutral" as const,
      files: reviewPlan.repairKeepFiles,
    },
  ].filter((bucket) => bucket.files.length > 0);
  const repairSteps = [
    {
      key: "move",
      title: "Clear out the older suite",
      count: reviewPlan.repairMoveFiles.length,
      description: `${reviewPlan.repairMoveFiles.length.toLocaleString()} file(s) will be moved out of the way before the update starts.`,
    },
    {
      key: "keep",
      title: "Keep the saved settings",
      count: reviewPlan.repairKeepFiles.length,
      description: `${reviewPlan.repairKeepFiles.length.toLocaleString()} file(s) will stay safe so saved choices do not get lost.`,
    },
    {
      key: "replace",
      title: "Swap in the new files",
      count: reviewPlan.repairReplaceFiles.length,
      description: `${reviewPlan.repairReplaceFiles.length.toLocaleString()} incoming file(s) will replace the older suite files.`,
    },
    {
      key: "recheck",
      title: "Re-check the setup",
      count: -1,
      description: reviewPlan.repairCanContinueInstall
        ? "If the incoming pack is complete, SimSuite can finish the update in the same approved run."
        : "SimSuite repairs the old layout first, then checks the batch again.",
    },
  ].filter((step) => step.count !== 0);
  const detailTabs: DownloadsStageTab[] = [
    {
      id: "reason",
      label: userView === "beginner" ? "Why" : "Reason",
      count:
        (reviewPlan.evidence.length ? reviewPlan.evidence : item.assessmentReasons).length +
        reviewPlan.postInstallNotes.length +
        (userView === "power" ? reviewPlan.existingLayoutFindings.length : 0) +
        reviewPlan.incompatibilityWarnings.length,
      content: (
        <div className="downloads-stage-tab-panel-stack">
          {reviewPlan.incompatibilityWarnings.length ? (
            <div className="downloads-guided-warnings">
              {reviewPlan.incompatibilityWarnings.map((warning) => (
                <div key={warning} className="status-banner status-banner-error">
                  {warning}
                </div>
              ))}
            </div>
          ) : null}
          <div className="downloads-guided-card downloads-guided-card-neutral">
            <div className="downloads-guided-card-header">
              <strong>
                {userView === "beginner" ? "Why SimSuite decided this" : "Evidence"}
              </strong>
            </div>
            <div className="downloads-evidence-list">
              {(reviewPlan.evidence.length ? reviewPlan.evidence : item.assessmentReasons).map(
                (reason) => (
                  <div key={reason} className="downloads-evidence-row">
                    {reason}
                  </div>
                ),
              )}
              {reviewPlan.postInstallNotes.map((note) => (
                <div key={note} className="downloads-evidence-row">
                  {note}
                </div>
              ))}
              {userView === "power"
                ? reviewPlan.existingLayoutFindings.map((finding) => (
                    <div key={finding} className="downloads-evidence-row">
                      {finding}
                    </div>
                  ))
                : null}
              {userView === "power" && reviewPlan.catalogSource ? (
                <div className="downloads-evidence-row">
                  Catalog reviewed {reviewPlan.catalogSource.reviewedAt ?? "recently"}.
                </div>
              ) : null}
            </div>
          </div>
        </div>
      ),
    },
    ...(reviewPlan.dependencies.length
      ? [
          {
            id: "dependencies",
            label: userView === "beginner" ? "Depends on" : "Dependencies",
            count: reviewPlan.dependencies.length,
            content: (
              <div className="downloads-stage-tab-panel-stack">
                <div className="downloads-guided-card downloads-guided-card-neutral">
                  <div className="downloads-guided-card-header">
                    <strong>
                      {userView === "beginner" ? "What it depends on" : "Dependencies"}
                    </strong>
                    <span className="ghost-chip">{reviewPlan.dependencies.length}</span>
                  </div>
                  <div className="downloads-evidence-list">
                    {reviewPlan.dependencies.map((dependency) => (
                      <div key={dependency.key} className="downloads-evidence-row">
                        <strong>{dependency.displayName}</strong>
                        <span>{friendlyDependencyState(dependency.status)}</span>
                        <span>{dependency.summary}</span>
                        {dependency.inboxItemId ? (
                          <span>
                            {dependency.inboxItemGuidedInstallAvailable &&
                            dependency.inboxItemIntakeMode === "guided"
                              ? "SimSuite can install this first from the Inbox."
                              : "Open this dependency in the Inbox before returning here."}
                          </span>
                        ) : null}
                      </div>
                    ))}
                  </div>
                </div>
              </div>
            ),
          },
        ]
      : []),
    {
      id: "files",
      label: userView === "beginner" ? "Files" : "Tracked files",
      count: trackedFiles.length,
      content: (
        <div className="downloads-stage-tab-panel-stack">
          <GuidedListCard
            title={
              userView === "beginner" ? "Files SimSuite stopped on" : "Tracked review files"
            }
            badge={trackedFiles.length.toString()}
            tone={item.intakeMode === "blocked" ? "medium" : "neutral"}
            files={trackedFiles}
            userView={userView}
            showPaths={userView === "power"}
          />
        </div>
      ),
    },
  ];

  return (
    <div className="downloads-assessment-layout">
      <div className={`downloads-assessment-card downloads-assessment-${item.intakeMode}`}>
        <div className="downloads-guided-title">
          <div>
            <p className="eyebrow">{modeEyebrow}</p>
            <h3>{reviewPlan.profileName ?? item.matchedProfileName ?? "Inbox item"}</h3>
          </div>
          <span className={`confidence-badge ${intakeModeTone(item.intakeMode)}`}>
            {intakeModeLabel(item.intakeMode)}
          </span>
        </div>
        <p>{reviewPlan.explanation}</p>
        <div className="detail-list">
          <DetailRow label="Family" value={reviewPlan.specialFamily ?? "Special mod"} />
          <DetailRow
            label="Dependency"
            value={summarizeDependencies(reviewPlan.dependencies)}
          />
          <DetailRow
            label={userView === "beginner" ? "Files in this batch" : "Tracked files"}
            value={trackedFiles.length.toLocaleString()}
          />
        </div>
        <div className="audit-what-card">
          <strong>{userView === "beginner" ? "What happens next" : "Recommended next step"}</strong>
          <span>{reviewPlan.recommendedNextStep}</span>
        </div>
      </div>

      {repairAction ? (
        <div className="downloads-guided-card downloads-guided-card-good downloads-repair-plan-card">
          <div className="downloads-guided-card-header">
            <strong>
              {userView === "beginner"
                ? "One safe fix is ready"
                : userView === "power"
                  ? "Repair queue"
                  : "Safe repair plan"}
            </strong>
            <span className="ghost-chip">
              {reviewPlan.repairMoveFiles.length +
                reviewPlan.repairReplaceFiles.length +
                reviewPlan.repairKeepFiles.length}{" "}
              files
            </span>
          </div>
          <div className="downloads-repair-hero">
            <div className="downloads-repair-copy">
              <div className="section-label">
                {userView === "beginner"
                  ? "Safe fix"
                  : userView === "power"
                    ? "Repair action"
                    : "Next approved action"}
              </div>
              <strong>{reviewPlan.repairReason ?? reviewActionDescription(repairAction)}</strong>
              <span>
                {userView === "beginner"
                  ? "SimSuite can clear the older setup out of the way first, then continue with the update."
                  : "SimSuite will clear the older setup out of the way first so the update can continue safely."}
              </span>
            </div>
            <button
              type="button"
              className="primary-action downloads-review-action-cta"
              onClick={() => void onResolveAction(repairAction)}
              disabled={isApplying}
            >
              <Workflow size={14} strokeWidth={2} />
              {reviewActionButtonLabel(repairAction, userView, isApplying)}
            </button>
          </div>
          <div className="summary-matrix">
            <SummaryStat
              label={userView === "beginner" ? "Clear old files" : "Clear old files"}
              value={reviewPlan.repairMoveFiles.length}
              tone="good"
            />
            <SummaryStat
              label={userView === "beginner" ? "Replace with new" : "Replace"}
              value={reviewPlan.repairReplaceFiles.length}
              tone="neutral"
            />
            <SummaryStat
              label={userView === "beginner" ? "Keep settings" : "Keep"}
              value={reviewPlan.repairKeepFiles.length}
              tone="neutral"
            />
          </div>
          {reviewPlan.repairTargetFolder ? (
            <div className="downloads-repair-target">
              <div className="section-label">Safe folder</div>
              <div className="path-card">{reviewPlan.repairTargetFolder}</div>
            </div>
          ) : null}
          <div className="downloads-repair-steps">
            {repairSteps.map((step, index) => (
              <div key={step.key} className="downloads-repair-step">
                <div className="downloads-repair-step-topline">
                  <span className="downloads-repair-step-number">{index + 1}</span>
                  {step.count >= 0 ? (
                    <span className="ghost-chip">
                      {step.count.toLocaleString()} {step.count === 1 ? "file" : "files"}
                    </span>
                  ) : (
                    <span className="ghost-chip">final check</span>
                  )}
                </div>
                <strong>{step.title}</strong>
                <p>{step.description}</p>
              </div>
            ))}
          </div>
          {userView !== "beginner" && repairBuckets.length ? (
            <div className={`downloads-repair-buckets downloads-repair-buckets-${userView}`}>
              {repairBuckets.map((bucket) => (
                <GuidedListCard
                  key={bucket.key}
                  title={bucket.title}
                  description={bucket.description}
                  badge={bucket.badge}
                  tone={bucket.tone}
                  files={bucket.files}
                  userView={userView}
                  showPaths={userView === "power"}
                  variant="bucket"
                  hideWhenEmpty
                />
              ))}
            </div>
          ) : null}
        </div>
      ) : secondaryActions.length ? (
        <div className="downloads-guided-card downloads-guided-card-good">
          <div className="downloads-guided-card-header">
            <strong>
              {userView === "beginner"
                ? "What SimSuite can do now"
                : "Safe next action"}
            </strong>
            <span className="ghost-chip">{secondaryActions.length}</span>
          </div>
          <div className="downloads-review-action-stack">
            {secondaryActions.map((action) => (
              <div
                key={reviewActionKey(action)}
                className="downloads-review-action-card"
              >
                <div className="downloads-review-action-copy">
                  <strong>{reviewActionCardTitle(action)}</strong>
                  <span>{reviewActionDescription(action)}</span>
                </div>
                <button
                  type="button"
                  className="primary-action downloads-review-action-cta"
                  onClick={() => void onResolveAction(action)}
                  disabled={isApplying}
                >
                  <Workflow size={14} strokeWidth={2} />
                  {reviewActionButtonLabel(action, userView, isApplying)}
                </button>
              </div>
            ))}
          </div>
        </div>
      ) : null}

      <DownloadsStageDeck
        deckId="downloads-review-deck"
        eyebrow={userView === "beginner" ? "More detail" : "Keep the stage calm"}
        title={
          userView === "power"
            ? "Keep the heavy proof tucked behind one switch"
            : "Open only the extra context you need"
        }
        summary={
          userView === "beginner"
            ? "The main story stays above. These tabs keep the extra checks out of the way."
            : undefined
        }
        defaultTabId={userView === "power" ? "files" : "reason"}
        tabs={detailTabs}
      />
    </div>
  );
}

function TrackedFilesPanel({
  files,
  userView,
}: {
  files: DownloadInboxDetail["files"];
  userView: UserView;
}) {
  return (
    <div className="downloads-guided-columns">
      <GuidedListCard
        title="Tracked files"
        badge={files.length.toString()}
        tone="neutral"
        files={files.map((file) => ({
          fileId: file.fileId,
          filename: file.filename,
          currentPath: file.currentPath,
          targetPath: null,
          archiveMemberPath: file.archiveMemberPath,
          kind: file.kind,
          subtype: file.subtype,
          creator: file.creator,
          notes: file.safetyNotes,
        }))}
        userView={userView}
        showPaths={false}
      />
    </div>
  );
}

function GuidedListCard({
  title,
  description,
  badge,
  tone,
  files,
  userView,
  showPaths,
  variant = "default",
  hideWhenEmpty = false,
}: {
  title: string;
  description?: string;
  badge: string;
  tone: "good" | "medium" | "neutral";
  files: GuidedInstallFileEntry[];
  userView: UserView;
  showPaths: boolean;
  variant?: "default" | "bucket";
  hideWhenEmpty?: boolean;
}) {
  if (hideWhenEmpty && files.length === 0) {
    return null;
  }

  const [showingAll, setShowingAll] = useState(userView === "power");
  const visibleFiles = showingAll ? files : files.slice(0, 8);
  const hasToggle = files.length > 0;

  return (
    <div
      className={`downloads-guided-card downloads-guided-card-${tone} ${
        variant === "bucket" ? "downloads-guided-card-bucket" : ""
      }`}
    >
      <div className="downloads-guided-card-header">
        <div className="downloads-guided-card-copy">
          <strong>{title}</strong>
          {description ? <span>{description}</span> : null}
        </div>
        <span className="ghost-chip">{badge}</span>
      </div>
      {files.length ? (
        <>
          <div className="downloads-guided-card-meta">
            <span className="downloads-preview-count">
              {sampleCountLabel(visibleFiles.length, files.length, showingAll)}
            </span>
            {hasToggle ? (
              <button
                type="button"
                className="secondary-action compact-action"
                onClick={() => setShowingAll((current) => !current)}
              >
                {sampleToggleLabel(showingAll)}
              </button>
            ) : null}
          </div>
          <div className="downloads-guided-list">
            {visibleFiles.map((file) => (
            <div key={`${file.filename}-${file.currentPath}`} className="downloads-guided-row">
              <strong>{file.filename}</strong>
              <span>
                {friendlyTypeLabel(file.kind)}
                {file.subtype ? ` · ${file.subtype}` : ""}
                {file.creator ? ` · ${file.creator}` : ""}
              </span>
              {showPaths && (file.targetPath || file.currentPath) ? (
                <code>{formatPreviewPath(file.targetPath ?? file.currentPath, userView)}</code>
              ) : null}
              {file.notes.length ? (
                <span className="downloads-preview-route-note">{file.notes.join(" · ")}</span>
              ) : null}
            </div>
            ))}
          </div>
        </>
      ) : (
        <div className="downloads-guided-card-meta">
          <span className="downloads-preview-count">No files in this group</span>
          {hasToggle ? (
            <button
              type="button"
              className="secondary-action compact-action"
              onClick={() => setShowingAll((current) => !current)}
            >
              {sampleToggleLabel(showingAll)}
            </button>
          ) : null}
        </div>
      )}
    </div>
  );
}

function buildInspectorSections({
  selectedItem,
  selectedFiles,
  selectedPreview,
  selectedGuidedPlan,
  selectedSpecialDecision,
  selectedVersionResolution,
  selectedReviewPlan,
  safeCount,
  reviewCount,
  unchangedCount,
  userView,
}: {
  selectedItem: DownloadsInboxItem;
  selectedFiles: DownloadInboxDetail["files"];
  selectedPreview: OrganizationPreview | null;
  selectedGuidedPlan: GuidedInstallPlan | null;
  selectedSpecialDecision: SpecialModDecision | null;
  selectedVersionResolution: VersionResolution | null;
  selectedReviewPlan: SpecialReviewPlan | null;
  safeCount: number;
  reviewCount: number;
  unchangedCount: number;
  userView: UserView;
}): DockSectionDefinition[] {
  const queueSection = buildQueueSection(selectedItem, userView);
  const sourceSection = buildSourceSection(selectedItem);
  const timelineSection = buildTimelineSection(selectedItem);
  const filesSection = buildFilesSection(selectedFiles, userView);
  const versionSection = selectedSpecialDecision
    ? buildSpecialVersionSection(selectedSpecialDecision, userView)
    : selectedVersionResolution
      ? buildGenericVersionSection(selectedVersionResolution, userView)
    : null;
  const sharedSections =
    userView === "beginner"
      ? [filesSection]
      : userView === "standard"
        ? [sourceSection, ...(versionSection ? [versionSection] : []), filesSection]
        : [
            ...(versionSection ? [versionSection] : []),
            queueSection,
            sourceSection,
            timelineSection,
            filesSection,
          ];

  if (
    selectedItem.intakeMode === "guided" &&
    selectedGuidedPlan &&
    (selectedGuidedPlan.applyReady || !selectedReviewPlan)
  ) {
    const guidedSummarySection: DockSectionDefinition = {
      id: "guidedSummary",
      label: userView === "beginner" ? "What this mod is" : "Setup",
      hint:
        userView === "beginner"
          ? "Why this download needs a guided install."
          : "Profile, care level, and readiness.",
      children: (
        <div className="detail-list">
          <DetailRow label="Setup type" value="Special setup" />
          <DetailRow label="Profile" value={selectedGuidedPlan.profileName} />
          <DetailRow
            label="Dependency"
            value={summarizeDependencies(selectedGuidedPlan.dependencies)}
          />
          <DetailRow label="Care level" value={riskLevelLabel(selectedItem.riskLevel)} />
          <DetailRow
            label="Existing install"
            value={selectedGuidedPlan.existingInstallDetected ? "Found" : "Not found"}
          />
        </div>
      ),
    };
    const guidedOutcomeSection: DockSectionDefinition = {
      id: "guidedOutcome",
      label: userView === "beginner" ? "What will happen" : "Outcome",
      hint:
        userView === "beginner"
          ? "What will move, what will be replaced, and what will stay."
          : "Install, replace, keep, and review counts.",
      children: (
        <>
          <div className="summary-matrix">
            <SummaryStat label="Install" value={selectedGuidedPlan.installFiles.length} tone="good" />
            <SummaryStat label="Replace" value={selectedGuidedPlan.replaceFiles.length} tone="neutral" />
            <SummaryStat label="Keep" value={selectedGuidedPlan.preserveFiles.length} tone="neutral" />
            <SummaryStat label="Needs review" value={selectedGuidedPlan.reviewFiles.length} tone="low" />
          </div>
          <div className="audit-what-card">
            <strong>Safe install path</strong>
            <span>
              SimSuite keeps the suite together, stays inside a safe script depth, and makes a restore point before anything moves.
            </span>
          </div>
        </>
      ),
    };
    const guidedTargetSection: DockSectionDefinition = {
      id: "guidedTarget",
      label: userView === "beginner" ? "Where it will go" : "Destination",
      hint:
        userView === "beginner"
          ? "The folder SimSuite will use."
          : "Final target folder.",
      children: <div className="path-card">{selectedGuidedPlan.installTargetFolder}</div>,
    };
    const guidedKeepSection: DockSectionDefinition = {
      id: "guidedKeep",
      label: userView === "beginner" ? "What stays" : "Keep + notes",
      hint:
        userView === "beginner"
          ? "What SimSuite will keep and what to remember after install."
          : "Preserved files and reminders.",
      defaultCollapsed: userView === "beginner",
      children: (
        <div className="downloads-evidence-list">
          {selectedGuidedPlan.preserveFiles.map((file) => (
            <div
              key={`${file.filename}-${file.currentPath}`}
              className="downloads-evidence-row"
            >
              Keep {file.filename}
            </div>
          ))}
          {selectedGuidedPlan.postInstallNotes.map((note) => (
            <div key={note} className="downloads-evidence-row">
              {note}
            </div>
          ))}
        </div>
      ),
    };
    const guidedEvidenceSection: DockSectionDefinition = {
      id: "guidedEvidence",
      label: userView === "beginner" ? "Why SimSuite is confident" : "Evidence",
      hint:
        userView === "beginner"
          ? "The clues SimSuite used."
          : "Matched clues and existing layout findings.",
      defaultCollapsed: userView === "beginner",
      children: (
        <div className="downloads-evidence-list">
          {selectedItem.assessmentReasons.map((reason) => (
            <div key={reason} className="downloads-evidence-row">
              {reason}
            </div>
          ))}
          {userView === "power"
            ? selectedGuidedPlan.existingLayoutFindings.map((finding) => (
                <div key={finding} className="downloads-evidence-row">
                  {finding}
                </div>
              ))
            : null}
        </div>
      ),
    };

    return [
      guidedSummarySection,
      guidedOutcomeSection,
      guidedTargetSection,
      ...(userView === "beginner" ? [] : [guidedKeepSection]),
      ...(userView === "power" ? [guidedEvidenceSection] : []),
      ...(userView === "beginner" && versionSection ? [versionSection] : []),
      ...sharedSections,
    ];
  }

  if (
    (selectedItem.intakeMode === "guided" ||
      selectedItem.intakeMode === "needs_review" ||
      selectedItem.intakeMode === "blocked") &&
    selectedReviewPlan
  ) {
    return [
      {
        id: "reviewSummary",
        label: userView === "beginner" ? "What this is" : "Summary",
        hint:
          userView === "beginner"
            ? "What SimSuite found and why it stopped here."
            : "Mode, profile, and current state.",
        children: (
          <div className="detail-list">
            <DetailRow label="Mode" value={intakeModeLabel(selectedItem.intakeMode)} />
            <DetailRow
              label="Profile"
              value={selectedReviewPlan.profileName ?? "Not matched"}
            />
            <DetailRow
              label="Dependency"
              value={summarizeDependencies(selectedReviewPlan.dependencies)}
            />
            <DetailRow label="Care level" value={riskLevelLabel(selectedItem.riskLevel)} />
            <DetailRow
              label="Files tracked"
              value={selectedReviewPlan.reviewFiles.length.toLocaleString()}
            />
          </div>
        ),
      },
      ...(userView === "beginner" && versionSection ? [versionSection] : []),
      {
        id: "reviewNextStep",
        label: userView === "beginner" ? "Safe next step" : "Next move",
        hint:
          userView === "beginner"
            ? "What to do before this download can move."
            : "Safest action from here.",
        children: (
          <div className="audit-what-card">
            <strong>{intakeModeLabel(selectedItem.intakeMode)}</strong>
            <span>{selectedReviewPlan.recommendedNextStep}</span>
          </div>
        ),
      },
      ...(selectedReviewPlan.dependencies.length ||
      selectedReviewPlan.incompatibilityWarnings.length ||
      selectedReviewPlan.postInstallNotes.length
        ? [{
        id: "reviewDependency",
        label: userView === "beginner" ? "What it depends on" : "Deps + warnings",
        hint:
          userView === "beginner"
            ? "Anything else this mod needs first."
            : "Required helpers, conflicts, and notes.",
        defaultCollapsed: userView === "beginner",
        children: (
          <div className="downloads-evidence-list">
            {selectedReviewPlan.dependencies.length ? (
              selectedReviewPlan.dependencies.map((dependency) => (
                <div key={dependency.key} className="downloads-evidence-row">
                  <strong>{dependency.displayName}</strong>: {dependency.summary}
                  {dependency.inboxItemId ? (
                    <>
                      {" "}
                      {dependency.inboxItemGuidedInstallAvailable &&
                      dependency.inboxItemIntakeMode === "guided"
                        ? "SimSuite can install it from the Inbox after approval."
                        : "Open that dependency in the Inbox before returning here."}
                    </>
                  ) : null}
                </div>
              ))
            ) : (
              <div className="downloads-evidence-row">No extra dependency is required.</div>
            )}
            {selectedReviewPlan.incompatibilityWarnings.map((warning) => (
              <div key={warning} className="downloads-evidence-row">
                {warning}
              </div>
            ))}
            {selectedReviewPlan.postInstallNotes.map((note) => (
              <div key={note} className="downloads-evidence-row">
                {note}
              </div>
            ))}
          </div>
        ),
      }] : []),
      ...(userView === "power"
        ? [{
        id: "reviewEvidence",
        label: "Evidence",
        hint: "Matched evidence, tracked findings, and catalog notes.",
        defaultCollapsed: false,
        children: (
          <div className="downloads-evidence-list">
            {selectedReviewPlan.evidence.map((reason) => (
              <div key={reason} className="downloads-evidence-row">
                {reason}
              </div>
            ))}
            {userView === "power"
              ? selectedReviewPlan.existingLayoutFindings.map((finding) => (
                  <div key={finding} className="downloads-evidence-row">
                    {finding}
                  </div>
                ))
            : null}
          </div>
        ),
      }] : []),
      ...sharedSections,
    ];
  }

  return [
    {
      id: "summary",
      label: userView === "beginner" ? "What this batch is" : "Summary",
      hint:
        userView === "beginner"
          ? "How many files are here and what kind of batch this is."
          : "Source type, file counts, and intake mode.",
      children: (
        <div className="detail-list">
          <DetailRow label="Mode" value={intakeModeLabel(selectedItem.intakeMode)} />
          <DetailRow
            label="Source"
            value={
              selectedItem.sourceKind === "archive"
                ? selectedItem.archiveFormat
                  ? `${selectedItem.archiveFormat.toUpperCase()} archive`
                  : "Archive"
                : "Direct file"
            }
          />
          <DetailRow label="Files found" value={selectedItem.detectedFileCount.toLocaleString()} />
          <DetailRow label="Still in inbox" value={selectedItem.activeFileCount.toLocaleString()} />
        </div>
      ),
    },
    {
      id: "handoff",
      label: userView === "beginner" ? "What move does" : "Hand-off",
      hint:
        userView === "beginner"
          ? "Only safe files move from here. Review files stay visible."
          : "Preview counts for safe moves, review holds, and already-correct files.",
      children: (
        <>
          <div className="summary-matrix">
            <SummaryStat label="Safe" value={safeCount} tone="good" />
            <SummaryStat label="Needs review" value={reviewCount} tone="low" />
            <SummaryStat label="Already fine" value={unchangedCount} tone="neutral" />
          </div>
          <div className="audit-what-card">
            <strong>Normal inbox flow</strong>
            <span>
              Approved files use the same validator and snapshot path as the main organizer. Files that need review stay in the inbox.
            </span>
          </div>
        </>
      ),
    },
    ...(userView === "beginner"
      ? []
      : [{
      id: "preset",
      label: "Rule set",
      hint: "Current rule set for normal hand-off items.",
      children: (
        <div className="detail-list">
          <DetailRow label="Preset" value={selectedPreview?.presetName ?? "Not loaded"} />
          <DetailRow
            label="Checked files"
            value={selectedPreview?.totalConsidered.toLocaleString() ?? "0"}
          />
          <DetailRow
            label="Suggested"
            value={selectedPreview?.recommendedPreset ?? "Current choice"}
          />
        </div>
      ),
    }]),
    ...sharedSections,
  ];
}

function buildQueueSection(
  selectedItem: DownloadsInboxItem,
  userView: UserView,
): DockSectionDefinition {
  const lane = selectedItem.queueLane ?? deriveQueueLane(selectedItem);

  return {
    id: "queue",
    label: userView === "beginner" ? "Inbox lane" : "Lane",
    hint:
      userView === "beginner"
        ? "Why this batch is sitting where it is in the Inbox."
        : "Lane, linked setup group, and the best quick summary.",
    children: (
      <div className="detail-list">
        <DetailRow label="Lane" value={queueLaneLabel(lane, userView)} />
        <DetailRow
          label="Linked items"
          value={(selectedItem.relatedItemIds?.length ?? 0) > 0
            ? `${(selectedItem.relatedItemIds?.length ?? 0) + 1} items in this setup chain`
            : "This batch is standing on its own"}
        />
      </div>
    ),
  };
}

function buildSourceSection(selectedItem: DownloadsInboxItem): DockSectionDefinition {
  return {
    id: "source",
    label: "Source",
    hint: "Download path, notes, and any intake errors.",
    children: (
      <div className="path-grid">
        <div className="detail-block">
          <div className="section-label">Original source</div>
          <div className="path-card">{selectedItem.sourcePath}</div>
        </div>
        {selectedItem.notes.length ? (
          <div className="tag-list">
            {selectedItem.notes.map((note) => (
              <span key={note} className="ghost-chip">
                {note}
              </span>
            ))}
          </div>
        ) : null}
        {selectedItem.errorMessage ? (
          <div className="status-banner status-banner-error">
            {selectedItem.errorMessage}
          </div>
        ) : null}
      </div>
    ),
  };
}

function buildTimelineSection(selectedItem: DownloadsInboxItem): DockSectionDefinition {
  const timelineEntries = selectedItem.timeline ?? [];

  return {
    id: "timeline",
    label: "Timeline",
    hint: "What happened in the Inbox.",
    defaultCollapsed: false,
    children: (
      <div className="downloads-timeline">
        {timelineEntries.length ? (
          timelineEntries.map((entry, index) => (
            <div key={`${entry.label}-${entry.at ?? index}`} className="downloads-timeline-row">
              <strong>{entry.label}</strong>
              <span>{entry.detail ?? "No extra detail saved."}</span>
              {entry.at ? (
                <span className="downloads-timeline-time">{formatDate(entry.at)}</span>
              ) : null}
            </div>
          ))
        ) : (
          <div className="downloads-evidence-row">No inbox timeline yet.</div>
        )}
      </div>
    ),
  };
}

function buildFilesSection(
  selectedFiles: DownloadInboxDetail["files"],
  userView: UserView,
): DockSectionDefinition {
  return {
    id: "files",
    label: userView === "beginner" ? "Included files" : "Files",
    hint:
      userView === "beginner"
        ? "A few files are shown first so you can skim the batch quickly."
        : "Files inside this inbox item.",
    badge: `${selectedFiles.length}`,
    defaultCollapsed: userView !== "power",
    children: selectedFiles.length ? (
      <TrackedFilesSampleList files={selectedFiles} userView={userView} />
    ) : (
      <p>No tracked files are active for this inbox item.</p>
    ),
  };
}

function buildSpecialVersionSection(
  specialDecision: SpecialModDecision,
  userView: UserView,
): DockSectionDefinition {
  const officialLatest = specialDecision.officialLatest;
  const officialVersion =
    officialLatest?.status === "known"
      ? officialLatest.latestVersion ?? "Known, but not labeled"
      : officialLatest?.status === "unknown"
        ? "Latest online version unknown"
        : "Not checked yet";

  return {
    id: "version",
    label: userView === "beginner" ? "Version check" : "Versions",
    hint:
      userView === "beginner"
        ? "What is installed, what you downloaded, and what clue SimSuite trusted."
        : "Installed copy, incoming pack, and the local evidence SimSuite used first.",
    defaultCollapsed: false,
    children: (
      <>
        <div className="detail-list">
          <DetailRow
            label="Installed"
            value={formatVersionValue(
              specialDecision.installedState.installedVersion,
              specialDecision.installedState.installState !== "not_installed",
            )}
          />
          <DetailRow
            label="Incoming"
            value={formatVersionValue(specialDecision.incomingVersion, true)}
          />
          <DetailRow
            label="Compare"
            value={specialVersionStatusLabel(specialDecision, userView)}
          />
          <DetailRow
            label="Incoming clue"
            value={formatEvidenceSourceValue(
              specialDecision.incomingVersionSource,
              Boolean(specialDecision.incomingVersion),
            )}
          />
          <DetailRow
            label="Installed clue"
            value={formatEvidenceSourceValue(
              specialDecision.installedVersionSource,
              specialDecision.installedState.installState !== "not_installed",
            )}
          />
          <DetailRow
            label="Main check"
            value={formatEvidenceSourceValue(
              specialDecision.comparisonSource,
              true,
            )}
          />
          <DetailRow label="Official latest" value={officialVersion} />
        </div>
        {specialDecision.incomingVersionEvidence.length ? (
          <div className="detail-block">
            <div className="section-label">Incoming evidence</div>
            <div className="downloads-evidence-list">
              {specialDecision.incomingVersionEvidence.map((line) => (
                <div key={line} className="downloads-evidence-row">
                  {line}
                </div>
              ))}
            </div>
          </div>
        ) : null}
        {specialDecision.installedVersionEvidence.length ? (
          <div className="detail-block">
            <div className="section-label">Installed evidence</div>
            <div className="downloads-evidence-list">
              {specialDecision.installedVersionEvidence.map((line) => (
                <div key={line} className="downloads-evidence-row">
                  {line}
                </div>
              ))}
            </div>
          </div>
        ) : null}
        {specialDecision.comparisonEvidence.length ? (
          <div className="detail-block">
            <div className="section-label">Main check</div>
            <div className="downloads-evidence-list">
              {specialDecision.comparisonEvidence.map((line) => (
                <div key={line} className="downloads-evidence-row">
                  {line}
                </div>
              ))}
            </div>
          </div>
        ) : null}
        {specialDecision.officialLatest?.note ? (
          <div className="downloads-evidence-list">
            <div className="downloads-evidence-row">
              {specialDecision.officialLatest.note}
            </div>
          </div>
        ) : null}
      </>
    ),
  };
}

function buildGenericVersionSection(
  versionResolution: VersionResolution,
  userView: UserView,
): DockSectionDefinition {
  return {
    id: "version",
    label: userView === "beginner" ? "Version check" : "Versions",
    hint:
      userView === "beginner"
        ? "What SimSuite matched in Mods and how sure it feels."
        : "Local compare result, confidence, and the evidence SimSuite used.",
    defaultCollapsed: false,
    children: (
      <>
        <div className="detail-list">
          <DetailRow
            label="Download"
            value={formatVersionValue(versionResolution.incomingVersion, true)}
          />
          <DetailRow
            label="Installed"
            value={formatVersionValue(
              versionResolution.installedVersion,
              Boolean(versionResolution.matchedSubjectLabel),
            )}
          />
          <DetailRow
            label="Compare"
            value={genericVersionStatusLabel(versionResolution, userView)}
          />
          <DetailRow
            label="Match"
            value={versionResolution.matchedSubjectLabel ?? "No clear installed match"}
          />
          <DetailRow
            label="Confidence"
            value={versionConfidenceLabel(versionResolution.confidence)}
          />
        </div>
        {versionResolution.incomingEvidence.length ? (
          <div className="detail-block">
            <div className="section-label">Download evidence</div>
            <div className="downloads-evidence-list">
              {versionResolution.incomingEvidence.map((line) => (
                <div key={line} className="downloads-evidence-row">
                  {line}
                </div>
              ))}
            </div>
          </div>
        ) : null}
        {versionResolution.installedEvidence.length ? (
          <div className="detail-block">
            <div className="section-label">Installed evidence</div>
            <div className="downloads-evidence-list">
              {versionResolution.installedEvidence.map((line) => (
                <div key={line} className="downloads-evidence-row">
                  {line}
                </div>
              ))}
            </div>
          </div>
        ) : null}
        {versionResolution.evidence.length ? (
          <div className="detail-block">
            <div className="section-label">Main check</div>
            <div className="downloads-evidence-list">
              {versionResolution.evidence.map((line) => (
                <div key={line} className="downloads-evidence-row">
                  {line}
                </div>
              ))}
            </div>
          </div>
        ) : null}
      </>
    ),
  };
}

function buildDownloadInspectorSignals(
  item: DownloadsInboxItem,
  specialDecision: SpecialModDecision | null,
  versionResolution: VersionResolution | null,
  reviewPlan: SpecialReviewPlan | null,
  autoRecheckNote: string | null,
) {
  const signals: Array<{
    id: string;
    tone: "guided" | "review" | "refresh";
    label: string;
    title: string;
    body: string;
  }> = [];

  if (item.intakeMode === "guided") {
    signals.push({
      id: "guided",
      tone: "guided",
      label: "Setup",
      title: "Special setup spotted",
      body: "SimSuite matched this mod and built a guided install path.",
    });
  } else if (
    item.matchedProfileName &&
    (item.intakeMode === "needs_review" || item.intakeMode === "blocked")
  ) {
    const coveredByInstalledFamily =
      specialDecision?.queueLane === "done" && specialDecision.familyRole === "superseded";
    signals.push({
      id: "review",
      tone: "review",
      label: "Setup clue",
      title: item.matchedProfileName,
      body: coveredByInstalledFamily
        ? "A fuller pack from this family is already installed, so this leftover batch can stay out of the install path."
        : reviewPlan?.repairPlanAvailable
          ? "A safe repair path is ready from this panel."
          : "It still needs one more check before anything can move.",
    });
  }

  if (autoRecheckNote) {
    signals.push({
      id: "recheck",
      tone: "refresh",
      label: "Rules refresh",
      title: "Checked again",
      body: autoRecheckNote.replace(`${AUTO_RECHECK_NOTE_PREFIX}. `, ""),
    });
  }

  if ((item.relatedItemIds?.length ?? 0) > 0) {
    signals.push({
      id: "family",
      tone: "refresh",
      label: "Linked family",
      title: `${(item.relatedItemIds?.length ?? 0) + 1} linked item(s)`,
      body: "This batch belongs to the same setup chain.",
    });
  }

  if (specialDecision?.sameVersion) {
    signals.push({
      id: "version",
      tone: "refresh",
      label: "Version",
      title: "Already current",
      body: "The downloaded pack matches the version that is already installed.",
    });
  } else if (specialDecision?.versionStatus === "incoming_older") {
    signals.push({
      id: "version",
      tone: "review",
      label: "Version",
      title: "Older than installed",
      body: "This download looks older than the copy already in Mods.",
    });
  } else if (specialDecision?.officialLatest?.status === "known") {
    signals.push({
      id: "latest",
      tone: "refresh",
      label: "Latest",
      title: `Official latest: ${specialDecision.officialLatest.latestVersion ?? "Known"}`,
      body: "This is extra guidance from the official source and does not block a safe local update.",
    });
  } else if (versionResolution?.status === "same_version") {
    signals.push({
      id: "version",
      tone: "refresh",
      label: "Version",
      title: "Already current",
      body: "The installed copy and this download look like the same version.",
    });
  } else if (versionResolution?.status === "incoming_older") {
    signals.push({
      id: "version",
      tone: "review",
      label: "Version",
      title: "Older than installed",
      body: "The installed copy looks newer than this download, so SimSuite is being cautious.",
    });
  } else if (versionResolution?.status === "incoming_newer") {
    signals.push({
      id: "version",
      tone: "refresh",
      label: "Version",
      title: "Newer download",
      body: "The incoming files look newer than the matching installed copy.",
    });
  } else if (versionResolution?.matchedSubjectLabel) {
    signals.push({
      id: "version",
      tone: "review",
      label: "Compare",
      title: "Possible installed match",
      body: `SimSuite found a likely match in Mods for ${versionResolution.matchedSubjectLabel}, but the version check is still cautious.`,
    });
  }

  return signals;
}

function TrackedFilesSampleList({
  files,
  userView,
}: {
  files: DownloadInboxDetail["files"];
  userView: UserView;
}) {
  const [showingAll, setShowingAll] = useState(userView === "power");
  const visibleFiles = showingAll ? files : files.slice(0, 8);

  return (
    <div className="downloads-mini-list">
      <div className="downloads-guided-card-header downloads-mini-list-header">
        <span className="downloads-preview-count">
          {sampleCountLabel(visibleFiles.length, files.length, showingAll)}
        </span>
        <button
          type="button"
          className="secondary-action compact-action"
          onClick={() => setShowingAll((current) => !current)}
          disabled={files.length === 0}
        >
          {sampleToggleLabel(showingAll)}
        </button>
      </div>
      {visibleFiles.map((file) => (
        <div key={file.fileId} className="downloads-mini-row">
          <strong>{file.filename}</strong>
          <span>
            {friendlyTypeLabel(file.kind)}
            {file.subtype ? ` · ${file.subtype}` : ""}
            {file.creator ? ` · ${file.creator}` : ""}
          </span>
          {userView === "power" ? <code>{formatPreviewPath(file.currentPath, userView)}</code> : null}
        </div>
      ))}
    </div>
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

const DOWNLOAD_LANE_ORDER: DownloadQueueLane[] = [
  "ready_now",
  "special_setup",
  "waiting_on_you",
  "blocked",
  "done",
  "rejected",
];

function groupDownloadItems(items: DownloadsInboxItem[]) {
  const grouped = new Map<DownloadQueueLane, DownloadsInboxItem[]>();

  for (const lane of DOWNLOAD_LANE_ORDER) {
    grouped.set(lane, []);
  }

  for (const item of items) {
    const lane = item.queueLane ?? deriveQueueLane(item);
    grouped.get(lane)?.push(item);
  }

  return DOWNLOAD_LANE_ORDER
    .map((lane) => ({ lane, items: grouped.get(lane) ?? [] }))
    .filter((group) => group.items.length > 0);
}

function deriveQueueLane(item: DownloadsInboxItem): DownloadQueueLane {
  if (item.status === "applied" || item.status === "ignored") {
    return "done";
  }

  if (item.status === "error" || item.intakeMode === "blocked") {
    return "blocked";
  }

  if (item.intakeMode === "guided") {
    return "special_setup";
  }

  if (item.intakeMode === "needs_review" || item.status === "needs_review") {
    return "waiting_on_you";
  }

  return "ready_now";
}

function fallbackQueueSummary(item: DownloadsInboxItem) {
  const lane = item.queueLane ?? deriveQueueLane(item);

  if (!item.specialDecision && item.versionResolution) {
    switch (item.versionResolution.status) {
      case "same_version":
        return "This download matches the version already installed.";
      case "incoming_older":
        return "The copy already in Mods looks newer than this download.";
      case "incoming_newer":
        return "This download looks newer than the matching installed copy.";
      case "not_installed":
        return "No matching installed copy was found yet.";
      default:
        return item.versionResolution.matchedSubjectLabel
          ? `SimSuite found a likely installed match for ${item.versionResolution.matchedSubjectLabel}, but the version result is still cautious.`
          : "SimSuite is still gathering enough clues to compare this download.";
    }
  }

  switch (lane) {
    case "special_setup":
      if (item.guidedInstallAvailable) {
        return item.existingInstallDetected
          ? "SimSuite found an older special setup and is ready to update it safely."
          : "SimSuite recognized a supported special mod and has a guided next step ready.";
      }
      if (item.existingInstallDetected) {
        return "SimSuite found an older special setup and is still checking the safest update path.";
      }
      return "SimSuite recognized a supported special mod and is checking the safest next step.";
    case "waiting_on_you":
      if (item.missingDependencies.length) {
        return `Waiting on ${item.missingDependencies[0]} before anything moves.`;
      }
      return "This batch needs one more choice from you before it can move.";
    case "blocked":
      return item.errorMessage ?? "SimSuite stopped this batch to avoid a risky move.";
    case "done":
      return item.appliedFileCount > 0
        ? "This batch already handed off its safe files."
        : "This batch is hidden from the active Inbox.";
    default:
      return item.reviewFileCount > 0
        ? "Safe files are ready, and the unsure ones will stay behind for review."
        : "This batch is ready for a safe hand-off.";
  }
}

function queueLaneLabel(lane: DownloadQueueLane, userView: UserView) {
  switch (lane) {
    case "ready_now":
      return userView === "beginner" ? "Ready now" : "Ready now";
    case "special_setup":
      return "Special setup";
    case "waiting_on_you":
      return userView === "beginner" ? "Waiting on you" : "Waiting on you";
    case "blocked":
      return "Blocked";
    case "done":
      return userView === "beginner" ? "Done" : "Done";
    default:
      return "Inbox";
  }
}

function queueLaneHint(lane: DownloadQueueLane, userView: UserView) {
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
    default:
      return "";
  }
}

function SummaryStat({
  label,
  value,
  tone,
}: {
  label: string;
  value: number;
  tone: "good" | "neutral" | "low";
}) {
  return (
    <div className={`summary-stat summary-stat-${tone}`}>
      <span>{label}</span>
      <strong>{value.toLocaleString()}</strong>
    </div>
  );
}

function findAutoRecheckNote(notes: string[]) {
  return notes.find((note) => note.startsWith(AUTO_RECHECK_NOTE_PREFIX)) ?? null;
}

function reviewActionKey(action: ReviewPlanAction) {
  return `${action.kind}-${action.relatedItemId ?? "none"}-${action.url ?? "none"}`;
}

function formatPreviewPath(path: string, userView: UserView) {
  if (!path) {
    return "";
  }

  const cleaned = path.replace(/[\\/]+/g, " > ").replace(/^[A-Za-z]: > /, "");
  const parts = cleaned.split(" > ").filter(Boolean);
  const maxParts = userView === "power" ? 8 : userView === "standard" ? 5 : 4;

  if (parts.length <= maxParts) {
    return cleaned;
  }

  return ["...", ...parts.slice(parts.length - maxParts)].join(" > ");
}

function friendlyDependencyState(status: string) {
  switch (status) {
    case "installed":
      return "Installed";
    case "inbox":
    case "in_inbox":
      return "Also in Inbox";
    case "missing":
      return "Missing";
    case "conflict":
      return "Conflict found";
    default:
      return status.replace(/[_-]+/g, " ");
  }
}

function summarizeDependencies(dependencies: DependencyStatus[]) {
  if (!dependencies.length) {
    return "None needed";
  }

  if (dependencies.every((dependency) => dependency.status === "installed")) {
    return "Ready";
  }

  if (dependencies.some((dependency) => dependency.status === "missing")) {
    return "Missing";
  }

  if (dependencies.some((dependency) => dependency.status === "in_inbox")) {
    return "Also in Inbox";
  }

  return "Check details";
}

function buildReviewActions(reviewPlan: SpecialReviewPlan): ReviewPlanAction[] {
  return [...reviewPlan.availableActions].sort((left, right) => right.priority - left.priority);
}

function buildDecisionActions(
  decision: SpecialModDecision,
  reviewPlan: SpecialReviewPlan | null,
): ReviewPlanAction[] {
  const source = decision.availableActions.length
    ? decision.availableActions
    : decision.applyReady
      ? []
      : reviewPlan?.availableActions.length
        ? reviewPlan.availableActions
        : [];
  return [...source].sort((left, right) => right.priority - left.priority);
}

function reviewActionLabel(
  action: ReviewPlanAction,
  userView: UserView,
  isApplying: boolean,
) {
  if (!isApplying) {
    return action.label;
  }

  return reviewActionButtonLabel(action, userView, true);
}

function reviewActionDescription(
  action: ReviewPlanAction,
) {
  return action.description;
}

function reviewActionNeedsApproval(kind: ReviewPlanAction["kind"]) {
  return (
    kind === "repair_special" ||
    kind === "install_dependency" ||
    kind === "download_missing_files" ||
    kind === "separate_supported_files"
  );
}

function reviewActionUpdatesInbox(kind: ReviewPlanAction["kind"]) {
  return (
    kind === "repair_special" ||
    kind === "install_dependency" ||
    kind === "download_missing_files" ||
    kind === "separate_supported_files"
  );
}

function downloadsInspectorIdleNote(
  intakeMode: DownloadIntakeMode,
  userView: UserView,
  safeCount: number,
  guidedPlan?: GuidedInstallPlan | null,
  specialDecision?: SpecialModDecision | null,
  versionResolution?: VersionResolution | null,
  reviewPlan?: SpecialReviewPlan | null,
) {
  if (specialDecision?.sameVersion) {
    return userView === "beginner"
      ? "This mod is already up to date. Reinstall only if you want to replace a damaged copy."
      : "This special-mod family is already current. Reinstall only if you need to replace a damaged copy.";
  }

  if (specialDecision?.versionStatus === "incoming_older") {
    return userView === "beginner"
      ? "This download looks older than what is already installed, so SimSuite is holding it back."
      : "This incoming pack looks older than the installed copy, so SimSuite is not treating it as the next update.";
  }

  if (versionResolution?.status === "same_version") {
    return userView === "beginner"
      ? "This download matches the copy already in Mods. You only need it if you want to replace a damaged install by hand."
      : "The local compare found the same version on both sides. This looks more like a duplicate than a new update.";
  }

  if (versionResolution?.status === "incoming_older") {
    return userView === "beginner"
      ? "This download looks older than the matching copy already in Mods, so SimSuite is holding the move back."
      : "The matching installed copy looks newer than this download, so SimSuite is keeping the apply action quiet.";
  }

  if (versionResolution?.status === "incoming_newer") {
    return userView === "beginner"
      ? "This download looks newer than the matching copy already in Mods."
      : "The shared compare engine found a newer incoming version for the matched installed content.";
  }

  if (versionResolution?.matchedSubjectLabel) {
    return userView === "beginner"
      ? `SimSuite found a possible match in Mods for ${versionResolution.matchedSubjectLabel}, but the version check is still cautious.`
      : `SimSuite matched this download to ${versionResolution.matchedSubjectLabel}, but the version result is still cautious.`;
  }

  if (specialDecision?.availableActions.length) {
    return userView === "beginner"
      ? "SimSuite already has the safest next move ready."
      : "The backend already picked the safest next step for this special-mod batch.";
  }

  if (specialDecision?.queueLane === "done" && specialDecision.familyRole === "superseded") {
    return userView === "beginner"
      ? "A fuller pack from this family is already installed, so this leftover download can stay rejected."
      : "A fuller family pack is already installed, so this leftover batch no longer needs a repair or update step.";
  }

  if (intakeMode === "needs_review") {
    return reviewPlan?.availableActions.length
      ? userView === "beginner"
        ? "SimSuite already has a safe next move ready."
        : "A backend-guided next step is ready for this special batch."
      : "SimSuite still needs a safer clue before it can continue.";
  }

  if (intakeMode === "blocked") {
    return reviewPlan?.availableActions.length
      ? userView === "beginner"
        ? "This batch is blocked, but SimSuite has a safe next move ready."
        : "The current install is blocked, but SimSuite has a backend-guided next step ready."
      : "Blocked until this batch is fixed or replaced.";
  }

  if (intakeMode === "guided") {
    if (reviewPlan?.availableActions.length) {
      return userView === "beginner"
        ? "SimSuite already has a safe next move for this special setup."
        : "A backend-guided next step is ready for this special setup.";
    }

    if (guidedPlan?.reviewFiles.length) {
      return userView === "beginner"
        ? `SimSuite matched this special mod, but ${guidedPlan.reviewFiles.length.toLocaleString()} file(s) still need one more safety check.`
        : `SimSuite recognized this special mod, but ${guidedPlan.reviewFiles.length.toLocaleString()} file(s) still need review before the guided install is safe.`;
    }

    return userView === "beginner"
      ? "This special setup still needs a safe install plan."
      : "This special setup still needs a safe guided plan before anything can move.";
  }

  if (safeCount === 0) {
    return userView === "beginner"
      ? "No files are ready to move from this batch yet."
      : "No safe hand-off is ready for this batch yet.";
  }

  return userView === "beginner"
    ? "This batch is ready for the normal safe hand-off."
    : "This batch can continue through the normal safe hand-off flow.";
}

function downloadsNextStepTitle(
  item: DownloadsInboxItem,
  guidedPlan: GuidedInstallPlan | null,
  specialDecision: SpecialModDecision | null,
  versionResolution: VersionResolution | null,
  reviewAction: ReviewPlanAction | null,
  canApply: boolean,
  safeCount: number,
  userView: UserView,
) {
  if (reviewAction?.kind === "repair_special") {
    return userView === "beginner"
      ? "Fix the old setup first"
      : "Repair the old special-mod setup";
  }

  if (reviewAction) {
    return reviewAction.label;
  }

  if (specialDecision) {
    if (specialDecision.sameVersion) {
      return userView === "beginner"
        ? "This special mod is already current"
        : "Installed version already matches";
    }

    if (specialDecision.versionStatus === "incoming_older") {
      return userView === "beginner"
        ? "This download is older than your installed copy"
        : "Incoming pack is older than installed";
    }

    if (specialDecision.applyReady && canApply) {
      return guidedPlan?.existingInstallDetected ||
        specialDecision.existingInstallState === "clean"
        ? userView === "beginner"
          ? "Update this special mod safely"
          : "Guided update is ready"
        : userView === "beginner"
          ? "Install this special mod safely"
          : "Guided install is ready";
    }

    if (specialDecision.queueLane === "done" && specialDecision.familyRole === "superseded") {
      return userView === "beginner"
        ? "This leftover pack is already covered"
        : "A fuller family pack is already installed";
    }

    return userView === "beginner"
      ? "Follow the next safe setup step"
      : "Use the safest next special-mod step";
  }

  if (versionResolution?.status === "same_version") {
    return userView === "beginner"
      ? "This download already matches your installed copy"
      : "Installed and incoming versions match";
  }

  if (versionResolution?.status === "incoming_older") {
    return userView === "beginner"
      ? "This download looks older than your installed copy"
      : "Incoming version looks older";
  }

  if (versionResolution?.status === "incoming_newer") {
    return canApply
      ? userView === "beginner"
        ? "This looks like a newer download"
        : "Incoming version looks newer"
      : userView === "beginner"
        ? "A newer copy was found"
        : "Newer incoming version found";
  }

  if (versionResolution?.matchedSubjectLabel) {
    return userView === "beginner"
      ? "SimSuite found a possible installed match"
      : "Installed match found, but still cautious";
  }

  if (item.intakeMode === "guided") {
    if (guidedPlan?.applyReady && canApply) {
      return guidedPlan.existingInstallDetected
        ? userView === "beginner"
          ? "Update this special mod safely"
          : "Guided update is ready"
        : userView === "beginner"
          ? "Install this special mod safely"
          : "Guided install is ready";
    }

    return userView === "beginner"
      ? "Check the guided setup first"
      : "Guided setup still needs review";
  }

  if (item.intakeMode === "needs_review") {
    return userView === "beginner"
      ? "Check what still needs review"
      : "Review is still blocking this batch";
  }

  if (item.intakeMode === "blocked") {
    return userView === "beginner"
      ? "Nothing can move from this batch"
      : "This batch is blocked";
  }

  if (safeCount > 0 && canApply) {
    return userView === "beginner"
      ? "Move the safe files from this batch"
      : "Apply the safe hand-off";
  }

  return userView === "beginner"
    ? "Nothing is ready to move yet"
    : "No safe hand-off is ready yet";
}

function downloadsNextStepDescription(
  item: DownloadsInboxItem,
  guidedPlan: GuidedInstallPlan | null,
  specialDecision: SpecialModDecision | null,
  versionResolution: VersionResolution | null,
  reviewAction: ReviewPlanAction | null,
  safeCount: number,
  userView: UserView,
) {
  if (reviewAction?.kind === "repair_special") {
    return userView === "beginner"
      ? "SimSuite can move the older files out of the way, keep your settings, and then continue the update."
      : "SimSuite found a safe repair path for the old install layout, so it can clear the older files out of the way and continue the update after approval.";
  }

  if (reviewAction) {
    return reviewActionDescription(reviewAction);
  }

  if (specialDecision) {
    if (specialDecision.sameVersion) {
      return userView === "beginner"
        ? "SimSuite checked the installed copy against this download and they match. Reinstall only if the current copy is damaged."
        : "SimSuite compared the installed copy with this incoming pack and found the same version. Reinstall only if you want to replace a damaged setup.";
    }

    if (specialDecision.versionStatus === "incoming_older") {
      return userView === "beginner"
        ? "This download looks older than the copy already in Mods, so SimSuite is not treating it as the next update."
        : "The installed special-mod family looks newer than this incoming pack, so SimSuite is holding the update action back.";
    }

    if (specialDecision.applyReady) {
      return userView === "beginner"
        ? "SimSuite has checked the files, the folder, and the update rules for this special mod."
        : "The backend has a full safe install or update plan ready for this special mod.";
    }

    return specialDecision.recommendedNextStep;
  }

  if (versionResolution?.status === "same_version") {
    return userView === "beginner"
      ? "SimSuite matched this download to the copy already in Mods and found the same version. You usually do not need to move it again."
      : "The shared compare found the same version on both sides. This looks like a duplicate copy unless you are replacing a damaged install on purpose.";
  }

  if (versionResolution?.status === "incoming_older") {
    return userView === "beginner"
      ? "The matching copy already in Mods looks newer than this download, so SimSuite is not treating this as the next update."
      : "The shared compare found a newer installed copy, so SimSuite is keeping this incoming batch out of the update path.";
  }

  if (versionResolution?.status === "incoming_newer") {
    return userView === "beginner"
      ? "SimSuite found a matching copy in Mods and this download looks newer."
      : "The shared compare matched this download to installed content and the incoming version looks newer.";
  }

  if (versionResolution?.matchedSubjectLabel) {
    return userView === "beginner"
      ? `SimSuite found a possible match in Mods for ${versionResolution.matchedSubjectLabel}, but it is not confident enough to make a firm version call.`
      : `SimSuite matched this download to ${versionResolution.matchedSubjectLabel}, but the local evidence is still too mixed for a firm version verdict.`;
  }

  if (item.intakeMode === "guided") {
    if (guidedPlan?.applyReady) {
      return userView === "beginner"
        ? "SimSuite knows where this mod should go, what it will replace, and what it will keep."
        : "The guided plan is ready and will still create a restore point before anything moves.";
    }

    return userView === "beginner"
      ? "SimSuite recognized the mod, but one more safety check is still needed."
      : "The download matches a known special mod, but the guided plan is not safe enough to apply yet.";
  }

  if (item.intakeMode === "needs_review") {
    return userView === "beginner"
      ? "SimSuite found something important but still needs one clear answer before it can continue."
      : "The batch needs one more clear answer before SimSuite can switch it into a safe path.";
  }

  if (item.intakeMode === "blocked") {
    return userView === "beginner"
      ? "The files or structure are not safe enough to move, so SimSuite stopped here."
      : "SimSuite stopped because the staged files or current install shape are not safe to continue.";
  }

  if (safeCount > 0) {
    return userView === "beginner"
      ? "Only the ready files will move. Anything uncertain will stay in the Inbox."
      : "Only the safe part of this batch will move. The rest stays visible for review.";
  }

  return userView === "beginner"
    ? "The batch is still being checked or still needs more clues."
    : "The batch has not reached a safe hand-off yet.";
}

function previewPanelTitle(
  intakeMode: DownloadIntakeMode | undefined,
  userView: UserView,
  guidedNeedsReview = false,
) {
  if (intakeMode === "guided") {
    if (guidedNeedsReview) {
      return userView === "beginner"
        ? "One more setup check is needed"
        : "Guided setup needs review";
    }
    return userView === "beginner" ? "How to install this safely" : "Guided install";
  }
  if (intakeMode === "blocked") {
    return userView === "beginner" ? "Why this was blocked" : "Blocked item";
  }
  if (intakeMode === "needs_review") {
    return userView === "beginner" ? "Why SimSuite stopped" : "Needs review";
  }
  return userView === "beginner" ? "What would move from this batch" : "Validated preview";
}

function applyButtonLabel(
  intakeMode: DownloadIntakeMode,
  guidedPlan: GuidedInstallPlan | null,
  specialDecision: SpecialModDecision | null,
  userView: UserView,
  isApplying: boolean,
  reviewPlan?: SpecialReviewPlan | null,
) {
  if (isApplying) {
    if (reviewPlan?.repairPlanAvailable && intakeMode !== "guided") {
      return userView === "beginner" ? "Fixing..." : "Repairing...";
    }
    if (specialDecision?.sameVersion) {
      return userView === "beginner" ? "Reinstalling..." : "Reinstalling...";
    }
    return intakeMode === "guided" ? "Installing..." : "Applying...";
  }

  if (intakeMode === "guided") {
    if (specialDecision?.sameVersion) {
      return userView === "beginner" ? "Reinstall anyway" : "Reinstall guided copy";
    }

    if (specialDecision?.versionStatus === "incoming_older") {
      return userView === "beginner" ? "Older version" : "Older than installed";
    }

    const existingInstallDetected =
      guidedPlan?.existingInstallDetected ??
      (specialDecision?.existingInstallState === "clean" ||
        specialDecision?.existingInstallState === "repairable");
    return userView === "beginner"
      ? existingInstallDetected
        ? "Update safely"
        : "Install safely"
      : existingInstallDetected
        ? "Apply guided update"
        : "Apply guided install";
  }

  if (intakeMode === "needs_review") {
    return reviewPlan?.repairPlanAvailable
      ? userView === "beginner"
        ? "Fix old setup"
        : "Run repair"
      : userView === "beginner"
        ? "Review needed first"
        : "Needs review first";
  }

  if (intakeMode === "blocked") {
    return reviewPlan?.repairPlanAvailable
      ? userView === "beginner"
        ? "Fix old setup"
        : "Run repair"
      : userView === "beginner"
        ? "Blocked"
        : "Blocked";
  }

  return userView === "beginner" ? "Move safe files" : "Apply safe batch";
}

function actionableCount(preview: OrganizationPreview | null) {
  if (!preview) {
    return 0;
  }

  return preview.suggestions.filter(
    (item) =>
      !item.reviewRequired &&
      Boolean(item.finalAbsolutePath) &&
      item.finalAbsolutePath !== item.currentPath,
  ).length;
}

function alignedCount(preview: OrganizationPreview | null) {
  if (!preview) {
    return 0;
  }

  return preview.suggestions.filter(
    (item) => item.finalAbsolutePath === item.currentPath,
  ).length;
}

function previewStateTone(state: "safe" | "review" | "aligned") {
  if (state === "safe") {
    return "good";
  }

  if (state === "review") {
    return "low";
  }

  return "neutral";
}

function previewStateLabel(state: "safe" | "review" | "aligned") {
  if (state === "safe") {
    return "Safe";
  }

  if (state === "review") {
    return "Needs review";
  }

  return "Already fine";
}

function formatVersionValue(version: string | null, isPresent: boolean) {
  if (version) {
    return version;
  }

  return isPresent ? "Found, but not labeled" : "Not installed";
}

function formatEvidenceSourceValue(
  source: string | null | undefined,
  hasComparableCopy: boolean,
) {
  if (!hasComparableCopy) {
    return "Not available";
  }

  if (!source) {
    return "No strong clue saved";
  }

  switch (source) {
    case "download name":
      return "Download name and file names";
    case "inside mod":
      return "Inside the mod files";
    case "installed files":
      return "Installed file names";
    case "saved family state":
      return "Last successful family record";
    case "file signature":
      return "Matching file fingerprint";
    default:
      return source;
  }
}

function specialVersionStatusLabel(
  decision: SpecialModDecision,
  userView: UserView,
) {
  switch (decision.versionStatus) {
    case "not_installed":
      return userView === "beginner" ? "Fresh install" : "Nothing installed yet";
    case "incoming_newer":
      return userView === "beginner" ? "Newer download" : "Incoming pack is newer";
    case "same_version":
      return userView === "beginner" ? "Already current" : "Installed and incoming match";
    case "incoming_older":
      return userView === "beginner"
        ? "Older than installed"
        : "Incoming pack looks older";
    default:
      return userView === "beginner" ? "Version unclear" : "Version could not be compared";
  }
}

function specialVersionTone(decision: SpecialModDecision) {
  switch (decision.versionStatus) {
    case "incoming_newer":
    case "not_installed":
      return "good";
    case "incoming_older":
      return "low";
    case "unknown":
      return "medium";
    default:
      return "neutral";
  }
}

function genericVersionStatusLabel(
  resolution: VersionResolution,
  userView: UserView,
) {
  switch (resolution.status) {
    case "not_installed":
      return userView === "beginner" ? "Fresh install" : "No installed match";
    case "incoming_newer":
      return userView === "beginner" ? "Newer download" : "Incoming looks newer";
    case "same_version":
      return userView === "beginner" ? "Already current" : "Installed and incoming match";
    case "incoming_older":
      return userView === "beginner" ? "Older than installed" : "Incoming looks older";
    default:
      return resolution.matchedSubjectLabel
        ? userView === "beginner"
          ? "Match unclear"
          : "Installed match, version unclear"
        : userView === "beginner"
          ? "Version unclear"
          : "Version could not be compared";
  }
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

function genericVersionTone(resolution: VersionResolution) {
  switch (resolution.status) {
    case "incoming_newer":
    case "not_installed":
      return "good";
    case "incoming_older":
      return "low";
    case "same_version":
      return "neutral";
    default:
      return resolution.matchedSubjectLabel ? "medium" : "neutral";
  }
}

function primaryInboxStateBadge(
  item: DownloadsInboxItem,
  userView: UserView,
): { label: string; tone: string } | null {
  if (item.specialDecision) {
    return {
      label: specialVersionStatusLabel(item.specialDecision, userView),
      tone: specialVersionTone(item.specialDecision),
    };
  }

  if (!item.versionResolution) {
    return null;
  }

  if (
    item.versionResolution.status === "not_installed" ||
    item.versionResolution.status === "unknown"
  ) {
    return item.versionResolution.matchedSubjectLabel
      ? {
          label: genericVersionStatusLabel(item.versionResolution, userView),
          tone: genericVersionTone(item.versionResolution),
        }
      : null;
  }

  return {
    label: genericVersionStatusLabel(item.versionResolution, userView),
    tone: genericVersionTone(item.versionResolution),
  };
}

function inboxItemTone(item: DownloadsInboxItem) {
  if (item.specialDecision) {
    return specialVersionTone(item.specialDecision);
  }

  if (item.versionResolution) {
    return genericVersionTone(item.versionResolution);
  }

  return itemStatusTone(item.status);
}

function friendlyItemStatus(status: string) {
  if (status === "needs_review") {
    return "Needs review";
  }

  if (status === "partial") {
    return "Partly ready";
  }

  if (status === "applied") {
    return "Applied";
  }

  if (status === "error") {
    return "Error";
  }

  if (status === "ignored") {
    return "Rejected";
  }

  return "Ready";
}

function itemStatusTone(status: string) {
  if (status === "ready") {
    return "good";
  }

  if (status === "partial" || status === "needs_review") {
    return "medium";
  }

  if (status === "error") {
    return "low";
  }

  return "neutral";
}

function intakeModeTone(mode: DownloadIntakeMode) {
  if (mode === "guided") {
    return "medium";
  }

  if (mode === "needs_review" || mode === "blocked") {
    return "low";
  }

  return "good";
}

function friendlyWatcherLabel(state: DownloadsWatcherStatus["state"]) {
  if (state === "watching") {
    return "Watching";
  }

  if (state === "processing") {
    return "Checking";
  }

  if (state === "error") {
    return "Error";
  }

  return "Idle";
}

function watcherTone(state: DownloadsWatcherStatus["state"]) {
  if (state === "watching") {
    return "good";
  }

  if (state === "processing") {
    return "medium";
  }

  if (state === "error") {
    return "low";
  }

  return "neutral";
}

function formatDate(value: string | null) {
  if (!value) {
    return "Not yet";
  }

  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    return value;
  }

  return date.toLocaleString();
}

function toErrorMessage(error: unknown) {
  if (error instanceof Error) {
    return error.message;
  }

  return String(error);
}

function isLockedDatabaseError(message: string) {
  const lowered = message.toLowerCase();
  return (
    lowered.includes("database is locked") ||
    lowered.includes("database table is locked") ||
    lowered.includes("database schema is locked") ||
    lowered.includes("database busy")
  );
}
