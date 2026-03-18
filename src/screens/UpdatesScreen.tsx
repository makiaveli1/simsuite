import { useEffect, useState } from "react";
import { m } from "motion/react";
import {
  AlertTriangle,
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
} from "lucide-react";
import { Workbench } from "../components/layout/Workbench";
import { WorkbenchInspector } from "../components/layout/WorkbenchInspector";
import { WorkbenchRail } from "../components/layout/WorkbenchRail";
import { WorkbenchStage } from "../components/layout/WorkbenchStage";
import { useUiPreferences } from "../components/UiPreferencesContext";
import { api } from "../lib/api";
import { rowHover, rowPress, stagedListItem } from "../lib/motion";
import {
  friendlyTypeLabel,
  screenHelperLine,
  unknownCreatorLabel,
} from "../lib/uiLanguage";
import type {
  FileDetail,
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

interface UpdatesScreenProps {
  refreshVersion: number;
  onNavigate: (screen: Screen) => void;
  onDataChanged?: () => void;
  userView: UserView;
  initialMode?: "tracked" | "setup" | "review";
  initialFilter?: WatchListFilter;
  initialFileId?: number;
}

type UpdateMode = "tracked" | "setup" | "review";
type ReviewFilter = "all" | "provider_needed" | "reference_only" | "unknown_result";

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

const REVIEW_FILTERS: Array<{
  id: ReviewFilter;
  label: string;
  beginnerLabel: string;
}> = [
  { id: "all", label: "All review items", beginnerLabel: "All review items" },
  { id: "provider_needed", label: "Provider needed", beginnerLabel: "Provider needed" },
  { id: "reference_only", label: "Reference only", beginnerLabel: "Reference only" },
  { id: "unknown_result", label: "Unknown result", beginnerLabel: "Unknown result" },
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
      beginner: "Confirmed update available",
      advanced: "Exact update available",
    },
    possible_update: { beginner: "Possible update", advanced: "Possible update" },
    unknown: { beginner: "Still unclear", advanced: "Unknown result" },
    current: { beginner: "Looks up to date", advanced: "Current" },
    not_watched: { beginner: "Not tracked yet", advanced: "Not watched" },
  };

  return userView === "beginner" ? labels[status].beginner : labels[status].advanced;
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
    return userView === "beginner" ? "No source saved yet" : "No saved source";
  }

  switch (watchResult.capability) {
    case "can_refresh_now":
      return userView === "beginner" ? "Can check right now" : "Check now supported";
    case "provider_required":
      return watchResult.providerName
        ? `${watchResult.providerName} setup needed`
        : "Provider setup needed";
    default:
      return userView === "beginner" ? "Saved as a reminder" : "Reference only";
  }
}

