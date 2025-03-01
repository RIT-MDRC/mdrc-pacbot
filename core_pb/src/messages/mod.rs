use crate::constants::{
    INITIAL_ENABLE_BATTERY_MONITOR, INITIAL_ENABLE_DISPLAY, INITIAL_ENABLE_DISTS,
    INITIAL_ENABLE_EXTRA_IMU_DATA, INITIAL_ENABLE_GAMEPAD, INITIAL_ENABLE_IMU,
    MAX_ROBOT_PATH_LENGTH,
};
use crate::grid::standard_grid::StandardGrid;
#[cfg(feature = "std")]
use crate::messages::server_status::ServerStatus;
#[cfg(feature = "std")]
use crate::messages::settings::PacbotSettings;
use crate::names::RobotName;
#[cfg(feature = "std")]
use crate::names::NUM_ROBOT_NAMES;
use crate::robot_definition::RobotDefinition;
#[cfg(feature = "std")]
use crate::util::ColoredStatus;
use core::time::Duration;
use nalgebra::Point2;
#[cfg(feature = "std")]
use nalgebra::Rotation2;
use nalgebra::Vector2;
use pacbot_rs::game_state::GameState;
use pacbot_rs::location::Direction;
use portable_atomic::{AtomicBool, AtomicF32, AtomicI32, AtomicI8};
use serde::{Deserialize, Serialize};

#[cfg(feature = "std")]
pub mod ota;
pub mod robot_tcp;
#[cfg(feature = "std")]
pub mod server_status;
#[cfg(feature = "std")]
pub mod settings;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg(feature = "std")]
#[allow(clippy::large_enum_variant)]
/// Messages sent from `gui_pb` to `server_pb`
pub enum GuiToServerMessage {
    /// Update server settings
    Settings(PacbotSettings),
    /// Send a message to the game server
    GameServerCommand(GameServerCommand),
    /// Send a message to the simulation
    SimulationCommand(ServerToSimulationMessage),
    /// Set a robot's target velocity (for WASD movement)
    RobotVelocity(RobotName, VelocityControl),
    /// Send a message to a robot
    RobotCommand(RobotName, ServerToRobotMessage),
    /// Initiate an Over the Air Programming update for a robot
    StartOtaFirmwareUpdate(RobotName),
    /// Cancel an Over the Air Programming update for a robot
    CancelOtaFirmwareUpdate(RobotName),
    /// Continue an Over the Air Programming update for a robot
    ConfirmFirmwareUpdate(RobotName),
    /// Clear Over the Air Programming update history for a robot
    ClearFirmwareUpdateHistory(RobotName),
    /// Set a robot's target location
    TargetLocation(Point2<i8>),
    /// Restart simulation (including rebuild)
    RestartSimulation,
}

#[derive(Copy, Clone, Debug, Default, PartialOrd, PartialEq, Serialize, Deserialize)]
pub enum VelocityControl {
    #[default]
    None,
    Stop,
    LinVelAngVel(Vector2<f32>, f32),
    LinVelFixedAng(Vector2<f32>, f32),
    LinVelFaceForward(Vector2<f32>),
    AssistedDriving(Vector2<f32>),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg(feature = "std")]
#[allow(clippy::large_enum_variant)]
/// Messages sent from `server_pb` to `gui_pb`
pub enum ServerToGuiMessage {
    /// Very frequent; includes all information about the status of the server and robots
    Status(ServerStatus),
    /// Less frequent; includes updated server settings
    Settings(PacbotSettings),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg(feature = "std")]
/// Messages sent from `sim_pb` to `server_pb`
pub enum SimulationToServerMessage {
    /// The positions of the simulated robots, to be shown in the gui
    RobotPositions([Option<(Point2<f32>, Rotation2<f32>)>; NUM_ROBOT_NAMES]),
    /// The display of a simulated robot
    RobotDisplay(RobotName, Vec<u128>),
}

#[derive(Copy, Clone, Debug, PartialOrd, PartialEq, Serialize, Deserialize)]
pub enum RobotButton {
    EastA,
    SouthB,
    NorthX,
    WestY,
    LeftStart,
    RightSelect,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg(feature = "std")]
pub enum ServerToSimulationMessage {
    Spawn(RobotName),
    Teleport(RobotName, Point2<i8>),
    Delete(RobotName),
    SetPacman(RobotName),
    SetStandardGrid(StandardGrid),
    /// A button press (true) or release (false) for a simulated robot
    RobotButton(RobotName, (RobotButton, bool)),
    RobotJoystick(RobotName, (f32, f32)),
}

