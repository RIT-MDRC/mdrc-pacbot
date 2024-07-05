use crate::messages::settings::PacbotSettings;
use nalgebra::{Rotation2, Vector2};
// use pacbot_rs::game_state::GameState;
use serde::{Deserialize, Serialize};

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct ServerStatus {
    // pub game_state: GameState,
    pub game_server_connected: bool,

    pub gui_clients: usize,
    pub robots: Vec<RobotStatus>,

    pub wasd_qe_input: Vec<(Vector2<f32>, Rotation2<f32>)>,
    pub settings: PacbotSettings,
}

#[derive(Clone, Default, Serialize, Deserialize, PartialOrd, PartialEq)]
pub struct RobotStatus {
    pub connected: bool,
}
