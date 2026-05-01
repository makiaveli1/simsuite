import { AlertTriangle, ExternalLink } from "lucide-react";
import type {
  ActionPreflightActionType,
  ActionPreflightDecision,
  ActionPreflightProofLevel,
  ActionPreflightRoute,
  ActionPreflightSeverity,
  ActionPreflightSignal,
  ActionPreflightSummary,
  FileDetail,
  FileRelationship,
  ProblemSignal,
  ProblemSignalProofLevel,
  ProblemSignalSeverity,
} from "../../lib/types";
import {
  describeRelationshipMeaning,
  proofLevelToLabel,
  relationshipCountLabel,
  relationshipTypeLabel,
} from "./libraryDisplay";

interface ActionPreflightDetailProps {
  preflight: ActionPreflightSummary | null;
  onOpenNeedsReview?: () => void;
  onOpenDuplicates?: () => void;
  onOpenUpdates?: () => void;
}

interface ActionPreflightCompactProps extends ActionPreflightDetailProps {
  onOpenHealthDetails: () => void;
}

const SIGNAL_RANK: Record<ActionPreflightSeverity, number> = {
  blocked: 4,
  warning: 3,
  caution: 2,
  info: 1,
};

const PROOF_RANK: Record<ActionPreflightProofLevel, number> = {
  confirmed: 4,
  detected: 3,
  inferred: 2,
  unavailable: 1,
};

const ROUTE_RANK: Record<ActionPreflightRoute, number> = {
  review: 4,
  duplicates: 3,
  updates: 2,
  organize: 1,
};

const ROUTE_LABEL: Record<ActionPreflightRoute, string> = {
  review: "Open Needs Review",
  duplicates: "Open in Duplicates",
  updates: "Open in Updates",
  organize: "Open Organize",
};

export function buildFileActionPreflight(
  file: FileDetail,
  relationship: FileRelationship | null,
  actionType: ActionPreflightActionType = "change",
): ActionPreflightSummary {
  const signals: ActionPreflightSignal[] = [];

  const reviewSignal = buildReviewSignal(file.problemSignals ?? []);
  if (reviewSignal) {
    signals.push(reviewSignal);
  }

  const weakMetadataSignal = (file.problemSignals ?? []).find(
    (signal) => signal.signalType === "weak_metadata",
  );
  if (weakMetadataSignal) {
    signals.push(
      mapProblemSignal(weakMetadataSignal, {
        id: "weak-metadata",
        route: null,
        severity: "caution",
      }),
    );
  }

  const duplicateSignal = buildDuplicateSignal(file.problemSignals ?? [], file);
  if (duplicateSignal) {
    signals.push(duplicateSignal);
  }

  const updatesSignal = (file.problemSignals ?? []).find(
    (signal) => signal.signalType === "no_update_source",
  );
  if (updatesSignal) {
    signals.push(
      mapProblemSignal(updatesSignal, {
        id: "no-update-source",
        route: "updates",
        severity: "info",
      }),
    );
  }

  const traySignal = (file.problemSignals ?? []).find(
    (signal) => signal.signalType === "stored_in_tray",
  );
  if (traySignal) {
    signals.push(
      mapProblemSignal(traySignal, {
        id: "stored-in-tray",
        route: null,
        severity: "info",
      }),
    );
  }

  const scriptCautionSignal = (file.problemSignals ?? []).find(
    (signal) => signal.signalType === "script_mod_caution",
  );
  if (scriptCautionSignal) {
    signals.push(
      mapProblemSignal(scriptCautionSignal, {
        id: "script-mod-caution",
        route: null,
        severity: "info",
      }),
    );
  }

  const relationshipSignal = buildRelationshipSignal(file, relationship);
  if (relationshipSignal) {
    signals.push(relationshipSignal);
  }

  const dedupedSignals = dedupeSignals(signals).sort(compareSignals);
  const primarySignal = dedupedSignals[0] ?? null;
  const severity = primarySignal?.severity ?? "info";
  const proofLevel = primarySignal?.proofLevel ?? "unavailable";
  const recommendedRoute = primarySignal?.route ?? null;
  const decision: ActionPreflightDecision =
    severity === "blocked"
      ? "blocked"
      : dedupedSignals.some((signal) => signal.severity === "warning" || signal.severity === "caution")
        ? "caution"
        : "allow";

  return {
    actionType,
    fileId: file.id,
    filename: file.filename,
    fileCount: 1,
    decision,
    severity,
    proofLevel,
    recommendedRoute,
    title: "Before you change this file",
    summary: buildPreflightSummary(actionType, dedupedSignals.length),
    disclaimer:
      "SimSuite has limited information here. These notes do not prove the file is safe or unsafe.",
    signals: dedupedSignals,
  };
}

