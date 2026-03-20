import { useEffect, useState } from "react";
import { m } from "motion/react";
import {
  AlertTriangle,
  CheckCircle2,
  ExternalLink,
  HelpCircle,
  LibraryBig,
  RefreshCw,
  Settings2,
  Trash2,
} from "lucide-react";
import { Workbench } from "../components/layout/Workbench";
import { WorkbenchInspector } from "../components/layout/WorkbenchInspector";
import { WorkbenchStage } from "../components/layout/WorkbenchStage";
import { api } from "../lib/api";
import {
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
const SETUP_FETCH_LIMIT = 200;

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

function setupSuggestedSourceLabel(item: LibraryWatchSetupItem) {
  return item.suggestedSourceKind === "creator_page"
    ? item.creator ?? item.subjectLabel
    : item.subjectLabel;
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
    }
  }, [initialMode]);

  useEffect(() => {
    if (initialFilter) {
      setFilter(initialFilter);
    }
  }, [initialFilter]);

  useEffect(() => {
    void Promise.all([
      loadTrackedList(mode !== "tracked", filter),
      loadSetupList(mode !== "setup"),
      loadReviewList(mode !== "review"),
    ]);
  }, [refreshVersion]);

  useEffect(() => {
    if (mode !== "tracked") {
      return;
    }

    void loadTrackedList();
  }, [filter, mode]);

  useEffect(() => {
    if (!initialFileId) {
      return;
    }

    void loadSelectedFile(initialFileId);
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
      const result = await api.listLibraryWatchSetupItems(SETUP_FETCH_LIMIT);
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
      sourceKind: item.suggestedSourceKind,
      sourceLabel: setupSuggestedSourceLabel(item),
      sourceUrl: "",
    });
  }

  async function handleSelectReviewItem(item: LibraryWatchReviewItem) {
    await loadSelectedFile(item.fileId);
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
  const trackedTotal = trackedList?.total ?? 0;
  const setupTotal = setupList?.total ?? 0;
  const reviewTotal = reviewList?.total ?? 0;
  const visibleRows =
    mode === "tracked"
      ? trackedList?.items ?? []
      : mode === "setup"
        ? setupList?.items ?? []
        : filteredReviewItems;
  const selectedWatchResult = selectedItem?.watchResult ?? null;
  const hasSavedWatchSource = Boolean(selectedWatchResult?.sourceKind);
  const selectedSetupItem =
    selectedItem && setupList
      ? setupList.items.find((item) => item.fileId === selectedItem.id) ?? null
      : null;
  const setupSuggestionKind = selectedSetupItem?.suggestedSourceKind ?? watchSourceKind;
  const setupSuggestionLabel = selectedSetupItem
    ? setupSuggestedSourceLabel(selectedSetupItem)
    : selectedItem?.creator ??
      selectedItem?.installedVersionSummary?.subjectLabel ??
      "Use the clearest file clue";
  const setupSuggestionHint =
    selectedSetupItem?.setupHint ??
    (userView === "beginner"
      ? "Save one page for this file so SimSuite can check it later."
      : "Save one source page for this file so it can move into the tracked lane.");
  const canEditSelectedSource =
    selectedWatchResult?.sourceOrigin !== "built_in_special";
  const canClearSelectedSource = selectedWatchResult?.sourceOrigin === "saved_by_user";
  const canRefreshSelectedSource = Boolean(
    hasSavedWatchSource && selectedWatchResult?.sourceUrl && selectedWatchResult.canRefreshNow,
  );
  const currentFilterLabel =
    mode === "review"
      ? REVIEW_FILTERS.find((item) => item.id === reviewFilter)?.label ?? "Review"
      : WATCH_LIST_FILTERS.find((item) => item.id === filter)?.label ?? "Tracked";
  const modeCopy: Record<
    UpdateMode,
    { title: string; body: string }
  > = {
    tracked: {
      title: "Tracked pages",
      body:
        "Confirmed updates, possible changes, and unclear results stay in one calm list so the next follow-up is easy to spot.",
    },
    setup: {
      title: "Source setup",
      body:
        "Pick the right page once, save it, and let SimSuite reuse that source on later checks.",
    },
    review: {
      title: "Needs review",
      body:
        "These pages still need provider setup, are reminder-only, or came back too unclear to trust yet.",
    },
  };
  const modeCounts = {
    tracked: trackedTotal,
    setup: setupTotal,
    review: reviewTotal,
  };
  const listCountLabel =
    mode === "setup" && setupList?.truncated
      ? `${setupList.items.length.toLocaleString()} shown of ${setupTotal.toLocaleString()}`
      : `${visibleRows.length.toLocaleString()} in view`;

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
      setSelectedItem(null);
      setWatchEditing(false);
      return;
    }

    if (selectedItem && visibleRows.some((item) => item.fileId === selectedItem.id)) {
      return;
    }

    if (mode === "tracked") {
      void handleSelectTrackedItem(visibleRows[0] as LibraryWatchListItem);
      return;
    }

    if (mode === "setup") {
      void handleSelectSetupItem(visibleRows[0] as LibraryWatchSetupItem);
      return;
    }

    void handleSelectReviewItem(visibleRows[0] as LibraryWatchReviewItem);
  }, [mode, selectedItem, visibleRows]);

  return (
    <Workbench fullHeight className="updates-workbench">
      <WorkbenchStage className="updates-stage">
        <UpdatesTopStrip
          userView={userView}
          mode={mode}
          filter={filter}
          reviewFilter={reviewFilter}
          modeCounts={modeCounts}
          refreshingAll={refreshingAll}
          onModeChange={setMode}
          onTrackedFilterChange={setFilter}
          onReviewFilterChange={setReviewFilter}
          onRefreshAll={() => void handleRefreshAll()}
          onOpenLibrary={() => onNavigate("library")}
        />

        {message ? <div className="updates-inline-message">{message}</div> : null}

        <section className="workbench-panel updates-list-panel">
          <div className="updates-list-header">
            <div className="updates-list-copy">
              <p className="eyebrow">Current lane</p>
              <h2>{modeCopy[mode].title}</h2>
              <p className="text-muted">{modeCopy[mode].body}</p>
            </div>

            <div className="updates-list-meta">
              <span className="ghost-chip">{listCountLabel}</span>
              <span className="ghost-chip">{currentFilterLabel}</span>
            </div>
          </div>

          {selectedItem ? (
            <div className="updates-selection-strip">
              <div className="updates-selection-copy">
                <span className="section-label">
                  {mode === "setup"
                    ? userView === "beginner"
                      ? "Next source"
                      : "Setup focus"
                    : userView === "beginner"
                      ? "Selected item"
                      : "Selection focus"}
                </span>
                <strong>{selectedItem.filename}</strong>
                <p className="text-muted">
                  {describeUpdatesStageFocus(selectedItem, mode, userView)}
                </p>
              </div>

              <div className="tag-list updates-selection-tags">
                <span className="ghost-chip">
                  {hasSavedWatchSource
                    ? watchStatusLabel(selectedWatchResult!.status, userView)
                    : "Needs source"}
                </span>
                <span className="ghost-chip">
                  {mode === "setup"
                    ? watchSourceKindLabel(watchSourceKind)
                    : hasSavedWatchSource
                      ? selectedWatchResult?.sourceLabel ??
                        watchSourceKindLabel(selectedWatchResult?.sourceKind ?? null)
                      : "No saved source"}
                </span>
                <span className="ghost-chip">
                  Installed {formatVersion(selectedItem.installedVersionSummary?.version)}
                </span>
              </div>
            </div>
          ) : null}

          <div
            className="table-scroll updates-table-scroll"
            role="region"
            aria-label={
              mode === "tracked"
                ? "Tracked updates list"
                : mode === "setup"
                  ? "Update source setup list"
                  : "Update review list"
            }
          >
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
        </section>
      </WorkbenchStage>

      <WorkbenchInspector ariaLabel="Update details" className="updates-inspector-shell">
        {selectedItem ? (
          <div className="updates-inspector">
            <div className="detail-header updates-detail-header">
              <div className="updates-detail-heading">
                <p className="eyebrow">{userView === "beginner" ? "Selected item" : "Inspector"}</p>
                <div className="updates-detail-title-row">
                  <h2>{selectedItem.filename}</h2>
                  {hasSavedWatchSource ? (
                    <span className="ghost-chip">
                      {watchStatusLabel(selectedWatchResult!.status, userView)}
                    </span>
                  ) : (
                    <span className="ghost-chip">Needs source</span>
                  )}
                </div>
                <p className="updates-detail-subline">
                  {(selectedItem.creator ?? unknownCreatorLabel(userView))} ·{" "}
                  {friendlyTypeLabel(selectedItem.kind)}
                </p>
              </div>
            </div>

            <div className="detail-block">
              <div className="section-label">Snapshot</div>
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
                {hasSavedWatchSource ? (
                  <DetailRow
                    label="Latest helper version"
                    value={formatVersion(selectedWatchResult!.latestVersion)}
                  />
                ) : null}
              </div>
            </div>

            <div className="detail-block">
              <div className="section-label">
                {hasSavedWatchSource ? "Watch state" : "Suggested source"}
              </div>
              {hasSavedWatchSource ? (
                <div className="updates-status-card">
                  <div className="updates-status-row">
                    {watchStatusIcon(selectedWatchResult!.status)}
                    <div>
                      <strong>
                        {watchStatusLabel(selectedWatchResult!.status, userView)}
                      </strong>
                      <p className="text-muted">
                        {watchCapabilityLabel(selectedWatchResult!, userView)}
                      </p>
                    </div>
                  </div>
                  <div className="detail-list">
                    <DetailRow
                      label="Source type"
                      value={watchSourceKindLabel(selectedWatchResult!.sourceKind)}
                    />
                    <DetailRow
                      label="Source origin"
                      value={watchSourceOriginLabel(selectedWatchResult!.sourceOrigin)}
                    />
                    <DetailRow
                      label="Last checked"
                      value={formatCheckedAt(selectedWatchResult!.checkedAt)}
                    />
                  </div>
                  {selectedWatchResult!.sourceLabel ? (
                    <p className="text-muted">
                      Watching <strong>{selectedWatchResult!.sourceLabel}</strong>
                    </p>
                  ) : null}
                  {selectedWatchResult!.note ? (
                    <p className="text-muted">{selectedWatchResult!.note}</p>
                  ) : null}
                  {selectedWatchResult!.evidence.length ? (
                    <div className="tag-list">
                      {selectedWatchResult!.evidence.map((entry) => (
                        <span key={entry} className="ghost-chip">
                          {entry}
                        </span>
                      ))}
                    </div>
                  ) : null}
                  {selectedWatchResult!.sourceUrl ? (
                    <a
                      href={selectedWatchResult!.sourceUrl}
                      target="_blank"
                      rel="noreferrer noopener"
                      className="updates-source-link"
                    >
                      <ExternalLink size={12} strokeWidth={2} />
                      Open source page
                    </a>
                  ) : null}
                </div>
              ) : (
                <div className="updates-status-card updates-status-card-setup">
                  <div className="updates-status-row">
                    <HelpCircle className="updates-status-icon updates-status-icon-muted" />
                    <div>
                      <strong>{watchSourceKindLabel(setupSuggestionKind)}</strong>
                      <p className="text-muted">
                        Best first guess: <strong>{setupSuggestionLabel}</strong>
                      </p>
                    </div>
                  </div>
                  <div className="detail-list">
                    <DetailRow
                      label="Suggested type"
                      value={watchSourceKindLabel(setupSuggestionKind)}
                    />
                    <DetailRow label="Best match" value={setupSuggestionLabel} />
                  </div>
                  <p className="text-muted">{setupSuggestionHint}</p>
                </div>
              )}
            </div>

            {watchEditing ? (
              <div className="detail-block">
                <div className="section-label">
                  {hasSavedWatchSource ? "Edit source" : "Save source"}
                </div>
                <div className="updates-inline-editor">
                  <p className="updates-editor-copy">
                    {!hasSavedWatchSource
                      ? "Save one trusted page here and this file can move into tracked."
                      : "Change the saved page here without leaving the current queue."}
                  </p>

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
                    <div className="updates-inline-editor-danger">
                      <strong>Remove the saved page</strong>
                      <p className="text-muted">
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

                  <div className="updates-action-row updates-editor-actions">
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
                </div>
              </div>
            ) : (
              <div className="detail-block">
                <div className="section-label">Next step</div>
                <div className="updates-action-grid">
                  {canEditSelectedSource ? (
                    <button
                      type="button"
                      className="secondary-action"
                      onClick={() => setWatchEditing(true)}
                    >
                      <Settings2 size={14} strokeWidth={2} />
                      {hasSavedWatchSource &&
                      (selectedWatchResult?.sourceUrl || selectedWatchResult?.sourceLabel)
                        ? "Edit source"
                        : "Set source"}
                    </button>
                  ) : (
                    <div className="updates-built-in-note">
                      SimSuite is using its built-in page for this item.
                    </div>
                  )}

                  {hasSavedWatchSource ? (
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
                  {!hasSavedWatchSource
                    ? "Set the source here when you are ready to save the page. Once it is saved, this file moves into tracked."
                    : mode === "setup"
                      ? "Keep the queue in view, then edit the saved page here only when you need to change it."
                      : "The right side keeps the proof close, and the source can be changed here whenever needed."}
                </p>
              </div>
            )}
          </div>
        ) : (
          <div className="detail-empty updates-empty-state">
            <p className="eyebrow">{userView === "beginner" ? "Selected item" : "Inspector"}</p>
            <h2>Select an item</h2>
            <p>
              {userView === "beginner"
                ? "Pick something from the list to check its update story or save a watch page."
                : "Select a row to inspect its current source, proof, and next step."}
            </p>
          </div>
        )}
      </WorkbenchInspector>

    </Workbench>
  );
}

function UpdatesTopStrip({
  userView,
  mode,
  filter,
  reviewFilter,
  modeCounts,
  refreshingAll,
  onModeChange,
  onTrackedFilterChange,
  onReviewFilterChange,
  onRefreshAll,
  onOpenLibrary,
}: {
  userView: UserView;
  mode: UpdateMode;
  filter: WatchListFilter;
  reviewFilter: ReviewFilter;
  modeCounts: Record<UpdateMode, number>;
  refreshingAll: boolean;
  onModeChange: (mode: UpdateMode) => void;
  onTrackedFilterChange: (filter: WatchListFilter) => void;
  onReviewFilterChange: (filter: ReviewFilter) => void;
  onRefreshAll: () => void;
  onOpenLibrary: () => void;
}) {
  const modeOptions: Array<{ id: UpdateMode; label: string }> = [
    { id: "tracked", label: "Tracked" },
    { id: "setup", label: "Need source" },
    { id: "review", label: "Needs review" },
  ];
  const filterOptions = mode === "review" ? REVIEW_FILTERS : WATCH_LIST_FILTERS;
  const activeFilter = mode === "review" ? reviewFilter : filter;

  return (
    <div className="updates-top-strip">
      <div className="updates-top-strip-summary" aria-label="Updates summary">
        {modeOptions.map((option) => (
          <span key={option.id} className="updates-top-strip-chip">
            <strong>{modeCounts[option.id].toLocaleString()}</strong>
            <span>{option.label}</span>
          </span>
        ))}
      </div>

      <div className="updates-top-strip-main">
        <div className="updates-top-strip-section">
          <span className="section-label">View</span>
          <div className="updates-top-strip-toggle-row" role="tablist" aria-label="Update views">
            {modeOptions.map((option) => (
              <button
                key={option.id}
                type="button"
                role="tab"
                aria-selected={mode === option.id}
                aria-label={`${option.label} (${modeCounts[option.id]})`}
                className={`workspace-toggle ${mode === option.id ? "is-active" : ""}`}
                onClick={() => onModeChange(option.id)}
              >
                <span>{option.label}</span>
                <span className="updates-top-strip-count">
                  {modeCounts[option.id].toLocaleString()}
                </span>
              </button>
            ))}
          </div>
        </div>

        <div className="updates-top-strip-section">
          <span className="section-label">
            {mode === "review" ? "Review filter" : "List filter"}
          </span>
          <div className="updates-top-strip-toggle-row">
            {filterOptions.map((item) => (
              <button
                key={item.id}
                type="button"
                className={`workspace-toggle ${activeFilter === item.id ? "is-active" : ""}`}
                onClick={() => {
                  if (mode === "review") {
                    onReviewFilterChange(item.id as ReviewFilter);
                    return;
                  }

                  onTrackedFilterChange(item.id as WatchListFilter);
                }}
              >
                {userView === "beginner" ? item.beginnerLabel : item.label}
              </button>
            ))}
          </div>
        </div>

        <div className="updates-top-strip-actions">
          <button
            type="button"
            className="primary-action"
            onClick={onRefreshAll}
            disabled={refreshingAll}
          >
            <RefreshCw
              size={14}
              strokeWidth={2}
              className={refreshingAll ? "spin" : undefined}
            />
            {refreshingAll ? "Checking..." : "Check tracked now"}
          </button>
          <button type="button" className="secondary-action" onClick={onOpenLibrary}>
            <LibraryBig size={14} strokeWidth={2} />
            Browse library
          </button>
          <p className="updates-top-strip-note">{screenHelperLine("updates", userView)}</p>
        </div>
      </div>
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
      : "Save one exact or creator page here so this file can move from setup into the tracked lane.";
  }

  if (mode === "review") {
    return item.watchResult?.note?.trim()
      ? item.watchResult.note
      : userView === "beginner"
        ? "This saved source still needs a calmer manual follow-up before you can trust the result."
        : "This saved source is still in a cautious state, so it stays in the review lane.";
  }

  return item.watchResult?.note?.trim()
    ? item.watchResult.note
    : userView === "beginner"
      ? "This tracked file has the clearest next update story in the current view."
      : "This tracked file is the clearest current follow-up in the selected watch lane.";
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
    <table className="library-table updates-table updates-setup-table">
      <thead>
        <tr>
          <th>File</th>
          <th>Suggested source</th>
          <th>Installed</th>
        </tr>
      </thead>
      <tbody>
        {loading ? (
          <tr>
            <td colSpan={3} className="empty-row">
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
                <div className="updates-table-meta">
                  {item.creator ?? unknownCreatorLabel(userView)}
                </div>
              </td>
              <td>
                <div className="file-title">{watchSourceKindLabel(item.suggestedSourceKind)}</div>
                <div className="updates-table-meta">{item.subjectLabel}</div>
              </td>
              <td>
                <div className="file-title">{formatVersion(item.installedVersion)}</div>
                <div className="updates-table-meta">No saved source yet</div>
              </td>
            </m.tr>
          ))
        ) : (
          <tr>
            <td colSpan={3} className="empty-row">
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
          <th>Watching</th>
          <th>Review</th>
        </tr>
      </thead>
      <tbody>
        {loading ? (
          <tr>
            <td colSpan={4} className="empty-row">
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
                <span className="warning-tag">
                  {reviewReasonLabel(item.reviewReason, userView)}
                </span>
                <div className="updates-table-meta">{item.reviewHint}</div>
              </td>
            </m.tr>
          ))
        ) : (
          <tr>
            <td colSpan={4} className="empty-row">
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
