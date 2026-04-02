import { PanelLeftClose } from "lucide-react";
import { friendlyTypeLabel } from "../../lib/uiLanguage";
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

const WATCH_FILTER_OPTIONS: Array<{ value: LibraryWatchFilter; label: string }> = [
  { value: "all", label: "All" },
  { value: "has_updates", label: "Updates" },
  { value: "needs_attention", label: "Attention" },
  { value: "not_tracked", label: "Not tracked" },
  { value: "duplicates", label: "Dupes" },
];

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
  watchFilter,
  activeFilterCount,
  resultsCount,
  isCollapsed,
  moreFiltersOpen,
  librarySummary,
  onToggleCollapsed,
  onFiltersChange,
  onResetFilters,
  onToggleMoreFilters,
  onWatchFilterChange,
}: LibraryFilterRailProps) {
  if (isCollapsed) {
    return null;
  }

  const flags = libraryViewFlags(userView);

  // Build subtype options — include only subtypes that exist for the selected kind
  // Subtype options — backend returns all subtypes in one flat list with no kind-scoped filtering.
  // Showing all subtypes is honest given the backend constraint; the kind dropdown acts as the
  // primary filter and subtype narrows within those results. Filtering to kind-scoped subtypes
  // would require backend support or client-side post-filtering of all indexed files.
  const subtypeOptions = (() => {
    if (!facets?.subtypes?.length) return null;
    return facets.subtypes;
  })();

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

      {/* ── Watch status — quick-access pills (always shown) ── */}
      <div className="filter-section">
        <div className="section-label">Status</div>
        <div className="watch-filter-row">
          {WATCH_FILTER_OPTIONS.map((opt) => (
            <button
              key={opt.value}
              type="button"
              className={`watch-filter-chip${
                watchFilter === opt.value ? " is-active" : ""
              }`}
              onClick={() => onWatchFilterChange(opt.value)}
              aria-pressed={watchFilter === opt.value}
            >
              <strong>{opt.label}</strong>
            </button>
          ))}
        </div>
      </div>

      {/* ── Body — type, creator, source ── */}
      <div className="library-filter-rail-body">
        <label className="field">
          <span>Type</span>
          <select
            aria-label="Type"
            value={filters.kind}
            onChange={(event) => onFiltersChange({ kind: event.target.value })}
          >
            <option value="">All types</option>
            {facets?.kinds.map((item) => (
              <option key={item} value={item}>
                {friendlyTypeLabel(item)}
              </option>
            ))}
          </select>
        </label>

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

        {/* Subtype — only show if subtypes exist in the facet data */}
        {subtypeOptions ? (
          <label className="field">
            <span>Subtype</span>
            <select
              aria-label="Subtype"
              value={filters.subtype}
              onChange={(event) =>
                onFiltersChange({ subtype: event.target.value })
              }
            >
              <option value="">All subtypes</option>
              {subtypeOptions.map((item) => (
                <option key={item} value={item}>
                  {item}
                </option>
              ))}
            </select>
          </label>
        ) : null}

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
