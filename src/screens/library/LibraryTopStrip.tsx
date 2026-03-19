import { ListFilter, RotateCcw, Search } from "lucide-react";
import type { LibraryFacets, UserView } from "../../lib/types";
import { libraryViewFlags } from "./libraryDisplay";

export interface LibraryFilterValues {
  kind: string;
  creator: string;
  source: string;
  subtype: string;
  minConfidence: string;
}

interface LibraryTopStripProps {
  userView: UserView;
  facets: LibraryFacets | null;
  search: string;
  filters: LibraryFilterValues;
  shownCount: number;
  totalCount: number;
  activeFilterCount: number;
  moreFiltersOpen: boolean;
  onSearchChange: (value: string) => void;
  onFilterChange: (next: Partial<LibraryFilterValues>) => void;
  onToggleMoreFilters: () => void;
  onReset: () => void;
}

export function LibraryTopStrip({
  userView,
  facets,
  search,
  filters,
  shownCount,
  totalCount,
  activeFilterCount,
  moreFiltersOpen,
  onSearchChange,
  onFilterChange,
  onToggleMoreFilters,
  onReset,
}: LibraryTopStripProps) {
  const flags = libraryViewFlags(userView);
  const showExtendedRow = moreFiltersOpen;

  return (
    <div className="library-top-strip">
      <div className="library-top-strip-summary" aria-label="Library summary">
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

      <div className="library-top-strip-main">
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

        <label className="field">
          <span>Type</span>
          <select
            aria-label="Type"
            value={filters.kind}
            onChange={(event) => onFilterChange({ kind: event.target.value })}
          >
            <option value="">All types</option>
            {facets?.kinds.map((item) => (
              <option key={item} value={item}>
                {item}
              </option>
            ))}
          </select>
        </label>

        <label className="field">
          <span>Creator</span>
          <select
            aria-label="Creator"
            value={filters.creator}
            onChange={(event) => onFilterChange({ creator: event.target.value })}
          >
            <option value="">All creators</option>
            {facets?.creators.map((item) => (
              <option key={item} value={item}>
                {item}
              </option>
            ))}
          </select>
        </label>

        <label className="field">
          <span>Folder</span>
          <select
            aria-label="Folder"
            value={filters.source}
            onChange={(event) => onFilterChange({ source: event.target.value })}
          >
            <option value="">All roots</option>
            {facets?.sources.map((item) => (
              <option key={item} value={item}>
                {item === "mods" ? "Mods" : item === "tray" ? "Tray" : item}
              </option>
            ))}
          </select>
        </label>

        <div className="library-top-strip-actions">
          <button
            type="button"
            className={`secondary-action${moreFiltersOpen ? " is-active" : ""}`}
            onClick={onToggleMoreFilters}
            aria-expanded={moreFiltersOpen}
          >
            <ListFilter size={14} strokeWidth={2} />
            More filters
          </button>
          {activeFilterCount > 0 && !showExtendedRow ? (
            <button type="button" className="secondary-action" onClick={onReset}>
              <RotateCcw size={14} strokeWidth={2} />
              Reset filters
            </button>
          ) : null}
        </div>
      </div>

      {showExtendedRow ? (
        <div className="library-top-strip-more">
          {flags.showAdvancedFilters ? (
            <label className="field">
              <span>Subtype</span>
              <select
                aria-label="Subtype"
                value={filters.subtype}
                onChange={(event) => onFilterChange({ subtype: event.target.value })}
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
              aria-label="Confidence"
              value={filters.minConfidence}
              onChange={(event) => onFilterChange({ minConfidence: event.target.value })}
            >
              <option value="">Any match</option>
              <option value="0.35">35%+</option>
              <option value="0.55">55%+</option>
              <option value="0.75">75%+</option>
            </select>
          </label>

          {activeFilterCount > 0 ? (
            <button
              type="button"
              className="secondary-action library-top-strip-reset-inline"
              onClick={onReset}
            >
              <RotateCcw size={14} strokeWidth={2} />
              Reset filters
            </button>
          ) : null}
        </div>
      ) : null}
    </div>
  );
}
