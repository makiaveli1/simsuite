import {
  CircleCheck,
  FolderTree,
  Gamepad2,
  LoaderCircle,
  ShieldCheck,
  TriangleAlert,
} from "lucide-react";
import type {
  GameInstallationAdapterReadinessReport,
  GameInstallationEvidenceStrength,
  GameInstallationProfile,
  GameInstallationProfileStatus,
  GameInstallationProfileValidationReport,
  GameInstallationRootValidationReport,
} from "../../lib/types";

interface SettingsGameProfileSectionProps {
  profile: GameInstallationProfile | null | undefined;
  validation: GameInstallationProfileValidationReport | null;
  error: string | null;
  isLoading: boolean;
}

export function SettingsGameProfileSection({
  profile,
  validation,
  error,
  isLoading,
}: SettingsGameProfileSectionProps) {
  if (isLoading) {
    return (
      <>
        <SettingsGameProfileHeading />
        <div className="settings-profile-empty" role="status">
          <LoaderCircle size={18} strokeWidth={2} className="spin" />
          <div>
            <strong>Checking your active game setup</strong>
            <p className="workspace-toolbar-copy">
              Reading the saved profile and running a transient, read-only root check.
            </p>
          </div>
        </div>
      </>
    );
  }

  if (profile === undefined) {
    return (
      <>
        <SettingsGameProfileHeading />
        <div className="settings-profile-alert" role="alert">
          <TriangleAlert size={18} strokeWidth={2} />
          <div>
            <strong>Active game profile could not be read</strong>
            <p>{error ?? "The saved profile lookup did not finish."}</p>
            <span>No profile or filesystem state was changed.</span>
          </div>
        </div>
        <SettingsProfileSafetyBoundary />
      </>
    );
  }

  if (!profile) {
    return (
      <>
        <SettingsGameProfileHeading />
        <div className="settings-profile-empty">
          <TriangleAlert size={18} strokeWidth={2} />
          <div>
            <strong>No active game profile</strong>
            <p className="workspace-toolbar-copy">
              SimSuite has not selected a saved Sims 4 setup. Profile creation and
              selection are not available in this read-only phase.
            </p>
          </div>
        </div>
        <SettingsProfileSafetyBoundary />
      </>
    );
  }

  const status = validation?.state ?? profile.status;
  const genericStatus = validation?.genericRootState ?? profile.status;
  const readiness = validation?.gameReadiness ?? null;
  const displayedRoots: GameInstallationRootValidationReport[] =
    validation?.roots ??
    profile.roots.map((root) => ({
      rootId: root.rootId,
      rootRole: root.rootRole,
      configuredPath: root.configuredPath,
      required: root.required,
      state: root.validationState,
      absolutePath: null,
      exists: null,
      metadataReadable: null,
      metadataState: "not_checked",
      canonicalPathDisplay: null,
      caseSensitivity: "not_checked",
      symlinkObserved: null,
      blockers: [],
      reviewNotes: ["Transient validation evidence is not available."],
    }));

  return (
    <>
      <SettingsGameProfileHeading />

      {error ? (
        <div className="settings-profile-alert" role="alert">
          <TriangleAlert size={16} strokeWidth={2} />
          <div>
            <strong>Live validation could not finish</strong>
            <p>{error}</p>
            <span>The saved profile is still shown below. No stored state was changed.</span>
          </div>
        </div>
      ) : null}

      <div className="settings-profile-overview">
        <div className="settings-profile-identity">
          <span className="section-label">Active profile</span>
          <strong>{profile.profileName}</strong>
          <p className="workspace-toolbar-copy">
            {gameLabel(profile.gameId)} · {environmentLabel(profile.operatingEnvironment)}
          </p>
          <div className="settings-profile-chip-row">
            <span className="ghost-chip">{confirmationLabel(profile.confirmationState)}</span>
            <span className="ghost-chip">{detectionMethodLabel(profile.detectionMethod)}</span>
          </div>
        </div>

        <div className={`settings-profile-readiness ${profileStatusTone(status)}`}>
          <ProfileStatusIcon status={status} />
          <div>
            <span>Overall readiness</span>
            <strong>{profileStatusLabel(status)}</strong>
            <p>
              Generic roots: {profileStatusLabel(genericStatus)}
              {readiness ? ` · ${adapterLabel(readiness.adapterId)}` : " · adapter pending"}
            </p>
          </div>
        </div>
      </div>

      <div className="settings-profile-facts" role="group" aria-label="Profile facts">
        <div>
          <span>Stored environment</span>
          <strong>{environmentLabel(profile.operatingEnvironment)}</strong>
        </div>
        <div>
          <span>Host compatibility</span>
          <strong>
            {validation
              ? environmentCompatibilityLabel(validation.environmentCompatibility)
              : "Not checked"}
          </strong>
        </div>
        <div>
          <span>Validation mode</span>
          <strong>{validation?.readOnly ? "Read only" : "Stored status only"}</strong>
        </div>
        <div>
          <span>Configured roots</span>
          <strong>{displayedRoots.length}</strong>
        </div>
      </div>

      <section className="settings-profile-section" aria-labelledby="profile-roots-heading">
        <div className="settings-profile-section-heading">
          <div>
            <span className="section-label">
              <FolderTree size={14} strokeWidth={2} />
              Configured roots
            </span>
            <h3 id="profile-roots-heading">What SimSuite can read</h3>
          </div>
          <span className="ghost-chip">Stable root IDs</span>
        </div>

        <div className="settings-profile-root-list" role="list">
          {displayedRoots.map((root) => (
            <div className="settings-profile-root-row" role="listitem" key={root.rootId}>
              <div className="settings-profile-root-name">
                <span>{rootLabel(root.rootId)}</span>
                <strong>{root.rootId}</strong>
              </div>
              <div className="settings-profile-root-path">
                <strong title={root.configuredPath}>{root.configuredPath}</strong>
                {root.canonicalPathDisplay ? (
                  <span title={root.canonicalPathDisplay}>
                    Canonical: {root.canonicalPathDisplay}
                  </span>
                ) : null}
                <span>{rootMetadataLabel(root)}</span>
              </div>
              <div
                className={`settings-profile-root-state ${profileStatusTone(
                  rootStateAsProfileStatus(root.state),
                )}`}
              >
                <span>{root.required ? "Required" : "Optional"}</span>
                <strong>{rootStateLabel(root.state)}</strong>
              </div>
            </div>
          ))}
        </div>
      </section>

      <div className="settings-profile-detail-grid">
        <SettingsAdapterReadiness readiness={readiness} />
        <SettingsProfileReasons validation={validation} error={error} />
      </div>

      <SettingsProfileSafetyBoundary />
    </>
  );
}

