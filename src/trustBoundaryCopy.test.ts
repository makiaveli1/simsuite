import { readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";

const userFacingClaimSurfaces = [
  "src/components/FieldGuide.tsx",
  "src/screens/DownloadsScreen.tsx",
  "src/screens/downloads/ConflictEvidenceDisplay.tsx",
  "src/screens/LibraryScreen.tsx",
  "src/screens/DuplicatesScreen.tsx",
  "src/screens/UpdatesScreen.tsx",
  "src/screens/OrganizeScreen.tsx",
  "src/screens/StagingScreen.tsx",
  "src/screens/library/actionPreflight.tsx",
  "src/screens/library/LibraryCollectionTable.tsx",
  "src/screens/library/LibraryDetailSheet.tsx",
  "src/screens/library/LibraryDetailsPanel.tsx",
];

const forbiddenTrustClaims =
  /auto[-\s]?fix|auto[-\s]?quarantine|quarantine|safe to delete|safe to replace|confirmed safe|AI verified|missing mesh|missing dependency|confirmed duplicate|duplicate candidate|possible duplicate|definitely outdated|definitely latest|official source found|remove this one|delete duplicate/i;

describe("trust-boundary user-facing copy", () => {
  it("does not expose forbidden automation or proof claims in current trust-sensitive surfaces", () => {
    const offenders = userFacingClaimSurfaces.flatMap((relativePath) => {
      const source = readFileSync(join(process.cwd(), relativePath), "utf8");
      return source
        .split(/\r?\n/)
        .map((line, index) => ({ line, lineNumber: index + 1, relativePath }))
        .filter(({ line }) => forbiddenTrustClaims.test(line))
        .map(({ relativePath: path, lineNumber, line }) => `${path}:${lineNumber}: ${line.trim()}`);
    });

    expect(offenders).toEqual([]);
  });

  it("keeps the current Plan Preview screen preview-only", () => {
    const source = readFileSync(
      join(process.cwd(), "src/screens/StagingScreen.tsx"),
      "utf8",
    );

    expect(source).toMatch(/preview-only/i);
    expect(source).toMatch(/No files\s+will be changed/i);
    expect(source).toMatch(/User confirmation required/i);
    expect(source).toMatch(/Plan Preview/);
    expect(source).not.toMatch(/Staging is preview-only|No staged content|Staging will show|Open Staging/i);
    expect(source).not.toMatch(/commitStagingArea|commitAllStagingAreas|cleanupStagingAreas/);
    expect(source).not.toMatch(/Commit to Library|Commit all|Reject all|Reject staged files|Remove staged files/i);
  });

  it("keeps the current Organize screen preview-only", () => {
    const source = readFileSync(
      join(process.cwd(), "src/screens/OrganizeScreen.tsx"),
      "utf8",
    );

    expect(source).toMatch(/generateSortingPreviewPlan/);
    expect(source).toMatch(/No files changed/i);
    expect(source).toMatch(/Preview only/i);
    expect(source).not.toMatch(/previewOrganization|applyPreviewOrganization|listSnapshots|restoreSnapshot|listRulePresets/);
    expect(source).not.toMatch(/Safe to move|Safe move|Ready to move|Auto-sort now|Sort automatically/i);
  });

  it("keeps the current Inbox screen intake/review-only", () => {
    const source = readFileSync(
      join(process.cwd(), "src/screens/DownloadsScreen.tsx"),
      "utf8",
    );

    expect(source).toMatch(/INBOX_FILE_ACTIONS_BLOCKED = true/);
    expect(source).toMatch(/Inbox is review-only/i);
    expect(source).toMatch(/No files changed/i);
    expect(source).not.toMatch(/<button[^>]*>\s*Apply|<button[^>]*>\s*Reject/i);
  });
});
