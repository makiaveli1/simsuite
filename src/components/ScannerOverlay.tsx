import { m } from "motion/react";
import { overlayTransition, panelSpring } from "../lib/motion";
import { ResizableEdgeHandle } from "./ResizableEdgeHandle";
import { useUiPreferences } from "./UiPreferencesContext";
import type { ExperienceMode, ScanProgress } from "../lib/types";

interface ScannerOverlayProps {
  progress: ScanProgress | null;
  experienceMode: ExperienceMode;
}

const PHASE_LABELS: Record<ScanProgress["phase"], string> = {
  collecting: "Looking through your Mods and Tray folders",
  hashing: "Checking files for exact copies",
  classifying: "Working out creators and types",
  bundling: "Keeping Tray files together",
  duplicates: "Looking for duplicate files",
  done: "Scan finished",
};

export function ScannerOverlay({
  progress,
  experienceMode,
}: ScannerOverlayProps) {
  const { scannerWidth, setScannerWidth } = useUiPreferences();
  const total = progress?.totalFiles ?? 0;
  const processed = progress?.processedFiles ?? 0;
  const isCollecting = progress?.phase === "collecting";
  const ratio = !isCollecting && total > 0 ? Math.min(processed / total, 1) : 0;
  const badgeLabel = isCollecting
    ? `${total.toLocaleString()} found`
    : `${Math.round(ratio * 100)}%`;
  const primaryCount = isCollecting ? total : processed;
  const primaryLabel = isCollecting ? "found" : "checked";
  const secondaryCount = isCollecting ? "Reading folders" : total.toLocaleString();
  const secondaryLabel = isCollecting ? "current step" : "in scan";

  return (
    <m.div
      className="scanner-overlay"
      role="status"
      aria-live="polite"
      initial={{ opacity: 0 }}
      animate={{ opacity: 1 }}
      exit={{ opacity: 0 }}
      transition={overlayTransition}
    >
      <m.div
        className="scanner-card"
        initial={{ opacity: 0, y: 18, scale: 0.985 }}
        animate={{ opacity: 1, y: 0, scale: 1 }}
        exit={{ opacity: 0, y: 12, scale: 0.99 }}
        transition={panelSpring}
      >
        <ResizableEdgeHandle
          label="Resize scan window"
          value={scannerWidth}
          min={420}
          max={860}
          onChange={setScannerWidth}
          side="right"
          className="scanner-resize-handle"
        />
        <div className="scanner-heading">
          <div>
            <p className="eyebrow">
              {experienceMode === "casual"
                ? "Checking your CC"
                : experienceMode === "creator"
                  ? "Deep scan running"
                  : "Library scan in progress"}
            </p>
            <h2>{progress ? PHASE_LABELS[progress.phase] : "Preparing scan"}</h2>
          </div>
          <div className="confidence-badge neutral">{badgeLabel}</div>
        </div>

        <div className={`scanner-bar${isCollecting ? " is-indeterminate" : ""}`}>
          <span style={isCollecting ? undefined : { width: `${ratio * 100}%` }} />
        </div>

        <div className="scanner-meta">
          <div>
            <strong>{primaryCount.toLocaleString()}</strong>
            <span>{primaryLabel}</span>
          </div>
          <div>
            <strong>{secondaryCount}</strong>
            <span>{secondaryLabel}</span>
          </div>
        </div>

        <div className="scanner-current">
          {progress?.currentItem || "Getting ready to read the first file..."}
        </div>
      </m.div>
    </m.div>
  );
}
