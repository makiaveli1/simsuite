import { m } from "motion/react";
import { rowHover, rowPress, stagedListItem } from "../../lib/motion";
import type { LibraryFileRow, UserView } from "../../lib/types";
import { buildLibraryRowModel } from "./libraryDisplay";

interface LibraryCollectionTableProps {
  userView: UserView;
  rows: LibraryFileRow[];
  selectedId: number | null;
  page: number;
  totalPages: number;
  onSelect: (row: LibraryFileRow) => void;
  onPrevPage: () => void;
  onNextPage: () => void;
}

export function LibraryCollectionTable({
  userView,
  rows,
  selectedId,
  page,
  totalPages,
  onSelect,
  onPrevPage,
  onNextPage,
}: LibraryCollectionTableProps) {
  return (
    <section className="library-collection-shell">
      <div
        className="table-scroll library-table-scroll library-table-viewport"
        role="region"
        aria-label="Library results"
      >
        <table className="library-table">
          <thead>
            <tr>
              <th>{userView === "beginner" ? "File" : "Mod or file"}</th>
              <th>Status</th>
              <th>{userView === "power" ? "Clues" : "At a glance"}</th>
            </tr>
          </thead>
          <tbody>
            {rows.length ? (
              rows.map((row, index) => {
                const model = buildLibraryRowModel(row, userView);

                return (
                  <m.tr
                    key={row.id}
                    className={selectedId === row.id ? "is-selected" : ""}
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
                    <td>
                      <div className="library-row-title">{model.title}</div>
                      <div className="library-row-type">
                        <span className="library-type-pill">{model.typeLabel}</span>
                      </div>
                    </td>
                    <td>
                      <span className={`library-health-pill is-${model.healthTone}`}>
                        {model.healthLabel}
                      </span>
                    </td>
                    <td>
                      <div className="library-row-facts">
                        {model.supportingFacts.map((fact) => (
                          <span key={fact} className="library-row-fact">
                            {fact}
                          </span>
                        ))}
                      </div>
                    </td>
                  </m.tr>
                );
              })
            ) : (
              <tr>
                <td colSpan={3} className="empty-row">
                  {userView === "beginner"
                    ? "Nothing matches these filters right now."
                    : "No indexed files match the current filters."}
                </td>
              </tr>
            )}
          </tbody>
        </table>
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
    </section>
  );
}
