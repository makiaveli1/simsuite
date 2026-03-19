import { useDeferredValue, useEffect, useState } from "react";
import { AnimatePresence, m } from "motion/react";
import { ExternalLink } from "lucide-react";
import { DockSectionStack } from "../components/DockSectionStack";
import { Workbench } from "../components/layout/Workbench";
import { WorkbenchRail } from "../components/layout/WorkbenchRail";
import { WorkbenchStage } from "../components/layout/WorkbenchStage";
import { WorkbenchInspector } from "../components/layout/WorkbenchInspector";
import { useUiPreferences } from "../components/UiPreferencesContext";
import { api } from "../lib/api";
import {
  downloadsSheetTransition,
  rowHover,
  rowPress,
  stagedListItem,
} from "../lib/motion";
import {
  friendlyTypeLabel,
  unknownCreatorLabel,
} from "../lib/uiLanguage";
import type {
  CategoryOverrideInfo,
  CreatorLearningInfo,
  FileDetail,
  InstalledVersionSummary,
  LibraryFacets,
  LibraryFileRow,
  LibraryLayoutPreset,
  LibraryListResponse,
  Screen,
  UserView,
  VersionConfidence,
  WatchListFilter,
} from "../lib/types";
import { libraryViewFlags } from "./library/libraryDisplay";
import {
  LibraryFilterRail,
  type LibraryFilterValues,
} from "./library/LibraryFilterRail";
import { LibraryTopStrip } from "./library/LibraryTopStrip";

interface LibraryScreenProps {
  refreshVersion: number;
  onNavigate: (screen: Screen) => void;
  onNavigateWithParams?: (
    screen: Screen,
    mode?: "tracked" | "setup" | "review",
    filter?: WatchListFilter,
    fileId?: number,
  ) => void;
  userView: UserView;
}

