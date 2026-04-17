import { ArrowUpDown, Grid3X3, LayoutList, RotateCcw, Search, SlidersHorizontal, X } from "lucide-react";
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
  moreFiltersOpen: boolean;
  librarySummary: LibrarySummary | null;
  viewMode: "list" | "grid";
  pageSize: number;
  onPageSizeChange: (value: number) => void;
  cardDensity: "small" | "medium" | "large";
  onCardDensityChange: (v: "small" | "medium" | "large") => void;
  onSearchChange: (value: string) => void;
  onSortByChange: (value: SortField) => void;
  onWatchFilterChange: (value: WatchFilter) => void;
  onFiltersChange: (next: Partial<LibraryToolbarFilters>) => void;
  onToggleMoreFilters: () => void;
  onResetFilters: () => void;
  onViewModeChange: (mode: "list" | "grid") => void;
}

export function LibraryTopStrip({
  userView,
  activeFilterCount,
  search,
  sortBy,
  watchFilter,
  filters,
  facets,
  moreFiltersOpen,
  librarySummary,
  viewMode,
  onSearchChange,
  onSortByChange,
  onWatchFilterChange,
  onFiltersChange,
  onToggleMoreFilters,
  onResetFilters,
  onViewModeChange,
  pageSize,
  onPageSizeChange,
  cardDensity,
  onCardDensityChange,
}: LibraryTopStripProps) {
  const flags = libraryViewFlags(userView);
  const hasActiveFilters = activeFilterCount > 0;

  return (
    <div className="library-top-strip">
      {/* Row 1: search + inline filter dropdowns + layout controls */}
      <div className="library-toolbar-row">

        {/* Center: search + inline filter dropdowns */}
        <div className="library-toolbar-controls">
          {/* Search */}
          <label className="field library-toolbar-search">
            <span className="sr-only">Search by file or creator</span>
            <div className="downloads-search-input">
              <Search size={14} strokeWidth={2} />
              <input
                value={search}
                onChange={(e) => onSearchChange(e.target.value)}
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

          {/* Creator dropdown */}
          <div className="field library-toolbar-select">
            <label className="sr-only" htmlFor="lib-filter-creator">Creator</label>
            <select
              id="lib-filter-creator"
              value={filters.creator}
              onChange={(e) => onFiltersChange({ creator: e.target.value })}
              aria-label="Filter by creator"
            >
              <option value="">All creators</option>
              {facets?.creators.map((c) => (
                <option key={c} value={c}>{c}</option>
              ))}
            </select>
          </div>

          {/* Source dropdown */}
          <div className="field library-toolbar-select">
            <label className="sr-only" htmlFor="lib-filter-source">Source</label>
            <select
              id="lib-filter-source"
              value={filters.source}
              onChange={(e) => onFiltersChange({ source: e.target.value })}
              aria-label="Filter by source"
            >
              <option value="">All sources</option>
              <option value="mods">Mods</option>
              <option value="tray">Tray</option>
            </select>
          </div>
        </div>

        {/* Right: reset + sort + advanced */}
        <div className="library-toolbar-actions">
          {hasActiveFilters && (
            <button
              type="button"
              className="library-reset-btn"
              onClick={onResetFilters}
              title="Clear all filters"
            >
              <RotateCcw size={13} strokeWidth={2} />
              Reset
              <span className="library-active-filter-badge">{activeFilterCount}</span>
            </button>
          )}

          {/* View mode toggle: grid / list */}
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

          {/* Card size slider — fluid, snap to small/medium/large */}
          <div className="library-slider-control" aria-label="Card size">
            <span className="library-slider-icon" aria-hidden="true">
              <svg width="11" height="11" viewBox="0 0 11 11" fill="none">
                <rect x="0.5" y="0.5" width="4" height="4" rx="0.5" stroke="currentColor" strokeWidth="1"/>
                <rect x="6.5" y="0.5" width="4" height="4" rx="0.5" stroke="currentColor" strokeWidth="1"/>
                <rect x="0.5" y="6.5" width="4" height="4" rx="0.5" stroke="currentColor" strokeWidth="1"/>
                <rect x="6.5" y="6.5" width="4" height="4" rx="0.5" stroke="currentColor" strokeWidth="1"/>
              </svg>
            </span>
            <div className="library-slider-track-wrap">
              <input
                type="range"
                min="0" max="100" step="1"
                value={cardDensity === 'small' ? 0 : cardDensity === 'medium' ? 50 : 100}
                onChange={(e) => {
                  const v = Number(e.target.value);
                  if (v < 30)      onCardDensityChange("small");
                  else if (v > 70) onCardDensityChange("large");
                  else             onCardDensityChange("medium");
                }}
                className="library-card-slider"
                aria-label="Card size: small, medium, or large"
              />
              <div className="library-slider-labels" aria-hidden="true">
                <span className={cardDensity === 'small' ? 'is-active' : ''}>S</span>
                <span className={cardDensity === 'medium' ? 'is-active' : ''}>M</span>
                <span className={cardDensity === 'large' ? 'is-active' : ''}>L</span>
              </div>
            </div>
          </div>

          {/* Items per page selector */}
          <div className="library-density-control" aria-label="Items per page">
            <select
              value={pageSize}
              onChange={(e) => onPageSizeChange(Number((e.target as HTMLSelectElement).value))}
              aria-label="Items per page"
              className="library-density-select"
            >
              <option value={50}>50 / page</option>
              <option value={100}>100 / page</option>
              <option value={200}>200 / page</option>
              <option value={500}>500 / page</option>
            </select>
          </div>

          <div className="library-sort-control" aria-label="Sort library">
            <ArrowUpDown size={13} strokeWidth={2} />
            <select
              value={sortBy}
              onChange={(e) => onSortByChange(e.target.value as SortField)}
              aria-label="Sort by"
            >
              {SORT_OPTIONS.map((opt) => (
                <option key={opt.value} value={opt.value}>{opt.label}</option>
              ))}
            </select>
          </div>

          {flags.showAdvancedFilters && (
            <button
              type="button"
              className={`secondary-action${moreFiltersOpen ? " is-active" : ""}`}
              onClick={onToggleMoreFilters}
              aria-expanded={moreFiltersOpen}
              title="More filters"
            >
              <SlidersHorizontal size={14} strokeWidth={2} />
              Filters
              {activeFilterCount > 0 && (
                <span className="library-active-filter-badge">{activeFilterCount}</span>
              )}
            </button>
          )}
        </div>
      </div>

      {/* Row 2: Type chips — primary browsing affordance (replaces kind dropdown) */}
      {facets?.kinds && facets.kinds.length > 0 && (
        <div className="library-type-chips" role="group" aria-label="Filter by type">
          {facets.kinds.map((k) => {
            const cssClass = `type-pill--${k.charAt(0).toLowerCase() + k.slice(1)}`;
            const isActive = filters.kind === k;
            return (
              <button
                key={k}
                type="button"
                className={`library-kind-chip ${cssClass}${isActive ? " is-active" : ""}`}
                onClick={() => onFiltersChange({ kind: isActive ? "" : k })}
                aria-pressed={isActive}
                title={isActive ? `Showing ${friendlyTypeLabel(k)} — click to clear` : `Show ${friendlyTypeLabel(k)}`}
              >
                {friendlyTypeLabel(k)}
              </button>
            );
          })}
        </div>
      )}

      {/* Row 3: Contextual subtype chips — backend already scopes these to the active kind */}
      {filters.kind && facets?.subtypes && facets.subtypes.length > 0 && (
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

      {/* Row 4: Quick chips (watch filter tabs) */}
      <div className="library-quick-chips" role="group" aria-label="Quick filters by status">
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

      {/* Row 3: Summary health strip */}
      {librarySummary ? (
        <div className="library-summary-strip" aria-label="Library health summary">
          <span className="library-summary-stat">
            <strong>{librarySummary.total.toLocaleString()}</strong>
            {" files"}
          </span>
          <span className="library-summary-sep" aria-hidden="true">·</span>
          <span className="library-summary-stat">
            <strong>{librarySummary.tracked.toLocaleString()}</strong>
            {" tracked"}
          </span>
          {librarySummary.hasUpdates > 0 && (
            <>
              <span className="library-summary-sep" aria-hidden="true">·</span>
              <span className="library-summary-stat has-updates">
                <strong>{librarySummary.hasUpdates.toLocaleString()}</strong>
                {" with updates"}
              </span>
            </>
          )}
          {librarySummary.needsReview > 0 && (
            <>
              <span className="library-summary-sep" aria-hidden="true">·</span>
              <span className="library-summary-stat needs-review">
                <strong>{librarySummary.needsReview.toLocaleString()}</strong>
                {" need review"}
              </span>
            </>
          )}
          {librarySummary.duplicates > 0 && (
            <>
              <span className="library-summary-sep" aria-hidden="true">·</span>
              <span className="library-summary-stat has-duplicates">
                <strong>{librarySummary.duplicates.toLocaleString()}</strong>
                {" duplicates"}
              </span>
            </>
          )}
          {librarySummary.disabled > 0 && (
            <>
              <span className="library-summary-sep" aria-hidden="true">·</span>
              <span className="library-summary-stat is-disabled">
                <strong>{librarySummary.disabled.toLocaleString()}</strong>
                {" disabled"}
              </span>
            </>
          )}
        </div>
      ) : null}
    </div>
  );
}
