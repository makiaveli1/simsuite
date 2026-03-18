import { startTransition, useEffect, useState } from "react";
import { m } from "motion/react";
import {
  FolderTree,
  RefreshCw,
  ShieldAlert,
  Workflow,
} from "lucide-react";
import { DockSectionStack } from "../components/DockSectionStack";
import { ResizableEdgeHandle } from "../components/ResizableEdgeHandle";
import { ResizableDetailPanel } from "../components/ResizableDetailPanel";
import { useUiPreferences } from "../components/UiPreferencesContext";
import { api } from "../lib/api";
import { hoverLift, rowHover, rowPress, stagedListItem, tapPress } from "../lib/motion";
import {
  friendlyTypeLabel,
  sampleCountLabel,
  sampleToggleLabel,
  reviewStateLabel,
  screenHelperLine,
  unknownCreatorLabel,
} from "../lib/uiLanguage";
import type {
  OrganizationPreview,
  PreviewSuggestion,
  RulePreset,
  Screen,
  SnapshotSummary,
  UserView,
} from "../lib/types";

interface OrganizeScreenProps {
  refreshVersion: number;
  onNavigate: (screen: Screen) => void;
  onDataChanged: () => void;
  userView: UserView;
}

type PreviewFilter = "all" | "safe" | "review" | "aligned";
type PreviewState = "safe" | "review" | "aligned";
type NoteTone = "good" | "warn" | "review" | "neutral";

interface PresetCopy {
  title: string;
  shortLabel: string;
  description: string;
}

interface NoteSummary {
  label: string;
  tone: NoteTone;
}

const PRESET_COPY: Record<string, PresetCopy> = {
  "Mirror Mode": {
    title: "Keep my current folders",
    shortLabel: "Keep current",
    description:
      "Leaves safe folders alone and only fixes placements that break the rules.",
  },
  "Category First": {
    title: "Sort by type",
    shortLabel: "By type",
    description:
      "Puts type folders first so CC is easier to browse by category.",
  },
  "Creator First": {
    title: "Sort by creator",
    shortLabel: "By creator",
    description:
      "Keeps each creator together before splitting their files by type.",
  },
  Hybrid: {
    title: "Blend type and creator",
    shortLabel: "Balanced",
    description:
      "Uses type folders first but still keeps each creator grouped underneath.",
  },
  "Minimal Safe": {
    title: "Safest cleanup",
    shortLabel: "Safest",
    description:
      "Uses a conservative layout and is the easiest first cleanup for mixed folders.",
  },
};

const FILTER_LABELS: Record<PreviewFilter, string> = {
  all: "All",
  safe: "Ready",
  review: "Needs review",
  aligned: "Already sorted",
};

const BEGINNER_PRESET_ORDER = ["Minimal Safe", "Mirror Mode", "Category First"] as const;

