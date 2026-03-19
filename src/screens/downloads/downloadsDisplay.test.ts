import { describe, expect, it } from "vitest";
import {
  capRowBadges,
  fallbackDownloadsLane,
  pickInitialDownloadsLane,
  viewModeDownloadsFlags,
} from "./downloadsDisplay";

describe("pickInitialDownloadsLane", () => {
  it("prefers waiting_on_you over ready_now", () => {
    expect(
      pickInitialDownloadsLane({
        ready_now: 4,
        special_setup: 0,
        waiting_on_you: 1,
        blocked: 0,
        done: 0,
      }),
    ).toBe("waiting_on_you");
  });
});

describe("fallbackDownloadsLane", () => {
  it("moves to the next useful non-empty lane when the preferred lane empties", () => {
    expect(
      fallbackDownloadsLane("special_setup", {
        ready_now: 2,
        special_setup: 0,
        waiting_on_you: 0,
        blocked: 1,
        done: 0,
      }),
    ).toBe("ready_now");
  });
});

describe("capRowBadges", () => {
  it("never returns more than two visible badges", () => {
    expect(capRowBadges(["Ready", "Newer", "Supported"])).toEqual([
      "Ready",
      "Newer",
    ]);
  });
});

describe("viewModeDownloadsFlags", () => {
  it("keeps advanced filters hidden in casual mode", () => {
    expect(viewModeDownloadsFlags("beginner").showAdvancedFiltersByDefault).toBe(
      false,
    );
  });
});
