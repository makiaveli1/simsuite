const path = require("node:path");
const { spawnSync } = require("node:child_process");

// Deprecated compatibility entry point. The maintained Library proof uses the
// canonical Tauri WebDriver lane through the portable PowerShell dispatcher.
console.warn(
  "smoke-library-redesign.cjs is deprecated; running the canonical desktop Library proof.",
);

const result = spawnSync(
  process.execPath,
  [
    path.join(__dirname, "scripts", "desktop", "run-powershell-script.mjs"),
    "scripts/desktop/run-desktop-library-proof.ps1",
  ],
  {
    cwd: __dirname,
    stdio: "inherit",
  },
);

if (result.error) {
  console.error("Could not launch the canonical Library proof:", result.error.message);
  process.exit(1);
}

process.exit(result.status ?? 1);