function SettingsGameProfileHeading() {
  return (
    <div className="panel-heading settings-focus-heading">
      <div>
        <span className="section-label">
          <Gamepad2 size={14} strokeWidth={2} />
          Game setup
        </span>
        <h2>Your active Sims 4 profile</h2>
      </div>
      <p className="workspace-toolbar-copy">
        Review saved roots and current readiness evidence. This screen cannot edit,
        select, discover, or create profiles.
      </p>
    </div>
  );
}

function SettingsAdapterReadiness({
  readiness,
}: {
  readiness: GameInstallationAdapterReadinessReport | null;
}) {
  return (
    <section className="settings-profile-section settings-profile-evidence-panel">
      <div className="settings-profile-section-heading">
        <div>
          <span className="section-label">
            <ShieldCheck size={14} strokeWidth={2} />
            Sims 4 evidence
          </span>
          <h3>Game-specific readiness</h3>
        </div>
        <span className="ghost-chip">
          {readiness ? profileStatusLabel(readiness.state) : "Not available"}
        </span>
      </div>

      {readiness ? (
        <>
          <div className="settings-profile-rule-strip">
            <span>Mods: {readiness.supportedModExtensions.join(", ")}</span>
            <span>Tray: {readiness.supportedTrayExtensions.length} formats</span>
            <span>Script depth: {readiness.maxScriptFolderDepth}</span>
            <span>Package depth: {readiness.maxPackageFolderDepth}</span>
          </div>
          <div className="settings-profile-evidence-list" role="list">
            {readiness.evidence.map((item) => (
              <div className="settings-profile-evidence-row" role="listitem" key={item.code}>
                <span className={`settings-profile-evidence-strength is-${item.strength}`}>
                  {evidenceStrengthLabel(item.strength)}
                </span>
                <div>
                  <strong>{evidenceCodeLabel(item.code)}</strong>
                  <p>{item.summary}</p>
                  {item.rootIds.length > 0 ? (
                    <span>Roots: {item.rootIds.join(", ")}</span>
                  ) : null}
                </div>
              </div>
            ))}
          </div>
        </>
      ) : (
        <p className="workspace-toolbar-copy">
          No matching game adapter report is available for this profile and host.
        </p>
      )}
    </section>
  );
}

