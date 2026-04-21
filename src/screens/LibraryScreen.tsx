import {
  useDeferredValue,
  useEffect,
  useMemo,
  useRef,
  useState,
  type ReactNode,
} from "react";
import { SkeletonLoader } from "../components/SkeletonLoader";
import { ExternalLink, Search, X, PanelLeftClose } from "lucide-react";
import { DockSectionStack } from "../components/DockSectionStack";
import { Workbench } from "../components/layout/Workbench";
import { WorkbenchStage } from "../components/layout/WorkbenchStage";
import { WorkbenchInspector } from "../components/layout/WorkbenchInspector";
import { api } from "../lib/api";
import {
  creatorConfidenceSuffix,
  friendlyTypeLabel,
} from "../lib/uiLanguage";
import type {
  CategoryOverrideInfo,
  CreatorLearningInfo,
  FileDetail,
  InstalledVersionSummary,
  LibraryFacets,
  LibraryFileRow,
  LibraryListResponse,
  Screen,
  UserView,
  VersionConfidence,
  WatchListFilter,
  LibrarySummary,
  LibrarySortField,
} from "../lib/types";
import {
  buildSheetAttributionSection,
  buildSheetCompatibilitySection,
  buildSheetContentsSection,
  buildSheetDiagnosticsSection,
  buildSheetTraySection,
  computeFileRelationship,
  describeCreatorForInspector,
  describeLibraryFamilyContext,
  describeTrayIdentity,
  describeVersionForInspector,
  extractParentFolder,
  formatLibraryFileFormat,
  groupedFilesLabel,
  trayLocationLabel,
  usefulTrayGroupingValue,
} from "./library/libraryDisplay";
import { LibraryCollectionTable } from "./library/LibraryCollectionTable";
import { LibraryThumbnailGrid } from "./library/LibraryThumbnailGrid";
import { LibraryDetailSheet, type LibrarySheetMode } from "./library/LibraryDetailSheet";
import { LibraryDetailsPanel } from "./library/LibraryDetailsPanel";
import { ErrorBoundary } from "../components/ErrorBoundary";
import { LibraryTopStrip } from "./library/LibraryTopStrip";
import { FolderTreePane } from "./library/FolderTreePane";
import { FolderContentPane } from "./library/FolderContentPane";
import { buildFolderTree, getFolderContents, getRootFiles, type FolderNode } from "./library/folderTree";
import type { FolderTreeMetadata, FolderTreeNode } from "../lib/types";

interface LibraryScreenProps {
  refreshVersion: number;
  onNavigate: (screen: Screen) => void;
  /** Navigate to Updates with optional context. */
  onNavigateWithParams?: (
    screen: Screen,
    mode?: "tracked" | "setup" | "review",
    filter?: WatchListFilter,
    fileId?: number,
    fileIds?: number[],
  ) => void;
  /** Navigate to Duplicates screen with the given file IDs pre-scoped. */
  onNavigateDuplicates?: (fileIds: number[]) => void;
  userView: UserView;
}

// ── Folder tree helpers ────────────────────────────────────────────────────
// Mirrors the path normalization logic from folderTree.ts so we can
// identify depth-0 root files without importing the tree builder.

const SOURCE_ROOT_NAMES = new Set(["Mods", "Tray"]);

function getRelativePath(path: string): string {
  return path.replace(/\\/g, "/").replace(/^[a-zA-Z]:/, "").replace(/^\/+/, "");
}

// Returns the source root name ("Mods" | "Tray") for a file path, or null.
function getSourceRoot(path: string): string | null {
  const segments = getRelativePath(path).split("/").filter(Boolean);
  const idx = segments.findIndex((s) => SOURCE_ROOT_NAMES.has(s));
  return idx < 0 ? null : segments[idx];
}

// Returns true if this file is stored directly in the root folder with no subfolder.
// E.g. Mods\filename.package → true  (depth-0, no subfolder)
// DEPTH-0 vs DEPTH-1 determination using relativeDepth from the database.
// relativeDepth=0: file is directly inside the source root (Mods or Tray) with no subfolder.
// relativeDepth=1+: file is inside a subfolder of the source root.
// The path-based approach (findRootSegments) fails on Windows because the stored path
// includes the full user folder path (e.g. C:/Users/likwi/.../Mods/File.package)
// which causes slice(1) to remove the WRONG segment (the username, not the game root).
function isDepthZeroFile(file: { path: string; relativeDepth: number }, sourceRoot: string): boolean {
  return file.relativeDepth === 0;
}

