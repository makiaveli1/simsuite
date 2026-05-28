import { describe, expect, it } from "vitest";
// @ts-expect-error Validation wrappers are Node ESM scripts outside the TS app bundle.
import { buildVitestInvocation, chunkTestFiles, shouldRunVitestInBatches } from "../../scripts/test/run-vitest.mjs";
// @ts-expect-error Validation wrappers are Node ESM scripts outside the TS app bundle.
import { buildRustTestInvocation } from "../../scripts/test/run-rust-tests.mjs";

describe("validation script wrappers", () => {
  it("forces Vitest to run with NODE_ENV=test while preserving caller arguments", () => {
    const invocation = buildVitestInvocation({
      argv: ["node", "scripts/test/run-vitest.mjs", "src/trustBoundaryCopy.test.ts", "--runInBand"],
      env: { NODE_ENV: "production", SIMSUITE_FLAG: "kept" },
      platform: "linux",
    });

    expect(invocation.command).toBe("npx");
    expect(invocation.args).toEqual(["vitest", "run", "src/trustBoundaryCopy.test.ts", "--runInBand"]);
    expect(invocation.env.NODE_ENV).toBe("test");
    expect(invocation.env.SIMSUITE_FLAG).toBe("kept");
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

  it("uses the Windows PowerShell Rust lane from WSL so cargo path semantics match the desktop target", () => {
    const invocation = buildRustTestInvocation({
      cwd: "/mnt/c/Users/likwi/OneDrive/Desktop/PROJS/SimSort",
      argv: ["node", "scripts/test/run-rust-tests.mjs", "core::scanner::tests::scan_empty_roots"],
      env: { WSL_DISTRO_NAME: "Ubuntu" },
      platform: "linux",
      releaseText: "Linux version microsoft-standard-WSL2",
    });

    expect(invocation.command).toBe("/mnt/c/Windows/System32/WindowsPowerShell/v1.0/powershell.exe");
    expect(invocation.args).toContain("-Command");
    expect(invocation.args.join(" ")).toContain("C:\\Users\\likwi\\OneDrive\\Desktop\\PROJS\\SimSort");
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
