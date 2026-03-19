import { PanelLeftClose } from "lucide-react";
import { friendlyTypeLabel } from "../../lib/uiLanguage";
import type { LibraryFacets, UserView } from "../../lib/types";
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
  activeFilterCount: number;
  isCollapsed: boolean;
  onToggleCollapsed: () => void;
  onFilterChange: (next: Partial<LibraryFilterValues>) => void;
  onReset: () => void;
  onOpenMoreFilters: () => void;
}

export function LibraryFilterRail({
  userView,
  facets,
  filters,
  activeFilterCount,
  isCollapsed,
  onToggleCollapsed,
  onFilterChange,
  onReset,
}: LibraryFilterRailProps) {
  const flags = libraryViewFlags(userView);

  if (isCollapsed) {
    return null;
  }

  return (
    <div className="library-filter-rail">
      <div className="library-filter-rail-header">
        <div>
          <p className="eyebrow">Narrow library</p>
          <h2>Filters</h2>
          <p className="library-filter-rail-copy">
            Keep only the filters you actually need here. The quick counts and search stay up top.
          </p>
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

      <div className="library-filter-rail-body">
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

        {flags.showRootFacts ? (
          <label className="field">
            <span>{userView === "power" ? "Root" : "Folder"}</span>
            <select
              aria-label={userView === "power" ? "Root" : "Folder"}
              value={filters.source}
              onChange={(event) => onFilterChange({ source: event.target.value })}
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

      <div className="library-filter-rail-footer">
        <button
          type="button"
          className="secondary-action"
          onClick={onReset}
          disabled={activeFilterCount === 0}
        >
          Reset filters
        </button>
        <span className="ghost-chip">
          {facets?.creators.length ?? 0} creators
        </span>
        {activeFilterCount > 0 ? (
          <span className="ghost-chip">{activeFilterCount} active</span>
        ) : null}
      </div>
    </div>
  );
}