export function OrganizeScreen({
  refreshVersion,
  onNavigate,
  onDataChanged,
  userView,
}: OrganizeScreenProps) {
  const {
    organizeRailWidth,
    setOrganizeRailWidth,
    organizePreviewHeight,
    setOrganizePreviewHeight,
  } = useUiPreferences();
  const [presets, setPresets] = useState<RulePreset[]>([]);
  const [selectedPreset, setSelectedPreset] = useState(() =>
    userView === "beginner" ? "Minimal Safe" : "Category First",
  );
  const [preview, setPreview] = useState<OrganizationPreview | null>(null);
  const [activeFilter, setActiveFilter] = useState<PreviewFilter>("all");
  const [showAllPreviewRows, setShowAllPreviewRows] = useState(
    userView === "power",
  );
  const [selectedFileId, setSelectedFileId] = useState<number | null>(null);
  const [snapshots, setSnapshots] = useState<SnapshotSummary[]>([]);
  const [isLoadingPreview, setIsLoadingPreview] = useState(false);
  const [isLoadingSnapshots, setIsLoadingSnapshots] = useState(false);
  const [isApplying, setIsApplying] = useState(false);
  const [restoringSnapshotId, setRestoringSnapshotId] = useState<number | null>(
    null,
  );
  const [statusMessage, setStatusMessage] = useState<string | null>(null);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);

  useEffect(() => {
    void api
      .listRulePresets()
      .then((items) => {
        startTransition(() => {
          setPresets(items);
          if (items.length > 0) {
            const preferredDefault =
              userView === "beginner" ? "Minimal Safe" : "Category First";
            setSelectedPreset((current) =>
              items.some((item) => item.name === current)
                ? current
                : items.some((item) => item.name === preferredDefault)
                  ? preferredDefault
                  : items[0].name,
            );
          }
        });
      })
      .catch((error) => setErrorMessage(toErrorMessage(error)));
  }, [userView]);

  useEffect(() => {
    if (!selectedPreset) {
      return;
    }

    void refreshWorkspace(selectedPreset, showAllPreviewRows);
  }, [refreshVersion, selectedPreset, showAllPreviewRows]);

  useEffect(() => {
    const visibleSuggestions = filterSuggestions(preview?.suggestions ?? [], activeFilter);
    if (!visibleSuggestions.length) {
      setSelectedFileId(null);
      return;
    }

    if (!visibleSuggestions.some((item) => item.fileId === selectedFileId)) {
      setSelectedFileId(visibleSuggestions[0].fileId);
    }
  }, [activeFilter, preview, selectedFileId]);

  async function refreshWorkspace(presetName: string, showAll = showAllPreviewRows) {
    await Promise.all([loadPreview(presetName, showAll), loadSnapshots()]);
  }

  async function loadPreview(presetName: string, showAll = showAllPreviewRows) {
    setIsLoadingPreview(true);
    setErrorMessage(null);

    try {
      const nextPreview = await api.previewOrganization(presetName, showAll ? 0 : 60);
      startTransition(() => setPreview(nextPreview));
    } catch (error) {
      setErrorMessage(toErrorMessage(error));
    } finally {
      setIsLoadingPreview(false);
    }
  }

  async function loadSnapshots() {
    setIsLoadingSnapshots(true);

    try {
      const nextSnapshots = await api.listSnapshots(10);
      startTransition(() => setSnapshots(nextSnapshots));
    } catch (error) {
      setErrorMessage(toErrorMessage(error));
    } finally {
      setIsLoadingSnapshots(false);
    }
  }

  async function handleApply() {
    const safeCount = preview?.safeCount ?? 0;
    if (!preview || safeCount === 0) {
      return;
    }

    const confirmed = globalThis.confirm(
      userView === "beginner"
        ? `Move ${safeCount} ready files? SimSuite will create a restore point first.`
        : `Apply ${safeCount} safe move suggestions using ${selectedPreset}? A snapshot will be created first.`,
    );
    if (!confirmed) {
      return;
    }

    setIsApplying(true);
    setErrorMessage(null);

    try {
      const result = await api.applyPreviewOrganization(selectedPreset, 80, true);
      setStatusMessage(
        userView === "beginner"
          ? `Moved ${result.movedCount} ready files. Restore point ${result.snapshotName} is ready if you want to undo.`
          : `Applied ${result.movedCount} safe moves. Snapshot ${result.snapshotName} is ready.`,
      );
      onDataChanged();
    } catch (error) {
      setErrorMessage(toErrorMessage(error));
    } finally {
      setIsApplying(false);
    }
  }

  async function handleRestore(snapshot: SnapshotSummary) {
    const confirmed = globalThis.confirm(
      userView === "beginner"
        ? `Restore ${snapshot.snapshotName}? SimSuite will try to put ${snapshot.itemCount} tracked files back where they were.`
        : `Restore snapshot ${snapshot.snapshotName}? This will move ${snapshot.itemCount} tracked items back to their original paths when possible.`,
    );
    if (!confirmed) {
      return;
    }

    setRestoringSnapshotId(snapshot.id);
    setErrorMessage(null);

    try {
      const result = await api.restoreSnapshot(snapshot.id, true);
      setStatusMessage(
        userView === "beginner"
          ? `Restored ${result.restoredCount} files from ${snapshot.snapshotName}.`
          : `Restored ${result.restoredCount} items from ${snapshot.snapshotName}.`,
      );
      onDataChanged();
    } catch (error) {
      setErrorMessage(toErrorMessage(error));
    } finally {
      setRestoringSnapshotId(null);
    }
  }

  const safeCount = preview?.safeCount ?? 0;
  const unchangedCount = preview?.alignedCount ?? 0;
  const reviewCount = preview?.reviewCount ?? 0;
  const filteredSuggestions = filterSuggestions(preview?.suggestions ?? [], activeFilter);
  const filteredTotalCount = preview
    ? filteredSuggestionTotal(preview, activeFilter)
    : 0;
  const selectedPresetCopy = getPresetCopy(selectedPreset);
  const recommendedPresetCopy = getPresetCopy(preview?.recommendedPreset);
  const isRecommendedSelected =
    Boolean(preview?.recommendedPreset) && preview?.recommendedPreset === selectedPreset;
  const visiblePresets = visiblePresetOptions(presets, userView, preview?.recommendedPreset);
  const isSamplingRows =
    Boolean(preview) &&
    !showAllPreviewRows &&
    filteredSuggestions.length < filteredTotalCount;
  const selectedSuggestion =
    filteredSuggestions.find((item) => item.fileId === selectedFileId) ??
    preview?.suggestions.find((item) => item.fileId === selectedFileId) ??
    null;

  return (
    <section className="screen-shell workbench workbench-screen">
      <div className="screen-header-row">
        <div className="screen-heading">
          <p className="eyebrow">{userView === "beginner" ? "Guided cleanup" : "Workflow"}</p>
          <div className="screen-title-row">
            <Workflow size={18} strokeWidth={2} />
            <h1>{userView === "beginner" ? "Tidy Up" : "Organize"}</h1>
          </div>
          <p className="workspace-toolbar-copy">{screenHelperLine("organize", userView)}</p>
        </div>
        <div className="header-actions">
          <button
            type="button"
            className="secondary-action"
            onClick={() => void refreshWorkspace(selectedPreset)}
            disabled={isLoadingPreview || isLoadingSnapshots || isApplying}
          >
            <RefreshCw size={14} strokeWidth={2} />
            {isLoadingPreview || isLoadingSnapshots ? "Refreshing..." : "Refresh"}
          </button>
          <button
            type="button"
            className="secondary-action"
            onClick={() => onNavigate("review")}
          >
            <ShieldAlert size={14} strokeWidth={2} />
            {userView === "beginner" ? "Open files in review" : "Review"}
          </button>
        </div>
      </div>

      {statusMessage ? <div className="status-banner">{statusMessage}</div> : null}
      {errorMessage ? (
        <div className="status-banner status-banner-error">{errorMessage}</div>
      ) : null}

      <div className="organize-layout">
        <ResizableEdgeHandle
          label="Resize organize left panel"
          value={organizeRailWidth}
          min={240}
          max={480}
          onChange={setOrganizeRailWidth}
          side="right"
          className="layout-resize-handle organize-layout-handle"
        />
        <div className="organize-rail">
          <div className="panel-card">
            <div className="panel-heading">
              <div>
                <p className="eyebrow">{userView === "beginner" ? "This pass" : "Batch window"}</p>
                <h2>{userView === "beginner" ? "Ready now" : "Safe subset"}</h2>
              </div>
              <span className="ghost-chip">{selectedPresetCopy.shortLabel}</span>
            </div>

            <div className="summary-matrix">
              <SummaryStat
                label={userView === "beginner" ? "Ready now" : "Safe"}
                value={safeCount}
                tone="good"
              />
              <SummaryStat
                label={userView === "beginner" ? "Needs review" : "Review"}
                value={reviewCount}
                tone="low"
              />
              <SummaryStat
                label={userView === "beginner" ? "Already tidy" : "Aligned"}
                value={unchangedCount}
                tone="neutral"
              />
              {userView === "power" || (userView === "beginner" && (preview?.correctedCount ?? 0) > 0) ? (
                <SummaryStat
                  label={userView === "beginner" ? "Safety fixes" : "Corrected"}
                  value={preview?.correctedCount ?? 0}
                  tone="neutral"
                />
              ) : null}
            </div>

            <div className="organize-recommendation-card">
              <div className="organize-recommendation-topline">
                <span className="section-label">
                  {userView === "beginner" ? "Best fit for this library" : "Recommended preset"}
                </span>
                {isRecommendedSelected ? (
                  <span className="confidence-badge good">Using it</span>
                ) : (
                  <span className="ghost-chip">Suggested</span>
                )}
              </div>
              <strong>{recommendedPresetCopy.title}</strong>
              <p className="organize-muted">
                {preview?.recommendedReason ??
                  "SimSuite will recommend the safest tidy style after it reads the current folder shape."}
              </p>
              <div className="system-ledger">
                <LedgerRow
                  label={userView === "beginner" ? "Current folder shape" : "Structure"}
                  value={
                    preview?.detectedStructure ??
                    (isLoadingPreview ? "Refreshing..." : "Awaiting preview")
                  }
                />
              </div>
              {!isRecommendedSelected && preview?.recommendedPreset ? (
                <button
                  type="button"
                  className="secondary-action"
                  onClick={() => {
                    setStatusMessage(null);
                    setSelectedPreset(preview.recommendedPreset);
                  }}
                >
                  <FolderTree size={14} strokeWidth={2} />
                  {userView === "beginner" ? "Use this style" : "Use recommended rule set"}
                </button>
              ) : null}
            </div>

            <div className="organize-action-stack">
              <button
                type="button"
                className="primary-action"
                onClick={() => void handleApply()}
                disabled={!preview || safeCount === 0 || isApplying}
              >
                {isApplying
                  ? "Applying..."
                  : userView === "beginner"
                    ? `Move ${safeCount} ready files`
                    : `Apply ${safeCount} safe moves`}
              </button>
              <button
                type="button"
                className="secondary-action"
                onClick={() => onNavigate("review")}
                disabled={reviewCount === 0}
              >
                <ShieldAlert size={14} strokeWidth={2} />
                {userView === "beginner"
                  ? `Check ${reviewCount} files in review`
                  : `Open ${reviewCount} review items`}
              </button>
            </div>
          </div>

          <div className="panel-card">
            <div className="panel-heading">
              <div>
                <p className="eyebrow">{userView === "beginner" ? "Tidy styles" : "Presets"}</p>
                <h2>{userView === "beginner" ? "Choose how to sort" : "Sorting rules"}</h2>
              </div>
            </div>

            {userView === "beginner" ? (
              <p className="organize-muted">
                Start with the safest style first. Switch only if you want a different folder shape.
              </p>
            ) : null}

            <div className="organize-preset-grid">
              {visiblePresets.map((preset, index) => {
                const presetCopy = getPresetCopy(preset.name);
                const isRecommended = preview?.recommendedPreset === preset.name;

                return (
                  <m.button
                    key={preset.name}
                    type="button"
                    className={`organize-preset-button ${
                      selectedPreset === preset.name ? "is-selected" : ""
                    } ${isRecommended ? "is-recommended" : ""}`}
                    title={preset.description}
                    onClick={() => {
                      setStatusMessage(null);
                      setSelectedPreset(preset.name);
                    }}
                    whileHover={hoverLift}
                    whileTap={tapPress}
                    {...stagedListItem(index)}
                  >
                    <div className="organize-preset-topline">
                      <strong>{presetCopy.title}</strong>
                      <div className="organize-preset-badges">
                        {isRecommended ? (
                          <span className="confidence-badge good">Best fit</span>
                        ) : null}
                        {selectedPreset === preset.name ? (
                          <span className="ghost-chip">Current</span>
                        ) : null}
                      </div>
                    </div>
                    <span className="organize-muted">{presetCopy.description}</span>
                    {userView === "power" ? <code>{preset.template}</code> : null}
                  </m.button>
                );
              })}
            </div>
          </div>

          <div className="panel-card">
            <div className="panel-heading">
              <div>
                <p className="eyebrow">
                  {userView === "beginner" ? "What needs review" : "Issue summary"}
                </p>
                <h2>{userView === "beginner" ? "Why some files stopped" : "Checks in this pass"}</h2>
              </div>
              <span className="ghost-chip">
                {preview?.issueSummary.length ?? 0} groups
              </span>
            </div>

            {preview?.issueSummary.length ? (
              <div className="organize-issue-list">
                {preview.issueSummary.map((issue, index) => (
                  <m.button
                    key={issue.code}
                    type="button"
                    className={`organize-issue-row organize-issue-row-${issueToneClass(
                      issue.tone,
                    )}`}
                    onClick={() =>
                      setActiveFilter(issue.tone === "review" ? "review" : "all")
                    }
                    whileHover={hoverLift}
                    whileTap={tapPress}
                    {...stagedListItem(index)}
                  >
                    <div className="organize-issue-copy">
                      <strong>{issue.label}</strong>
                      <span>
                        {issue.tone === "review"
                          ? "Opens the files that still need review in the sample list."
                          : "Shows a safety correction or warning in this pass."}
                      </span>
                    </div>
                    <span className={`confidence-badge ${issueToneClass(issue.tone)}`}>
                      {issue.count}
                    </span>
                  </m.button>
                ))}
              </div>
            ) : (
              <div className="detail-empty compact-empty">
                <p className="eyebrow">Checks</p>
                <h2>{userView === "beginner" ? "No blockers in this sample" : "No grouped issues"}</h2>
              </div>
            )}
          </div>
        </div>

        <div className="organize-stage">
          <div className="organize-main-column">
            <ResizableEdgeHandle
              label="Resize preview and restore point sections"
              value={organizePreviewHeight}
              min={280}
              max={720}
              onChange={setOrganizePreviewHeight}
              side="bottom"
              className="layout-resize-handle organize-stage-handle"
            />
            <div className="panel-card organize-preview-panel">
              <div className="panel-heading organize-preview-heading">
                <div>
                  <p className="eyebrow">{userView === "beginner" ? "Step 2" : "Preview list"}</p>
                  <h2>
                    {userView === "beginner"
                      ? "Check example files"
                      : "Example files from this pass"}
                  </h2>
                </div>
                <div className="downloads-guided-card-actions organize-preview-actions">
                  <button
                    type="button"
                    className="secondary-action compact-action"
                    onClick={() => setShowAllPreviewRows((current) => !current)}
                    disabled={!preview || (filteredTotalCount || preview?.totalConsidered || 0) === 0}
                  >
                    {sampleToggleLabel(showAllPreviewRows)}
                  </button>
                  <span className="ghost-chip">
                    {sampleCountLabel(
                      filteredSuggestions.length,
                      (filteredTotalCount || preview?.totalConsidered) ?? 0,
                      !isSamplingRows,
                    )}
                  </span>
                </div>
              </div>

              <p className="organize-muted organize-preview-caption">
                {isSamplingRows
                  ? userView === "beginner"
                    ? "These are a few sample files from the full tidy pass. Open the full list if you want every file."
                    : "This list starts as a sample so you can skim the pass quickly, then open the full list when needed."
                  : userView === "beginner"
                    ? "You are seeing every file in this pass."
                    : "The full checked list is open."}
              </p>

              <div className="organize-filter-strip" role="tablist" aria-label="Preview filter">
                {(Object.keys(FILTER_LABELS) as PreviewFilter[]).map((filter) => (
                  <button
                    key={filter}
                    type="button"
                    className={`organize-filter-button ${
                      activeFilter === filter ? "is-active" : ""
                    }`}
                    onClick={() => setActiveFilter(filter)}
                  >
                    {FILTER_LABELS[filter]}
                  </button>
                ))}
              </div>

              <div className="preview-list">
                {filteredSuggestions.length ? (
                  filteredSuggestions.map((item, index) => {
                    const state = previewState(item);
                    const primaryNote = getPrimaryNoteSummary(item.validatorNotes);
                    const supportCopy = previewSupportCopy(item, userView, primaryNote?.label);

                    return (
                      <m.button
                        key={item.fileId}
                        type="button"
                        className={`preview-row organize-preview-row ${
                          selectedFileId === item.fileId ? "is-selected" : ""
                        } preview-row-state-${state}`}
                        onClick={() => setSelectedFileId(item.fileId)}
                        title={item.ruleLabel}
                        whileHover={rowHover}
                        whileTap={rowPress}
                        {...stagedListItem(index)}
                      >
                        <div className="preview-row-main">
                          <strong className="organize-file-name">{item.filename}</strong>
                          <span>{composePreviewMeta(item, userView)}</span>
                        </div>
                        <div className="preview-row-route organize-preview-route">
                          <div className="organize-route-card">
                            <div className="section-label">
                              {previewRouteLabel(item, userView)}
                            </div>
                            <div
                              className="organize-route-path"
                              title={cleanPreviewPath(item.finalRelativePath)}
                            >
                              <code>{compactPreviewPath(item.finalRelativePath, userView)}</code>
                            </div>
                          </div>
                          <div className="organize-route-details">
                            <strong className="organize-route-status">
                              {previewStateDetail(item, userView)}
                            </strong>
                            {supportCopy ? (
                              <span className="organize-row-note">{supportCopy}</span>
                            ) : null}
                          </div>
                        </div>
                        <div className="preview-row-meta">
                          {item.corrected ? (
                            <span className="ghost-chip organize-preview-fix-chip">
                              {userView === "beginner" ? "Safety fix" : "Corrected"}
                            </span>
                          ) : null}
                          <span className={`confidence-badge ${previewStateTone(state)}`}>
                            {previewStateLabel(state, userView)}
                          </span>
                        </div>
                      </m.button>
                    );
                  })
                ) : (
                  <div className="detail-empty compact-empty">
                    <p className="eyebrow">Preview</p>
                    <h2>
                      {activeFilter === "all"
                        ? "No files in this sample"
                        : `No ${FILTER_LABELS[activeFilter].toLowerCase()} files in this sample`}
                    </h2>
                  </div>
                )}
              </div>
            </div>
            <div className="panel-card organize-snapshot-panel">
              <div className="panel-heading">
                <div>
                  <p className="eyebrow">Rollback</p>
                  <h2>{userView === "beginner" ? "Restore points" : "Snapshots"}</h2>
                </div>
                <span className="ghost-chip">
                  {isLoadingSnapshots ? "Loading..." : `${snapshots.length} recent`}
                </span>
              </div>

              <div className="snapshot-list organize-snapshot-list">
                {snapshots.length ? (
                  snapshots.map((snapshot, index) => (
                    <m.div
                      key={snapshot.id}
                      className="snapshot-row"
                      title={snapshot.description ?? "Approved organization batch"}
                      whileHover={rowHover}
                      {...stagedListItem(index)}
                    >
                      <div className="snapshot-main">
                        <strong>{snapshot.snapshotName}</strong>
                        <span>{formatDate(snapshot.createdAt)}</span>
                      </div>
                      <div className="snapshot-meta">
                        <span className="ghost-chip">{snapshot.itemCount} items</span>
                        <button
                          type="button"
                          className="secondary-action"
                          onClick={() => void handleRestore(snapshot)}
                          disabled={restoringSnapshotId === snapshot.id}
                        >
                          {restoringSnapshotId === snapshot.id
                            ? "Restoring..."
                            : userView === "beginner"
                              ? "Undo"
                            : "Restore"}
                        </button>
                      </div>
                    </m.div>
                  ))
                ) : (
                  <div className="detail-empty compact-empty">
                    <p className="eyebrow">
                      {userView === "beginner" ? "Restore points" : "Snapshots"}
                    </p>
                    <h2>
                      {userView === "beginner" ? "No restore points yet" : "No rollback history"}
                    </h2>
                  </div>
                )}
              </div>
            </div>
          </div>

          <ResizableDetailPanel
            ariaLabel="Preview inspector"
            className="organize-preview-inspector"
          >
            {selectedSuggestion ? (
              <PreviewInspector
                suggestion={selectedSuggestion}
                userView={userView}
              />
            ) : (
              <div className="detail-empty">
                <p className="eyebrow">
                  {userView === "beginner" ? "Selected file" : "Inspector"}
                </p>
                <h2>{userView === "beginner" ? "Select a file" : "Select a preview item"}</h2>
              </div>
            )}
          </ResizableDetailPanel>
        </div>
      </div>
    </section>
  );
}

