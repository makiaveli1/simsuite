import { useDeferredValue, useEffect, useState } from "react";
import { m } from "motion/react";
import { CornerUpLeft, LibraryBig } from "lucide-react";
import { DockSectionStack } from "../components/DockSectionStack";
import { LayoutPresetBar } from "../components/LayoutPresetBar";
import { ResizableEdgeHandle } from "../components/ResizableEdgeHandle";
import { ResizableDetailPanel } from "../components/ResizableDetailPanel";
import { useUiPreferences } from "../components/UiPreferencesContext";
import { api } from "../lib/api";
import { rowHover, rowPress, stagedListItem } from "../lib/motion";
import {
  friendlyTypeLabel,
  screenHelperLine,
  screenLabel,
  unknownCreatorLabel,
} from "../lib/uiLanguage";
import type {
  CategoryOverrideInfo,
  CreatorLearningInfo,
  FileDetail,
  LibraryLayoutPreset,
  LibraryFacets,
  LibraryFileRow,
  LibraryListResponse,
  Screen,
  UserView,
  VersionConfidence,
  WatchResult,
  WatchSourceKind,
} from "../lib/types";

interface LibraryScreenProps {
  refreshVersion: number;
  onNavigate: (screen: Screen) => void;
  userView: UserView;
}

const PAGE_SIZE = 100;
const LIBRARY_LAYOUT_PRESETS: Array<{
  id: LibraryLayoutPreset;
  label: string;
  hint: string;
}> = [
  {
    id: "browse",
    label: "Browse",
    hint: "Balanced list and inspector with filters open.",
  },
  {
    id: "inspect",
    label: "Inspect",
    hint: "Gives the right-hand inspector more room for details.",
  },
  {
    id: "catalog",
    label: "Catalog",
    hint: "Puts more space on the table and hides filters until needed.",
  },
];

