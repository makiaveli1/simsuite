import { useEffect, useRef } from "react";
import { ArrowUpDown, Grid3X3, LayoutList, Search, SlidersHorizontal, X } from "lucide-react";
import type { LibraryFacets, LibrarySortField, LibrarySummary, LibraryWatchFilter, UserView } from "../../lib/types";
import { libraryViewFlags } from "./libraryDisplay";
import { friendlyTypeLabel } from "../../lib/uiLanguage";

type WatchFilter = LibraryWatchFilter;
type SortField = LibrarySortField;

const WATCH_FILTER_OPTIONS: { value: WatchFilter; label: string }[] = [
  { value: "all", label: "All" },
  { value: "has_updates", label: "Has Updates" },
  { value: "needs_attention", label: "Needs review" },
  { value: "not_tracked", label: "Not Tracked" },
  { value: "duplicates", label: "Duplicates" },
];

const SORT_OPTIONS: { value: SortField; label: string }[] = [
  { value: "name", label: "Name" },
  { value: "creator", label: "Creator" },
  { value: "recently_modified", label: "Recently Modified" },
  { value: "has_updates_first", label: "Has updates first" },
];

const CONFIDENCE_OPTIONS = [
  { value: "", label: "Any match" },
  { value: "0.35", label: "35%+" },
  { value: "0.55", label: "55%+" },
  { value: "0.75", label: "75%+" },
] as const;

export interface LibraryToolbarFilters {
  kind: string;
  creator: string;
  source: string;
  subtype: string;
  minConfidence: string;
}

interface LibraryTopStripProps {
  userView: UserView;
  activeFilterCount: number;
  search: string;
  sortBy: SortField;
  watchFilter: WatchFilter;
  filters: LibraryToolbarFilters;
  facets: LibraryFacets | null;
  drawerOpen: boolean;
  librarySummary: LibrarySummary | null;
  viewMode: "list" | "grid";
  pageSize: number;
  densityValue: number;
  onPageSizeChange: (value: number) => void;
  onDensityChange: (value: number) => void;
  onSearchChange: (value: string) => void;
  onSortByChange: (value: SortField) => void;
  onWatchFilterChange: (value: WatchFilter) => void;
  onFiltersChange: (next: Partial<LibraryToolbarFilters>) => void;
  onDrawerToggle: () => void;
  onResetFilters: () => void;
  onViewModeChange: (mode: "list" | "grid") => void;
}

function clampDensity(value: number) {
  return Math.max(0, Math.min(100, value));
}

function sourceLabel(source: string) {
  if (!source) return "";
  if (source === "mods") return "Mods";
  if (source === "tray") return "Tray";
  return source;
}