const PAGE_SIZE = 100;
export function LibraryScreen({
  refreshVersion,
  onNavigate,
  onNavigateWithParams,
  userView,
}: LibraryScreenProps) {
  const {
    libraryFiltersCollapsed,
    setLibraryFiltersCollapsed,
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
  const [libraryRailWidth, setLibraryRailWidth] = useState(292);
  const [moreFiltersOpen, setMoreFiltersOpen] = useState(false);
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

  function resetFilters() {
    setSearch("");
    setKind("");
    setSubtype("");
    setCreator("");
    setSource("");
    setMinConfidence("");
    setPage(0);
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
  const isPowerView = userView === "power";
  const hasInspectionSignals = Boolean(
    selected &&
      (selected.insights.format ||
        selected.insights.creatorHints.length ||
        selected.insights.familyHints.length ||
        selected.insights.versionHints.length ||
        selected.insights.versionSignals.length ||
        selected.insights.embeddedNames.length ||
        selected.insights.scriptNamespaces.length ||
        selected.insights.resourceSummary.length)
  );
  
  const playerFacingNames = selected ? collectPlayerFacingNames(selected) : [];
  const showSafetySection = Boolean(
    selected &&
      (isPowerView ||
        selected.bundleName ||
        selected.safetyNotes.length ||
        selected.parserWarnings.length),
  );
  
  const hasVersionWatchInfo = Boolean(selected?.installedVersionSummary);
  const updatesTarget = selected ? getUpdatesWorkspaceTarget(selected) : null;
  const activeFilterCount = [
    search.trim(),
    kind,
    subtype,
    creator,
    source,
    minConfidence,
  ].filter(Boolean).length;
  const selectedTypeLabel = selected
    ? [friendlyTypeLabel(selected.kind), selected.subtype?.trim()]
        .filter((value): value is string => Boolean(value))
        .join(" / ")
    : null;
  const stageSelectionTags = selected
    ? [
        selectedTypeLabel,
        selected.creator ?? unknownCreatorLabel(userView),
        selected.sourceLocation === "tray" ? "Tray root" : "Mods root",
        selected.installedVersionSummary ? "Update watch ready" : null,
      ].filter((value): value is string => Boolean(value))
    : [];
  const viewFlags = libraryViewFlags(userView);

  const libraryInspectorSections = selected
    ? [
        {
          id: "facts",
          label:
            userView === "beginner"
              ? "What matters"
              : isPowerView
                ? "File details"
                : "Overview",
          hint:
            isPowerView
              ? "Core classification, file metadata, and confidence."
              : "Only the details most simmers usually care about.",
          children: isPowerView ? (
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
                  "Unspecified"
                }
              />
              <DetailRow label="Format" value={formatLibraryFileFormat(selected)} />
              <DetailRow label="Root" value={selected.sourceLocation} />
              <DetailRow label="Depth" value={`${selected.relativeDepth}`} />
              <DetailRow label="Size" value={formatBytes(selected.size)} />
              <DetailRow
                label="Modified"
                value={
                  selected.modifiedAt
                    ? new Date(selected.modifiedAt).toLocaleString()
                    : "Unknown"
                }
              />
              <DetailRow label="Hash" value={selected.hash ?? "Not available"} mono />
            </div>
          ) : (
            <>
              <div className="detail-list">
                <DetailRow
                  label="Creator"
                  value={selected.creator ?? unknownCreatorLabel(userView)}
                />
                <DetailRow
                  label="Type"
                  value={friendlyTypeLabel(selected.kind)}
                />
                {selected.subtype?.trim() ? (
                  <DetailRow label="Subtype" value={selected.subtype} />
                ) : null}
                <DetailRow label="File format" value={formatLibraryFileFormat(selected)} />
              </div>
              {playerFacingNames.length ? (
                <div className="detail-block">
                  <div className="section-label">Found in game as</div>
                  {playerFacingNames.length === 1 ? (
                    <p>{playerFacingNames[0]}</p>
                  ) : (
                    <div className="tag-list">
                      {playerFacingNames.map((item) => (
                        <span key={item} className="ghost-chip">
                          {item}
                        </span>
                      ))}
                    </div>
                  )}
                </div>
              ) : null}
            </>
          ),
        },
        ...(hasVersionWatchInfo
          ? [
              {
                id: "updatesHint",
                label: "Updates",
                hint: "Version tracking and updates have moved to the Updates workspace.",
                children: (
                  <div className="detail-block">
                    <p>
                      <strong>{selected.installedVersionSummary?.version ?? "Version unknown"}</strong>
                    </p>
                    <p className="text-muted">
                      Check the Updates tab to manage tracking, edit sources, and review changes for this file.
                    </p>
                    {onNavigateWithParams && updatesTarget ? (
                      <button
                        type="button"
                        className="secondary-action"
                        onClick={() =>
                          onNavigateWithParams(
                            "updates",
                            updatesTarget.mode,
                            updatesTarget.filter,
                            selected.id,
                          )
                        }
                        style={{ marginTop: '0.5rem' }}
                      >
                        <ExternalLink size={12} strokeWidth={2} />
                        Open in Updates
                      </button>
                    ) : null}
                  </div>
                ),
              },
            ]
          : []),
        ...(showSafetySection
          ? [
              {
                id: "safety",
                label:
                  userView === "beginner"
                    ? "Keep together"
                    : isPowerView
                      ? "Bundle and warnings"
                      : "Care and bundle",
                hint: isPowerView
                  ? "Bundle grouping, warnings, and parser notes."
                  : "Shows grouped files and any warning notes that matter for normal play.",
                badge:
                  selected.safetyNotes.length > 0
                    ? `${selected.safetyNotes.length} warning${selected.safetyNotes.length === 1 ? "" : "s"}`
                    : selected.bundleName
                      ? "bundled"
                      : null,
                children: (
                  <>
                    {selected.bundleName ? (
                      <div className="detail-block">
                        <div className="section-label">
                          {userView === "beginner" ? "Keep with" : "Bundle"}
                        </div>
                        <p>
                          {selected.bundleName}
                          {isPowerView && selected.bundleType
                            ? ` (${selected.bundleType})`
                            : ""}
                        </p>
                      </div>
                    ) : null}

                    {selected.safetyNotes.length || isPowerView ? (
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
                    ) : null}

                    {isPowerView ? (
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
            ]
          : []),
        ...(isPowerView && hasInspectionSignals
          ? [
              {
                id: "inspection",
                label: "Inside the file",
                hint: "Signals pulled from package or script contents.",
                defaultCollapsed: false,
                badge: selected.insights.format ?? null,
                children: (
                  <>
                    {selected.insights.format ? (
                      <DetailRow label="Format" value={selected.insights.format} />
                    ) : null}
                    {selected.insights.creatorHints.length ? (
                      <div className="detail-block">
                        <div className="section-label">Creator names found</div>
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
                        <div className="section-label">Version numbers found</div>
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
                        <div className="section-label">Package contents</div>
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
                        <div className="section-label">Script folders</div>
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
                        <div className="section-label">In-game names</div>
                        <div className="tag-list">
                          {selected.insights.embeddedNames.map((item) => (
                            <span key={item} className="ghost-chip">
                              {item}
                            </span>
                          ))}
                        </div>
                      </div>
                    ) : null}
                    {selected.insights.familyHints.length ? (
                      <div className="detail-block">
                        <div className="section-label">Family hints</div>
                        <div className="tag-list">
                          {selected.insights.familyHints.map((item) => (
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
        ...(isPowerView
          ? [
              {
                id: "creator",
                label: "Creator learning",
                hint: "Save creator matches and optional folder preferences.",
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
                label: "Type override",
                hint: "Override type and subtype for later scans and previews.",
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
                label: "Path",
                hint: "Absolute path for this indexed file.",
                defaultCollapsed: true,
                children: <div className="path-card">{selected.path}</div>,
              },
            ]
          : []),
      ]
    : [];

  return (
    <Workbench threePanel fullHeight>
      {/* Left rail for filters */}
      <WorkbenchRail 
        className="library-rail-shell"
        resizable 
        width={libraryFiltersCollapsed ? 0 : libraryRailWidth}
        onWidthChange={(width) => {
          if (width === 0) {
            setLibraryFiltersCollapsed(true);
          } else {
            setLibraryFiltersCollapsed(false);
            setLibraryRailWidth(width);
          }
        }}
        minWidth={248}
        maxWidth={360}
        noBorder
      >
        {/* Filter panel content */}
        <LibraryFilterRail
          userView={userView}
          facets={facets}
          filters={{
            kind,
            creator,
            source,
            subtype,
            minConfidence,
          }}
          activeFilterCount={activeFilterCount}
          resultsCount={rows?.items.length ?? 0}
          isCollapsed={libraryFiltersCollapsed}
          onToggleCollapsed={() => setLibraryFiltersCollapsed(true)}
          onFilterChange={(next: Partial<LibraryFilterValues>) => {
            if (typeof next.kind === "string") {
              setKind(next.kind);
            }
            if (typeof next.creator === "string") {
              setCreator(next.creator);
            }
            if (typeof next.source === "string") {
              setSource(next.source);
            }
            if (typeof next.subtype === "string") {
              setSubtype(next.subtype);
            }
            if (typeof next.minConfidence === "string") {
              setMinConfidence(next.minConfidence);
            }
            setPage(0);
          }}
          onReset={resetFilters}
          onOpenMoreFilters={() => setMoreFiltersOpen((current) => !current)}
        />
      </WorkbenchRail>

      {/* Central work area - table */}
      <WorkbenchStage className="library-stage-shell">
        <LibraryTopStrip
          userView={userView}
          search={search}
          shownCount={rows?.items.length ?? 0}
          totalCount={rows?.total ?? 0}
          activeFilterCount={activeFilterCount}
          filtersCollapsed={libraryFiltersCollapsed}
          moreFiltersOpen={moreFiltersOpen}
          onSearchChange={(value) => {
            setSearch(value);
            setPage(0);
          }}
          onToggleFiltersRail={() => setLibraryFiltersCollapsed(false)}
          onToggleMoreFilters={() => setMoreFiltersOpen((current) => !current)}
        />

        <AnimatePresence>
          {viewFlags.showAdvancedFilters && moreFiltersOpen ? (
            <m.div
              className="library-more-filters-popover"
              initial={{ opacity: 0, y: -8, scale: 0.985 }}
              animate={{ opacity: 1, y: 0, scale: 1 }}
              exit={{ opacity: 0, y: -6, scale: 0.985 }}
              transition={downloadsSheetTransition}
            >
              <div className="library-more-filters-header">
                <div>
                  <p className="eyebrow">{userView === "power" ? "Inspect filters" : "More filters"}</p>
                  <strong>
                    {userView === "power"
                      ? "Keep the deeper clues one click away"
                      : "Use the extra narrowing only when you need it"}
                  </strong>
                </div>
              </div>

              <div className="filter-grid library-more-filters-grid">
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
                      <option value="">All subtypes</option>
                      {facets?.subtypes.map((item) => (
                        <option key={item} value={item}>
                          {item}
                        </option>
                      ))}
                    </select>
                  </label>
                ) : null}

                <label className="field">
                  <span>Confidence</span>
                  <select
                    value={minConfidence}
                    onChange={(event) => {
                      setMinConfidence(event.target.value);
                      setPage(0);
                    }}
                  >
                    <option value="">Any match</option>
                    <option value="0.35">35%+</option>
                    <option value="0.55">55%+</option>
                    <option value="0.75">75%+</option>
                  </select>
                </label>
              </div>
            </m.div>
          ) : null}
        </AnimatePresence>

        {/* Table content */}
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

        {/* Table footer with pagination */}
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
      </WorkbenchStage>

      {/* Right inspector panel */}
      <WorkbenchInspector className="library-inspector-shell">
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
      </WorkbenchInspector>
    </Workbench>
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

function formatLibraryFileFormat(detail: FileDetail) {
  if (detail.insights.format === "ts4script-zip" || detail.extension === ".ts4script") {
    return "Script archive (.ts4script)";
  }
  if (detail.insights.format === "dbpf-package" || detail.extension === ".package") {
    return "Package file (.package)";
  }
  return detail.insights.format ?? detail.extension ?? "Unknown";
}

function collectPlayerFacingNames(detail: FileDetail) {
  const normalizedFilename = normalizeForNameComparison(detail.filename);

  return detail.insights.embeddedNames
    .map((value) => value.trim())
    .filter((value) => {
      if (!value || value.length < 3 || value.length > 72) {
        return false;
      }
      if (/^0x[0-9a-f]+$/i.test(value) || /[\\/]/.test(value)) {
        return false;
      }

      const normalizedValue = normalizeForNameComparison(value);
      if (!normalizedValue || normalizedValue === normalizedFilename) {
        return false;
      }

      const letters = value.match(/[a-z]/gi)?.length ?? 0;
      return letters >= 3;
    })
    .filter((value, index, values) =>
      values.findIndex((candidate) => candidate.toLowerCase() === value.toLowerCase()) === index,
    )
    .slice(0, 4);
}

function normalizeForNameComparison(value: string) {
  return value
    .replace(/\.[^.]+$/, "")
    .replace(/[_\-\s]+/g, "")
    .toLowerCase()
    .trim();
}

function formatPlayerInstalledVersion(summary: InstalledVersionSummary | null) {
  if (!summary?.version?.trim()) {
    return "Not confirmed yet";
  }

  return hasConfirmedInstalledVersion(summary)
    ? summary.version
    : "Not confirmed yet";
}

function hasConfirmedInstalledVersion(summary: InstalledVersionSummary | null) {
  return Boolean(
    summary?.version?.trim() &&
      (summary.confidence === "exact" || summary.confidence === "strong"),
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

function getUpdatesWorkspaceTarget(file: FileDetail): {
  mode: "tracked" | "setup" | "review";
  filter?: WatchListFilter;
} {
  const watchResult = file.watchResult;

  if (!watchResult?.sourceKind) {
    return { mode: "setup", filter: "all" };
  }

  switch (watchResult.status) {
    case "exact_update_available":
      return { mode: "tracked", filter: "exact_updates" };
    case "possible_update":
      return { mode: "tracked", filter: "possible_updates" };
    case "current":
    case "not_watched":
      if (watchResult.capability === "provider_required" || !watchResult.canRefreshNow) {
        return { mode: "review" };
      }
      return { mode: "tracked", filter: "all" };
    default:
      return { mode: "review" };
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