export function ActionPreflightCompact({
  preflight,
  onOpenHealthDetails,
  onOpenNeedsReview,
  onOpenDuplicates,
  onOpenUpdates,
}: ActionPreflightCompactProps) {
  if (!preflight || preflight.signals.length === 0) {
    return null;
  }

  const primaryRoute = buildRouteButton(preflight.recommendedRoute, {
    onOpenNeedsReview,
    onOpenDuplicates,
    onOpenUpdates,
  });

  return (
    <section className="library-details-card action-preflight-card action-preflight-card--compact">
      <div className="section-label">Before changing</div>
      <p className="library-care-summary">{preflight.summary}</p>
      <div className="tag-list action-preflight-chip-list">
        {preflight.signals.slice(0, 2).map((signal) => (
          <span key={signal.id} className={`preflight-pill is-${signal.severity}`}>
            {signal.label}
          </span>
        ))}
      </div>
      <div className="library-details-actions-grid action-preflight-route-grid">
        <button type="button" className="secondary-action" onClick={onOpenHealthDetails}>
          <AlertTriangle size={14} strokeWidth={2} />
          Review cautions
        </button>
        {primaryRoute}
      </div>
    </section>
  );
}

export function ActionPreflightDetail({
  preflight,
  onOpenNeedsReview,
  onOpenDuplicates,
  onOpenUpdates,
}: ActionPreflightDetailProps) {
  if (!preflight || preflight.signals.length === 0) {
    return null;
  }

  const routes = uniqueRoutes(preflight.signals)
    .map((route) =>
      buildRouteButton(route, {
        onOpenNeedsReview,
        onOpenDuplicates,
        onOpenUpdates,
      }),
    )
    .filter(Boolean);

  return (
    <div className="detail-block action-preflight-detail-block">
      <div className="section-label">Before you change this file</div>
      <p className="library-care-summary">{preflight.summary}</p>
      <p className="text-muted action-preflight-disclaimer">{preflight.disclaimer}</p>
      <div className="detail-list action-preflight-signal-list">
        {preflight.signals.map((signal) => (
          <div key={signal.id} className="detail-row detail-row--block action-preflight-row">
            <div className="action-preflight-row-meta">
              <span className={`preflight-pill is-${signal.severity}`}>{signal.label}</span>
              <span className="ghost-chip">{proofLabel(signal.proofLevel)}</span>
            </div>
            <strong>{signal.explanation}</strong>
            {signal.evidence.length ? (
              <div className="tag-list action-preflight-evidence-list">
                {signal.evidence.slice(0, 3).map((item) => (
                  <span key={`${signal.id}:${item}`} className="ghost-chip">
                    {item}
                  </span>
                ))}
              </div>
            ) : null}
          </div>
        ))}
      </div>
      {routes.length ? <div className="action-preflight-route-grid">{routes}</div> : null}
    </div>
  );
}

