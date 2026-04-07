import { type ReactNode } from "react";
import { ExternalLink, Eye, FolderOpen, PencilLine, ShieldAlert } from "lucide-react";
import { formatLibraryFileFormat, summarizeLibraryCareState, typeColorForKind } from "./libraryDisplay";
import { friendlyTypeLabel, unknownCreatorLabel } from "../../lib/uiLanguage";
import { api } from "../../lib/api";
import type { FileDetail, UserView, WatchStatus } from "../../lib/types";

interface LibraryDetailsPanelProps {
  userView: UserView;
  selectedFile: FileDetail | null;
  onOpenInspectDetails: () => void;
  onOpenHealthDetails: () => void;
  onOpenEditDetails: () => void;
  onOpenUpdates: () => void;
  /** Optional content rendered at the top-right of the detail header (e.g. collapse button) */
  headerRight?: ReactNode;
}

export function LibraryDetailsPanel({
  userView,
  selectedFile,
  onOpenInspectDetails,
  onOpenHealthDetails,
  onOpenEditDetails,
  onOpenUpdates,
  headerRight,
}: LibraryDetailsPanelProps) {
  const isCasual = userView === "beginner";
  const isPower = userView === "power";

  if (!selectedFile) {
    return (
      <div className="detail-empty library-details-empty">
        <p className="eyebrow">{isCasual ? "Selected file" : "Inspector"}</p>
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
  const hasHealthDetails = Boolean(
    selectedFile.bundleName ||
      selectedFile.watchResult ||
      selectedFile.installedVersionSummary ||
      hasSafetyNotes ||
      hasParserWarnings,
  );

  const isTray = selectedFile.sourceLocation === "tray";
  const typeColor = typeColorForKind(selectedFile.kind);
  const confidenceColor =
    selectedFile.confidence >= 0.8
      ? "high"
      : selectedFile.confidence >= 0.55
        ? "medium"
        : "low";

  // ─── Snapshot — view-aware ───────────────────────────────────────────────
  // Casual: only what matters for a quick read — creator, type, watch
  // Seasoned: adds installed version and confidence for maintenance workflows
  // Creator: adds file format for deep inspection
  // ────────────────────────────────────────────────────────────────────────
  const snapshotLines: Array<{ label: string; value: ReactNode }> = [
    {
      label: "Creator",
      value: selectedFile.creator ?? unknownCreatorLabel(userView),
    },
    { label: "Type", value: friendlyTypeLabel(selectedFile.kind) },
  ];

  // Subtype: useful for CAS/Gameplay categorization
  if (!isCasual && selectedFile.subtype?.trim()) {
    snapshotLines.push({ label: "Subtype", value: selectedFile.subtype });
  }

  // Installed version: relevant for seasoned simmer workflows
  if (!isCasual && selectedFile.installedVersionSummary?.version) {
    snapshotLines.push({
      label: "Installed",
      value: selectedFile.installedVersionSummary.version,
    });
  }

  // Watch: always — core state indicator
  if (watchStatus) {
    snapshotLines.push({
      label: "Watch",
      value: (
        <span className={`library-health-pill is-${watchStatusTone}`}>
          {watchStatusLabel}
          {watchSourceLabel ? ` · ${watchSourceLabel}` : null}
        </span>
      ),
    });
  }

  // Confidence: shown in seasoned+ for maintenance quality signals
  if (!isCasual) {
    snapshotLines.push({ label: "Confidence", value: confidenceLabel });
  }

  // File format: creator-only deep inspection signal
  if (isPower) {
    const format = formatLibraryFileFormat(selectedFile);
    if (format !== "Unknown") {
      snapshotLines.push({ label: "Format", value: format });
    }
  }

  // ─── Care section — view-aware ───────────────────────────────────────────
  // Casual: plain-language summary, no tag clutter
  // Seasoned: shows warnings inline so they don't have to open More details
  // Creator: shows everything; expects the user to manage it
  // ────────────────────────────────────────────────────────────────────────
  const showCareTags = !isCasual && (hasSafetyNotes || hasParserWarnings);

  // ─── More actions — view-aware ───────────────────────────────────────────
  // Casual: one button, nothing else
  // Seasoned: adds Open in Updates for maintenance
  // Creator: adds Open folder for file operations
  // ────────────────────────────────────────────────────────────────────────

  return (
    <div className={`library-details-panel${isTray ? " is-tray-item" : ""}`}>
      {/* ── Header ── */}
      <div className="detail-header">
        <div className="detail-header-top">
          {headerRight}
        </div>
        <h2 className="detail-filename">{selectedFile.filename}</h2>
        <div className="detail-header-meta">
          <span className={`library-type-pill type-pill--${typeColor}`}>
            {friendlyTypeLabel(selectedFile.kind)}
          </span>
          <span
            className={`library-confidence-badge confidence--${confidenceColor}`}
            title={`${confidenceLabel} confidence`}
            aria-label={`${confidenceLabel} confidence`}
          >
            {selectedFile.confidence >= 0.8
              ? "✓"
              : selectedFile.confidence >= 0.55
                ? "⚠"
                : "?"}
          </span>
          {!isCasual && (
            <span className="detail-confidence-text">{confidenceLabel} confidence</span>
          )}
        </div>
      </div>

      {/* ── Snapshot — view-aware depth ── */}
      <section className="library-details-card">
        <div className="section-label">Snapshot</div>
        <div className="detail-list">
          {snapshotLines.map((line) => (
            <DetailLine key={line.label} label={line.label} value={line.value} />
          ))}
        </div>
      </section>

      {/* ── Care — view-aware depth ── */}
      <section className="library-details-card">
        <div className="section-label">Care</div>
        <p className="library-care-summary">{careSummary}</p>

        {/* Warnings shown inline for seasoned+ so they don't need to open More details */}
        {showCareTags ? (
          <>
            {hasSafetyNotes ? (
              <div className="detail-block">
                <div
                  className="section-label"
                  style={{ fontSize: "0.65rem", marginBottom: "0.2rem" }}
                >
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
                <div
                  className="section-label"
                  style={{ fontSize: "0.65rem", marginBottom: "0.2rem" }}
                >
                  Parser warnings
                </div>
                <div className="tag-list">
                  {selectedFile.parserWarnings.slice(0, isPower ? undefined : 5).map(
                    (item) => (
                      <span key={item} className="ghost-chip">
                        {item}
                      </span>
                    ),
                  )}
                  {selectedFile.parserWarnings.length > 5 && !isPower && (
                    <button
                      type="button"
                      className="ghost-chip-inline-button"
                      onClick={onOpenHealthDetails}
                    >
                      +{selectedFile.parserWarnings.length - 5} more
                    </button>
                  )}
                </div>
              </div>
            ) : null}
          </>
        ) : null}

        {/* Casual gets a simple nudge instead of raw tags */}
        {isCasual && (hasSafetyNotes || hasParserWarnings) ? (
          <p className="text-muted">
            Open More details to see the full picture.
          </p>
        ) : null}
      </section>

      {/* ── Duplicates — shown in seasoned+ ── */}
      {hasDuplicates && !isCasual ? (
        <section className="library-details-card">
          <div className="section-label">Duplicates</div>
          <p className="library-care-summary">
            This file appears in{" "}
            {selectedFile.duplicatesCount === 1
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

      {/* ── More actions — view-aware ── */}
      <section className="library-details-card">
        <div className="section-label">Open</div>
        <div className="library-details-actions">
          <button
            type="button"
            className="secondary-action"
            onClick={onOpenInspectDetails}
          >
            <Eye size={14} strokeWidth={2} />
            {isCasual ? "More details" : "Inspect file"}
          </button>

          {!isCasual && hasHealthDetails ? (
            <button
              type="button"
              className="secondary-action"
              onClick={onOpenHealthDetails}
            >
              <ShieldAlert size={14} strokeWidth={2} />
              Warnings & updates
            </button>
          ) : null}

          {!isCasual ? (
            <button
              type="button"
              className="secondary-action"
              onClick={onOpenEditDetails}
            >
              <PencilLine size={14} strokeWidth={2} />
              Edit details
            </button>
          ) : null}

          {/* Open in Updates — useful for seasoned+ maintenance */}
          {hasUpdates && !isCasual ? (
            <button
              type="button"
              className="secondary-action"
              onClick={onOpenUpdates}
            >
              <ExternalLink size={14} strokeWidth={2} />
              Open in Updates
            </button>
          ) : null}

          {/* Open folder — useful for creator-level file operations */}
          {isPower ? (
            <button
              type="button"
              className="secondary-action"
              onClick={() => {
                void api.revealFileInFolder(selectedFile.path);
              }}
              title={selectedFile.path}
            >
              <FolderOpen size={14} strokeWidth={2} />
              Open folder
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

