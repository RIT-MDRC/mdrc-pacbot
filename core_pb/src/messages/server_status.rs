use crate::messages::NetworkStatus;
use crate::names::{RobotName, NUM_ROBOT_NAMES};
use pacbot_rs::game_state::GameState;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ServerStatus {
    pub simulation_connection: NetworkStatus,

    pub game_state: GameState,
    pub game_server_connection: NetworkStatus,
    pub advanced_game_server: bool,

    pub gui_clients: usize,
    pub robots: [RobotStatus; NUM_ROBOT_NAMES],
}

impl Default for ServerStatus {
    fn default() -> Self {
        Self {
            simulation_connection: NetworkStatus::default(),

            game_state: GameState::default(),
            game_server_connection: NetworkStatus::default(),
            advanced_game_server: false,

            gui_clients: 0,
            robots: RobotName::get_all().map(|name| RobotStatus::new(name)),
        }
    }
}

/// Indicates the last completed action
#[derive(Copy, Clone, Debug, Default, Serialize, Deserialize, PartialOrd, PartialEq)]
#[repr(usize)]
pub enum OverTheAirStep {
    #[default]
    GuiRequest = 0,
    RobotReadyConfirmation = 1,
    FetchBinary = 2,
    DataTransfer {
        received: usize,
        total: usize,
    } = 3,
    HashConfirmation = 4,
    GuiConfirmation = 5,
    MarkUpdateReady = 6,
    Reboot = 7,
    CheckFirmwareSwapped = 8,
    FinalGuiConfirmation = 9,
    MarkUpdateBooted = 10,
    Finished = 11,
    Failed = 12,
}

impl From<OverTheAirStep> for usize {
    fn from(value: OverTheAirStep) -> Self {
        match value {
            OverTheAirStep::GuiRequest => 0,
            OverTheAirStep::RobotReadyConfirmation => 1,
            OverTheAirStep::FetchBinary => 2,
            OverTheAirStep::DataTransfer { .. } => 3,
            OverTheAirStep::HashConfirmation => 4,
            OverTheAirStep::GuiConfirmation => 5,
            OverTheAirStep::MarkUpdateReady => 6,
            OverTheAirStep::Reboot => 7,
            OverTheAirStep::CheckFirmwareSwapped => 8,
            OverTheAirStep::FinalGuiConfirmation => 9,
            OverTheAirStep::MarkUpdateBooted => 10,
            OverTheAirStep::Finished => 11,
            OverTheAirStep::Failed => 12,
        }
    }
}

impl From<usize> for OverTheAirStep {
    fn from(value: usize) -> Self {
        match value {
            0 => OverTheAirStep::GuiRequest,
            1 => OverTheAirStep::RobotReadyConfirmation,
            2 => OverTheAirStep::FetchBinary,
            3 => OverTheAirStep::DataTransfer {
                received: 0,
                total: 0,
            },
            4 => OverTheAirStep::HashConfirmation,
            5 => OverTheAirStep::GuiConfirmation,
            6 => OverTheAirStep::MarkUpdateReady,
            7 => OverTheAirStep::Reboot,
            8 => OverTheAirStep::CheckFirmwareSwapped,
            9 => OverTheAirStep::FinalGuiConfirmation,
            10 => OverTheAirStep::MarkUpdateBooted,
            11 => OverTheAirStep::Finished,
            12 => OverTheAirStep::Failed,
            _ => OverTheAirStep::default(),
        }
    }
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize, PartialOrd, PartialEq)]
pub struct OverTheAirStepCompletion {
    pub step: OverTheAirStep,
    pub since_beginning: Duration,
    pub success: Option<bool>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialOrd, PartialEq)]
pub struct RobotStatus {
    pub name: RobotName,
    pub connection: NetworkStatus,

    pub ota: Vec<OverTheAirStepCompletion>,
}

impl RobotStatus {
    pub fn new(name: RobotName) -> Self {
        Self {
            name,
            connection: NetworkStatus::default(),

            ota: vec![],
        }
    }
}