function buildReviewSignal(problemSignals: ProblemSignal[]): ActionPreflightSignal | null {
  const reviewSignals = problemSignals.filter(
    (signal) => signal.destination === "review" && signal.showInNeedsReview,
  );
  if (!reviewSignals.length) {
    return null;
  }

  const primary = [...reviewSignals].sort((left, right) => compareProblemSignals(right, left))[0];
  return {
    id: "review-suggested",
    signalType: primary.signalType,
    severity: primary.signalType === "inspection_failed" ? "warning" : mapProblemSeverity(primary.severity),
    proofLevel: mapProblemProof(primary.proofLevel),
    label: primary.shortLabel,
    explanation: primary.explanation,
    evidence: uniqueStrings(reviewSignals.flatMap((signal) => signal.evidence)),
    route: "review",
    source: primary.source,
  };
}

function buildDuplicateSignal(
  problemSignals: ProblemSignal[],
  file: Pick<FileDetail, "duplicatesCount" | "duplicateTypes">,
): ActionPreflightSignal | null {
  const problemSignal = problemSignals.find((signal) => signal.destination === "duplicates");
  if (problemSignal) {
    return mapProblemSignal(problemSignal, {
      id: "duplicate-candidate",
      route: "duplicates",
      severity: "caution",
    });
  }

  if (!file.duplicatesCount) {
    return null;
  }

  return {
    id: "duplicate-candidate",
    signalType: "duplicate_candidate",
    severity: "caution",
    proofLevel: "confirmed",
    label: "Duplicate candidate",
    explanation:
      file.duplicatesCount === 1
        ? "SimSuite found one matching duplicate pair for this file, so compare the files before changing either copy."
        : `SimSuite found ${file.duplicatesCount} matching duplicate pairs for this file, so compare the files before changing any copy.`,
    evidence: file.duplicateTypes.map(humanizeDuplicateType),
    route: "duplicates",
    source: "duplicate_types",
  };
}

function buildRelationshipSignal(
  file: FileDetail,
  relationship: FileRelationship | null,
): ActionPreflightSignal | null {
  if (!relationship || relationship.type === "none" || relationship.type === "duplicate") {
    return null;
  }

  const countLabel = relationshipCountLabel(relationship);
  return {
    id: `relationship-${relationship.type}`,
    signalType: relationship.type,
    severity: "caution",
    proofLevel: mapRelationshipProof(relationship.proofLevel),
    label: relationshipTypeLabel(relationship.type),
    explanation: `${describeRelationshipMeaning(relationship, file)} This does not mean the files depend on each other.`,
    evidence: uniqueStrings([countLabel, proofLevelToLabel(relationship.proofLevel)].filter(Boolean) as string[]),
    route: null,
    source: relationship.evidenceSource ?? "relationship_hint",
  };
}

function buildPreflightSummary(actionType: ActionPreflightActionType, signalCount: number) {
  if (signalCount <= 0) {
    return "No obvious warning was found before this action.";
  }

  const actionCopy =
    actionType === "delete"
      ? "before you remove this file"
      : actionType === "disable"
        ? "before you disable this file"
        : actionType === "move" || actionType === "organize"
          ? "before you move this file"
          : actionType === "update" || actionType === "replace"
            ? "before you replace this file"
            : "before you move, disable, or remove this file";

  return signalCount === 1
    ? `SimSuite found 1 thing to check ${actionCopy}.`
    : `SimSuite found ${signalCount} things to check ${actionCopy}.`;
}

function buildRouteButton(
  route: ActionPreflightRoute | null,
  handlers: {
    onOpenNeedsReview?: () => void;
    onOpenDuplicates?: () => void;
    onOpenUpdates?: () => void;
  },
) {
  if (!route) {
    return null;
  }

  const onClick =
    route === "review"
      ? handlers.onOpenNeedsReview
      : route === "duplicates"
        ? handlers.onOpenDuplicates
        : route === "updates"
          ? handlers.onOpenUpdates
          : undefined;

  if (!onClick) {
    return null;
  }

  return (
    <button key={route} type="button" className="secondary-action" onClick={onClick}>
      <ExternalLink size={14} strokeWidth={2} />
      {ROUTE_LABEL[route]}
    </button>
  );
}

