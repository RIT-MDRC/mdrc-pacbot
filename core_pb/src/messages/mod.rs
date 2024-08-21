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
use nalgebra::Vector2;
#[cfg(feature = "std")]
use nalgebra::{Point2, Rotation2};
use pacbot_rs::game_state::GameState;
use pacbot_rs::location::Direction;
use serde::{Deserialize, Serialize};

#[cfg(feature = "std")]
pub mod ota;
#[cfg(feature = "std")]
pub mod server_status;
#[cfg(feature = "std")]
pub mod settings;

pub const GAME_SERVER_MAGIC_NUMBER: [u8; 4] = [170, 115, 26, 153];

#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg(feature = "std")]
#[allow(clippy::large_enum_variant)]
pub enum GuiToServerMessage {
    Settings(PacbotSettings),
    GameServerCommand(GameServerCommand),
    SimulationCommand(ServerToSimulationMessage),
    RobotVelocity(RobotName, Option<(Vector2<f32>, f32)>),
    StartOtaFirmwareUpdate(RobotName),
    CancelOtaFirmwareUpdate(RobotName),
    ConfirmFirmwareUpdate(RobotName),
    ClearFirmwareUpdateHistory(RobotName),
    TargetLocation(Point2<i8>),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg(feature = "std")]
#[allow(clippy::large_enum_variant)]
pub enum ServerToGuiMessage {
    Status(ServerStatus),
    Settings(PacbotSettings),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg(feature = "std")]
pub struct SimulationToServerMessage {
    pub robot_positions: [Option<(Point2<f32>, Rotation2<f32>)>; NUM_ROBOT_NAMES],
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg(feature = "std")]
pub enum ServerToSimulationMessage {
    Spawn(RobotName),
    Teleport(RobotName, Point2<i8>),
    Delete(RobotName),
    SetPacman(RobotName),
}

/// This is sent regularly and frequently to robots via [`ServerToRobotMessage::FrequentRobotItems`]
///
/// Holds information that may change often, or where low latency is critical. Its contents should be passed
/// along as quickly as possible.
#[derive(Clone, Debug, PartialOrd, PartialEq, Serialize, Deserialize)]
pub struct FrequentServerToRobot {
    /// Overall requested velocity of the robot, ex. using WASD or controller manual input
    pub target_velocity: Option<(Vector2<f32>, f32)>,
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
    /// Basic parameters for the PID controller
    pub pid: [f32; 3],
}

impl FrequentServerToRobot {
    /// Create one with default parameters of the given robot
    pub fn new(robot: RobotName) -> Self {
        let definition = RobotDefinition::new(robot);
        Self {
            target_velocity: None,
            motors_override: [None; 3],
            pwm_override: [[None; 2]; 3],
            motor_config: definition.default_motor_config,
            pid: definition.default_pid,
        }
    }
}

/// Firmware related items MUST remain first, or OTA programming will break
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ServerToRobotMessage {
    ReadyToStartUpdate,
    FirmwareWritePart {
        offset: usize,
        len: usize,
    },
    CalculateFirmwareHash(u32),
    MarkFirmwareUpdated,
    IsFirmwareSwapped,
    Reboot,
    MarkFirmwareBooted,
    CancelFirmwareUpdate,
    /// See [`FrequentServerToRobot`]
    FrequentRobotItems(FrequentServerToRobot),
}

#[derive(Copy, Clone, Debug, Default, Serialize, Deserialize)]
pub struct MotorControlStatus {
    pub pwm: [[u16; 2]; 3],
    pub speed_set_points: [f32; 3],
    pub measured_speeds: [f32; 3],
}

/// Firmware related items MUST remain first, or OTA programming will break
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum RobotToServerMessage {
    ReadyToStartUpdate,
    ConfirmFirmwarePart { offset: usize, len: usize },
    MarkedFirmwareUpdated,
    FirmwareHash([u8; 32]),
    Rebooting,
    FirmwareIsSwapped(bool),
    MarkedFirmwareBooted,
    Name(RobotName),
    MotorControlStatus((Duration, MotorControlStatus)),
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