function PreviewInspector({
  suggestion,
  userView,
}: {
  suggestion: PreviewSuggestion;
  userView: UserView;
}) {
  const state = previewState(suggestion);
  const stateLabel = previewStateLabel(state, userView);
  const presetCopy = getPresetCopy(suggestion.ruleLabel);
  const noteSummaries = suggestion.validatorNotes.map(describeValidatorNote);
  const previewInspectorSections = [
    {
      id: "outcome",
        label: userView === "beginner" ? "What will happen" : "Outcome",
      hint:
        userView === "beginner"
          ? "Shows whether this file is ready, already tidy, or needs review."
          : "Final status, preset choice, and file basics.",
      children: (
        <>
          <div className={`organize-inspector-state organize-inspector-state-${state}`}>
            <span className={`confidence-badge ${previewStateTone(state)}`}>
              {stateLabel}
            </span>
            <strong>{previewStateHeadline(suggestion, userView)}</strong>
            <p>{previewStateDetail(suggestion, userView)}</p>
          </div>
          <div className="detail-list">
            <LedgerRow
              label={userView === "beginner" ? "Tidy style" : "Preset"}
              value={presetCopy.title}
            />
            <LedgerRow label="Type" value={friendlyTypeLabel(suggestion.kind)} />
            <LedgerRow
              label="Creator"
              value={suggestion.creator ?? unknownCreatorLabel(userView)}
            />
            {userView === "power" && suggestion.bundleName ? (
              <LedgerRow label="Bundle" value={suggestion.bundleName} />
            ) : null}
          </div>
        </>
      ),
    },
    {
      id: "paths",
      label: userView === "beginner" ? "Current and safe destination" : "Path preview",
      hint:
        userView === "beginner"
          ? "Shows where the file is now and where the validated pass would place it."
          : "Current, rule output, and validated output paths.",
      children: (
        <div className="path-grid organize-path-grid">
          <div className="detail-block">
            <div className="section-label">{userView === "beginner" ? "Now" : "Current"}</div>
            <div className="path-card">{suggestion.currentPath}</div>
          </div>
          {userView !== "beginner" ? (
            <div className="detail-block">
              <div className="section-label">Rule output</div>
              <div className="path-card">{suggestion.suggestedRelativePath}</div>
            </div>
          ) : null}
          <div className="detail-block">
            <div className="section-label">
              {userView === "beginner" ? "Safe destination" : "Validated output"}
            </div>
            <div className="path-card">{suggestion.finalRelativePath}</div>
          </div>
        </div>
      ),
    },
    {
      id: "checks",
      label:
        suggestion.reviewRequired
          ? userView === "beginner"
            ? "Why it needs review"
            : "Review reasons"
          : userView === "beginner"
            ? "Safety checks"
            : "Validator notes",
      hint:
        suggestion.reviewRequired
          ? "These checks explain what still needs a human decision."
          : "These checks show any safety corrections applied before the move.",
      badge: noteSummaries.length ? `${noteSummaries.length}` : null,
      children: noteSummaries.length ? (
        <div className="organize-note-list">
          {noteSummaries.map((note) => (
            <div
              key={`${note.label}-${note.tone}`}
              className={`organize-note organize-note-${issueToneClass(note.tone)}`}
            >
              {note.label}
            </div>
          ))}
        </div>
      ) : (
        <p className="organize-muted">
          {userView === "beginner"
            ? "No extra safety checks were needed for this file."
            : "No validator notes."}
        </p>
      ),
    },
    ...(userView === "power"
      ? [
          {
            id: "source",
            label: "Source details",
            hint: "Where the file came from and how it entered this pass.",
            children: (
              <div className="detail-list">
                <LedgerRow
                  label="Current root"
                  value={friendlySourceLocation(suggestion.sourceLocation)}
                />
                <LedgerRow label="Rule label" value={suggestion.ruleLabel} />
                <LedgerRow
                  label="Confidence"
                  value={`${Math.round(suggestion.confidence * 100)}%`}
                />
              </div>
            ),
          },
        ]
      : []),
  ];

  return (
    <>
      <div className="detail-header">
        <div>
          <p className="eyebrow">{userView === "beginner" ? "Selected file" : "Selected item"}</p>
          <h2>{suggestion.filename}</h2>
        </div>
        <span className={`confidence-badge ${previewStateTone(state)}`}>{stateLabel}</span>
      </div>

      <DockSectionStack
        layoutId="organizeInspector"
        sections={previewInspectorSections}
        intro={
          userView === "beginner"
            ? "Open the parts you need and tuck the rest away while you check the move."
            : "Reorder or collapse preview sections to suit quick triage or deep validation."
        }
      />
    </>
  );
}

