import assert from "node:assert/strict";
import test from "node:test";

import {
  buildWarningBudgetSteps,
  mergeDenyWarningsFlag,
} from "./run-rust-warning-budget.mjs";

test("buildWarningBudgetSteps runs cargo check with deny warnings", () => {
  const [step] = buildWarningBudgetSteps({ env: { RUSTFLAGS: "-C target-cpu=native" } });

  assert.equal(step.label, "cargo check warning budget");
  assert.equal(step.command, "cargo");
  assert.deepEqual(step.args, ["check", "--manifest-path", "src-tauri/Cargo.toml"]);
  assert.equal(step.env.RUSTFLAGS, "-C target-cpu=native -Dwarnings");
});

test("buildWarningBudgetSteps runs clippy with explicit deny warnings", () => {
  const [, step] = buildWarningBudgetSteps({ env: {} });

  assert.equal(step.label, "cargo clippy warning budget");
  assert.equal(step.command, "cargo");
  assert.deepEqual(step.args, [
    "clippy",
    "--all-targets",
    "--all-features",
    "--manifest-path",
    "src-tauri/Cargo.toml",
    "--",
    "-D",
    "warnings",
  ]);
});

test("mergeDenyWarningsFlag does not duplicate an existing deny-warning flag", () => {
  assert.equal(mergeDenyWarningsFlag("-Dwarnings"), "-Dwarnings");
  assert.equal(mergeDenyWarningsFlag("-D warnings"), "-D warnings");
});
