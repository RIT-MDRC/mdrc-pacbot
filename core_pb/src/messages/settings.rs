use crate::grid::standard_grid::StandardGrid;
use serde::{Deserialize, Serialize};

/// Rarely changed options for the pacbot server
#[derive(Clone, Debug, Default, PartialOrd, PartialEq, Serialize, Deserialize)]
pub struct PacbotSettings {
    /// Which grid is current in use
    pub grid: StandardGrid,
    /// Options for the pico
    pub pico: PicoSettings,
    /// Options for the go server
    pub game_server: GameServerSettings,
    /// Options for pathing, speed
    pub driving: DriveSettings,
    /// Options for localization
    pub particle_filter: ParticleFilterSettings,
}

/// Pico network options, on-robot drive code options
#[derive(Clone, Debug, PartialOrd, PartialEq, Serialize, Deserialize)]
pub struct PicoSettings {
    /// IP address of the pico, if it should be connected
    pub ip: String,
    /// The UDP port the pico is listening on
    pub udp_port: u16,
    /// The TCP port the pico is listening on
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

impl Default for PicoSettings {
    fn default() -> Self {
        Self {
            ip: "127.0.0.2:20001".to_string(),
            udp_port: 20013,
            tcp_port: 20014,
            pid: [18.0, 0.1, 0.0],
            collision_avoidance: true,
            collision_avoidance_thresholds: (15, 130),
            sensor_range_interval: 5,
            max_accel: 1000.0,
        }
    }
}

/// Game server network options
#[derive(Clone, Debug, PartialOrd, PartialEq, Serialize, Deserialize)]
pub struct GameServerSettings {
    pub connect: bool,
    /// IP address of the game server, if it should be connected
    pub ip: String,
    /// Websocket port the game server is listening on
    pub ws_port: u16,
}

impl Default for GameServerSettings {
    fn default() -> Self {
        Self {
            connect: false,
            ip: "192.168.0.100:12345".to_string(),
            ws_port: 3002,
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

#[derive(Copy, Clone, Debug, PartialOrd, PartialEq, Serialize, Deserialize)]
pub struct ParticleFilterSettings {
    /// Whether particle filter is calculated
    pub enable_pf: bool,
    /// Determines what is used as CV position
    ///
    /// When particle filter is disabled, this position is used directly
    pub cv_position: CvPositionSource,

    /// The number of guesses tracked by ParticleFilter
    pub pf_total_points: usize,
    /// The number of points displayed on the gui
    pub pf_gui_points: usize,
    /// Chance 0.0-1.0 that a new point will spawn near an existing one instead of randomly
    pub pf_chance_near_other: f32,
    /// The average number of times the robot is kidnapped per second, in our theoretical motion
    /// model. This determines the probability that a particle will be teleported to a random
    /// position.
    pub pf_avg_kidnaps_per_sec: f32,
    /// The standard deviation of the CV position error, in our theoretical sensor model.
    pub pf_cv_error_std: f32,
    /// The standard deviation of the distance sensor errors, in our theoretical sensor model.
    pub pf_sensor_error_std: f32,

    /// When generating a point based on an existing point, how far can it be moved in x and y?
    pub pf_translation_limit: f32,
    /// When generating a point based on an existing point, how far can it be moved in rotation?
    pub pf_rotation_limit: f32,

    /// When moving particles by Rapier-reported distance, add noise proportional to translation
    pub pf_simulated_translation_noise: f32,
    /// When moving particles by Rapier-reported distance, add noise proportional to rotation
    pub pf_simulated_rotation_noise: f32,
    /// When moving particles by Rapier-reported distance, add noise
    pub pf_generic_noise: f32,
}

impl Default for ParticleFilterSettings {
    fn default() -> Self {
        Self {
            enable_pf: false,
            cv_position: CvPositionSource::default(),
            pf_total_points: 10000,
            pf_gui_points: 1000,
            pf_chance_near_other: 0.99,
            pf_avg_kidnaps_per_sec: 1.0,
            pf_cv_error_std: 1.0,
            pf_sensor_error_std: 1.0,
            pf_translation_limit: 0.3,
            pf_rotation_limit: 0.3,
            pf_simulated_translation_noise: 0.01,
            pf_simulated_rotation_noise: 0.02,
            pf_generic_noise: 1.0,
        }
    }
}

/// Determines what is used as CV position
#[derive(Copy, Clone, Debug, Default, PartialOrd, PartialEq, Serialize, Deserialize)]
pub enum CvPositionSource {
    /// Game state
    #[default]
    GameState,
    /// Particle filter position (gives confirmation bias to PF)
    ParticleFilter,
    /// Some constant position
    Constant(i8, i8),
}
