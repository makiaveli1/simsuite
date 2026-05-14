import { m } from "motion/react";
import { ArrowRight, ShieldCheck } from "lucide-react";
import { PendingPlansPreview } from "./organize/PendingPlansPreview";
import type { Screen, UserView } from "../lib/types";

interface StagingScreenProps {
  onNavigate: (screen: Screen) => void;
  userView?: UserView;
}

export function StagingScreen({ onNavigate }: StagingScreenProps) {
  return (
    <div className="staging-screen">
      <div className="staging-header">
        <div className="staging-header-left">
          <h2 className="staging-title">Plan Preview</h2>
          <span className="staging-summary">
            Pending plans now live inside Organize.
          </span>
        </div>

        <div className="staging-header-actions">
          <button
            type="button"
            className="secondary-action"
            onClick={() => onNavigate("organize")}
          >
            <ArrowRight size={16} />
            Open Organize
          </button>
          <button
            type="button"
            className="staging-btn staging-btn--disabled"
            disabled
          >
            <ShieldCheck size={14} />
            User confirmation required
          </button>
        </div>
      </div>

      <m.div
        className="staging-result staging-result--warn"
        initial={{ opacity: 0, y: -4 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.14 }}
      >
        Plan Preview is still preview-only, but the main workflow now lives in
        Organize. No files will be changed from this route. Future file-changing
        workflows need preview, user confirmation, backup and restore support,
        and recoverable errors.
      </m.div>

      <PendingPlansPreview onNavigate={onNavigate} />
    </div>
  );
}
