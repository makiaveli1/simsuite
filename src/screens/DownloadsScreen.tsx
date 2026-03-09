import {
  startTransition,
  useDeferredValue,
  useEffect,
  useState,
} from "react";
import { m } from "motion/react";
import {
  Ban,
  Download,
  FolderSearch,
  Inbox,
  LoaderCircle,
  RefreshCw,
  ShieldAlert,
  Workflow,
} from "lucide-react";
import { DockSectionStack } from "../components/DockSectionStack";
import { ResizableDetailPanel } from "../components/ResizableDetailPanel";
import { ResizableEdgeHandle } from "../components/ResizableEdgeHandle";
import { StatePanel } from "../components/StatePanel";
import { useUiPreferences } from "../components/UiPreferencesContext";
import { api } from "../lib/api";
import { rowHover, rowPress, stagedListItem } from "../lib/motion";
import type {
  DownloadInboxDetail,
  DownloadsInboxItem,
  DownloadsInboxResponse,
  DownloadsWatcherStatus,
  OrganizationPreview,
  RulePreset,
  Screen,
  UserView,
} from "../lib/types";

interface DownloadsScreenProps {
  refreshVersion: number;
  onNavigate: (screen: Screen) => void;
  onDataChanged: () => void;
  userView: UserView;
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
  const [watcherStatus, setWatcherStatus] = useState<DownloadsWatcherStatus | null>(
    null,
  );
  const [inbox, setInbox] = useState<DownloadsInboxResponse | null>(null);
  const [presets, setPresets] = useState<RulePreset[]>([]);
  const [selectedPreset, setSelectedPreset] = useState("Category First");
  const [selectedItemId, setSelectedItemId] = useState<number | null>(null);
  const [selectedDetail, setSelectedDetail] = useState<DownloadInboxDetail | null>(
    null,
  );
  const [selectedPreview, setSelectedPreview] = useState<OrganizationPreview | null>(
    null,
  );
  const [search, setSearch] = useState("");
  const [statusFilter, setStatusFilter] = useState("");
  const [statusMessage, setStatusMessage] = useState<string | null>(null);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [isLoadingInbox, setIsLoadingInbox] = useState(false);
  const [isRefreshing, setIsRefreshing] = useState(false);
  const [isLoadingSelection, setIsLoadingSelection] = useState(false);
  const [isApplying, setIsApplying] = useState(false);
  const [isIgnoring, setIsIgnoring] = useState(false);
  const deferredSearch = useDeferredValue(search);

  useEffect(() => {
    void api
      .listRulePresets()
      .then((items) => {
        startTransition(() => {
          setPresets(items);
          if (items.length > 0) {
            setSelectedPreset((current) =>
              items.some((item) => item.name === current) ? current : items[0].name,
            );
          }
        });
      })
      .catch((error) => setErrorMessage(toErrorMessage(error)));
  }, []);

  useEffect(() => {
    void api
      .getDownloadsWatcherStatus()
      .then(setWatcherStatus)
      .catch((error) => setErrorMessage(toErrorMessage(error)));

    const unlisten = api.listenToDownloadsStatus((status) => {
      setWatcherStatus(status);
    });

    return () => {
      void unlisten.then((dispose) => dispose());
    };
  }, []);

  useEffect(() => {
    void loadInbox();
  }, [refreshVersion, deferredSearch, statusFilter]);

  useEffect(() => {
    const items = inbox?.items ?? [];

    if (!items.length) {
      setSelectedItemId(null);
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
      setSelectedDetail(null);
      setSelectedPreview(null);
      return;
    }

    void loadSelectedItem(selectedItem);
  }, [selectedItem, selectedPreset]);

  async function loadInbox() {
    setIsLoadingInbox(true);
    setErrorMessage(null);

    try {
      const response = await api.getDownloadsInbox({
        search: deferredSearch || undefined,
        status: statusFilter || undefined,
        limit: 120,
      });
      startTransition(() => setInbox(response));
    } catch (error) {
      setErrorMessage(toErrorMessage(error));
    } finally {
      setIsLoadingInbox(false);
    }
  }

