import { m } from "motion/react";
import { rowHover, rowPress, stagedListItem } from "../../lib/motion";
import type { LibraryFileRow, UserView } from "../../lib/types";
import { buildLibraryCardModel, type LibraryCardModel } from "./libraryDisplay";

interface LibraryThumbnailGridProps {
  userView: UserView;
  rows: LibraryFileRow[];
  selectedId: number | null;
  onSelect: (row: LibraryFileRow) => void;
  onPrevPage: () => void;
  onNextPage: () => void;
  page: number;
  totalPages: number;
}

export function LibraryThumbnailGrid({
  userView,
  rows,
  selectedId,
  onSelect,
  onPrevPage,
  onNextPage,
  page,
  totalPages,
}: LibraryThumbnailGridProps) {
  return (
    <>
      <div className="library-grid-scroll library-list-shell">
        <div className="library-grid">
          {rows.length ? (
            rows.map((row, index) => {
              const model = buildLibraryCardModel(row, userView);
              const isSelected = selectedId === row.id;

              return (
                <m.div
                  key={row.id}
                  data-kind={model.kind}
                  className={[
                    "library-card",
                    `library-card--${model.typeColor}`,
                    isSelected ? "is-selected" : "",
                    model.isTray ? "is-tray" : "",
                    model.hasIssues ? "has-issues" : "",
                  ]
                    .filter(Boolean)
                    .join(" ")}
                  onClick={() => onSelect(row)}
                  role="button"
                  tabIndex={0}
                  onKeyDown={(event) => {
                    if (event.key === "Enter" || event.key === " ") {
                      event.preventDefault();
                      onSelect(row);
                    }
                  }}
                  whileHover={rowHover}
                  whileTap={rowPress}
                  {...stagedListItem(index)}
                >
                  {/* Card header: type pill + watch status */}
                  <div className="library-card-header">
                    <span className={`library-type-pill type-pill--${model.typeColor}`}>
                      {model.typeLabel}
                    </span>
                    <div className="library-card-header-right">
                      {model.hasIssues && (
                        <span className="library-issues-badge" title="Has safety notes or parser warnings">
                          ⚑
                        </span>
                      )}
                      {model.hasDuplicate && (
                        <span className="library-duplicate-badge">Duplicate</span>
                      )}
                      <span className={`library-health-pill is-${model.watchStatusTone}`}>
                        {model.watchStatusLabel}
                      </span>
                    </div>
                  </div>

                  {/* Content preview — the "swatch" substitute */}
                  <div className="library-card-content-preview">
                    {model.embeddedNames.length > 0 ? (
                      <div className="library-card-names">
                        {model.embeddedNames.map((name) => (
                          <span key={name} className="library-card-name-chip">
                            {name}
                          </span>
                        ))}
                        {model.totalEmbeddedNames > model.embeddedNames.length && (
                          <span className="library-card-name-overflow">
                            +{model.totalEmbeddedNames - model.embeddedNames.length} more
                          </span>
                        )}
                      </div>
                    ) : model.resourceSummary ? (
                      <div className="library-card-resource-summary">
                        {model.resourceSummary}
                      </div>
                    ) : (
                      <div className="library-card-empty-preview">
                        No content preview available
                      </div>
                    )}
                  </div>

                  {/* Card footer: title + creator + version */}
                  <div className="library-card-footer">
                    <div className="library-card-title" title={model.title}>
                      {model.title}
                    </div>
                    <div className="library-card-meta">
                      <span className="library-card-creator">{model.creatorLabel}</span>
                      {model.versionLabel && (
                        <span className="library-card-version">{model.versionLabel}</span>
                      )}
                    </div>
                    {model.isTray && (
                      <div className="library-card-tray-badge">tray · disabled</div>
                    )}
                  </div>

                  {/* Confidence bar at bottom of card */}
                  <div
                    className={`library-card-confidence-bar confidence-bar--${model.confidenceLevel}`}
                    aria-label={`${model.confidenceLevel} confidence`}
                  />
                </m.div>
              );
            })
          ) : (
            <div className="library-grid-empty">
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
