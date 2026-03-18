import { useEffect, useState } from "react";
import { m } from "motion/react";
import {
  Fingerprint,
  RefreshCw,
  SearchX,
  Shapes,
  ShieldAlert,
  ShieldCheck,
  Workflow,
} from "lucide-react";
import { DockSectionStack } from "../components/DockSectionStack";
import { ResizableDetailPanel } from "../components/ResizableDetailPanel";
import { StatePanel } from "../components/StatePanel";
import { useUiPreferences } from "../components/UiPreferencesContext";
import { api } from "../lib/api";
import { rowHover, rowPress, stagedListItem } from "../lib/motion";
import {
  friendlyTypeLabel,
  reviewLabel,
  screenHelperLine,
  unknownCreatorLabel,
} from "../lib/uiLanguage";
import type {
  ReviewLayoutPreset,
  ReviewQueueItem,
  Screen,
  UserView,
} from "../lib/types";

interface ReviewScreenProps {
  refreshVersion: number;
  onNavigate: (screen: Screen) => void;
  userView: UserView;
}

const REVIEW_LAYOUT_PRESETS: Array<{
  id: ReviewLayoutPreset;
  label: string;
  hint: string;
}> = [
  {
    id: "queue",
    label: "Queue",
    hint: "Gives more room to the left list for triage work.",
  },
  {
    id: "balanced",
    label: "Balanced",
    hint: "Keeps the list and details closer to even.",
  },
  {
    id: "focus",
    label: "Focus",
    hint: "Gives the right panel more space for deeper review.",
  },
];

