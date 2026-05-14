import { describe, expect, it } from "vitest";
import { screenLabel, screenHelperLine } from "./uiLanguage";

describe("user-facing screen language", () => {
  it("labels the internal staging route as Plan Preview for users", () => {
    expect(screenLabel("staging", "standard")).toBe("Plan Preview");
    expect(screenHelperLine("staging", "standard")).toMatch(/Plan Preview/);
    expect(screenHelperLine("staging", "standard")).toMatch(/without changing files/i);
  });
});
