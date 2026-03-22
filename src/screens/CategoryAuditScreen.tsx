import { startTransition, useDeferredValue, useEffect, useState } from "react";
import { m } from "motion/react";
import { RefreshCw, SearchX, Shapes, ShieldAlert, Sparkles } from "lucide-react";
import { DockSectionStack } from "../components/DockSectionStack";
import { ResizableEdgeHandle } from "../components/ResizableEdgeHandle";
import { ResizableDetailPanel } from "../components/ResizableDetailPanel";
import { StatePanel } from "../components/StatePanel";
import { useUiPreferences } from "../components/UiPreferencesContext";
import { api } from "../lib/api";
import { toast } from "../components/Toast";
import { hoverLift, rowHover, rowPress, stagedListItem } from "../lib/motion";
import { friendlyTypeLabel, reviewLabel, screenHelperLine } from "../lib/uiLanguage";
import type {
  CategoryAuditFile,
  CategoryAuditResponse,
  LibraryFacets,
  Screen,
  UserView,
} from "../lib/types";

interface CategoryAuditScreenProps {
  refreshVersion: number;
  onNavigate: (screen: Screen) => void;
  onDataChanged: () => void;
  userView: UserView;
}

export function CategoryAuditScreen({
  refreshVersion,
  onNavigate,
  onDataChanged,
  userView,
}: CategoryAuditScreenProps) {
  const { auditGroupWidth, setAuditGroupWidth, auditStageHeight, setAuditStageHeight } =
    useUiPreferences();
  const [audit, setAudit] = useState<CategoryAuditResponse | null>(null);
  const [facets, setFacets] = useState<LibraryFacets | null>(null);
  const [selectedGroupId, setSelectedGroupId] = useState<string | null>(null);
  const [search, setSearch] = useState("");
  const [minGroupSize, setMinGroupSize] = useState("2");
  const [kindDraft, setKindDraft] = useState("");
  const [subtypeDraft, setSubtypeDraft] = useState("");
  const [statusMessage, setStatusMessage] = useState<string | null>(null);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [isApplying, setIsApplying] = useState(false);
  const [expandedGroups, setExpandedGroups] = useState<Record<string, boolean>>({});
  const [groupFileCache, setGroupFileCache] = useState<
    Record<string, CategoryAuditFile[]>
  >({});
  const [loadingGroupId, setLoadingGroupId] = useState<string | null>(null);
  const deferredSearch = useDeferredValue(search);

  useEffect(() => {
    void api.getLibraryFacets().then(setFacets);
  }, [refreshVersion]);

  useEffect(() => {
    void loadAudit();
  }, [refreshVersion, deferredSearch, minGroupSize]);

  useEffect(() => {
    if (!audit?.groups.length) {
      setSelectedGroupId(null);
      return;
    }

    if (!audit.groups.some((group) => group.id === selectedGroupId)) {
      setSelectedGroupId(audit.groups[0].id);
    }
  }, [audit, selectedGroupId]);

  const selectedGroup =
    audit?.groups.find((group) => group.id === selectedGroupId) ?? null;
  const isShowingAllFiles = selectedGroup
    ? Boolean(expandedGroups[selectedGroup.id])
    : false;
  const fullGroupFiles = selectedGroup ? groupFileCache[selectedGroup.id] ?? null : null;
  const visibleGroupFiles = selectedGroup
    ? isShowingAllFiles && fullGroupFiles?.length
      ? fullGroupFiles
      : selectedGroup.sampleFiles
    : [];
  const taxonomyKinds = facets?.taxonomyKinds ?? [
    "CAS",
    "BuildBuy",
    "Gameplay",
    "ScriptMods",
    "OverridesAndDefaults",
    "PosesAndAnimation",
    "PresetsAndSliders",
    "Unknown",
  ];
  const categoryAuditInspectorSections = selectedGroup
    ? [
        {
          id: "what-save-does",
          label: userView === "beginner" ? "What save does" : "Save for this group",
          hint:
            userView === "beginner"
              ? "This saves one type for the whole group."
              : "Saves one type across the selected group.",
          children: (
            <div className="audit-what-card">
              <strong>What save does</strong>
              <span>
                SimSuite will remember this type for this whole group and reuse it on later scans.
              </span>
            </div>
          ),
        },
        {
          id: "cluster-facts",
          label: userView === "beginner" ? "Group summary" : "Group details",
          hint:
            userView === "beginner"
              ? "How many files are in this group and how strong the match is."
              : "Confidence, keyword clues, and group identity.",
          children: (
            <div className="detail-list">
              <DetailRow
                label="Confidence"
                value={confidenceLabel(selectedGroup.confidence)}
              />
              <DetailRow
                label={userView === "beginner" ? "Shared clues" : "Keyword cues"}
                value={selectedGroup.keywordSamples.slice(0, 3).join(", ") || "None"}
              />
              {userView === "power" ? (
                <DetailRow label="Group id" value={selectedGroup.id} mono />
              ) : null}
            </div>
          ),
        },
        ...(userView !== "beginner" && selectedGroup.sourceSignals.length
          ? [
              {
                id: "signals",
                label: "Why these files were grouped",
                hint: "Shared filename, folder, and inspection signals.",
                defaultCollapsed: userView !== "power",
                children: (
                  <div className="tag-list">
                    {selectedGroup.sourceSignals.map((signal) => (
                      <span key={signal} className="ghost-chip">
                        {signal}
                      </span>
                    ))}
                  </div>
                ),
              },
            ]
          : []),
        ...(selectedGroup.keywordSamples.length
          ? [
              {
                id: "keywords",
                label:
                  userView === "beginner" ? "Seen in file names" : "Filename clues",
                hint:
                  userView === "beginner"
                    ? "Shared words that helped SimSuite guess the type."
                    : "Common filename keywords inside this group.",
                children: (
                  <div className="creator-suggestion-strip">
                    {selectedGroup.keywordSamples.map((sample) => (
                      <span key={sample} className="creator-suggestion is-active">
                        {sample}
                      </span>
                    ))}
                  </div>
                ),
              },
            ]
          : []),
        {
          id: "teach",
          label: "Save type",
          hint:
            userView === "beginner"
              ? "Save the right type once for the whole batch."
              : "Set the type and optional subtype for this group.",
          children: (
            <>
              <div className="creator-learning-grid">
                <label className="field">
                  <span>Type</span>
                  <select
                    value={kindDraft}
                    onChange={(event) => setKindDraft(event.target.value)}
                  >
                    <option value="">
                      {userView === "beginner" ? "Choose a type" : "Select type"}
                    </option>
                    {taxonomyKinds.map((kind) => (
                      <option key={kind} value={kind}>
                        {friendlyTypeLabel(kind)}
                      </option>
                    ))}
                  </select>
                </label>

                <label className="field">
                  <span>Subtype</span>
                  <input
                    value={subtypeDraft}
                    onChange={(event) => setSubtypeDraft(event.target.value)}
                    placeholder={
                      userView === "beginner" ? "Optional smaller type" : "Optional subtype"
                    }
                  />
                </label>
              </div>

              <div className="creator-learning-actions">
                <button
                  type="button"
                  className="primary-action"
                  disabled={!kindDraft.trim() || isApplying}
                  onClick={() => void handleApplyGroup()}
                >
                  {isApplying
                    ? "Applying..."
                    : userView === "beginner"
                      ? `Save this type for ${selectedGroup.itemCount.toLocaleString()} files`
                      : `Apply to ${selectedGroup.itemCount.toLocaleString()} files`}
                </button>
              </div>
            </>
          ),
        },
      ]
    : [];

  useEffect(() => {
    if (!selectedGroup) {
      setKindDraft("");
      setSubtypeDraft("");
      return;
    }

    setKindDraft(selectedGroup.suggestedKind);
    setSubtypeDraft(selectedGroup.suggestedSubtype ?? "");
  }, [selectedGroup]);

  async function loadAudit() {
    setIsLoading(true);
    setErrorMessage(null);

    try {
      const response = await api.getCategoryAudit({
        search: deferredSearch || undefined,
        limit: 64,
        minGroupSize: Number(minGroupSize) || 2,
      });
      startTransition(() => setAudit(response));
    } catch (error) {
      setErrorMessage(toErrorMessage(error));
    } finally {
      setIsLoading(false);
    }
  }

  async function handleApplyGroup() {
    if (!selectedGroup || !kindDraft.trim()) {
      return;
    }

    const confirmed = globalThis.confirm(
      `Apply ${kindDraft.trim()}${subtypeDraft.trim() ? ` / ${subtypeDraft.trim()}` : ""} to ${selectedGroup.itemCount} files in this type group?`,
    );
    if (!confirmed) {
      return;
    }

    setIsApplying(true);
    setErrorMessage(null);
    setStatusMessage(null);

    try {
      const result = await api.applyCategoryAudit(
        selectedGroup.fileIds,
        kindDraft.trim(),
        subtypeDraft.trim() || undefined,
      );
      setStatusMessage(
        `Saved ${friendlyTypeLabel(result.kind)}${result.subtype ? ` / ${result.subtype}` : ""} for ${result.updatedCount} files and cleared ${result.clearedReviewCount} review items.`,
      );
      toast("success", `Category updated for ${result.updatedCount} files`);
      onDataChanged();
      await loadAudit();
    } catch (error) {
      setErrorMessage(toErrorMessage(error));
      toast("error", "Failed to apply category audit");
    } finally {
      setIsApplying(false);
    }
  }

  async function handleToggleGroupFiles() {
    if (!selectedGroup) {
      return;
    }

    if (isShowingAllFiles) {
      setExpandedGroups((current) => ({ ...current, [selectedGroup.id]: false }));
      return;
    }

    if (!fullGroupFiles) {
      setLoadingGroupId(selectedGroup.id);
      setErrorMessage(null);

      try {
        const files = await api.getCategoryAuditGroupFiles(selectedGroup.id);
        setGroupFileCache((current) => ({ ...current, [selectedGroup.id]: files }));
      } catch (error) {
        setErrorMessage(toErrorMessage(error));
        return;
      } finally {
        setLoadingGroupId((current) =>
          current === selectedGroup.id ? null : current,
        );
      }
    }

    setExpandedGroups((current) => ({ ...current, [selectedGroup.id]: true }));
  }

  return (
    <section className="screen-shell workbench workbench-screen types-screen">
      <div className="screen-header-row">
        <div className="screen-heading">
          <p className="eyebrow">{userView === "beginner" ? "Type cleanup" : "Learning"}</p>
          <div className="screen-title-row">
            <Shapes size={18} strokeWidth={2} />
            <h1>{userView === "beginner" ? "Types" : "Type groups"}</h1>
          </div>
          <p className="workspace-toolbar-copy">{screenHelperLine("categoryAudit", userView)}</p>
        </div>
        <div className="header-actions">
          <button
            type="button"
            className="secondary-action"
            onClick={() => void loadAudit()}
            disabled={isLoading || isApplying}
          >
            <RefreshCw size={14} strokeWidth={2} />
            {isLoading ? "Refreshing..." : "Refresh"}
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

      {statusMessage ? <div className="status-banner">{statusMessage}</div> : null}
      {errorMessage ? (
        <div className="status-banner status-banner-error">{errorMessage}</div>
      ) : null}

      <div className="summary-matrix types-summary-strip">
        <SummaryStat
          label={userView === "beginner" ? "Still unknown" : "Backlog"}
          value={audit?.totalCandidateFiles ?? 0}
          tone="neutral"
        />
        <SummaryStat
          label={userView === "beginner" ? "Groups to fix" : "Groups"}
          value={audit?.totalGroups ?? 0}
          tone="neutral"
        />
        <SummaryStat
          label={userView === "beginner" ? "Strong matches" : "High confidence"}
          value={audit?.highConfidenceGroups ?? 0}
          tone="good"
        />
        <SummaryStat
          label={userView === "beginner" ? "Manual check" : "Unknown"}
          value={audit?.unknownFiles ?? 0}
          tone="low"
        />
      </div>

      <div className="audit-layout">
        <ResizableEdgeHandle
          label="Resize group list"
          value={auditGroupWidth}
          min={240}
          max={520}
          onChange={setAuditGroupWidth}
          side="right"
          className="layout-resize-handle audit-layout-handle"
        />
        <div className="audit-column">
          <div className="panel-card">
            <div className="panel-heading">
              <div>
                <p className="eyebrow">Step 1</p>
                <h2>{userView === "beginner" ? "Choose a type group" : "Type groups"}</h2>
              </div>
              <span className="ghost-chip">
                {audit?.groups.length ?? 0} shown
              </span>
            </div>

            <div className="audit-rail-note">
              <strong>Teach once, reuse later.</strong>
              <p>
                Pick one group, check a couple of examples, then save the label for the whole batch on the right.
              </p>
            </div>

            <div className="audit-filter-grid">
              <label className="field">
                <span>Search</span>
                <input
                  value={search}
                  onChange={(event) => setSearch(event.target.value)}
                  placeholder="Type, subtype, filename"
                />
              </label>

              {userView !== "beginner" ? (
                <label className="field">
                  <span>Min size</span>
                  <select
                    value={minGroupSize}
                    onChange={(event) => setMinGroupSize(event.target.value)}
                  >
                    <option value="2">2+</option>
                    <option value="3">3+</option>
                    <option value="5">5+</option>
                    <option value="8">8+</option>
                  </select>
                </label>
              ) : null}
            </div>

            <div className="audit-group-list">
              {audit?.groups.length ? (
                audit.groups.map((group, index) => (
                  <m.button
                    key={group.id}
                    type="button"
                    className={`audit-group-row ${
                      selectedGroupId === group.id ? "is-selected" : ""
                    } audit-group-row-${confidenceTone(group.confidence)}`}
                    onClick={() => {
                      setStatusMessage(null);
                      setSelectedGroupId(group.id);
                    }}
                    title={`${group.itemCount} files`}
                    whileHover={rowHover}
                    whileTap={rowPress}
                    {...stagedListItem(index)}
                  >
                    <div className="audit-group-main">
                      <strong>
                        {friendlyTypeLabel(group.suggestedKind)}
                        {group.suggestedSubtype ? ` / ${group.suggestedSubtype}` : ""}
                      </strong>
                      <span>{group.itemCount.toLocaleString()} files</span>
                    </div>
                    <div className="audit-group-meta">
                      {group.keywordSamples[0] ? (
                        <span className="ghost-chip">clue: {group.keywordSamples[0]}</span>
                      ) : null}
                      <span
                        className={`confidence-badge ${confidenceTone(group.confidence)}`}
                      >
                        {confidenceLabel(group.confidence)}
                      </span>
                    </div>
                  </m.button>
                ))
              ) : (
                <StatePanel
                  eyebrow={userView === "beginner" ? "Types" : "Type groups"}
                  title="No groups match"
                  body={
                    userView === "beginner"
                      ? "Try a broader search or lower the minimum size to show smaller mod-type groups."
                      : "Clear the search or reduce the minimum group size to surface weaker type groups."
                  }
                  icon={SearchX}
                  compact
                />
              )}
            </div>
          </div>
        </div>

        <div className="audit-column audit-stage">
          <ResizableEdgeHandle
            label="Resize sample and leftover sections"
            value={auditStageHeight}
            min={220}
            max={620}
            onChange={setAuditStageHeight}
            side="bottom"
            className="layout-resize-handle audit-stage-handle"
          />
          <div className="panel-card">
            <div className="panel-heading">
              <div>
                <p className="eyebrow">Step 2</p>
                <h2>{userView === "beginner" ? "Check these example files" : "Files in group"}</h2>
              </div>
              <div className="header-actions">
                {selectedGroup && selectedGroup.itemCount > selectedGroup.sampleFiles.length ? (
                  <button
                    type="button"
                    className="secondary-action"
                    onClick={() => void handleToggleGroupFiles()}
                    disabled={loadingGroupId === selectedGroup.id}
                  >
                    {loadingGroupId === selectedGroup.id
                      ? "Loading..."
                      : isShowingAllFiles
                        ? userView === "beginner"
                          ? "Show examples only"
                          : "Show sample"
                        : userView === "beginner"
                          ? `Show all ${selectedGroup.itemCount.toLocaleString()}`
                          : `Show all ${selectedGroup.itemCount.toLocaleString()} files`}
                  </button>
                ) : null}
                {selectedGroup ? (
                  <span className="ghost-chip">
                    {isShowingAllFiles
                      ? `${visibleGroupFiles.length.toLocaleString()} shown`
                      : `${visibleGroupFiles.length.toLocaleString()} of ${selectedGroup.itemCount.toLocaleString()} shown`}
                  </span>
                ) : null}
              </div>
            </div>

            {selectedGroup ? (
              <div className="audit-file-list">
                {visibleGroupFiles.map((file, index) => (
                  <CategoryAuditFileRow
                    key={file.id}
                    index={index}
                    file={file}
                    suggestedKind={selectedGroup.suggestedKind}
                    suggestedSubtype={selectedGroup.suggestedSubtype}
                    userView={userView}
                  />
                ))}
              </div>
            ) : (
                <StatePanel
                  eyebrow="Samples"
                  title="Select a group"
                  body={
                    userView === "beginner"
                      ? "Pick one type group from the left to preview a few example files before you save the type."
                      : "Choose a type group to inspect the sample files and confirm the type before applying it."
                  }
                  icon={Shapes}
                  compact
                />
            )}
          </div>

          <div className="panel-card">
            <div className="panel-heading">
              <div>
                <p className="eyebrow">Edge cases</p>
                <h2>{userView === "beginner" ? "Still unsure" : "Still unresolved"}</h2>
              </div>
              <span className="ghost-chip">
                {audit?.unresolvedSamples.length ?? 0} sampled
              </span>
            </div>

            <div className="audit-unresolved-list">
              {audit?.unresolvedSamples.length ? (
                audit.unresolvedSamples.map((file, index) => (
                  <m.div
                    key={file.id}
                    className="audit-unresolved-row audit-unresolved-row-low"
                    whileHover={rowHover}
                    {...stagedListItem(index)}
                  >
                    <div className="audit-group-main">
                      <strong>{file.filename}</strong>
                      <span>
                        {friendlyTypeLabel(file.currentKind)}
                        {file.currentSubtype ? ` / ${file.currentSubtype}` : ""}
                      </span>
                    </div>
                    <div className="audit-group-meta">
                      <span
                        className={`confidence-badge ${confidenceTone(file.confidence)}`}
                      >
                        {Math.round(file.confidence * 100)}%
                      </span>
                    </div>
                  </m.div>
                ))
              ) : (
                <StatePanel
                  eyebrow="Unresolved"
                  title={
                    userView === "beginner"
                      ? "No extra leftovers"
                      : "No leftover samples"
                  }
                  body={
                    userView === "beginner"
                      ? "That means the sampled backlog is grouping cleanly right now."
                      : "The sampled backlog does not currently have extra unmatched files outside the shown groups."
                  }
                  icon={Sparkles}
                  tone="good"
                  compact
                />
              )}
            </div>
          </div>
        </div>

        <ResizableDetailPanel className="audit-inspector" ariaLabel="Category audit details">
          {selectedGroup ? (
            <>
              <div className="detail-header">
                <div>
                  <p className="eyebrow">Step 3</p>
                  <h2>
                    {friendlyTypeLabel(selectedGroup.suggestedKind)}
                    {selectedGroup.suggestedSubtype
                      ? ` / ${selectedGroup.suggestedSubtype}`
                      : ""}
                  </h2>
                </div>
                <span className="ghost-chip">
                  {selectedGroup.itemCount.toLocaleString()} files
                </span>
              </div>

              <DockSectionStack
                layoutId="categoryAuditInspector"
                sections={categoryAuditInspectorSections}
                intro={
                  userView === "beginner"
                    ? "Keep the clues you care about open and hide the rest while you batch-fix type names."
                    : "Reorder or collapse type sections to fit quick batches or deeper checks."
                }
              />
            </>
          ) : (
            <StatePanel
                  eyebrow={userView === "beginner" ? "Types" : "Type groups"}
              title="Select a group"
              body={
                userView === "beginner"
                  ? "The right panel will explain the group and save one shared type across the full batch when you confirm it."
                  : "The inspector holds the grouped clues and save controls for the selected type group."
              }
              icon={Shapes}
              meta={["Applies to future scans", "Does not move files"]}
            />
          )}
        </ResizableDetailPanel>
      </div>
    </section>
  );
}

