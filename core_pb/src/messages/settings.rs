use crate::constants::{GAME_SERVER_PORT, SIMULATION_LISTENER_PORT};
use crate::grid::standard_grid::StandardGrid;
use crate::names::{RobotName, NUM_ROBOT_NAMES};
use serde::{Deserialize, Serialize};

/// Rarely changed options for the pacbot server
#[derive(Clone, Debug, PartialOrd, PartialEq, Serialize, Deserialize)]
pub struct PacbotSettings {
    /// Host a web server for browser clients
    pub host_http: bool,
    /// In safe mode, only messages related to over the air programming will be sent and received
    pub safe_mode: bool,
    /// Which grid is current in use
    pub standard_grid: StandardGrid,
    /// Options for the simulation
    pub simulation: SimulationSettings,
    /// Options for the go server
    pub game_server: GameServerSettings,
    /// Options for the robots
    pub robots: [RobotSettings; NUM_ROBOT_NAMES],
    /// Options for pathing, speed
    pub driving: DriveSettings,
}

impl Default for PacbotSettings {
    fn default() -> Self {
        Self {
            host_http: false,
            safe_mode: false,
            simulation: Default::default(),
            standard_grid: Default::default(),
            robots: RobotName::get_all().map(RobotSettings::new),
            game_server: Default::default(),
            driving: Default::default(),
        }
    }
}

/// Generic network connection settings
#[derive(Clone, Debug, PartialOrd, PartialEq, Serialize, Deserialize)]
pub struct ConnectionSettings {
    /// Whether the app should try to connect/reconnect
    pub connect: bool,
    /// IP address, if it should be connected
    pub ipv4: [u8; 4],
    /// Port
    pub port: u16,
}

/// Simulation options
#[derive(Clone, Debug, PartialOrd, PartialEq, Serialize, Deserialize)]
pub struct SimulationSettings {
    /// Launch a fake game server and physics simulation as a child process
    pub simulate: bool,
    /// Network details
    pub connection: ConnectionSettings,
    /// Which robots should be spawned in
    pub robots: [bool; NUM_ROBOT_NAMES],
    /// Which robot's position should be used as the pacman location
    pub pacman: RobotName,
}

impl Default for SimulationSettings {
    fn default() -> Self {
        Self {
            simulate: false,
            connection: ConnectionSettings {
                connect: false,
                ipv4: [127, 0, 0, 1],
                port: SIMULATION_LISTENER_PORT,
            },
            robots: RobotName::get_all().map(|name| name == RobotName::Stella),
            pacman: RobotName::Stella,
        }
    }
}

/// Game server network options
#[derive(Clone, Debug, PartialOrd, PartialEq, Serialize, Deserialize)]
pub struct GameServerSettings {
    /// Network details
    pub connection: ConnectionSettings,
}

impl Default for GameServerSettings {
    fn default() -> Self {
        Self {
            connection: ConnectionSettings {
                connect: false,
                ipv4: [127, 0, 0, 1],
                port: GAME_SERVER_PORT,
            },
        }
    }
}

/// Pico network options, on-robot drive code options
#[derive(Clone, Debug, PartialOrd, PartialEq, Serialize, Deserialize)]
pub struct RobotSettings {
    pub name: RobotName,
    /// Connection settings
    pub connection: ConnectionSettings,

    /// P, I, and D parameters for the PID loop
    pub pid: [f32; 3],
    /// Which pwm pin corresponds to forwards and backwards for each motor
    pub motor_config: [[usize; 2]; 3],
    /// Overrides the outputs from wheel speed
    pub pwm_override: [[Option<u16>; 2]; 3],
    /// Overrides the set points of the motors
    pub set_point_override: [Option<f32>; 3],
}

impl RobotSettings {
    pub fn new(name: RobotName) -> Self {
        Self {
            name,
            connection: ConnectionSettings {
                connect: false,
                ipv4: name.default_ip(),
                port: name.port(),
            },
            pid: name.robot().default_pid,
            motor_config: name.robot().default_motor_config,
            pwm_override: [[None; 2]; 3],
            set_point_override: [None; 3],
        }
    }
}

#[derive(Clone, Debug, PartialOrd, PartialEq, Serialize, Deserialize)]
pub struct DriveSettings {
    /// Determines target position and path
    pub strategy: StrategyChoice,

    /// The speed, in gu/s, to travel when the path length is 1, when pathing autonomously
    pub speed_base: f32,
    /// The speed, in gu/s, to add for each additional grid unit in the same direction, when pathing autonomously
    pub speed_multiplier: f32,
    /// The maximum speed, in gu/s, when pathing autonomously
    pub speed_cap: f32,

    /// The translational speed, in gu/s, when driving with manual controls
    pub manual_speed: f32,
    /// The rotational speed, in rad/s, when driving with manual controls
    pub manual_rotation_speed: f32,

    /// When giving motor commands to the robot, should the particle
    /// filter's current rotation be accounted for?
    pub commands_use_pf_angle: bool,
}

impl Default for DriveSettings {
    fn default() -> Self {
        Self {
            strategy: StrategyChoice::default(),
            speed_base: 3.0,
            speed_multiplier: 2.0,
            speed_cap: 8.0,
            manual_speed: 8.0,
            manual_rotation_speed: 2.0,
            commands_use_pf_angle: true,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialOrd, PartialEq, Hash, Serialize, Deserialize, Ord, Eq)]
pub enum KnownRLModel {
    QNet,
    Endgame,
}

impl KnownRLModel {
    pub fn path(&self) -> &'static str {
        match self {
            KnownRLModel::QNet => "checkpoints/q_net.safetensors",
            KnownRLModel::Endgame => "checkpoints/endgame.safetensors",
        }
    }
}

#[derive(Clone, Debug, Default, PartialOrd, PartialEq, Serialize, Deserialize)]
pub enum StrategyChoice {
    /// No movement
    Stop,
    /// WASD, or right click to set target
    #[default]
    Manual,
    /// AI
    ReinforcementLearning(KnownRLModel),
    /// Test (random, uniform over all cells)
    TestUniform,
    /// Test (never goes back on itself)
    TestForward,
}
