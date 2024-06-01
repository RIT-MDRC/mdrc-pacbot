use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Default, Serialize, Deserialize)]
pub struct ServerStatus {
    pub clients: usize,
}
