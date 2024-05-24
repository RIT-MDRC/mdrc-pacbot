use pacbot_rs::game_state::GameState;
use serde::{Deserialize, Serialize};

pub const GAME_SERVER_MAGIC_NUMBER: [u8; 4] = [170, 115, 26, 153];

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum GameServerCommand {
    Pause,
    Unpause,
    Reset,
    SetState(GameState),
}
