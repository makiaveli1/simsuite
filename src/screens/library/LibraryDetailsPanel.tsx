import { type ReactNode } from "react";
import { ExternalLink, Eye, FolderOpen, PencilLine, ShieldAlert } from "lucide-react";
import {
  computeFileRelationship,
  describeCreatorForInspector,
  describeLibraryFamilyContext,
  describeTrayIdentity,
  describeTraySummary,
  describeVersionForInspector,
  extractParentFolder,
  formatLibraryFileFormat,
  groupedFilesLabel,
  summarizeLibraryCareState,
  summarizeLibraryResourceBadge,
  summarizeLibraryScriptContent,
  summarizeScriptScopeForUi,
  summarizeVersionSignalForUi,
  trayKindLabel,
  trayLocationLabel,
  typeColorForKind,
  usefulTrayGroupingValue,
  type FolderSummaryData,
  type FolderSummaryMode,
} from "./libraryDisplay";
import { friendlyTypeLabel } from "../../lib/uiLanguage";
import type { FileDetail, FileRelationship, UserView, WatchStatus } from "../../lib/types";

interface LibraryDetailsPanelProps {
  userView: UserView;
  selectedFile: FileDetail | null;
  onOpenInspectDetails: () => void;
  onOpenHealthDetails: () => void;
  onOpenEditDetails: () => void;
  onOpenUpdates: () => void;
  onOpenFolder?: (path: string) => void;
  onNavigateDuplicates?: (fileIds: number[]) => void;
  onNavigateNeedsReview?: (fileId: number) => void;
  /** Optional content rendered at the top-right of the detail header (e.g. collapse button) */
  headerRight?: ReactNode;
  /** Pre-computed relationship signal from LibraryScreen. */
  relationship?: FileRelationship | null;
  /** Pre-computed parent folder name. */
  folderName?: string | null;
  /** Pre-computed folder summary (when a folder is selected instead of a file). */
  folderSummary?: FolderSummaryData | null;
  /** The selected folder path (when in folder view, null otherwise). */
  folderPath?: string | null;
  /** Full Windows absolute path for the selected folder (for "Open folder"). */
  folderFullPath?: string | null;
}