function SummaryStat({
  label,
  value,
  tone,
}: {
  label: string;
  value: number;
  tone?: "good" | "low" | "neutral";
}) {
  return (
    <div className={`summary-stat ${tone ? `summary-stat-${tone}` : ""}`}>
      <span>{label}</span>
      <strong>{value.toLocaleString()}</strong>
    </div>
  );
}

function LedgerRow({
  label,
  value,
}: {
  label: string;
  value: string;
}) {
  return (
    <div className="ledger-row">
      <span>{label}</span>
      <strong>{value}</strong>
    </div>
  );
}

function formatDate(value: string) {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    return value;
  }

  return date.toLocaleString();
}

function filterSuggestions(
  suggestions: PreviewSuggestion[],
  filter: PreviewFilter,
) {
  if (filter === "all") {
    return suggestions;
  }

  return suggestions.filter((item) => previewState(item) === filter);
}

function filteredSuggestionTotal(
  preview: OrganizationPreview,
  filter: PreviewFilter,
) {
  if (filter === "all") {
    return preview.totalConsidered;
  }

  if (filter === "safe") {
    return preview.safeCount;
  }

  if (filter === "review") {
    return preview.reviewCount;
  }

  return preview.alignedCount;
}

function previewState(suggestion: PreviewSuggestion): PreviewState {
  if (suggestion.reviewRequired) {
    return "review";
  }

  if (suggestion.finalAbsolutePath === suggestion.currentPath) {
    return "aligned";
  }

  return "safe";
}

