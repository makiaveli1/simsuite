import { describe, expect, it } from "vitest";
import { api } from "./api";

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
