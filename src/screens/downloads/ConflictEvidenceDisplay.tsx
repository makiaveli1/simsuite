/**
 * ConflictEvidenceDisplay — read-only Inbox diagnostic, tiered by user view.
 *
 * SCOPE: Inbox only. Read-only display of backend-owned conflict/comparison data.
 *
 * RULES (enforced here and in usage):
 * - NEVER add state setters, useState, or dispatch calls
 * - NEVER add workflow actions: block, move, dismiss, reclassify, resolve
 * - NEVER import or depend on Library, Updates, Needs Review, or other screens
 * - NEVER cache, suppress, or persist conflict state client-side
 * - NEVER show implied decisions — only evidence
 * - Backend owns all conflict detection — frontend is a display layer only
 *
 * TIERED DEPTH:
 * - Casual (beginner):   One calm plain-language sentence explaining what this means.
 *                        No confidence, no evidence strings, no technical detail.
 * - Seasoned (standard): Short reason + useful context. More than Casual, less than Creator.
 *                        No full evidence string wall. Fast to scan.
 * - Creator (power):     Full evidence: comparison, confidence, version strings, evidence.
 *                        Structured and readable. No duplicated data elsewhere.
 *
 * ALIGNMENT:
 * - Badge language and panel language are consistent — no contradictory signals.
 * - Panel does not duplicate what the row badge already conveys.
 */

import type { SpecialDecisionState, SpecialFamilyRole, SpecialModDecision, VersionConfidence, VersionCompareStatus, VersionResolution } from "../../lib/types";

/** Maps VersionResolution.status to a human-readable comparison label */
function versionLabel(status: VersionCompareStatus): string {
  switch (status) {
    case "incoming_newer": return "Incoming version is newer";
    case "incoming_older": return "Incoming version is older";
    case "same_version":    return "Same version detected";
    case "not_installed":  return "No installed version found";
    case "unknown":        return "Version comparison unclear";
    default:               return "Version unclear";
  }
}

/** Maps VersionResolution.confidence to a display string and severity */
function confidenceLabel(confidence: VersionConfidence): {
  text: string;
  severity: "high" | "medium" | "low";
} {
  switch (confidence) {
    case "exact":   return { text: "High confidence", severity: "high" };
    case "strong":  return { text: "Strong confidence", severity: "high" };
    case "medium":  return { text: "Medium confidence", severity: "medium" };
    case "weak":    return { text: "Weak confidence", severity: "low" };
    case "unknown": return { text: "Unclear — verify first", severity: "low" };
  }
}

/** Maps SpecialDecisionState to a plain-language explanation for Casual */
function casualExplanation(state: SpecialDecisionState): string | null {
  switch (state) {
    case "review_manually":          return "This needs a quick review before applying.";
    case "repair_before_update":      return "This may need repair before it can be updated.";
    case "install_dependency_first":  return "Some required files are missing first.";
    case "open_dependency_item":      return "Another item needs to be opened first.";
    case "open_related_item":        return "This is related to another item.";
    case "separate_supported_files":  return "Some files may need to be handled separately.";
    case "guided_ready":             return null; // not shown — nothing needs attention
    case "open_official_source":      return "This may need to be checked at the source.";
    case "download_missing_files":   return "Some files may need to be downloaded separately.";
  }
}

/** Maps SpecialDecisionState to a short specific reason for Seasoned */
function seasonedReason(state: SpecialDecisionState): string | null {
  switch (state) {
    case "review_manually":          return "manual decision";
    case "repair_before_update":     return "repair first";
    case "install_dependency_first": return "missing dependency";
    case "open_dependency_item":     return "open dependency";
    case "open_related_item":        return "related item";
    case "separate_supported_files":  return "separate files";
    case "guided_ready":             return null; // shown via state badge
    case "open_official_source":     return "check source";
    case "download_missing_files":   return "missing files";
  }
}

/** Maps SpecialFamilyRole to a display string */
function familyRoleLabel(role: SpecialFamilyRole | null | undefined): string {
  switch (role) {
    case "primary":   return "Primary family item";
    case "related":   return "Related family item";
    case "superseded": return "Superseded by newer family item";
    default:           return "Family item";
  }
}

/** Returns true if the VersionResolution signals something worth highlighting */
function versionNeedsAttention(res: VersionResolution): boolean {
  return (
    res.status === "unknown" ||
    res.status === "incoming_older" ||
    res.confidence === "weak" ||
    res.confidence === "unknown"
  );
}

interface ConflictEvidenceDisplayProps {
  versionResolution: VersionResolution | null;
  specialDecision: SpecialModDecision | null;
  userView: "beginner" | "standard" | "power";
}