  async function loadSelectedItem(item: DownloadsInboxItem) {
    setIsLoadingSelection(true);
    setErrorMessage(null);

    try {
      const detailPromise = api.getDownloadItemDetail(item.id);
      const previewPromise =
        item.status === "ready" ||
        item.status === "needs_review" ||
        item.status === "partial"
          ? api.previewDownloadItem(item.id, selectedPreset)
          : Promise.resolve(null);

      const [detail, preview] = await Promise.all([detailPromise, previewPromise]);
      startTransition(() => {
        setSelectedDetail(detail);
        setSelectedPreview(preview);
      });
    } catch (error) {
      startTransition(() => {
        setSelectedDetail(null);
        setSelectedPreview(null);
      });
      setErrorMessage(toErrorMessage(error));
    } finally {
      setIsLoadingSelection(false);
    }
  }

  async function handleRefresh() {
    setIsRefreshing(true);
    setStatusMessage(null);
    setErrorMessage(null);

    try {
      const nextStatus = await api.refreshDownloadsInbox();
      setWatcherStatus(nextStatus);
      await loadInbox();
      setStatusMessage(
        userView === "beginner"
          ? "Downloads inbox refreshed."
          : "Downloads inbox refreshed and rechecked.",
      );
    } catch (error) {
      setErrorMessage(toErrorMessage(error));
    } finally {
      setIsRefreshing(false);
    }
  }

