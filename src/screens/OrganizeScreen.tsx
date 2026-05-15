import { useMemo, useState } from "react";
import type { FormEvent } from "react";
import { m } from "motion/react";
import {
  AlertCircle,
  CheckCircle2,
  FileCheck2,
  Info,
  ListChecks,
  LoaderCircle,
  RefreshCw,
  Save,
  ShieldCheck,
  Workflow,
} from "lucide-react";
import { api } from "../lib/api";
import { hoverLift, stagedListItem, tapPress } from "../lib/motion";
import { PendingPlansPreview } from "./organize/PendingPlansPreview";
import { SavedPlansReview } from "./organize/SavedPlansReview";
import type {
  GenerateSortingPreviewPlanRequest,
  Screen,
  StagingPlan,
  StagingPlanActionKind,
  StagingPlanBucket,
  StagingPlanCurrentRoot,
  StagingPlanEvidenceLevel,
  StagingPlanItem,
  UserView,
} from "../lib/types";

interface OrganizeScreenProps {
  refreshVersion: number;
  onNavigate: (screen: Screen) => void;
  onDataChanged: () => void;
  userView: UserView;
}

type SourceLocation = "mods" | "tray";
type OrganizeTab = "create_plan" | "saved_plans" | "pending_batches";

const BUCKET_ORDER: StagingPlanBucket[] = [
  "script_mods",
  "cas",
  "build_buy",
  "gameplay",
  "presets_sliders",
  "overrides_defaults",
  "tray",
  "needs_review",
  "unknown_leave_in_place",
];

const BUCKET_LABELS: Record<StagingPlanBucket, string> = {
  script_mods: "Script Mods",
  cas: "CAS",
  build_buy: "Build/Buy",
  gameplay: "Gameplay",
  presets_sliders: "Presets & Sliders",
  overrides_defaults: "Overrides & Defaults",
  tray: "Tray",
  needs_review: "Needs Review",
  unknown_leave_in_place: "Unknown / Leave in place",
};

const ACTION_LABELS: Record<StagingPlanActionKind, string> = {
  suggest_move: "Suggested destination",
  suggest_group: "Suggested group",
  suggest_review: "Needs review",
  leave_in_place: "Leave in place",
  no_action: "No action suggested",
};

const EVIDENCE_LABELS: Record<StagingPlanEvidenceLevel, string> = {
  deterministic: "Deterministic",
  evidence_backed: "Evidence-backed",
  heuristic: "Heuristic",
  review_only: "Review-only",
};

const ROOT_LABELS: Record<StagingPlanCurrentRoot, string> = {
  mods: "Mods",
  tray: "Tray",
  downloads: "Downloads",
  inbox: "Inbox",
  unknown: "Unknown",
};

function statusLabel(plan: StagingPlan): string {
  switch (plan.status) {
    case "blocked":
      return "Blocked";
    case "ready_for_review":
      return "Ready for review";
    case "preview_only":
    default:
      return "Preview only";
  }
}

function groupItemsByBucket(items: StagingPlanItem[]) {
  const grouped = new Map<StagingPlanBucket, StagingPlanItem[]>();
  for (const bucket of BUCKET_ORDER) {
    grouped.set(bucket, []);
  }
  for (const item of items) {
    grouped.get(item.bucket)?.push(item);
  }
  return BUCKET_ORDER.map((bucket) => ({
    bucket,
    label: BUCKET_LABELS[bucket],
    items: grouped.get(bucket) ?? [],
  })).filter((group) => group.items.length > 0);
}

function SafetyBanner() {
  return (
    <m.section
      className="organize-plan-safety"
      initial={{ opacity: 0, y: -4 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.14 }}
      aria-label="Preview safety boundary"
    >
      <ShieldCheck size={18} />
      <div>
        <strong>No files changed</strong>
        <span>
          Organize now reviews preview plans only. Future file-changing work
          still needs user confirmation, backup and restore support, and a
          recoverable result log.
        </span>
      </div>
    </m.section>
  );
}