export function LibraryScreen({
  refreshVersion,
  onNavigate,
  userView,
}: LibraryScreenProps) {
  const {
    libraryDetailWidth,
    libraryTableHeight,
    setLibraryDetailWidth,
    setLibraryTableHeight,
    libraryFiltersCollapsed,
    setLibraryFiltersCollapsed,
    libraryLayoutPreset,
    applyLibraryLayoutPreset,
  } = useUiPreferences();
  const [facets, setFacets] = useState<LibraryFacets | null>(null);
  const [rows, setRows] = useState<LibraryListResponse | null>(null);
  const [selected, setSelected] = useState<FileDetail | null>(null);
  const [search, setSearch] = useState("");
  const [kind, setKind] = useState("");
  const [subtype, setSubtype] = useState("");
  const [creator, setCreator] = useState("");
  const [source, setSource] = useState("");
  const [minConfidence, setMinConfidence] = useState("");
  const [page, setPage] = useState(0);
  const [creatorDraft, setCreatorDraft] = useState("");
  const [aliasDraft, setAliasDraft] = useState("");
  const [lockPreference, setLockPreference] = useState(false);
  const [preferredPathDraft, setPreferredPathDraft] = useState("");
  const [creatorMessage, setCreatorMessage] = useState<string | null>(null);
  const [savingCreator, setSavingCreator] = useState(false);
  const [categoryKindDraft, setCategoryKindDraft] = useState("");
  const [categorySubtypeDraft, setCategorySubtypeDraft] = useState("");
  const [categoryMessage, setCategoryMessage] = useState<string | null>(null);
  const [savingCategory, setSavingCategory] = useState(false);
  const [watchEditing, setWatchEditing] = useState(false);
  const [watchSourceKind, setWatchSourceKind] = useState<WatchSourceKind>("exact_page");
  const [watchSourceLabel, setWatchSourceLabel] = useState("");
  const [watchSourceUrl, setWatchSourceUrl] = useState("");
  const [savingWatch, setSavingWatch] = useState(false);
  const [watchMessage, setWatchMessage] = useState<string | null>(null);
  const deferredSearch = useDeferredValue(search);

  useEffect(() => {
    void api.getLibraryFacets().then(setFacets);
  }, [refreshVersion]);

  useEffect(() => {
    void loadRows();
  }, [
    refreshVersion,
    deferredSearch,
    kind,
    subtype,
    creator,
    source,
    minConfidence,
    page,
  ]);

  useEffect(() => {
    if (!selected) {
      setCreatorDraft("");
      setAliasDraft("");
      setLockPreference(false);
      setPreferredPathDraft("");
      setCreatorMessage(null);
      setCategoryKindDraft("");
      setCategorySubtypeDraft("");
      setCategoryMessage(null);
      setWatchEditing(false);
      setWatchSourceKind("exact_page");
      setWatchSourceLabel("");
      setWatchSourceUrl("");
      setWatchMessage(null);
      return;
    }

    setCreatorDraft(selected.creator ?? selected.insights.creatorHints[0] ?? "");
    setAliasDraft("");
    setLockPreference(selected.creatorLearning.lockedByUser);
    setPreferredPathDraft(selected.creatorLearning.preferredPath ?? "");
    setCreatorMessage(null);
    setCategoryKindDraft(selected.categoryOverride.kind ?? selected.kind);
    setCategorySubtypeDraft(selected.categoryOverride.subtype ?? selected.subtype ?? "");
    setCategoryMessage(null);

    const currentWatch = selected.watchResult;
    if (currentWatch?.sourceKind && currentWatch.sourceUrl) {
      setWatchSourceKind(currentWatch.sourceKind);
      setWatchSourceLabel(currentWatch.sourceLabel ?? "");
      setWatchSourceUrl(currentWatch.sourceUrl);
    } else {
      setWatchSourceKind("exact_page");
      setWatchSourceLabel("");
      setWatchSourceUrl("");
    }
    setWatchEditing(false);
    setWatchMessage(null);
  }, [selected]);

  async function loadRows(preferredSelectedId?: number) {
    const result = await api.listLibraryFiles({
      search: deferredSearch || undefined,
      kind: kind || undefined,
      subtype: subtype || undefined,
      creator: creator || undefined,
      source: source || undefined,
      minConfidence: minConfidence ? Number(minConfidence) : undefined,
      limit: PAGE_SIZE,
      offset: page * PAGE_SIZE,
    });

    setRows(result);

    const detailId = preferredSelectedId ?? selected?.id ?? result.items[0]?.id;
    if (detailId) {
      setSelected(await api.getFileDetail(detailId));
    } else {
      setSelected(null);
    }
  }

  async function openFile(row: LibraryFileRow) {
    setSelected(await api.getFileDetail(row.id));
  }

  async function saveCreatorOverride() {
    const trimmedCreator = creatorDraft.trim();
    if (!selected || !trimmedCreator) {
      return;
    }

    setSavingCreator(true);
    setCreatorMessage(null);

    try {
      const updated = await api.saveCreatorLearning(
        selected.id,
        trimmedCreator,
        aliasDraft.trim() || undefined,
        lockPreference,
        preferredPathDraft.trim() || undefined,
      );

      if (!updated) {
        return;
      }

      setSelected(updated);
      setCreatorMessage("Saved for future scans.");
      await Promise.all([loadRows(updated.id), api.getLibraryFacets().then(setFacets)]);
    } finally {
      setSavingCreator(false);
    }
  }

  async function saveCategoryClassification() {
    const trimmedKind = categoryKindDraft.trim();
    if (!selected || !trimmedKind) {
      return;
    }

    setSavingCategory(true);
    setCategoryMessage(null);

    try {
      const updated = await api.saveCategoryOverride(
        selected.id,
        trimmedKind,
        categorySubtypeDraft.trim() || undefined,
      );

      if (!updated) {
        return;
      }

      setSelected(updated);
      setCategoryMessage("Type override saved.");
      await Promise.all([loadRows(updated.id), api.getLibraryFacets().then(setFacets)]);
    } finally {
      setSavingCategory(false);
    }
  }

  const creatorSuggestions = selected
    ? Array.from(
        new Set(
          [selected.creator, ...selected.insights.creatorHints].filter(
            (value): value is string => Boolean(value),
          ),
        ),
      )
    : [];
  const categoryKindOptions = facets?.taxonomyKinds?.length
    ? facets.taxonomyKinds
    : ["CAS", "BuildBuy", "Gameplay", "ScriptMods", "OverridesAndDefaults", "PosesAndAnimation", "PresetsAndSliders", "TrayHousehold", "TrayLot", "TrayRoom", "TrayItem", "Unknown"];
  const totalPages = rows ? Math.max(1, Math.ceil(rows.total / PAGE_SIZE)) : 1;

  const tableColumns =
    userView === "beginner"
      ? ["name", "creator", "kind", "confidence"]
      : userView === "power"
        ? ["name", "creator", "kind", "root", "depth", "confidence"]
        : ["name", "creator", "kind", "root", "confidence"];
  const hasInspectionSignals = Boolean(
    selected &&
      (selected.insights.format ||
        selected.insights.creatorHints.length ||
        selected.insights.versionHints.length ||
        selected.insights.embeddedNames.length ||
        selected.insights.scriptNamespaces.length ||
        selected.insights.resourceSummary.length),
  );
  const hasVersionWatchInfo = Boolean(
    selected?.installedVersionSummary || selected?.watchResult,
  );

  async function saveWatchSource() {
    if (!selected) {
      return;
    }

    const trimmedUrl = watchSourceUrl.trim();
    if (!trimmedUrl) {
      setWatchMessage("Enter a watch URL first.");
      return;
    }

    setSavingWatch(true);
    setWatchMessage(null);

    try {
      const updated = await api.saveWatchSourceForFile(
        selected.id,
        watchSourceKind,
        watchSourceLabel.trim() || undefined,
        trimmedUrl,
      );

      if (!updated) {
        return;
      }

      setSelected(updated);
      setWatchEditing(false);
      setWatchMessage("Watch source saved.");
      await loadRows(updated.id);
    } catch (error) {
      setWatchMessage(watchActionError(error, "save the watch source"));
    } finally {
      setSavingWatch(false);
    }
  }

  async function clearWatchSource() {
    if (!selected) {
      return;
    }

    setSavingWatch(true);
    setWatchMessage(null);

    try {
      const updated = await api.clearWatchSourceForFile(selected.id);
      if (!updated) {
        return;
      }

      setSelected(updated);
      setWatchEditing(false);
      setWatchSourceLabel("");
      setWatchSourceUrl("");
      setWatchMessage("Watch source cleared.");
      await loadRows(updated.id);
    } catch (error) {
      setWatchMessage(watchActionError(error, "clear the watch source"));
    } finally {
      setSavingWatch(false);
    }
  }
  const libraryInspectorSections = selected
    ? [
        {
          id: "facts",
          label: userView === "beginner" ? "File facts" : "File details",
          hint:
            userView === "beginner"
              ? "The basics SimSuite knows about this file."
              : "Core classification, file metadata, and confidence.",
          children: (
            <div className="detail-list">
              <DetailRow
                label="Creator"
                value={selected.creator ?? unknownCreatorLabel(userView)}
              />
              <DetailRow
                label="Type"
                value={friendlyTypeLabel(selected.kind)}
              />
              <DetailRow
                label="Subtype"
                value={
                  selected.subtype ??
                  (userView === "beginner" ? "Not set" : "Unspecified")
                }
              />
              {userView !== "beginner" ? (
                <DetailRow label="Root" value={selected.sourceLocation} />
              ) : null}
              {userView !== "beginner" ? (
                <DetailRow label="Depth" value={`${selected.relativeDepth}`} />
              ) : null}
              {userView === "power" ? (
                <DetailRow label="Size" value={formatBytes(selected.size)} />
              ) : null}
              {userView === "power" ? (
                <DetailRow
                  label="Modified"
                  value={
                    selected.modifiedAt
                      ? new Date(selected.modifiedAt).toLocaleString()
                      : "Unknown"
                  }
                />
              ) : null}
              {userView === "power" ? (
                <DetailRow label="Hash" value={selected.hash ?? "Not available"} mono />
              ) : null}
            </div>
          ),
        },
        ...(hasVersionWatchInfo
          ? [
              {
                id: "versionWatch",
                label:
                  userView === "beginner" ? "Version and updates" : "Installed version",
                hint:
                  userView === "beginner"
                    ? "What SimSuite knows about this installed item and any saved watch result."
                    : "Installed version summary, local evidence, and watch status.",
                children: (
                  <>
                    <div className="detail-list">
                      <DetailRow
                        label="Subject"
                        value={
                          selected.installedVersionSummary?.subjectLabel ??
                          selected.filename
                        }
                      />
                      <DetailRow
                        label="Installed version"
                        value={formatInstalledVersionValue(
                          selected.installedVersionSummary?.version ?? null,
                        )}
                      />
                      <DetailRow
                        label="Confidence"
                        value={versionConfidenceLabel(
                          selected.installedVersionSummary?.confidence ?? "unknown",
                        )}
                      />
                      <DetailRow
                        label="Watch status"
                        value={watchStatusLabel(selected.watchResult, userView)}
                      />
                      {selected.watchResult?.sourceKind ? (
                        <DetailRow
                          label="Watch source"
                          value={watchSourceKindLabel(selected.watchResult)}
                        />
                      ) : null}
                      {selected.watchResult?.latestVersion ? (
                        <DetailRow
                          label="Latest seen"
                          value={selected.watchResult.latestVersion}
                        />
                      ) : null}
                    </div>
                    {selected.installedVersionSummary?.evidence.length ? (
                      <div className="detail-block">
                        <div className="section-label">Local version evidence</div>
                        <div className="downloads-evidence-list">
                          {selected.installedVersionSummary.evidence.map((line) => (
                            <div key={line} className="downloads-evidence-row">
                              {line}
                            </div>
                          ))}
                        </div>
                      </div>
                    ) : null}
                    {selected.watchResult?.note || selected.watchResult?.evidence.length ? (
                      <div className="detail-block">
                        <div className="section-label">Watch notes</div>
                        <div className="downloads-evidence-list">
                          {selected.watchResult?.note ? (
                            <div className="downloads-evidence-row">
                              {selected.watchResult.note}
                            </div>
                          ) : null}
                          {selected.watchResult?.evidence.map((line) => (
                            <div key={line} className="downloads-evidence-row">
                              {line}
                            </div>
                          ))}
                        </div>
                      </div>
                    ) : null}
                    <div className="detail-block">
                      <div className="section-label">
                        {userView === "beginner" ? "Watch source" : "Watch settings"}
                      </div>
                      {!watchEditing ? (
                        <div className="detail-row-actions">
                          <button
                            type="button"
                            className="secondary-action"
                            onClick={() => {
                              setWatchEditing(true);
                              setWatchMessage(null);
                            }}
                          >
                            {selected.watchResult?.sourceKind
                              ? userView === "beginner"
                                ? "Change watch source"
                                : "Change watch source"
                              : userView === "beginner"
                                ? "Add watch source"
                                : "Add watch source"}
                          </button>
                          {selected.watchResult?.sourceKind ? (
                            <button
                              type="button"
                              className="ghost-action"
                              disabled={savingWatch}
                              onClick={() => void clearWatchSource()}
                            >
                              {userView === "beginner" ? "Stop watching" : "Clear watch source"}
                            </button>
                          ) : null}
                          {watchMessage ? (
                            <span className="creator-learning-message">{watchMessage}</span>
                          ) : null}
                        </div>
                      ) : (
                        <div className="creator-learning-grid">
                          <label className="field">
                            <span>Source type</span>
                            <select
                              value={watchSourceKind}
                              onChange={(event) =>
                                setWatchSourceKind(event.target.value as WatchSourceKind)
                              }
                            >
                              <option value="exact_page">Exact mod page</option>
                              <option value="creator_page">Creator page</option>
                            </select>
                          </label>
                          <label className="field">
                            <span>URL</span>
                            <input
                              value={watchSourceUrl}
                              onChange={(event) => setWatchSourceUrl(event.target.value)}
                              placeholder="https://example.com/mod-page"
                            />
                          </label>
                          <label className="field">
                            <span>Label (optional)</span>
                            <input
                              value={watchSourceLabel}
                              onChange={(event) => setWatchSourceLabel(event.target.value)}
                              placeholder={
                                watchSourceKind === "creator_page"
                                  ? "Creator name"
                                  : "Mod name on page"
                              }
                            />
                          </label>
                          <div className="creator-learning-actions">
                            <button
                              type="button"
                              className="primary-action"
                              disabled={savingWatch || !watchSourceUrl.trim()}
                              onClick={() => void saveWatchSource()}
                            >
                              {savingWatch ? "Saving..." : "Save watch"}
                            </button>
                            <button
                              type="button"
                              className="secondary-action"
                              disabled={savingWatch}
                              onClick={() => {
                                setWatchEditing(false);
                                setWatchMessage(null);
                                if (!selected.watchResult?.sourceKind) {
                                  setWatchSourceLabel("");
                                  setWatchSourceUrl("");
                                } else if (selected.watchResult.sourceKind) {
                                  setWatchSourceKind(selected.watchResult.sourceKind);
                                  setWatchSourceLabel(selected.watchResult.sourceLabel ?? "");
                                  setWatchSourceUrl(selected.watchResult.sourceUrl ?? "");
                                }
                              }}
                            >
                              Cancel
                            </button>
                            {watchMessage ? (
                              <span className="creator-learning-message">{watchMessage}</span>
                            ) : null}
                          </div>
                        </div>
                      )}
                    </div>
                  </>
                ),
              },
            ]
          : []),
        {
          id: "safety",
          label: userView === "beginner" ? "Move rules" : "Bundle and warnings",
          hint:
            userView === "beginner"
              ? "Shows what should stay together and any safety warnings."
              : "Bundle grouping, warnings, and parser notes.",
          badge:
            selected.safetyNotes.length > 0
              ? `${selected.safetyNotes.length} warning${selected.safetyNotes.length === 1 ? "" : "s"}`
              : selected.bundleName
                ? "bundled"
                : null,
          children: (
            <>
              <div className="detail-block">
                <div className="section-label">
                  {userView === "beginner" ? "Move with" : "Bundle"}
                </div>
                <p>
                  {selected.bundleName
                    ? `${selected.bundleName} (${selected.bundleType ?? "bundle"})`
                    : "None"}
                </p>
              </div>

              <div className="detail-block">
                <div className="section-label">
                  {userView === "beginner" ? "Safety notes" : "Warnings"}
                </div>
                {selected.safetyNotes.length ? (
                  <div className="tag-list">
                    {selected.safetyNotes.map((note) => (
                      <span key={note} className="warning-tag">
                        {note}
                      </span>
                    ))}
                  </div>
                ) : (
                  <p>No safety warnings.</p>
                )}
              </div>

              {userView !== "beginner" ? (
                <div className="detail-block">
                  <div className="section-label">Parser</div>
                  {selected.parserWarnings.length ? (
                    <div className="tag-list">
                      {selected.parserWarnings.map((note) => (
                        <span key={note} className="ghost-chip">
                          {note}
                        </span>
                      ))}
                    </div>
                  ) : (
                    <p>No parser warnings.</p>
                  )}
                </div>
              ) : null}
            </>
          ),
        },
        ...(userView !== "beginner" && hasInspectionSignals
          ? [
              {
                id: "inspection",
                label: "Inside the file",
                hint: "Signals pulled from package or script contents.",
                defaultCollapsed: userView !== "power",
                badge: selected.insights.format ?? null,
                children: (
                  <>
                    {selected.insights.format ? (
                      <DetailRow label="Format" value={selected.insights.format} />
                    ) : null}
                    {selected.insights.creatorHints.length ? (
                      <div className="detail-block">
                        <div className="section-label">Creator hints</div>
                        <div className="tag-list">
                          {selected.insights.creatorHints.map((hint) => (
                            <span key={hint} className="ghost-chip">
                              {hint}
                            </span>
                          ))}
                        </div>
                      </div>
                    ) : null}
                    {selected.insights.versionHints.length ? (
                      <div className="detail-block">
                        <div className="section-label">Version hints</div>
                        <div className="tag-list">
                          {selected.insights.versionHints.map((item) => (
                            <span key={item} className="ghost-chip">
                              {item}
                            </span>
                          ))}
                        </div>
                      </div>
                    ) : null}
                    {selected.insights.resourceSummary.length ? (
                      <div className="detail-block">
                        <div className="section-label">Resources</div>
                        <div className="tag-list">
                          {selected.insights.resourceSummary.map((item) => (
                            <span key={item} className="ghost-chip">
                              {item}
                            </span>
                          ))}
                        </div>
                      </div>
                    ) : null}
                    {selected.insights.scriptNamespaces.length ? (
                      <div className="detail-block">
                        <div className="section-label">Namespaces</div>
                        <div className="tag-list">
                          {selected.insights.scriptNamespaces.map((item) => (
                            <span key={item} className="ghost-chip">
                              {item}
                            </span>
                          ))}
                        </div>
                      </div>
                    ) : null}
                    {selected.insights.embeddedNames.length ? (
                      <div className="detail-block">
                        <div className="section-label">Embedded names</div>
                        <div className="tag-list">
                          {selected.insights.embeddedNames.map((item) => (
                            <span key={item} className="ghost-chip">
                              {item}
                            </span>
                          ))}
                        </div>
                      </div>
                    ) : null}
                  </>
                ),
              },
            ]
          : []),
        {
          id: "creator",
          label: userView === "beginner" ? "Set creator" : "Creator learning",
          hint:
            userView === "beginner"
              ? "Save the creator once and reuse it later."
              : "Save creator matches and optional folder preferences.",
          defaultCollapsed: false,
          children: (
            <CreatorLearningBlock
              userView={userView}
              creatorDraft={creatorDraft}
              aliasDraft={aliasDraft}
              lockPreference={lockPreference}
              preferredPathDraft={preferredPathDraft}
              savingCreator={savingCreator}
              creatorMessage={creatorMessage}
              suggestions={creatorSuggestions}
              creatorLearning={selected.creatorLearning}
              onCreatorDraftChange={(value) => {
                setCreatorDraft(value);
                setCreatorMessage(null);
              }}
              onAliasDraftChange={(value) => {
                setAliasDraft(value);
                setCreatorMessage(null);
              }}
              onLockPreferenceChange={(value) => {
                setLockPreference(value);
                setCreatorMessage(null);
              }}
              onPreferredPathChange={(value) => {
                setPreferredPathDraft(value);
                setCreatorMessage(null);
              }}
              onSelectSuggestion={(value) => {
                setCreatorDraft(value);
                setCreatorMessage(null);
              }}
              onSave={() => void saveCreatorOverride()}
              onOpenAudit={() => onNavigate("creatorAudit")}
            />
          ),
        },
        {
          id: "category",
          label: userView === "beginner" ? "Set type" : "Type override",
          hint:
            userView === "beginner"
              ? "Correct the type label without moving the file."
              : "Override type and subtype for later scans and previews.",
          defaultCollapsed: false,
          children: (
            <CategoryOverrideBlock
              userView={userView}
              kindOptions={categoryKindOptions}
              categoryKindDraft={categoryKindDraft}
              categorySubtypeDraft={categorySubtypeDraft}
              categoryMessage={categoryMessage}
              savingCategory={savingCategory}
              categoryOverride={selected.categoryOverride}
              onKindChange={(value) => {
                setCategoryKindDraft(value);
                setCategoryMessage(null);
              }}
              onSubtypeChange={(value) => {
                setCategorySubtypeDraft(value);
                setCategoryMessage(null);
              }}
              onSave={() => void saveCategoryClassification()}
              onOpenAudit={() => onNavigate("categoryAudit")}
            />
          ),
        },
        {
          id: "path",
          label: userView === "beginner" ? "Full file path" : "Path",
          hint:
            userView === "beginner"
              ? "Where the file lives right now."
              : "Absolute path for this indexed file.",
          defaultCollapsed: userView === "beginner",
          children: <div className="path-card">{selected.path}</div>,
        },
      ]
    : [];

  return (
    <section className="screen-shell">
      <div className="screen-header-row">
        <div className="screen-heading">
          <p className="eyebrow">{userView === "beginner" ? "Your files" : "Index"}</p>
          <div className="screen-title-row">
            <LibraryBig size={18} strokeWidth={2} />
            <h1>{screenLabel("library", userView)}</h1>
          </div>
          <p className="workspace-toolbar-copy">{screenHelperLine("library", userView)}</p>
        </div>
        <div className="header-actions">
          <button
            type="button"
            className="secondary-action"
            onClick={() => onNavigate("home")}
          >
            <CornerUpLeft size={14} strokeWidth={2} />
            Home
          </button>
        </div>
      </div>

      <LayoutPresetBar
        title={userView === "beginner" ? "Quick filters" : "Library layout"}
        summary={
          userView === "beginner"
            ? "Keep the list calm by hiding or showing the filters when you need them."
            : userView === "power"
              ? "Saved layout presets for denser browse and inspection passes."
              : "Saved layout presets for browsing, inspecting, or catalog-style review."
        }
        presets={userView === "beginner" ? [] : LIBRARY_LAYOUT_PRESETS}
        activePreset={libraryLayoutPreset}
        onApplyPreset={(preset) =>
          applyLibraryLayoutPreset(preset as LibraryLayoutPreset)
        }
        filterToggle={{
          collapsed: libraryFiltersCollapsed,
          onToggle: () => setLibraryFiltersCollapsed(!libraryFiltersCollapsed),
          hiddenLabel: "Show filters",
          shownLabel: "Hide filters",
        }}
      />

      {!libraryFiltersCollapsed ? (
        <div className="panel-card filter-panel">
          <div className="filter-grid">
            <label className="field">
              <span>Search</span>
              <input
                value={search}
                onChange={(event) => {
                  setSearch(event.target.value);
                  setPage(0);
                }}
                placeholder="Name or creator"
              />
            </label>

            <label className="field">
              <span>Type</span>
              <select
                value={kind}
                onChange={(event) => {
                  setKind(event.target.value);
                  setPage(0);
                }}
              >
                <option value="">All</option>
                {facets?.kinds.map((item) => (
                  <option key={item} value={item}>
                    {friendlyTypeLabel(item)}
                  </option>
                ))}
              </select>
            </label>

            {userView === "power" ? (
              <label className="field">
                <span>Subtype</span>
                <select
                  value={subtype}
                  onChange={(event) => {
                    setSubtype(event.target.value);
                    setPage(0);
                  }}
                >
                  <option value="">All</option>
                  {facets?.subtypes.map((item) => (
                    <option key={item} value={item}>
                      {item}
                    </option>
                  ))}
                </select>
              </label>
            ) : null}

            <label className="field">
              <span>Creator</span>
              <select
                value={creator}
                onChange={(event) => {
                  setCreator(event.target.value);
                  setPage(0);
                }}
              >
                <option value="">All</option>
                {facets?.creators.map((item) => (
                  <option key={item} value={item}>
                    {item}
                  </option>
                ))}
              </select>
            </label>

            {userView !== "beginner" ? (
              <label className="field">
                <span>Root</span>
                <select
                  value={source}
                  onChange={(event) => {
                    setSource(event.target.value);
                    setPage(0);
                  }}
                >
                  <option value="">All</option>
                  {facets?.sources.map((item) => (
                    <option key={item} value={item}>
                      {item}
                    </option>
                  ))}
                </select>
              </label>
            ) : null}

            {userView !== "beginner" ? (
              <label className="field">
                <span>Confidence</span>
                <select
                  value={minConfidence}
                  onChange={(event) => {
                    setMinConfidence(event.target.value);
                    setPage(0);
                  }}
                >
                  <option value="">Any</option>
                  <option value="0.35">35%+</option>
                  <option value="0.55">55%+</option>
                  <option value="0.75">75%+</option>
                </select>
              </label>
            ) : null}
          </div>
        </div>
      ) : null}

      <div className="library-layout">
        <div className="panel-card table-panel library-table-panel">
          <div className="table-meta">
            <div>
              <strong>{rows?.total.toLocaleString() ?? "0"}</strong>
              <span>{userView === "beginner" ? "found" : "matches"}</span>
            </div>
            <div>
              <strong>{rows?.items.length.toLocaleString() ?? "0"}</strong>
              <span>{userView === "beginner" ? "on this page" : "visible"}</span>
            </div>
          </div>

          <div className="vertical-dock library-table-dock">
            <div className="table-scroll library-table-scroll">
              <table className="library-table">
                <thead>
                  <tr>
                    <th>{userView === "beginner" ? "File" : "Name"}</th>
                    <th>Creator</th>
                    <th>Type</th>
                    {tableColumns.includes("root") ? (
                      <th>{userView === "beginner" ? "Folder" : "Root"}</th>
                    ) : null}
                    {tableColumns.includes("depth") ? <th>Depth</th> : null}
                    <th>{userView === "beginner" ? "Match" : "Confidence"}</th>
                  </tr>
                </thead>
                <tbody>
                  {rows?.items.length ? (
                    rows.items.map((row, index) => (
                      <m.tr
                        key={row.id}
                        className={selected?.id === row.id ? "is-selected" : ""}
                        onClick={() => void openFile(row)}
                        role="button"
                        tabIndex={0}
                        onKeyDown={(event) => {
                          if (event.key === "Enter" || event.key === " ") {
                            event.preventDefault();
                            void openFile(row);
                          }
                        }}
                        whileHover={rowHover}
                        whileTap={rowPress}
                        {...stagedListItem(index)}
                      >
                        <td>
                          <div className="file-title">{row.filename}</div>
                          <div className="file-path">{row.path}</div>
                        </td>
                        <td>{row.creator ?? unknownCreatorLabel(userView)}</td>
                        <td>
                          <span className={`kind-pill kind-${kindSlug(row.kind)}`}>
                            {friendlyTypeLabel(row.kind)}
                          </span>
                        </td>
                        {tableColumns.includes("root") ? (
                          <td>
                            <span className="source-pill">{row.sourceLocation}</span>
                          </td>
                        ) : null}
                        {tableColumns.includes("depth") ? (
                          <td>{row.relativeDepth}</td>
                        ) : null}
                        <td>
                          <span
                            className={`confidence-badge ${confidenceTone(
                              row.confidence,
                            )}`}
                          >
                            {Math.round(row.confidence * 100)}%
                          </span>
                        </td>
                      </m.tr>
                    ))
                  ) : (
                    <tr>
                      <td colSpan={tableColumns.length} className="empty-row">
                        {userView === "beginner"
                          ? "Nothing matches these filters."
                          : "No indexed files match the current filters."}
                      </td>
                    </tr>
                  )}
                </tbody>
              </table>
            </div>
            <ResizableEdgeHandle
              label="Resize library table height"
              value={libraryTableHeight}
              min={260}
              max={860}
              onChange={setLibraryTableHeight}
              side="bottom"
              className="dock-resize-handle library-table-height-handle"
            />
          </div>

          <div className="table-footer">
            <button
              type="button"
              className="secondary-action"
              onClick={() => setPage((current) => Math.max(current - 1, 0))}
              disabled={page === 0}
            >
              Previous
            </button>
            <div className="table-page-label">
              Page {Math.min(page + 1, totalPages)} of {totalPages}
            </div>
            <button
              type="button"
              className="secondary-action"
              onClick={() => setPage((current) => current + 1)}
              disabled={!rows || page + 1 >= totalPages}
            >
              Next
            </button>
          </div>
        </div>

        <ResizableDetailPanel
          ariaLabel="Library inspector"
          width={libraryDetailWidth}
          onWidthChange={setLibraryDetailWidth}
          maxWidth={760}
        >
          {selected ? (
            <>
              <div className="detail-header">
                <div>
                  <p className="eyebrow">{userView === "beginner" ? "Selected file" : "Inspector"}</p>
                  <h2>{selected.filename}</h2>
                </div>
                <span className={`confidence-badge ${confidenceTone(selected.confidence)}`}>
                  {Math.round(selected.confidence * 100)}%
                </span>
              </div>

              <DockSectionStack
                layoutId="libraryInspector"
                sections={libraryInspectorSections}
                intro={
                  userView === "beginner"
                    ? "Open the parts you need, hide the rest, and move sections up or down."
                    : "Collapse, reorder, and reset inspector sections to fit your workflow."
                }
              />
            </>
          ) : (
            <div className="detail-empty">
              <p className="eyebrow">{userView === "beginner" ? "Selected file" : "Inspector"}</p>
              <h2>Select a file</h2>
            </div>
          )}
        </ResizableDetailPanel>
      </div>
    </section>
  );
}

