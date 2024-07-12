#[cfg(feature = "std")]
use crate::messages::settings::PacbotSettings;
use pacbot_rs::game_state::GameState;
use serde::{Deserialize, Serialize};

#[cfg(feature = "std")]
pub mod server_status;
#[cfg(feature = "std")]
pub mod settings;

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
#[cfg(feature = "std")]
pub enum GuiToGameServerMessage {
    Settings(PacbotSettings),
}

pub const GAME_SERVER_MAGIC_NUMBER: [u8; 4] = [170, 115, 26, 153];

#[derive(Clone, Debug, Serialize, Deserialize)]
#[allow(clippy::large_enum_variant)]
pub enum GameServerCommand {
    Pause,
    Unpause,
    Reset,
    SetState(GameState),
}
