use crate::constants::{
    GAME_SERVER_PORT, ROBOT_TCP_PORT, ROBOT_UDP_LISTENING_PORT, SIMULATION_LISTENER_PORT,
};
use crate::grid::standard_grid::StandardGrid;
use serde::{Deserialize, Serialize};

/// Rarely changed options for the pacbot server
#[derive(Clone, Debug, PartialOrd, PartialEq, Serialize, Deserialize)]
pub struct PacbotSettings {
    /// Host a web server for browser clients
    pub host_http: bool,
    /// Which grid is current in use
    pub standard_grid: StandardGrid,
    /// Options for the simulation
    pub simulation: SimulationSettings,
    /// Options for the go server
    pub game_server: GameServerSettings,
    /// Options for the robot
    pub robots: Vec<RobotSettings>,
    /// Options for pathing, speed
    pub driving: DriveSettings,
}

impl Default for PacbotSettings {
    fn default() -> Self {
        Self {
            host_http: false,
            simulation: Default::default(),
            standard_grid: Default::default(),
            robots: vec![],
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
    /// Whether the app should try to connect/reconnect to the robot
    pub connect: bool,
    /// IP address of the robot, if it should be connected
    pub ipv4: [u8; 4],
    /// The UDP port the robot is listening on
    pub udp_listening_port: u16,
    /// The TCP port the robot is listening on
    pub tcp_port: u16,

    /// P, I, and D parameters for the PID loop
    pub pid: [f32; 3],
    /// Whether the robot will modify the requested velocity to avoid collisions
    pub collision_avoidance: bool,
    /// Minimum and maximum thresholds, in mm, for the distance at which the robot will modify the requested velocity to avoid collisions
    pub collision_avoidance_thresholds: (u8, u8),
    /// Distance sensor continuous range interval
    ///
    /// period = 0 means 10ms intervals. Then + 1 adds 10ms, so period = 2 means 30ms intervals.
    pub sensor_range_interval: u8,
    /// The maximum change in velocity, in gu/s/s, for the setpoint of the PID loop
    pub max_accel: f32,
}

impl Default for RobotSettings {
    fn default() -> Self {
        Self {
            connect: false,
            ipv4: [127, 0, 0, 2],
            udp_listening_port: ROBOT_UDP_LISTENING_PORT,
            tcp_port: ROBOT_TCP_PORT,
            pid: [18.0, 0.1, 0.0],
            collision_avoidance: true,
            collision_avoidance_thresholds: (15, 130),
            sensor_range_interval: 5,
            max_accel: 1000.0,
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

#[derive(Clone, Debug, Default, PartialOrd, PartialEq, Serialize, Deserialize)]
pub enum StrategyChoice {
    /// No movement
    Stop,
    /// WASD, or right click to set target
    #[default]
    Manual,
    /// AI, with a path to the .safetensors file
    ReinforcementLearning(String),
    /// Test (random, uniform over all cells)
    TestUniform,
    /// Test (never goes back on itself)
    TestForward,
}