export function LibraryDetailsPanel({
  userView,
  selectedFile,
  onOpenInspectDetails,
  onOpenHealthDetails,
  onOpenEditDetails,
  onOpenUpdates,
  onOpenFolder = () => {},
  onNavigateDuplicates,
  onNavigateNeedsReview,
  headerRight,
  relationship: relationshipProp,
  folderName: folderNameProp,
  folderSummary: folderSummaryProp,
  folderPath: folderPathProp,
  folderFullPath: folderFullPathProp,
}: LibraryDetailsPanelProps) {
  const isCasual = userView === "beginner";
  const isPower = userView === "power";

  // ── Folder summary view ─────────────────────────────────────────────────
  // Shown when: folder view is active (folderPath is set), no file selected.
  if (!selectedFile && folderSummaryProp && folderPathProp) {
    return (
      <FolderSummaryPanel
        userView={userView}
        data={folderSummaryProp}
        folderPath={folderPathProp}
        folderFullPath={folderFullPathProp}
        onOpenFolder={onOpenFolder}
        headerRight={headerRight}
      />
    );
  }

  // ── Empty state ────────────────────────────────────────────────────────
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
  const scriptContentSummary = summarizeLibraryScriptContent(selectedFile);
  const scriptNamespace = summarizeScriptScopeForUi(selectedFile.insights);
  const scriptVersionClue = summarizeVersionSignalForUi(selectedFile.insights, 0.8);
  const resourceBadge = summarizeLibraryResourceBadge(selectedFile);
  const creatorInfo = describeCreatorForInspector(selectedFile);
  const trayIdentity = describeTrayIdentity(selectedFile);
  const isTrayKind = trayIdentity.kind !== "standard";
  const familyContext = !isCasual ? describeLibraryFamilyContext(selectedFile) : null;
  const trayGroupingValue = usefulTrayGroupingValue(selectedFile);
  const trayGroupedCount = groupedFilesLabel(selectedFile.groupedFileCount);
  const traySummary = isTrayKind ? describeTraySummary(selectedFile) : null;
  const hasHealthDetails = Boolean(
    selectedFile.bundleName ||
      selectedFile.watchResult ||
      selectedFile.installedVersionSummary ||
      hasSafetyNotes ||
      hasParserWarnings ||
      isTrayKind,
  );

  const isTray = selectedFile.sourceLocation === "tray";
  const typeColor = typeColorForKind(selectedFile.kind);
  const confidenceColor =
    selectedFile.confidence >= 0.8
      ? "high"
      : selectedFile.confidence >= 0.55
        ? "medium"
        : "low";

  // ── Folder + relationship context ─────────────────────────────────────
  // relationship may be passed in from LibraryScreen (pre-computed with full items list)
  // folderName may be passed in or computed fresh from path
  const relationship = relationshipProp ?? computeFileRelationship(selectedFile, []);
  const folderName = folderNameProp ?? extractParentFolder(selectedFile.path);

  // ─── Snapshot — view-aware ───────────────────────────────────────────────
  // Casual: only what matters for a quick read — creator, type, watch
  // Seasoned: adds installed version and confidence for maintenance workflows
  // Creator: adds file format for deep inspection
  // ────────────────────────────────────────────────────────────────────────
  // Casual: no suffix (no raw confidence jargon for beginners)
  // Seasoned / Creator: show disclosure suffix when creator is uncertain
  const creatorSuffix = !isCasual && creatorInfo.suffix ? (
    <span className="detail-creator-suffix" title={`Creator ${creatorInfo.suffix}`}>
      {` (${creatorInfo.suffix})`}
    </span>
  ) : null;

  const snapshotLines: Array<{ label: string; value: ReactNode }> = [
    {
      label: "Creator",
      value: (
        <span>
          {creatorInfo.label}
          {creatorSuffix}
        </span>
      ),
    },
    { label: "Type", value: friendlyTypeLabel(selectedFile.kind) },
  ];

  if (!isCasual && isTrayKind) {
    snapshotLines.push({
      label: "Tray type",
      value: (
        <span>
          {trayKindLabel(trayIdentity.kind)}
          <span className="detail-row-suffix"> ({trayIdentity.evidenceKind})</span>
        </span>
      ),
    });
    snapshotLines.push({
      label: "Stored",
      value: trayIdentity.isMisplaced
        ? `${trayLocationLabel(trayIdentity.location)} · review needed`
        : trayLocationLabel(trayIdentity.location),
    });
    if (trayGroupingValue) {
      snapshotLines.push({ label: "Grouped as", value: trayGroupingValue });
    }
    if (trayGroupedCount) {
      snapshotLines.push({ label: "Tray set", value: trayGroupedCount });
    }
  }

  // Subtype: useful for CAS/Gameplay categorization
  if (!isCasual && selectedFile.subtype?.trim() && !isTrayKind) {
    snapshotLines.push({ label: "Subtype", value: selectedFile.subtype });
  }

  // Family context — derived clue, shown in seasoned+ (not casual)
  if (!isCasual && familyContext) {
    snapshotLines.push({
      label: "Family",
      value: <span className="ghost-chip">{familyContext}</span>,
    });
  }

  if (!isCasual && scriptNamespace) {
    snapshotLines.push({
      label: "Namespace",
      value: <span className="ghost-chip">{scriptNamespace}</span>,
    });
  }

  if (!isCasual && scriptContentSummary) {
    snapshotLines.push({
      label: "Script content",
      value: <span className="ghost-chip">{scriptContentSummary}</span>,
    });
  } else if (!isCasual && resourceBadge) {
    snapshotLines.push({
      label: "Contents",
      value: <span className="ghost-chip">{resourceBadge}</span>,
    });
  }

  if (!isCasual && scriptVersionClue) {
    snapshotLines.push({
      label: "Version clue",
      value: <span className="ghost-chip">{scriptVersionClue}</span>,
    });
  }

  // Installed version — show only when we have a label (confidence not unknown)
  if (!isCasual) {
    const versionInfo = describeVersionForInspector(
      selectedFile.installedVersionSummary?.version ?? null,
      selectedFile.installedVersionSummary?.confidence ?? null,
    );
    if (versionInfo.label) {
      snapshotLines.push({
        label: "Installed",
        value: versionInfo.label,
      });
    }
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

  // ─── Relationship signal — shown in seasoned+ (not casual) ──────────────
  // Location row removed (Ariadne Phase 5an): redundant with More Details full path.
  // "Open folder" button is the actionable alternative.
  if (!isCasual && relationship && relationship.type !== "none") {
    snapshotLines.push({
      label: "Related",
      value: (
        <span>
          {relationship.label}{" "}
          <span
            className={`detail-row-suffix detail-row-suffix--${relationship.proofLevel}`}
            title={`This relationship is ${relationship.proofLevel}`}
          >
            ({relationship.proofLevel})
          </span>
        </span>
      ),
    });
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

      {/* ── Preview — real thumbnail or honest category fallback ── */}
      <section className="detail-preview-section">
        <div className="section-label">Preview</div>
        {/* Cascade: embedded THUM → game cache → fallback */}
        {selectedFile.insights?.thumbnailPreview ?? selectedFile.insights?.cachedThumbnailPreview ? (
          <div className="detail-preview-image-wrap">
            <img
              src={`data:image/png;base64,${selectedFile.insights?.thumbnailPreview ?? selectedFile.insights?.cachedThumbnailPreview}`}
              alt={`Preview for ${selectedFile.filename}`}
              className="detail-preview-image"
            />
          </div>
        ) : (
          <div
            className={`detail-preview-fallback detail-preview-fallback--${typeColor}`}
            title={`${friendlyTypeLabel(selectedFile.kind)} — no preview available`}
          >
            <div className="fallback-icon-wrap">
              {/* CAS */}
              {selectedFile.kind === "CAS" && (
                <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="rgba(255,255,255,0.75)" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                  <path d="M6 2 3 6v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2V6l-3-4z"/>
                  <line x1="3" y1="6" x2="21" y2="6"/>
                  <path d="M16 10a4 4 0 0 1-8 0"/>
                </svg>
              )}
              {/* BuildBuy */}
              {(selectedFile.kind === "BuildBuy" || selectedFile.kind === "OverridesAndDefaults") && (
                <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="rgba(255,255,255,0.75)" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                  <path d="m3 9 9-7 9 7v11a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"/>
                  <polyline points="9 22 9 12 15 12 15 22"/>
                </svg>
              )}
              {/* ScriptMods */}
              {selectedFile.kind === "ScriptMods" && (
                <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="rgba(255,255,255,0.75)" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                  <polyline points="16 18 22 12 16 6"/>
                  <polyline points="8 6 2 12 8 18"/>
                </svg>
              )}
              {/* Tray */}
              {(selectedFile.kind === "TrayHousehold" || selectedFile.kind === "TrayItem" || selectedFile.kind === "TrayLot" || selectedFile.kind === "TrayRoom" || selectedFile.kind === "Household" || selectedFile.kind === "Lot" || selectedFile.kind === "Room") && (
                <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="rgba(255,255,255,0.75)" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                  <path d="M21 16V8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16z"/>
                </svg>
              )}
              {/* Default */}
              {(["Gameplay", "PosesAndAnimation", "PresetsAndSliders", "Unknown"].includes(selectedFile.kind)) && (
                <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="rgba(255,255,255,0.75)" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                  <rect x="3" y="3" width="18" height="18" rx="2" ry="2"/>
                  <circle cx="8.5" cy="8.5" r="1.5"/>
                  <polyline points="21 15 16 10 5 21"/>
                </svg>
              )}
            </div>
            <span className="detail-preview-no-label">No preview available</span>
          </div>
        )}
      </section>

      {/* ── Snapshot — view-aware depth ── */}
      <section className="library-details-card">
        <div className="section-label">At a glance</div>
        <div className="detail-list">
          {snapshotLines.map((line) => (
            <DetailLine key={line.label} label={line.label} value={line.value} />
          ))}
        </div>
      </section>

      {/* ── Care — view-aware depth ── */}
      <section className="library-details-card">
        <div className="section-label">
          {hasSafetyNotes || hasParserWarnings ? "Needs attention" : "Care"}
        </div>
        <p className="library-care-summary">{careSummary}</p>
        {traySummary ? <p className="library-care-summary">{traySummary}</p> : null}

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
            Open More details to inspect file clues and warnings.
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

      {/* ── Safe-delete pre-checks — shown when file has duplicates or is a script mod ── */}
      {(() => {
        const isScriptMod =
          selectedFile.kind.includes("Script") || /\.ts4script$/i.test(selectedFile.filename);
        if (!isScriptMod && !(selectedFile.duplicateTypes ?? []).includes("exact")) return null;
        const warnings: Array<{ text: string; actionLabel?: string; onAction?: () => void }> = [];
        if ((selectedFile.duplicateTypes ?? []).includes("exact")) {
          warnings.push({
            text: "This file has exact duplicates — disabling or deleting it may break saves that reference the other copy.",
            actionLabel: onNavigateDuplicates ? "View duplicates →" : undefined,
            onAction: onNavigateDuplicates ? () => onNavigateDuplicates([selectedFile.id]) : undefined,
          });
        }
        if (isScriptMod) {
          warnings.push({
            text: "This appears to be a script mod — disabling it may break mods that depend on its scripts or namespace.",
          });
        }
        return (
          <section className="library-details-card library-safedelete-warning">
            <div className="section-label">⚠ Delete carefully</div>
            <div className="detail-list">
              {warnings.map((w) => (
                <div key={w.text} className="detail-row detail-row--block">
                  <span>Check first</span>
                  <strong>{w.text}</strong>
                  {w.actionLabel && w.onAction ? (
                    <div style={{ marginTop: "0.45rem" }}>
                      <button type="button" className="secondary-action" onClick={w.onAction}>
                        {w.actionLabel}
                      </button>
                    </div>
                  ) : null}
                </div>
              ))}
            </div>
          </section>
        );
      })()}

      {/* ── More actions — view-aware ── */}
      <section className="library-details-card">
        <div className="section-label">Open</div>
        {!isCasual ? (
          <p className="text-muted library-details-actions-copy">
            Inspect first, then open warnings, updates, or fixes only if you need them.
          </p>
        ) : null}
        <div className="library-details-actions">
          <button
            type="button"
            className="primary-action library-details-action-primary"
            onClick={onOpenInspectDetails}
          >
            <Eye size={14} strokeWidth={2} />
            {isCasual ? "More details" : "Inspect file"}
          </button>

          <div className="library-details-actions-grid">
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

            {isPower ? (
              <button
                type="button"
                className="secondary-action"
                onClick={() => {
                  onOpenFolder(selectedFile.path);
                }}
                title={selectedFile.path}
              >
                <FolderOpen size={14} strokeWidth={2} />
                Open folder
              </button>
            ) : null}
          </div>
        </div>
      </section>
    </div>
  );
}

function DetailLine({ label, value }: { label: string; value: ReactNode }) {
  return (
    <div className="detail-row">
      <span className="detail-row-label">{label}</span>
      <strong className="detail-row-value">
        <span className="detail-row-value-content">{value}</span>
      </strong>
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


// ─── FolderSummaryPanel (Phase 5ao) ─────────────────────────────────────

interface FolderSummaryPanelProps {
  userView: UserView;
  data: FolderSummaryData;
  folderPath: string;
  /** Full Windows absolute path — used for "Open folder" so Explorer opens the right place. */
  folderFullPath?: string | null;
  onOpenFolder?: (path: string) => void;
  headerRight?: ReactNode;
}

function FolderSummaryPanel({
  userView,
  data,
  folderPath,
  folderFullPath,
  onOpenFolder,
  headerRight,
}: FolderSummaryPanelProps) {
  const isPower = userView === "power";

  // Build summary sentence
  const totalLabel = `${data.counts.totalFiles} file${data.counts.totalFiles !== 1 ? "s" : ""}`;
  const subfolderLabel = data.counts.subfolderCount > 0
    ? ` in ${data.counts.subfolderCount} subfolder${data.counts.subfolderCount !== 1 ? "s" : ""}`
    : "";
  const dominantLabel = data.dominantKind
    ? `. Mostly ${data.dominantKind.replace(/([A-Z])/g, " $1").trim()}`
    : "";
  const topCreatorLabel = data.creatorDistribution[0]
    ? ` · ${data.creatorDistribution[0].count} from ${data.creatorDistribution[0].label}`
    : "";
  const summarySentence = `${totalLabel}${subfolderLabel}${dominantLabel}${topCreatorLabel}.`;

  return (
    <div className="library-folder-summary">
      {/* ── Header ─────────────────────────────────────────────────────── */}
      <div className="library-details-hero folder-summary-hero">
        <div className="folder-summary-hero-icon">
          <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
            <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"/>
          </svg>
        </div>
        <div className="folder-summary-hero-text">
          <div className="folder-summary-folder-name">{data.folderName}</div>
          <div className="folder-summary-path">{data.folderPath}</div>
        </div>
        {headerRight && (
          <div className="folder-summary-header-right">{headerRight}</div>
        )}
      </div>

      {/* ── Summary sentence ────────────────────────────────────────────── */}
      <div className="folder-summary-sentence">{summarySentence}</div>

      {/* ── Stat tiles ─────────────────────────────────────────────────── */}
      <div className="folder-summary-stats">
        <div className="folder-stat-tile">
          <span className="folder-stat-tile__value">{data.counts.totalFiles.toLocaleString()}</span>
          <span className="folder-stat-tile__label">Files</span>
        </div>
        <div className="folder-stat-tile">
          <span className="folder-stat-tile__value">{data.counts.subfolderCount.toLocaleString()}</span>
          <span className="folder-stat-tile__label">Subfolders</span>
        </div>
        {data.counts.duplicateCount > 0 && (
          <div className="folder-stat-tile folder-stat-tile--alert">
            <span className="folder-stat-tile__value">{data.counts.duplicateCount}</span>
            <span className="folder-stat-tile__label">Duplicates</span>
          </div>
        )}
      </div>

      {/* ── Type distribution ──────────────────────────────────────────── */}
      {data.kindDistribution.length > 0 && (
        <div className="folder-summary-section">
          <div className="section-label">What&apos;s in here</div>
          <div className="folder-type-list">
            {data.kindDistribution.slice(0, isPower ? undefined : 3).map((item) => (
              <div key={item.key} className="folder-type-row">
                <span className="folder-type-row__label">{item.label}</span>
                <div className="folder-type-row__bar-wrap">
                  <div
                    className="folder-type-row__bar"
                    style={{ width: `${Math.round(item.percentage)}%` }}
                  />
                </div>
                <span className="folder-type-row__count">
                  {item.count}{!isPower && data.kindDistribution.length > 3 ? ` (${Math.round(item.percentage)}%)` : ""}
                </span>
              </div>
            ))}
            {!isPower && data.kindDistribution.length > 3 && (
              <div className="folder-type-more">+{data.kindDistribution.length - 3} more types</div>
            )}
          </div>
        </div>
      )}

      {/* ── Top creators ───────────────────────────────────────────────── */}
      {data.creatorDistribution.length > 0 && (
        <div className="folder-summary-section">
          <div className="section-label">Top creators</div>
          <div className="folder-creator-list">
            {data.creatorDistribution.slice(0, isPower ? 8 : 3).map((item) => (
              <div key={item.key} className="folder-creator-row">
                <span className="folder-creator-row__label">{item.label}</span>
                <span className="folder-creator-row__count">{item.count}</span>
              </div>
            ))}
          </div>
          {!isPower && data.creatorDistribution.length === 0 && data.counts.totalFiles > 5 && (
            <div className="folder-summary-note">Creator info missing on most files</div>
          )}
        </div>
      )}

      {/* ── Relationship clusters ──────────────────────────────────────── */}
      {data.relationshipClusters.length > 0 && (
        <div className="folder-summary-section">
          <div className="section-label">Connections in this folder</div>
          <div className="folder-cluster-list">
            {data.relationshipClusters.map((cluster) => (
              <div key={cluster.id} className="folder-cluster-row">
                <div className="folder-cluster-row__header">
                  <span className={`folder-cluster-badge folder-cluster-badge--${cluster.confidenceLabel.toLowerCase()}`}>
                    {cluster.confidenceLabel}
                  </span>
                  <span className="folder-cluster-row__title">{cluster.title}</span>
                </div>
                <div className="folder-cluster-row__desc">{cluster.description}</div>
                {isPower && (
                  <div className="folder-cluster-row__meta">
                    {cluster.affectedFileCount} affected · proof: {cluster.proofLevel}
                  </div>
                )}
              </div>
            ))}
          </div>
        </div>
      )}

      {/* ── Notes / caveats ────────────────────────────────────────────── */}
      {data.notes.length > 0 && (
        <div className="folder-summary-section">
          {data.notes.map((note, i) => (
            <div key={i} className="folder-summary-note">{note}</div>
          ))}
        </div>
      )}

      {/* ── Open folder action ─────────────────────────────────────────── */}
      {onOpenFolder && (
        <div className="folder-summary-actions">
          <button
            type="button"
            className="folder-open-btn"
            onClick={() => onOpenFolder(folderFullPath ?? data.folderPath)}
            title="Open this folder in Windows Explorer"
          >
            <FolderOpen size={13} strokeWidth={2} />
            Open folder
          </button>
        </div>
      )}
    </div>
  );
}