function CreatorLearningBlock({
  userView,
  creatorDraft,
  aliasDraft,
  lockPreference,
  preferredPathDraft,
  savingCreator,
  creatorMessage,
  suggestions,
  creatorLearning,
  onCreatorDraftChange,
  onAliasDraftChange,
  onLockPreferenceChange,
  onPreferredPathChange,
  onSelectSuggestion,
  onSave,
  onOpenAudit,
}: {
  userView: UserView;
  creatorDraft: string;
  aliasDraft: string;
  lockPreference: boolean;
  preferredPathDraft: string;
  savingCreator: boolean;
  creatorMessage: string | null;
  suggestions: string[];
  creatorLearning: CreatorLearningInfo;
  onCreatorDraftChange: (value: string) => void;
  onAliasDraftChange: (value: string) => void;
  onLockPreferenceChange: (value: boolean) => void;
  onPreferredPathChange: (value: string) => void;
  onSelectSuggestion: (value: string) => void;
  onSave: () => void;
  onOpenAudit: () => void;
}) {
  const showAdvanced = userView !== "beginner";

  return (
    <div className="creator-learning-block">
      <div className="creator-learning-header">
        <div className="section-label">
          {userView === "beginner" ? "Set creator" : "Creator learning"}
        </div>
        {creatorLearning.lockedByUser ? (
          <span className="ghost-chip">
            {userView === "beginner" ? "saved folder" : "locked route"}
          </span>
        ) : null}
      </div>

      <div className="learning-intro">
        <strong>
          {userView === "beginner"
            ? "Use this when the creator is wrong or missing."
            : "Save the creator for this file."}
        </strong>
        <span>
          {userView === "beginner"
            ? "Saving here helps future scans recognize this creator again."
            : "This saves a creator name or extra clue and reuses it in later scans."}
        </span>
      </div>

      {suggestions.length ? (
        <div className="creator-suggestion-strip">
          {suggestions.map((value) => (
            <button
              key={value}
              type="button"
              className={`creator-suggestion ${creatorDraft === value ? "is-active" : ""}`}
              onClick={() => onSelectSuggestion(value)}
              title={`Use ${value}`}
            >
              {value}
            </button>
          ))}
        </div>
      ) : null}

      <div className={`creator-learning-grid ${showAdvanced ? "" : "is-compact"}`}>
        <label className="field">
          <span>Creator</span>
          <input
            value={creatorDraft}
            onChange={(event) => onCreatorDraftChange(event.target.value)}
            placeholder={userView === "beginner" ? "Creator name" : "Creator"}
          />
        </label>

        {showAdvanced ? (
          <label className="field">
            <span>Also save this clue</span>
            <input
              value={aliasDraft}
              onChange={(event) => onAliasDraftChange(event.target.value)}
              placeholder="[creator] / file tag"
            />
          </label>
        ) : null}
      </div>

      {showAdvanced ? (
        <>
          <label className="creator-toggle">
            <input
              type="checkbox"
              checked={lockPreference}
              onChange={(event) => onLockPreferenceChange(event.target.checked)}
            />
            <span>Lock this creator to one preview folder</span>
          </label>

          {lockPreference ? (
            <label className="field">
              <span>Preferred path</span>
              <input
                value={preferredPathDraft}
                onChange={(event) => onPreferredPathChange(event.target.value)}
                placeholder="Relative to Mods or Tray root"
              />
            </label>
          ) : null}
        </>
      ) : null}

      {creatorLearning.learnedAliases.length ? (
        <div className="creator-known-strip">
          {creatorLearning.learnedAliases.map((alias) => (
            <span key={alias} className="ghost-chip">
              {alias}
            </span>
          ))}
        </div>
      ) : null}

      <div className="creator-learning-actions">
        <button
          type="button"
          className="primary-action"
          disabled={!creatorDraft.trim() || savingCreator}
          onClick={onSave}
        >
          {savingCreator
            ? "Saving..."
            : userView === "beginner"
              ? "Save creator"
              : "Save creator"}
        </button>
        <button type="button" className="secondary-action" onClick={onOpenAudit}>
          {userView === "beginner" ? "Fix many creators" : "Open creator batches"}
        </button>
        {creatorMessage ? <span className="creator-learning-message">{creatorMessage}</span> : null}
      </div>
    </div>
  );
}

