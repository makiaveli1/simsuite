import type { ReactNode } from "react";
import { AnimatePresence, m } from "motion/react";
import { Eye, X } from "lucide-react";
import { DockSectionStack, type DockSectionDefinition } from "../../components/DockSectionStack";
import {
  overlayTransition,
  panelSpring,
} from "../../lib/motion";
import { friendlyTypeLabel } from "../../lib/uiLanguage";
import type { FileDetail, UserView } from "../../lib/types";
import {
  buildInspectorPreviewStrip,
  describeCreatorForInspector,
  describeLibraryFamilyContext,
  describeLibraryPrimaryLabel,
  libraryIdentityLabelForFilename,
  summarizeLibraryResourceBadge,
  summarizeLibraryScriptContent,
  summarizeScriptScopeForUi,
  summarizeVersionSignalForUi,
  typeColorForKind,
} from "./libraryDisplay";

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

  const inspectFamilyContext =
    mode === "inspect" ? describeLibraryFamilyContext(selectedFile) : null;
  const inspectContentBadge =
    mode === "inspect"
      ? summarizeScriptScopeForUi(selectedFile.insights) ??
        summarizeLibraryScriptContent(selectedFile) ??
        summarizeLibraryResourceBadge(selectedFile)
      : null;
  const inspectVersionBadge =
    mode === "inspect"
      ? summarizeVersionSignalForUi(selectedFile.insights, 0.8) ??
        (selectedFile.insights?.versionSignals?.length
          ? null
          : selectedFile.insights?.versionHints?.[0] ?? null)
      : null;
  const inspectPreviewStrip =
    mode === "inspect" ? buildInspectorPreviewStrip(selectedFile, userView) : null;
  const typeColor = typeColorForKind(selectedFile.kind);
  const inspectPrimaryLabel = describeLibraryPrimaryLabel(selectedFile);
  const inspectIdentityLabel = libraryIdentityLabelForFilename(selectedFile.filename, inspectPrimaryLabel);

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
                  <strong title={selectedFile.filename}>{selectedFile.filename}</strong>
                  <p className="workspace-toolbar-copy">
                    {friendlyTypeLabel(selectedFile.kind)}
                  </p>
                  {inspectIdentityLabel ? (
                    <p className="library-detail-sheet-identity" title={inspectIdentityLabel}>
                      {inspectIdentityLabel}
                    </p>
                  ) : selectedFile.subtype?.trim() ? (
                    <p className="library-detail-sheet-identity" title={selectedFile.subtype}>
                      {selectedFile.subtype}
                    </p>
                  ) : null}
                  {inspectFamilyContext ? (
                    <p className="workspace-toolbar-copy">{inspectFamilyContext}</p>
                  ) : null}
                  {mode === "inspect" && (inspectVersionBadge || inspectContentBadge) ? (
                    <div className="tag-list" style={{ marginTop: "0.6rem" }}>
                      {inspectVersionBadge ? (
                        <span className={`ghost-chip ghost-chip--${typeColor}`}>Version {inspectVersionBadge}</span>
                      ) : null}
                      {inspectContentBadge ? (
                        <span className={`ghost-chip ghost-chip--${typeColor}`}>{inspectContentBadge}</span>
                      ) : null}
                    </div>
                  ) : null}
                  {mode === "inspect" && inspectPreviewStrip?.summaryLabel ? (
                    <div className="library-inspector-preview-strip">
                      {userView === "beginner" ? (
                        <span className="library-inspector-preview-strip-summary">
                          {inspectPreviewStrip.summaryLabel}
                        </span>
                      ) : (
                        <>
                          <div className="library-inspector-preview-strip-left">
                            {inspectPreviewStrip.leftTokens.map((token, index) => (
                              <span className={`ghost-chip ghost-chip--${typeColor}`} key={`${token}:${index}`}>
                                {token}
                              </span>
                            ))}
                          </div>
                          {userView === "power" && inspectPreviewStrip.rightToken ? (
                            <div className="library-inspector-preview-strip-right">
                              <span className={`ghost-chip ghost-chip--${typeColor}`}>{inspectPreviewStrip.rightToken}</span>
                            </div>
                          ) : null}
                        </>
                      )}
                    </div>
                  ) : null}
                </div>
                {/* Thumbnail preview (THUM 0x3C1AF1F2) — shown in inspect mode */}
                {mode === "inspect" && (
                  <div className="library-detail-sheet-thumbnail">
                    {selectedFile.insights?.thumbnailPreview ? (
                      <>
                        <img
                          src={`data:image/png;base64,${selectedFile.insights.thumbnailPreview}`}
                          alt={`Preview for ${selectedFile.filename}`}
                          className="library-detail-sheet-thumbnail-img"
                        />
                        <span className="library-detail-sheet-thumbnail-caption">
                          THUM preview
                        </span>
                      </>
                    ) : (
                      <>
                        <DetailSheetFallbackIcon kind={selectedFile.kind} />
                        <span className="library-detail-sheet-thumbnail-caption">
                          no THUM preview
                        </span>
                      </>
                    )}
                  </div>
                )}

                <div className="library-detail-sheet-meta">
                  <span className="ghost-chip">
                    {describeCreatorForInspector(selectedFile).label}
                    {describeCreatorForInspector(selectedFile).suffix
                      ? ` (${describeCreatorForInspector(selectedFile).suffix})`
                      : null}
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
              {selectedFile.path ? (
                <span className="library-sheet-file-path" title={selectedFile.path}>
                  {selectedFile.path.length > 60
                    ? `…${selectedFile.path.slice(-57)}`
                    : selectedFile.path}
                </span>
              ) : null}
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

// ─── Detail Sheet Fallback Icon ───────────────────────────────────────────────
// Uses pre-generated category PNG icons from public/assets/fallback-icons/.
// Displayed when no THUM preview is available — clearly a fallback, not a real preview.
function DetailSheetFallbackIcon({ kind }: { kind: string }) {
  const iconMap: Record<string, string> = {
    CAS: "/assets/fallback-icons/cas-icon.png",
    BuildBuy: "/assets/fallback-icons/buildbuy-icon.png",
    ScriptMods: "/assets/fallback-icons/scriptmod-icon.png",
    Household: "/assets/fallback-icons/tray-icon.png",
    Lot: "/assets/fallback-icons/tray-icon.png",
    Room: "/assets/fallback-icons/tray-icon.png",
    TrayHousehold: "/assets/fallback-icons/tray-icon.png",
    TrayLot: "/assets/fallback-icons/tray-icon.png",
    TrayRoom: "/assets/fallback-icons/tray-icon.png",
    TrayItem: "/assets/fallback-icons/tray-icon.png",
  };
  const src = iconMap[kind] ?? "/assets/fallback-icons/unknown-icon.png";
  return (
    <img
      src={src}
      alt={`${kind} category icon`}
      className="library-detail-sheet-fallback-icon"
    />
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
