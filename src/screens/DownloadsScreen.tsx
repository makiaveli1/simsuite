import {
  startTransition,
  useDeferredValue,
  useEffect,
  useRef,
  useState,
} from "react";
import { m } from "motion/react";
import {
  AlertTriangle,
  Ban,
  Download,
  FolderSearch,
  Inbox,
  LoaderCircle,
  RefreshCw,
  ShieldAlert,
  Workflow,
} from "lucide-react";
import {
  DockSectionStack,
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
import { rowHover, rowPress, stagedListItem } from "../lib/motion";
import {
  friendlyTypeLabel,
  intakeModeLabel,
  reviewLabel,
  riskLevelLabel,
  sampleCountLabel,
  sampleToggleLabel,
  screenHelperLine,
} from "../lib/uiLanguage";
import type {
  DependencyStatus,
  DownloadInboxDetail,
  DownloadIntakeMode,
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
  selectedItemId: number | null;
  selectedPreset: string;
  search: string;
  statusFilter: string;
}

const AUTO_RECHECK_NOTE_PREFIX = "Rechecked with newer SimSuite rules";
const DEFAULT_DOWNLOADS_PRESET = "Category First";
const WORKSPACE_RELOAD_GRACE_MS = 1200;
const downloadsScreenCache: DownloadsScreenCache = {
  refreshVersion: -1,
  watcherStatus: null,
  inbox: null,
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
  const [inbox, setInbox] = useState<DownloadsInboxResponse | null>(
    downloadsScreenCache.inbox,
  );
  const [presets, setPresets] = useState<RulePreset[]>([]);
  const [selectedPreset, setSelectedPreset] = useState(
    downloadsScreenCache.selectedPreset,
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
  const [isLoadingInbox, setIsLoadingInbox] = useState(false);
  const [isRefreshing, setIsRefreshing] = useState(false);
  const [isLoadingSelection, setIsLoadingSelection] = useState(false);
  const [isApplying, setIsApplying] = useState(false);
  const [isIgnoring, setIsIgnoring] = useState(false);
  const latestWatcherStatus = useRef<DownloadsWatcherStatus | null>(
    downloadsScreenCache.watcherStatus,
  );
  const latestInbox = useRef<DownloadsInboxResponse | null>(
    downloadsScreenCache.inbox,
  );
  const latestBusyState = useRef({
    isLoadingInbox: false,
    isApplying: false,
    isIgnoring: false,
    isRefreshing: false,
  });
  const inboxRetryTimer = useRef<number | null>(null);
  const watcherPollTimer = useRef<number | null>(null);
  const workspaceReloadTimer = useRef<number | null>(null);
  const queueRequestId = useRef(0);
  const selectionRequestId = useRef(0);
  const pendingWorkspaceReload = useRef(false);
  const pendingPreferredSelectionId = useRef<number | null>(null);
  const previousRefreshVersion = useRef(refreshVersion);
  const latestRefreshVersion = useRef(refreshVersion);
  const skipWorkspaceReloadUntil = useRef(0);
  const deferredSearch = useDeferredValue(search);

  useEffect(() => {
    latestRefreshVersion.current = refreshVersion;
    latestWatcherStatus.current = watcherStatus;
    latestInbox.current = inbox;
    latestBusyState.current = {
      isLoadingInbox,
      isApplying,
      isIgnoring,
      isRefreshing,
    };
    downloadsScreenCache.refreshVersion = refreshVersion;
    downloadsScreenCache.watcherStatus = watcherStatus;
    downloadsScreenCache.inbox = inbox;
    downloadsScreenCache.selectedItemId = selectedItemId;
    downloadsScreenCache.selectedPreset = selectedPreset;
    downloadsScreenCache.search = search;
    downloadsScreenCache.statusFilter = statusFilter;
  }, [
    inbox,
    isApplying,
    isIgnoring,
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
    isIgnoring,
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
    isIgnoring,
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
    const items = inbox?.items ?? [];
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

    if (!items.some((item) => item.id === selectedItemId)) {
      setSelectedItemId(items[0].id);
    }
  }, [inbox, selectedItemId]);

  const selectedItem =
    inbox?.items.find((item) => item.id === selectedItemId) ?? null;

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
      busyState.isIgnoring ||
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

  async function handleReviewAction(action: ReviewPlanAction) {
    if (!selectedItem) {
      return;
    }

    const needsApproval = reviewActionNeedsApproval(action.kind);
    if (needsApproval) {
      const confirmed = globalThis.confirm(
        reviewActionConfirmation(
          action,
          selectedItem.displayName,
          selectedReviewPlan,
          userView,
        ),
      );
      if (!confirmed) {
        return;
      }
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

  async function handleApply() {
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
      const confirmed = globalThis.confirm(
        isSameVersion
          ? `Reinstall ${selectedGuidedPlan.profileName}? SimSuite checked this download against the installed copy and they match. It will only replace the current files, keep ${selectedGuidedPlan.preserveFiles.length} settings file(s), and create a restore point first.`
          : `Install ${selectedGuidedPlan.profileName} safely? SimSuite will replace ${selectedGuidedPlan.replaceFiles.length} old file(s), keep ${selectedGuidedPlan.preserveFiles.length} settings file(s), and create a restore point first.`,
      );
      if (!confirmed) {
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

    const confirmed = globalThis.confirm(
      `Move ${safeCount} safe file(s) from ${selectedItem.displayName}? Files that need review will stay in the inbox, and a restore point will be created first.`,
    );
    if (!confirmed) {
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
      onDataChanged();
      await reloadInboxAfterMutation();
    } catch (error) {
      setErrorMessage(toErrorMessage(error));
    } finally {
      setIsApplying(false);
    }
  }

  async function handleIgnore() {
    if (!selectedItem) {
      return;
    }

    const confirmed = globalThis.confirm(
      `Hide ${selectedItem.displayName} from the inbox? This keeps it out of the active queue until the download changes again.`,
    );
    if (!confirmed) {
      return;
    }

    setIsIgnoring(true);
    setStatusMessage(null);
    setErrorMessage(null);

    try {
      await api.ignoreDownloadItem(selectedItem.id);
      setStatusMessage(`${selectedItem.displayName} was removed from the active inbox.`);
      onDataChanged();
      await reloadInboxAfterMutation();
    } catch (error) {
      setErrorMessage(toErrorMessage(error));
    } finally {
      setIsIgnoring(false);
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
  const reviewActions = selectedSpecialDecision
    ? buildDecisionActions(selectedSpecialDecision, selectedReviewPlan)
    : selectedReviewPlan
      ? buildReviewActions(selectedReviewPlan)
      : [];
  const primaryReviewAction =
    selectedSpecialDecision?.primaryAction ?? reviewActions[0] ?? null;
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
  const groupedItems = groupDownloadItems(inbox?.items ?? []);
  const visibleLaneCounts = Object.fromEntries(
    DOWNLOAD_LANE_ORDER.map((lane) => [
      lane,
      groupedItems.find((group) => group.lane === lane)?.items.length ?? 0,
    ]),
  ) as Record<DownloadQueueLane, number>;
  const splitStage = userView !== "beginner";
  const inspectorSignals = selectedResolvedItem
    ? buildDownloadInspectorSignals(
        selectedResolvedItem,
        selectedSpecialDecision,
        selectedVersionResolution,
        selectedReviewPlan,
        selectedAutoRecheckNote,
      )
    : [];
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

  return (
    <section className="screen-shell downloads-shell">
      {/* Slim strip with watcher state and queue summary */}
      <div className="slim-strip">
        <div className="slim-strip-group">
          {statusMessage ? (
            <span className="health-chip is-warn">{statusMessage}</span>
          ) : errorMessage ? (
            <span className="health-chip is-danger">{errorMessage}</span>
          ) : (
            <>
              <span className="health-chip is-good">
                <span className="health-chip-dot"></span>
                {overview?.totalItems ?? 0} items
              </span>
              <span className="health-chip">
                {overview?.readyNowItems ?? overview?.readyItems ?? 0} ready
              </span>
              <span className={`health-chip ${(overview?.waitingOnYouItems ?? overview?.needsReviewItems ?? 0) > 0 ? 'is-warn' : ''}`}>
                {overview?.waitingOnYouItems ?? overview?.needsReviewItems ?? 0} waiting
              </span>
              <span className={`health-chip ${(overview?.blockedItems ?? overview?.errorItems ?? 0) > 0 ? 'is-danger' : ''}`}>
                {overview?.blockedItems ?? overview?.errorItems ?? 0} blocked
              </span>
            </>
          )}
        </div>
        <div className="slim-strip-group">
          <button
            type="button"
            className="secondary-action"
            onClick={() => void handleRefresh()}
            disabled={isRefreshing || isLoadingInbox}
          >
            <RefreshCw size={14} strokeWidth={2} />
            {isRefreshing ? "Refreshing..." : "Refresh"}
          </button>
          <button
            type="button"
            className="secondary-action"
            onClick={() => onNavigate("review")}
          >
            <ShieldAlert size={14} strokeWidth={2} />
            {reviewLabel(userView)}
          </button>
        </div>
      </div>

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
            <div className="downloads-rail">
              <div className="workbench-header">
                <div>
                  <p className="eyebrow">Workspace</p>
                  <h1 className="downloads-rail-title">Downloads</h1>
                  <p className="downloads-rail-copy">
                    {screenHelperLine("downloads", userView)}
                  </p>
                </div>
                <span
                  className={`confidence-badge ${watcherTone(
                    activeWatcherStatus.state,
                  )}`}
                >
                  {friendlyWatcherLabel(activeWatcherStatus.state)}
                </span>
              </div>

              <div className="downloads-rail-section">
                <div className="section-label">Watcher</div>
                <div className="downloads-rail-card downloads-watch-card">
                  <div className="path-card">
                    {activeWatcherStatus.watchedPath ?? "No downloads folder set"}
                  </div>
                  <div className="downloads-watch-meta">
                    <span className="ghost-chip">
                      {activeWatcherStatus.activeItems.toLocaleString()} active item(s)
                    </span>
                    {activeWatcherStatus.currentItem ? (
                      <span className="ghost-chip">{activeWatcherStatus.currentItem}</span>
                    ) : null}
                    {activeWatcherStatus.lastRunAt ? (
                      <span className="ghost-chip">
                        Last check {formatDate(activeWatcherStatus.lastRunAt)}
                      </span>
                    ) : null}
                  </div>
                </div>
              </div>

              <div className="downloads-rail-section">
                <div className="section-label">Controls</div>
                <div className="downloads-rail-card downloads-filter-card">
                  <div className="filter-grid">
                    <label className="field">
                      <span>Search</span>
                      <input
                        value={search}
                        onChange={(event) => setSearch(event.target.value)}
                        placeholder="Archive, file, or creator"
                      />
                    </label>

                    <label className="field">
                      <span>{userView === "beginner" ? "Show" : "Status"}</span>
                      <select
                        value={statusFilter}
                        onChange={(event) => setStatusFilter(event.target.value)}
                      >
                        <option value="">All items</option>
                        <option value="ready">Ready</option>
                        <option value="partial">Partial</option>
                        <option value="needs_review">Needs review</option>
                        <option value="applied">Applied</option>
                        <option value="error">Error</option>
                        <option value="ignored">Ignored</option>
                      </select>
                    </label>

                    <label className="field">
                      <span>{userView === "beginner" ? "Tidy style" : "Rule set"}</span>
                      <select
                        value={selectedPreset}
                        onChange={(event) => setSelectedPreset(event.target.value)}
                        disabled={selectedItem?.intakeMode === "guided"}
                      >
                        {presets.map((preset) => (
                          <option key={preset.name} value={preset.name}>
                            {preset.name}
                          </option>
                        ))}
                      </select>
                    </label>
                  </div>
                  {selectedItem?.intakeMode === "guided" ? (
                    <p className="downloads-rail-note">
                      This batch has its own install rules, so the tidy style stays locked.
                    </p>
                  ) : null}
                </div>
              </div>

              <div className="downloads-rail-section">
                <div className="section-label">Queue lanes</div>
                <div className="downloads-lane-summary-list">
                  {DOWNLOAD_LANE_ORDER.map((lane) => (
                    <div key={lane} className="downloads-lane-summary">
                      <div>
                        <strong>{queueLaneLabel(lane, userView)}</strong>
                        <span>{queueLaneHint(lane, userView)}</span>
                      </div>
                      <span className="ghost-chip">
                        {visibleLaneCounts[lane].toLocaleString()}
                      </span>
                    </div>
                  ))}
                </div>
              </div>

              <div className="downloads-rail-section">
                <div className="section-label">Quick actions</div>
                <div className="downloads-rail-actions">
                  <button
                    type="button"
                    className="primary-action"
                    onClick={() => void handleRefresh()}
                    disabled={isRefreshing || isLoadingInbox}
                  >
                    <RefreshCw size={14} strokeWidth={2} />
                    {isRefreshing ? "Refreshing..." : "Refresh inbox"}
                  </button>
                  <button
                    type="button"
                    className="secondary-action"
                    onClick={() => onNavigate("review")}
                  >
                    <ShieldAlert size={14} strokeWidth={2} />
                    {reviewLabel(userView)}
                  </button>
                  <button
                    type="button"
                    className="secondary-action"
                    onClick={() => onNavigate("home")}
                  >
                    <FolderSearch size={14} strokeWidth={2} />
                    Home folders
                  </button>
                </div>
              </div>
            </div>
          </WorkbenchRail>

          <WorkbenchStage className="downloads-stage-panel">
            <div className="table-header downloads-stage-header">
              <div className="table-meta">
                <div>
                  <strong>{(inbox?.items.length ?? 0).toLocaleString()}</strong>
                  <span>visible</span>
                </div>
                <div>
                  <strong>{selectedItem ? "1" : "0"}</strong>
                  <span>selected</span>
                </div>
                <div>
                  <strong>{selectedPreset}</strong>
                  <span>{userView === "beginner" ? "tidy style" : "rule set"}</span>
                </div>
              </div>
              <div className="downloads-stage-status">
                <span className="ghost-chip">{stageStatusMessage}</span>
              </div>
            </div>

            <div className={`downloads-stage${splitStage ? " downloads-stage-split" : ""}`}>
              <div className="panel-card downloads-queue-panel workbench-panel">
                <div className="panel-heading">
                  <div>
                    <p className="eyebrow">Inbox queue</p>
                    <h2>{userView === "beginner" ? "What just arrived" : "Download items"}</h2>
                  </div>
                  <span className="ghost-chip">
                    {isLoadingInbox ? "Loading..." : `${inbox?.items.length ?? 0} shown`}
                  </span>
                </div>

                <div className="vertical-dock downloads-queue-dock">
                  <div className="queue-list downloads-queue-list">
                    {inbox?.items.length ? (
                      groupedItems.map((group) => (
                        <div key={group.lane} className="downloads-lane-group">
                          <div className="downloads-lane-header">
                            <div>
                              <strong>{queueLaneLabel(group.lane, userView)}</strong>
                              {userView === "beginner" ? (
                                <span>{queueLaneHint(group.lane, userView)}</span>
                              ) : null}
                            </div>
                            <span className="ghost-chip">
                              {group.items.length.toLocaleString()}
                            </span>
                          </div>

                          <div className="downloads-lane-list">
                            {group.items.map((item, index) => {
                              const primaryBadge = primaryInboxStateBadge(item, userView);
                              const rowTone = inboxItemTone(item);

                              return (
                              <m.button
                                key={item.id}
                                type="button"
                                className={`downloads-item-row ${
                                  selectedItemId === item.id ? "is-selected" : ""
                                } downloads-item-row-${rowTone}`}
                                onClick={() => {
                                  setStatusMessage(null);
                                  setSelectedItemId(item.id);
                                }}
                                title={item.sourcePath}
                                whileHover={rowHover}
                                whileTap={rowPress}
                                {...stagedListItem(index)}
                              >
                                <div className="downloads-item-main">
                                  <strong>{item.displayName}</strong>
                                  <span>
                                    {item.sourceKind === "archive" ? "Archive" : "Direct file"} ·{" "}
                                    {item.detectedFileCount.toLocaleString()} file(s)
                                    {userView === "power" && item.archiveFormat
                                      ? ` · ${item.archiveFormat.toUpperCase()}`
                                      : ""}
                                  </span>
                                  <div className="downloads-item-samples">
                                    {item.queueSummary ?? fallbackQueueSummary(item)}
                                  </div>
                                  {item.sampleFiles.length ? (
                                    <div className="downloads-item-samples downloads-item-samples-muted">
                                      {item.sampleFiles.slice(0, 3).join(" · ")}
                                    </div>
                                  ) : null}
                                </div>
                                <div className="downloads-item-meta">
                                  {findAutoRecheckNote(item.notes) ? (
                                    <span className="ghost-chip">Rechecked</span>
                                  ) : null}
                                  {item.relatedItemIds?.length ? (
                                    <span className="ghost-chip">
                                      Linked {item.relatedItemIds.length + 1}
                                    </span>
                                  ) : null}
                                  {primaryBadge ? (
                                    <span className={`confidence-badge ${primaryBadge.tone}`}>
                                      {primaryBadge.label}
                                    </span>
                                  ) : (
                                    <>
                                      <span
                                        className={`confidence-badge ${intakeModeTone(item.intakeMode)}`}
                                      >
                                        {intakeModeLabel(item.intakeMode)}
                                      </span>
                                      <span
                                        className={`confidence-badge ${itemStatusTone(item.status)}`}
                                      >
                                        {friendlyItemStatus(item.status)}
                                      </span>
                                    </>
                                  )}
                                </div>
                              </m.button>
                              );
                            })}
                          </div>
                        </div>
                      ))
                    ) : (
                      <StatePanel
                        eyebrow="Downloads inbox"
                        title={
                          userView === "beginner"
                            ? "No inbox items match this view"
                            : "No download items match the current filter"
                        }
                        body={
                          userView === "beginner"
                            ? "Try clearing the search, changing the filter, or refresh the inbox after a new download lands."
                            : "Clear the search, adjust status filters, or refresh the inbox to pull in newly detected downloads."
                        }
                        icon={Inbox}
                        compact
                        badge="Queue clear"
                        meta={["Filters stay local to this workspace"]}
                      />
                    )}
                  </div>

                  {splitStage ? null : (
                    <ResizableEdgeHandle
                      label="Resize download queue height"
                      value={downloadsQueueHeight}
                      min={220}
                      max={720}
                      onChange={setDownloadsQueueHeight}
                      side="bottom"
                      className="dock-resize-handle downloads-queue-height-handle"
                    />
                  )}
                </div>
              </div>
              <div className="panel-card downloads-preview-panel workbench-panel">
                <div className="panel-heading">
                  <div>
                    <p className="eyebrow">
                      {selectedItem?.intakeMode === "guided"
                        ? guidedNeedsReview
                          ? "Decision panel"
                          : "Special setup"
                        : selectedItem?.intakeMode === "standard"
                          ? "Safe hand-off"
                          : "Decision panel"}
                    </p>
                    <h2>{previewPanelTitle(selectedItem?.intakeMode, userView, guidedNeedsReview)}</h2>
                  </div>
                  {selectedItem ? (
                    <span className="ghost-chip">
                      {selectedItem.intakeMode === "guided" && selectedGuidedPlan
                        ? guidedNeedsReview && selectedReviewPlan
                          ? `${selectedReviewPlan.reviewFiles.length.toLocaleString()} tracked`
                          : `${selectedGuidedPlan.installFiles.length.toLocaleString()} install`
                        : (selectedItem.intakeMode === "needs_review" ||
                              selectedItem.intakeMode === "blocked") &&
                            selectedReviewPlan
                          ? `${selectedReviewPlan.reviewFiles.length.toLocaleString()} tracked`
                        : previewSuggestions.length
                          ? `${previewSuggestions.length.toLocaleString()} shown`
                          : `${selectedFiles.length.toLocaleString()} tracked`}
                    </span>
                  ) : null}
                </div>

                <div className="preview-list downloads-preview-list">
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
                      title={
                        userView === "beginner"
                          ? "Select an inbox item"
                          : "Select a download item to inspect"
                      }
                        body={
                          userView === "beginner"
                            ? "Pick one batch from the queue to see whether it is a normal sort, a special setup, or something that needs review."
                            : "Select a staged archive or file batch to load the correct inbox preview."
                        }
                      icon={Download}
                      compact
                      meta={["Normal", "Special setup", "Needs review", "Blocked"]}
                    />
                  )}
                </div>
              </div>
            </div>
          </WorkbenchStage>

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
              <>
                <div className="detail-header">
                  <div>
                    <p className="eyebrow">
                      {userView === "beginner" ? "Selected batch" : "Selected inbox item"}
                    </p>
                    <h2>{selectedItem.displayName}</h2>
                    <p className="workspace-toolbar-copy">
                      {selectedItem.queueSummary ?? fallbackQueueSummary(selectedItem)}
                    </p>
                  </div>
                  <div className="downloads-detail-badges">
                    <span className="ghost-chip">
                      {queueLaneLabel(selectedItem.queueLane ?? deriveQueueLane(selectedItem), userView)}
                    </span>
                    {selectedStateBadge ? (
                      <span className={`confidence-badge ${selectedStateBadge.tone}`}>
                        {selectedStateBadge.label}
                      </span>
                    ) : (
                      <>
                        <span
                          className={`confidence-badge ${intakeModeTone(selectedItem.intakeMode)}`}
                        >
                          {intakeModeLabel(selectedItem.intakeMode)}
                        </span>
                        <span
                          className={`confidence-badge ${itemStatusTone(selectedItem.status)}`}
                        >
                          {friendlyItemStatus(selectedItem.status)}
                        </span>
                      </>
                    )}
                  </div>
                </div>

                {inspectorSignals.length ? (
                  <div className="downloads-signal-strip">
                    {inspectorSignals.map((signal) => (
                      <div
                        key={signal.id}
                        className={`downloads-signal-card downloads-signal-card-${signal.tone}`}
                      >
                        <span className="downloads-signal-label">{signal.label}</span>
                        <strong>{signal.title}</strong>
                        <span>{signal.body}</span>
                      </div>
                    ))}
                  </div>
                ) : null}

                <div className="downloads-next-step-card">
                  <div className="downloads-next-step-copy">
                    <p className="eyebrow">
                      {userView === "beginner" ? "Safe next step" : "Next move"}
                    </p>
                    <strong className="downloads-next-step-title">{nextStepTitle}</strong>
                    <p className="downloads-next-step-description">{nextStepDescription}</p>
                  </div>
                  <div className="downloads-next-step-actions">
                    {showPrimaryAction ? (
                      <button
                        type="button"
                        className="primary-action"
                        onClick={() =>
                          void (
                            primaryReviewAction
                              ? handleReviewAction(primaryReviewAction)
                              : handleApply()
                          )
                        }
                        disabled={primaryActionDisabled}
                      >
                        <Workflow size={14} strokeWidth={2} />
                        {applyLabel}
                      </button>
                    ) : null}
                    <button
                      type="button"
                      className="secondary-action"
                      onClick={() => void handleIgnore()}
                      disabled={isIgnoring}
                    >
                      <Ban size={14} strokeWidth={2} />
                      {isIgnoring ? "Ignoring..." : "Ignore"}
                    </button>
                  </div>
                  {!showPrimaryAction ? (
                    <div className="downloads-inspector-note">
                      {downloadsInspectorIdleNote(
                        effectiveSelectedIntakeMode ?? selectedItem.intakeMode,
                        userView,
                        safeCount,
                        selectedGuidedPlan,
                        selectedSpecialDecision,
                        selectedVersionResolution,
                        selectedReviewPlan,
                      )}
                    </div>
                  ) : null}
                </div>

                <DockSectionStack
                  layoutId="downloadsInspector"
                  sections={inspectorSections}
                  intro="Reset this side panel"
                />
              </>
            ) : (
              <StatePanel
                eyebrow={userView === "beginner" ? "Downloads inbox" : "Inbox"}
                title={
                  userView === "beginner"
                    ? "Select a batch"
                    : "Select an inbox item to inspect"
                }
                body={
                  userView === "beginner"
                    ? "The right panel shows what the batch contains, what can move safely, and whether it needs special setup."
                    : "The inspector shows intake mode, evidence, and the file set for the selected batch."
                }
                icon={Download}
                meta={["Approval first", "Snapshots happen before moves"]}
              />
            )}
          </WorkbenchInspector>
        </Workbench>
      )}
    </section>
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

      {plan.dependencies.length ? (
        <div className="downloads-guided-card downloads-guided-card-neutral">
          <div className="downloads-guided-card-header">
            <strong>{userView === "beginner" ? "What it depends on" : "Dependencies"}</strong>
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
      ) : null}

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

      {plan.postInstallNotes.length ? (
        <div className="downloads-guided-card downloads-guided-card-neutral">
          <div className="downloads-guided-card-header">
            <strong>{userView === "beginner" ? "What to remember" : "Post-install notes"}</strong>
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

      {userView !== "beginner" ? (
        <div className="downloads-guided-evidence">
          <div className="section-label">
            {userView === "power" ? "Matched evidence" : "Why SimSuite chose this"}
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
      ) : null}
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
              className="primary-action"
              onClick={() => void onResolveAction(repairAction)}
              disabled={isApplying}
            >
              <Workflow size={14} strokeWidth={2} />
              {reviewActionLabel(repairAction, userView, isApplying)}
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
                  <strong>{reviewActionLabel(action, userView, false)}</strong>
                  <span>{reviewActionDescription(action)}</span>
                </div>
                <button
                  type="button"
                  className="primary-action"
                  onClick={() => void onResolveAction(action)}
                  disabled={isApplying}
                >
                  <Workflow size={14} strokeWidth={2} />
                  {reviewActionLabel(action, userView, isApplying)}
                </button>
              </div>
            ))}
          </div>
        </div>
      ) : null}

      {reviewPlan.dependencies.length ? (
        <div className="downloads-guided-card downloads-guided-card-neutral">
          <div className="downloads-guided-card-header">
            <strong>{userView === "beginner" ? "What it depends on" : "Dependencies"}</strong>
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
      ) : null}

      {reviewPlan.incompatibilityWarnings.length ? (
        <div className="downloads-guided-warnings">
          {reviewPlan.incompatibilityWarnings.map((warning) => (
            <div key={warning} className="status-banner status-banner-error">
              {warning}
            </div>
          ))}
        </div>
      ) : null}

      <GuidedListCard
        title={userView === "beginner" ? "Files SimSuite stopped on" : "Tracked review files"}
        badge={trackedFiles.length.toString()}
        tone={item.intakeMode === "blocked" ? "medium" : "neutral"}
        files={trackedFiles}
        userView={userView}
        showPaths={userView === "power"}
      />

      <div className="downloads-guided-card downloads-guided-card-neutral">
        <div className="downloads-guided-card-header">
          <strong>{userView === "beginner" ? "Why SimSuite decided this" : "Evidence"}</strong>
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

