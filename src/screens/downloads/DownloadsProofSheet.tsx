import { AnimatePresence, m } from "motion/react";
import { Eye, X } from "lucide-react";
import {
  downloadsSheetTransition,
  overlayTransition,
  panelSpring,
} from "../../lib/motion";
import type { UserView } from "../../lib/types";
import {
  DockSectionStack,
  type DockSectionDefinition,
} from "../../components/DockSectionStack";
import type {
  DownloadsDecisionBadge,
  DownloadsDecisionSignal,
} from "./DownloadsDecisionPanel";

interface DownloadsProofSheetProps {
  open: boolean;
  onClose: () => void;
  title: string;
  summary: string;
  laneLabel: string;
  badges: DownloadsDecisionBadge[];
  signals: DownloadsDecisionSignal[];
  sections: DockSectionDefinition[];
  userView: UserView;
}

export function DownloadsProofSheet({
  open,
  onClose,
  title,
  summary,
  laneLabel,
  badges,
  signals,
  sections,
  userView,
}: DownloadsProofSheetProps) {
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
            className="workbench-sheet downloads-proof-sheet"
            role="dialog"
            aria-modal="true"
            aria-labelledby="downloads-proof-sheet-title"
            initial={{ opacity: 0, x: 52 }}
            animate={{ opacity: 1, x: 0 }}
            exit={{ opacity: 0, x: 58 }}
            transition={panelSpring}
            onClick={(event) => event.stopPropagation()}
          >
            <div className="workbench-sheet-header">
              <div>
                <p className="eyebrow">
                  {userView === "beginner" ? "Proof" : "Proof sheet"}
                </p>
                <h2 id="downloads-proof-sheet-title">
                  {userView === "beginner"
                    ? "See the full safety story"
                    : "See files, versions, and evidence"}
                </h2>
                <p className="workbench-sheet-copy">
                  Open the deeper checks here without crowding the main desk.
                </p>
              </div>
              <button
                type="button"
                className="workspace-toggle"
                onClick={onClose}
                aria-label="Close proof sheet"
              >
                <X size={14} strokeWidth={2} />
              </button>
            </div>

            <div className="workbench-sheet-body downloads-proof-sheet-body">
              <m.div
                className="downloads-proof-sheet-lead"
                layout
                transition={downloadsSheetTransition}
              >
                <div>
                  <span className="section-label">Selected</span>
                  <strong>{title}</strong>
                  <p className="workspace-toolbar-copy">{summary}</p>
                </div>
                <div className="downloads-proof-sheet-meta">
                  <span className="ghost-chip">{laneLabel}</span>
                  {badges.map((badge) => (
                    <span
                      key={`${title}-${badge.label}`}
                      className={`confidence-badge ${badge.tone}`}
                    >
                      {badge.label}
                    </span>
                  ))}
                </div>
              </m.div>

              {signals.length ? (
                <div className="downloads-signal-strip downloads-proof-signals">
                  {signals.map((signal) => (
                    <div
                      key={signal.id}
                      className={`downloads-signal-card downloads-signal-card-${signal.tone}`}
                    >
                      <span className="downloads-signal-label">{signal.label}</span>
                      <strong>{signal.title}</strong>
                      <span>{signal.body}</span>
                    </div>
                  ))}
                </div>
              ) : null}

              <DockSectionStack
                layoutId="downloadsProofSheet"
                sections={sections}
                intro="Reset this proof view"
                className="downloads-proof-stack"
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
