import { m } from "motion/react";
import { rowHover, rowPress, stagedListItem } from "../../lib/motion";
import type { LibraryFileRow, UserView } from "../../lib/types";
import { buildLibraryRowModel } from "./libraryDisplay";

interface LibraryCollectionTableProps {
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
}

export function LibraryCollectionTable({
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
}: LibraryCollectionTableProps) {
  return (
    <>
      <div className="table-scroll library-table-scroll library-list-shell">
        <div className="library-list-header" role="row">
          <div className="library-list-col library-list-col--type" aria-label="Type" />
          <div className="library-list-col library-list-col--select" aria-label="Select" />
          <div className="library-list-col library-list-col--name">
            {userView === "beginner" ? "File" : "Mod or file"}
          </div>
          <div className="library-list-col library-list-col--status">Status</div>
          <div className="library-list-col library-list-col--facts">
            {userView === "power" ? "Clues" : "At a glance"}
          </div>
        </div>

        <div className="library-list-body">
          {rows.length ? (
            rows.map((row, index) => {
              const model = buildLibraryRowModel(row, userView);
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
                    if ((e.target as HTMLElement).closest(".library-table-checkbox-col")) {
                      return;
                    }
                    onSelect(row);
                  }}
                  role="button"
                  tabIndex={0}
                  onKeyDown={(event) => {
                    if (event.key === "Enter" || event.key === " ") {
                      if ((event.target as HTMLElement).closest(".library-table-checkbox-col")) {
                        return;
                      }
                      event.preventDefault();
                      onSelect(row);
                    }
                  }}
                  whileHover={rowHover}
                  whileTap={rowPress}
                  {...stagedListItem(index)}
                >
                  <div className="library-list-col library-list-col--type library-type-accent-col">
                    {model.typeColor ? (
                      <div
                        className={`type-accent type-accent--${model.typeColor}`}
                        aria-label={model.typeLabel}
                      />
                    ) : null}
                  </div>

                  <div
                    className="library-list-col library-list-col--select library-table-checkbox-col"
                    onClick={(e) => {
                      e.stopPropagation();
                      onToggleSelect(row.id);
                    }}
                  >
                    <input
                      type="checkbox"
                      checked={isChecked}
                      onChange={() => onToggleSelect(row.id)}
                      aria-label={`Select ${model.title}`}
                      className="library-table-checkbox"
                    />
                  </div>

                  <div className="library-list-col library-list-col--name library-name-cell">
                    <div className="library-row-title" title={model.title}>{model.title}</div>
                    {model.identityLabel ? (
                      <div className="library-row-identity" title={model.identityLabel}>{model.identityLabel}</div>
                    ) : null}
                    <div className="library-row-meta">
                      <span className={`library-type-pill type-pill--${model.typeColor}`}>
                        {model.typeLabel}
                      </span>
                      {model.hasDuplicate && !model.duplicateLabel && (
                        <span className="library-duplicate-badge">Duplicate</span>
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
                  </div>

                  <div className="library-list-col library-list-col--status library-status-cell">
                    <div className="library-status-pills">
                      <span className={`library-health-pill is-${model.watchStatusTone}`}>
                        {model.watchStatusLabel}
                      </span>
                      {model.healthTone && (
                        <span className={`library-health-pill is-${model.healthTone}`}>
                          {model.healthLabel}
                        </span>
                      )}
                      {model.duplicateLabel && (
                        <span className={`library-health-pill is-${model.duplicateTone}`}>
                          {model.duplicateLabel}
                        </span>
                      )}
                    </div>
                  </div>

                  <div className="library-list-col library-list-col--facts library-facts-cell">
                    <div className="library-row-facts">
                      {model.supportingFacts.map((fact) => (
                        <span key={fact} className="library-row-fact">
                          {fact}
                        </span>
                      ))}
                    </div>
                  </div>
                </m.div>
              );
            })
          ) : (
            <div className="library-list-empty">
              {userView === "beginner"
                ? "Nothing matches these filters right now."
                : "No indexed files match the current filters."}
            </div>
          )}
        </div>
      </div>

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
  );
}