  async function handleApply() {
    if (!selectedItem) {
      return;
    }

    const safeCount = actionableCount(selectedPreview);
    if (safeCount === 0) {
      return;
    }

    const confirmed = globalThis.confirm(
      `Move ${safeCount} safe files from ${selectedItem.displayName}? Review-required files will stay in the inbox, and a restore point will be created first.`,
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
        `Moved ${result.movedCount} safe files from ${selectedItem.displayName}. ${result.deferredReviewCount} file(s) stayed in the inbox.`,
      );
      onDataChanged();
      await loadInbox();
      const nextStatus = await api.getDownloadsWatcherStatus();
      setWatcherStatus(nextStatus);
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
      `Hide ${selectedItem.displayName} from the inbox? This will keep it out of the active download queue until it changes again.`,
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
      await loadInbox();
      const nextStatus = await api.getDownloadsWatcherStatus();
      setWatcherStatus(nextStatus);
    } catch (error) {
      setErrorMessage(toErrorMessage(error));
    } finally {
      setIsIgnoring(false);
    }
  }

  const overview = inbox?.overview ?? null;
  const previewSuggestions = selectedPreview?.suggestions ?? [];
  const safeCount = actionableCount(selectedPreview);
  const reviewCount = selectedPreview?.reviewCount ?? selectedItem?.reviewFileCount ?? 0;
  const unchangedCount = alignedCount(selectedPreview);
  const selectedFiles = selectedDetail?.files ?? [];
  const inspectorSections = selectedItem
    ? buildInspectorSections({
        selectedItem,
        selectedFiles,
        safeCount,
        reviewCount,
        unchangedCount,
        userView,
      })
    : [];

  return (
    <section className="screen-shell">
      <div className="screen-header-row">
        <div className="screen-heading">
          <p className="eyebrow">{userView === "beginner" ? "New downloads" : "Intake"}</p>
          <div className="screen-title-row">
            <Download size={18} strokeWidth={2} />
            <h1>{userView === "beginner" ? "Downloads Inbox" : "Downloads"}</h1>
          </div>
        </div>
        <div className="header-actions">
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
            {userView === "beginner" ? "Needs attention" : "Review"}
          </button>
        </div>
      </div>

      {statusMessage ? <div className="status-banner">{statusMessage}</div> : null}
      {errorMessage ? (
        <div className="status-banner status-banner-error">{errorMessage}</div>
      ) : null}

      <div className="summary-matrix">
        <SummaryStat
          label={userView === "beginner" ? "Inbox items" : "Items"}
          value={overview?.totalItems ?? 0}
          tone="neutral"
        />
        <SummaryStat
          label={userView === "beginner" ? "Ready to sort" : "Ready"}
          value={overview?.readyItems ?? 0}
          tone="good"
        />
        <SummaryStat
          label={userView === "beginner" ? "Need a look" : "Needs review"}
          value={overview?.needsReviewItems ?? 0}
          tone="low"
        />
        <SummaryStat
          label={userView === "beginner" ? "Already moved" : "Applied"}
          value={overview?.appliedItems ?? 0}
          tone="neutral"
        />
        {userView !== "beginner" ? (
          <SummaryStat
            label="Errors"
            value={overview?.errorItems ?? 0}
            tone="low"
          />
        ) : null}
      </div>

      {!watcherStatus?.configured ? (
        <StatePanel
          eyebrow="Downloads folder"
          title={
            userView === "beginner"
              ? "Choose a Downloads folder first"
              : "Downloads watcher is not configured"
          }
          body={
            userView === "beginner"
              ? "Set one inbox folder on Home so SimSuite can stage new CC and mod archives safely before they touch your game."
              : "Point SimSuite at a downloads inbox before using intake, archive checks, or safe hand-off previews."
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
          meta={["No watcher path", "No files move from this screen automatically"]}
        />
      ) : (
        <div className="downloads-layout">
          <div className="downloads-main-column">
            <div className="panel-card downloads-control-panel">
              <div className="panel-heading">
                <div>
                  <p className="eyebrow">Watch folder</p>
                  <h2>{userView === "beginner" ? "Inbox station" : "Watcher status"}</h2>
                </div>
                <span className={`confidence-badge ${watcherTone(watcherStatus.state)}`}>
                  {friendlyWatcherLabel(watcherStatus.state)}
                </span>
              </div>

              <div className="downloads-control-grid">
                <div className="downloads-watch-card">
                  <div className="section-label">Watching</div>
                  <div className="path-card">
                    {watcherStatus.watchedPath ?? "No downloads folder set"}
                  </div>
                  <div className="downloads-watch-meta">
                    <span className="ghost-chip">
                      {watcherStatus.activeItems.toLocaleString()} active item(s)
                    </span>
                    {watcherStatus.currentItem ? (
                      <span className="ghost-chip">{watcherStatus.currentItem}</span>
                    ) : null}
                    {watcherStatus.lastRunAt ? (
                      <span className="ghost-chip">
                        Last check {formatDate(watcherStatus.lastRunAt)}
                      </span>
                    ) : null}
                  </div>
                </div>

                <div className="downloads-filter-card">
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
                      <span>{userView === "beginner" ? "Sorting style" : "Preset"}</span>
                      <select
                        value={selectedPreset}
                        onChange={(event) => setSelectedPreset(event.target.value)}
                      >
                        {presets.map((preset) => (
                          <option key={preset.name} value={preset.name}>
                            {preset.name}
                          </option>
                        ))}
                      </select>
                    </label>
                  </div>
                </div>
              </div>
            </div>

            <div className="downloads-stage">
              <div className="panel-card downloads-queue-panel">
                <div className="panel-heading">
                  <div>
                    <p className="eyebrow">Inbox queue</p>
                    <h2>
                      {userView === "beginner" ? "What just arrived" : "Download items"}
                    </h2>
                  </div>
                  <span className="ghost-chip">
                    {isLoadingInbox
                      ? "Loading..."
                      : `${inbox?.items.length ?? 0} shown`}
                  </span>
                </div>

                <div className="vertical-dock downloads-queue-dock">
                  <div className="queue-list downloads-queue-list">
                    {inbox?.items.length ? (
                      inbox.items.map((item, index) => (
                        <m.button
                          key={item.id}
                          type="button"
                          className={`downloads-item-row ${
                            selectedItemId === item.id ? "is-selected" : ""
                          } downloads-item-row-${itemStatusTone(item.status)}`}
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
                            {item.sampleFiles.length ? (
                              <div className="downloads-item-samples">
                                {item.sampleFiles.slice(0, 3).join(" · ")}
                              </div>
                            ) : null}
                          </div>
                          <div className="downloads-item-meta">
                            <span
                              className={`confidence-badge ${itemStatusTone(item.status)}`}
                            >
                              {friendlyItemStatus(item.status)}
                            </span>
                            <span className="ghost-chip">
                              {item.activeFileCount.toLocaleString()} active
                            </span>
                            {item.reviewFileCount > 0 ? (
                              <span className="warning-tag">
                                {item.reviewFileCount.toLocaleString()} review
                              </span>
                            ) : null}
                          </div>
                        </m.button>
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
                            ? "Try clearing the search, switching the filter, or refresh the inbox after a new download lands."
                            : "Clear the search, adjust status filters, or refresh the inbox to pull in newly detected downloads."
                        }
                        icon={Inbox}
                        compact
                        badge="Queue clear"
                        meta={["Filters stay local to this workspace"]}
                      />
                    )}
                  </div>

                  <ResizableEdgeHandle
                    label="Resize download queue height"
                    value={downloadsQueueHeight}
                    min={220}
                    max={720}
                    onChange={setDownloadsQueueHeight}
                    side="bottom"
                    className="dock-resize-handle downloads-queue-height-handle"
                  />
                </div>
              </div>

              <div className="panel-card downloads-preview-panel">
                <div className="panel-heading">
                  <div>
                    <p className="eyebrow">Safe hand-off</p>
                    <h2>
                      {userView === "beginner"
                        ? "What would move from this batch"
                        : "Validated preview"}
                    </h2>
                  </div>
                  {selectedItem ? (
                    <span className="ghost-chip">
                      {previewSuggestions.length
                        ? `${previewSuggestions.length.toLocaleString()} planned`
                        : `${selectedFiles.length.toLocaleString()} tracked`}
                    </span>
                  ) : null}
                </div>

                <div className="preview-list downloads-preview-list">
                  {isLoadingSelection ? (
                    <StatePanel
                      eyebrow="Preview"
                      title="Loading batch details"
                      body="SimSuite is pulling the staged files and re-checking the current safe hand-off preview."
                      icon={LoaderCircle}
                      tone="info"
                      compact
                      badge="Working"
                    />
                  ) : previewSuggestions.length ? (
                    previewSuggestions.map((item, index) => {
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
                              {item.creator ?? "Unknown"} · {friendlyKindLabel(item.kind)}
                            </span>
                          </div>
                          <div className="preview-row-route">
                            <code>{item.finalRelativePath}</code>
                          </div>
                          <div className="preview-row-meta">
                            <span
                              className={`confidence-badge ${previewStateTone(state)}`}
                            >
                              {previewStateLabel(state)}
                            </span>
                          </div>
                        </m.div>
                      );
                    })
                  ) : selectedFiles.length ? (
                    selectedFiles.map((file, index) => (
                      <m.div
                        key={file.fileId}
                        className="downloads-file-row"
                        whileHover={rowHover}
                        {...stagedListItem(index)}
                      >
                        <div className="downloads-item-main">
                          <strong>{file.filename}</strong>
                          <span>
                            {friendlyKindLabel(file.kind)}
                            {file.subtype ? ` · ${file.subtype}` : ""}
                            {file.creator ? ` · ${file.creator}` : ""}
                          </span>
                        </div>
                        <div className="downloads-item-meta">
                          {file.safetyNotes.length ? (
                            <span className="warning-tag">Needs review</span>
                          ) : (
                            <span className="confidence-badge good">Ready</span>
                          )}
                        </div>
                      </m.div>
                    ))
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
                          ? "Pick one batch from the left to see which files are ready to move and which ones still need a closer look."
                          : "Select a staged archive or file batch to populate the validated hand-off preview."
                      }
                      icon={Download}
                      compact
                      meta={["Safe files only", "Review items stay put"]}
                    />
                  )}
                </div>
              </div>
            </div>
          </div>

          <ResizableDetailPanel
            ariaLabel="Downloads inbox details"
            width={downloadsDetailWidth}
            onWidthChange={setDownloadsDetailWidth}
            minWidth={320}
            maxWidth={780}
          >
            {selectedItem ? (
              <>
                <div className="detail-header">
                  <div>
                    <p className="eyebrow">
                      {userView === "beginner" ? "Selected batch" : "Selected inbox item"}
                    </p>
                    <h2>{selectedItem.displayName}</h2>
                  </div>
                  <span
                    className={`confidence-badge ${itemStatusTone(selectedItem.status)}`}
                  >
                    {friendlyItemStatus(selectedItem.status)}
                  </span>
                </div>

                <div className="downloads-inspector-actions">
                  <button
                    type="button"
                    className="primary-action"
                    onClick={() => void handleApply()}
                    disabled={safeCount === 0 || isApplying}
                  >
                    <Workflow size={14} strokeWidth={2} />
                    {isApplying
                      ? "Applying..."
                      : userView === "beginner"
                        ? "Move safe files"
                        : "Apply safe batch"}
                  </button>
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

                <DockSectionStack
                  layoutId="downloadsInspector"
                  sections={inspectorSections}
                  intro={
                    userView === "beginner"
                      ? "Keep the batch clues you care about open and tuck the rest away while you sort new downloads."
                      : "Reorder or collapse inbox sections to fit quick intake sweeps or deeper archive checks."
                  }
                />
              </>
            ) : (
              <StatePanel
                eyebrow={userView === "beginner" ? "Downloads inbox" : "Downloads"}
                title={
                  userView === "beginner"
                    ? "Select a batch"
                    : "Select an inbox item to inspect"
                }
                body={
                  userView === "beginner"
                    ? "The right panel will show what the batch contains, what can move safely, and what should stay in the inbox."
                    : "The inspector shows hand-off counts, source notes, and the tracked file set for the selected batch."
                }
                icon={Download}
                meta={["Approval-first", "Snapshots happen before moves"]}
              />
            )}
          </ResizableDetailPanel>
        </div>
      )}
    </section>
  );
}