function previewStateTone(state: PreviewState) {
  if (state === "safe") {
    return "good";
  }

  if (state === "review") {
    return "low";
  }

  return "neutral";
}

function previewStateLabel(state: PreviewState, userView: UserView) {
  if (state === "safe") {
    return userView === "beginner" ? "Ready now" : "Ready";
  }

  if (state === "review") {
    return reviewStateLabel(userView);
  }

  return userView === "beginner" ? "Already tidy" : "Aligned";
}

function previewStateHeadline(suggestion: PreviewSuggestion, userView: UserView) {
  const state = previewState(suggestion);

  if (state === "safe") {
    return suggestion.corrected
      ? userView === "beginner"
        ? "Ready to move after a safety fix"
        : "Ready to move with validator corrections"
      : userView === "beginner"
        ? "Ready to move"
        : "Safe to move";
  }

  if (state === "review") {
    return userView === "beginner"
      ? "Needs review before moving"
      : "Held for review";
  }

  return userView === "beginner" ? "Already in a safe spot" : "Already aligned";
}

function previewStateDetail(suggestion: PreviewSuggestion, userView: UserView) {
  const state = previewState(suggestion);

  if (state === "review") {
    return userView === "beginner"
      ? "Needs a quick check before it can move."
      : "Held until the review notes are cleared.";
  }

  if (state === "aligned") {
    return userView === "beginner"
      ? "Already in a safe folder."
      : "No move is needed in this pass.";
  }

  return userView === "beginner"
    ? "Ready for the approved batch."
    : "Ready for the next approved batch.";
}

