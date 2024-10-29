/// The default port where `server_pb` should expect to find the game server
pub const GAME_SERVER_PORT: u16 = 3002;
/// The default port where `gui_pb` should expect to connect to `server_pb`
pub const GUI_LISTENER_PORT: u16 = 20010;
/// The default port where `server_pb` should expect to find the simulation controls
pub const SIMULATION_LISTENER_PORT: u16 = 20014;

/// this message lets game server clients know that a game server supports
/// extra messages like pause, reset, custom game state
pub const GAME_SERVER_MAGIC_NUMBER: [u8; 4] = [170, 115, 26, 153];

/// The maximum number of nodes in the target path sent from the server to the robot
pub const MAX_ROBOT_PATH_LENGTH: usize = 10;
/// The size of the OLED display on the robot
pub const ROBOT_DISPLAY_WIDTH: usize = 128;
/// The size of the OLED display on the robot
pub const ROBOT_DISPLAY_HEIGHT: usize = 64;

pub const ROBOT_LOGS_BUFFER: usize = 4096;

/// Millimeters per inch
pub const MM_PER_INCH: f32 = 25.4;
/// Inches per grid unit
pub const INCHES_PER_GU: f32 = 3.5;

/// Inches per meter
pub const INCHES_PER_M: f32 = 1000.0 / MM_PER_INCH;
/// Grid units per inch
pub const GU_PER_INCH: f32 = 1.0 / INCHES_PER_GU;
/// Millimeters per grid unit
pub const MM_PER_GU: f32 = MM_PER_INCH * INCHES_PER_GU;
/// Grid units per meter
pub const GU_PER_M: f32 = GU_PER_INCH * INCHES_PER_M;
