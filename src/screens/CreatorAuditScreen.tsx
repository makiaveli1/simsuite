import { startTransition, useDeferredValue, useEffect, useState } from "react";
import { Fingerprint, RefreshCw, ShieldAlert } from "lucide-react";
import { DockSectionStack } from "../components/DockSectionStack";
import { ResizableEdgeHandle } from "../components/ResizableEdgeHandle";
import { ResizableDetailPanel } from "../components/ResizableDetailPanel";
import { useUiPreferences } from "../components/UiPreferencesContext";
import { api } from "../lib/api";
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
          label: userView === "beginner" ? "What save does" : "Batch learning",
          hint:
            userView === "beginner"
              ? "This teaches one creator name to the whole group."
              : "Applies one creator identity across the selected cluster.",
          children: (
            <div className="audit-what-card">
              <strong>What save does</strong>
              <span>
                SimSuite will remember this creator name for this whole group and reuse it on later scans.
              </span>
            </div>
          ),
        },
        {
          id: "cluster-facts",
          label: userView === "beginner" ? "Group summary" : "Cluster facts",
          hint:
            userView === "beginner"
              ? "How many files are in this group and how strong the match is."
              : "Dominant kind, confidence, and known-creator status.",
          children: (
            <div className="detail-list">
              <DetailRow
                label={userView === "beginner" ? "Main type" : "Dominant kind"}
                value={selectedGroup.dominantKind}
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
          label: userView === "beginner" ? "Save creator name" : "Teach creator",
          hint:
            userView === "beginner"
              ? "Enter the maker name once for this whole batch."
              : "Set the canonical creator, alias, and optional route lock.",
          children: (
            <>
              <div className="creator-learning-grid">
                <label className="field">
                  <span>{userView === "beginner" ? "Creator name" : "Canonical creator"}</span>
                  <input
                    value={creatorDraft}
                    onChange={(event) => setCreatorDraft(event.target.value)}
                    placeholder={userView === "beginner" ? "Who made these files?" : "Creator name"}
                  />
                </label>

                {userView !== "beginner" ? (
                  <label className="field">
                    <span>Alias to learn</span>
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
      `Apply ${creatorDraft.trim()} to ${selectedGroup.itemCount} files in this creator cluster? This will teach future scans too.`,
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
        `Learned ${result.creatorName} for ${result.updatedCount} files and cleared ${result.clearedReviewCount} review items.`,
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
    <section className="screen-shell">
      <div className="screen-header-row">
        <div className="screen-heading">
          <p className="eyebrow">{userView === "beginner" ? "Name cleanup" : "Learning"}</p>
          <div className="screen-title-row">
            <Fingerprint size={18} strokeWidth={2} />
            <h1>{userView === "beginner" ? "Creator Names" : "Creator Audit"}</h1>
          </div>
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
            {userView === "beginner" ? "Needs attention" : "Review"}
          </button>
        </div>
      </div>

      {statusMessage ? <div className="status-banner">{statusMessage}</div> : null}
      {errorMessage ? (
        <div className="status-banner status-banner-error">{errorMessage}</div>
      ) : null}

      <div className="learning-flow-strip">
        <LearningStep
          step="1"
          title="Pick a group"
          detail="SimSuite groups files that look like they came from the same creator."
        />
        <LearningStep
          step="2"
          title="Check examples"
          detail="Look over a few filenames before you save anything."
        />
        <LearningStep
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
                <h2>{userView === "beginner" ? "Choose a creator group" : "Grouped suggestions"}</h2>
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
                audit.groups.map((group) => (
                  <button
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
                  >
                    <div className="audit-group-main">
                      <strong>{group.suggestedCreator}</strong>
                      <span>
                        {friendlyKindLabel(group.dominantKind)} ·{" "}
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
                  </button>
                ))
              ) : (
                <div className="detail-empty compact-empty">
                  <p className="eyebrow">
                    {userView === "beginner" ? "Creator names" : "Creator audit"}
                  </p>
                  <h2>No clusters match</h2>
                </div>
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
                <h2>{userView === "beginner" ? "Check these example files" : "Files in cluster"}</h2>
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
                {visibleGroupFiles.map((file) => (
                  <AuditFileRow key={file.id} file={file} userView={userView} />
                ))}
              </div>
            ) : (
                <div className="detail-empty compact-empty">
                  <p className="eyebrow">Samples</p>
                  <h2>{userView === "beginner" ? "Select a group" : "Select a cluster"}</h2>
                </div>
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
                audit.unresolvedSamples.map((file) => (
                  <div key={file.id} className="audit-unresolved-row audit-unresolved-row-low">
                    <div className="audit-group-main">
                      <strong>{file.filename}</strong>
                      <span>
                        {file.kind}
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
                  </div>
                ))
              ) : (
                <div className="detail-empty compact-empty">
                  <p className="eyebrow">Unresolved</p>
                  <h2>{userView === "beginner" ? "No extra leftovers" : "No leftover samples"}</h2>
                </div>
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
                    ? "Keep the clues you care about open and tuck the rest away while you batch-fix creator names."
                    : "Reorder or collapse creator-audit sections to fit quick batches or deeper verification."
                }
              />
            </>
          ) : (
            <div className="detail-empty">
              <p className="eyebrow">
                {userView === "beginner" ? "Creator names" : "Creator audit"}
              </p>
              <h2>{userView === "beginner" ? "Select a group" : "Select a cluster"}</h2>
            </div>
          )}
        </ResizableDetailPanel>
      </div>
    </section>
  );
}

function LearningStep({
  step,
  title,
  detail,
}: {
  step: string;
  title: string;
  detail: string;
}) {
  return (
    <div className="learning-step">
      <span className="learning-step-index">{step}</span>
      <div className="learning-step-copy">
        <strong>{title}</strong>
        <span>{detail}</span>
      </div>
    </div>
  );
}

function AuditFileRow({
  file,
  userView,
}: {
  file: CreatorAuditFile;
  userView: UserView;
}) {
  return (
    <div className="audit-file-row audit-file-row-sample">
      <div className="audit-group-main">
        <strong>{file.filename}</strong>
        <span>
          {file.kind}
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