function composePreviewMeta(suggestion: PreviewSuggestion, userView: UserView) {
  const creator = suggestion.creator ?? unknownCreatorLabel(userView);
  const type = friendlyTypeLabel(suggestion.kind);
  return userViewNeedsBundleMeta(suggestion, userView)
    ? `${creator} · ${type} · ${suggestion.bundleName}`
    : `${creator} · ${type}`;
}

function userViewNeedsBundleMeta(suggestion: PreviewSuggestion, userView: UserView) {
  return userView !== "beginner" && Boolean(suggestion.bundleName);
}

function cleanPreviewPath(path: string) {
  return path
    .replace(/^Mods[\\/]/i, "")
    .replace(/[\\/]+/g, " > ");
}

function compactPreviewPath(path: string, userView: UserView) {
  const cleaned = cleanPreviewPath(path);
  const parts = cleaned.split(" > ").filter(Boolean);
  const maxParts = userView === "power" ? 6 : userView === "standard" ? 5 : 4;

  if (parts.length <= maxParts) {
    return cleaned;
  }

  return ["...", ...parts.slice(parts.length - maxParts)].join(" > ");
}

function previewRouteLabel(suggestion: PreviewSuggestion, userView: UserView) {
  const state = previewState(suggestion);

  if (state === "safe") {
    return userView === "beginner" ? "Safe folder" : "Validated destination";
  }

  if (state === "review") {
    return userView === "beginner" ? "Planned folder" : "Planned destination";
  }

  return userView === "beginner" ? "Current safe folder" : "Current safe destination";
}

