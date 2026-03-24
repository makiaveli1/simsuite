import { AnimatePresence, m } from "motion/react";
import { Eye, Workflow } from "lucide-react";
import {
  downloadsSelectionTransition,
  hoverLift,
  tapPress,
} from "../../lib/motion";
import type { UserView } from "../../lib/types";
import type { DownloadQueueLane } from "../../lib/guidedFlowStorage";

const LANE_MEANINGS: Record<DownloadQueueLane, string> = {
  ready_now: "This is safe and ready to add to your game",
  waiting_on_you: "Something needs your attention before this can be added",
  special_setup: "This needs a few extra steps first",
  blocked: "This was stopped — check the warning if you want to add it",
  done: "This has already been added or set aside",
  rejected: "This was rejected and moved to SimSuite_Rejected",
};

export interface DownloadsDecisionSignal {
  id: string;
  tone: "guided" | "review" | "refresh";
  label: string;
  title: string;
  body: string;
}

export interface DownloadsDecisionBadge {
  label: string;
  tone: string;
}

interface DownloadsDecisionPanelProps {
  userView: UserView;
  title: string;
  summary: string;
  laneLabel: string;
  resolvedLane?: DownloadQueueLane;
  badges: DownloadsDecisionBadge[];
  signals: DownloadsDecisionSignal[];
  nextStepTitle: string | null;
  nextStepDescription: string | null;
  primaryActionLabel?: string | null;
  primaryActionDisabled?: boolean;
  onPrimaryAction?: () => void;
  secondaryActionLabel: string;
  secondaryActionDisabled?: boolean;
  onSecondaryAction: () => void;
  onOpenProof: () => void;
  proofSummary: string;
  idleNote?: string | null;
}

export function DownloadsDecisionPanel({
  userView,
  title,
  summary,
  laneLabel,
  resolvedLane,
  badges,
  signals,
  nextStepTitle,
  nextStepDescription,
  primaryActionLabel,
  primaryActionDisabled,
  onPrimaryAction,
  secondaryActionLabel,
  secondaryActionDisabled,
  onSecondaryAction,
  onOpenProof,
  proofSummary,
  idleNote,
}: DownloadsDecisionPanelProps) {
  return (
    <div className="downloads-decision-panel">
      <m.div
        className="downloads-decision-header"
        layout
        transition={downloadsSelectionTransition}
      >
        <div className="downloads-decision-copy">
          <p className="eyebrow">
            {userView === "beginner" ? "Selected batch" : "Selected inbox item"}
          </p>
          <h2>{title}</h2>
          <p className="workspace-toolbar-copy">{summary}</p>
        </div>

        <div className="downloads-decision-badges">
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

      {userView === "beginner" && resolvedLane && (
        <div
          style={{
            background: "color-mix(in srgb, var(--accent) 8%, transparent)",
            border: "1px solid color-mix(in srgb, var(--accent) 20%, transparent)",
            borderRadius: "10px",
            padding: "12px 14px",
            marginBottom: "16px",
          }}
        >
          <p
            style={{
              fontSize: "12px",
              fontWeight: 600,
              color: "var(--text)",
              margin: "0 0 4px",
            }}
          >
            {laneLabel}: {LANE_MEANINGS[resolvedLane]}
          </p>
          <p
            style={{
              fontSize: "12px",
              color: "var(--text-soft)",
              margin: 0,
            }}
          >
            Action available:{" "}
            <strong style={{ color: "var(--text)" }}>
              {primaryActionLabel}
            </strong>{" "}
            — tap to move this to your game
          </p>
        </div>
      )}

      {signals.length ? (
        <div className="downloads-signal-strip">
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

      <m.section
        className="downloads-next-step-card downloads-decision-card"
        layout
        transition={downloadsSelectionTransition}
      >
        <AnimatePresence mode="wait" initial={false}>
          <m.div
            key={`${title}-${nextStepTitle ?? "idle"}`}
            className="downloads-next-step-copy"
            initial={{ opacity: 0, y: 6 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0, y: -4 }}
            transition={downloadsSelectionTransition}
          >
            <p className="eyebrow">
              {userView === "beginner" ? "Safe next step" : "Next move"}
            </p>
            <strong className="downloads-next-step-title">
              {nextStepTitle ?? "Pick a batch to continue"}
            </strong>
            <p className="downloads-next-step-description">
              {nextStepDescription ??
                "The calmest next move appears here once a batch is selected."}
            </p>
          </m.div>
        </AnimatePresence>

        <div className="downloads-next-step-actions">
          {primaryActionLabel && onPrimaryAction ? (
            <m.button
              type="button"
              className="primary-action"
              onClick={onPrimaryAction}
              disabled={primaryActionDisabled}
              whileHover={primaryActionDisabled ? undefined : hoverLift}
              whileTap={primaryActionDisabled ? undefined : tapPress}
            >
              <Workflow size={14} strokeWidth={2} />
              {primaryActionLabel}
            </m.button>
          ) : null}

          <m.button
            type="button"
            className="secondary-action"
            onClick={onSecondaryAction}
            disabled={secondaryActionDisabled}
            whileHover={secondaryActionDisabled ? undefined : hoverLift}
            whileTap={secondaryActionDisabled ? undefined : tapPress}
          >
            {secondaryActionLabel}
          </m.button>
        </div>

        {idleNote ? <div className="downloads-inspector-note">{idleNote}</div> : null}
      </m.section>

      <m.section
        className="downloads-decision-proof"
        layout
        transition={downloadsSelectionTransition}
      >
        <div className="downloads-decision-proof-copy">
          <p className="eyebrow">
            {userView === "beginner" ? "Need the full story?" : "Proof on demand"}
          </p>
          <strong>
            {userView === "power"
              ? "Open the full receipt trail"
              : "Open the calmer proof sheet"}
          </strong>
          <p>{proofSummary}</p>
        </div>

        <m.button
          type="button"
          className="secondary-action"
          onClick={onOpenProof}
          whileHover={hoverLift}
          whileTap={tapPress}
        >
          <Eye size={14} strokeWidth={2} />
          Open proof
        </m.button>
      </m.section>
    </div>
  );
}
