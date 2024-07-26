pub const GAME_SERVER_PORT: u16 = 3002;
pub const GUI_LISTENER_PORT: u16 = 20010;
pub const SIMULATION_LISTENER_PORT: u16 = 20014;

pub const MM_PER_INCH: f32 = 25.4;
pub const INCHES_PER_GU: f32 = 3.5;

pub const INCHES_PER_M: f32 = 1000.0 / MM_PER_INCH;
pub const GU_PER_INCH: f32 = 1.0 / INCHES_PER_GU;
pub const MM_PER_GU: f32 = MM_PER_INCH * INCHES_PER_GU;
pub const GU_PER_M: f32 = GU_PER_INCH * INCHES_PER_M;
