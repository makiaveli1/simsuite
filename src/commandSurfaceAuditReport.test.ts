import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

const reportPath = "docs/COMMAND_SURFACE_APPLY_SAFETY_AUDIT_V1_REPORT.md";

describe("command surface safety audit report", () => {
  it("records critical legacy file-changing commands as public backend risks", () => {
    const report = readFileSync(reportPath, "utf8");

    for (const command of [
      "apply_preview_organization",
      "commit_staging_area",
      "commit_all_staging_areas",
      "apply_download_item",
      "apply_download_items",
      "apply_guided_download_item",
      "apply_special_review_fix",
      "apply_review_plan_action",
      "restore_snapshot",
      "undo_applied_item",
      "reject_download_item",
    ]) {
      expect(report).toContain(command);
    }

    expect(report).toContain("UI gating is not a backend safety boundary");
    expect(report).toContain("backend-issued confirmation token");
    expect(report).toContain("canonical root checks");
  });
});
