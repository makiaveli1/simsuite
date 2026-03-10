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
import { LayoutPresetBar } from "../components/LayoutPresetBar";
import { ResizableEdgeHandle } from "../components/ResizableEdgeHandle";
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
    reviewQueueHeight,
    setReviewDetailWidth,
    setReviewQueueHeight,
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
    <section className="screen-shell">
      <div className="screen-header-row">
        <div className="screen-heading">
          <p className="eyebrow">{userView === "beginner" ? "Check these" : "Queue"}</p>
          <div className="screen-title-row">
            <ShieldAlert size={18} strokeWidth={2} />
            <h1>{reviewLabel(userView)}</h1>
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

      <LayoutPresetBar
        title={userView === "beginner" ? "Quick view" : "Review layout"}
        summary={
          userView === "beginner"
            ? "Keep the queue and the selected file easy to read while you work through the hold list."
            : userView === "power"
              ? "Saved queue and inspector presets for denser review passes."
              : "Saved queue and detail presets for triage and deeper file review."
        }
        presets={userView === "beginner" ? [] : REVIEW_LAYOUT_PRESETS}
        activePreset={reviewLayoutPreset}
        onApplyPreset={(preset) =>
          applyReviewLayoutPreset(preset as ReviewLayoutPreset)
        }
      />

      {items.length ? (
        <div className="review-layout review-layout-screen">
          <div className="panel-card queue-panel review-queue-panel">
            <div className="panel-heading">
              <div>
                <p className="eyebrow">
                  {userView === "beginner" ? "Files to check" : "Manual queue"}
                </p>
                <h2>{userView === "beginner" ? "Waiting for you" : "Items waiting"}</h2>
              </div>
              <span className="ghost-chip">{items.length} items</span>
            </div>

            <div className="vertical-dock queue-dock">
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
              <ResizableEdgeHandle
                label="Resize review queue height"
                value={reviewQueueHeight}
                min={260}
                max={860}
                onChange={setReviewQueueHeight}
                side="bottom"
                className="dock-resize-handle review-queue-height-handle"
              />
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

function humanize(reason: string) {
  return reason.replace(/_/g, " ");
}
