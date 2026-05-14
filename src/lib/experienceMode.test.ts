import { describe, expect, it } from "vitest";
import { EXPERIENCE_MODE_PROFILES } from "./experienceMode";

describe("experience mode navigation", () => {
  it("keeps Plan Preview out of the normal sidebar modes", () => {
    for (const profile of Object.values(EXPERIENCE_MODE_PROFILES)) {
      expect(profile.primaryScreens).not.toContain("staging");
      expect(profile.toolScreens).not.toContain("staging");
    }
  });
});
