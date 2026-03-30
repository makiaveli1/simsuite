import { ArrowUpDown, ListFilter, PanelLeftOpen, Search } from "lucide-react";
import type { LibrarySortField, LibrarySummary, UserView } from "../../lib/types";
import { libraryViewFlags } from "./libraryDisplay";

type WatchFilter = "all" | "has_updates" | "needs_attention" | "not_tracked" | "duplicates";

const WATCH_FILTER_OPTIONS: { value: WatchFilter; label: string }[] = [
  { value: "all", label: "All" },
  { value: "has_updates", label: "Has Updates" },
  { value: "needs_attention", label: "Needs review" },
  { value: "not_tracked", label: "Not Tracked" },
  { value: "duplicates", label: "Duplicates" },
];

const SORT_OPTIONS: { value: LibrarySortField; label: string }[] = [
  { value: "name", label: "Name" },
  { value: "creator", label: "Creator" },
  { value: "recently_modified", label: "Recently Modified" },
  { value: "has_updates_first", label: "Has updates first" },
];

interface LibraryTopStripProps {
  userView: UserView;
  search: string;
  shownCount: number;
  totalCount: number;
  activeFilterCount: number;
  filtersCollapsed: boolean;
  moreFiltersOpen: boolean;
  watchFilter: WatchFilter;
  sortBy: LibrarySortField;
  librarySummary: LibrarySummary | null;
  onSearchChange: (value: string) => void;
  onToggleFiltersRail: () => void;
  onToggleMoreFilters: () => void;
  onWatchFilterChange: (value: WatchFilter) => void;
  onSortByChange: (value: LibrarySortField) => void;
}

export function LibraryTopStrip({
  userView,
  search,
  shownCount,
  totalCount,
  activeFilterCount,
  filtersCollapsed,
  moreFiltersOpen,
  watchFilter,
  sortBy,
  librarySummary,
  onSearchChange,
  onToggleFiltersRail,
  onToggleMoreFilters,
  onWatchFilterChange,
  onSortByChange,
}: LibraryTopStripProps) {
  const flags = libraryViewFlags(userView);

  return (
    <div className="library-top-strip">
      <div className="library-top-strip-metrics" aria-label="Library summary">
        <div className="library-top-strip-metric">
          <span>Shown</span>
          <strong>{shownCount.toLocaleString()}</strong>
        </div>
        <div className="library-top-strip-metric">
          <span>In library</span>
          <strong>{totalCount.toLocaleString()}</strong>
        </div>
        <div className="library-top-strip-metric">
          <span>Filters</span>
          <strong>{activeFilterCount}</strong>
        </div>
      </div>

      <div className="library-top-strip-tools">
        <label className="field library-top-strip-search">
          <span className="sr-only">Search library</span>
          <div className="downloads-search-input">
            <Search size={14} strokeWidth={2} />
            <input
              value={search}
              onChange={(event) => onSearchChange(event.target.value)}
              placeholder="Search by file or creator"
              aria-label="Search library"
            />
          </div>
        </label>

        {flags.showAdvancedFilters ? (
          <button
            type="button"
            className={`secondary-action${moreFiltersOpen ? " is-active" : ""}`}
            onClick={onToggleMoreFilters}
            aria-expanded={moreFiltersOpen}
          >
            <ListFilter size={14} strokeWidth={2} />
            More filters
          </button>
        ) : null}

        {filtersCollapsed ? (
          <button
            type="button"
            className="secondary-action"
            onClick={onToggleFiltersRail}
          >
            <PanelLeftOpen size={14} strokeWidth={2} />
            Show filters
          </button>
        ) : null}

        <div className="library-sort-control" aria-label="Sort library">
          <ArrowUpDown size={13} strokeWidth={2} />
          <select
            value={sortBy}
            onChange={(e) => onSortByChange(e.target.value as LibrarySortField)}
            aria-label="Sort by"
          >
            {SORT_OPTIONS.map((opt) => (
              <option key={opt.value} value={opt.value}>
                {opt.label}
              </option>
            ))}
          </select>
        </div>
      </div>

      <div className="library-quick-chips" role="group" aria-label="Quick filters">
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
