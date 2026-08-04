import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";

const PROOF_NAME = "native_windows_game_profile_candidates";
const TEST_FILTER = "core::game_installation_candidate_detection::tests::";
const DEFAULT_SUMMARY_PATH = "output/desktop/windows-game-profile-proof/latest-summary.json";
const ONE_DRIVE_KEYS = ["OneDrive", "OneDriveConsumer", "OneDriveCommercial"];

function isObject(value) {
  return value !== null && typeof value === "object" && !Array.isArray(value);
}

function isNonEmptyString(value) {
  return typeof value === "string" && value.trim().length > 0;
}

function isWindowsAbsolutePath(value) {
  if (!isNonEmptyString(value)) {
    return false;
  }
  return /^[A-Za-z]:[\\/]/.test(value) || /^\\\\[^\\]+\\[^\\]+/.test(value);
}

function isIsoTimestamp(value) {
  return isNonEmptyString(value) && value.endsWith("Z") && Number.isFinite(Date.parse(value));
}

function isGitSha(value) {
  return typeof value === "string" && /^[0-9a-f]{40}$/i.test(value);
}

function readCurrentGitHead(cwd = process.cwd()) {
  try {
    return execFileSync("git", ["-C", cwd, "rev-parse", "HEAD"], {
      encoding: "utf8",
      stdio: ["ignore", "pipe", "ignore"],
    }).trim();
  } catch {
    return null;
  }
}

function createCheck(checks, id, label, passed, actual = undefined) {
  checks.push({ id, label, passed: Boolean(passed), ...(actual === undefined ? {} : { actual }) });
}

export function reviewWindowsGameProfileProof(summary, { expectedGitHead = null } = {}) {
  const checks = [];
  const root = isObject(summary) ? summary : {};

  createCheck(checks, "summary.object", "Receipt is a JSON object", isObject(summary));
  createCheck(checks, "summary.schema", "Outer receipt schema is 1.0", root.schemaVersion === "1.0", root.schemaVersion);
  createCheck(checks, "summary.proof", "Outer proof identity is correct", root.proof === PROOF_NAME, root.proof);
  createCheck(checks, "summary.timestamp", "Receipt has a valid UTC timestamp", isIsoTimestamp(root.recordedAtUtc), root.recordedAtUtc);
  createCheck(checks, "summary.git", "Receipt records a full Git SHA", isGitSha(root.gitHead), root.gitHead);
  if (expectedGitHead) {
    createCheck(
      checks,
      "summary.git_match",
      "Receipt Git SHA matches the expected revision",
      root.gitHead === expectedGitHead,
      { expected: expectedGitHead, actual: root.gitHead },
    );
  }
  createCheck(checks, "summary.test_filter", "Receipt records the candidate test module", root.testFilter === TEST_FILTER, root.testFilter);
  createCheck(checks, "summary.ignored", "Windows-only ignored tests were included", root.includedIgnoredTests === true, root.includedIgnoredTests);

  const host = isObject(root.host) ? root.host : {};
  createCheck(checks, "host.object", "Host evidence is present", isObject(root.host));
  createCheck(
    checks,
    "host.os",
    "Windows OS version is recorded",
    isNonEmptyString(host.osVersion) && /windows/i.test(host.osVersion),
    host.osVersion,
  );
  createCheck(checks, "host.arch", "Processor architecture is recorded", isNonEmptyString(host.architecture), host.architecture);
  createCheck(checks, "host.powershell", "PowerShell version is recorded", isNonEmptyString(host.powershellVersion), host.powershellVersion);

  const detector = isObject(root.detector) ? root.detector : {};
  createCheck(checks, "detector.object", "Native detector receipt is present", isObject(root.detector));
  createCheck(checks, "detector.schema", "Detector receipt schema is 1.0", detector.schemaVersion === "1.0", detector.schemaVersion);
  createCheck(checks, "detector.proof", "Detector proof identity is correct", detector.proof === PROOF_NAME, detector.proof);
  createCheck(
    checks,
    "detector.documents",
    "Configured Documents is an absolute Windows path",
    isWindowsAbsolutePath(detector.configuredDocuments),
    detector.configuredDocuments,
  );
  createCheck(
    checks,
    "detector.home",
    "Home directory is an absolute Windows path",
    isWindowsAbsolutePath(detector.homeDirectory),
    detector.homeDirectory,
  );
  createCheck(
    checks,
    "detector.downloads",
    "Downloads is absent or an absolute Windows path",
    detector.downloadsDirectory == null || isWindowsAbsolutePath(detector.downloadsDirectory),
    detector.downloadsDirectory,
  );

  const oneDrive = isObject(detector.oneDriveEnvironment) ? detector.oneDriveEnvironment : {};
  createCheck(checks, "detector.onedrive_object", "OneDrive environment evidence is present", isObject(detector.oneDriveEnvironment));
  for (const key of ONE_DRIVE_KEYS) {
    createCheck(
      checks,
      `detector.onedrive.${key}`,
      `${key} is absent or an absolute Windows path`,
      Object.hasOwn(oneDrive, key) && (oneDrive[key] == null || isWindowsAbsolutePath(oneDrive[key])),
      oneDrive[key],
    );
  }

  const detectorResult = isObject(detector.detectorResult) ? detector.detectorResult : {};
  createCheck(checks, "result.object", "Detector result is present", isObject(detector.detectorResult));
  createCheck(
    checks,
    "result.environment",
    "Detector ran as native Windows",
    detectorResult.currentEnvironment === "native_windows",
    detectorResult.currentEnvironment,
  );
  createCheck(checks, "result.supported", "Windows candidate detection is supported", detectorResult.supported === true, detectorResult.supported);
  createCheck(checks, "result.read_only", "Detector result is read-only", detectorResult.readOnly === true, detectorResult.readOnly);
  createCheck(checks, "result.candidates", "Candidate list is present", Array.isArray(detectorResult.candidates));
  createCheck(checks, "result.review_notes", "Review notes are present", Array.isArray(detectorResult.reviewNotes));

  const candidates = Array.isArray(detectorResult.candidates) ? detectorResult.candidates : [];
  createCheck(
    checks,
    "result.candidates_read_only",
    "Every detected candidate is native-Windows and read-only",
    candidates.every(
      (candidate) =>
        isObject(candidate) &&
        candidate.readOnly === true &&
        candidate.operatingEnvironment === "native_windows",
    ),
    { count: candidates.length },
  );

  const artifacts = isObject(root.artifacts) ? root.artifacts : {};
  createCheck(checks, "artifacts.object", "Artifact metadata is present", isObject(root.artifacts));
  createCheck(checks, "artifacts.log", "Cargo test log path is recorded", isNonEmptyString(artifacts.log), artifacts.log);
  createCheck(checks, "artifacts.summary", "Summary path is recorded", isNonEmptyString(artifacts.summary), artifacts.summary);

  const failedChecks = checks.filter((check) => !check.passed);
  return {
    schemaVersion: "1.0",
    review: "windows_game_profile_proof",
    verdict: failedChecks.length === 0 ? "pass" : "fail",
    evidenceLevel: "hosted_windows_baseline",
    gitHead: isGitSha(root.gitHead) ? root.gitHead : null,
    candidateCount: candidates.length,
    checks,
    failedCheckIds: failedChecks.map((check) => check.id),
    representativeWindowsProofReady: false,
    unlocksCandidateRefresh: false,
    unlocksFileMutation: false,
    limitations: [
      "A hosted runner does not reproduce a player's redirected Documents or signed-in OneDrive layout.",
      "A hosted pass does not prove representative permissions, inaccessible folders, case aliases, or desktop interaction.",
      "Candidate refresh, Apply, Restore, and other file-changing capability remain locked.",
    ],
  };
}