function reviewActionConfirmation(
  action: ReviewPlanAction,
  itemName: string,
  reviewPlan: SpecialReviewPlan | null,
  userView: UserView,
) {
  switch (action.kind) {
    case "repair_special":
      return userView === "beginner"
        ? `${action.label}? SimSuite will make a restore point, move the older files out of the way, keep your settings files, and then continue the special-mod update.`
        : `${action.label}? SimSuite will create a restore point, clear the older install out of the way, keep ${reviewPlan?.repairKeepFiles.length ?? 0} setting file(s), and continue the special update when it is safe.`;
    case "install_dependency":
      return `${action.label}? SimSuite will safely set up ${action.relatedItemName ?? "the helper mod"} first, create a restore point, and then re-check ${itemName}.`;
    case "download_missing_files":
      return userView === "beginner"
        ? `${action.label}? SimSuite will download the trusted official file into the Inbox first, then check the full set again before anything moves.`
        : `${action.label}? SimSuite will fetch the trusted official archive into Inbox staging, re-check the batch, and only continue if the special install is safe.`;
    case "separate_supported_files":
      return userView === "beginner"
        ? `${action.label}? SimSuite will pull the clean supported files into their own batch and leave the extra files behind so nothing gets mixed up.`
        : `${action.label}? SimSuite will split the supported special-mod files into a clean batch and keep the leftovers in a separate review item.`;
    default:
      return action.description;
  }
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
      ? "A fuller pack from this family is already installed, so this leftover download can stay ignored."
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
    return "Ignored";
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
