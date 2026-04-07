import type { ReactNode } from "react";
import { AnimatePresence, m } from "motion/react";
import { Eye, X } from "lucide-react";
import { DockSectionStack, type DockSectionDefinition } from "../../components/DockSectionStack";
import {
  overlayTransition,
  panelSpring,
} from "../../lib/motion";
import { friendlyTypeLabel, unknownCreatorLabel } from "../../lib/uiLanguage";
import type { FileDetail, UserView } from "../../lib/types";

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
            key={`library-sheet:${selectedFile.id}:${mode}:${userView}`}
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
                <p className="workbench-sheet-copy">{librarySheetCopy(mode, userView)}</p>
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
              <div className="library-detail-sheet-lead">
                <div>
                  <span className="section-label">Selected</span>
                  <strong>{selectedFile.filename}</strong>
                  <p className="workspace-toolbar-copy">
                    {friendlyTypeLabel(selectedFile.kind)}
                    {selectedFile.subtype?.trim() ? ` / ${selectedFile.subtype}` : ""}
                  </p>
                </div>
                <div className="library-detail-sheet-meta">
                  <span className="ghost-chip">
                    {selectedFile.creator ?? unknownCreatorLabel(userView)}
                  </span>
                  <span className="confidence-badge neutral">
                    {Math.round(selectedFile.confidence * 100)}%
                  </span>
                </div>
              </div>

              <DockSectionStack
                key={`librarySheetStack:${selectedFile.id}:${mode}:${userView}`}
                layoutId={`librarySheet:${mode}`}
                sections={sections}
                intro="Reset this detail view"
                showHints={userView !== "beginner"}
              />
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

function librarySheetEyebrow(mode: Exclude<LibrarySheetMode, null>, userView: UserView) {
  if (mode === "health") {
    return userView === "beginner" ? "Warnings" : "Warnings and updates";
  }

  if (mode === "inspect") {
    return userView === "beginner" ? "Inspect" : "Inspect file";
  }

  return userView === "beginner" ? "Edit" : "Edit details";
}

function librarySheetTitle(mode: Exclude<LibrarySheetMode, null>, userView: UserView) {
  if (mode === "health") {
    return userView === "beginner"
      ? "Warnings, updates, and bundle notes"
      : userView === "power"
        ? "Diagnostics, watch evidence, and bundle context"
        : "Warnings, updates, and bundle context";
  }

  if (mode === "inspect") {
    return userView === "beginner"
      ? "File facts and deeper clues"
      : userView === "power"
        ? "Embedded identity, clues, and full path"
        : "Embedded names, version clues, and file facts";
  }

  return userView === "beginner"
    ? "Fix the saved details here"
    : userView === "power"
      ? "Edit creator learning and type overrides"
      : "Fix creator and type details without moving the file";
}

function librarySheetCopy(mode: Exclude<LibrarySheetMode, null>, userView: UserView) {
  if (mode === "health") {
    return userView === "power"
      ? "Warnings, update watch evidence, and bundle notes that affect how safely this file can live in your library."
      : "Warnings, update status, and bundle notes that matter before you keep, replace, or trust this file.";
  }

  if (mode === "inspect") {
    return userView === "power"
      ? "Embedded names, version evidence, structure clues, and the full file path."
      : "The most useful clues SimSuite pulled from inside the file, plus the facts you need to identify it properly.";
  }

  return userView === "power"
    ? "Save creator learning and type overrides that change how SimSuite reads this file later."
    : "Fix creator and type details here without moving the file or leaving Library.";
}
