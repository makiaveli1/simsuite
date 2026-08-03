import assert from "node:assert/strict";
import net from "node:net";
import test from "node:test";
import {
  assertPortAvailable,
  buildPortPreparationInvocation,
  buildTauriBuildInvocation,
  buildTauriDevInvocation,
  normalizeTauriEnvironment,
} from "./dev-runtime.mjs";

const WINDOWS_RELEASE = "10.0.26100";
const WSL_RELEASE = "6.6.87.2-microsoft-standard-WSL2";

function listen(server, host = "127.0.0.1") {
  return new Promise((resolve, reject) => {
    server.once("error", reject);
    server.listen({ host, port: 0 }, () => resolve());
  });
}

function close(server) {
  return new Promise((resolve, reject) => {
    server.close((error) => (error ? reject(error) : resolve()));
  });
}

test("native macOS and Linux launch the local Tauri CLI directly", () => {
  for (const platform of ["darwin", "linux"]) {
    const invocation = buildTauriDevInvocation({
      cwd: "/Users/player/SimSuite",
      env: {},
      platform,
      releaseText: platform === "linux" ? "6.8.0-generic" : "24.0.0",
      passthroughArgs: ["--features", "fixture-mode"],
    });

    assert.equal(invocation.command, process.execPath);
    assert.deepEqual(invocation.args, [
      "/Users/player/SimSuite/node_modules/@tauri-apps/cli/tauri.js",
      "dev",
      "--features",
      "fixture-mode",
    ]);
    assert.equal(invocation.preparesPortInternally, false);
  }
});

test("native Windows keeps the existing guarded PowerShell launcher", () => {
  const invocation = buildTauriDevInvocation({
    cwd: "C:\\Users\\player\\SimSuite",
    env: {},
    platform: "win32",
    releaseText: WINDOWS_RELEASE,
    port: 1420,
    passthroughArgs: ["--features", "fixture-mode"],
  });

  assert.equal(invocation.command, "powershell.exe");
  assert.deepEqual(invocation.args.slice(0, 4), [
    "-NoProfile",
    "-ExecutionPolicy",
    "Bypass",
    "-File",
  ]);
  assert.deepEqual(invocation.args.slice(-4), [
    "-Port",
    "1420",
    "--features",
    "fixture-mode",
  ]);
  assert.match(invocation.args[4], /scripts\\dev\\run-tauri-dev\.ps1$/i);
  assert.equal(invocation.preparesPortInternally, true);
});

test("WSL dispatches to Windows PowerShell without pretending to be native Linux", () => {
  const invocation = buildPortPreparationInvocation({
    cwd: "/mnt/c/Users/player/SimSuite",
    env: { WSL_DISTRO_NAME: "Ubuntu" },
    platform: "linux",
    releaseText: WSL_RELEASE,
    port: 1420,
    quiet: true,
  });

  assert.ok(invocation);
  assert.equal(
    invocation.command,
    "/mnt/c/Windows/System32/WindowsPowerShell/v1.0/powershell.exe",
  );
  assert.match(invocation.args[4], /^C:\\Users\\player\\SimSuite\\scripts\\dev\\cleanup-dev-port\.ps1$/i);
  assert.deepEqual(invocation.args.slice(-3), ["-Port", "1420", "-Quiet"]);
});

test("Tauri wrappers normalize CI for strict boolean parsing", () => {
  assert.equal(normalizeTauriEnvironment({ CI: "1", KEEP: "yes" }).CI, "true");
  assert.equal(normalizeTauriEnvironment({ CI: "0" }).CI, "false");
  assert.equal(normalizeTauriEnvironment({ CI: "false" }).CI, "false");
  assert.equal(normalizeTauriEnvironment({ KEEP: "yes" }).KEEP, "yes");

  const macInvocation = buildTauriBuildInvocation({
    cwd: "/Users/player/SimSuite",
    env: { CI: "1" },
    platform: "darwin",
    passthroughArgs: ["--no-sign"],
  });
  assert.equal(macInvocation.command, process.execPath);
  assert.deepEqual(macInvocation.args, [
    "/Users/player/SimSuite/node_modules/@tauri-apps/cli/tauri.js",
    "build",
    "--no-sign",
  ]);
  assert.equal(macInvocation.env.CI, "true");

  const windowsInvocation = buildTauriBuildInvocation({
    cwd: "C:\\Users\\player\\SimSuite",
    env: { CI: "0" },
    platform: "win32",
  });
  assert.equal(windowsInvocation.command, process.execPath);
  assert.deepEqual(windowsInvocation.args, [
    "C:\\Users\\player\\SimSuite\\node_modules\\@tauri-apps\\cli\\tauri.js",
    "build",
  ]);
  assert.equal(windowsInvocation.env.CI, "false");
});

test("native port preparation never kills an unknown listener", async () => {
  const server = net.createServer();
  await listen(server);
  const address = server.address();
  assert.ok(address && typeof address === "object");

  await assert.rejects(
    assertPortAvailable(address.port),
    /already in use.*did not stop an unknown process automatically/i,
  );

  await close(server);
  await assert.doesNotReject(assertPortAvailable(address.port));
});


test("native port preparation checks IPv6 loopback when available", async (context) => {
  const server = net.createServer();
  try {
    await listen(server, "::1");
  } catch (error) {
    if (error && ["EAFNOSUPPORT", "EADDRNOTAVAIL"].includes(error.code)) {
      context.skip("IPv6 loopback is unavailable on this host");
      return;
    }
    throw error;
  }

  const address = server.address();
  assert.ok(address && typeof address === "object");
  try {
    await assert.rejects(
      assertPortAvailable(address.port),
      /already in use.*did not stop an unknown process automatically/i,
    );
  } finally {
    await close(server);
  }
});