/**
 * Tiered evidence display for Inbox conflict/comparison diagnostics.
 *
 * Returns null when there is no conflict data to show.
 * Adapts depth and language to the user view.
 */
export function ConflictEvidenceDisplay({
  versionResolution,
  specialDecision,
  userView,
}: ConflictEvidenceDisplayProps) {
  const hasConflict = specialDecision != null || versionResolution != null;

  if (!hasConflict) return null;

  // ── CASUAL ─────────────────────────────────────────────────────────────────
  // One calm sentence. No confidence. No evidence strings. Plain explanation.
  if (userView === "beginner") {
    // Use the specialDecision explanation if available
    if (specialDecision) {
      const explanation = casualExplanation(specialDecision.state);
      if (explanation) {
        return (
          <div className="conflict-evidence-display conflict-evidence-casual">
            <p className="conflict-evidence-casual-sentence">{explanation}</p>
          </div>
        );
      }
      // guided_ready — nothing to explain, stay silent
      return null;
    }

    // Fallback: brief explanation from version resolution
    if (versionResolution && versionNeedsAttention(versionResolution)) {
      let explanation = "This may need a quick review before applying.";
      if (versionResolution.status === "incoming_older") {
        explanation = "This looks like an older version.";
      } else if (
        versionResolution.confidence === "weak" ||
        versionResolution.confidence === "unknown"
      ) {
        explanation = "This may overlap with something you already have.";
      }
      return (
        <div className="conflict-evidence-display conflict-evidence-casual">
          <p className="conflict-evidence-casual-sentence">{explanation}</p>
        </div>
      );
    }

    return null;
  }

  // ── SEASONED ─────────────────────────────────────────────────────────────
  // Short reason + useful context. No full evidence string wall. Scannable.
  if (userView === "standard") {
    // Use specialDecision reason if available and useful
    if (specialDecision) {
      const reason = seasonedReason(specialDecision.state);
      if (reason !== null) {
        return (
          <div className="conflict-evidence-display conflict-evidence-seasoned">
            <div className="conflict-evidence-header">
              <span className="conflict-evidence-label">What</span>
              <span className="conflict-evidence-seasoned-reason">{reason}</span>
            </div>
            {specialDecision.familyRole && (
              <p className="conflict-evidence-seasoned-context">
                {familyRoleLabel(specialDecision.familyRole)}
              </p>
            )}
          </div>
        );
      }
      return null;
    }

    // Version resolution — short explanation when confidence is uncertain
    if (versionResolution && versionNeedsAttention(versionResolution)) {
      let hint: string;
      if (versionResolution.confidence === "weak" || versionResolution.confidence === "unknown") {
        hint = "version unclear — verify first";
      } else if (versionResolution.status === "incoming_older") {
        hint = "incoming is older";
      } else {
        hint = "review recommended";
      }
      return (
        <div className="conflict-evidence-display conflict-evidence-seasoned">
          <div className="conflict-evidence-header">
            <span className="conflict-evidence-label">What</span>
            <span className="conflict-evidence-seasoned-reason">{hint}</span>
          </div>
        </div>
      );
    }

    return null;
  }

  // ── CREATOR ──────────────────────────────────────────────────────────────
  // Full evidence: comparison, confidence, version strings, evidence, family context.
  // Confident and structured. The full diagnostic tool.
  const hasVersionData = versionResolution != null && (
    versionResolution.status !== "not_installed" ||
    versionResolution.incomingEvidence.length > 0 ||
    versionResolution.installedEvidence.length > 0
  );

  const hasDecisionData = specialDecision != null;

  if (!hasVersionData && !hasDecisionData) return null;

  return (
    <div className="conflict-evidence-display">
      <div className="conflict-evidence-header">
        <span className="conflict-evidence-label">Evidence</span>
        {hasDecisionData && (
          <span
            className={`conflict-evidence-badge ${
              specialDecision!.state === "guided_ready"
                ? "is-muted"
                : "is-warn"
            }`}
          >
            {specialDecision!.state === "guided_ready" ? "Ready" : "Review suggested"}
          </span>
        )}
      </div>

      {/* Version comparison block */}
      {hasVersionData && (
        <div className="conflict-evidence-block">
          {/* Confidence first — most scan-worthy signal */}
          <div className="conflict-evidence-row conflict-confidence-row">
            <span className="conflict-evidence-item-label">Confidence</span>
            <span
              className={`conflict-confidence-badge conflict-confidence-${confidenceLabel(versionResolution!.confidence).severity}`}
              title={
                confidenceLabel(versionResolution!.confidence).severity === "low"
                  ? "Backend detection is uncertain. Verify manually before acting."
                  : undefined
              }
            >
              <span className="conflict-confidence-dot" />
              {confidenceLabel(versionResolution!.confidence).text}
            </span>
          </div>

          {/* Comparison */}
          <div className="conflict-evidence-row">
            <span className="conflict-evidence-item-label">Comparison</span>
            <span className="conflict-evidence-item-value">
              {versionLabel(versionResolution!.status)}
            </span>
          </div>

          {/* Version strings */}
          {versionResolution!.incomingVersion && (
            <div className="conflict-evidence-row">
              <span className="conflict-evidence-item-label">Incoming</span>
              <span className="conflict-evidence-item-value conflict-evidence-mono">
                {versionResolution!.incomingVersion}
              </span>
            </div>
          )}
          {versionResolution!.installedVersion && (
            <div className="conflict-evidence-row">
              <span className="conflict-evidence-item-label">Installed</span>
              <span className="conflict-evidence-item-value conflict-evidence-mono">
                {versionResolution!.installedVersion}
              </span>
            </div>
          )}

          {/* Evidence strings */}
          {versionResolution!.incomingEvidence.length > 0 && (
            <div className="conflict-evidence-row">
              <span className="conflict-evidence-item-label">Incoming</span>
              <span className="conflict-evidence-item-value conflict-evidence-mono">
                {versionResolution!.incomingEvidence.join(", ")}
              </span>
            </div>
          )}
          {versionResolution!.installedEvidence.length > 0 && (
            <div className="conflict-evidence-row">
              <span className="conflict-evidence-item-label">Installed</span>
              <span className="conflict-evidence-item-value conflict-evidence-mono">
                {versionResolution!.installedEvidence.join(", ")}
              </span>
            </div>
          )}
          {versionResolution!.evidence.length > 0 && (
            <div className="conflict-evidence-row">
              <span className="conflict-evidence-item-label">Details</span>
              <span className="conflict-evidence-item-value conflict-evidence-mono">
                {versionResolution!.evidence.join(" · ")}
              </span>
            </div>
          )}
        </div>
      )}

      {/* Special decision / family block */}
      {hasDecisionData && (
        <div className="conflict-evidence-block">
          {specialDecision!.familyRole && (
            <div className="conflict-evidence-row">
              <span className="conflict-evidence-item-label">Relationship</span>
              <span className="conflict-evidence-item-value">
                {familyRoleLabel(specialDecision!.familyRole)}
              </span>
            </div>
          )}

          {specialDecision!.installedState.installedVersion && (
            <div className="conflict-evidence-row">
              <span className="conflict-evidence-item-label">Installed version</span>
              <span className="conflict-evidence-item-value conflict-evidence-mono">
                {specialDecision!.installedState.installedVersion}
              </span>
            </div>
          )}

          {specialDecision!.incomingVersion && (
            <div className="conflict-evidence-row">
              <span className="conflict-evidence-item-label">Incoming version</span>
              <span className="conflict-evidence-item-value conflict-evidence-mono">
                {specialDecision!.incomingVersion}
              </span>
            </div>
          )}

          {specialDecision!.incomingVersionEvidence.length > 0 && (
            <div className="conflict-evidence-row">
              <span className="conflict-evidence-item-label">Incoming evidence</span>
              <span className="conflict-evidence-item-value conflict-evidence-mono">
                {specialDecision!.incomingVersionEvidence.join(" · ")}
              </span>
            </div>
          )}
          {specialDecision!.installedVersionEvidence.length > 0 && (
            <div className="conflict-evidence-row">
              <span className="conflict-evidence-item-label">Installed evidence</span>
              <span className="conflict-evidence-item-value conflict-evidence-mono">
                {specialDecision!.installedVersionEvidence.join(" · ")}
              </span>
            </div>
          )}
          {specialDecision!.comparisonEvidence.length > 0 && (
            <div className="conflict-evidence-row">
              <span className="conflict-evidence-item-label">Comparison</span>
              <span className="conflict-evidence-item-value conflict-evidence-mono">
                {specialDecision!.comparisonEvidence.join(" · ")}
              </span>
            </div>
          )}

          {specialDecision!.explanation && specialDecision!.explanation.length > 0 && (
            <div className="conflict-evidence-row conflict-evidence-note">
              <span className="conflict-evidence-item-value">
                {specialDecision!.explanation}
              </span>
            </div>
          )}
        </div>
      )}
    </div>
  );
}
