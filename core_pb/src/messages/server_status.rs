use crate::messages::ota::{OverTheAirStep, OverTheAirStepCompletion};
use crate::messages::NetworkStatus;
use crate::names::{RobotName, NUM_ROBOT_NAMES};
use pacbot_rs::game_state::GameState;
use serde::{Deserialize, Serialize};

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

#[derive(Clone, Debug, Serialize, Deserialize, PartialOrd, PartialEq)]
pub struct RobotStatus {
    pub name: RobotName,
    pub connection: NetworkStatus,

    pub ota_current: OverTheAirStep,
    pub ota_completed: Vec<OverTheAirStepCompletion>,
}

impl RobotStatus {
    pub fn new(name: RobotName) -> Self {
        Self {
            name,
            connection: NetworkStatus::default(),

            ota_current: OverTheAirStep::GuiRequest,
            ota_completed: vec![],
        }
    }
}
