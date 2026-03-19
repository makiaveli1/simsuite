import { FolderSearch, ListFilter, Search } from "lucide-react";
import { AnimatePresence, m } from "motion/react";
import { downloadsSheetTransition } from "../../lib/motion";
import { screenHelperLine } from "../../lib/uiLanguage";
import type { DownloadQueueLane, UserView } from "../../lib/types";
import {
  DOWNLOADS_LANE_SUMMARY_ORDER,
  type DownloadsLaneCounts,
  downloadsLaneHint,
  downloadsLaneLabel,
  viewModeDownloadsFlags,
} from "./downloadsDisplay";

interface DownloadsRailProps {
  userView: UserView;
  watcherLabel: string;
  watchedPath: string | null;
  lastCheckLabel: string;
  activeItemsLabel: string;
  currentItemLabel?: string | null;
  activeLane: DownloadQueueLane;
  laneCounts: DownloadsLaneCounts;
  search: string;
  statusFilter: string;
  selectedPreset: string;
  presetOptions: string[];
  filtersOpen?: boolean;
  filterLocked?: boolean;
  onLaneChange: (lane: DownloadQueueLane) => void;
  onSearchChange: (value: string) => void;
  onStatusFilterChange: (value: string) => void;
  onPresetChange: (value: string) => void;
  onToggleFilters: () => void;
}

