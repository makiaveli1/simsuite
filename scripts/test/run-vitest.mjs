#!/usr/bin/env node

import { spawnSync } from "node:child_process";
import fs from "node:fs";
import { fileURLToPath } from "node:url";
import path from "node:path";

const DEFAULT_BATCH_SIZE = 3;
const TEST_FILE_PATTERN = /\.test\.(ts|tsx)$/;

function localVitestCliPath(cwd = process.cwd(), platform = process.platform) {
  const pathApi = platform === "win32" ? path.win32 : path;
  return pathApi.join(cwd, "node_modules", "vitest", "vitest.mjs");
}

export function supportsWebStorageDisable(nodeVersion = process.versions.node) {
  const [major = 0, minor = 0] = String(nodeVersion)
    .split(".", 2)
    .map((value) => Number.parseInt(value, 10));
  return major > 22 || (major === 22 && minor >= 4);
}

export function sanitizeNodeOptions(value = "", nodeVersion = process.versions.node) {
  const tokens = String(value).match(/(?:[^\s"']+|"[^"]*"|'[^']*')+/g) ?? [];
  const sanitized = [];

  for (let index = 0; index < tokens.length; index += 1) {
    const token = tokens[index];
    if (token === "--experimental-webstorage" || token === "--no-experimental-webstorage") {
      continue;
    }
    if (token === "--localstorage-file") {
      index += 1;
      continue;
    }
    if (token.startsWith("--localstorage-file=")) {
      continue;
    }
    sanitized.push(token);
  }

  if (supportsWebStorageDisable(nodeVersion)) {
    sanitized.push("--no-experimental-webstorage");
  }
  return sanitized.join(" ");
}

function normalizePathForVitest(filePath) {
  return filePath.split(path.sep).join("/");
}

export function discoverVitestTestFiles({ root = process.cwd(), sourceDir = "src" } = {}) {
  const sourceRoot = path.resolve(root, sourceDir);
  const discovered = [];

  function walk(directory) {
    if (!fs.existsSync(directory)) {
      return;
    }

    for (const entry of fs.readdirSync(directory, { withFileTypes: true })) {
      const entryPath = path.join(directory, entry.name);
      if (entry.isDirectory()) {
        walk(entryPath);
        continue;
      }

      if (entry.isFile() && TEST_FILE_PATTERN.test(entry.name)) {
        discovered.push(normalizePathForVitest(path.relative(root, entryPath)));
      }
    }
  }

  walk(sourceRoot);
  return discovered.sort();
}

export function chunkTestFiles(files, batchSize = DEFAULT_BATCH_SIZE) {
  const normalizedBatchSize = Number.isInteger(batchSize) && batchSize > 0 ? batchSize : DEFAULT_BATCH_SIZE;
  const chunks = [];

  for (let index = 0; index < files.length; index += normalizedBatchSize) {
    chunks.push(files.slice(index, index + normalizedBatchSize));
  }

  return chunks;
}

export function shouldRunVitestInBatches({ passthroughArgs, env = process.env } = {}) {
  return passthroughArgs.length === 0 && env.SIMSUITE_VITEST_BATCHED !== "0";
}

export function buildVitestInvocation({
  cwd = process.cwd(),
  argv = process.argv,
  env = process.env,
  platform = process.platform,
  nodeVersion = process.versions.node,
  nodeExecutable = process.execPath,
} = {}) {
  const passthroughArgs = argv.slice(2);
  const cliPath = localVitestCliPath(cwd, platform);

  return {
    command: nodeExecutable,
    args: [cliPath, "run", ...passthroughArgs],
    cwd,
    requiredPath: cliPath,
    env: {
      ...env,
      NODE_ENV: "test",
      NODE_OPTIONS: sanitizeNodeOptions(env.NODE_OPTIONS, nodeVersion),
    },
    passthroughArgs,
  };
}

function runVitest(invocation, args) {
  const result = spawnSync(invocation.command, args, {
    cwd: invocation.cwd,
    env: invocation.env,
    stdio: "inherit",
    shell: false,
  });

  if (result.error) {
    throw new Error(`Could not start ${invocation.command}: ${result.error.message}`);
  }

  return result.status ?? 1;
}

export function run(argv = process.argv) {
  const invocation = buildVitestInvocation({ argv });
  if (!fs.existsSync(invocation.requiredPath)) {
    throw new Error(
      `SimSuite could not find the local Vitest CLI at ${invocation.requiredPath}. Run pnpm install first.`,
    );
  }

  if (!shouldRunVitestInBatches({ passthroughArgs: invocation.passthroughArgs, env: invocation.env })) {
    return runVitest(invocation, invocation.args);
  }

  const testFiles = discoverVitestTestFiles();
  if (testFiles.length === 0) {
    return runVitest(invocation, invocation.args);
  }

  const configuredBatchSize = Number.parseInt(invocation.env.SIMSUITE_VITEST_BATCH_SIZE ?? "", 10);
  const batches = chunkTestFiles(testFiles, configuredBatchSize);

  for (const [index, batch] of batches.entries()) {
    console.log(`\n[run-vitest] batch ${index + 1}/${batches.length}: ${batch.join(" ")}`);
    const batchPool = invocation.env.SIMSUITE_VITEST_BATCH_POOL ?? "forks";
    const status = runVitest(invocation, [
      invocation.requiredPath,
      "run",
      "--pool",
      batchPool,
      ...batch,
    ]);
    if (status !== 0) {
      return status;
    }
  }

  return 0;
}

function main() {
  try {
    process.exitCode = run();
  } catch (error) {
    console.error(error instanceof Error ? error.message : String(error));
    process.exitCode = 1;
  }
}

const modulePath = fileURLToPath(import.meta.url);
if (process.argv[1] && path.resolve(process.argv[1]) === modulePath) {
  main();
}