function CategoryAuditFileRow({
  index,
  file,
  suggestedKind,
  suggestedSubtype,
  userView,
}: {
  index: number;
  file: CategoryAuditFile;
  suggestedKind: string;
  suggestedSubtype: string | null;
  userView: UserView;
}) {
  return (
    <m.div
      className="audit-file-row audit-file-row-sample"
      whileHover={rowHover}
      {...stagedListItem(index)}
    >
      <div className="audit-group-main">
        <strong>{file.filename}</strong>
        <span>
          {friendlyTypeLabel(file.currentKind)}
          {file.currentSubtype ? ` / ${file.currentSubtype}` : ""}
          {" -> "}
          {friendlyTypeLabel(suggestedKind)}
          {suggestedSubtype ? ` / ${suggestedSubtype}` : ""}
        </span>
      </div>
      {userView !== "beginner" ? (
        <div className="audit-file-path">{file.path}</div>
      ) : null}
      {userView === "power" && (file.keywordSamples.length || file.matchReasons.length) ? (
        <div className="tag-list">
          {file.keywordSamples.map((sample) => (
            <span key={sample} className="ghost-chip">
              {sample}
            </span>
          ))}
          {file.matchReasons.map((reason) => (
            <span key={reason} className="ghost-chip">
              {reason}
            </span>
          ))}
        </div>
      ) : null}
    </m.div>
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

function confidenceLabel(confidence: number) {
  if (confidence >= 0.85) {
    return "Strong";
  }

  if (confidence >= 0.6) {
    return "Likely";
  }

  return "Weak";
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

function toErrorMessage(error: unknown) {
  if (error instanceof Error) {
    return error.message;
  }

  return String(error);
}
