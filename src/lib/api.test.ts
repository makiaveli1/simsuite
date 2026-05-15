import { describe, expect, it } from "vitest";
import { api } from "./api";

const DEFAULT_MOCK_PATH_MARKER = "C:\\Users\\Player";

describe("sorting preview plan API", () => {
  it("returns a preview-only mock plan with rule-audit fields", async () => {
    const plan = await api.generateSortingPreviewPlan({
      scope: {
        kind: "selected_files",
        fileIds: [101, 102],
      },
    });

    expect(plan.source).toBe("organize");
    expect(plan.status).toBe("preview_only");
    expect(plan.wouldTouchFiles).toBe(false);
    expect(plan.caveats.join(" ")).toMatch(/No files changed/i);
    expect(plan.items.length).toBeGreaterThan(0);

    for (const item of plan.items) {
      expect(item.wouldTouchFiles).toBe(false);
      expect(item.sourceSignals.length).toBeGreaterThan(0);
      expect(item.bucket).toBeTruthy();
      expect(item.confidenceLabel).toBeTruthy();
      expect(item.currentRoot).toBeTruthy();
    }

    expect(plan.items.some((item) => item.actionKind === "leave_in_place")).toBe(true);
  });
});

describe("ApplyPlan preview persistence API", () => {
  it("builds a saved draft ApplyPlan from a backend-owned preview request in the mock API", async () => {
    const saved = await api.buildApplyPlanFromStagingPlan({
      previewRequest: {
        scope: {
          kind: "selected_files",
          fileIds: [101, 102],
        },
      },
    });

    expect(saved.planId).toBeGreaterThan(0);
    expect(saved.plan.sourcePlanKind).toBe("sorting_preview");
    expect(saved.plan.wouldTouchFiles).toBe(false);
    expect(saved.plan.applyableItems).toBe(0);

    const plan = await api.getApplyPlan(saved.planId);
    expect(plan).not.toBeNull();
    expect(plan?.sourceScope?.kind).toBe("selected_files");
    expect(plan?.caveats.join(" ")).toMatch(/No files changed/i);
    expect(plan?.items.length).toBeGreaterThan(0);
    expect(plan?.items.every((item) => item.signals.length > 0)).toBe(true);
    expect(plan?.items.some((item) => item.blockers.length > 0)).toBe(true);
  });

  it("returns a read-only validation preview for saved draft ApplyPlans in the mock API", async () => {
    const saved = await api.buildApplyPlanFromStagingPlan({
      previewRequest: {
        scope: {
          kind: "selected_files",
          fileIds: [101, 102],
        },
      },
    });

    const preview = await api.previewApplyPlanValidation({
      planId: saved.planId,
    });

    expect(preview.planId).toBe(saved.planId);
    expect(preview.canProceedToConfirmation).toBe(false);
    expect(preview.caveats.join(" ")).toMatch(/No files changed/i);
    expect(preview.caveats.join(" ")).toMatch(/Backup\/restore/i);
    expect(preview.summary.totalItems).toBeGreaterThan(0);
    expect(preview.summary.backupBlockedItems).toBe(preview.summary.totalItems);
    expect(preview.items.every((item) => item.canApplyLater === false)).toBe(true);
    expect(
      preview.items.some(
        (item) =>
          item.validationStatus === "blocked" ||
          item.validationStatus === "review_only_blocked",
      ),
    ).toBe(true);
  });

  it("saves, lists, reads, and soft-cancels preview-only plans in the mock API", async () => {
    const sourcePlan = await api.generateSortingPreviewPlan({
      scope: {
        kind: "selected_files",
        fileIds: [101, 102],
      },
    });

    const saved = await api.saveApplyPlanPreview({
      sourcePlan,
      sourcePlanKind: "sorting_preview",
      sourceScope: {
        kind: "selected_files",
        fileIds: [101, 102],
      },
    });

    expect(saved.planId).toBeGreaterThan(0);
    expect(saved.plan.wouldTouchFiles).toBe(false);
    expect(saved.plan.confirmationRequired).toBe(true);
    expect(saved.plan.backupRequired).toBe(true);
    expect(saved.plan.applyableItems).toBe(0);

    const summaries = await api.listSavedApplyPlans();
    const summary = summaries.find((plan) => plan.id === saved.planId);
    expect(summary).toBeTruthy();
    expect(JSON.stringify(summary)).not.toContain(DEFAULT_MOCK_PATH_MARKER);

    const plan = await api.getApplyPlan(saved.planId);
    expect(plan).not.toBeNull();
    expect(plan?.wouldTouchFiles).toBe(false);
    expect(plan?.items.length).toBe(sourcePlan.items.length);
    expect(plan?.items.some((item) => item.signals.length > 0)).toBe(true);
    expect(plan?.items.some((item) => item.blockers.length > 0)).toBe(true);

    const cancelled = await api.deleteDraftApplyPlan(saved.planId);
    expect(cancelled.status).toBe("cancelled");

    const visibleAfterCancel = await api.listSavedApplyPlans();
    expect(visibleAfterCancel.some((plan) => plan.id === saved.planId)).toBe(false);

    const allPlans = await api.listSavedApplyPlans({ includeCancelled: true });
    expect(allPlans.find((plan) => plan.id === saved.planId)?.status).toBe(
      "cancelled",
    );
  });
});
