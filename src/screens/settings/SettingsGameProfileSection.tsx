import {
  CircleCheck,
  FolderPlus,
  FolderSearch,
  FolderTree,
  Gamepad2,
  LoaderCircle,
  ShieldCheck,
  TriangleAlert,
} from "lucide-react";
import type {
  CreateManualGameInstallationProfileRequest,
  GameInstallationAdapterReadinessReport,
  GameInstallationCandidate,
  GameInstallationCandidateDetectionResult,
  GameInstallationEvidenceStrength,
  GameInstallationProfile,
  GameInstallationProfileStatus,
  GameInstallationProfileValidationReport,
  GameInstallationRootValidationReport,
} from "../../lib/types";

interface SettingsGameProfileSectionProps {
  profile: GameInstallationProfile | null | undefined;
  profiles: GameInstallationProfile[];
  candidateDetection: GameInstallationCandidateDetectionResult | null;
  selectedProfileId: string;
  selectedValidation: GameInstallationProfileValidationReport | null;
  validation: GameInstallationProfileValidationReport | null;
  error: string | null;
  candidateError: string | null;
  selectionCheckError: string | null;
  selectionError: string | null;
  selectionMessage: string | null;
  manualDraft: CreateManualGameInstallationProfileRequest;
  manualSetupOpen: boolean;
  manualError: string | null;
  manualMessage: string | null;
  isLoading: boolean;
  isDetectingCandidates: boolean;
  isCheckingSelection: boolean;
  isSelecting: boolean;
  isCreatingManualProfile: boolean;
  isConfirmingManualProfile: boolean;
  onSelectedProfileIdChange: (profileId: string) => void;
  onSelectProfile: () => void;
  onReviewCandidate: (candidateId: string) => void;
  onToggleManualSetup: () => void;
  onManualDraftChange: (
    values: Partial<CreateManualGameInstallationProfileRequest>,
  ) => void;
  onPickManualFolder: (
    field: "userDataPath" | "modsPath" | "trayPath" | "downloadsPath",
    title: string,
  ) => void;
  onCreateManualProfile: () => void;
  onConfirmManualProfile: () => void;
}

