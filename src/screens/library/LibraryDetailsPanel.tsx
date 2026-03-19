import { ExternalLink, Eye, PencilLine, ShieldAlert } from "lucide-react";
import { summarizeLibraryCareState } from "./libraryDisplay";
import { friendlyTypeLabel, unknownCreatorLabel } from "../../lib/uiLanguage";
import type { FileDetail, UserView } from "../../lib/types";

interface LibraryDetailsPanelProps {
  userView: UserView;
  selectedFile: FileDetail | null;
  onOpenHealthDetails: () => void;
  onOpenInspectFile: () => void;
  onOpenEditDetails: () => void;
  onOpenUpdates: () => void;
}

export function LibraryDetailsPanel({
  userView,
  selectedFile,
  onOpenHealthDetails,
  onOpenInspectFile,
  onOpenEditDetails,
  onOpenUpdates,
}: LibraryDetailsPanelProps) {
  if (!selectedFile) {
    return (
      <div className="detail-empty library-details-empty">
        <p className="eyebrow">{userView === "beginner" ? "Selected file" : "Inspector"}</p>
        <h2>Select a file</h2>
        <p className="text-muted">
          Pick something from the list to see what it is, whether it needs care,
          and which deeper panel is worth opening.
        </p>
      </div>
    );
  }

  const careSummary = summarizeLibraryCareState(selectedFile);
  const careTags = [...selectedFile.safetyNotes, ...selectedFile.parserWarnings].slice(0, 3);
  const hasUpdates = Boolean(selectedFile.installedVersionSummary);

  return (
    <div className="library-details-panel">
      <div className="detail-header">
        <div>
          <p className="eyebrow">{userView === "beginner" ? "Selected file" : "Inspector"}</p>
          <h2>{selectedFile.filename}</h2>
        </div>
        <span className="confidence-badge neutral">
          {Math.round(selectedFile.confidence * 100)}%
        </span>
      </div>

      <section className="library-details-card">
        <div className="section-label">Snapshot</div>
        <div className="detail-list">
          <DetailLine
            label="Creator"
            value={selectedFile.creator ?? unknownCreatorLabel(userView)}
          />
          <DetailLine label="Type" value={friendlyTypeLabel(selectedFile.kind)} />
          {selectedFile.subtype?.trim() ? (
            <DetailLine label="Subtype" value={selectedFile.subtype} />
          ) : null}
          <DetailLine
            label="Updates"
            value={
              hasUpdates
                ? selectedFile.installedVersionSummary?.version ?? "Tracked"
                : "Not tracked"
            }
          />
        </div>
      </section>

      <section className="library-details-card">
        <div className="section-label">Care</div>
        <p className="library-care-summary">{careSummary}</p>
        {careTags.length ? (
          <div className="tag-list">
            {careTags.map((item) => (
              <span key={item} className="warning-tag">
                {item}
              </span>
            ))}
          </div>
        ) : (
          <p className="text-muted">No warnings are standing out right now.</p>
        )}
      </section>

      <section className="library-details-card">
        <div className="section-label">More</div>
        <div className="library-details-actions">
          <button
            type="button"
            className="secondary-action"
            onClick={onOpenHealthDetails}
          >
            <ShieldAlert size={14} strokeWidth={2} />
            Health details
          </button>
          <button
            type="button"
            className="secondary-action"
            onClick={onOpenInspectFile}
          >
            <Eye size={14} strokeWidth={2} />
            Inspect file
          </button>
          <button
            type="button"
            className="secondary-action"
            onClick={onOpenEditDetails}
          >
            <PencilLine size={14} strokeWidth={2} />
            Edit details
          </button>
          {hasUpdates ? (
            <button
              type="button"
              className="secondary-action"
              onClick={onOpenUpdates}
            >
              <ExternalLink size={14} strokeWidth={2} />
              Open in Updates
            </button>
          ) : null}
        </div>
      </section>
    </div>
  );
}

function DetailLine({ label, value }: { label: string; value: string }) {
  return (
    <div className="detail-row">
      <span>{label}</span>
      <strong>{value}</strong>
    </div>
  );
}