export function ReviewScreen({
  refreshVersion,
  onNavigate,
  userView,
}: ReviewScreenProps) {
  const {
    reviewDetailWidth,
    setReviewDetailWidth,
    reviewLayoutPreset,
    applyReviewLayoutPreset,
  } = useUiPreferences();
  const [items, setItems] = useState<ReviewQueueItem[]>([]);
  const [selectedId, setSelectedId] = useState<number | null>(null);
  const [isLoading, setIsLoading] = useState(false);

  useEffect(() => {
    void loadReviewQueue();
  }, [refreshVersion]);

  useEffect(() => {
    if (items.length === 0) {
      setSelectedId(null);
      return;
    }

    if (!items.some((item) => item.id === selectedId)) {
      setSelectedId(items[0].id);
    }
  }, [items, selectedId]);

  async function loadReviewQueue() {
    setIsLoading(true);
    try {
      setItems(await api.getReviewQueue("Category First", 80));
    } finally {
      setIsLoading(false);
    }
  }

  const selected = items.find((item) => item.id === selectedId) ?? null;
  const reasonCounts = items.reduce<Record<string, number>>((counts, item) => {
    counts[item.reason] = (counts[item.reason] ?? 0) + 1;
    return counts;
  }, {});
  const reasonRows = Object.entries(reasonCounts).sort((left, right) => right[1] - left[1]);
  const maxReasonCount = reasonRows[0]?.[1] ?? 0;
  const highConfidenceCount = items.filter((item) => item.confidence >= 0.85).length;
  const mediumConfidenceCount = items.filter(
    (item) => item.confidence >= 0.6 && item.confidence < 0.85,
  ).length;
  const lowConfidenceCount = items.filter((item) => item.confidence < 0.6).length;
  const reviewInspectorSections = selected
    ? [
        {
          id: "summary",
          label: userView === "beginner" ? "Why this file stopped" : "Summary",
          hint:
            userView === "beginner"
              ? "Why SimSuite sent this file to review and how sure it is."
              : "Reason, type, and confidence.",
          children: (
            <div className="detail-list">
              <DetailRow
                label={userView === "beginner" ? "Why it was flagged" : "Reason"}
                value={humanize(selected.reason)}
              />
              <DetailRow
                label="Type"
                value={friendlyTypeLabel(selected.kind)}
              />
              {userView !== "beginner" ? (
                <DetailRow label="Subtype" value={selected.subtype ?? "Unspecified"} />
              ) : null}
              <DetailRow
                label="Creator"
                value={selected.creator ?? unknownCreatorLabel(userView)}
              />
              {userView === "power" ? (
                <DetailRow label="Root" value={selected.sourceLocation} />
              ) : null}
            </div>
          ),
        },
        {
          id: "paths",
          label: userView === "beginner" ? "Where it is and where it should go" : "Path",
          hint:
            userView === "beginner"
              ? "Compares the current location with the safer suggestion."
              : "Current and suggested validated path.",
          children: (
            <>
              <div className="detail-block">
                <div className="section-label">
                  {userView === "beginner" ? "Where it is now" : "Current path"}
                </div>
                <div className="path-card">{selected.path}</div>
              </div>

              <div className="detail-block">
                <div className="section-label">
                  {userView === "beginner" ? "Safer place" : "Suggested path"}
                </div>
                <div className="path-card">
                  {selected.suggestedPath ?? "No safe destination yet"}
                </div>
              </div>
            </>
          ),
        },
        ...(userView !== "beginner"
          ? [
              {
                id: "notes",
                label: "Notes",
                hint: "Validator and placement notes for this review item.",
                defaultCollapsed: false,
                badge: selected.safetyNotes.length
                  ? `${selected.safetyNotes.length}`
                  : null,
                children: selected.safetyNotes.length ? (
                  <div className="tag-list">
                    {selected.safetyNotes.map((note) => (
                      <span key={note} className="warning-tag">
                        {note}
                      </span>
                    ))}
                  </div>
                ) : (
                  <p>No additional notes.</p>
                ),
              },
            ]
          : []),
      ]
    : [];

  return (
    <section className="screen-shell workbench workbench-screen">
      <div className="screen-header-row">
        <div className="screen-heading">
          <p className="eyebrow">{reviewLabel(userView)}</p>
          <div className="screen-title-row">
            <ShieldAlert size={18} strokeWidth={2} />
            <h1>{userView === "beginner" ? "Needs review" : "Review queue"}</h1>
          </div>
          <p className="workspace-toolbar-copy">{screenHelperLine("review", userView)}</p>
        </div>
        <div className="header-actions">
          <button
            type="button"
            className="secondary-action"
            onClick={() => void loadReviewQueue()}
            disabled={isLoading}
          >
            <RefreshCw size={14} strokeWidth={2} />
            {isLoading ? "Refreshing..." : "Refresh"}
          </button>
          <button
            type="button"
            className="secondary-action"
            onClick={() => onNavigate("creatorAudit")}
          >
            <Fingerprint size={14} strokeWidth={2} />
            Creators
          </button>
          <button
            type="button"
            className="secondary-action"
            onClick={() => onNavigate("categoryAudit")}
          >
            <Shapes size={14} strokeWidth={2} />
            Types
          </button>
          <button
            type="button"
            className="secondary-action"
            onClick={() => onNavigate("organize")}
          >
            <Workflow size={14} strokeWidth={2} />
            {userView === "beginner" ? "Tidy Up" : "Organize"}
          </button>
        </div>
      </div>

      {items.length ? (
        <div className="review-layout review-layout-screen">
          <div className="review-rail">
            <div className="panel-card">
              <div className="panel-heading">
                <div>
                  <p className="eyebrow">
                    {userView === "beginner" ? "Queue health" : "Manual backlog"}
                  </p>
                  <h2>{userView === "beginner" ? "What needs a closer look" : "Review health"}</h2>
                </div>
                <span className="ghost-chip">{items.length} items</span>
              </div>

              <div className="summary-matrix review-summary-grid">
                <SummaryStat
                  label={userView === "beginner" ? "Strong clues" : "High confidence"}
                  value={highConfidenceCount}
                  tone="good"
                />
                <SummaryStat
                  label={userView === "beginner" ? "Needs checking" : "Medium confidence"}
                  value={mediumConfidenceCount}
                  tone="neutral"
                />
                <SummaryStat
                  label={userView === "beginner" ? "Very unclear" : "Low confidence"}
                  value={lowConfidenceCount}
                  tone="low"
                />
              </div>

              <div className="review-rail-note">
                <strong>
                  {userView === "beginner"
                    ? "Nothing moves from this screen."
                    : "Review is still a stop sign, not an action lane."}
                </strong>
                <p>
                  {userView === "beginner"
                    ? "Use this queue to see why a file stopped before you fix creators, save types, or head back to Organize."
                    : "Use this queue to inspect why a file stopped, then batch-fix creators or types before running another tidy pass."}
                </p>
              </div>
            </div>

            <div className="panel-card">
              <div className="panel-heading">
                <div>
                  <p className="eyebrow">
                    {userView === "beginner" ? "Main reasons" : "Reason groups"}
                  </p>
                  <h2>{userView === "beginner" ? "Why files stopped" : "Top reasons"}</h2>
                </div>
              </div>

              <div className="review-reason-list">
                {reasonRows.map(([reason, count]) => (
                  <div
                    key={reason}
                    className={`review-reason-row review-reason-row-${reviewReasonTone(
                      count,
                      maxReasonCount,
                    )}`}
                  >
                    <div className="review-reason-copy">
                      <strong>{humanize(reason)}</strong>
                      <span>
                        {userView === "beginner"
                          ? "These files need a manual check before SimSuite can move them."
                          : "Grouped by the main rule or validator stop reason."}
                      </span>
                    </div>
                    <span className="ghost-chip">{count}</span>
                  </div>
                ))}
              </div>

              <div className="review-rail-actions">
                <button
                  type="button"
                  className="secondary-action"
                  onClick={() => onNavigate("creatorAudit")}
                >
                  <Fingerprint size={14} strokeWidth={2} />
                  Creators
                </button>
                <button
                  type="button"
                  className="secondary-action"
                  onClick={() => onNavigate("categoryAudit")}
                >
                  <Shapes size={14} strokeWidth={2} />
                  Types
                </button>
                <button
                  type="button"
                  className="secondary-action"
                  onClick={() => onNavigate("organize")}
                >
                  <Workflow size={14} strokeWidth={2} />
                  {userView === "beginner" ? "Tidy Up" : "Organize"}
                </button>
              </div>
            </div>
          </div>

          <div className="panel-card queue-panel review-stage-panel workbench-panel">
            <div className="panel-heading">
              <div>
                <p className="eyebrow">
                  {userView === "beginner" ? "Files to check" : "Manual queue"}
                </p>
                <h2>{userView === "beginner" ? "Waiting for you" : "Items waiting"}</h2>
              </div>
              <span className="ghost-chip">{items.length} items</span>
            </div>

            <div className="workspace-toggles review-layout-toggles">
              {REVIEW_LAYOUT_PRESETS.map((preset) => (
                <button
                  key={preset.id}
                  type="button"
                  className={`workspace-toggle ${reviewLayoutPreset === preset.id ? "is-active" : ""}`}
                  onClick={() => applyReviewLayoutPreset(preset.id)}
                  title={preset.hint}
                >
                  {preset.label}
                </button>
              ))}
            </div>

            <div className="queue-list review-queue-list">
              {items.map((item, index) => (
                <m.button
                  key={item.id}
                  type="button"
                  className={`queue-row ${selectedId === item.id ? "is-selected" : ""}`}
                  onClick={() => setSelectedId(item.id)}
                  title={item.reason}
                  whileHover={rowHover}
                  whileTap={rowPress}
                  {...stagedListItem(index)}
                >
                  <div className="queue-main">
                    <strong>{item.filename}</strong>
                    <span>
                        {item.creator ?? unknownCreatorLabel(userView)} · {friendlyTypeLabel(item.kind)}
                        {userView === "power" && item.subtype
                          ? ` · ${item.subtype}`
                          : ""}
                    </span>
                  </div>
                  <div className="queue-meta">
                    <span className="warning-tag">{humanize(item.reason)}</span>
                    {userView !== "beginner" ? (
                      <span className="ghost-chip">{item.sourceLocation}</span>
                    ) : null}
                    <span
                      className={`confidence-badge ${confidenceTone(
                        item.confidence,
                      )}`}
                    >
                      {Math.round(item.confidence * 100)}%
                    </span>
                  </div>
                </m.button>
              ))}
            </div>
          </div>

          <ResizableDetailPanel
            ariaLabel="Review details"
            width={reviewDetailWidth}
            onWidthChange={setReviewDetailWidth}
            maxWidth={760}
          >
            {selected ? (
              <>
                <div className="detail-header">
                  <div>
                    <p className="eyebrow">
                      {userView === "beginner" ? "Selected file" : "Selected item"}
                    </p>
                    <h2>{selected.filename}</h2>
                  </div>
                  <span className={`confidence-badge ${confidenceTone(selected.confidence)}`}>
                    {Math.round(selected.confidence * 100)}%
                  </span>
                </div>

                <DockSectionStack
                  layoutId="reviewInspector"
                  sections={reviewInspectorSections}
                  intro="Reset this side panel"
                />
              </>
            ) : (
              <StatePanel
                eyebrow={userView === "beginner" ? "Selected file" : "Review"}
                title="Select an item"
                body={
                  userView === "beginner"
                    ? "Pick one file from the queue to see why SimSuite sent it to review and what safer place it expects."
                    : "Choose a queued file to inspect the stop reason, confidence, and suggested validated path."
                }
                icon={SearchX}
                meta={["Nothing moves from Review", "Use Creators or Types for batch fixes"]}
              />
            )}
          </ResizableDetailPanel>
        </div>
      ) : (
        <StatePanel
          eyebrow={reviewLabel(userView)}
          title={
            userView === "beginner"
              ? "Nothing needs your help right now"
              : "The queue is clear"
          }
          body={
            userView === "beginner"
              ? "That means the current scan did not find any files that need a manual decision."
              : "The current library pass does not have any unresolved files waiting for review."
          }
          icon={ShieldCheck}
          tone="good"
          actions={
            <button
              type="button"
              className="secondary-action"
              onClick={() => onNavigate("organize")}
            >
              <Workflow size={14} strokeWidth={2} />
              {userView === "beginner" ? "Open Tidy Up" : "Open Organize"}
            </button>
          }
          meta={["Scans stay read-only", "Queue updates after each scan"]}
        />
      )}
    </section>
  );
}

function SummaryStat({
  label,
  value,
  tone,
}: {
  label: string;
  value: number;
  tone: "good" | "neutral" | "low";
}) {
  return (
    <div className={`summary-stat summary-stat-${tone}`}>
      <span>{label}</span>
      <strong>{value.toLocaleString()}</strong>
    </div>
  );
}

function DetailRow({
  label,
  value,
}: {
  label: string;
  value: string;
}) {
  return (
    <div className="detail-row">
      <span>{label}</span>
      <strong>{value}</strong>
    </div>
  );
}

function confidenceTone(confidence: number) {
  if (confidence >= 0.85) {
    return "good";
  }
  if (confidence >= 0.6) {
    return "medium";
  }
  return "low";
}

function reviewReasonTone(count: number, maxCount: number) {
  if (count >= Math.max(2, maxCount)) {
    return "good";
  }
  if (count >= 2) {
    return "medium";
  }
  return "low";
}

function humanize(reason: string) {
  return reason.replace(/_/g, " ");
}