export function SettingsGameProfileSection({
  profile,
  profiles,
  candidateDetection,
  selectedProfileId,
  selectedValidation,
  validation,
  error,
  candidateError,
  selectionCheckError,
  selectionError,
  selectionMessage,
  manualDraft,
  manualSetupOpen,
  manualError,
  manualMessage,
  isLoading,
  isDetectingCandidates,
  isCheckingSelection,
  isSelecting,
  isCreatingManualProfile,
  isConfirmingManualProfile,
  onSelectedProfileIdChange,
  onSelectProfile,
  onReviewCandidate,
  onToggleManualSetup,
  onManualDraftChange,
  onPickManualFolder,
  onCreateManualProfile,
  onConfirmManualProfile,
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
        <SettingsProfileSelector
          profiles={profiles}
          activeProfileId={null}
          selectedProfileId={selectedProfileId}
          selectedValidation={selectedValidation}
          selectionCheckError={selectionCheckError}
          selectionError={selectionError}
          selectionMessage={selectionMessage}
          isCheckingSelection={isCheckingSelection}
          isSelecting={isSelecting}
          onSelectedProfileIdChange={onSelectedProfileIdChange}
          onSelectProfile={onSelectProfile}
        />
        <SettingsGameCandidateSuggestions
          detection={candidateDetection}
          error={candidateError}
          isLoading={isDetectingCandidates}
          onReviewCandidate={onReviewCandidate}
        />
        <SettingsManualProfileSetup
          profiles={profiles}
          selectedProfileId={selectedProfileId}
          selectedValidation={selectedValidation}
          draft={manualDraft}
          isOpen={manualSetupOpen}
          error={manualError}
          message={manualMessage}
          isCreating={isCreatingManualProfile}
          isConfirming={isConfirmingManualProfile}
          onToggle={onToggleManualSetup}
          onDraftChange={onManualDraftChange}
          onPickFolder={onPickManualFolder}
          onCreate={onCreateManualProfile}
          onConfirm={onConfirmManualProfile}
        />
        <div className="settings-profile-empty">
          <TriangleAlert size={18} strokeWidth={2} />
          <div>
            <strong>No active game profile</strong>
            <p className="workspace-toolbar-copy">
              Choose one of the saved setups above, or add one manually by selecting the
              existing Sims 4 folders yourself.
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
      <SettingsProfileSelector
        profiles={profiles}
        activeProfileId={profile.profileId}
        selectedProfileId={selectedProfileId}
        selectedValidation={selectedValidation}
        selectionCheckError={selectionCheckError}
        selectionError={selectionError}
        selectionMessage={selectionMessage}
        isCheckingSelection={isCheckingSelection}
        isSelecting={isSelecting}
        onSelectedProfileIdChange={onSelectedProfileIdChange}
        onSelectProfile={onSelectProfile}
      />
      <SettingsGameCandidateSuggestions
        detection={candidateDetection}
        error={candidateError}
        isLoading={isDetectingCandidates}
        onReviewCandidate={onReviewCandidate}
      />
      <SettingsManualProfileSetup
        profiles={profiles}
        selectedProfileId={selectedProfileId}
        selectedValidation={selectedValidation}
        draft={manualDraft}
        isOpen={manualSetupOpen}
        error={manualError}
        message={manualMessage}
        isCreating={isCreatingManualProfile}
        isConfirming={isConfirmingManualProfile}
        onToggle={onToggleManualSetup}
        onDraftChange={onManualDraftChange}
        onPickFolder={onPickManualFolder}
        onCreate={onCreateManualProfile}
        onConfirm={onConfirmManualProfile}
      />

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

function SettingsProfileSelector({
  profiles,
  activeProfileId,
  selectedProfileId,
  selectedValidation,
  selectionCheckError,
  selectionError,
  selectionMessage,
  isCheckingSelection,
  isSelecting,
  onSelectedProfileIdChange,
  onSelectProfile,
}: {
  profiles: GameInstallationProfile[];
  activeProfileId: string | null;
  selectedProfileId: string;
  selectedValidation: GameInstallationProfileValidationReport | null;
  selectionCheckError: string | null;
  selectionError: string | null;
  selectionMessage: string | null;
  isCheckingSelection: boolean;
  isSelecting: boolean;
  onSelectedProfileIdChange: (profileId: string) => void;
  onSelectProfile: () => void;
}) {
  const selectedProfile =
    profiles.find((candidate) => candidate.profileId === selectedProfileId) ?? null;
  const selectionIsActive = selectedProfileId === activeProfileId;
  const selectionIsConfirmed = selectedProfile?.confirmationState === "confirmed";
  const environmentMatches =
    selectedValidation?.environmentCompatibility === "matches";
  const selectionBlocker = selectionIsActive
    ? null
    : !selectedProfile
      ? "Choose a saved profile."
      : !selectionIsConfirmed
        ? "This setup must be confirmed before it can become active."
        : selectionCheckError
          ? `Compatibility could not be verified: ${selectionCheckError}`
          : isCheckingSelection
            ? "Checking whether this setup matches the current host."
            : selectedValidation?.environmentCompatibility === "mismatch"
              ? "This setup belongs to a different native operating system. Open SimSuite on that host to activate it."
              : selectedValidation?.environmentCompatibility === "requires_adapter"
                ? "Wine, Proton, or Lutris activation needs an environment adapter that is not available yet."
                : selectedValidation?.environmentCompatibility === "unknown"
                  ? "The operating environment is unknown, so activation is blocked until that environment is identified."
                  : !environmentMatches
                    ? "Compatibility has not been proven, so activation remains blocked."
                    : null;
  const canSelect =
    Boolean(selectedProfile) &&
    !selectionIsActive &&
    selectionIsConfirmed &&
    environmentMatches &&
    !isCheckingSelection &&
    !isSelecting;

  return (
    <section className="settings-profile-selector" aria-labelledby="profile-selector-heading">
      <div className="settings-profile-selector-copy">
        <span className="section-label">Saved setups</span>
        <h3 id="profile-selector-heading">Choose the folders SimSuite should use</h3>
        <p className="workspace-toolbar-copy">
          This updates SimSuite's active folder preference and restarts Downloads
          monitoring. The watcher may refresh inbox or staging data, but this action does
          not run Apply or Restore or move installed Mods or Tray files.
        </p>
      </div>

      {profiles.length > 0 ? (
        <div className="settings-profile-selector-controls">
          <label htmlFor="active-game-profile-select">Saved Sims 4 setup</label>
          <div className="settings-profile-selector-action-row">
            <select
              id="active-game-profile-select"
              value={selectedProfileId}
              disabled={isSelecting}
              onChange={(event) => onSelectedProfileIdChange(event.target.value)}
            >
              {profiles.map((candidate) => (
                <option key={candidate.profileId} value={candidate.profileId}>
                  {candidate.profileName}
                  {candidate.profileId === activeProfileId ? " (active)" : ""}
                </option>
              ))}
            </select>
            <button
              type="button"
              className="primary-action settings-profile-select-button"
              disabled={!canSelect}
              onClick={onSelectProfile}
            >
              {isSelecting ? (
                <>
                  <LoaderCircle size={15} strokeWidth={2} className="spin" />
                  Switching setup
                </>
              ) : selectionIsActive ? (
                "Already active"
              ) : isCheckingSelection ? (
                <>
                  <LoaderCircle size={15} strokeWidth={2} className="spin" />
                  Checking setup
                </>
              ) : selectionBlocker ? (
                "Unavailable on this host"
              ) : (
                "Use selected profile"
              )}
            </button>
          </div>
          {selectedProfile ? (
            <span className="settings-profile-selector-preview">
              {environmentLabel(selectedProfile.operatingEnvironment)} · {selectedProfile.roots.length} configured roots · {confirmationLabel(selectedProfile.confirmationState)}
            </span>
          ) : null}
          {selectionBlocker ? (
            <span className="settings-profile-selector-eligibility" role="status">
              <TriangleAlert size={14} strokeWidth={2} />
              {selectionBlocker}
            </span>
          ) : null}
        </div>
      ) : (
        <div className="settings-profile-selector-empty">
          No saved game setups are available yet. SimSuite will not invent one.
        </div>
      )}

      {selectionMessage ? (
        <div className="settings-profile-selection-message is-success" role="status">
          <CircleCheck size={16} strokeWidth={2} />
          <span>{selectionMessage}</span>
        </div>
      ) : null}
      {selectionError ? (
        <div className="settings-profile-selection-message is-error" role="alert">
          <TriangleAlert size={16} strokeWidth={2} />
          <span>{selectionError}</span>
        </div>
      ) : null}
    </section>
  );
}

function SettingsGameCandidateSuggestions({
  detection,
  error,
  isLoading,
  onReviewCandidate,
}: {
  detection: GameInstallationCandidateDetectionResult | null;
  error: string | null;
  isLoading: boolean;
  onReviewCandidate: (candidateId: string) => void;
}) {
  const candidates = detection?.candidates ?? [];
  const supported = detection?.supported ?? null;
  const currentEnvironment = detection?.currentEnvironment ?? null;
  const inspectionNotes = supported ? detection?.reviewNotes ?? [] : [];
  const supportedHostLabel =
    currentEnvironment === "native_windows"
      ? "this Windows PC"
      : currentEnvironment === "native_macos"
        ? "this Mac"
        : "this host";
  const heading =
    supported === false
      ? "Automatic suggestions are not available on this host"
      : `Existing setups found on ${supportedHostLabel}`;
  const supportedDescription =
    currentEnvironment === "native_windows"
      ? "SimSuite checks Windows' configured Documents location, explicit OneDrive roots, and the normal home Documents fallback. It ranks only Sims 4 folders that already exist. A suggestion is temporary until you review and save it manually."
      : "SimSuite checks standard macOS Documents locations and ranks only folders that already exist. A suggestion is temporary until you review and save it manually.";
  const emptyHeading =
    currentEnvironment === "native_windows"
      ? "No standard Windows setup was found"
      : "No standard macOS setup was found";

  return (
    <section
      className="settings-game-candidates"
      aria-labelledby="game-candidates-heading"
    >
      <div className="settings-game-candidates-heading">
        <div>
          <span className="section-label">
            <FolderSearch size={14} strokeWidth={2} />
            Read-only suggestions
          </span>
          <h3 id="game-candidates-heading">{heading}</h3>
          <p className="workspace-toolbar-copy">
            {supported === false
              ? `SimSuite does not guess standard Sims 4 paths for ${
                  currentEnvironment
                    ? environmentLabel(currentEnvironment)
                    : "this operating system"
                } yet. Use the guarded manual folder chooser below.`
              : supportedDescription}
          </p>
        </div>
        <span className="ghost-chip">
          {supported === false ? "Manual only" : `${candidates.length} found`}
        </span>
      </div>

      {isLoading ? (
        <div className="settings-game-candidate-state" role="status">
          <LoaderCircle size={16} strokeWidth={2} className="spin" />
          <span>Checking supported folder locations without changing them.</span>
        </div>
      ) : error ? (
        <div className="settings-game-candidate-state is-error" role="alert">
          <TriangleAlert size={16} strokeWidth={2} />
          <div>
            <strong>Suggestions could not be checked</strong>
            <span>{error}</span>
          </div>
        </div>
      ) : supported === false ? (
        <div className="settings-game-candidate-state">
          <FolderSearch size={16} strokeWidth={2} />
          <div>
            <strong>Manual setup remains available</strong>
            <span>
              {detection?.reviewNotes[0] ??
                "Automatic folder suggestions are not available on this host yet."}
            </span>
          </div>
        </div>
      ) : candidates.length === 0 && inspectionNotes.length > 0 ? (
        <div className="settings-game-candidate-state is-error" role="status">
          <TriangleAlert size={16} strokeWidth={2} />
          <div>
            <strong>Some standard locations could not be inspected</strong>
            <span>{inspectionNotes[0]}</span>
          </div>
        </div>
      ) : candidates.length === 0 ? (
        <div className="settings-game-candidate-state">
          <FolderSearch size={16} strokeWidth={2} />
          <div>
            <strong>{emptyHeading}</strong>
            <span>Use the manual folder chooser below. SimSuite will not invent paths.</span>
          </div>
        </div>
      ) : (
        <div className="settings-game-candidate-list" role="list">
          {candidates.map((candidate) => (
            <article
              className="settings-game-candidate-row"
              role="listitem"
              key={candidate.candidateId}
            >
              <div className="settings-game-candidate-rank" aria-label={`Rank ${candidate.rank}`}>
                <span>#{candidate.rank}</span>
                <strong>{candidateConfidenceLabel(candidate.confidence)}</strong>
              </div>
              <div className="settings-game-candidate-body">
                <div className="settings-game-candidate-title">
                  <div>
                    <strong>{candidate.suggestedName}</strong>
                    <span>{environmentLabel(candidate.operatingEnvironment)}</span>
                  </div>
                  <span className="ghost-chip">Read only</span>
                </div>
                <div className="settings-game-candidate-roots" role="list">
                  {candidate.suggestedRoots.map((root) => (
                    <div role="listitem" key={root.rootId}>
                      <span>{rootLabel(root.rootId)}</span>
                      <strong title={root.configuredPath}>{root.configuredPath}</strong>
                      <em className={root.exists ? "is-ready" : "is-review"}>
                        {root.exists ? "Found" : "Not found"}
                      </em>
                    </div>
                  ))}
                </div>
                <div className="settings-game-candidate-evidence">
                  {candidate.detectionEvidence.map((item) => (
                    <span key={item}>
                      <CircleCheck size={13} strokeWidth={2} />
                      {item}
                    </span>
                  ))}
                  {candidate.warnings.map((warning) => (
                    <span className="is-warning" key={warning}>
                      <TriangleAlert size={13} strokeWidth={2} />
                      {warning}
                    </span>
                  ))}
                </div>
              </div>
              <div className="settings-game-candidate-action">
                <button
                  type="button"
                  className="secondary-action"
                  onClick={() => onReviewCandidate(candidate.candidateId)}
                >
                  Review this suggestion
                </button>
                <span>Copies paths into the unsaved manual form only.</span>
              </div>
            </article>
          ))}
        </div>
      )}

      {!isLoading && !error && candidates.length > 0 && inspectionNotes.length > 0 ? (
        <div className="settings-game-candidate-state" role="status">
          <TriangleAlert size={16} strokeWidth={2} />
          <div>
            <strong>Some other locations could not be inspected</strong>
            <span>{inspectionNotes[0]}</span>
          </div>
        </div>
      ) : null}
    </section>
  );
}

function SettingsManualProfileSetup({
  profiles,
  selectedProfileId,
  selectedValidation,
  draft,
  isOpen,
  error,
  message,
  isCreating,
  isConfirming,
  onToggle,
  onDraftChange,
  onPickFolder,
  onCreate,
  onConfirm,
}: {
  profiles: GameInstallationProfile[];
  selectedProfileId: string;
  selectedValidation: GameInstallationProfileValidationReport | null;
  draft: CreateManualGameInstallationProfileRequest;
  isOpen: boolean;
  error: string | null;
  message: string | null;
  isCreating: boolean;
  isConfirming: boolean;
  onToggle: () => void;
  onDraftChange: (
    values: Partial<CreateManualGameInstallationProfileRequest>,
  ) => void;
  onPickFolder: (
    field: "userDataPath" | "modsPath" | "trayPath" | "downloadsPath",
    title: string,
  ) => void;
  onCreate: () => void;
  onConfirm: () => void;
}) {
  const selectedProfile =
    profiles.find((profile) => profile.profileId === selectedProfileId) ?? null;
  const isManualDraft =
    selectedProfile?.detectionMethod === "manual" &&
    selectedProfile.confirmationState !== "confirmed";
  const canConfirm =
    isManualDraft &&
    selectedValidation?.environmentCompatibility === "matches" &&
    selectedValidation.state === "valid" &&
    selectedValidation.blockers.length === 0 &&
    !isConfirming;

  return (
    <section className="settings-manual-profile" aria-labelledby="manual-profile-heading">
      <div className="settings-manual-profile-heading">
        <div>
          <span className="section-label">
            <FolderPlus size={14} strokeWidth={2} />
            Manual setup
          </span>
          <h3 id="manual-profile-heading">Add another Sims 4 setup</h3>
          <p className="workspace-toolbar-copy">
            Choose existing folders yourself. SimSuite saves an unconfirmed draft first,
            then requires a separate read-only validation and confirmation step.
          </p>
        </div>
        <button
          type="button"
          className="secondary-action"
          aria-expanded={isOpen}
          onClick={onToggle}
        >
          {isOpen ? "Close setup" : "Choose folders manually"}
        </button>
      </div>

      {isOpen ? (
        <div className="settings-manual-profile-form">
          <label className="settings-manual-profile-name">
            <span>Profile name</span>
            <input
              type="text"
              maxLength={80}
              value={draft.profileName}
              placeholder="For example, Main Sims 4 setup"
              disabled={isCreating}
              onChange={(event) =>
                onDraftChange({ profileName: event.target.value })
              }
            />
          </label>

          <ManualFolderField
            inputId="manual-profile-user-data-path"
            label="Sims 4 user-data folder"
            hint="Optional. Usually the folder that directly contains Mods and Tray."
            value={draft.userDataPath ?? ""}
            required={false}
            disabled={isCreating}
            onChange={(value) => onDraftChange({ userDataPath: value || null })}
            onPick={() =>
              onPickFolder(
                "userDataPath",
                "Choose the Sims 4 user-data folder",
              )
            }
          />
          <ManualFolderField
            inputId="manual-profile-mods-path"
            label="Mods folder"
            hint="Required. Select the existing folder where installed .package and .ts4script files live."
            value={draft.modsPath}
            required
            disabled={isCreating}
            onChange={(value) => onDraftChange({ modsPath: value })}
            onPick={() => onPickFolder("modsPath", "Choose the Sims 4 Mods folder")}
          />
          <ManualFolderField
            inputId="manual-profile-tray-path"
            label="Tray folder"
            hint="Required. Select the existing folder where Sims, households, rooms, and lots are stored."
            value={draft.trayPath}
            required
            disabled={isCreating}
            onChange={(value) => onDraftChange({ trayPath: value })}
            onPick={() => onPickFolder("trayPath", "Choose the Sims 4 Tray folder")}
          />
          <ManualFolderField
            inputId="manual-profile-downloads-path"
            label="Downloads folder"
            hint="Optional. SimSuite can watch this folder after the profile becomes active."
            value={draft.downloadsPath ?? ""}
            required={false}
            disabled={isCreating}
            onChange={(value) => onDraftChange({ downloadsPath: value || null })}
            onPick={() =>
              onPickFolder("downloadsPath", "Choose a Downloads intake folder")
            }
          />

          <div className="settings-manual-profile-actions">
            <button
              type="button"
              className="primary-action"
              disabled={
                isCreating ||
                !draft.profileName.trim() ||
                !draft.modsPath.trim() ||
                !draft.trayPath.trim()
              }
              onClick={onCreate}
            >
              {isCreating ? (
                <>
                  <LoaderCircle size={15} strokeWidth={2} className="spin" />
                  Saving draft
                </>
              ) : (
                "Save unconfirmed draft"
              )}
            </button>
            <span>No folder is created, moved, renamed, or changed.</span>
          </div>
        </div>
      ) : null}

      {isManualDraft ? (
        <div className="settings-manual-profile-review">
          <div className="settings-manual-profile-confirmation">
            <div>
              <span className="section-label">Selected draft evidence</span>
              <strong>Confirm “{selectedProfile.profileName}”</strong>
              <p>
                {selectedValidation?.state === "valid"
                  ? "The current read-only evidence is valid. Confirmation will rerun that check before saving the confirmed state."
                  : "This evidence belongs to the selected draft, not the active profile. Confirmation stays blocked until every required root is valid."}
              </p>
            </div>
            <button
              type="button"
              className="secondary-action"
              disabled={!canConfirm}
              onClick={onConfirm}
            >
              {isConfirming ? (
                <>
                  <LoaderCircle size={15} strokeWidth={2} className="spin" />
                  Rechecking and confirming
                </>
              ) : (
                "Confirm validated profile"
              )}
            </button>
          </div>

          <div className="settings-manual-profile-evidence-summary">
            <div>
              <span>Live status</span>
              <strong>
                {selectedValidation
                  ? profileStatusLabel(selectedValidation.state)
                  : "Checking evidence"}
              </strong>
            </div>
            <div>
              <span>Host compatibility</span>
              <strong>
                {selectedValidation
                  ? environmentCompatibilityLabel(
                      selectedValidation.environmentCompatibility,
                    )
                  : "Not proven"}
              </strong>
            </div>
            <div>
              <span>Validation mode</span>
              <strong>{selectedValidation?.readOnly ? "Read only" : "Pending"}</strong>
            </div>
          </div>

          <div className="settings-manual-profile-root-review" role="list">
            {selectedProfile.roots.map((storedRoot) => {
              const liveRoot = selectedValidation?.roots.find(
                (root) => root.rootId === storedRoot.rootId,
              );
              return (
                <div
                  className="settings-manual-profile-root-review-row"
                  role="listitem"
                  key={storedRoot.rootId}
                >
                  <div>
                    <span>{rootLabel(storedRoot.rootId)}</span>
                    <strong title={storedRoot.configuredPath}>
                      {storedRoot.configuredPath}
                    </strong>
                  </div>
                  <span>{storedRoot.required ? "Required" : "Optional"}</span>
                  <strong
                    className={
                      liveRoot
                        ? profileStatusTone(rootStateAsProfileStatus(liveRoot.state))
                        : "is-review"
                    }
                  >
                    {liveRoot ? rootStateLabel(liveRoot.state) : "Checking"}
                  </strong>
                </div>
              );
            })}
          </div>

          {selectedValidation?.blockers.length ? (
            <div className="settings-manual-profile-blockers" role="alert">
              <TriangleAlert size={16} strokeWidth={2} />
              <div>
                <strong>Why confirmation is blocked</strong>
                <ul>
                  {selectedValidation.blockers.map((blocker) => (
                    <li key={blocker}>{blocker}</li>
                  ))}
                </ul>
              </div>
            </div>
          ) : null}
        </div>
      ) : null}

      {message ? (
        <div className="settings-profile-selection-message is-success" role="status">
          <CircleCheck size={16} strokeWidth={2} />
          <span>{message}</span>
        </div>
      ) : null}
      {error ? (
        <div className="settings-profile-selection-message is-error" role="alert">
          <TriangleAlert size={16} strokeWidth={2} />
          <span>{error}</span>
        </div>
      ) : null}
    </section>
  );
}

function ManualFolderField({
  inputId,
  label,
  hint,
  value,
  required,
  disabled,
  onChange,
  onPick,
}: {
  inputId: string;
  label: string;
  hint: string;
  value: string;
  required: boolean;
  disabled: boolean;
  onChange: (value: string) => void;
  onPick: () => void;
}) {
  const hintId = `${inputId}-hint`;
  return (
    <div className="settings-manual-folder-field">
      <label htmlFor={inputId}>
        {label} {required ? <em>Required</em> : <em>Optional</em>}
      </label>
      <small id={hintId}>{hint}</small>
      <div>
        <input
          id={inputId}
          type="text"
          value={value}
          disabled={disabled}
          aria-describedby={hintId}
          placeholder={`Choose ${label.toLowerCase()}`}
          onChange={(event) => onChange(event.target.value)}
        />
        <button type="button" className="secondary-action" disabled={disabled} onClick={onPick}>
          Browse
        </button>
      </div>
    </div>
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
        Choose a saved setup, review its roots and evidence, or add another setup by
        selecting existing folders manually. Editing confirmed profiles remains unavailable.
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
        <strong>Guarded setup boundary</strong>
        <span>
          Validation evidence remains read-only. Selecting a saved profile updates the
          active preference and restarts Downloads monitoring; it does not run Apply or
          Restore or move installed Mods or Tray files. Suggestions remain temporary,
          and confirmed-profile editing stays unavailable.
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

function candidateConfidenceLabel(
  confidence: GameInstallationCandidate["confidence"],
) {
  return confidence === "strong" ? "Strong match" : "Possible match";
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