export function LibraryTopStrip({
  userView,
  activeFilterCount,
  search,
  sortBy,
  watchFilter,
  filters,
  facets,
  drawerOpen,
  librarySummary,
  viewMode,
  onSearchChange,
  onSortByChange,
  onWatchFilterChange,
  onFiltersChange,
  onDrawerToggle,
  onResetFilters,
  onViewModeChange,
  pageSize,
  onPageSizeChange,
  densityValue,
  onDensityChange,
}: LibraryTopStripProps) {
  const flags = libraryViewFlags(userView);
  const filterButtonRef = useRef<HTMLButtonElement | null>(null);
  const firstDrawerControlRef = useRef<HTMLSelectElement | null>(null);
  const previousDrawerOpenRef = useRef(drawerOpen);
  const normalizedDensity = clampDensity(densityValue);
  const hasActiveFilters = activeFilterCount > 0;
  const drawerFilterCount = [filters.creator, filters.source, filters.minConfidence, filters.subtype].filter(Boolean).length;
  const activeDrawerFilters = [
    filters.creator
      ? { key: "creator", label: `Creator: ${filters.creator}`, clear: () => onFiltersChange({ creator: "" }) }
      : null,
    filters.source
      ? { key: "source", label: `Source: ${sourceLabel(filters.source)}`, clear: () => onFiltersChange({ source: "" }) }
      : null,
    filters.minConfidence
      ? {
          key: "minConfidence",
          label: `Confidence: ${Math.round(Number(filters.minConfidence) * 100)}%+`,
          clear: () => onFiltersChange({ minConfidence: "" }),
        }
      : null,
    filters.subtype
      ? { key: "subtype", label: `Subtype: ${filters.subtype}`, clear: () => onFiltersChange({ subtype: "" }) }
      : null,
  ].filter(Boolean) as { key: string; label: string; clear: () => void }[];

  useEffect(() => {
    if (drawerOpen && !previousDrawerOpenRef.current) {
      firstDrawerControlRef.current?.focus();
    }

    if (!drawerOpen && previousDrawerOpenRef.current) {
      filterButtonRef.current?.focus();
    }

    previousDrawerOpenRef.current = drawerOpen;
  }, [drawerOpen]);

  return (
    <div className="library-top-strip">
      {/* ── Primary row: search + core controls + density widget ── */}
      <div className="library-toolbar-row">
        <label className="field library-toolbar-search">
          <span className="sr-only">Search by file or creator</span>
          <div className="downloads-search-input">
            <Search size={14} strokeWidth={2} />
            <input
              value={search}
              onChange={(event) => onSearchChange(event.target.value)}
              placeholder="Search by name or creator…"
              aria-label="Search library"
            />
            {search ? (
              <button
                type="button"
                className="search-clear"
                onClick={() => onSearchChange("")}
                aria-label="Clear search"
              >
                <X size={12} strokeWidth={2} />
              </button>
            ) : null}
          </div>
        </label>

        <div className="library-toolbar-actions">
          <div className="library-sort-control" aria-label="Sort library">
            <ArrowUpDown size={13} strokeWidth={2} />
            <select
              value={sortBy}
              onChange={(event) => onSortByChange(event.target.value as SortField)}
              aria-label="Sort by"
            >
              {SORT_OPTIONS.map((opt) => (
                <option key={opt.value} value={opt.value}>{opt.label}</option>
              ))}
            </select>
          </div>

          {/* ── Items per page — sits with layout controls, not filters ── */}
          <div className="library-page-size-control" aria-label="Items per page">
            <select
              value={pageSize}
              onChange={(event) => onPageSizeChange(Number(event.target.value))}
              aria-label="Items per page"
            >
              {[50, 100, 250, 500].map((n) => (
                <option key={n} value={n}>{n}</option>
              ))}
            </select>
          </div>

          <div className="library-view-toggle" role="group" aria-label="View mode">
            <button
              type="button"
              className={`library-view-btn${viewMode === "grid" ? " is-active" : ""}`}
              onClick={() => onViewModeChange("grid")}
              aria-pressed={viewMode === "grid"}
              title="Grid view"
            >
              <Grid3X3 size={14} strokeWidth={2} />
            </button>
            <button
              type="button"
              className={`library-view-btn${viewMode === "list" ? " is-active" : ""}`}
              onClick={() => onViewModeChange("list")}
              aria-pressed={viewMode === "list"}
              title="List view"
            >
              <LayoutList size={14} strokeWidth={2} />
            </button>
          </div>

          {/* ── Density widget — only meaningful in grid view ── */}
          {viewMode === "grid" && (
          <div className="library-density-widget" aria-label="Card density">
            <div className="library-density-widget__label" aria-hidden="true">
              dense <span className="library-density-widget__arrow">⟷</span> spacious
            </div>
            <div className="library-density-widget__control">
              <div
                className="library-density-fill-track"
                style={{ width: `${normalizedDensity}%` }}
                aria-hidden="true"
              />
              <input
                type="range"
                min="0"
                max="100"
                step="1"
                value={normalizedDensity}
                onChange={(event) => onDensityChange(clampDensity(Number(event.target.value)))}
                className="library-density-slider"
                aria-label={`Card density ${Math.round(normalizedDensity)} percent`}
              />
            </div>
            <div className="library-density-widget__hint">
              ≈{Math.max(2, Math.round(8 - (normalizedDensity / 100) * 5))} cards/row
            </div>
          </div>
          )}

          {flags.showAdvancedFilters ? (
            <button
              ref={filterButtonRef}
              type="button"
              className={`library-advanced-btn${drawerOpen ? " is-active" : ""}`}
              onClick={onDrawerToggle}
              aria-expanded={drawerOpen}
              aria-controls="library-filter-drawer"
            >
              <SlidersHorizontal size={13} strokeWidth={2} />
              Advanced
              {drawerFilterCount > 0 ? (
                <span className="active-badge">{drawerFilterCount}</span>
              ) : null}
              <span className={`library-advanced-btn__caret${drawerOpen ? " is-open" : ""}`} aria-hidden="true">
                ▾
              </span>
            </button>
          ) : null}
        </div>
      </div>

      {/* ── Quick filters row: kind chips + watch chips (secondary, visually subordinate) ── */}
      <div className="library-browse-row">
        <div className="library-browse-group library-browse-group--kinds" role="group" aria-label="Filter by type">
          <button
            type="button"
            className={`library-kind-chip${filters.kind === "" ? " is-active" : ""}`}
            onClick={() => onFiltersChange({ kind: "" })}
            aria-pressed={filters.kind === ""}
            title="Show all types"
          >
            All
          </button>
          {facets?.kinds.map((kind) => {
            const cssClass = `type-pill--${kind.charAt(0).toLowerCase() + kind.slice(1)}`;
            const isActive = filters.kind === kind;
            return (
              <button
                key={kind}
                type="button"
                className={`library-kind-chip ${cssClass}${isActive ? " is-active" : ""}`}
                onClick={() => onFiltersChange({ kind: isActive ? "" : kind })}
                aria-pressed={isActive}
                title={isActive ? `Showing ${friendlyTypeLabel(kind)} — click to clear` : `Show ${friendlyTypeLabel(kind)}`}
              >
                {friendlyTypeLabel(kind)}
              </button>
            );
          })}
        </div>

        {/* Contextual subtype chips — only in grid view, when a kind is active */}
        {viewMode === 'grid' && filters.kind && facets?.subtypes && facets.subtypes.length > 0 && (
          <div className="library-browse-sep" aria-hidden="true" />
        )}
        {viewMode === 'grid' && filters.kind && facets?.subtypes && facets.subtypes.length > 0 && (
          <div className="library-subtype-chips" role="group" aria-label="Filter by subtype">
            {facets.subtypes.map((s) => {
              const isActive = filters.subtype === s;
              return (
                <button
                  key={s}
                  type="button"
                  className={`library-subtype-chip${isActive ? " is-active" : ""}`}
                  onClick={() => onFiltersChange({ subtype: isActive ? "" : s })}
                  aria-pressed={isActive}
                  title={isActive ? `Showing ${s} — click to clear` : `Show ${s}`}
                >
                  {s}
                </button>
              );
            })}
          </div>
        )}

        <div className="library-browse-sep" aria-hidden="true" />

        <div className="library-browse-group library-browse-group--watch" role="group" aria-label="Quick filters by status">
          {WATCH_FILTER_OPTIONS.map((opt) => (
            <button
              key={opt.value}
              type="button"
              className={`library-quick-chip${watchFilter === opt.value ? " is-active" : ""}`}
              onClick={() => onWatchFilterChange(opt.value)}
              aria-pressed={watchFilter === opt.value}
            >
              {opt.label}
            </button>
          ))}
        </div>

        {librarySummary ? (
          <div className="library-browse-summary" aria-label="Library health summary">
            {librarySummary.tracked > 0 && (
              <span className="library-summary-pill">
                <strong>{librarySummary.tracked.toLocaleString()}</strong> tracked
              </span>
            )}
            {librarySummary.hasUpdates > 0 && (
              <span className="library-summary-pill has-updates">
                <strong>{librarySummary.hasUpdates.toLocaleString()}</strong> updates
              </span>
            )}
            {librarySummary.needsReview > 0 && (
              <span className="library-summary-pill needs-review">
                <strong>{librarySummary.needsReview.toLocaleString()}</strong> review
              </span>
            )}
          </div>
        ) : null}
      </div>

      {/* ── Advanced drawer: creator/source/confidence/subtype + active filter pills ── */}
      {flags.showAdvancedFilters ? (
        <div
          id="library-filter-drawer"
          className="library-filter-drawer"
          aria-hidden={!drawerOpen}
        >
          <div className="library-filter-drawer-inner">
            <label className="field library-toolbar-select">
              <span className="sr-only">Creator</span>
              <select
                ref={firstDrawerControlRef}
                id="lib-filter-creator"
                value={filters.creator}
                onChange={(event) => onFiltersChange({ creator: event.target.value })}
                aria-label="Filter by creator"
              >
                <option value="">All creators</option>
                {facets?.creators.map((creatorOption) => (
                  <option key={creatorOption} value={creatorOption}>{creatorOption}</option>
                ))}
              </select>
            </label>

            <label className="field library-toolbar-select">
              <span className="sr-only">Source</span>
              <select
                id="lib-filter-source"
                value={filters.source}
                onChange={(event) => onFiltersChange({ source: event.target.value })}
                aria-label="Filter by source"
              >
                <option value="">All sources</option>
                <option value="mods">Mods</option>
                <option value="tray">Tray</option>
              </select>
            </label>

            <label className="field library-toolbar-select">
              <span className="sr-only">Confidence</span>
              <select
                value={filters.minConfidence}
                onChange={(event) => onFiltersChange({ minConfidence: event.target.value })}
                aria-label="Minimum confidence"
              >
                {CONFIDENCE_OPTIONS.map((option) => (
                  <option key={option.value || "any"} value={option.value}>{option.label}</option>
                ))}
              </select>
            </label>

            <label className="field library-toolbar-select">
              <span className="sr-only">Subtype</span>
              <select
                value={filters.subtype}
                onChange={(event) => onFiltersChange({ subtype: event.target.value })}
                aria-label="Filter by subtype"
              >
                <option value="">All subtypes</option>
                {facets?.subtypes.map((item) => (
                  <option key={item} value={item}>{item}</option>
                ))}
              </select>
            </label>

            {activeDrawerFilters.length > 0 && (
              <div className="library-drawer-active-filters" aria-label="Active precision filters">
                {activeDrawerFilters.map((filter) => (
                  <button
                    key={filter.key}
                    type="button"
                    className="library-inline-filter-pill"
                    onClick={filter.clear}
                    title={`Clear ${filter.label}`}
                  >
                    {filter.label}
                    <X size={11} strokeWidth={2} />
                  </button>
                ))}
              </div>
            )}

            <button
              type="button"
              className="secondary-action library-drawer-reset"
              onClick={onResetFilters}
              disabled={!hasActiveFilters}
            >
              Clear all
            </button>
          </div>
        </div>
      ) : null}
    </div>
  );
}
