import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import path from "node:path";
import test from "node:test";

const helperPaths = [
  "smoke-library-redesign.cjs",
  "verify-phase5ag.cjs",
  "verify-phase5ag.mjs",
  "mode-compare.cjs",
  "scripts/desktop/library-inspector-sheet-audit.mjs",
];

for (const helperPath of helperPaths) {
  test(`${helperPath} parses with the active Node runtime`, () => {
    const absolutePath = path.resolve(helperPath);
    const result = spawnSync(process.execPath, ["--check", absolutePath], {
      encoding: "utf8",
    });

    assert.equal(
      result.status,
      0,
      `${helperPath} failed syntax validation:\n${result.stderr || result.stdout}`,
    );
  });
}
