import { useMemo, memo } from "react";
import { m } from "motion/react";
import { rowHover, rowPress } from "../../lib/motion";
import type { LibraryFileRow, UserView } from "../../lib/types";
import {
  buildLibraryRowModel,
  summarizeLibraryRowFacts,
  summarizeLibraryRowStatus,
  type LibraryRowModel,
} from "./libraryDisplay";
import { LibraryRowThumbnail } from "./LibraryRowThumbnail";

export interface LibraryCollectionTableProps {
  userView: UserView;
  rows: LibraryFileRow[];
  selectedId: number | null;
  selectedIds: Set<number>;
  page: number;
  totalPages: number;
  onSelect: (row: LibraryFileRow) => void;
  onToggleSelect: (id: number) => void;
  onPrevPage: () => void;
  onNextPage: () => void;
  enableSelection?: boolean;
  showPagination?: boolean;
}

// Phase 5y: memoize table so parent state changes don't trigger full row re-renders.
export const LibraryCollectionTable = memo(function LibraryCollectionTable({
  userView,
  rows,
  selectedId,
  selectedIds,
  page,
  totalPages,
  onSelect,
  onToggleSelect,
  onPrevPage,
  onNextPage,
  enableSelection = true,
  showPagination = true,
}: LibraryCollectionTableProps) {
  // Build row models at the component top level so React hook order stays stable
  // when the list changes between empty and populated states.
  const modelCache = useMemo(() => {
    const cache = new Map<number, LibraryRowModel>();
    for (const row of rows) {
      cache.set(row.id, buildLibraryRowModel(row, userView));
    }
    return cache;
  }, [rows, userView]);

  const filteredRows = useMemo(() => {
    const seenBundleNames = new Set<string>();
    const nextRows: LibraryFileRow[] = [];

    for (const row of rows) {
      const model = modelCache.get(row.id);
      if (!model) continue;

      const isGhost = model.kind === "Unknown" && !row.filename;
      const isTrayDup =
        row.bundleName != null &&
        (row.groupedFileCount ?? 0) > 1 &&
        seenBundleNames.has(row.bundleName);

      if (isGhost || isTrayDup) continue;
      if (row.bundleName) seenBundleNames.add(row.bundleName);
      nextRows.push(row);
    }

    return nextRows;
  }, [modelCache, rows]);

  return (
    <>
      <div className="table-scroll library-table-scroll library-list-shell">
        <div className="library-list-header" role="row">
          <div className="library-list-col library-list-col--type" aria-label="Type" />
          <div className="library-list-col library-list-col--thumb" aria-label="Preview" />
          <div className="library-list-col library-list-col--select" aria-label={enableSelection ? "Select" : undefined} />
          <div className="library-list-col library-list-col--name">
            {userView === "beginner" ? "File" : "Mod or file"}
          </div>
          <div className="library-list-col library-list-col--status">Status</div>
          <div className="library-list-col library-list-col--facts">
            {userView === "power" ? "Clues" : "At a glance"}
          </div>
        </div>

        <div className="library-list-body">
          {filteredRows.length ? (
            filteredRows.map((row) => {
              const model = modelCache.get(row.id)!;
              const statusSummary = summarizeLibraryRowStatus(model);
              const factSummary = summarizeLibraryRowFacts(model, userView);
              const isChecked = selectedIds.has(row.id);

              return (
                <m.div
                  key={row.id}
                  data-kind={model.kind}
                  className={[
                    "library-list-row",
                    selectedId === row.id ? "is-selected" : "",
                    isChecked ? "is-multiselected" : "",
                    model.isTray ? "is-tray" : "",
                  ]
                    .filter(Boolean)
                    .join(" ")}
                  onClick={(e) => {
                    if (enableSelection && (e.target as HTMLElement).closest(".library-table-checkbox-col")) {
                      return;
                    }
                    onSelect(row);
                  }}
                  role="button"
                  tabIndex={0}
                  onKeyDown={(event) => {
                    if (event.key === "Enter" || event.key === " ") {
                      if (enableSelection && (event.target as HTMLElement).closest(".library-table-checkbox-col")) {
                        return;
                      }
                      event.preventDefault();
                      onSelect(row);
                    }
                  }}
                  whileHover={rowHover}
                  whileTap={rowPress}
                >
                  <div className="library-list-col library-list-col--type library-type-accent-col">
                    {model.typeColor ? (
                      <div
                        className={`type-accent type-accent--${model.typeColor}`}
                        aria-label={model.typeLabel}
                      />
                    ) : null}
                  </div>

                  {/* Thumbnail / preview cell — real preview when present, honest fallback otherwise */}
                  <div className="library-list-col library-list-col--thumb">
                    <LibraryRowThumbnail model={model} />
                  </div>

                  <div
                    className={`library-list-col library-list-col--select${enableSelection ? " library-table-checkbox-col" : ""}`}
                    onClick={enableSelection
                      ? (e) => {
                          e.stopPropagation();
                          onToggleSelect(row.id);
                        }
                      : undefined}
                  >
                    {enableSelection ? (
                      <input
                        type="checkbox"
                        checked={isChecked}
                        onChange={() => onToggleSelect(row.id)}
                        aria-label={`Select ${model.title}`}
                        className="library-table-checkbox"
                      />
                    ) : null}
                  </div>

                  <div className="library-list-col library-list-col--name library-name-cell">
                    <div className="library-row-title" title={model.title}>{model.displayTitle}</div>
                    <div className="library-row-meta">
                      <span className={`library-type-pill type-pill--${model.typeColor}`}>
                        {model.typeLabel}
                      </span>
                      {model.hasDuplicate && !model.duplicateLabel && (
                        <span className="library-duplicate-badge">Possible duplicate</span>
                      )}
                      {(model.kind === "ScriptMods" ||
                        model.kind === "OverridesAndDefaults") && (
                        <span
                          className={`library-confidence-badge confidence--${model.confidenceLevel}`}
                          title={model.confidenceLabel}
                          aria-label={model.confidenceLabel}
                        >
                          {model.confidenceLevel === "high"
                            ? "✓"
                            : model.confidenceLevel === "medium"
                              ? "⚠"
                              : "?"}
                        </span>
                      )}
                      {model.hasIssues && (
                        <span
                          className="library-issues-badge"
                          title="Has safety notes or parser warnings"
                          aria-label="Has review issues"
                        >
                          ⚑
                        </span>
                      )}
                    </div>
                    {model.identityLabel ? (
                      <div className="library-row-identity" title={model.identityLabel}>{model.identityLabel}</div>
                    ) : null}
                  </div>

                  <div className="library-list-col library-list-col--status library-status-cell">
                    <div className="library-status-pills">
                      {statusSummary.visible.map((pill) => (
                        <span
                          key={pill.key}
                          className={`library-health-pill library-health-pill--${pill.key} is-${pill.tone}`}
                        >
                          {pill.label}
                        </span>
                      ))}
                    </div>
                  </div>

                  <div className="library-list-col library-list-col--facts library-facts-cell">
                    <div className="library-row-facts">
                      {factSummary.visible.map((fact) => (
                        <span key={fact} className="library-row-fact">
                          {fact}
                        </span>
                      ))}
                    </div>
                    {model.colorSwatches.length > 0 && (
                      <div className="library-row-swatches" aria-label="Color hints">
                        {model.colorSwatches.map((hex, i) => (
                          <div
                            key={i}
                            className="library-row-swatch"
                            style={{ background: hex }}
                            title={hex}
                          />
                        ))}
                      </div>
                    )}
                  </div>
                </m.div>
              );
            })
          ) : (
            <div className="library-list-empty">
              {userView === "beginner"
                ? "No files match these filters. Try clearing search, choosing another type, or showing all signals."
                : "No indexed files match the current filters. Clear search, choose another type, or show all signals."}
            </div>
          )}
        </div>
      </div>

      {showPagination ? (
        <>
          <div className="table-footer">
            <button
              type="button"
              className="secondary-action"
              onClick={onPrevPage}
              disabled={page === 0}
            >
              Previous
            </button>
            <div className="table-page-label">
              Page {Math.min(page + 1, totalPages)} of {totalPages}
            </div>
            <button
              type="button"
              className="secondary-action"
              onClick={onNextPage}
              disabled={page + 1 >= totalPages}
            >
              Next
            </button>
          </div>

          <div className="table-pagination-bar" />
        </>
      ) : null}
    </>
  );
});