/// This is sent regularly and frequently to robots via [`ServerToRobotMessage::FrequentRobotItems`]
///
/// Holds information that may change often, or where low latency is critical. Its contents should be passed
/// along as quickly as possible.
#[derive(Clone, Debug, PartialOrd, PartialEq, Serialize, Deserialize)]
pub struct FrequentServerToRobot {
    /// Which grid is currently in use
    pub grid: StandardGrid,
    /// Overall requested velocity of the robot, ex. using WASD or controller manual input
    pub target_velocity: VelocityControl,
    /// Requested velocity for each individual motor, forwards (+) or backwards (-), for testing
    pub motors_override: [Option<f32>; 3],
    /// Requested output for each PWM pin, for testing
    pub pwm_override: [[Option<u16>; 2]; 3],
    /// Which pwm pin corresponds to which motor
    ///
    /// Example: for the config `[[0, 1], [5, 4], [2, 3]]`:
    /// - Raising the first physical pin (denoted `0`) causes motor 0 to turn clockwise
    /// - Raising pin `1` causes motor 0 to turn counter-clockwise
    /// - Raising pin `5` causes motor 1 to turn clockwise
    /// - `4` -> motor 1 counter-clockwise
    /// - `2` -> motor 2 clockwise
    /// - `3` -> motor 2 counter-clockwise
    pub motor_config: [[usize; 2]; 3],
    /// Which encoder belongs to which motor, and whether the encoder is reversed
    pub encoder_config: [(usize, bool); 3],
    /// The order of the distance sensors
    pub dist_sensor_config: [usize; 4],
    /// Basic parameters for the PID controller
    pub pid: [f32; 3],
    /// The grid cell the CV system thinks the robot is in
    ///
    /// Not used when this struct functions as a configuration in server settings
    pub cv_location: Option<Point2<i8>>,
    /// The points the robot should try to go to
    pub target_path: heapless::Vec<Point2<i8>, MAX_ROBOT_PATH_LENGTH>,
    /// Whether the robot should try to follow the target path (including maintaining heading 0)
    pub follow_target_path: bool,
    /// This angle should be considered angle 0
    pub angle_offset: f32,
    pub lookahead_dist: f32,
    pub robot_speed: f32,
    pub snapping_dist: f32,
    pub cv_error: f32,

    pub enable_imu: bool,
    pub enable_extra_imu_data: bool,
    pub enable_dists: bool,
    pub enable_battery_monitor: bool,
    pub enable_display: bool,
    pub enable_gamepad: bool,
    pub display_loop_interval: u64,
}

impl FrequentServerToRobot {
    /// Create one with default parameters of the given robot
    pub fn new(robot: RobotName) -> Self {
        let definition = RobotDefinition::new(robot);
        Self {
            grid: StandardGrid::Pacman,
            target_velocity: VelocityControl::None,
            motors_override: [None; 3],
            pwm_override: [[None; 2]; 3],
            motor_config: definition.default_motor_config,
            encoder_config: definition.default_encoder_config,
            dist_sensor_config: definition.default_dist_sensor_order,
            pid: definition.default_pid,
            cv_location: None,
            target_path: heapless::Vec::new(),
            follow_target_path: false,
            angle_offset: 0.0,
            lookahead_dist: 0.5,
            robot_speed: 1.5,
            snapping_dist: 0.3,
            cv_error: 1.5,

            enable_imu: INITIAL_ENABLE_IMU,
            enable_extra_imu_data: INITIAL_ENABLE_EXTRA_IMU_DATA,
            enable_dists: INITIAL_ENABLE_DISTS,
            enable_battery_monitor: INITIAL_ENABLE_BATTERY_MONITOR,
            enable_display: INITIAL_ENABLE_DISPLAY,
            enable_gamepad: INITIAL_ENABLE_GAMEPAD,
            display_loop_interval: 500,
        }
    }
}

/// Firmware related items MUST remain first, or OTA programming will break
#[derive(Clone, Debug, Serialize, Deserialize)]
#[repr(usize)]
pub enum ServerToRobotMessage {
    ReadyToStartUpdate = 0,
    FirmwareWritePart {
        offset: usize,
        len: usize,
    } = 1,
    CalculateFirmwareHash(u32) = 2,
    MarkFirmwareUpdated = 3,
    IsFirmwareSwapped = 4,
    Reboot = 5,
    MarkFirmwareBooted = 6,
    CancelFirmwareUpdate = 7,
    /// See [`FrequentServerToRobot`]
    FrequentRobotItems(FrequentServerToRobot) = 8,
    Ping = 9,
    ExtraOpts(ExtraOptsTypes) = 11,
}

