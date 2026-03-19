import { ListFilter, PanelLeftOpen, Search } from "lucide-react";
import type { UserView } from "../../lib/types";
import { libraryViewFlags } from "./libraryDisplay";

interface LibraryTopStripProps {
  userView: UserView;
  search: string;
  shownCount: number;
  totalCount: number;
  activeFilterCount: number;
  filtersCollapsed: boolean;
  moreFiltersOpen: boolean;
  onSearchChange: (value: string) => void;
  onToggleFiltersRail: () => void;
  onToggleMoreFilters: () => void;
}

export function LibraryTopStrip({
  userView,
  search,
  shownCount,
  totalCount,
  activeFilterCount,
  filtersCollapsed,
  moreFiltersOpen,
  onSearchChange,
  onToggleFiltersRail,
  onToggleMoreFilters,
}: LibraryTopStripProps) {
  const flags = libraryViewFlags(userView);

  return (
    <div className="library-top-strip">
      <div className="library-top-strip-metrics" aria-label="Library summary">
        <span className="library-top-strip-chip">
          <strong>{shownCount.toLocaleString()}</strong>
          <span>shown</span>
        </span>
        <span className="library-top-strip-chip">
          <strong>{totalCount.toLocaleString()}</strong>
          <span>in library</span>
        </span>
        <span className="library-top-strip-chip">
          <strong>{activeFilterCount}</strong>
          <span>filters</span>
        </span>
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
      </div>
    </div>
  );
}
