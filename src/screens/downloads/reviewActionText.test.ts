import { describe, expect, it } from "vitest";
import type { ReviewPlanAction } from "../../lib/types";
import { reviewActionButtonLabel, reviewActionCardTitle } from "./reviewActionText";

function makeAction(overrides: Partial<ReviewPlanAction>): ReviewPlanAction {
  return {
    kind: "open_related_item",
    label: "Use McCmdCenter_AllModules_2026_1_1.zip",
    description: "Open the fuller local pack first so SimSuite can keep the suite together.",
    priority: 1,
    relatedItemId: null,
    relatedItemName: null,
    url: null,
    ...overrides,
  };
}

describe("reviewActionText", () => {
  it("uses the pack name as the action card title for related-item actions", () => {
    const action = makeAction({});

    expect(reviewActionCardTitle(action)).toBe("McCmdCenter_AllModules_2026_1_1.zip");
  });

  it("keeps the secondary action button short for related-item actions", () => {
    const action = makeAction({});

    expect(reviewActionButtonLabel(action, "standard", false)).toBe("Use this pack");
  });

  it("keeps the progress label clear while a related-item action is running", () => {
    const action = makeAction({});

    expect(reviewActionButtonLabel(action, "beginner", true)).toBe("Opening better pack...");
  });
});
