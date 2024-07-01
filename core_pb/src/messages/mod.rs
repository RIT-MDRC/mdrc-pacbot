#[cfg(std)]
use crate::messages::settings::PacbotSettings;
use pacbot_rs::game_state::GameState;
use serde::{Deserialize, Serialize};

#[cfg(std)]
pub mod server_status;
#[cfg(std)]
pub mod settings;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg(std)]
pub enum GuiToGameServerMessage {
    Settings(PacbotSettings),
}

pub const GAME_SERVER_MAGIC_NUMBER: [u8; 4] = [170, 115, 26, 153];

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum GameServerCommand {
    Connect(Option<([u8; 4], u16)>),
    Pause,
    Unpause,
    Reset,
    SetState(GameState),
}