#[derive(Copy, Clone, Debug, Default, Serialize, Deserialize, PartialOrd, PartialEq)]
pub struct ExtraOptsTypes {
    pub opts_bool: [bool; 8],
    pub opts_f32: [f32; 4],
    pub opts_i8: [i8; 4],
    pub opts_i32: [i32; 4],
}

pub type ExtraOptsAtomicTypes = (
    [AtomicBool; 8],
    [AtomicF32; 8],
    [AtomicI8; 8],
    [AtomicI32; 8],
);

#[derive(Copy, Clone, Debug, Default, Serialize, Deserialize)]
pub struct MotorControlStatus {
    pub pwm: [[u16; 2]; 3],
    pub speed_set_points: [f32; 3],
    pub measured_speeds: [f32; 3],
}

#[derive(Copy, Clone, Debug, Default, Serialize, Deserialize)]
pub struct ExtraImuData {
    pub accel: ([f32; 3], u8),
    pub gyro: ([f32; 3], u8),
    pub mag: ([f32; 3], u8),
    pub rotation_vector: ([f32; 4], f32, u8),
}

/// Firmware related items MUST remain first, or OTA programming will break
#[derive(Clone, Debug, Serialize, Deserialize)]
#[repr(usize)]
pub enum RobotToServerMessage {
    ReadyToStartUpdate = 0,
    ConfirmFirmwarePart { offset: usize, len: usize } = 1,
    MarkedFirmwareUpdated = 2,
    FirmwareHash([u8; 32]) = 3,
    Rebooting = 4,
    FirmwareIsSwapped(bool) = 5,
    MarkedFirmwareBooted = 6,
    Name(RobotName) = 7,
    MotorControlStatus((Duration, MotorControlStatus)) = 8,
    Utilization([f32; 3]) = 9,
    Sensors(SensorData) = 10,
    Pong = 11,
    ReceivedExtraOpts(ExtraOptsTypes) = 12,
    ExtraIndicators(ExtraOptsTypes) = 13,
    ExtraImuData(ExtraImuData) = 14,
}

/// The different async tasks that run on the robot
#[derive(Copy, Clone, Debug)]
#[repr(usize)]
pub enum Task {
    Wifi = 0,
    Motors = 1,
    Peripherals = 2,
}

impl Task {
    pub fn get_all() -> [Self; 3] {
        [Task::Wifi, Task::Motors, Task::Peripherals]
    }
}

pub const MAX_SENSOR_ERR_LEN: usize = 10;

/// Sent from the robot peripherals task to the wifi task and back to the server
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SensorData {
    /// The absolute orientation of the robot, given by the IMU
    pub angle: Result<f32, heapless::String<MAX_SENSOR_ERR_LEN>>,
    /// Readings from the distance sensors, in order of angle 0, 90, 180, 270
    ///
    /// - Err(_) indicates that something is wrong with the sensor and the reading can't be trusted
    /// - Ok(None) indicates that the sensor is working, but didn't detect any object in its range
    /// - Ok(x) indicates an object x grid units in front of the sensor
    pub distances: [Result<Option<f32>, heapless::String<MAX_SENSOR_ERR_LEN>>; 4],
    /// The best guess location of the robot
    pub location: Option<Point2<f32>>,
    /// The battery level of the robot
    pub battery: Result<f32, heapless::String<MAX_SENSOR_ERR_LEN>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RobotStatus {}

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

impl NetworkStatus {
    #[cfg(feature = "std")]
    pub fn status(&self) -> ColoredStatus {
        match self {
            NetworkStatus::NotConnected => {
                ColoredStatus::NotApplicable(Some("Not connected".to_string()))
            }
            NetworkStatus::ConnectionFailed => {
                ColoredStatus::Error(Some("Connection failed".to_string()))
            }
            NetworkStatus::Connecting => ColoredStatus::Warn(Some("Connecting".to_string())),
            NetworkStatus::Connected => ColoredStatus::Ok(Some("Connected".to_string())),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[allow(clippy::large_enum_variant)]
pub enum GameServerCommand {
    Pause,
    Unpause,
    Reset,
    Direction(Direction),
    SetState(GameState),
}

impl GameServerCommand {
    pub fn text(&self) -> Option<&'static str> {
        match self {
            GameServerCommand::Pause => Some("p"),
            GameServerCommand::Unpause => Some("P"),
            GameServerCommand::Reset => Some("r"),
            GameServerCommand::Direction(Direction::Up) => Some("w"),
            GameServerCommand::Direction(Direction::Left) => Some("a"),
            GameServerCommand::Direction(Direction::Down) => Some("s"),
            GameServerCommand::Direction(Direction::Right) => Some("d"),
            _ => None,
        }
    }
}