function buildInspectorSections({
  selectedItem,
  selectedFiles,
  safeCount,
  reviewCount,
  unchangedCount,
  userView,
}: {
  selectedItem: DownloadsInboxItem;
  selectedFiles: DownloadInboxDetail["files"];
  safeCount: number;
  reviewCount: number;
  unchangedCount: number;
  userView: UserView;
}) {
  return [
    {
      id: "summary",
      label: userView === "beginner" ? "What this batch is" : "Inbox summary",
      hint:
        userView === "beginner"
          ? "How many files are here and what kind of download this is."
          : "Source kind, file counts, and inbox status.",
      children: (
        <div className="detail-list">
          <DetailRow
            label={userView === "beginner" ? "Source" : "Source kind"}
            value={
              selectedItem.sourceKind === "archive"
                ? selectedItem.archiveFormat
                  ? `${selectedItem.archiveFormat.toUpperCase()} archive`
                  : "Archive"
                : "Direct file"
            }
          />
          <DetailRow
            label={userView === "beginner" ? "Files found" : "Detected files"}
            value={selectedItem.detectedFileCount.toLocaleString()}
          />
          <DetailRow
            label={userView === "beginner" ? "Still in inbox" : "Active files"}
            value={selectedItem.activeFileCount.toLocaleString()}
          />
          <DetailRow
            label={userView === "beginner" ? "Need a look" : "Review files"}
            value={selectedItem.reviewFileCount.toLocaleString()}
          />
          {userView !== "beginner" ? (
            <DetailRow label="Last seen" value={formatDate(selectedItem.lastSeenAt)} />
          ) : null}
        </div>
      ),
    },
    {
      id: "handoff",
      label: userView === "beginner" ? "What move does" : "Safe hand-off",
      hint:
        userView === "beginner"
          ? "Only safe files move from here. The rest stay visible."
          : "Preview counts for safe moves, review holds, and aligned files.",
      children: (
        <>
          <div className="summary-matrix">
            <SummaryStat label="Safe" value={safeCount} tone="good" />
            <SummaryStat label="Review" value={reviewCount} tone="low" />
            <SummaryStat label="Stay put" value={unchangedCount} tone="neutral" />
          </div>
          <div className="audit-what-card">
            <strong>Safe hand-off</strong>
            <span>
              Approved files move through the same validator and snapshot path as the main organizer. Anything uncertain stays here for another pass.
            </span>
          </div>
        </>
      ),
    },
    {
      id: "source",
      label: userView === "beginner" ? "Where it came from" : "Source pack",
      hint:
        userView === "beginner"
          ? "The original download path and any notes from unpacking."
          : "Original source path, archive notes, and errors.",
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
    },
    {
      id: "files",
      label: userView === "beginner" ? "Included files" : "Current file set",
      hint:
        userView === "beginner"
          ? "A quick sample of the files inside this batch."
          : "Live file set from the inbox item record.",
      badge: `${selectedFiles.length}`,
      defaultCollapsed: userView !== "power",
      children: selectedFiles.length ? (
        <div className="downloads-mini-list">
          {selectedFiles.slice(0, 8).map((file) => (
            <div key={file.fileId} className="downloads-mini-row">
              <strong>{file.filename}</strong>
              <span>
                {friendlyKindLabel(file.kind)}
                {file.subtype ? ` · ${file.subtype}` : ""}
                {file.creator ? ` · ${file.creator}` : ""}
              </span>
            </div>
          ))}
        </div>
      ) : (
        <p>No tracked files are active for this inbox item.</p>
      ),
    },
  ];
}

function DetailRow({
  label,
  value,
}: {
  label: string;
  value: string;
}) {
  return (
    <div className="detail-row">
      <span>{label}</span>
      <strong>{value}</strong>
    </div>
  );
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

function friendlyKindLabel(kind: string) {
  if (kind === "BuildBuy") {
    return "Build/Buy";
  }

  if (kind === "ScriptMods") {
    return "Script Mods";
  }

  if (kind === "OverridesAndDefaults") {
    return "Overrides";
  }

  if (kind === "PosesAndAnimation") {
    return "Poses & Animations";
  }

  if (kind === "PresetsAndSliders") {
    return "Presets & Sliders";
  }

  return kind;
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
    return "Review";
  }

  return "Aligned";
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