function PlanItemCard({ item, index }: { item: StagingPlanItem; index: number }) {
  const hasPathDetail = item.currentPath || item.suggestedDestinationPath;
  const hasSignals = item.sourceSignals.length > 0;
  const hasBlockedReasons = item.blockedReasons.length > 0;

  return (
    <m.article
      className="organize-plan-item"
      {...stagedListItem(index)}
      whileHover={hoverLift}
      whileTap={tapPress}
    >
      <div className="organize-plan-item-header">
        <div className="organize-plan-item-title">
          <span>{item.fileName}</span>
          <small>{ROOT_LABELS[item.currentRoot]} source</small>
        </div>
        <div className="organize-plan-status-row" aria-label="Plan item labels">
          <span className="organize-plan-status-chip">
            {ACTION_LABELS[item.actionKind]}
          </span>
          <span className="organize-plan-status-chip">
            {EVIDENCE_LABELS[item.evidenceLevel]}
          </span>
          <span className="organize-plan-status-chip">
            {item.confidenceLabel}
          </span>
        </div>
      </div>

      <div className="organize-plan-detail-block">
        <h4>Why SimSuite suggested this</h4>
        <p>{item.reason}</p>
      </div>

      {hasPathDetail && (
        <div className="organize-plan-path-grid">
          {item.currentPath && (
            <div className="organize-plan-path-card">
              <span>Current path</span>
              <code>{item.currentPath}</code>
            </div>
          )}
          {item.suggestedDestinationPath && (
            <div className="organize-plan-path-card">
              <span>Suggested destination</span>
              <code>{item.suggestedDestinationPath}</code>
            </div>
          )}
        </div>
      )}

      {item.caveats.length > 0 && (
        <div className="organize-plan-detail-block">
          <h4>Caveats</h4>
          <ul className="organize-plan-caveats">
            {item.caveats.map((caveat) => (
              <li key={caveat}>{caveat}</li>
            ))}
          </ul>
        </div>
      )}

      {hasSignals && (
        <div className="organize-plan-detail-block">
          <h4>Source signals</h4>
          <div className="organize-plan-tags">
            {item.sourceSignals.map((signal) => (
              <span key={signal} className="organize-plan-tag">
                {signal}
              </span>
            ))}
          </div>
        </div>
      )}

      {hasBlockedReasons && (
        <div className="organize-plan-detail-block">
          <h4>Blocked reasons</h4>
          <div className="organize-plan-tags">
            {item.blockedReasons.map((reason) => (
              <span
                key={reason}
                className="organize-plan-tag organize-plan-tag--blocked"
              >
                {reason}
              </span>
            ))}
          </div>
        </div>
      )}
    </m.article>
  );
}

interface PlanResultProps {
  plan: StagingPlan | null;
  isGenerating: boolean;
  errorMessage: string | null;
  canSavePlan: boolean;
  isSavingPlan: boolean;
  saveErrorMessage: string | null;
  onSavePreviewPlan: () => void;
}

