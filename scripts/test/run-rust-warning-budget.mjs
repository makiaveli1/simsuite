#!/usr/bin/env node

import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import path from "node:path";

const MANIFEST_PATH = "src-tauri/Cargo.toml";

export function mergeDenyWarningsFlag(existing = "") {
  const trimmed = String(existing ?? "").trim();
  if (/(^|\s)-D\s*warnings(\s|$)/.test(trimmed)) {
    return trimmed;
  }

  return [trimmed, "-Dwarnings"].filter(Boolean).join(" ");
}

export function buildWarningBudgetSteps({ env = process.env } = {}) {
  return [
    {
      label: "cargo check warning budget",
      command: "cargo",
      args: ["check", "--manifest-path", MANIFEST_PATH],
      env: {
        ...env,
        RUSTFLAGS: mergeDenyWarningsFlag(env.RUSTFLAGS),
      },
    },
    {
      label: "cargo clippy warning budget",
      command: "cargo",
      args: [
        "clippy",
        "--all-targets",
        "--all-features",
        "--manifest-path",
        MANIFEST_PATH,
        "--",
        "-D",
        "warnings",
      ],
      env: {
        ...env,
      },
    },
  ];
}

function formatCommand(step) {
  return [step.command, ...step.args].join(" ");
}

function runStep(step) {
  console.log(`\n[rust-warning-budget] ${step.label}`);
  console.log(`[rust-warning-budget] ${formatCommand(step)}`);

  const result = spawnSync(step.command, step.args, {
    env: step.env,
    stdio: "inherit",
    shell: false,
  });

  if (result.error) {
    throw new Error(`Could not start ${step.command}: ${result.error.message}`);
  }

  return result.status ?? 1;
}

export function run({ steps = buildWarningBudgetSteps() } = {}) {
  for (const step of steps) {
    const status = runStep(step);
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
