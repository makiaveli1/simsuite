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
      <div className="table-scroll library-table-scroll">
        <table className="library-table">
          <thead>
            <tr>
              <th className="library-table-checkbox-col" aria-label="Select" />
              <th>{userView === "beginner" ? "File" : "Mod or file"}</th>
              <th>Status</th>
              <th>{userView === "power" ? "Clues" : "At a glance"}</th>
            </tr>
          </thead>
          <tbody>
            {rows.length ? (
              rows.map((row, index) => {
                const model = buildLibraryRowModel(row, userView);
                const isChecked = selectedIds.has(row.id);

                return (
                  <m.tr
                    key={row.id}
                    className={[
                      selectedId === row.id ? "is-selected" : "",
                      isChecked ? "is-multiselected" : "",
                    ]
                      .filter(Boolean)
                      .join(" ")}
                    onClick={(e) => {
                      // Don't select row if clicking the checkbox itself.
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
                    <td
                      className="library-table-checkbox-col"
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
                    </td>
                    <td>
                      <div className="library-row-title">{model.title}</div>
                      <div className="library-row-type">
                        <span className="library-type-pill">{model.typeLabel}</span>
                      </div>
                    </td>
                    <td>
                      <div className="library-status-pills">
                        <span className={`library-health-pill is-${model.watchStatusTone}`}>
                          {model.watchStatusLabel}
                        </span>
                        {model.healthTone && (
                          <span className={`library-health-pill is-${model.healthTone}`}>
                            {model.healthLabel}
                          </span>
                        )}
                        {model.duplicateTone && (
                          <span className={`library-health-pill is-${model.duplicateTone}`}>
                            {model.duplicateLabel}
                          </span>
                        )}
                      </div>
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
                <td colSpan={4} className="empty-row">
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
    </>
  );
}
