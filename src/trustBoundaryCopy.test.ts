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
});
