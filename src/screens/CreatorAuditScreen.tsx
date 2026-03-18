import { startTransition, useDeferredValue, useEffect, useState } from "react";
import { m } from "motion/react";
import { Fingerprint, RefreshCw, SearchX, ShieldAlert, Sparkles } from "lucide-react";
import { DockSectionStack } from "../components/DockSectionStack";
import { ResizableEdgeHandle } from "../components/ResizableEdgeHandle";
import { ResizableDetailPanel } from "../components/ResizableDetailPanel";
import { StatePanel } from "../components/StatePanel";
import { useUiPreferences } from "../components/UiPreferencesContext";
import { api } from "../lib/api";
import { hoverLift, rowHover, rowPress, stagedListItem } from "../lib/motion";
import { friendlyTypeLabel, reviewLabel, screenHelperLine } from "../lib/uiLanguage";
import type {
  CreatorAuditFile,
  CreatorAuditResponse,
  Screen,
  UserView,
} from "../lib/types";

interface CreatorAuditScreenProps {
  refreshVersion: number;
  onNavigate: (screen: Screen) => void;
  onDataChanged: () => void;
  userView: UserView;
}

export function CreatorAuditScreen({
  refreshVersion,
  onNavigate,
  onDataChanged,
  userView,
}: CreatorAuditScreenProps) {
  const { auditGroupWidth, setAuditGroupWidth, auditStageHeight, setAuditStageHeight } =
    useUiPreferences();
  const [audit, setAudit] = useState<CreatorAuditResponse | null>(null);
  const [selectedGroupId, setSelectedGroupId] = useState<string | null>(null);
  const [search, setSearch] = useState("");
  const [minGroupSize, setMinGroupSize] = useState("2");
  const [creatorDraft, setCreatorDraft] = useState("");
  const [aliasDraft, setAliasDraft] = useState("");
  const [lockPreference, setLockPreference] = useState(false);
  const [preferredPathDraft, setPreferredPathDraft] = useState("");
  const [statusMessage, setStatusMessage] = useState<string | null>(null);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [isApplying, setIsApplying] = useState(false);
  const [expandedGroups, setExpandedGroups] = useState<Record<string, boolean>>({});
  const [groupFileCache, setGroupFileCache] = useState<
    Record<string, CreatorAuditFile[]>
  >({});
  const [loadingGroupId, setLoadingGroupId] = useState<string | null>(null);
  const deferredSearch = useDeferredValue(search);

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
  const creatorAuditInspectorSections = selectedGroup
    ? [
        {
          id: "what-save-does",
          label: userView === "beginner" ? "What save does" : "Save for this group",
          hint:
            userView === "beginner"
              ? "This saves one creator for the whole group."
              : "Saves one creator name across the selected group.",
          children: (
            <div className="audit-what-card">
              <strong>What save does</strong>
              <span>
                SimSuite will remember this creator for this whole group and reuse it on later scans.
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
              : "Main type, confidence, and known-creator status.",
          children: (
            <div className="detail-list">
              <DetailRow
                label="Main type"
                value={friendlyTypeLabel(selectedGroup.dominantKind)}
              />
              <DetailRow
                label="Confidence"
                value={`${Math.round(selectedGroup.confidence * 100)}%`}
              />
              <DetailRow
                label={userView === "beginner" ? "Already known?" : "Known creator"}
                value={selectedGroup.knownCreator ? "Yes" : "No"}
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
                hint: "Shared signals pulled from filenames, paths, and inspection.",
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
        ...(selectedGroup.aliasSamples.length
          ? [
              {
                id: "aliases",
                label:
                  userView === "beginner" ? "Seen in file names" : "Filename clues",
                hint:
                  userView === "beginner"
                    ? "Common tags or names seen in this group."
                    : "Alias samples pulled from filenames.",
                defaultCollapsed: false,
                children: (
                  <div className="creator-suggestion-strip">
                    {selectedGroup.aliasSamples.map((alias) => (
                      <button
                        key={alias}
                        type="button"
                        className={`creator-suggestion ${
                          aliasDraft === alias ? "is-active" : ""
                        }`}
                        onClick={() => setAliasDraft(alias)}
                        title={`Use ${alias}`}
                      >
                        {alias}
                      </button>
                    ))}
                  </div>
                ),
              },
            ]
          : []),
        {
          id: "teach",
          label: "Save creator",
          hint:
            userView === "beginner"
              ? "Enter the creator once for this whole batch."
              : "Save the creator name, an extra clue, and an optional folder lock.",
          children: (
            <>
              <div className="creator-learning-grid">
                <label className="field">
                  <span>Creator</span>
                  <input
                    value={creatorDraft}
                    onChange={(event) => setCreatorDraft(event.target.value)}
                    placeholder={userView === "beginner" ? "Creator name" : "Creator"}
                  />
                </label>

                {userView !== "beginner" ? (
                  <label className="field">
                    <span>Also save this clue</span>
                    <input
                      value={aliasDraft}
                      onChange={(event) => setAliasDraft(event.target.value)}
                      placeholder="[creator] or filename prefix"
                    />
                  </label>
                ) : null}
              </div>

              {userView !== "beginner" ? (
                <>
                  <label className="creator-toggle">
                    <input
                      type="checkbox"
                      checked={lockPreference}
                      onChange={(event) => setLockPreference(event.target.checked)}
                    />
                    <span>Lock this creator to one preview folder</span>
                  </label>

                  {lockPreference ? (
                    <label className="field">
                      <span>Preferred path</span>
                      <input
                        value={preferredPathDraft}
                        onChange={(event) =>
                          setPreferredPathDraft(event.target.value)
                        }
                        placeholder="e.g. CAS/Hair/dogsill"
                      />
                    </label>
                  ) : null}
                </>
              ) : null}

              <div className="creator-learning-actions">
                <button
                  type="button"
                  className="primary-action"
                  disabled={!creatorDraft.trim() || isApplying}
                  onClick={() => void handleApplyGroup()}
                >
                  {isApplying
                    ? "Applying..."
                    : userView === "beginner"
                      ? `Save this name for ${selectedGroup.itemCount.toLocaleString()} files`
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
      setCreatorDraft("");
      setAliasDraft("");
      setLockPreference(false);
      setPreferredPathDraft("");
      return;
    }

    setCreatorDraft(selectedGroup.suggestedCreator);
    setAliasDraft(selectedGroup.aliasSamples[0] ?? "");
    setLockPreference(false);
    setPreferredPathDraft("");
  }, [selectedGroup]);

  async function loadAudit() {
    setIsLoading(true);
    setErrorMessage(null);

    try {
      const response = await api.getCreatorAudit({
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
    if (!selectedGroup || !creatorDraft.trim()) {
      return;
    }

      const confirmed = globalThis.confirm(
      `Apply ${creatorDraft.trim()} to ${selectedGroup.itemCount} files in this creator group? SimSuite will remember it on later scans too.`,
    );
    if (!confirmed) {
      return;
    }

    setIsApplying(true);
    setErrorMessage(null);
    setStatusMessage(null);

    try {
      const result = await api.applyCreatorAudit(
        selectedGroup.fileIds,
        creatorDraft.trim(),
        aliasDraft.trim() || undefined,
        lockPreference,
        preferredPathDraft.trim() || undefined,
      );
      setStatusMessage(
        `Saved ${result.creatorName} for ${result.updatedCount} files and cleared ${result.clearedReviewCount} review items.`,
      );
      onDataChanged();
      await loadAudit();
    } catch (error) {
      setErrorMessage(toErrorMessage(error));
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
        const files = await api.getCreatorAuditGroupFiles(selectedGroup.id);
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
    <section className="screen-shell workbench">
      <div className="screen-header-row">
        <div className="screen-heading">
          <p className="eyebrow">{userView === "beginner" ? "Name cleanup" : "Learning"}</p>
          <div className="screen-title-row">
            <Fingerprint size={18} strokeWidth={2} />
            <h1>{userView === "beginner" ? "Creators" : "Creator groups"}</h1>
          </div>
          <p className="workspace-toolbar-copy">{screenHelperLine("creatorAudit", userView)}</p>
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

      <div className="learning-flow-strip">
        <LearningStep
          index={0}
          step="1"
          title="Pick a group"
          detail="SimSuite groups files that look like they came from the same creator."
        />
        <LearningStep
          index={1}
          step="2"
          title="Check examples"
          detail="Look over a few filenames before you save anything."
        />
        <LearningStep
          index={2}
          step="3"
          title="Save once"
          detail="Your saved name will be reused on future scans. No files move here."
        />
      </div>

      <div className="summary-matrix">
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
          label={userView === "beginner" ? "Manual check" : "Unresolved"}
          value={audit?.unresolvedFiles ?? 0}
          tone="low"
        />
        {userView === "power" ? (
          <SummaryStat
            label="Root loose"
            value={audit?.rootLooseFiles ?? 0}
            tone="low"
          />
        ) : null}
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
                <h2>{userView === "beginner" ? "Choose a creator group" : "Creator groups"}</h2>
              </div>
              <span className="ghost-chip">
                {audit?.groups.length ?? 0} shown
              </span>
            </div>

            <div className="audit-filter-grid">
              <label className="field">
                <span>Search</span>
                <input
                  value={search}
                  onChange={(event) => setSearch(event.target.value)}
                  placeholder="Creator, alias, filename"
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
                      <strong>{group.suggestedCreator}</strong>
                      <span>
                        {friendlyTypeLabel(group.dominantKind)} ·{" "}
                        {group.itemCount.toLocaleString()} files
                      </span>
                    </div>
                    <div className="audit-group-meta">
                      {group.aliasSamples[0] ? (
                        <span className="ghost-chip">
                          clue: {group.aliasSamples[0]}
                        </span>
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
                  eyebrow={userView === "beginner" ? "Creators" : "Creator groups"}
                  title="No groups match"
                  body={
                    userView === "beginner"
                      ? "Try a broader search or lower the minimum size if you want SimSuite to show smaller creator-name groups."
                      : "Clear the search or reduce the minimum group size to surface weaker creator groups."
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
                  <AuditFileRow
                    key={file.id}
                    index={index}
                    file={file}
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
                      ? "Pick one creator group from the left to preview a few matching filenames before you save anything."
                      : "Choose a creator group to inspect the sample files and decide whether the shared clues are trustworthy."
                  }
                  icon={Fingerprint}
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
                        {friendlyTypeLabel(file.kind)}
                        {userView !== "beginner" && file.subtype
                          ? ` · ${file.subtype}`
                          : ""}
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
                      ? "This means the sampled backlog is grouping cleanly right now."
                      : "The sampled backlog does not currently have extra edge cases outside the grouped files."
                  }
                  icon={Sparkles}
                  tone="good"
                  compact
                />
              )}
            </div>
          </div>
        </div>

        <ResizableDetailPanel className="audit-inspector" ariaLabel="Creator audit details">
          {selectedGroup ? (
            <>
              <div className="detail-header">
                <div>
                  <p className="eyebrow">Step 3</p>
                  <h2>{selectedGroup.suggestedCreator}</h2>
                </div>
                <span className="ghost-chip">
                  {selectedGroup.itemCount.toLocaleString()} files
                </span>
              </div>

              <DockSectionStack
                layoutId="creatorAuditInspector"
                sections={creatorAuditInspectorSections}
                intro={
                  userView === "beginner"
                    ? "Keep the clues you care about open and tuck the rest away while you batch-fix creators."
                    : "Reorder or collapse creator sections to fit quick batches or deeper checks."
                }
              />
            </>
          ) : (
            <StatePanel
              eyebrow={userView === "beginner" ? "Creators" : "Creator groups"}
              title="Select a group"
              body={
                userView === "beginner"
                  ? "The right panel will explain the group and save one creator across the full batch when you confirm it."
                  : "The inspector holds the shared clues, extra name hints, and save controls for the selected creator group."
              }
              icon={Fingerprint}
              meta={["Applies to future scans", "Does not move files"]}
            />
          )}
        </ResizableDetailPanel>
      </div>
    </section>
  );
}

function LearningStep({
  index,
  step,
  title,
  detail,
}: {
  index: number;
  step: string;
  title: string;
  detail: string;
}) {
  return (
    <m.div
      className="learning-step"
      whileHover={hoverLift}
      {...stagedListItem(index)}
    >
      <span className="learning-step-index">{step}</span>
      <div className="learning-step-copy">
        <strong>{title}</strong>
        <span>{detail}</span>
      </div>
    </m.div>
  );
}

function AuditFileRow({
  index,
  file,
  userView,
}: {
  index: number;
  file: CreatorAuditFile;
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
          {friendlyTypeLabel(file.kind)}
          {file.subtype ? ` · ${file.subtype}` : ""}
          {userView === "power" && file.currentCreator
            ? ` · ${file.currentCreator}`
            : ""}
        </span>
      </div>
      {userView !== "beginner" ? (
        <div className="audit-file-path">{file.path}</div>
      ) : null}
      {userView === "power" && file.matchReasons.length ? (
        <div className="tag-list">
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