function SettingsProfileReasons({
  validation,
  error,
}: {
  validation: GameInstallationProfileValidationReport | null;
  error: string | null;
}) {
  const blockers = validation?.blockers ?? [];
  const reviewNotes = validation?.reviewNotes ?? [];
  const reasonCount = blockers.length + reviewNotes.length + (error ? 1 : 0);

  return (
    <section className="settings-profile-section settings-profile-reasons-panel">
      <div className="settings-profile-section-heading">
        <div>
          <span className="section-label">
            <TriangleAlert size={14} strokeWidth={2} />
            Reasons
          </span>
          <h3>What needs attention</h3>
        </div>
        <span className="ghost-chip">{reasonCount}</span>
      </div>

      {blockers.length === 0 && reviewNotes.length === 0 && !error ? (
        <div className="settings-profile-clear-state">
          <CircleCheck size={16} strokeWidth={2} />
          <span>No blockers or review notes were reported.</span>
        </div>
      ) : (
        <div className="settings-profile-reason-list" role="list">
          {blockers.map((message) => (
            <div className="settings-profile-reason is-blocker" role="listitem" key={`blocker-${message}`}>
              <strong>Blocked</strong>
              <span>{message}</span>
            </div>
          ))}
          {reviewNotes.map((message) => (
            <div className="settings-profile-reason is-review" role="listitem" key={`review-${message}`}>
              <strong>Review</strong>
              <span>{message}</span>
            </div>
          ))}
          {error ? (
            <div className="settings-profile-reason is-review" role="listitem">
              <strong>Live check</strong>
              <span>{error}</span>
            </div>
          ) : null}
        </div>
      )}
    </section>
  );
}

function SettingsProfileSafetyBoundary() {
  return (
    <div className="settings-profile-safety-note">
      <ShieldCheck size={15} strokeWidth={2} />
      <div>
        <strong>Read-only setup evidence</strong>
        <span>
          No files changed. No profile status or filesystem capability was saved. Apply,
          Restore, profile editing, profile selection, and discovery remain unavailable.
        </span>
      </div>
    </div>
  );
}

function ProfileStatusIcon({ status }: { status: GameInstallationProfileStatus }) {
  return status === "valid" ? (
    <CircleCheck size={22} strokeWidth={2} />
  ) : (
    <TriangleAlert size={22} strokeWidth={2} />
  );
}

function profileStatusLabel(status: GameInstallationProfileStatus) {
  switch (status) {
    case "valid":
      return "Ready";
    case "needs_review":
      return "Needs review";
    case "unavailable":
      return "Unavailable";
    case "draft":
      return "Draft";
  }
}

function profileStatusTone(status: GameInstallationProfileStatus) {
  switch (status) {
    case "valid":
      return "is-valid";
    case "unavailable":
      return "is-unavailable";
    case "draft":
      return "is-draft";
    case "needs_review":
      return "is-review";
  }
}

function rootStateAsProfileStatus(
  state: GameInstallationRootValidationReport["state"],
): GameInstallationProfileStatus {
  switch (state) {
    case "valid":
      return "valid";
    case "unavailable":
      return "unavailable";
    case "unvalidated":
    case "needs_review":
      return "needs_review";
  }
}