function CategoryOverrideBlock({
  userView,
  kindOptions,
  categoryKindDraft,
  categorySubtypeDraft,
  categoryMessage,
  savingCategory,
  categoryOverride,
  onKindChange,
  onSubtypeChange,
  onSave,
  onOpenAudit,
}: {
  userView: UserView;
  kindOptions: string[];
  categoryKindDraft: string;
  categorySubtypeDraft: string;
  categoryMessage: string | null;
  savingCategory: boolean;
  categoryOverride: CategoryOverrideInfo;
  onKindChange: (value: string) => void;
  onSubtypeChange: (value: string) => void;
  onSave: () => void;
  onOpenAudit: () => void;
}) {
  const showSubtype = userView !== "beginner";

  return (
    <div className="category-override-block">
      <div className="creator-learning-header">
        <div className="section-label">
          {userView === "beginner" ? "Set type" : "Type override"}
        </div>
        {categoryOverride.savedByUser ? (
          <span className="ghost-chip">
            {userView === "beginner" ? "saved" : "manual"}
          </span>
        ) : null}
      </div>

      <div className="learning-intro">
        <strong>
          {userView === "beginner"
            ? "Use this when the type looks wrong."
            : "Set the type and subtype when the automatic guess is off."}
        </strong>
        <span>
          {userView === "beginner"
            ? "Saving here changes the label SimSuite uses later. It does not move the file yet."
            : "This changes the library label and future guesses. Moves still happen only from Organize."}
        </span>
      </div>

      <div className={`creator-learning-grid ${showSubtype ? "" : "is-compact"}`}>
        <label className="field">
          <span>Type</span>
          <select
            value={categoryKindDraft}
            onChange={(event) => onKindChange(event.target.value)}
          >
            <option value="">{userView === "beginner" ? "Choose a type" : "Select type"}</option>
            {kindOptions.map((item) => (
              <option key={item} value={item}>
                {friendlyTypeLabel(item)}
              </option>
            ))}
          </select>
        </label>

        {showSubtype ? (
          <label className="field">
            <span>Subtype</span>
            <input
              value={categorySubtypeDraft}
              onChange={(event) => onSubtypeChange(event.target.value)}
              placeholder="Optional subtype"
            />
          </label>
        ) : null}
      </div>

      <div className="creator-learning-actions">
        <button
          type="button"
          className="primary-action"
          disabled={!categoryKindDraft.trim() || savingCategory}
          onClick={onSave}
        >
          {savingCategory
            ? "Saving..."
            : userView === "beginner"
              ? "Save type"
              : "Save type"}
        </button>
        <button type="button" className="secondary-action" onClick={onOpenAudit}>
          {userView === "beginner" ? "Fix many types" : "Open type batches"}
        </button>
        {categoryMessage ? <span className="creator-learning-message">{categoryMessage}</span> : null}
      </div>
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

function formatInstalledVersionValue(value: string | null) {
  return value?.trim() ? value : "Not clear yet";
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

function watchStatusLabel(watchResult: WatchResult | null, userView: UserView) {
  if (!watchResult) {
    return userView === "beginner" ? "Not watched yet" : "Not watched";
  }

  switch (watchResult.status) {
    case "exact_update_available":
      return userView === "beginner"
        ? "Confirmed update found"
        : "Exact update available";
    case "possible_update":
      return userView === "beginner"
        ? "Possible update spotted"
        : "Possible update";
    case "current":
      return userView === "beginner" ? "Looks current" : "Looks current";
    case "not_watched":
      return userView === "beginner" ? "Not watched yet" : "Not watched";
    default:
      return userView === "beginner"
        ? "Watch result is still unclear"
        : "Unknown";
  }
}

function watchSourceKindLabel(watchResult: WatchResult | null) {
  if (!watchResult?.sourceKind) {
    return "Not set";
  }

  switch (watchResult.sourceKind) {
    case "exact_page":
      return "Exact mod page";
    case "creator_page":
      return "Creator page";
    default:
      return "Saved source";
  }
}

function formatBytes(size: number) {
  if (size < 1024) {
    return `${size} B`;
  }

  if (size < 1024 * 1024) {
    return `${(size / 1024).toFixed(1)} KB`;
  }

  return `${(size / (1024 * 1024)).toFixed(1)} MB`;
}

function watchActionError(error: unknown, action: string) {
  if (error instanceof Error && error.message.trim()) {
    return error.message;
  }
  if (typeof error === "string" && error.trim()) {
    return error;
  }
  return `SimSuite could not ${action}.`;
}

function kindSlug(kind: string) {
  return kind.toLowerCase().replace(/[^a-z0-9]+/g, "-");
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
