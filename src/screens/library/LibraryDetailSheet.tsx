import type { ReactNode } from "react";
import { AnimatePresence, m } from "motion/react";
import { Eye, X } from "lucide-react";
import type { DockSectionDefinition } from "../../components/DockSectionStack";
import {
  overlayTransition,
  panelSpring,
} from "../../lib/motion";
import { friendlyTypeLabel, unknownCreatorLabel } from "../../lib/uiLanguage";
import type { FileDetail, UserView } from "../../lib/types";
import { buildLibraryRowModel } from "./libraryDisplay";

export type LibrarySheetMode = "health" | "inspect" | "edit" | null;

interface LibraryDetailSheetProps {
  open: boolean;
  mode: LibrarySheetMode;
  selectedFile: FileDetail | null;
  sections: DockSectionDefinition[];
  userView: UserView;
  onClose: () => void;
}

export function LibraryDetailSheet({
  open,
  mode,
  selectedFile,
  sections,
  userView,
  onClose,
}: LibraryDetailSheetProps) {
  if (!selectedFile || !mode) {
    return null;
  }

  const rowModel = buildLibraryRowModel(selectedFile, userView);

  return (
    <AnimatePresence>
      {open ? (
        <m.div
          className="workbench-sheet-shell"
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          exit={{ opacity: 0 }}
          transition={overlayTransition}
          onClick={onClose}
        >
          <m.aside
            className="workbench-sheet library-detail-sheet"
            role="dialog"
            aria-modal="true"
            aria-labelledby="library-detail-sheet-title"
            initial={{ opacity: 0, x: 52 }}
            animate={{ opacity: 1, x: 0 }}
            exit={{ opacity: 0, x: 58 }}
            transition={panelSpring}
            onClick={(event) => event.stopPropagation()}
          >
            <div className="workbench-sheet-header">
              <div>
                <p className="eyebrow">{librarySheetEyebrow(mode, userView)}</p>
                <h2 id="library-detail-sheet-title">{librarySheetTitle(mode, userView)}</h2>
                <p className="workbench-sheet-copy">
                  Keep the main Library page quiet and open the fuller file story here only when
                  you need it.
                </p>
              </div>
              <button
                type="button"
                className="workspace-toggle"
                onClick={onClose}
                aria-label="Close Library detail sheet"
              >
                <X size={14} strokeWidth={2} />
              </button>
            </div>

            <div className="workbench-sheet-body library-detail-sheet-body">
              <section className="library-detail-sheet-summary">
                <div className="library-detail-sheet-summary-copy">
                  <span className="section-label">Selected file</span>
                  <h3>{selectedFile.filename}</h3>
                  <p className="library-detail-sheet-summary-subcopy">
                    {friendlyTypeLabel(selectedFile.kind)}
                    {selectedFile.subtype?.trim() ? ` / ${selectedFile.subtype}` : ""}
                  </p>
                </div>
                <div className="library-detail-sheet-summary-meta">
                  <span className={`library-health-pill is-${rowModel.healthTone}`}>
                    {rowModel.healthLabel}
                  </span>
                  <span className={`library-type-pill is-${rowModel.typeTone}`}>
                    {rowModel.typeLabel}
                  </span>
                  <span className="ghost-chip">
                    {selectedFile.creator ?? unknownCreatorLabel(userView)}
                  </span>
                  <span className="confidence-badge neutral">
                    {Math.round(selectedFile.confidence * 100)}%
                  </span>
                </div>
              </section>

              <div className="library-sheet-section-list">
                {sections.map((section) => (
                  <LibrarySheetSection
                    key={section.id}
                    section={section}
                    showHint={userView !== "beginner"}
                  />
                ))}
              </div>
            </div>

            <div className="workbench-sheet-footer">
              <button type="button" className="primary-action" onClick={onClose}>
                <Eye size={14} strokeWidth={2} />
                Done
              </button>
            </div>
          </m.aside>
        </m.div>
      ) : null}
    </AnimatePresence>
  );
}

function LibrarySheetSection({
  section,
  showHint,
}: {
  section: DockSectionDefinition;
  showHint: boolean;
}) {
  return (
    <section className="library-sheet-section-card">
      <div className="library-sheet-section-header">
        <div className="library-sheet-section-copy">
          <strong>{section.label}</strong>
          {showHint && section.hint ? (
            <p className="library-sheet-section-hint">{section.hint}</p>
          ) : null}
        </div>
        {section.badge ? <span className="ghost-chip">{section.badge}</span> : null}
      </div>
      <div className="library-sheet-section-body">{section.children as ReactNode}</div>
    </section>
  );
}

function librarySheetEyebrow(mode: Exclude<LibrarySheetMode, null>, userView: UserView) {
  if (mode === "health") {
    return userView === "beginner" ? "Health" : "Health details";
  }

  if (mode === "inspect") {
    return userView === "beginner" ? "Inspect" : "Inspect file";
  }

  return userView === "beginner" ? "Edit" : "Edit details";
}

function librarySheetTitle(mode: Exclude<LibrarySheetMode, null>, userView: UserView) {
  if (mode === "health") {
    return userView === "beginner"
      ? "See the warnings and care notes"
      : "See warnings, bundle notes, and update context";
  }

  if (mode === "inspect") {
    return userView === "beginner"
      ? "See the fuller file details"
      : "See the path, clues, and deeper file facts";
  }

  return userView === "beginner"
    ? "Fix the saved details here"
    : "Edit creator and type details without crowding the main page";
}
