use std::path::PathBuf;

use crate::models::{
    GameInstallationAdapterReadinessReport, GameInstallationProfile,
    GameInstallationRootValidationState,
};

use super::sims4_adapter::Sims4Adapter;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameAdapterRootEvidence {
    pub root_id: String,
    pub required: bool,
    pub state: GameInstallationRootValidationState,
    pub canonical_path: Option<PathBuf>,
    pub symlink_observed: Option<bool>,
}

pub struct GameAdapterProfileEvidence<'a> {
    pub profile: &'a GameInstallationProfile,
    pub roots: &'a [GameAdapterRootEvidence],
}

pub trait GameAdapter {
    fn adapter_id(&self) -> &'static str;
    fn game_id(&self) -> &'static str;
    fn supported_mod_extensions(&self) -> &'static [&'static str];
    fn supported_tray_extensions(&self) -> &'static [&'static str];
    fn max_script_folder_depth(&self) -> usize;
    fn max_package_folder_depth(&self) -> usize;
    fn validate_readiness(
        &self,
        evidence: GameAdapterProfileEvidence<'_>,
    ) -> GameInstallationAdapterReadinessReport;
}

pub fn validate_game_readiness(
    evidence: GameAdapterProfileEvidence<'_>,
) -> Option<GameInstallationAdapterReadinessReport> {
    let adapter = Sims4Adapter;
    if evidence.profile.game_id == adapter.game_id() {
        Some(adapter.validate_readiness(evidence))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{
        GameInstallationConfirmationState, GameInstallationDetectionMethod,
        GameInstallationProfileStatus, GameOperatingEnvironment,
    };

    #[test]
    fn unknown_games_do_not_receive_a_guessed_adapter() {
        let profile = GameInstallationProfile {
            profile_id: "unknown-game".to_owned(),
            profile_name: "Unknown game".to_owned(),
            game_id: "unknown".to_owned(),
            operating_environment: GameOperatingEnvironment::NativeLinux,
            status: GameInstallationProfileStatus::NeedsReview,
            detection_method: GameInstallationDetectionMethod::Manual,
            detection_evidence_json: "{}".to_owned(),
            confirmation_state: GameInstallationConfirmationState::Confirmed,
            confirmed_at: None,
            last_validated_at: None,
            created_at: "2026-08-04T00:00:00Z".to_owned(),
            updated_at: "2026-08-04T00:00:00Z".to_owned(),
            roots: Vec::new(),
        };

        assert!(validate_game_readiness(GameAdapterProfileEvidence {
            profile: &profile,
            roots: &[],
        })
        .is_none());
    }
}
