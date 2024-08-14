use crate::messages::ota::{OverTheAirStep, OverTheAirStepCompletion};
use crate::messages::{MotorControlStatus, NetworkStatus};
use crate::names::{RobotName, NUM_ROBOT_NAMES};
use crate::util::ColoredStatus;
use nalgebra::Point2;
use pacbot_rs::game_state::GameState;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ServerStatus {
    pub utilization: ColoredStatus,

    pub simulation_connection: NetworkStatus,

    pub game_state: GameState,
    pub game_server_connection: NetworkStatus,
    pub advanced_game_server: bool,

    pub target_path: Vec<Point2<i8>>,

    pub gui_clients: usize,
    pub robots: [RobotStatus; NUM_ROBOT_NAMES],
}

impl Default for ServerStatus {
    fn default() -> Self {
        Self {
            utilization: ColoredStatus::Ok(Some("Loading...".to_string())),

            simulation_connection: NetworkStatus::default(),

            game_state: GameState::default(),
            game_server_connection: NetworkStatus::default(),
            advanced_game_server: false,

            target_path: vec![],

            gui_clients: 0,
            robots: RobotName::get_all().map(RobotStatus::new),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RobotStatus {
    pub name: RobotName,
    pub connection: NetworkStatus,

    pub ota_current: OverTheAirStep,
    pub ota_completed: Vec<OverTheAirStepCompletion>,

    pub last_motor_status: (Duration, MotorControlStatus),

    pub sim_position: Option<Point2<f32>>,
}

impl RobotStatus {
    pub fn new(name: RobotName) -> Self {
        Self {
            name,
            connection: NetworkStatus::default(),

            ota_current: OverTheAirStep::GuiRequest,
            ota_completed: vec![],

            last_motor_status: Default::default(),

            sim_position: None,
        }
    }
}