export function DownloadsRail({
  userView,
  watcherLabel,
  watchedPath,
  lastCheckLabel,
  activeItemsLabel,
  currentItemLabel,
  activeLane,
  laneCounts,
  search,
  statusFilter,
  selectedPreset,
  presetOptions,
  filtersOpen = false,
  filterLocked = false,
  onLaneChange,
  onSearchChange,
  onStatusFilterChange,
  onPresetChange,
  onToggleFilters,
}: DownloadsRailProps) {
  const flags = viewModeDownloadsFlags(userView);
  const showFiltersToggle = userView !== "beginner";

  return (
    <div className="downloads-rail">
      <div className="workbench-header">
        <div>
          <p className="eyebrow">Workspace</p>
          <h1 className="downloads-rail-title">Downloads</h1>
          <p className="downloads-rail-copy">{screenHelperLine("downloads", userView)}</p>
        </div>
        <span className="confidence-badge neutral">{watcherLabel}</span>
      </div>

      <div className="downloads-rail-section">
        <div className="section-label">Watch folder</div>
        <div className="downloads-rail-card downloads-watch-card">
          <div className="path-card">
            {watchedPath ?? "No downloads folder set"}
          </div>
          <div className="downloads-watch-meta">
            <span className="ghost-chip">{activeItemsLabel}</span>
            {currentItemLabel ? <span className="ghost-chip">{currentItemLabel}</span> : null}
            <span className="ghost-chip">{lastCheckLabel}</span>
          </div>
        </div>
      </div>

      <div className="downloads-rail-section">
        <div className="section-label">Queue lanes</div>
        <div className="downloads-lane-picker" role="list" aria-label="Downloads lanes">
          {DOWNLOADS_LANE_SUMMARY_ORDER.map((lane) => {
            const isActive = lane === activeLane;
            return (
              <button
                key={lane}
                type="button"
                className={`downloads-lane-button${isActive ? " is-active" : ""}`}
                onClick={() => onLaneChange(lane)}
              >
                <span className="downloads-lane-button-main">
                  <span className="downloads-lane-button-title">
                    {downloadsLaneLabel(lane, userView)}
                  </span>
                  <span className="downloads-lane-button-count">
                    {laneCounts[lane].toLocaleString()}
                  </span>
                </span>
                {isActive ? (
                  <span className="downloads-lane-button-hint">
                    {downloadsLaneHint(lane, userView)}
                  </span>
                ) : null}
              </button>
            );
          })}
        </div>
      </div>

      <div className="downloads-rail-section">
        <div className="section-label">Search</div>
        <div className="downloads-rail-card downloads-rail-search-card">
          <label className="field">
            <span className="sr-only">Search downloads</span>
            <div className="downloads-search-input">
              <Search size={14} strokeWidth={2} />
              <input
                value={search}
                onChange={(event) => onSearchChange(event.target.value)}
                placeholder="Archive, file, or creator"
              />
            </div>
          </label>

          {!showFiltersToggle ? (
            <label className="field">
              <span>{userView === "beginner" ? "Show" : "Status"}</span>
              <select
                value={statusFilter}
                onChange={(event) => onStatusFilterChange(event.target.value)}
              >
                <option value="">All items</option>
                <option value="ready">Ready</option>
                <option value="partial">Partial</option>
                <option value="needs_review">Needs review</option>
                <option value="applied">Applied</option>
                <option value="error">Error</option>
                <option value="ignored">Ignored</option>
              </select>
            </label>
          ) : null}
        </div>
      </div>

      {showFiltersToggle ? (
        <div className="downloads-rail-section downloads-rail-section-has-popover">
          <div className="downloads-rail-section-header">
            <div className="section-label">Filters</div>
            <button
              type="button"
              className="secondary-action downloads-filters-toggle"
              onClick={onToggleFilters}
              aria-expanded={filtersOpen}
            >
              <ListFilter size={14} strokeWidth={2} />
              More filters
            </button>
          </div>

          <AnimatePresence>
            {filtersOpen ? (
              <m.div
                className="downloads-filter-popover downloads-rail-card downloads-filter-card"
                initial={{ opacity: 0, y: -8, scale: 0.985 }}
                animate={{ opacity: 1, y: 0, scale: 1 }}
                exit={{ opacity: 0, y: -6, scale: 0.985 }}
                transition={downloadsSheetTransition}
              >
                <div className="downloads-filter-popover-header">
                  <div>
                    <p className="eyebrow">
                      {userView === "power" ? "Proof filters" : "Filters"}
                    </p>
                    <strong>
                      {userView === "power"
                        ? "Keep the queue short without crowding the rail"
                        : "Tuck the extra sorting choices away until you need them"}
                    </strong>
                  </div>
                </div>

                <div className="downloads-rail-filters">
                  <label className="field">
                    <span>Status</span>
                    <select
                      value={statusFilter}
                      onChange={(event) => onStatusFilterChange(event.target.value)}
                    >
                      <option value="">All items</option>
                      <option value="ready">Ready</option>
                      <option value="partial">Partial</option>
                      <option value="needs_review">Needs review</option>
                      <option value="applied">Applied</option>
                      <option value="error">Error</option>
                      <option value="ignored">Ignored</option>
                    </select>
                  </label>

                  <label className="field">
                    <span>{userView === "power" ? "Rule set" : "Tidy style"}</span>
                    <select
                      value={selectedPreset}
                      onChange={(event) => onPresetChange(event.target.value)}
                      disabled={filterLocked}
                    >
                      {presetOptions.map((preset) => (
                        <option key={preset} value={preset}>
                          {preset}
                        </option>
                      ))}
                    </select>
                  </label>
                </div>

                {filterLocked ? (
                  <p className="downloads-rail-note">
                    This batch has its own install rules, so the tidy style stays locked.
                  </p>
                ) : userView === "power" ? (
                  <p className="downloads-rail-note">
                    Creator view keeps the deeper sorting choices one click away instead of
                    permanently open.
                  </p>
                ) : flags.showAdvancedFiltersByDefault ? (
                  <p className="downloads-rail-note">
                    Keep the extra filters nearby when you need a faster proof pass.
                  </p>
                ) : null}
              </m.div>
            ) : null}
          </AnimatePresence>
        </div>
      ) : null}

      <div className="downloads-rail-section downloads-rail-section-muted">
        <div className="downloads-rail-tip">
          <FolderSearch size={14} strokeWidth={2} />
          <span>New downloads stay here until SimSuite is confident they are safe.</span>
        </div>
      </div>
    </div>
  );
}
