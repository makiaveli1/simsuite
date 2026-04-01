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
                  {mode === "health"
                    ? "Version tracking, warnings, and bundle notes — the deeper health picture."
                    : mode === "inspect"
                      ? "File size, path, embedded names, and clues — the full file story."
                      : "Creator learning and type overrides — change how SimSuite reads this file."}
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
      ? "File facts, warnings, and update context"
      : "Diagnostics — version signals, warnings, and bundle context";
  }

  if (mode === "inspect") {
    return userView === "beginner"
      ? "File facts and embedded details"
      : "The file's embedded identity, clues, and full path";
  }

  return userView === "beginner"
    ? "Fix the saved details here"
    : "Edit creator and type details without crowding the main page";
}
