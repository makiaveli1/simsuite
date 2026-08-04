import assert from "node:assert/strict";
import test from "node:test";

import { reviewWindowsGameProfileProof } from "./review-windows-game-profile-proof.mjs";

const GIT_HEAD = "2945ae26badd8e1311e71c770eb9de50d7fe6060";

function validSummary() {
  return {
    schemaVersion: "1.0",
    proof: "native_windows_game_profile_candidates",
    recordedAtUtc: "2026-08-05T00:30:00.000Z",
    gitHead: GIT_HEAD,
    testFilter: "core::game_installation_candidate_detection::tests::",
    includedIgnoredTests: true,
    host: {
      osVersion: "Microsoft Windows NT 10.0.26100.0",
      architecture: "AMD64",
      powershellVersion: "7.5.2",
    },
    detector: {
      schemaVersion: "1.0",
      proof: "native_windows_game_profile_candidates",
      configuredDocuments: "C:\\Users\\runneradmin\\Documents",
      homeDirectory: "C:\\Users\\runneradmin",
      downloadsDirectory: "C:\\Users\\runneradmin\\Downloads",
      oneDriveEnvironment: {
        OneDrive: null,
        OneDriveConsumer: null,
        OneDriveCommercial: null,
      },
      detectorResult: {
        currentEnvironment: "native_windows",
        supported: true,
        candidates: [],
        readOnly: true,
        reviewNotes: [],
      },
    },
    artifacts: {
      log: "D:\\a\\simsuite\\output\\desktop\\windows-game-profile-proof\\cargo-test.log",
      summary: "D:\\a\\simsuite\\output\\desktop\\windows-game-profile-proof\\summary.json",
    },
  };
}

test("accepts a bounded hosted Windows baseline without unlocking product features", () => {
  const report = reviewWindowsGameProfileProof(validSummary(), { expectedGitHead: GIT_HEAD });

  assert.equal(report.verdict, "pass");
  assert.equal(report.evidenceLevel, "hosted_windows_baseline");
  assert.equal(report.representativeWindowsProofReady, false);
  assert.equal(report.unlocksCandidateRefresh, false);
  assert.equal(report.unlocksFileMutation, false);
  assert.deepEqual(report.failedCheckIds, []);
});

test("rejects a receipt from the wrong Git revision", () => {
  const report = reviewWindowsGameProfileProof(validSummary(), {
    expectedGitHead: "1111111111111111111111111111111111111111",
  });

  assert.equal(report.verdict, "fail");
  assert.ok(report.failedCheckIds.includes("summary.git_match"));
});

test("rejects non-Windows or writable detector evidence", () => {
  const summary = validSummary();
  summary.detector.detectorResult.currentEnvironment = "native_macos";
  summary.detector.detectorResult.readOnly = false;
  summary.detector.detectorResult.candidates = [
    {
      readOnly: false,
      operatingEnvironment: "native_macos",
    },
  ];

  const report = reviewWindowsGameProfileProof(summary, { expectedGitHead: GIT_HEAD });

  assert.equal(report.verdict, "fail");
  assert.ok(report.failedCheckIds.includes("result.environment"));
  assert.ok(report.failedCheckIds.includes("result.read_only"));
  assert.ok(report.failedCheckIds.includes("result.candidates_read_only"));
});

test("rejects malformed known-folder and OneDrive evidence", () => {
  const summary = validSummary();
  summary.detector.configuredDocuments = "Documents";
  summary.detector.oneDriveEnvironment.OneDriveCommercial = "relative\\OneDrive";
  delete summary.detector.oneDriveEnvironment.OneDriveConsumer;

  const report = reviewWindowsGameProfileProof(summary, { expectedGitHead: GIT_HEAD });

  assert.equal(report.verdict, "fail");
  assert.ok(report.failedCheckIds.includes("detector.documents"));
  assert.ok(report.failedCheckIds.includes("detector.onedrive.OneDriveConsumer"));
  assert.ok(report.failedCheckIds.includes("detector.onedrive.OneDriveCommercial"));
});

test("accepts read-only native Windows candidates when they exist", () => {
  const summary = validSummary();
  summary.detector.detectorResult.candidates = [
    {
      candidateId: "windows_configured_documents",
      operatingEnvironment: "native_windows",
      readOnly: true,
    },
  ];

  const report = reviewWindowsGameProfileProof(summary, { expectedGitHead: GIT_HEAD });

  assert.equal(report.verdict, "pass");
  assert.equal(report.candidateCount, 1);
});