function previewSupportCopy(
  suggestion: PreviewSuggestion,
  userView: UserView,
  primaryNote?: string,
) {
  const state = previewState(suggestion);

  if (state === "review") {
    return primaryNote ?? "Open this row to see what still needs checking.";
  }

  if (state === "aligned") {
    return userView === "beginner"
      ? "Nothing will move for this row."
      : "This row stays where it is.";
  }

  if (suggestion.corrected) {
    return userView === "beginner"
      ? "SimSuite adjusted the folder to keep the move safe."
      : "SimSuite adjusted the route to satisfy safety rules.";
  }

  if (suggestion.bundleName) {
    return `Moves together with ${suggestion.bundleName}.`;
  }

  return userView === "beginner"
    ? "Uses the current tidy style."
    : "Uses the selected rule set.";
}

function getPresetCopy(name: string | null | undefined): PresetCopy {
  if (!name) {
    return PRESET_COPY["Minimal Safe"];
  }

  return PRESET_COPY[name] ?? {
    title: name,
    shortLabel: name,
    description: "Uses the current organization preset.",
  };
}

function visiblePresetOptions(
  presets: RulePreset[],
  userView: UserView,
  recommendedPreset?: string | null,
) {
  if (userView !== "beginner") {
    return presets;
  }

  const allowed = new Set<string>(BEGINNER_PRESET_ORDER);
  if (recommendedPreset) {
    allowed.add(recommendedPreset);
  }

  const beginnerPresets = presets.filter((preset) => allowed.has(preset.name));
  beginnerPresets.sort((left, right) => {
    const leftIndex = BEGINNER_PRESET_ORDER.indexOf(
      left.name as (typeof BEGINNER_PRESET_ORDER)[number],
    );
    const rightIndex = BEGINNER_PRESET_ORDER.indexOf(
      right.name as (typeof BEGINNER_PRESET_ORDER)[number],
    );

    return (leftIndex === -1 ? 999 : leftIndex) - (rightIndex === -1 ? 999 : rightIndex);
  });

  return beginnerPresets;
}