const DEFAULT_PAGE_SIZE = 100;
export function LibraryScreen({
  refreshVersion,
  onNavigate,
  onNavigateWithParams,
  onNavigateDuplicates,
  userView,
}: LibraryScreenProps) {
  const [facets, setFacets] = useState<LibraryFacets | null>(null);
  const [rows, setRows] = useState<LibraryListResponse | null>(null);
  const [selected, setSelected] = useState<FileDetail | null>(null);
  const [search, setSearch] = useState("");
  const [kind, setKind] = useState("");
  const [subtype, setSubtype] = useState("");
  const [creator, setCreator] = useState("");
  const [source, setSource] = useState("");
  const [minConfidence, setMinConfidence] = useState("");
  const [watchFilter, setWatchFilter] = useState<"all" | "has_updates" | "needs_attention" | "not_tracked" | "duplicates">("all");
  const [sortBy, setSortBy] = useState<LibrarySortField>("name");
  const [librarySummary, setLibrarySummary] = useState<LibrarySummary | null>(null);
  const [selectedIds, setSelectedIds] = useState<Set<number>>(new Set());
  const [page, setPage] = useState(0);
  const [pageSize, setPageSize] = useState(DEFAULT_PAGE_SIZE);
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
  const [moreFiltersOpen, setMoreFiltersOpen] = useState(false);
  const [activeLibrarySheet, setActiveLibrarySheet] = useState<LibrarySheetMode>(null);
  const [inspectorCollapsed, setInspectorCollapsed] = useState(false);
  const [viewMode, setViewMode] = useState<"list" | "grid" | "folders">("list");
  const [activeFolderPath, setActiveFolderPath] = useState<string | null>(null);
  const [treeRows, setTreeRows] = useState<LibraryListResponse | null>(null);
  // Loading state for the tree fetch — separate from the paged rows loading.
  // Shows a subtle spinner in the tree pane while treeRows is being fetched.
  const [isTreeLoading, setIsTreeLoading] = useState(false);
  const [densityValue, setDensityValue] = useState(50);
  const deferredSearch = useDeferredValue(search);

  // Phase 5ac: Track the filter params the current treeRows was loaded with.
  // If filters haven't changed, we skip the reload — tree persists across folder exits.
  const lastTreeFiltersRef = useRef<{
    search: string;
    kind: string;
    subtype: string;
    creator: string;
    source: string;
    minConfidence: string;
    watchFilter: string;
  } | null>(null);

  // Phase 5ac: Instantly-accessible previous tree — used to render the tree
  // immediately while a new load is in progress (warm reload), avoiding flicker.
  const prevTreeCacheRef = useRef<{
    mods: FolderNode;
    tray: FolderNode;
  } | null>(null);

  // Phase 5ac: Holds the precomputed folder tree from Rust (lightweight metadata, no file rows).
  // Separate from treeRows (file content) — this is ONLY for tree structure rendering.
  const treeMetaRef = useRef<FolderTreeMetadata | null>(null);

  // Phase 5ac: Renderable tree structure — derived from treeMetaRef.
  // Separated from treeRows so tree rendering does NOT depend on loading flat file rows.
  const [folderTree, setFolderTree] = useState<{
    mods: FolderNode;
    tray: FolderNode;
  } | null>(null);

  // Phase 5ac: Persist expanded folder state across view switches so the tree
  // does NOT collapse when switching list↔folders. Uses path-keyed Set for O(1) lookup.
  const [expandedPaths, setExpandedPaths] = useState<Set<string>>(new Set(["Mods"]));

  // Phase 5ac: Monotonic sequence counter for the lightweight tree metadata API.
  // Follows the same proven pattern as loadSeqRef for list queries.
  const treeSeqRef = useRef(0);

  // Phase 5ac: Background preloader — warm the tree cache while in list/grid mode.
  // Runs silently (no spinner, no UI disruption) so the tree is ready before user enters folder mode.
  // Only preloads if filters differ from last loaded tree.
  useEffect(() => {
    if (viewMode === "folders") return;
    const filters = {
      search: deferredSearch,
      kind: kind ?? "",
      subtype: subtype ?? "",
      creator: creator ?? "",
      source: source ?? "",
      minConfidence: minConfidence ?? "",
      watchFilter: watchFilter ?? "",
    };
    const last = lastTreeFiltersRef.current;
    const filtersUnchanged =
      last !== null &&
      last.search === filters.search &&
      last.kind === filters.kind &&
      last.subtype === filters.subtype &&
      last.creator === filters.creator &&
      last.source === filters.source &&
      last.minConfidence === filters.minConfidence &&
      last.watchFilter === filters.watchFilter;
    if (filtersUnchanged) return; // Tree already loaded for these filters

    // Phase 5ac: Lightweight metadata API — returns prebuilt tree structure, NOT flat file rows.
    // No need to also load listLibraryFilesForTree here; treeRows is for folder contents only.
    const seq = ++treeSeqRef.current;
    const treeFilters = {
      search: deferredSearch || undefined,
      kind: kind || undefined,
      subtype: subtype || undefined,
      creator: creator || undefined,
      source: source || undefined,
      minConfidence: minConfidence ? Number(minConfidence) : undefined,
      watchFilter: watchFilter || undefined,
    };
    Promise.all([
      api.getFolderTreeMetadata(treeFilters),
      // Phase 5ah: ALSO prefetch full file list for folder contents.
      // treeRows (all 13k files) is needed for getFolderContents — pre-warming
      // it here makes folder mode feel INSTANT when the user first enters.
      api.listLibraryFilesForTree(treeFilters),
    ])
      .then(([meta, treeResult]) => {
        if (seq !== treeSeqRef.current) return; // discard stale
        treeMetaRef.current = meta;
        const tree = convertMetaToFolderTree(meta);
        setFolderTree(tree);
        prevTreeCacheRef.current = tree;
        // Phase 5ah: warm treeRows so folder content is ready instantly
        setTreeRows(treeResult);
        // Phase 5ah: also persist filter state so the loadTreeRows effect
        // sees filtersUnchanged=true and skips the redundant cold fetch.
        lastTreeFiltersRef.current = filters;
      })
      .catch(() => {
        // Silently fail — the tree will load on-demand when user enters folder mode
      })
      .finally(() => {
        // Phase 5ah: mark filters as loaded so loadTreeRows effect skips redundant rebuild.
        // We must set this here (not in .then) so it also runs if the Promise resolves
        // after the load effect has already checked lastTreeFiltersRef.current.
        lastTreeFiltersRef.current = filters;
      });
  }, [viewMode, deferredSearch, kind, subtype, creator, source, minConfidence, watchFilter]);

  useEffect(() => {
    const clampedDensity = Math.max(0, Math.min(100, densityValue));
    const rootStyle = document.documentElement.style;
    rootStyle.setProperty("--card-density", (clampedDensity / 100).toString());
    rootStyle.setProperty("--slider-fill", `${clampedDensity}%`);

    return () => {
      rootStyle.removeProperty("--card-density");
      rootStyle.removeProperty("--slider-fill");
    };
  }, [densityValue]);

  // Reload facets when kind changes so subtype chips are kind-scoped.
  useEffect(() => {
    void api.getLibraryFacets(kind || undefined).then(setFacets);
  }, [kind]);

  // Monotonic sequence counter — discards stale async responses from overlapping filter changes.
  const loadSeqRef = useRef(0);

  useEffect(() => {
    void api.getLibraryFacets(kind || undefined).then(setFacets);
    void api.getLibrarySummary().then(setLibrarySummary).catch(() => {
      // If getLibrarySummary fails (e.g. not yet implemented), fall through silently.
    });
  }, [refreshVersion]);

  // Close the detail sheet whenever a different file is selected so stale sections don't show.
  useEffect(() => {
    setActiveLibrarySheet(null);
  }, [selected?.id]);

  // Clear selection when page changes to prevent phantom selections.
  useEffect(() => {
    setSelectedIds(new Set());
  }, [page]);

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
    watchFilter,
    sortBy,
    page,
    pageSize,
  ]);

  // Load the full-tree dataset when entering folder mode or when filters change.
  // The tree needs ALL files (no pagination) to show real subfolder structure.
  // Phase 5ab: viewMode removed from deps — switching list↔grid must not invalidate treeRows.
  // Phase 5ac: lastTreeFiltersRef prevents redundant reloads when entering folder mode
  // with the same filters the tree was already loaded for.
  // Phase 5ah FIX: setActiveFolderPath(null) and setSelected(null) moved to a separate
  // "enter folder mode" effect below — they must NOT run on every search/filter change
  // (which was causing blank-screen: every keystroke fired setSelected(null), wiping
  // the sidebar preview mid-fetch whenever the user typed with a file selected).
  useEffect(() => {
    if (viewMode !== "folders") return;

    // Phase 5ac: Skip if tree is already loaded for the current filter set.
    // The background preloader keeps treeRows warm, so this is usually a no-op.
    const filters = {
      search: deferredSearch,
      kind: kind ?? "",
      subtype: subtype ?? "",
      creator: creator ?? "",
      source: source ?? "",
      minConfidence: minConfidence ?? "",
      watchFilter: watchFilter ?? "",
    };
    const last = lastTreeFiltersRef.current;
    const filtersUnchanged =
      last !== null &&
      last.search === filters.search &&
      last.kind === filters.kind &&
      last.subtype === filters.subtype &&
      last.creator === filters.creator &&
      last.source === filters.source &&
      last.minConfidence === filters.minConfidence &&
      last.watchFilter === filters.watchFilter;
    // Phase 5ac: Skip if tree is already loaded for current filter set.
    // folderTree state holds the rendered tree; treeRows holds file content.
    // Both must be checked to ensure both are available.
    if (filtersUnchanged && folderTree !== null && treeRows !== null) {
      return;
    }
    void loadTreeRows(filters);
    // NOTE: deferredSearch intentionally OMITTED from deps — adding it would cause
    // setSelected(null) to fire on every debounced-search change, blanking the sidebar.
    // The background preloader (above) handles search warm-up independently.
  }, [viewMode, kind, subtype, creator, source, minConfidence, watchFilter]);

  // Phase 5ah: Enter folder mode — runs ONCE when switching from list/grid to folders.
  // NOT on every filter/search change (that belongs to the effect above).
  useEffect(() => {
    if (viewMode !== "folders") return;
    setActiveFolderPath(null); // Reset to root when entering folder mode
    setSelected(null); // Clear stale sidebar from list/grid mode
  }, [viewMode]);

  // Phase 5ab: separate effect — reset active path when LEAVING folder mode.
  // Does NOT clear treeRows; the tree cache survives view switches.
  useEffect(() => {
    if (viewMode === "folders") return;
    setActiveFolderPath(null);
  }, [viewMode]);

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

    // Phase 5ai: creatorLearning/categoryOverride may be undefined in the preview
    // object (LibraryFileRow spread doesn't include them). Guard to avoid crashes
    // when the effect fires immediately after setSelected(preview) in openFile.
    const cl = selected.creatorLearning;
    const co = selected.categoryOverride;
    setCreatorDraft(selected.creator ?? selected.insights.creatorHints[0] ?? "");
    setAliasDraft("");
    setLockPreference(cl?.lockedByUser ?? false);
    setPreferredPathDraft(cl?.preferredPath ?? "");
    setCreatorMessage(null);
    setCategoryKindDraft(co?.kind ?? selected.kind);
    setCategorySubtypeDraft(co?.subtype ?? selected.subtype ?? "");
    setCategoryMessage(null);
  }, [selected]);

  useEffect(() => {
    if (!subtype || !facets?.subtypes?.length) {
      return;
    }
    if (!facets.subtypes.includes(subtype)) {
      setSubtype("");
      setPage(0);
    }
  }, [facets, subtype]);


  async function loadRows(preferredSelectedId?: number) {
    const seq = ++loadSeqRef.current;

    const result = await api.listLibraryFiles({
      search: deferredSearch || undefined,
      kind: kind || undefined,
      subtype: subtype || undefined,
      creator: creator || undefined,
      source: source || undefined,
      minConfidence: minConfidence ? Number(minConfidence) : undefined,
      watchFilter: watchFilter || undefined,
      sortBy: sortBy || undefined,
      limit: pageSize,
      offset: page * pageSize,
    });

    // Discard stale response — a newer request may have fired since this one started.
    if (seq !== loadSeqRef.current) return;

    setRows(result);

    // If the previously selected file is no longer in the filtered results,
    // clear selection so the inspector doesn't show a phantom file.
    const currentSelectedId = preferredSelectedId ?? selected?.id;
    const stillInResults = currentSelectedId != null && result.items.some((r) => r.id === currentSelectedId);
    const detailId = stillInResults ? currentSelectedId : result.items[0]?.id;

    if (detailId) {
      setSelected(await api.getFileDetail(detailId));
    } else {
      setSelected(null);
      setActiveLibrarySheet(null);
    }
  }

  // Loads ALL filtered files (no pagination) for folder-tree construction.
  // Phase 5ac: Loads tree metadata (NOT flat file rows) — lightweight and fast.
  async function loadTreeRows(
    filters: {
      search: string;
      kind: string;
      subtype: string;
      creator: string;
      source: string;
      minConfidence: string;
      watchFilter: string;
    } | null,
  ) {
    setIsTreeLoading(true);
    try {
      // Phase 5af: Build tree from file list directly — avoids broken Rust
      // get_folder_tree_metadata SQL (direct_file_count formula bug) and ensures
      // tree counts always match the actual file list used for content.
      const result = await api.listLibraryFilesForTree({
        search: deferredSearch || undefined,
        kind: kind || undefined,
        subtype: subtype || undefined,
        creator: creator || undefined,
        source: source || undefined,
        minConfidence: minConfidence ? Number(minConfidence) : undefined,
        watchFilter: watchFilter || undefined,
      });
      const tree = buildFolderTree(result.items);
      setTreeRows(result);
      setFolderTree(tree);
      prevTreeCacheRef.current = tree;
      if (filters) lastTreeFiltersRef.current = filters;
    } catch (err) {
      // Silently fail — the app continues with whatever data it already had cached.
      void err;
    } finally {
      setIsTreeLoading(false);
    }
  }

  // Phase 5ac: Convert Rust FolderTreeMetadata to the FolderNode[] tree format.
  // Rust already built the tree — JS just maps the fields and injects root files.
  function convertMetaToFolderTree(meta: FolderTreeMetadata): { mods: FolderNode; tray: FolderNode } {
    function nodeToFolderNode(n: FolderTreeNode): FolderNode {
      return {
        name: n.name,
        fullPath: n.path,
        depth: n.depth,
        children: n.children.map(nodeToFolderNode),
        directFileCount: n.directFileCount,
        totalFileCount: n.totalFileCount,
        childFolderCount: n.childFolderCount,
      };
    }
    const roots = meta.roots.map(nodeToFolderNode);
    const mods = roots.find((r) => r.name === "Mods") ?? {
      name: "Mods", fullPath: "Mods", depth: 0,
      children: [], directFileCount: 0, totalFileCount: 0, childFolderCount: 0,
    };
    const tray = roots.find((r) => r.name === "Tray") ?? {
      name: "Tray", fullPath: "Tray", depth: 0,
      children: [], directFileCount: 0, totalFileCount: 0, childFolderCount: 0,
    };
    return { mods, tray };
  }

  async function openFile(row: LibraryFileRow) {
    // Phase 5y: show a minimal preview IMMEDIATELY so the sidebar does not go
    // blank while getFileDetail round-trips. Only the thumbnail + id are needed
    // from the row; everything else populates when the full detail arrives.
    //
    // Phase 5ai — two crash guards:
    // 1. creatorLearning / categoryOverride are required by creator-learning and
    //    category-override blocks in LibraryDetailsPanel, but LibraryFileRow does
    //    not carry them (they come only from get_file_detail). Supply safe empty
    //    defaults here so the preview is always safe to render.
    // 2. If getFileDetail throws in Tauri, do NOT blank the sidebar — keep the
    //    preview showing rather than wiping it.
    let previewThumbnail: string | null = null;
    try {
      previewThumbnail =
        row.insights?.thumbnailPreview ??
        row.insights?.cachedThumbnailPreview ??
        null;
      const preview: FileDetail = {
        ...row,
        _cardThumbnail: previewThumbnail,
        insights: row.insights ?? {
          embeddedNames: [],
          creatorHints: [],
          familyHints: [],
          scriptNamespaces: [],
          versionHints: [],
          resourceSummary: [],
          safetyNotes: [],
          parserWarnings: [],
          thumbnailPreview: null,
          cachedThumbnailPreview: null,
        },
        // Phase 5ai — required by creator-learning and category-override blocks
        creatorLearning: {
          lockedByUser: false,
          preferredPath: null,
          learnedAliases: [],
        },
        categoryOverride: {
          savedByUser: false,
          kind: null,
          subtype: null,
        },
      } as unknown as FileDetail;
      setSelected(preview);

      const detail = await api.getFileDetail(row.id);
      if (!detail) return;
      setSelected(detail);
    } catch (err) {
      // Phase 5ai: keep the preview — don't blank the sidebar on detail fetch failure
      console.error("openFile detail fetch failed:", err);
    }
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
      await Promise.all([
        loadRows(updated.id),
        api.getLibraryFacets(kind || undefined).then(setFacets),
      ]);
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
      await Promise.all([
        loadRows(updated.id),
        api.getLibraryFacets(kind || undefined).then(setFacets),
      ]);
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
    setWatchFilter("all");
    setSortBy("name");
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
  const totalPages = rows ? Math.max(1, Math.ceil(rows.total / pageSize)) : 1;

  const isPowerView = userView === "power";
  const traySelection = selected ? describeTrayIdentity(selected) : null;
  const isTraySelection = Boolean(traySelection && traySelection.kind !== "standard");
  const hasInspectionSignals = Boolean(
    selected &&
      (isTraySelection ||
        selected.insights.format ||
        selected.insights.creatorHints.length ||
        selected.insights.familyHints.length ||
        selected.insights.versionHints.length ||
        selected.insights.versionSignals.length ||
        selected.insights.embeddedNames.length ||
        selected.insights.scriptNamespaces.length ||
        selected.insights.resourceSummary.length)
  );
  
  const playerFacingNames = selected ? collectPlayerFacingNames(selected) : [];
  const trayGroupingValue = selected ? usefulTrayGroupingValue(selected) : null;
  const trayGroupedCount = selected ? groupedFilesLabel(selected.groupedFileCount) : null;
  const showSafetySection = Boolean(
    selected &&
      (isPowerView ||
        selected.bundleName ||
        selected.safetyNotes.length ||
        selected.parserWarnings.length),
  );
  
  const hasVersionWatchInfo = Boolean(selected?.installedVersionSummary);
  const updatesTarget = selected ? getUpdatesWorkspaceTarget(selected) : null;
  const isCasualView = userView === "beginner";
  // Tracks every active narrowing dimension so the reset button and badge are honest.
  const hasActiveFilters =
    search.trim().length > 0 ||
    kind !== "" ||
    subtype !== "" ||
    creator !== "" ||
    source !== "" ||
    minConfidence !== "" ||
    watchFilter !== "all" ||
    sortBy !== "name";
  const activeFilterCount = hasActiveFilters
    ? [
        search.trim(),
        kind,
        subtype,
        creator,
        source,
        minConfidence,
        watchFilter !== "all" ? "watch" : null,
        sortBy !== "name" ? "sort" : null,
      ].filter(Boolean).length
    : 0;

  // ── Relationship + folder context (derived, memoised) ─────────────────────
  const relationship = useMemo(
    () => (selected && rows ? computeFileRelationship(selected, rows.items) : null),
    [selected, rows],
  );
  const folderName = useMemo(
    () => (selected ? extractParentFolder(selected.path) : null),
    [selected],
  );

  // ── Folder tree ──────────────────────────────────────────────────────────
  // Phase 5ac: tree rendering now uses folderTree state (set by loadTreeRows from
  // listLibraryFilesForTree), not a per-render rebuild from flat rows.
  // Falls back to prevTreeCacheRef during warm reload to avoid flicker.
  const folderTreeRoots = folderTree ?? prevTreeCacheRef.current ?? null;

  // Phase 5ag: Combined single-pass computation — produces rootItemsBySource + rootFileCount.
  // Eliminates the duplicate O(n) loop that previously existed in folderContents and syntheticRoot.
  const { rootItemsBySource, rootFileCount } = useMemo(() => {
    const items = treeRows?.items ?? rows?.items ?? [];
    const rootItemsBySource: Record<string, LibraryFileRow[]> = { Mods: [], Tray: [] };
    let rootFileCount = 0;
    for (const item of items) {
      const src = getSourceRoot(item.path);
      if ((src === "Mods" || src === "Tray") && isDepthZeroFile(item, src)) {
        rootItemsBySource[src].push(item);
        rootFileCount += 1;
      }
    }
    return { rootItemsBySource, rootFileCount };
  }, [treeRows?.items, rows?.items]);

  // ── Folder contents for the active path ─────────────────────────────────
  // Uses treeRows (full dataset) when available for accurate file listing.
  //
  // Root-level (depth-0) files: files stored directly in the game root folder
  // (e.g. Mods\filename.package) with no subfolder. These are NOT in the
  // folder tree (buildFolderTree skips them) and must be surfaced separately.
  const folderContents = useMemo(() => {
    const items = treeRows?.items ?? rows?.items ?? [];
    if (!items.length || !folderTreeRoots) {
      return { subfolders: [], files: [], rootFiles: [] };
    }

    // Root-level files: items with folderSegments = ["Mods"] or ["Tray"].
    // These are depth-0 files that buildFolderTree skips.
    if (!activeFolderPath) {
      // Attach root files to the Mods/Tray nodes so folder row badges show
      // correct totals (tree files + depth-0 loose files) at the root level.
      const modsWithRoots: FolderNode = {
        ...folderTreeRoots.mods,
        rootFiles: rootItemsBySource.Mods,
      };
      const trayWithRoots: FolderNode = {
        ...folderTreeRoots.tray,
        rootFiles: rootItemsBySource.Tray,
      };
      return {
        subfolders: [modsWithRoots, trayWithRoots],
        files: [],
        // Surface ALL root-level files (depth-0) from both sources.
        // These 9,777 files are stored directly in Mods with no subfolder
        // and are NOT visible in the folder tree structure.
        rootFiles: [...rootItemsBySource.Mods, ...rootItemsBySource.Tray],
      };
    }

    // Inside a specific folder: reuse the prebuilt tree from loadTreeRows so
    // getFolderContents does not rebuild the full tree from flat rows.
    return getFolderContents(activeFolderPath, items, folderTreeRoots);
  }, [treeRows?.items, rows?.items, folderTreeRoots, activeFolderPath, rootItemsBySource]);

  // ── Synthetic root node for FolderContentPane when at root ─────────────
  const syntheticRoot: FolderNode = useMemo(() => {
    if (!folderTreeRoots) {
      return { name: "Root", fullPath: "", depth: 0, children: [], directFileCount: 0, totalFileCount: 0, childFolderCount: 0 };
    }
    return {
      name: "Root",
      fullPath: "",
      depth: -1,
      children: [folderTreeRoots.mods, folderTreeRoots.tray],
      directFileCount: 0,
      // Tree total (2,963 files in subfolders) + root-level files (9,777)
      totalFileCount: folderTreeRoots.mods.totalFileCount + folderTreeRoots.tray.totalFileCount + rootFileCount,
      childFolderCount: 2,
    };
  }, [folderTreeRoots, rootFileCount]);

  const libraryInspectorSections = selected
    ? [
        {
          id: "facts",
          label:
            userView === "beginner"
              ? "File facts"
              : isPowerView
                ? "File details"
                : "Overview",
          hint:
            isPowerView
              ? "Core classification, file metadata, and confidence."
              : "Only the details most simmers usually care about.",
          children: (
            <>
              {/* ── Casual: file identity + size/date (genuinely new vs sidebar Snapshot) ── */}
              {isCasualView ? (
                <div className="detail-list">
                  <DetailRow
                    label="Type"
                    value={`${friendlyTypeLabel(selected.kind)}${
                      selected.subtype?.trim() ? ` / ${selected.subtype}` : ""
                    }`}
                  />
                  {selected.size > 0 && (
                    <DetailRow
                      label="Size"
                      value={formatBytes(selected.size)}
                    />
                  )}
                  {selected.modifiedAt && (
                    <DetailRow
                      label="Modified"
                      value={new Date(selected.modifiedAt).toLocaleDateString()}
                    />
                  )}
                </div>
              ) : isPowerView ? (
                /* ── Power: full file story ── */
                <div className="detail-list">
                  <DetailRow
                    label="Creator"
                    value={((): string | ReactNode => {
                      const info = describeCreatorForInspector(selected);
                      return (
                        <span>
                          {info.label}
                          {info.suffix ? (
                            <span className="detail-row-suffix" title={`Creator ${info.suffix}`}>
                              {" "}({info.suffix})
                            </span>
                          ) : null}
                        </span>
                      );
                    })()}
                  />
                  <DetailRow
                    label="Type"
                    value={friendlyTypeLabel(selected.kind)}
                  />
                  {isTraySelection ? (
                    <>
                      <DetailRow
                        label="Tray type"
                        value={`${traySelection?.label ?? "Tray Item"} (${traySelection?.evidenceKind ?? "inferred"})`}
                      />
                      <DetailRow
                        label="Stored"
                        value={traySelection?.isMisplaced ? `${trayLocationLabel(traySelection?.location ?? "mods")} · review needed` : trayLocationLabel(traySelection?.location ?? "tray")}
                      />
                      {trayGroupingValue ? (
                        <DetailRow label="Grouped as" value={trayGroupingValue} />
                      ) : null}
                      {trayGroupedCount ? (
                        <DetailRow label="Tray set" value={trayGroupedCount} />
                      ) : null}
                    </>
                  ) : null}
                  <DetailRow
                    label="Subtype"
                    value={selected.subtype ?? "Unspecified"}
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
                  {relationship && relationship.type !== "none" ? (
                    <DetailRow
                      label="Related"
                      value={
                        <span>
                          {relationship.label}
                          <span
                            className={`detail-row-suffix detail-row-suffix--${relationship.proofLevel}`}
                            title={`This relationship is ${relationship.proofLevel}`}
                          >
                            {" "}({relationship.proofLevel})
                          </span>
                        </span>
                      }
                    />
                  ) : folderName ? (
                    <DetailRow
                      label="Folder"
                      value={
                        <span className="ghost-chip">
                          {folderName}
                        </span>
                      }
                    />
                  ) : null}
                </div>
              ) : (
                /* ── Seasoned: middle ground ── */
                <>
                  <div className="detail-list">
                    <DetailRow
                      label="Creator"
                      value={((): string | ReactNode => {
                        const info = describeCreatorForInspector(selected);
                        return (
                          <span>
                            {info.label}
                            {info.suffix ? (
                              <span className="detail-row-suffix" title={`Creator ${info.suffix}`}>
                                {" "}({info.suffix})
                              </span>
                            ) : null}
                          </span>
                        );
                      })()}
                    />
                    <DetailRow
                      label="Type"
                      value={friendlyTypeLabel(selected.kind)}
                    />
                    {isTraySelection ? (
                      <>
                        <DetailRow
                          label="Stored"
                          value={traySelection?.isMisplaced ? `${trayLocationLabel(traySelection?.location ?? "mods")} · review needed` : trayLocationLabel(traySelection?.location ?? "tray")}
                        />
                        {trayGroupingValue ? (
                          <DetailRow label="Grouped as" value={trayGroupingValue} />
                        ) : null}
                        {trayGroupedCount ? (
                          <DetailRow label="Tray set" value={trayGroupedCount} />
                        ) : null}
                      </>
                    ) : null}
                    {selected.subtype?.trim() ? (
                      <DetailRow label="Subtype" value={selected.subtype} />
                    ) : null}
                    <DetailRow label="File format" value={formatLibraryFileFormat(selected)} />
                    <DetailRow
                      label="Size"
                      value={formatBytes(selected.size)}
                    />
                    {selected.modifiedAt ? (
                      <DetailRow
                        label="Modified"
                        value={new Date(selected.modifiedAt).toLocaleDateString()}
                      />
                    ) : null}
                    {selected.createdAt ? (
                      <DetailRow
                        label="Added to library"
                        value={new Date(selected.createdAt).toLocaleDateString()}
                      />
                    ) : null}
                    {relationship && relationship.type !== "none" ? (
                      <DetailRow
                        label="Related"
                        value={
                          <span>
                            {relationship.label}
                            <span
                              className={`detail-row-suffix detail-row-suffix--${relationship.proofLevel}`}
                              title={`This relationship is ${relationship.proofLevel}`}
                            >
                              {" "}({relationship.proofLevel})
                            </span>
                          </span>
                        }
                      />
                    ) : folderName ? (
                      <DetailRow
                        label="Folder"
                        value={
                          <span className="ghost-chip">
                            {folderName}
                          </span>
                        }
                      />
                    ) : null}
                  </div>
                  {playerFacingNames.length ? (
                    <div className="detail-block">
                      <div className="section-label">Found in game as</div>
                      <div className="tag-list">
                        {playerFacingNames.map((item) => (
                          <span key={item} className="ghost-chip">
                            {item}
                          </span>
                        ))}
                      </div>
                    </div>
                  ) : null}
                  {selected.insights.resourceSummary.length ? (
                    <div className="detail-block">
                      <div className="section-label">Contents</div>
                      <div className="tag-list">
                        {selected.insights.resourceSummary.map((item) => (
                          <span key={item} className="ghost-chip">{item}</span>
                        ))}
                      </div>
                    </div>
                  ) : null}
                  {selected.insights.familyHints.length ? (
                    <div className="detail-block">
                      <div className="section-label">Family</div>
                      <div className="tag-list">
                        {selected.insights.familyHints.map((hint) => (
                          <span key={hint} className="ghost-chip">{hint}</span>
                        ))}
                      </div>
                    </div>
                  ) : null}
                </>
              )}
            </>
          ),
        },
        ...(hasVersionWatchInfo || selected
          ? [
              {
                id: "updatesHint",
                label: userView === "beginner" ? "Updates" : "Update watch",
                hint:
                  isPowerView
                    ? "Tracking status, latest version clues, and watch source details."
                    : isCasualView
                      ? "Whether this file is being tracked for updates."
                      : "Whether this file is tracked and what SimSuite knows about new versions.",
                children: (
                  <div className="detail-block">
                    {hasVersionWatchInfo ? (
                      <>
                        <DetailRow
                          label="Watch status"
                          value={selected.watchResult?.status ? watchStatusLabel(selected.watchResult.status) : "Tracked"}
                        />
                        <DetailRow
                          label="Installed"
                          value={
                            ((): string | ReactNode => {
                              const vInfo = describeVersionForInspector(
                                selected.installedVersionSummary?.version ?? null,
                                selected.installedVersionSummary?.confidence ?? null,
                              );
                              if (!vInfo.label) {
                                return "Unknown";
                              }
                              return (
                                <span>
                                  {vInfo.label}
                                  {vInfo.tierLabel !== "Confirmed" ? (
                                    <span
                                      className="detail-row-suffix"
                                      title={`Version ${vInfo.tierLabel.toLowerCase()}`}
                                    >
                                      {" "}
                                      ({vInfo.tierLabel})
                                    </span>
                                  ) : null}
                                </span>
                              );
                            })()
                          }
                        />
                        {selected.watchResult?.sourceLabel ? (
                          <DetailRow
                            label="Source"
                            value={selected.watchResult.sourceLabel}
                          />
                        ) : null}
                        {selected.watchResult?.latestVersion ? (
                          <DetailRow
                            label="Latest seen"
                            value={selected.watchResult.latestVersion}
                          />
                        ) : null}
                        {selected.watchResult?.checkedAt ? (
                          <DetailRow
                            label="Last checked"
                            value={new Date(selected.watchResult.checkedAt).toLocaleString()}
                          />
                        ) : null}
                        {isPowerView && selected.watchResult?.capability ? (
                          <DetailRow
                            label="Watch capability"
                            value={selected.watchResult.capability}
                          />
                        ) : null}
                        {selected.watchResult?.note ? (
                          <p className="text-muted" style={{ marginTop: "0.4rem" }}>
                            {selected.watchResult.note}
                          </p>
                        ) : (
                          <p className="text-muted" style={{ marginTop: "0.4rem" }}>
                            Open Updates to manage sources, check for new versions, and review changes.
                          </p>
                        )}
                      </>
                    ) : (
                      <p className="text-muted">
                        No version tracking yet. Open Updates to start tracking this file.
                      </p>
                    )}

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
                        style={{ marginTop: "0.5rem" }}
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

                    {/* Parser warnings — uncapped in sheet, shown for seasoned+ */}
                    {isPowerView || selected.parserWarnings.length > 0 ? (
                      <div className="detail-block">
                        <div className="section-label">Parser notes</div>
                        {selected.parserWarnings.length ? (
                          <div className="tag-list">
                            {selected.parserWarnings.map((note) => (
                              <span key={note} className="ghost-chip">
                                {note}
                              </span>
                            ))}
                          </div>
                        ) : (
                          <p>No parser notes.</p>
                        )}
                      </div>
                    ) : null}

                    {/* Version signals — rich structured data for power users only */}
                    {isPowerView && selected.insights.versionSignals.length > 0 ? (
                      <div className="detail-block">
                        <div className="section-label">Version signals</div>
                        <p className="text-muted" style={{ marginBottom: "0.4rem" }}>
                          Where SimSuite found version clues and how confident it is.
                        </p>
                        {selected.insights.versionSignals.map((signal) => (
                          <div
                            key={signal.rawValue}
                            style={{
                              display: "flex",
                              justifyContent: "space-between",
                              gap: "0.4rem",
                              marginBottom: "0.3rem",
                            }}
                          >
                            <span
                              className="ghost-chip"
                              style={{ fontSize: "0.72rem" }}
                            >
                              {signal.sourceKind}
                              {signal.matchedBy ? ` · ${signal.matchedBy}` : null}
                            </span>
                            <span
                              style={{
                                fontSize: "0.68rem",
                                color: "var(--text-dim)",
                                fontWeight: 600,
                              }}
                            >
                              {Math.round(signal.confidence * 100)}%
                            </span>
                          </div>
                        ))}
                      </div>
                    ) : null}
                  </>
                ),
              },
            ]
          : []),
        // Replaced inspection soup with three focused sections
        ...(userView !== "beginner" && hasInspectionSignals
          ? [
              ...(isTraySelection
                ? [
                    {
                      id: "tray-context",
                      label: "Tray Context",
                      hint:
                        "What kind of tray item this is, where it lives, and what SimSuite is inferring versus actually knows.",
                      defaultCollapsed: false,
                      children: buildSheetTraySection(selected, userView),
                    },
                  ]
                : [
                    {
                      id: "whats-inside",
                      label: "What's Inside",
                      hint:
                        "What SimSuite extracted from the file — namespaces, contents, and embedded names.",
                      defaultCollapsed: false,
                      children: buildSheetContentsSection(selected, userView),
                    },
                  ]),
              {
                id: "attribution",
                label: isTraySelection ? "Identity & Attribution" : "Attribution",
                hint: isTraySelection
                  ? "Creator clues, tray grouping, and any related identity hints SimSuite can support honestly."
                  : "Who made this, what set it belongs to, and the evidence behind those clues.",
                defaultCollapsed: false,
                children: buildSheetAttributionSection(selected, userView),
              },
              {
                id: "compatibility",
                label: "Compatibility & Health",
                hint:
                  "Version signals, update status, and what might need attention.",
                defaultCollapsed: true,
                children: buildSheetCompatibilitySection(selected, "inspect", userView),
              },
            ]
          : []),
        ...(!isCasualView
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
  // ─── Sheet sections — view-aware ──────────────────────────────────────────
  // health: genuinely deeper diagnostics not shown in the sidebar
  //   • file path + size + modified date (shown nowhere else in Casual)
  //   • version signals with source + confidence (only in power view)
  //   • last-checked-at + watch capability (never shown anywhere)
  //   • full warnings list uncapped for all views
  // inspect: the file's embedded identity and structural facts
  //   • all insights (creator hints, version numbers, embedded names, etc.)
  //   • file path (shown in sidebar only for power)
  // edit: creator learning + category override (always the same — interactive)
  // ─────────────────────────────────────────────────────────────────────────

  const librarySheetSections =
    activeLibrarySheet && selected
      ? libraryInspectorSections.filter((section) => {
          if (activeLibrarySheet === "health") {
            // health = deep diagnostics:
            // Casual: path/size/date (genuinely new) + full warnings (uncapped)
            // Seasoned: + bundle info + watch signals (versionSignals)
            // Creator: + watch capability + last-checked-at
            if (isCasualView) {
              return ["safety", "updatesHint"].includes(section.id);
            }
            if (isPowerView) {
              return ["safety", "updatesHint", "facts", "tray-context", "whats-inside", "attribution", "compatibility"].includes(section.id);
            }
            // Seasoned
            return ["safety", "updatesHint"].includes(section.id);
          }

          if (activeLibrarySheet === "inspect") {
            // inspect = file's full story: facts + type-aware insight sections + path
            if (isCasualView) {
              return ["facts"].includes(section.id);
            }
            if (isPowerView) {
              return [
                "facts",
                "tray-context",
                "whats-inside",
                "attribution",
                "compatibility",
                "path",
              ].includes(section.id);
            }
            return ["facts", "tray-context", "whats-inside", "attribution", "compatibility"].includes(section.id);
          }

          // edit: always creator + category
          return ["creator", "category"].includes(section.id);
        })
      : [];

  return (
    <>
    <Workbench fullHeight className="library-workbench">
      <WorkbenchStage className="library-stage-shell">
        <LibraryTopStrip
          userView={userView}
          search={search}
          activeFilterCount={activeFilterCount}
          drawerOpen={moreFiltersOpen}
          watchFilter={watchFilter}
          sortBy={sortBy}
          librarySummary={librarySummary}
          facets={facets}
          viewMode={viewMode}
          pageSize={pageSize}
          densityValue={densityValue}
          onPageSizeChange={(v) => { setPageSize(v); setPage(0); }}
          onDensityChange={setDensityValue}
          onViewModeChange={setViewMode}
          filters={{
            kind,
            creator,
            source,
            subtype,
            minConfidence,
          }}
          onSearchChange={(value) => {
            setSearch(value);
            setPage(0);
          }}
          onFiltersChange={(next) => {
            if (typeof next.kind === "string") {
              setKind(next.kind);
              setSubtype("");
            }
            if (typeof next.creator === "string") setCreator(next.creator);
            if (typeof next.source === "string") setSource(next.source);
            if (typeof next.subtype === "string") setSubtype(next.subtype);
            if (typeof next.minConfidence === "string") setMinConfidence(next.minConfidence);
            setPage(0);
          }}
          onResetFilters={resetFilters}
          onDrawerToggle={() => setMoreFiltersOpen((current) => !current)}
          onWatchFilterChange={(value) => {
            setWatchFilter(value);
            setPage(0);
          }}
          onSortByChange={(value) => {
            setSortBy(value);
            setPage(0);
          }}
        />

        {selectedIds.size > 0 ? (
          <div className="library-selection-strip">
            <span className="library-selection-count">
              {selectedIds.size} selected
            </span>
            <div className="library-selection-actions">
              <button
                type="button"
                className="secondary-action"
                onClick={() => {
                  const selected = rows?.items.filter((r) => selectedIds.has(r.id)) ?? [];
                  const paths = selected.map((r) => r.path).join("\n");
                  void navigator.clipboard.writeText(paths);
                }}
                title="Copy file paths to clipboard"
              >
                Copy path{selectedIds.size > 1 ? "s" : ""}
              </button>
              <button
                type="button"
                className="secondary-action"
                onClick={() => {
                  const selected = rows?.items.filter((r) => selectedIds.has(r.id)) ?? [];
                  const names = selected.map((r) => r.filename).join("\n");
                  void navigator.clipboard.writeText(names);
                }}
                title="Copy filenames to clipboard"
              >
                Copy names
              </button>
              {selectedIds.size === 1 ? (
                <button
                  type="button"
                  className="secondary-action"
                  onClick={() => {
                    const selected = rows?.items.find((r) => selectedIds.has(r.id));
                    if (selected) {
                      void api.revealFileInFolder(selected.path);
                    }
                  }}
                  title="Open containing folder"
                >
                  Open folder
                </button>
              ) : null}
              <button
                type="button"
                className="secondary-action"
                onClick={() => {
                  const pageIds = (rows?.items ?? []).map((r) => r.id);
                  setSelectedIds((current) => {
                    const next = new Set(current);
                    pageIds.forEach((id) => next.add(id));
                    return next;
                  });
                }}
                title="Select all items on this page"
              >
                Select all
              </button>
              {selectedIds.size > 0 && onNavigateDuplicates ? (
                <button
                  type="button"
                  className="secondary-action"
                  onClick={() => {
                    onNavigateDuplicates(Array.from(selectedIds));
                  }}
                  title="View duplicate pairs for selected files"
                >
                  Open in Duplicates
                </button>
              ) : null}
              {selectedIds.size > 0 && onNavigateWithParams ? (
                <button
                  type="button"
                  className="secondary-action"
                  onClick={() => {
                    onNavigateWithParams(
                      "updates",
                      undefined,
                      undefined,
                      undefined,
                      Array.from(selectedIds),
                    );
                  }}
                  title="Open update status for selected files"
                >
                  Open in Updates
                </button>
              ) : null}
              <button
                type="button"
                className="secondary-action"
                onClick={() => setSelectedIds(new Set())}
              >
                Clear
              </button>
            </div>
          </div>
        ) : null}

        {rows === null ? (
          <SkeletonLoader rows={8} height={44} />
        ) : viewMode === "grid" ? (
          <LibraryThumbnailGrid
            userView={userView}
            rows={rows?.items ?? []}
            selectedId={selected?.id ?? null}
            page={page}
            totalPages={totalPages}
            onSelect={(row) => void openFile(row)}
            onPrevPage={() => setPage((current) => Math.max(current - 1, 0))}
            onNextPage={() => setPage((current) => current + 1)}
          />
        ) : viewMode === "folders" ? (
          <div className="library-folders-layout">
            <div className="library-folder-tree-pane">
              {/*
                Phase 5ac: Cold start — no tree cached AND nothing loaded yet.
                Show skeleton rows (not spinner) so the first entry has a polished feel.
                After first load, prevTreeCacheRef is always warm (background preloader
                keeps treeRows populated while in list/grid mode).
              */}
              {!prevTreeCacheRef.current && !folderTreeRoots ? (
                <div className="folder-tree-skeleton" aria-label="Loading folder tree">
                  <div className="folder-tree-skeleton-row">
                    <div className="folder-tree-skeleton-row__chevron" />
                    <div className="folder-tree-skeleton-row__icon" />
                    <div className="folder-tree-skeleton-row__label" style={{ width: "60px" }} />
                  </div>
                  <div className="folder-tree-skeleton-row folder-tree-skeleton-row--l1">
                    <div className="folder-tree-skeleton-row__chevron" />
                    <div className="folder-tree-skeleton-row__icon" />
                    <div className="folder-tree-skeleton-row__label" style={{ width: "90px" }} />
                  </div>
                  <div className="folder-tree-skeleton-row folder-tree-skeleton-row--l1">
                    <div className="folder-tree-skeleton-row__chevron" />
                    <div className="folder-tree-skeleton-row__icon" />
                    <div className="folder-tree-skeleton-row__label" style={{ width: "120px" }} />
                  </div>
                  <div className="folder-tree-skeleton-row folder-tree-skeleton-row--l2">
                    <div className="folder-tree-skeleton-row__chevron" />
                    <div className="folder-tree-skeleton-row__icon" />
                    <div className="folder-tree-skeleton-row__label" style={{ width: "80px" }} />
                  </div>
                  <div className="folder-tree-skeleton-row folder-tree-skeleton-row--l2">
                    <div className="folder-tree-skeleton-row__chevron" />
                    <div className="folder-tree-skeleton-row__icon" />
                    <div className="folder-tree-skeleton-row__label" style={{ width: "110px" }} />
                  </div>
                </div>
              ) : null}
              {/*
                Phase 5ac: Show tree immediately — either fresh (folderTreeRoots) or
                cached (prevTreeCacheRef.current during a warm reload).
                No spinner, no "Building folder tree…" text — the tree feels instant.
              */}
              {(folderTreeRoots ?? prevTreeCacheRef.current) ? (
                <FolderTreePane
                  tree={(folderTreeRoots ?? prevTreeCacheRef.current)!.mods}
                  activePath={activeFolderPath}
                  onNavigate={(path) => setActiveFolderPath(path)}
                />
              ) : null}
              {(folderTreeRoots ?? prevTreeCacheRef.current) ? (
                <FolderTreePane
                  tree={(folderTreeRoots ?? prevTreeCacheRef.current)!.tray}
                  activePath={activeFolderPath}
                  onNavigate={(path) => setActiveFolderPath(path)}
                />
              ) : null}
            </div>
            <FolderContentPane
              userView={userView}
              folderPath={activeFolderPath}
              subfolders={folderContents.subfolders}
              files={folderContents.files}
              rootFiles={folderContents.rootFiles}
              tree={syntheticRoot}
              onNavigate={(path) => setActiveFolderPath(path)}
              onSelectFile={(row) => void openFile(row)}
              selectedFile={selected}
            />
          </div>
        ) : (
          <LibraryCollectionTable
            userView={userView}
            rows={rows?.items ?? []}
            selectedId={selected?.id ?? null}
            selectedIds={selectedIds}
            page={page}
            totalPages={totalPages}
            onSelect={(row) => void openFile(row)}
            onToggleSelect={(id) => {
              setSelectedIds((prev) => {
                const next = new Set(prev);
                if (next.has(id)) {
                  next.delete(id);
                } else {
                  next.add(id);
                }
                return next;
              });
            }}
            onPrevPage={() => setPage((current) => Math.max(current - 1, 0))}
            onNextPage={() => setPage((current) => current + 1)}
          />
        )}
      </WorkbenchStage>

      {!inspectorCollapsed ? (
        <button
          type="button"
          className="library-inspector-backdrop"
          aria-label="Close inspector overlay"
          onClick={() => setInspectorCollapsed(true)}
        />
      ) : null}

      {/* Right inspector panel */}
      <WorkbenchInspector
        className="library-inspector-shell"
        collapsible
        collapsed={inspectorCollapsed}
        onCollapse={setInspectorCollapsed}
      >
        <ErrorBoundary
          fallback={
            <div style={{ padding: "1rem", color: "var(--text-dim, #7f928a)", fontSize: "0.82rem" }}>
              Details unavailable.
            </div>
          }
        >
          <LibraryDetailsPanel
            userView={userView}
            selectedFile={selected}
            onOpenInspectDetails={() => setActiveLibrarySheet("inspect")}
            onOpenHealthDetails={() => setActiveLibrarySheet("health")}
            onOpenEditDetails={() => setActiveLibrarySheet("edit")}
            onOpenUpdates={() => {
              if (!selected || !onNavigateWithParams || !updatesTarget) {
                return;
              }

              onNavigateWithParams(
                "updates",
                updatesTarget.mode,
                updatesTarget.filter,
                selected.id,
              );
            }}
            relationship={relationship}
            folderName={folderName}
            headerRight={
              <button
                type="button"
                className="inspector-collapse-btn"
                onClick={() => setInspectorCollapsed(true)}
                aria-label="Collapse inspector"
                title="Collapse inspector"
              >
                <PanelLeftClose size={16} strokeWidth={2} />
              </button>
            }
          />
        </ErrorBoundary>
      </WorkbenchInspector>
    </Workbench>

      <LibraryDetailSheet
        open={Boolean(activeLibrarySheet && selected)}
        mode={activeLibrarySheet}
        selectedFile={selected}
        sections={librarySheetSections}
        userView={userView}
        onClose={() => setActiveLibrarySheet(null)}
        relationship={relationship}
        folderName={folderName}
      />
    </>
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
  value: string | ReactNode;
  mono?: boolean;
}) {
  return (
    <div className="detail-row">
      <span>{label}</span>
      <strong className={mono ? "mono-text" : ""}>{value}</strong>
    </div>
  );
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

function watchStatusLabel(status: NonNullable<FileDetail["watchResult"]>["status"]) {
  switch (status) {
    case "current":
      return "Up to date";
    case "exact_update_available":
      return "Update available";
    case "possible_update":
      return "Possible update";
    case "unknown":
      return "Unknown";
    case "not_watched":
    default:
      return "Not tracked";
  }
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

function confidenceTone(confidence: number) {
  if (confidence >= 0.85) {
    return "good";
  }

  if (confidence >= 0.6) {
    return "medium";
  }

  return "low";
}
