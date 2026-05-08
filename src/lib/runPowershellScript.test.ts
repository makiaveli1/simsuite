import { describe, expect, it } from "vitest";
// @ts-expect-error The desktop wrapper is a Node ESM script outside the TS app bundle.
import { buildPowerShellInvocation, toWindowsPath } from "../../scripts/desktop/run-powershell-script.mjs";

describe("desktop PowerShell wrapper", () => {
  it("converts WSL repo script paths before invoking Windows PowerShell", () => {
    const invocation = buildPowerShellInvocation({
      cwd: "/mnt/c/Users/likwi/OneDrive/Desktop/PROJS/SimSort",
      env: { WSL_DISTRO_NAME: "Ubuntu" },
      platform: "linux",
      script: "scripts/desktop/run-tauri-smoke.ps1",
      scriptArgs: ["-IncludeApply"],
      releaseText: "Linux version microsoft-standard-WSL2",
    });

    expect(invocation.executable).toBe("/mnt/c/Windows/System32/WindowsPowerShell/v1.0/powershell.exe");
    expect(invocation.args).toContain("-File");
    expect(invocation.args).toContain(
      "C:\\Users\\likwi\\OneDrive\\Desktop\\PROJS\\SimSort\\scripts\\desktop\\run-tauri-smoke.ps1",
    );
    expect(invocation.args).toContain("-IncludeApply");
  });

  it("uses native PowerShell and native paths on Windows", () => {
    const invocation = buildPowerShellInvocation({
      cwd: "C:\\Users\\likwi\\OneDrive\\Desktop\\PROJS\\SimSort",
      env: {},
      platform: "win32",
      script: "scripts/desktop/run-desktop-library-proof.ps1",
    });

    expect(invocation.executable).toBe("powershell.exe");
    expect(invocation.args).toContain(
      "C:\\Users\\likwi\\OneDrive\\Desktop\\PROJS\\SimSort\\scripts\\desktop\\run-desktop-library-proof.ps1",
    );
  });

  it("rejects non-WSL Unix paths because Windows PowerShell cannot read them", () => {
    expect(() => toWindowsPath("/home/user/SimSort/scripts/desktop/run.ps1")).toThrow(
      /Cannot convert/,
    );
  });
});
