import { type ReactNode } from "react";
import { ExternalLink, SlidersHorizontal } from "lucide-react";
import { summarizeLibraryCareState } from "./libraryDisplay";
import { friendlyTypeLabel, unknownCreatorLabel } from "../../lib/uiLanguage";
import { typeColorForKind } from "./libraryDisplay";
import type { FileDetail, UserView, WatchStatus } from "../../lib/types";

interface LibraryDetailsPanelProps {
  userView: UserView;
  selectedFile: FileDetail | null;
  /** Opens the full detail sheet (health/inspect/edit accessible inside) */
  onOpenMoreDetails: () => void;
  onOpenUpdates: () => void;
}

export function LibraryDetailsPanel({
  userView,
  selectedFile,
  onOpenMoreDetails,
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
  const hasUpdates = Boolean(selectedFile.installedVersionSummary);
  const watchStatus = selectedFile.watchResult?.status ?? null;
  const watchStatusLabel = watchStatusToLabel(watchStatus);
  const watchStatusTone = watchStatusToTone(watchStatus);
  const watchSourceLabel = selectedFile.watchResult?.sourceLabel ?? null;
  const hasSafetyNotes = selectedFile.safetyNotes.length > 0;
  const hasParserWarnings = selectedFile.parserWarnings.length > 0;
  const confidenceLabel =
    selectedFile.confidence >= 0.8
      ? "High"
      : selectedFile.confidence >= 0.55
        ? "Medium"
        : "Low";

  const hasDuplicates = (selectedFile.duplicatesCount ?? 0) > 0;
  const duplicateTypes = selectedFile.duplicateTypes ?? [];

  const isTray = selectedFile.sourceLocation === 'tray';
  const typeColor = typeColorForKind(selectedFile.kind);
  const confidenceColor =
    selectedFile.confidence >= 0.8 ? 'high' :
    selectedFile.confidence >= 0.55 ? 'medium' : 'low';

  return (
    <div className={`library-details-panel${isTray ? " is-tray-item" : ""}`}>
      <div className="detail-header">
        <div className="detail-header-top">
          <p className="eyebrow">{userView === "beginner" ? "Selected file" : "Inspector"}</p>
          {isTray && (
            <span className="library-tray-badge">Tray</span>
          )}
        </div>
        <h2 className="detail-filename">{selectedFile.filename}</h2>
        <div className="detail-header-meta">
          {/* Type badge with color accent */}
          <span className={`library-type-pill type-pill--${typeColor}`}>
            {friendlyTypeLabel(selectedFile.kind)}
          </span>
          {/* Confidence */}
          <span className={`library-confidence-badge confidence--${confidenceColor}`}
            title={`${confidenceLabel} confidence`}
            aria-label={`${confidenceLabel} confidence`}
          >
            {selectedFile.confidence >= 0.8 ? '✓' :
             selectedFile.confidence >= 0.55 ? '⚠' : '?'}
          </span>
          <span className="detail-confidence-text">{confidenceLabel} confidence</span>
        </div>
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
          {selectedFile.installedVersionSummary?.version ? (
            <DetailLine
              label="Installed"
              value={selectedFile.installedVersionSummary.version}
            />
          ) : null}
          {watchStatus ? (
            <DetailLine
              label="Watch"
              value={
                <span className={`library-health-pill is-${watchStatusTone}`}>
                  {watchStatusLabel}
                  {watchSourceLabel ? ` · ${watchSourceLabel}` : null}</span>
              }
            />
          ) : null}
          {selectedFile.installedVersionSummary?.version &&
           selectedFile.watchResult?.status !== 'current' ? (
            <DetailLine
              label="Updates"
              value={
                <span className="library-health-pill is-attention">
                  {watchStatusLabel}
                </span>
              }
            />
          ) : null}
          <DetailLine label="Confidence" value={confidenceLabel} />
        </div>
      </section>

      <section className="library-details-card">
        <div className="section-label">Care</div>
        <p className="library-care-summary">{careSummary}</p>
        {hasSafetyNotes ? (
          <div className="detail-block">
            <div className="section-label" style={{ fontSize: "0.65rem", marginBottom: "0.2rem" }}>
              Safety notes
            </div>
            <div className="tag-list">
              {selectedFile.safetyNotes.map((item) => (
                <span key={item} className="warning-tag">
                  {item}
                </span>
              ))}
            </div>
          </div>
        ) : null}
        {hasParserWarnings ? (
          <div className="detail-block">
            <div className="section-label" style={{ fontSize: "0.65rem", marginBottom: "0.2rem" }}>
              Parser warnings
            </div>
            <div className="tag-list">
              {selectedFile.parserWarnings.slice(0, 5).map((item) => (
                <span key={item} className="ghost-chip">
                  {item}
                </span>
              ))}
            </div>
          </div>
        ) : null}
        {!hasSafetyNotes && !hasParserWarnings ? (
          <p className="text-muted">No warnings are standing out right now.</p>
        ) : null}
      </section>

      {hasDuplicates ? (
        <section className="library-details-card">
          <div className="section-label">Duplicates</div>
          <p className="library-care-summary">
            This file appears in {selectedFile.duplicatesCount === 1
              ? "1 duplicate pair"
              : `${selectedFile.duplicatesCount} duplicate pairs`}.
          </p>
          <div className="tag-list">
            {duplicateTypes.map((type) => (
              <span key={type} className="ghost-chip">
                {type === "exact"
                  ? "Exact match"
                  : type === "filename"
                    ? "Same filename"
                    : type === "version"
                      ? "Same version"
                      : type}
              </span>
            ))}
          </div>
        </section>
      ) : null}

      <section className="library-details-card">
        <div className="section-label">More</div>
        <div className="library-details-actions">
          <button
            type="button"
            className="secondary-action"
            onClick={onOpenMoreDetails}
          >
            <SlidersHorizontal size={14} strokeWidth={2} />
            More details
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

function DetailLine({ label, value }: { label: string; value: ReactNode }) {
  return (
    <div className="detail-row">
      <span>{label}</span>
      <strong>{value}</strong>
    </div>
  );
}

function watchStatusToLabel(status: WatchStatus | null): string {
  switch (status) {
    case "current":
      return "Up to date";
    case "exact_update_available":
      return "Update available";
    case "possible_update":
      return "May have update";
    case "unknown":
      return "Check updates";
    case "not_watched":
    default:
      return "Not tracked";
  }
}

function watchStatusToTone(status: WatchStatus | null): "calm" | "attention" | "muted" {
  switch (status) {
    case "current":
      return "calm";
    case "exact_update_available":
      return "attention";
    case "possible_update":
    case "unknown":
    case "not_watched":
    default:
      return "muted";
  }
}