function dedupeSignals(signals: ActionPreflightSignal[]) {
  const seen = new Set<string>();
  return signals.filter((signal) => {
    if (seen.has(signal.id)) {
      return false;
    }
    seen.add(signal.id);
    return true;
  });
}

function compareSignals(left: ActionPreflightSignal, right: ActionPreflightSignal) {
  return compareActionSignals(right, left);
}

function compareActionSignals(left: ActionPreflightSignal, right: ActionPreflightSignal) {
  return (
    SIGNAL_RANK[left.severity] - SIGNAL_RANK[right.severity] ||
    (left.route && right.route ? ROUTE_RANK[left.route] - ROUTE_RANK[right.route] : left.route ? 1 : right.route ? -1 : 0) ||
    PROOF_RANK[left.proofLevel] - PROOF_RANK[right.proofLevel]
  );
}

function compareProblemSignals(left: ProblemSignal, right: ProblemSignal) {
  return (
    problemSignalSeverityRank(left.severity) - problemSignalSeverityRank(right.severity) ||
    problemSignalProofRank(left.proofLevel) - problemSignalProofRank(right.proofLevel)
  );
}

function problemSignalSeverityRank(severity: ProblemSignalSeverity) {
  switch (severity) {
    case "severe":
      return 4;
    case "warning":
      return 3;
    case "caution":
      return 2;
    default:
      return 1;
  }
}

function problemSignalProofRank(proof: ProblemSignalProofLevel) {
  switch (proof) {
    case "confirmed":
      return 4;
    case "detected":
      return 3;
    case "inferred":
      return 2;
    default:
      return 1;
  }
}

function mapProblemSignal(
  signal: ProblemSignal,
  overrides: {
    id: string;
    route: ActionPreflightRoute | null;
    severity?: ActionPreflightSeverity;
  },
): ActionPreflightSignal {
  return {
    id: overrides.id,
    signalType: signal.signalType,
    severity: overrides.severity ?? mapProblemSeverity(signal.severity),
    proofLevel: mapProblemProof(signal.proofLevel),
    label: signal.shortLabel,
    explanation: signal.explanation,
    evidence: signal.evidence,
    route: overrides.route,
    source: signal.source,
  };
}

function mapProblemSeverity(severity: ProblemSignalSeverity): ActionPreflightSeverity {
  switch (severity) {
    case "severe":
      return "blocked";
    case "warning":
      return "warning";
    case "caution":
      return "caution";
    default:
      return "info";
  }
}

function mapProblemProof(proof: ProblemSignalProofLevel): ActionPreflightProofLevel {
  switch (proof) {
    case "confirmed":
      return "confirmed";
    case "detected":
      return "detected";
    case "inferred":
      return "inferred";
    default:
      return "unavailable";
  }
}

function mapRelationshipProof(proof: FileRelationship["proofLevel"]): ActionPreflightProofLevel {
  switch (proof) {
    case "fact":
      return "confirmed";
    case "claim":
      return "detected";
    default:
      return "inferred";
  }
}

function humanizeDuplicateType(type: string) {
  switch (type) {
    case "exact":
      return "Exact duplicate match";
    case "filename":
      return "Same filename match";
    case "version":
      return "Same version match";
    default:
      return type;
  }
}

function uniqueStrings(values: string[]) {
  return Array.from(new Set(values.filter(Boolean)));
}

function uniqueRoutes(signals: ActionPreflightSignal[]) {
  return Array.from(new Set(signals.map((signal) => signal.route).filter(Boolean))) as ActionPreflightRoute[];
}

function proofLabel(proofLevel: ActionPreflightProofLevel) {
  switch (proofLevel) {
    case "confirmed":
      return "Confirmed";
    case "detected":
      return "Detected";
    case "inferred":
      return "Possible";
    default:
      return "Unavailable";
  }
}
