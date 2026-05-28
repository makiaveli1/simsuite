import { readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";

const userFacingClaimSurfaces = [
  "src/components/FieldGuide.tsx",
  "src/screens/HomeScreen.tsx",
  "src/screens/DownloadsScreen.tsx",
  "src/screens/downloads/ConflictEvidenceDisplay.tsx",
  "src/screens/LibraryScreen.tsx",
  "src/screens/DuplicatesScreen.tsx",
  "src/screens/UpdatesScreen.tsx",
  "src/screens/OrganizeScreen.tsx",
  "src/screens/organize/SavedPlansReview.tsx",
  "src/screens/StagingScreen.tsx",
  "src/screens/library/actionPreflight.tsx",
  "src/screens/library/LibraryCollectionTable.tsx",
  "src/screens/library/LibraryDetailSheet.tsx",
  "src/screens/library/LibraryDetailsPanel.tsx",
];

const forbiddenTrustClaims =
  /auto[-\s]?fix|auto[-\s]?quarantine|quarantine|safe to delete|safe to replace|confirmed safe|AI verified|missing mesh|missing dependency|confirmed duplicate|duplicate candidate|possible duplicate|definitely outdated|definitely latest|official source found|remove this one|delete duplicate/i;

function readRepoFile(relativePath: string) {
  return readFileSync(join(process.cwd(), relativePath), "utf8");
}

describe("trust-boundary user-facing copy", () => {
  it("does not expose forbidden automation or proof claims in current trust-sensitive surfaces", () => {
    const offenders = userFacingClaimSurfaces.flatMap((relativePath) => {
      const source = readRepoFile(relativePath);
      return source
        .split(/\r?\n/)
        .map((line, index) => ({ line, lineNumber: index + 1, relativePath }))
        .filter(({ line }) => forbiddenTrustClaims.test(line))
        .map(({ relativePath: path, lineNumber, line }) => `${path}:${lineNumber}: ${line.trim()}`);
    });

    expect(offenders).toEqual([]);
  });

  it("keeps the current Plan Preview screen preview-only", () => {
    const source = readRepoFile("src/screens/StagingScreen.tsx");

    expect(source).toMatch(/preview-only/i);
    expect(source).toMatch(/No files\s+will be changed/i);
    expect(source).toMatch(/User confirmation required/i);
    expect(source).toMatch(/Plan Preview/);
    expect(source).not.toMatch(/Staging is preview-only|No staged content|Staging will show|Open Staging/i);
    expect(source).not.toMatch(/commitStagingArea|commitAllStagingAreas|cleanupStagingAreas/);
    expect(source).not.toMatch(/Commit to Library|Commit all|Reject all|Reject staged files|Remove staged files/i);
  });

  it("keeps the current Organize screen preview-only", () => {
    const source = readRepoFile("src/screens/OrganizeScreen.tsx");

    expect(source).toMatch(/generateSortingPreviewPlan/);
    expect(source).toMatch(/saveApplyPlanFromPreviewSnapshot/);
    expect(source).toMatch(/No files changed/i);
    expect(source).toMatch(/Preview only/i);
    expect(source).not.toMatch(/saveApplyPlanPreview/);
    expect(source).not.toMatch(/previewOrganization|applyPreviewOrganization|listSnapshots|restoreSnapshot|listRulePresets/);
    expect(source).not.toMatch(/Safe to move|Safe move|Ready to move|Ready to apply|Auto-sort now|Sort automatically/i);
  });

  it("keeps the current Inbox screen intake/review-only", () => {
    const source = readRepoFile("src/screens/DownloadsScreen.tsx");

    expect(source).toMatch(/INBOX_FILE_ACTIONS_BLOCKED = true/);
    expect(source).toMatch(/Inbox is review-only/i);
    expect(source).toMatch(/No files changed/i);
    expect(source).not.toMatch(/<button[^>]*>\s*Apply|<button[^>]*>\s*Reject/i);
  });

  it("documents Apply as blocked until the safety contract exists", () => {
    const source = readRepoFile("docs/planning/APPLY_SAFETY_CONTRACT_V1.md");

    expect(source).toMatch(/Apply is not implemented/i);
    expect(source).toMatch(/explicitly confirmed/i);
    expect(source).toMatch(/backup or restore/i);
    expect(source).toMatch(/per-file result log/i);
    expect(source).toMatch(/Future Apply must not:[\s\S]*delete files/i);
    expect(source).toMatch(/Future Apply must not:[\s\S]*quarantine files/i);
    expect(source).toMatch(/Future Apply must not:[\s\S]*AI-only suggestions/i);
  });

  it("documents the existing systems integration contract", () => {
    const source = readRepoFile("docs/planning/EXISTING_SYSTEMS_INTEGRATION_CONTRACT_V1.md");

    expect(source).toMatch(/Existing systems reused/);
    expect(source).toMatch(/New data or logic added/);
    expect(source).toMatch(/Do not reimplement duplicate truth/);
    expect(source).toMatch(/Do not parse `?\.package`? files in UI render paths/);
    expect(source).toMatch(/StagingPlan/);
    expect(source).toMatch(/ApplyPlan/);
  });

  it("documents the ApplyPlan persistence audit without implementing real Apply", () => {
    const source = readRepoFile("docs/planning/APPLYPLAN_PERSISTENCE_AUDIT_V1.md");

    expect(source).toMatch(/Real Apply is not implemented/i);
    expect(source).toMatch(/Existing Systems Integration Contract/);
    expect(source).toMatch(/Apply Safety Contract/);
    expect(source).toMatch(/Existing systems reused/);
    expect(source).toMatch(/No files changed/);
  });

  it("documents validation conflict preview as no-file-change design work", () => {
    const source = readRepoFile("docs/planning/VALIDATION_CONFLICT_PREVIEW_V1.md");

    expect(source).toMatch(/No files changed/);
    expect(source).toMatch(/does not implement Apply/i);
    expect(source).toMatch(/Validation preview/);
    expect(source).toMatch(/Conflict preview/);
    expect(source).toMatch(/preview_apply_plan_validation/);
    expect(source).toMatch(/canProceedToConfirmation:\s*false/);
    expect(source).toMatch(/must not:[\s\S]*move files/i);
    expect(source).toMatch(/must not:[\s\S]*create folders/i);
  });

  it("documents the backup restore result-log design without making Apply ready", () => {
    const source = readRepoFile("docs/planning/BACKUP_RESTORE_RESULT_LOG_DESIGN_V1.md");

    expect(source).toMatch(/No files changed/);
    expect(source).toMatch(/Apply is not ready/);
    expect(source).toMatch(/result log/i);
    expect(source).toMatch(/restore map/i);
    expect(source).toMatch(/backup required/i);
    expect(source).toMatch(/Existing systems reused/);
  });

  it("documents dry-run Apply design as read-only rehearsal work", () => {
    const source = readRepoFile("docs/planning/DRY_RUN_APPLY_DESIGN_V1.md");

    expect(source).toMatch(/Dry-run Apply is a read-only rehearsal/);
    expect(source).toMatch(/No files changed/);
    expect(source).toMatch(/Apply is not ready yet/i);
    expect(source).toMatch(/canProceedToApply/);
    expect(source).toMatch(/canProceedToConfirmation/);
    expect(source).toMatch(/Existing systems reused/);
  });

  it("documents Confirmation Design V1 as read-only custom-folder and cross-system contract", () => {
    const source = readRepoFile("docs/planning/CONFIRMATION_DESIGN_V1.md");

    expect(source).toMatch(/Confirmation Design V1/);
    expect(source).toMatch(/No files changed/);
    expect(source).toMatch(/confirmation token is not issued/i);
    expect(source).toMatch(/custom folder configurations/i);
    expect(source).toMatch(/folder configuration snapshot/i);
    expect(source).toMatch(/cross-system context/i);
    expect(source).toMatch(/Library/);
    expect(source).toMatch(/Inbox/);
    expect(source).toMatch(/Updates/);
    expect(source).toMatch(/Duplicates/);
    expect(source).toMatch(/Creator and Category Audit/);
    expect(source).toMatch(/must not:[\s\S]*move files/i);
    expect(source).toMatch(/must not:[\s\S]*create folders/i);
  });

  it("keeps the consolidated command-surface audit explicit about blocked execution", () => {
    const source = readRepoFile("docs/COMMAND_SURFACE_APPLY_SAFETY_AUDIT_V1_REPORT.md");

    expect(source).toMatch(/UI gating is not a backend safety boundary/);
    expect(source).toMatch(/backend-issued confirmation token/);
    expect(source).toMatch(/canonical root checks/);
    expect(source).toMatch(/does not enable Apply, Restore, backup execution/i);
    expect(source).toMatch(/Backend Command Gating V1/);
    expect(source).toMatch(/fail closed/i);
  });
});
