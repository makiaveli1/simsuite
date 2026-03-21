import { describe, expect, it } from "vitest";
import {
  createUpdatesNavigationParams,
  nextUpdatesParamsForPlainNavigation,
  resolveUpdatesNavigationParams,
} from "./updatesNavigation";

describe("updatesNavigation", () => {
  it("keeps only valid update focus params from the URL", () => {
    expect(
      resolveUpdatesNavigationParams("?mode=setup&filter=all&fileId=42"),
    ).toEqual({
      mode: "setup",
      filter: "all",
      fileId: 42,
    });

    expect(
      resolveUpdatesNavigationParams("?mode=nope&filter=bad&fileId=wat"),
    ).toEqual({});
  });

  it("clears old update focus when the user opens Updates normally", () => {
    expect(
      nextUpdatesParamsForPlainNavigation(
        "updates",
        createUpdatesNavigationParams("review", "unclear", 99),
      ),
    ).toEqual({});
  });

  it("keeps the current focus when the user is going somewhere else", () => {
    const current = createUpdatesNavigationParams("tracked", "exact_updates", 7);

    expect(nextUpdatesParamsForPlainNavigation("library", current)).toBe(current);
  });
});