function describeValidatorNote(note: string): NoteSummary {
  const map: Record<string, NoteSummary> = {
    low_confidence_requires_review: {
      label: "The name or type still looks uncertain.",
      tone: "review",
    },
    "Low confidence classification requires review.": {
      label: "The name or type still looks uncertain.",
      tone: "review",
    },
    unknown_kind_requires_review: {
      label: "The file type is still unknown.",
      tone: "review",
    },
    existing_path_collision_detected: {
      label: "Another file already uses that destination.",
      tone: "review",
    },
    preview_path_collision_detected: {
      label: "Two files in this pass want the same destination.",
      tone: "review",
    },
    tray_file_will_be_relocated_from_mods: {
      label: "Tray files were found inside Mods.",
      tone: "warn",
    },
    validator_routed_tray_content_to_tray_root: {
      label: "Tray files were rerouted back to Tray.",
      tone: "warn",
    },
    "Tray content rerouted to the Tray root.": {
      label: "Tray files were rerouted back to Tray.",
      tone: "warn",
    },
    validator_flattened_script_depth: {
      label: "Script mods were flattened to a safe depth.",
      tone: "warn",
    },
    "Script depth corrected to one subfolder.": {
      label: "Script mods were flattened to a safe depth.",
      tone: "warn",
    },
    validator_limited_package_depth: {
      label: "A deep folder path was shortened.",
      tone: "warn",
    },
    missing_target_root: {
      label: "A required root folder is missing from settings.",
      tone: "review",
    },
  };

  return map[note] ?? {
    label: "SimSuite raised an extra safety check for this file.",
    tone: "neutral",
  };
}

function getPrimaryNoteSummary(notes: string[]) {
  if (!notes.length) {
    return null;
  }

  return describeValidatorNote(notes[0]);
}

function issueToneClass(tone: string | NoteTone) {
  if (tone === "review") {
    return "low";
  }

  if (tone === "warn") {
    return "medium";
  }

  if (tone === "good") {
    return "good";
  }

  return "neutral";
}

function friendlySourceLocation(sourceLocation: string) {
  if (sourceLocation === "mods") {
    return "Mods";
  }

  if (sourceLocation === "tray") {
    return "Tray";
  }

  if (sourceLocation === "downloads") {
    return "Downloads";
  }

  return sourceLocation;
}

function toErrorMessage(error: unknown) {
  if (error instanceof Error) {
    return error.message;
  }

  return String(error);
}