function PlanResult({
  plan,
  isGenerating,
  errorMessage,
  canSavePlan,
  isSavingPlan,
  saveErrorMessage,
  onSavePreviewPlan,
}: PlanResultProps) {
  const groupedItems = useMemo(
    () => groupItemsByBucket(plan?.items ?? []),
    [plan],
  );

  if (isGenerating) {
    return (
      <section className="organize-plan-empty" aria-live="polite">
        <LoaderCircle size={24} className="spin" />
        <h3>Generating preview</h3>
        <p>SimSuite is building a bounded review plan. No files changed.</p>
      </section>
    );
  }

  if (errorMessage) {
    return (
      <section className="organize-plan-empty organize-plan-empty--error">
        <AlertCircle size={24} />
        <h3>Could not generate preview plan</h3>
        <p>{errorMessage}</p>
      </section>
    );
  }

  if (!plan) {
    return (
      <section className="organize-plan-empty">
        <Workflow size={24} />
        <h3>No generated plan yet</h3>
        <p>
          Choose a bounded Library scope, then generate a preview. SimSuite will
          show review items, reasons, and caveats before any file-changing
          workflow exists.
        </p>
      </section>
    );
  }

  return (
    <section className="organize-plan-results" aria-label="Generated plan">
      <div className="organize-plan-summary">
        <div>
          <div className="eyebrow">Suggested plan</div>
          <h3>{plan.title}</h3>
          <p>{plan.summary}</p>
        </div>
        <div className="organize-plan-summary-grid">
          <span>
            <strong>{statusLabel(plan)}</strong>
            <small>Status</small>
          </span>
          <span>
            <strong>{plan.itemCount}</strong>
            <small>Items</small>
          </span>
          <span>
            <strong>No</strong>
            <small>Files changed</small>
          </span>
        </div>
      </div>

      <div className="pending-plans-safety-strip" aria-label="Save preview plan safety">
        <ShieldCheck size={16} />
        <strong>No files changed</strong>
        <span>
          Save this as a draft preview record for later review. Saving does not
          move, delete, replace, or change files.
        </span>
      </div>

      <div className="organize-next-step-actions">
        <button
          type="button"
          className="primary-action"
          disabled={!canSavePlan || isSavingPlan}
          onClick={onSavePreviewPlan}
        >
          {isSavingPlan ? (
            <>
              <LoaderCircle size={16} className="spin" />
              Saving preview
            </>
          ) : (
            <>
              <Save size={16} />
              Save preview plan
            </>
          )}
        </button>
      </div>

      {saveErrorMessage ? (
        <div className="staging-result staging-result--warn" role="status">
          <AlertCircle size={16} />
          <span>{saveErrorMessage} No files changed.</span>
        </div>
      ) : null}

      {plan.caveats.length > 0 && (
        <div className="organize-plan-detail-block">
          <h4>Plan caveats</h4>
          <ul className="organize-plan-caveats">
            {plan.caveats.map((caveat) => (
              <li key={caveat}>{caveat}</li>
            ))}
          </ul>
        </div>
      )}

      {plan.items.length === 0 ? (
        <section className="organize-plan-empty">
          <Info size={24} />
          <h3>{plan.status === "blocked" ? "Blocked" : "No preview items"}</h3>
          <p>
            SimSuite did not find enough bounded file data for a review plan.
            No files changed.
          </p>
        </section>
      ) : (
        <div className="organize-plan-bucket-list">
          {groupedItems.map((group) => (
            <section
              key={group.bucket}
              className="organize-plan-bucket"
              aria-label={`${group.label} suggestions`}
            >
              <div className="organize-plan-bucket-heading">
                <h3>{group.label}</h3>
                <span>
                  {group.items.length} item{group.items.length !== 1 ? "s" : ""}
                </span>
              </div>
              <m.div
                className="organize-plan-item-list"
                initial="hidden"
                animate="show"
                variants={{
                  hidden: { opacity: 1 },
                  show: {
                    opacity: 1,
                    transition: { staggerChildren: 0.04 },
                  },
                }}
              >
                {group.items.map((item, index) => (
                  <PlanItemCard key={item.id} item={item} index={index} />
                ))}
              </m.div>
            </section>
          ))}
        </div>
      )}
    </section>
  );
}

