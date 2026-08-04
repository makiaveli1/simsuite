const path = require("node:path");
const { spawnSync } = require("node:child_process");

// Deprecated CommonJS compatibility entry point. The maintained smoke check is ESM.
const result = spawnSync(process.execPath, [path.join(__dirname, "verify-phase5ag.mjs")], {
  stdio: "inherit",
});

if (result.error) {
  console.error("Could not launch verify-phase5ag.mjs:", result.error.message);
  process.exit(1);
}

process.exit(result.status ?? 1);
