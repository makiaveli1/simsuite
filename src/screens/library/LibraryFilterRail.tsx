import { PanelLeftClose } from "lucide-react";
import {
  type LibraryFacets,
  type LibraryWatchFilter,
  type LibrarySummary,
  type UserView,
} from "../../lib/types";
import { libraryViewFlags } from "./libraryDisplay";

export interface LibraryFilterValues {
  kind: string;
  creator: string;
  source: string;
  subtype: string;
  minConfidence: string;
}

interface LibraryFilterRailProps {
  userView: UserView;
  facets: LibraryFacets | null;
  filters: LibraryFilterValues;
  watchFilter: LibraryWatchFilter;
  activeFilterCount: number;
  resultsCount: number;
  isCollapsed: boolean;
  moreFiltersOpen: boolean;
  librarySummary: LibrarySummary | null;
  onToggleCollapsed: () => void;
  onFiltersChange: (next: Partial<LibraryFilterValues>) => void;
  onResetFilters: () => void;
  onToggleMoreFilters: () => void;
  onWatchFilterChange: (value: LibraryWatchFilter) => void;
}

const CONFIDENCE_OPTIONS = [
  { value: "", label: "Any" },
  { value: "high", label: "High" },
  { value: "medium", label: "Med" },
  { value: "low", label: "Low" },
] as const;

export function LibraryFilterRail({
  userView,
  facets,
  filters,
  watchFilter: _watchFilter,
  activeFilterCount,
  resultsCount,
  isCollapsed,
  moreFiltersOpen,
  librarySummary,
  onToggleCollapsed,
  onFiltersChange,
  onResetFilters,
  onToggleMoreFilters,
  onWatchFilterChange: _onWatchFilterChange,
}: LibraryFilterRailProps) {
  if (isCollapsed) {
    return null;
  }

  const flags = libraryViewFlags(userView);
  const selectedConfidence =
    filters.minConfidence === "" || !filters.minConfidence
      ? ""
      : filters.minConfidence;

  return (
    <div className="library-filter-rail">
      {/* ── Header ── */}
      <div className="library-filter-rail-header">
        <div>
          <p className="eyebrow">Narrow library</p>
          <h2>Filters</h2>
        </div>
        <button
          type="button"
          className="icon-action"
          onClick={onToggleCollapsed}
          aria-label="Hide filters"
          title="Hide filters"
        >
          <PanelLeftClose size={16} strokeWidth={2} />
        </button>
      </div>

      {/* ── Metrics bar ── */}
      <div className="library-filter-rail-metrics">
        <div className="library-filter-rail-metric">
          <span>Visible</span>
          <strong>{resultsCount.toLocaleString()}</strong>
        </div>
        <div className="library-filter-rail-metric">
          <span>Filters on</span>
          <strong>{activeFilterCount}</strong>
        </div>
      </div>

      {/* ── Refinement body ── */}
      {/*
        Type and subtype live as color-coded chips in the TopStrip — the primary
        browsing affordance. This rail is for deep refinement.

        NOTE: In Seasoned/Creator view the type chips row is prominent in the TopStrip.
        Subtype chips appear contextually when a type is selected.
        The dropdowns below are kept as a secondary access path for discoverability
        and keyboard accessibility — but the primary type experience is the chip row.
      */}
      <div className="library-filter-rail-body">
        {/* Creator filter */}
        <label className="field">
          <span>Creator</span>
          <select
            aria-label="Creator"
            value={filters.creator}
            onChange={(event) =>
              onFiltersChange({ creator: event.target.value })
            }
          >
            <option value="">All creators</option>
            {facets?.creators.map((item) => (
              <option key={item} value={item}>
                {item}
              </option>
            ))}
          </select>
        </label>

        {/* Confidence — quick-select buttons (seasoned+) */}
        {userView !== "beginner" && (
          <div className="field">
            <span>Confidence</span>
            <div className="confidence-segmented">
              {CONFIDENCE_OPTIONS.map((opt) => (
                <button
                  key={opt.value}
                  type="button"
                  className={`confidence-seg-btn${
                    selectedConfidence === opt.value ? " is-active" : ""
                  }`}
                  onClick={() => onFiltersChange({ minConfidence: opt.value })}
                  aria-pressed={selectedConfidence === opt.value}
                >
                  {opt.label}
                </button>
              ))}
            </div>
          </div>
        )}

        {/* Root/source — power only */}
        {flags.showRootFacts ? (
          <label className="field">
            <span>{userView === "power" ? "Root" : "Folder"}</span>
            <select
              aria-label={userView === "power" ? "Root" : "Folder"}
              value={filters.source}
              onChange={(event) =>
                onFiltersChange({ source: event.target.value })
              }
            >
              <option value="">All roots</option>
              {facets?.sources.map((item) => (
                <option key={item} value={item}>
                  {item === "tray" ? "Tray" : "Mods"}
                </option>
              ))}
            </select>
          </label>
        ) : null}
      </div>

      {/* ── Footer ── */}
      <div className="library-filter-rail-footer">
        <button
          type="button"
          className="secondary-action"
          onClick={onResetFilters}
          disabled={activeFilterCount === 0}
        >
          Reset
        </button>
        <span className="ghost-chip">
          {facets?.creators.length ?? 0} creators
        </span>
        {librarySummary ? (
          <span className="ghost-chip">
            {librarySummary.total.toLocaleString()} files
          </span>
        ) : null}
      </div>
    </div>
  );
}