function reviewReasonLabel(reason: ReviewFilter, userView: UserView) {
  switch (reason) {
    case "provider_needed":
      return userView === "beginner" ? "Provider needed" : "Provider needed";
    case "reference_only":
      return userView === "beginner" ? "Reminder only" : "Reference only";
    case "unknown_result":
      return userView === "beginner" ? "Still unclear" : "Unknown result";
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

function trackedEmptyMessage(filter: WatchListFilter, userView: UserView) {
  switch (filter) {
    case "exact_updates":
      return userView === "beginner"
        ? "No confirmed updates are waiting right now."
        : "No tracked exact updates are waiting.";
    case "possible_updates":
      return userView === "beginner"
        ? "No possible updates are waiting right now."
        : "No tracked possible updates are waiting.";
    case "unclear":
      return userView === "beginner"
        ? "No tracked items are unclear right now."
        : "No tracked unclear results right now.";
    case "all":
      return userView === "beginner"
        ? "No items are tracked yet."
        : "No tracked items yet.";
    default:
      return userView === "beginner"
        ? "Nothing tracked needs attention right now."
        : "No tracked items need attention right now.";
  }
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
    sidebarWidth,
    setSidebarWidth,
    updatesFiltersCollapsed,
    setUpdatesFiltersCollapsed,
  } = useUiPreferences();
  const [mode, setMode] = useState<UpdateMode>(initialMode ?? "tracked");
  const [filter, setFilter] = useState<WatchListFilter>(initialFilter ?? "attention");
  const [reviewFilter, setReviewFilter] = useState<ReviewFilter>("all");
  const [trackedList, setTrackedList] = useState<LibraryWatchListResponse | null>(null);
  const [setupList, setSetupList] = useState<LibraryWatchSetupResponse | null>(null);
  const [reviewList, setReviewList] = useState<LibraryWatchReviewResponse | null>(null);
  const [selectedItem, setSelectedItem] = useState<FileDetail | null>(null);
  const [loadingTracked, setLoadingTracked] = useState(false);
  const [loadingSetup, setLoadingSetup] = useState(false);
  const [loadingReview, setLoadingReview] = useState(false);
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
    if (initialMode) {
      setMode(initialMode);
      setWatchEditing(initialMode === "setup");
    }
  }, [initialMode]);

  useEffect(() => {
    if (initialFilter) {
      setFilter(initialFilter);
    }
  }, [initialFilter]);

  useEffect(() => {
    if (mode === "tracked") {
      void loadTrackedList();
      return;
    }

    if (mode === "setup") {
      void loadSetupList();
      return;
    }

    void loadReviewList();
  }, [refreshVersion, mode, filter]);

  useEffect(() => {
    if (!initialFileId) {
      return;
    }

    void loadSelectedFile(initialFileId, { edit: initialMode === "setup" });
  }, [initialFileId, initialMode]);

  async function loadTrackedList(skipLoading = false, nextFilter = filter) {
    if (!skipLoading) {
      setLoadingTracked(true);
    }

    try {
      const result = await api.listLibraryWatchItems(nextFilter);
      setTrackedList(result);
    } catch (error) {
      console.error("Could not load tracked update items.", error);
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
      const result = await api.listLibraryWatchSetupItems();
      setSetupList(result);
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
      const result = await api.listLibraryWatchReviewItems();
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
    await Promise.all([
      loadTrackedList(true),
      loadSetupList(true),
      loadReviewList(true),
    ]);
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
    await loadSelectedFile(item.fileId);
  }

  async function handleSelectSetupItem(item: LibraryWatchSetupItem) {
    await loadSelectedFile(item.fileId, {
      edit: true,
      sourceKind: item.suggestedSourceKind,
      sourceLabel:
        item.suggestedSourceKind === "creator_page"
          ? item.creator ?? item.subjectLabel
          : item.subjectLabel,
      sourceUrl: "",
    });
  }

  async function handleSelectReviewItem(item: LibraryWatchReviewItem) {
    await loadSelectedFile(item.fileId);
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
      setSelectedItem(detail);
      syncWatchFields(detail);
      setWatchEditing(false);
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

    setClearingWatch(true);
    setMessage(null);

    try {
      const detail = await api.clearWatchSourceForFile(selectedItem.id);
      setSelectedItem(detail);
      syncWatchFields(detail);
      setWatchEditing(false);
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

      setSelectedItem(detail);
      syncWatchFields(detail);
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
          ? "Checked 1 tracked page."
          : `Checked ${summary.checkedSubjects} tracked pages.`;
      const updateLabel =
        summary.exactUpdateItems === 1
          ? "1 confirmed update found."
          : `${summary.exactUpdateItems} confirmed updates found.`;
      setMessage(`${checkedLabel} ${updateLabel}`);
    } catch (error) {
      setMessage(error instanceof Error ? error.message : "Could not refresh tracked pages.");
    } finally {
      setRefreshingAll(false);
    }
  }

  const filteredReviewItems =
    reviewList?.items.filter(
      (item) => reviewFilter === "all" || item.reviewReason === reviewFilter,
    ) ?? [];
  const currentModeCount =
    mode === "tracked"
      ? trackedList?.total ?? 0
      : mode === "setup"
        ? setupList?.total ?? 0
        : filteredReviewItems.length;
  const canEditSelectedSource =
    selectedItem?.watchResult?.sourceOrigin !== "built_in_special";
  const canClearSelectedSource =
    selectedItem?.watchResult?.sourceOrigin === "saved_by_user";
  const canRefreshSelectedSource = Boolean(
    selectedItem?.watchResult?.sourceUrl && selectedItem.watchResult.canRefreshNow,
  );

  return (
    <Workbench threePanel fullHeight className="updates-workbench">
      <WorkbenchRail
        ariaLabel="Updates controls"
        width={updatesFiltersCollapsed ? 0 : sidebarWidth}
        onWidthChange={(width) => {
          if (width < 196) {
            setUpdatesFiltersCollapsed(true);
            return;
          }

          setUpdatesFiltersCollapsed(false);
          setSidebarWidth(width);
        }}
        minWidth={220}
        maxWidth={360}
        resizable
        noBorder
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
                  className={`action-item ${mode === "tracked" ? "is-active" : ""}`}
                  onClick={() => setMode("tracked")}
                >
                  <Eye size={14} strokeWidth={2} className="action-item-icon" />
                  <span className="action-item-label">Tracked</span>
                  <span className="action-item-badge">{trackedList?.total ?? 0}</span>
                </button>
                <button
                  type="button"
                  className={`action-item ${mode === "setup" ? "is-active" : ""}`}
                  onClick={() => setMode("setup")}
                >
                  <ScanSearch size={14} strokeWidth={2} className="action-item-icon" />
                  <span className="action-item-label">Setup</span>
                  <span className="action-item-badge">{setupList?.total ?? 0}</span>
                </button>
                <button
                  type="button"
                  className={`action-item ${mode === "review" ? "is-active" : ""}`}
                  onClick={() => setMode("review")}
                >
                  <ListChecks size={14} strokeWidth={2} className="action-item-icon" />
                  <span className="action-item-label">Review</span>
                  <span className="action-item-badge">{reviewList?.total ?? 0}</span>
                </button>
              </div>
            </div>

            <div className="updates-rail-section">
              <div className="section-label">
                {mode === "review" ? "Review filter" : "List filter"}
              </div>
              <div className="updates-filter-list">
                {(mode === "review" ? REVIEW_FILTERS : WATCH_LIST_FILTERS).map((item) => (
                  <button
                    key={item.id}
                    type="button"
                    className={`workspace-toggle ${
                      (mode === "review" ? reviewFilter : filter) === item.id
                        ? "is-active"
                        : ""
                    }`}
                    onClick={() => {
                      if (mode === "review") {
                        setReviewFilter(item.id as ReviewFilter);
                      } else {
                        setFilter(item.id as WatchListFilter);
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
                  {refreshingAll ? "Checking..." : "Check tracked now"}
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
          <div className="table-meta">
            <div>
              <strong>{currentModeCount.toLocaleString()}</strong>
              <span>{mode === "tracked" ? "in this view" : mode}</span>
            </div>
            <div>
              <strong>{selectedItem ? "1" : "0"}</strong>
              <span>selected</span>
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

        {message ? <div className="updates-inline-message">{message}</div> : null}

        <div className="table-scroll workbench-panel updates-table-scroll">
          {mode === "tracked" ? (
            <UpdatesTrackedTable
              items={trackedList?.items ?? []}
              loading={loadingTracked}
              selectedId={selectedItem?.id ?? null}
              onSelect={handleSelectTrackedItem}
              userView={userView}
              filter={filter}
            />
          ) : null}

          {mode === "setup" ? (
            <UpdatesSetupTable
              items={setupList?.items ?? []}
              loading={loadingSetup}
              selectedId={selectedItem?.id ?? null}
              onSelect={handleSelectSetupItem}
              userView={userView}
            />
          ) : null}

          {mode === "review" ? (
            <UpdatesReviewTable
              items={filteredReviewItems}
              loading={loadingReview}
              selectedId={selectedItem?.id ?? null}
              onSelect={handleSelectReviewItem}
              userView={userView}
            />
          ) : null}
        </div>
      </WorkbenchStage>

      <WorkbenchInspector ariaLabel="Update details">
        {selectedItem ? (
          <div className="updates-inspector">
            <div className="detail-header">
              <div>
                <p className="eyebrow">{userView === "beginner" ? "Selected item" : "Inspector"}</p>
                <h2>{selectedItem.filename}</h2>
              </div>
              {selectedItem.watchResult ? (
                <span className="ghost-chip">
                  {watchStatusLabel(selectedItem.watchResult.status, userView)}
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

            {mode === "setup" || watchEditing ? (
              <div className="detail-block">
                <div className="section-label">
                  {userView === "beginner" ? "Set source" : "Source setup"}
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
                <div className="updates-action-row">
                  <button
                    type="button"
                    className="primary-action"
                    onClick={() => void handleSaveWatchSource()}
                    disabled={savingWatch || !watchSourceUrl.trim()}
                  >
                    {savingWatch ? "Saving..." : "Save source"}
                  </button>
                  <button
                    type="button"
                    className="secondary-action"
                    onClick={() => {
                      syncWatchFields(selectedItem);
                      setWatchEditing(false);
                    }}
                  >
                    Cancel
                  </button>
                </div>
              </div>
            ) : (
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
                      Edit source
                    </button>
                  ) : (
                    <div className="updates-built-in-note">
                      SimSuite is using its built-in page for this item.
                    </div>
                  )}

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

                  {canClearSelectedSource ? (
                    <button
                      type="button"
                      className="secondary-action"
                      onClick={() => void handleClearWatchSource()}
                      disabled={clearingWatch}
                    >
                      <Trash2 size={14} strokeWidth={2} />
                      {clearingWatch ? "Clearing..." : "Clear source"}
                    </button>
                  ) : null}
                </div>
              </div>
            )}
          </div>
        ) : (
          <div className="detail-empty">
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
    </Workbench>
  );
}

function UpdatesTrackedTable({
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
  filter: WatchListFilter;
}) {
  return (
    <table className="library-table updates-table">
      <thead>
        <tr>
          <th>Status</th>
          <th>File</th>
          <th>Creator</th>
          <th>Watching</th>
          <th>Installed</th>
        </tr>
      </thead>
      <tbody>
        {loading ? (
          <tr>
            <td colSpan={5} className="empty-row">
              Loading tracked items...
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
              </td>
              <td>{item.creator ?? unknownCreatorLabel(userView)}</td>
              <td>{item.subjectLabel}</td>
              <td>{formatVersion(item.installedVersion)}</td>
            </m.tr>
          ))
        ) : (
          <tr>
            <td colSpan={5} className="empty-row">
              {trackedEmptyMessage(filter, userView)}
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
  items: LibraryWatchSetupItem[];
  loading: boolean;
  selectedId: number | null;
  onSelect: (item: LibraryWatchSetupItem) => Promise<void>;
  userView: UserView;
}) {
  return (
    <table className="library-table updates-table">
      <thead>
        <tr>
          <th>File</th>
          <th>Creator</th>
          <th>Suggested source</th>
          <th>Installed</th>
          <th>Hint</th>
        </tr>
      </thead>
      <tbody>
        {loading ? (
          <tr>
            <td colSpan={5} className="empty-row">
              Loading setup items...
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
                <div className="file-path">{item.subjectLabel}</div>
              </td>
              <td>{item.creator ?? unknownCreatorLabel(userView)}</td>
              <td>
                <span className="ghost-chip">
                  {watchSourceKindLabel(item.suggestedSourceKind)}
                </span>
              </td>
              <td>{formatVersion(item.installedVersion)}</td>
              <td>{item.setupHint}</td>
            </m.tr>
          ))
        ) : (
          <tr>
            <td colSpan={5} className="empty-row">
              {userView === "beginner"
                ? "Nothing needs source setup right now."
                : "No files currently need watch setup."}
            </td>
          </tr>
        )}
      </tbody>
    </table>
  );
}

function UpdatesReviewTable({
  items,
  loading,
  selectedId,
  onSelect,
  userView,
}: {
  items: LibraryWatchReviewItem[];
  loading: boolean;
  selectedId: number | null;
  onSelect: (item: LibraryWatchReviewItem) => Promise<void>;
  userView: UserView;
}) {
  return (
    <table className="library-table updates-table">
      <thead>
        <tr>
          <th>Status</th>
          <th>File</th>
          <th>Creator</th>
          <th>Watching</th>
          <th>Review</th>
        </tr>
      </thead>
      <tbody>
        {loading ? (
          <tr>
            <td colSpan={5} className="empty-row">
              Loading review items...
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
              </td>
              <td>{item.creator ?? unknownCreatorLabel(userView)}</td>
              <td>{item.subjectLabel}</td>
              <td>
                <span className="warning-tag">
                  {reviewReasonLabel(item.reviewReason, userView)}
                </span>
              </td>
            </m.tr>
          ))
        ) : (
          <tr>
            <td colSpan={5} className="empty-row">
              {userView === "beginner"
                ? "Nothing needs review right now."
                : "No review items match this filter."}
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
