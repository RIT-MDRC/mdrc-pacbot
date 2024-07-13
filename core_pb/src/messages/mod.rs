use crate::messages::server_status::ServerStatus;
#[cfg(feature = "std")]
use crate::messages::settings::PacbotSettings;
use crate::names::RobotName;
use nalgebra::Vector2;
use pacbot_rs::game_state::GameState;
use serde::{Deserialize, Serialize};

#[cfg(feature = "std")]
pub mod server_status;
#[cfg(feature = "std")]
pub mod settings;

pub const GAME_SERVER_MAGIC_NUMBER: [u8; 4] = [170, 115, 26, 153];

#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg(feature = "std")]
pub enum GuiToServerMessage {
    Settings(PacbotSettings),
    GameServerCommand(GameServerCommand),
    RobotVelocity(RobotName, Option<(Vector2<f32>, f32)>),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg(feature = "std")]
pub enum ServerToGuiMessage {
    Status(ServerStatus),
    Settings(PacbotSettings),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg(feature = "std")]
pub enum SimulationToServerMessage {
    None,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg(feature = "std")]
pub enum ServerToSimulationMessage {
    RobotVelocity(RobotName, Option<(Vector2<f32>, f32)>),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg(feature = "std")]
pub enum ServerToRobotMessage {}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg(feature = "std")]
pub enum RobotToServerMessage {
    Name(RobotName),
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize, Default, PartialOrd, PartialEq)]
pub enum NetworkStatus {
    /// Settings dictate that a connection should not be made
    #[default]
    NotConnected,
    /// A connection could not be established
    ConnectionFailed,
    /// After a connection is established, but before a message is received
    Connecting,
    /// After a message is received
    Connected,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[allow(clippy::large_enum_variant)]
pub enum GameServerCommand {
    Pause,
    Unpause,
    Reset,
    SetState(GameState),
}

impl GameServerCommand {
    pub fn text(&self) -> Option<&'static str> {
        match self {
            GameServerCommand::Pause => Some("p"),
            GameServerCommand::Unpause => Some("P"),
            GameServerCommand::Reset => Some("r"),
            _ => None,
        }
    }
}
