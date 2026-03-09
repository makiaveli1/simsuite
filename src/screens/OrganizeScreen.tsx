import { startTransition, useEffect, useState } from "react";
import { m } from "motion/react";
import { RefreshCw, ShieldAlert, Workflow } from "lucide-react";
import { DockSectionStack } from "../components/DockSectionStack";
import { ResizableEdgeHandle } from "../components/ResizableEdgeHandle";
import { ResizableDetailPanel } from "../components/ResizableDetailPanel";
import { useUiPreferences } from "../components/UiPreferencesContext";
import { api } from "../lib/api";
import { hoverLift, rowHover, rowPress, stagedListItem, tapPress } from "../lib/motion";
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
  const [selectedPreset, setSelectedPreset] = useState("Category First");
  const [preview, setPreview] = useState<OrganizationPreview | null>(null);
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
            setSelectedPreset((current) =>
              items.some((item) => item.name === current) ? current : items[0].name,
            );
          }
        });
      })
      .catch((error) => setErrorMessage(toErrorMessage(error)));
  }, []);

  useEffect(() => {
    if (!selectedPreset) {
      return;
    }

    void refreshWorkspace(selectedPreset);
  }, [refreshVersion, selectedPreset]);

  useEffect(() => {
    if (!preview?.suggestions.length) {
      setSelectedFileId(null);
      return;
    }

    if (!preview.suggestions.some((item) => item.fileId === selectedFileId)) {
      setSelectedFileId(preview.suggestions[0].fileId);
    }
  }, [preview, selectedFileId]);

  async function refreshWorkspace(presetName: string) {
    await Promise.all([loadPreview(presetName), loadSnapshots()]);
  }

  async function loadPreview(presetName: string) {
    setIsLoadingPreview(true);
    setErrorMessage(null);

    try {
      const nextPreview = await api.previewOrganization(presetName, 60);
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
    if (!preview || actionableCount(preview) === 0) {
      return;
    }

    const confirmed = globalThis.confirm(
      `Apply ${actionableCount(preview)} safe move suggestions using ${selectedPreset}? A snapshot will be created first.`,
    );
    if (!confirmed) {
      return;
    }

    setIsApplying(true);
    setErrorMessage(null);

    try {
      const result = await api.applyPreviewOrganization(selectedPreset, 80, true);
      setStatusMessage(
        `Applied ${result.movedCount} safe moves. Snapshot ${result.snapshotName} is ready.`,
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
      `Restore snapshot ${snapshot.snapshotName}? This will move ${snapshot.itemCount} tracked items back to their original paths when possible.`,
    );
    if (!confirmed) {
      return;
    }

    setRestoringSnapshotId(snapshot.id);
    setErrorMessage(null);

    try {
      const result = await api.restoreSnapshot(snapshot.id, true);
      setStatusMessage(
        `Restored ${result.restoredCount} items from ${snapshot.snapshotName}.`,
      );
      onDataChanged();
    } catch (error) {
      setErrorMessage(toErrorMessage(error));
    } finally {
      setRestoringSnapshotId(null);
    }
  }

  const safeCount = actionableCount(preview);
  const unchangedCount = alignedCount(preview);
  const reviewCount = preview?.reviewCount ?? 0;
  const selectedSuggestion =
    preview?.suggestions.find((item) => item.fileId === selectedFileId) ?? null;

  return (
    <section className="screen-shell">
      <div className="screen-header-row">
        <div className="screen-heading">
          <p className="eyebrow">{userView === "beginner" ? "Safe sorting" : "Workflow"}</p>
          <div className="screen-title-row">
            <Workflow size={18} strokeWidth={2} />
            <h1>{userView === "beginner" ? "Tidy Up" : "Organize"}</h1>
          </div>
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
            {userView === "beginner" ? "Needs attention" : "Review"}
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
                <p className="eyebrow">Batch window</p>
                <h2>{userView === "beginner" ? "Ready to tidy" : "Safe subset"}</h2>
              </div>
              <span className="ghost-chip">{selectedPreset}</span>
            </div>

            <div className="summary-matrix">
              <SummaryStat label="Safe" value={safeCount} tone="good" />
              <SummaryStat label="Review" value={reviewCount} tone="low" />
              <SummaryStat label="Aligned" value={unchangedCount} tone="neutral" />
              {userView === "power" ? (
                <SummaryStat
                  label="Corrected"
                  value={preview?.correctedCount ?? 0}
                  tone="neutral"
                />
              ) : null}
            </div>

            <div className="system-ledger">
              <LedgerRow
                label={userView === "beginner" ? "Library shape" : "Structure"}
                value={
                  preview?.detectedStructure ??
                  (isLoadingPreview ? "Refreshing..." : "Awaiting preview")
                }
              />
            </div>

            <button
              type="button"
              className="primary-action"
              onClick={() => void handleApply()}
              disabled={!preview || safeCount === 0 || isApplying}
            >
              {isApplying
                ? "Applying..."
                : userView === "beginner"
                  ? "Move safe files"
                  : "Apply safe batch"}
            </button>
          </div>

          <div className="panel-card">
            <div className="panel-heading">
              <div>
                <p className="eyebrow">Presets</p>
                <h2>{userView === "beginner" ? "Sort styles" : "Rule sets"}</h2>
              </div>
            </div>

            <div className="preset-list">
              {presets.map((preset, index) => (
                <m.button
                  key={preset.name}
                  type="button"
                  className={`preset-card ${
                    selectedPreset === preset.name ? "is-selected" : ""
                  }`}
                  title={`${preset.description} Template: ${preset.template}`}
                  onClick={() => {
                    setStatusMessage(null);
                    setSelectedPreset(preset.name);
                  }}
                  whileHover={hoverLift}
                  whileTap={tapPress}
                  {...stagedListItem(index)}
                >
                  <div className="preset-topline">
                    <strong>{preset.name}</strong>
                    <span className="ghost-chip">P{preset.priority}</span>
                  </div>
                  <code>{preset.template}</code>
                </m.button>
              ))}
            </div>
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
              <div className="panel-heading">
                <div>
                  <p className="eyebrow">Preview list</p>
                  <h2>{userView === "beginner" ? "What will move" : "Sampled items"}</h2>
                </div>
                <span className="ghost-chip">
                  {preview?.totalConsidered ?? 0} considered
                </span>
              </div>

              <div className="preview-list">
                {preview?.suggestions.length ? (
                  preview.suggestions.map((item, index) => {
                    const state =
                      item.reviewRequired
                        ? "review"
                        : item.finalAbsolutePath === item.currentPath
                          ? "aligned"
                          : "safe";

                    return (
                      <m.button
                        key={item.fileId}
                        type="button"
                        className={`preview-row ${
                          selectedFileId === item.fileId ? "is-selected" : ""
                        } preview-row-state-${state}`}
                        onClick={() => setSelectedFileId(item.fileId)}
                        title={item.ruleLabel}
                        whileHover={rowHover}
                        whileTap={rowPress}
                        {...stagedListItem(index)}
                      >
                        <div className="preview-row-main">
                          <strong>{item.filename}</strong>
                          <span>
                            {item.creator ?? "Unknown"} · {item.kind}
                            {userView === "power" && item.bundleName
                              ? ` · ${item.bundleName}`
                              : ""}
                          </span>
                        </div>
                        <div className="preview-row-route">
                          <code>{item.finalRelativePath}</code>
                        </div>
                        <div className="preview-row-meta">
                          {item.corrected ? (
                            <span className="ghost-chip">Corrected</span>
                          ) : null}
                          <span className={`confidence-badge ${previewStateTone(state)}`}>
                            {stateLabel(state)}
                          </span>
                        </div>
                      </m.button>
                    );
                  })
                ) : (
                  <div className="detail-empty compact-empty">
                    <p className="eyebrow">Preview</p>
                    <h2>No items yet</h2>
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
  const state =
    suggestion.reviewRequired
      ? "Needs review"
      : suggestion.finalAbsolutePath === suggestion.currentPath
        ? "Already aligned"
        : "Safe preview";
  const previewInspectorSections = [
    {
      id: "summary",
      label: userView === "beginner" ? "Move summary" : "Rule summary",
      hint:
        userView === "beginner"
          ? "Shows why this file is ready to move or still paused."
          : "Rule output, kind, creator, and source details.",
      children: (
        <div className="detail-list">
          <LedgerRow label="Rule" value={suggestion.ruleLabel} />
          <LedgerRow label="Kind" value={suggestion.kind} />
          <LedgerRow label="Creator" value={suggestion.creator ?? "Unknown"} />
          {userView !== "beginner" ? (
            <LedgerRow label="Root" value={suggestion.sourceLocation} />
          ) : null}
          {userView === "power" && suggestion.bundleName ? (
            <LedgerRow label="Bundle" value={suggestion.bundleName} />
          ) : null}
        </div>
      ),
    },
    {
      id: "paths",
      label: userView === "beginner" ? "Before and after" : "Path preview",
      hint:
        userView === "beginner"
          ? "Compare the current path with the validated result."
          : "Current, rule output, and validated output paths.",
      children: (
        <div className="path-grid">
          <div className="detail-block">
            <div className="section-label">Current</div>
            <div className="path-card">{suggestion.currentPath}</div>
          </div>
          {userView !== "beginner" ? (
            <div className="detail-block">
              <div className="section-label">Rule output</div>
              <div className="path-card">{suggestion.suggestedRelativePath}</div>
            </div>
          ) : null}
          <div className="detail-block">
            <div className="section-label">Validated output</div>
            <div className="path-card">{suggestion.finalRelativePath}</div>
          </div>
        </div>
      ),
    },
    ...(userView !== "beginner"
      ? [
          {
            id: "validator",
            label: "Validator notes",
            hint: "Safety corrections and reasons for review.",
            defaultCollapsed: false,
            badge: suggestion.validatorNotes.length
              ? `${suggestion.validatorNotes.length}`
              : null,
            children: suggestion.validatorNotes.length ? (
              <div className="tag-list">
                {suggestion.validatorNotes.map((note) => (
                  <span key={note} className="warning-tag">
                    {note}
                  </span>
                ))}
              </div>
            ) : (
              <p>No validator notes.</p>
            ),
          },
        ]
      : []),
  ];

  return (
    <>
      <div className="detail-header">
        <div>
          <p className="eyebrow">Selected item</p>
          <h2>{suggestion.filename}</h2>
        </div>
        <span className="confidence-badge neutral">{state}</span>
      </div>

      <DockSectionStack
        layoutId="organizeInspector"
        sections={previewInspectorSections}
        intro={
          userView === "beginner"
            ? "Keep the move summary you care about open and tuck the rest away."
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

function formatDate(value: string) {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    return value;
  }

  return date.toLocaleString();
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

function stateLabel(state: "safe" | "review" | "aligned") {
  if (state === "safe") {
    return "Safe";
  }

  if (state === "review") {
    return "Review";
  }

  return "Aligned";
}

function toErrorMessage(error: unknown) {
  if (error instanceof Error) {
    return error.message;
  }

  return String(error);
}