export function OrganizeScreen({
  refreshVersion: _refreshVersion,
  onNavigate,
  onDataChanged: _onDataChanged,
  userView,
}: OrganizeScreenProps) {
  const [activeTab, setActiveTab] = useState<OrganizeTab>("create_plan");
  const [sourceLocation, setSourceLocation] = useState<SourceLocation>("mods");
  const [folderPath, setFolderPath] = useState("");
  const [recursive, setRecursive] = useState(true);
  const [limit, setLimit] = useState(60);
  const [plan, setPlan] = useState<StagingPlan | null>(null);
  const [lastPreviewRequest, setLastPreviewRequest] =
    useState<GenerateSortingPreviewPlanRequest | null>(null);
  const [isGenerating, setIsGenerating] = useState(false);
  const [isSavingPlan, setIsSavingPlan] = useState(false);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [saveErrorMessage, setSaveErrorMessage] = useState<string | null>(null);
  const [saveStatusMessage, setSaveStatusMessage] = useState<string | null>(null);
  const [savedPlansRefreshVersion, setSavedPlansRefreshVersion] = useState(0);
  const [selectedSavedPlanId, setSelectedSavedPlanId] = useState<number | null>(null);

  const handleGeneratePreview = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    setIsGenerating(true);
    setErrorMessage(null);
    setSaveErrorMessage(null);
    setSaveStatusMessage(null);

    const request: GenerateSortingPreviewPlanRequest = {
      scope: {
        kind: "library_folder",
        sourceLocation,
        folderPath: folderPath.trim(),
        recursive,
        limit,
      },
    };

    try {
      const nextPlan = await api.generateSortingPreviewPlan(request);
      setPlan(nextPlan);
      setLastPreviewRequest(request);
    } catch (error) {
      setPlan(null);
      setLastPreviewRequest(null);
      setErrorMessage(error instanceof Error ? error.message : String(error));
    } finally {
      setIsGenerating(false);
    }
  };

  const handleSavePreviewPlan = async () => {
    if (!plan || !lastPreviewRequest) {
      setSaveErrorMessage("Generate a preview plan before saving a draft.");
      return;
    }

    setIsSavingPlan(true);
    setSaveErrorMessage(null);
    setSaveStatusMessage(null);

    try {
      const saved = await api.buildApplyPlanFromStagingPlan({
        previewRequest: lastPreviewRequest,
        sourcePlanKind: "sorting_preview",
      });
      setSelectedSavedPlanId(saved.planId);
      setSavedPlansRefreshVersion((value) => value + 1);
      setSaveStatusMessage("Saved as draft preview plan. No files changed.");
      setActiveTab("saved_plans");
    } catch (error) {
      setSaveErrorMessage(error instanceof Error ? error.message : String(error));
    } finally {
      setIsSavingPlan(false);
    }
  };

  return (
    <div className="screen-shell workbench workbench-screen organize-screen">
      <section className="screen-hero workbench-hero">
        <div className="screen-hero-copy">
          <span className="eyebrow">Organize</span>
          <h1>Review suggested organization plans.</h1>
          <p>
            Generate preview-only Library plans, inspect the reasons and
            caveats, then decide what needs manual review. SimSuite does not
            change files from this page.
          </p>
        </div>
      </section>

      <SafetyBanner />

      <section className="panel-card organize-workspace-tabs">
        <div className="organize-workspace-tabs-copy">
          <span className="eyebrow">Planning workspace</span>
          <p>
            Create a new preview suggestion from your Library, review saved
            draft plans, or check pending batch handoff notes without leaving
            Organize.
          </p>
        </div>
        <div
          className="segmented-control organize-tablist"
          role="tablist"
          aria-label="Organize planning sections"
        >
          <button
            id="organize-tab-create-plan"
            type="button"
            role="tab"
            aria-selected={activeTab === "create_plan"}
            aria-controls="organize-panel-create-plan"
            className={`segment-button ${
              activeTab === "create_plan" ? "is-active" : ""
            }`}
            onClick={() => setActiveTab("create_plan")}
          >
            Create plan
          </button>
          <button
            id="organize-tab-saved-plans"
            type="button"
            role="tab"
            aria-selected={activeTab === "saved_plans"}
            aria-controls="organize-panel-saved-plans"
            className={`segment-button ${
              activeTab === "saved_plans" ? "is-active" : ""
            }`}
            onClick={() => setActiveTab("saved_plans")}
          >
            Saved plans
          </button>
          <button
            id="organize-tab-pending-plans"
            type="button"
            role="tab"
            aria-selected={activeTab === "pending_batches"}
            aria-controls="organize-panel-pending-batches"
            className={`segment-button ${
              activeTab === "pending_batches" ? "is-active" : ""
            }`}
            onClick={() => setActiveTab("pending_batches")}
          >
            Pending batches
          </button>
        </div>
      </section>

      {activeTab === "create_plan" ? (
        <section
          id="organize-panel-create-plan"
          role="tabpanel"
          aria-labelledby="organize-tab-create-plan"
          className="organize-plan-layout"
          aria-label="Create preview plan"
        >
          <form
            className="panel-card organize-plan-controls"
            onSubmit={handleGeneratePreview}
          >
            <div className="panel-card-heading">
              <div>
                <span className="eyebrow">Create preview plan</span>
                <h2>Bounded Library folder scope</h2>
              </div>
              <span className="organize-plan-status-chip">Preview only</span>
            </div>

            <p className="organize-muted">
              Use a bounded Mods or Tray folder scope for the first visible
              planning workflow. Leave the path blank to preview the selected root
              within the item limit.
            </p>

            <div className="organize-plan-control-grid">
              <label className="organize-plan-field">
                <span>Source root</span>
                <select
                  aria-label="Source root"
                  value={sourceLocation}
                  onChange={(event) =>
                    setSourceLocation(event.target.value as SourceLocation)
                  }
                >
                  <option value="mods">Mods</option>
                  <option value="tray">Tray</option>
                </select>
              </label>

              <label className="organize-plan-field">
                <span>Folder path</span>
                <input
                  aria-label="Folder path"
                  value={folderPath}
                  onChange={(event) => setFolderPath(event.target.value)}
                  placeholder="Example: CAS/Hair"
                />
              </label>

              <label className="organize-plan-field">
                <span>Preview limit</span>
                <select
                  aria-label="Preview limit"
                  value={limit}
                  onChange={(event) => setLimit(Number(event.target.value))}
                >
                  <option value={25}>25 items</option>
                  <option value={60}>60 items</option>
                  <option value={100}>100 items</option>
                  <option value={150}>150 items</option>
                  <option value={250}>250 items</option>
                </select>
              </label>
            </div>

            <label className="organize-check-row">
              <input
                type="checkbox"
                checked={recursive}
                onChange={(event) => setRecursive(event.target.checked)}
              />
              <span>Include nested folders</span>
            </label>

            <div className="organize-next-step-actions">
              <button
                type="submit"
                className="primary-action"
                disabled={isGenerating}
              >
                {isGenerating ? (
                  <>
                    <LoaderCircle size={16} className="spin" />
                    Generating preview
                  </>
                ) : (
                  <>
                    <RefreshCw size={16} />
                    Generate preview
                  </>
                )}
              </button>
              <button
                type="button"
                className="secondary-action"
                onClick={() => setActiveTab("saved_plans")}
              >
                <FileCheck2 size={16} />
                Saved plans
              </button>
              <button
                type="button"
                className="secondary-action"
                onClick={() => onNavigate("library")}
              >
                <ListChecks size={16} />
                Open Library
              </button>
            </div>

            <div className="organize-plan-scope-note">
              <CheckCircle2 size={16} />
              <span>
                Current mode: {userView}. Plan items are suggestions only and
                always return wouldTouchFiles=false.
              </span>
            </div>
          </form>

          <div className="panel-card organize-plan-panel">
            <PlanResult
              plan={plan}
              isGenerating={isGenerating}
              errorMessage={errorMessage}
              canSavePlan={Boolean(plan && lastPreviewRequest)}
              isSavingPlan={isSavingPlan}
              saveErrorMessage={saveErrorMessage}
              onSavePreviewPlan={handleSavePreviewPlan}
            />
          </div>
        </section>
      ) : activeTab === "saved_plans" ? (
        <section
          id="organize-panel-saved-plans"
          role="tabpanel"
          aria-labelledby="organize-tab-saved-plans"
          className="organize-saved-plans-panel"
        >
          <SavedPlansReview
            refreshVersion={savedPlansRefreshVersion}
            selectedPlanId={selectedSavedPlanId}
            statusMessage={saveStatusMessage}
            onCreatePlan={() => setActiveTab("create_plan")}
          />
        </section>
      ) : (
        <section
          id="organize-panel-pending-batches"
          role="tabpanel"
          aria-labelledby="organize-tab-pending-plans"
          className="panel-card organize-pending-plan-panel"
        >
          <PendingPlansPreview
            onNavigate={onNavigate}
            onCreatePlan={() => setActiveTab("create_plan")}
          />
        </section>
      )}
    </div>
  );
}
