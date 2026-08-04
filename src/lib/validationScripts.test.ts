// @ts-expect-error This repository-contract test intentionally reads files through Node.
import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
// @ts-expect-error Validation wrappers are Node ESM scripts outside the TS app bundle.
import { buildVitestInvocation, chunkTestFiles, sanitizeNodeOptions, shouldRunVitestInBatches, supportsWebStorageDisable } from "../../scripts/test/run-vitest.mjs";
// @ts-expect-error Validation wrappers are Node ESM scripts outside the TS app bundle.
import { buildRustTestInvocation } from "../../scripts/test/run-rust-tests.mjs";

describe("validation script wrappers", () => {
  it("runs the pinned local Vitest CLI with NODE_ENV=test while preserving caller arguments", () => {
    const invocation = buildVitestInvocation({
      cwd: "/home/player/SimSuite",
      argv: ["node", "scripts/test/run-vitest.mjs", "src/trustBoundaryCopy.test.ts", "--runInBand"],
      env: { NODE_ENV: "production", SIMSUITE_FLAG: "kept" },
      platform: "linux",
      nodeExecutable: "/usr/bin/node",
    });

    expect(invocation.command).toBe("/usr/bin/node");
    expect(invocation.args).toEqual([
      "/home/player/SimSuite/node_modules/vitest/vitest.mjs",
      "run",
      "src/trustBoundaryCopy.test.ts",
      "--runInBand",
    ]);
    expect(invocation.requiredPath).toBe("/home/player/SimSuite/node_modules/vitest/vitest.mjs");
    expect(invocation.env.NODE_ENV).toBe("test");
    expect(invocation.env.SIMSUITE_FLAG).toBe("kept");

    const windowsInvocation = buildVitestInvocation({
      cwd: "C:\\Users\\player\\SimSuite",
      argv: ["node", "scripts/test/run-vitest.mjs"],
      env: {},
      platform: "win32",
      nodeExecutable: "C:\\Program Files\\nodejs\\node.exe",
    });
    expect(windowsInvocation.command).toBe("C:\\Program Files\\nodejs\\node.exe");
    expect(windowsInvocation.requiredPath).toBe(
      "C:\\Users\\player\\SimSuite\\node_modules\\vitest\\vitest.mjs",
    );
  });

  it("disables inherited Node Web Storage only on runtimes that support the flag", () => {
    expect(supportsWebStorageDisable("22.3.0")).toBe(false);
    expect(supportsWebStorageDisable("22.4.0")).toBe(true);
    expect(supportsWebStorageDisable("24.19.0")).toBe(true);
    expect(
      sanitizeNodeOptions(
        "--trace-warnings --experimental-webstorage --localstorage-file=/tmp/node-storage.json",
        "24.19.0",
      ),
    ).toBe("--trace-warnings --no-experimental-webstorage");
    expect(sanitizeNodeOptions("--trace-warnings", "20.19.0")).toBe("--trace-warnings");

    const invocation = buildVitestInvocation({
      env: {
        NODE_OPTIONS:
          "--experimental-webstorage --localstorage-file /tmp/node-storage.json --trace-warnings",
      },
      platform: "darwin",
      nodeVersion: "24.19.0",
    });

    expect(invocation.env.NODE_OPTIONS).toBe("--trace-warnings --no-experimental-webstorage");
  });

  it("batches full Vitest runs so WSL/OneDrive worker startup limits do not sink precommit", () => {
    expect(shouldRunVitestInBatches({ passthroughArgs: [], env: {} })).toBe(true);
    expect(
      shouldRunVitestInBatches({
        passthroughArgs: [],
        env: { SIMSUITE_VITEST_BATCHED: "0" },
      }),
    ).toBe(false);
    expect(
      shouldRunVitestInBatches({
        passthroughArgs: ["src/trustBoundaryCopy.test.ts"],
        env: {},
      }),
    ).toBe(false);

    expect(chunkTestFiles(["a.test.ts", "b.test.ts", "c.test.ts", "d.test.ts"], 3)).toEqual([
      ["a.test.ts", "b.test.ts", "c.test.ts"],
      ["d.test.ts"],
    ]);
  });

  it("keeps Windows desktop verification repo-relative and pnpm-native", () => {
    const canonicalProofScripts = [
      "scripts/desktop/run-desktop-library-proof.ps1",
      "scripts/desktop/run-tauri-smoke.ps1",
    ];

    for (const scriptPath of canonicalProofScripts) {
      const source = readFileSync(scriptPath, "utf8");
      expect(source).toContain("$ErrorActionPreference = 'Stop'");
      expect(source).toContain("pnpm run tauri:build");
      expect(source).not.toMatch(/\bnpm run\b/i);
      expect(source).not.toMatch(/[A-Za-z]:\\Users\\/);
    }

    const compatibilitySource = readFileSync(
      "scripts/desktop/run-tauri-smoke-fixed.ps1",
      "utf8",
    );
    expect(compatibilitySource).toContain("run-tauri-smoke.ps1");
    expect(compatibilitySource).toContain("@PSBoundParameters");
    expect(compatibilitySource).not.toMatch(/\bnpm run\b/i);
    expect(compatibilitySource).not.toMatch(/[A-Za-z]:\\Users\\/);

    const rebuildSource = readFileSync("scripts/dev/rebuild-msi.ps1", "utf8");
    expect(rebuildSource).toContain("$repoRoot = Resolve-Path");
    expect(rebuildSource).toContain("pnpm run tauri:build");
    expect(rebuildSource).not.toMatch(/\bnpm run\b/i);
    expect(rebuildSource).not.toMatch(/[A-Za-z]:\\Users\\/);

    const standaloneSmokeSource = readFileSync("smoke-library-redesign.cjs", "utf8");
    expect(standaloneSmokeSource).toContain("run-powershell-script.mjs");
    expect(standaloneSmokeSource).toContain("run-desktop-library-proof.ps1");
    expect(standaloneSmokeSource).not.toContain("connectOverCDP");

    const portableHelperPaths = [
      "smoke-library-redesign.cjs",
      "verify-phase5ag.cjs",
      "verify-phase5ag.mjs",
      "mode-compare.cjs",
      "scripts/desktop/library-inspector-sheet-audit.mjs",
      "patch-babel.ps1",
    ];
    for (const helperPath of portableHelperPaths) {
      const source = readFileSync(helperPath, "utf8");
      expect(source).not.toMatch(/(?:[A-Za-z]:\\Users\\|\/mnt\/[a-z]\/Users\/|\/home\/[^/]+\/\.openclaw\/)/i);
    }

    const retiredBabelPatch = readFileSync("patch-babel.ps1", "utf8");
    expect(retiredBabelPatch).toContain("legacy node_modules patch is retired");
    expect(retiredBabelPatch).not.toContain("Set-Content");

    const agentInstructions = readFileSync("AGENTS.md", "utf8");
    expect(agentInstructions).toContain("Resolve `anthropic-frontend-design` and `frontend-design` by skill name");
    expect(agentInstructions).not.toMatch(/[A-Za-z]:\/Users\//);
  });

  it("uses the Windows PowerShell Rust lane from WSL so cargo path semantics match the desktop target", () => {
    const invocation = buildRustTestInvocation({
      cwd: "/mnt/c/Users/player/Projects/SimSuite",
      argv: ["node", "scripts/test/run-rust-tests.mjs", "core::scanner::tests::scan_empty_roots"],
      env: { WSL_DISTRO_NAME: "Ubuntu" },
      platform: "linux",
      releaseText: "Linux version microsoft-standard-WSL2",
    });

    expect(invocation.command).toBe("/mnt/c/Windows/System32/WindowsPowerShell/v1.0/powershell.exe");
    expect(invocation.args).toContain("-Command");
    expect(invocation.args.join(" ")).toContain("C:\\Users\\player\\Projects\\SimSuite");
    expect(invocation.args.join(" ")).toContain("cargo test --manifest-path src-tauri/Cargo.toml");
    expect(invocation.args.join(" ")).toContain("core::scanner::tests::scan_empty_roots");
  });

  it("uses local cargo directly outside WSL", () => {
    const invocation = buildRustTestInvocation({
      cwd: "/home/user/SimSort",
      argv: ["node", "scripts/test/run-rust-tests.mjs"],
      env: {},
      platform: "linux",
      releaseText: "Linux version generic",
    });

    expect(invocation.command).toBe("cargo");
    expect(invocation.args).toEqual(["test", "--manifest-path", "src-tauri/Cargo.toml"]);
  });
});