function parseArguments(argv) {
  const args = [...argv];
  let summaryPath = DEFAULT_SUMMARY_PATH;
  let expectedGitHead = null;
  let json = false;

  while (args.length > 0) {
    const argument = args.shift();
    if (argument === "--summary") {
      summaryPath = args.shift();
    } else if (argument === "--expected-git") {
      expectedGitHead = args.shift();
    } else if (argument === "--json") {
      json = true;
    } else if (argument?.startsWith("--")) {
      throw new Error(`Unknown argument: ${argument}`);
    } else if (argument) {
      summaryPath = argument;
    }
  }

  if (!isNonEmptyString(summaryPath)) {
    throw new Error("A proof summary path is required.");
  }
  if (expectedGitHead && !isGitSha(expectedGitHead)) {
    throw new Error("--expected-git must be a full 40-character Git SHA.");
  }

  return { summaryPath, expectedGitHead, json };
}

function printHumanReview(report, summaryPath) {
  console.log(`WINDOWS_GAME_PROFILE_REVIEW verdict=${report.verdict} evidence=${report.evidenceLevel}`);
  console.log(`summary=${summaryPath}`);
  console.log(`git=${report.gitHead ?? "unavailable"} candidates=${report.candidateCount}`);
  for (const check of report.checks) {
    console.log(`${check.passed ? "PASS" : "FAIL"} ${check.id} ${check.label}`);
  }
  console.log("Representative player-machine proof remains required.");
  console.log("Candidate refresh and file mutation remain locked.");
}

async function main() {
  const { summaryPath, expectedGitHead: requestedGitHead, json } = parseArguments(process.argv.slice(2));
  const absoluteSummaryPath = resolve(summaryPath);
  const summary = JSON.parse(readFileSync(absoluteSummaryPath, "utf8"));
  const expectedGitHead = requestedGitHead ?? readCurrentGitHead();
  const report = reviewWindowsGameProfileProof(summary, { expectedGitHead });

  if (json) {
    console.log(JSON.stringify(report, null, 2));
  } else {
    printHumanReview(report, absoluteSummaryPath);
  }

  if (report.verdict !== "pass") {
    process.exitCode = 1;
  }
}

const isDirectRun = process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url);
if (isDirectRun) {
  main().catch((error) => {
    console.error(`WINDOWS_GAME_PROFILE_REVIEW_ERROR ${error instanceof Error ? error.message : String(error)}`);
    process.exitCode = 1;
  });
}