function rootStateLabel(state: GameInstallationRootValidationReport["state"]) {
  switch (state) {
    case "valid":
      return "Ready";
    case "needs_review":
      return "Needs review";
    case "unavailable":
      return "Unavailable";
    case "unvalidated":
      return "Not checked";
  }
}

function rootLabel(rootId: string) {
  const labels: Record<string, string> = {
    user_data: "Sims 4 user data",
    mods: "Installed Mods",
    tray: "Tray library",
    downloads: "Downloads intake",
    reject: "Rejected downloads",
  };
  return labels[rootId] ?? rootId.replace(/_/g, " ");
}

function rootMetadataLabel(root: GameInstallationRootValidationReport) {
  if (root.metadataState === "not_checked") {
    return "Filesystem evidence not checked";
  }
  if (root.metadataState === "directory") {
    return `Directory · ${caseSensitivityLabel(root.caseSensitivity)}`;
  }
  if (root.metadataState === "symlink") {
    return `Alias or symlink · ${caseSensitivityLabel(root.caseSensitivity)}`;
  }
  return root.metadataState.replace(/_/g, " ");
}

function caseSensitivityLabel(
  state: GameInstallationRootValidationReport["caseSensitivity"],
) {
  switch (state) {
    case "sensitive":
      return "case-sensitive";
    case "insensitive":
      return "case-insensitive";
    case "unknown":
      return "case behavior unknown";
    case "not_checked":
      return "case behavior not checked";
  }
}

function gameLabel(gameId: string) {
  return gameId === "sims4" ? "The Sims 4" : gameId;
}

function environmentLabel(
  environment: GameInstallationProfile["operatingEnvironment"],
) {
  switch (environment) {
    case "native_windows":
      return "Native Windows";
    case "native_macos":
      return "Native macOS";
    case "native_linux":
      return "Native Linux";
    case "wine":
      return "Wine";
    case "proton":
      return "Proton";
    case "lutris":
      return "Lutris";
    case "unknown":
      return "Unknown environment";
  }
}

function environmentCompatibilityLabel(
  compatibility: GameInstallationProfileValidationReport["environmentCompatibility"],
) {
  switch (compatibility) {
    case "matches":
      return "Matches this computer";
    case "mismatch":
      return "Different operating system";
    case "requires_adapter":
      return "Adapter required";
    case "unknown":
      return "Unknown";
  }
}

function confirmationLabel(
  confirmation: GameInstallationProfile["confirmationState"],
) {
  switch (confirmation) {
    case "confirmed":
      return "Player confirmed";
    case "unconfirmed":
      return "Not confirmed";
    case "confirmation_stale":
      return "Confirmation needs review";
  }
}

function detectionMethodLabel(
  method: GameInstallationProfile["detectionMethod"],
) {
  switch (method) {
    case "manual":
      return "Manual profile";
    case "legacy_settings_migration":
      return "Migrated folder settings";
    case "platform_candidate":
      return "Platform candidate";
  }
}

function evidenceStrengthLabel(strength: GameInstallationEvidenceStrength) {
  switch (strength) {
    case "confirmed":
      return "Confirmed";
    case "strongly_supported":
      return "Strong support";
    case "possible":
      return "Possible";
    case "unknown":
      return "Unknown";
  }
}

function adapterLabel(adapterId: string) {
  return adapterId === "sims4_v1" ? "Sims 4 rules" : "Game rules";
}

function evidenceCodeLabel(code: string) {
  const labels: Record<string, string> = {
    sims4_adapter_contract: "Sims 4 rules loaded",
    sims4_user_data_confirmed: "Explicit user-data tree confirmed",
    sims4_user_data_inferred: "Shared user-data tree supported",
    sims4_user_data_nested: "Custom nested layout",
    sims4_user_data_conflict: "Roots belong to different trees",
    sims4_user_data_escape: "Root outside user-data tree",
    sims4_root_roles_overlap: "Mods and Tray overlap",
    sims4_user_data_role_overlap: "User-data role overlaps a child root",
    sims4_user_data_parent_too_broad: "Shared parent is too broad",
    sims4_root_coherence_unknown: "Root coherence unknown",
  };
  return labels[code] ?? code.replace(/_/g, " ");
}
