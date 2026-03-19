import { AnimatePresence, m } from "motion/react";
import { AlertTriangle, ShieldCheck, X } from "lucide-react";
import {
  downloadsDialogSpring,
  overlayTransition,
} from "../../lib/motion";

export interface DownloadsSetupDialogMetric {
  label: string;
  value: string;
}

interface DownloadsSetupDialogProps {
  open: boolean;
  onClose: () => void;
  onConfirm: () => void;
  eyebrow: string;
  title: string;
  description: string;
  confirmLabel: string;
  cancelLabel?: string;
  isWorking?: boolean;
  tone?: "accent" | "warn" | "danger";
  metrics?: DownloadsSetupDialogMetric[];
  notes?: string[];
}

export function DownloadsSetupDialog({
  open,
  onClose,
  onConfirm,
  eyebrow,
  title,
  description,
  confirmLabel,
  cancelLabel = "Cancel",
  isWorking = false,
  tone = "accent",
  metrics = [],
  notes = [],
}: DownloadsSetupDialogProps) {
  const Icon = tone === "danger" ? AlertTriangle : ShieldCheck;

  return (
    <AnimatePresence>
      {open ? (
        <m.div
          className="downloads-action-dialog-shell"
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          exit={{ opacity: 0 }}
          transition={overlayTransition}
          onClick={isWorking ? undefined : onClose}
        >
          <m.div
            className={`downloads-action-dialog downloads-action-dialog-${tone}`}
            role="dialog"
            aria-modal="true"
            aria-labelledby="downloads-action-dialog-title"
            initial={{ opacity: 0, y: 22, scale: 0.985 }}
            animate={{ opacity: 1, y: 0, scale: 1 }}
            exit={{ opacity: 0, y: 18, scale: 0.985 }}
            transition={downloadsDialogSpring}
            onClick={(event) => event.stopPropagation()}
          >
            <div className="downloads-action-dialog-header">
              <div className="downloads-action-dialog-icon">
                <Icon size={18} strokeWidth={2} />
              </div>
              <button
                type="button"
                className="workspace-toggle"
                onClick={onClose}
                aria-label="Close action dialog"
                disabled={isWorking}
              >
                <X size={14} strokeWidth={2} />
              </button>
            </div>

            <div className="downloads-action-dialog-copy">
              <p className="eyebrow">{eyebrow}</p>
              <h2 id="downloads-action-dialog-title">{title}</h2>
              <p>{description}</p>
            </div>

            {metrics.length ? (
              <div className="downloads-action-dialog-grid">
                {metrics.map((metric) => (
                  <div
                    key={`${metric.label}-${metric.value}`}
                    className="downloads-action-dialog-metric"
                  >
                    <span>{metric.label}</span>
                    <strong>{metric.value}</strong>
                  </div>
                ))}
              </div>
            ) : null}

            {notes.length ? (
              <div className="downloads-action-dialog-notes">
                {notes.map((note) => (
                  <div key={note} className="downloads-action-dialog-note">
                    {note}
                  </div>
                ))}
              </div>
            ) : null}

            <div className="downloads-action-dialog-footer">
              <button
                type="button"
                className="secondary-action"
                onClick={onClose}
                disabled={isWorking}
              >
                {cancelLabel}
              </button>
              <button
                type="button"
                className="primary-action"
                onClick={onConfirm}
                disabled={isWorking}
              >
                {isWorking ? "Working..." : confirmLabel}
              </button>
            </div>
          </m.div>
        </m.div>
      ) : null}
    </AnimatePresence>
  );
}
