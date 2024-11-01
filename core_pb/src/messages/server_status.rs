use crate::messages::ota::{OverTheAirStep, OverTheAirStepCompletion};
use crate::messages::{MotorControlStatus, NetworkStatus};
use crate::names::{RobotName, NUM_ROBOT_NAMES};
use crate::util::ColoredStatus;
use nalgebra::{Point2, Rotation2};
use pacbot_rs::game_state::GameState;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ServerStatus {
    pub utilization: ColoredStatus,
    pub inference_time: ColoredStatus,

    pub simulation_connection: NetworkStatus,

    pub game_state: GameState,
    pub game_server_connection: NetworkStatus,
    pub advanced_game_server: bool,

    pub cv_location: Option<Point2<i8>>,
    pub target_path: Vec<Point2<i8>>,

    pub gui_clients: usize,
    pub robots: [RobotStatus; NUM_ROBOT_NAMES],
}

impl Default for ServerStatus {
    fn default() -> Self {
        Self {
            utilization: ColoredStatus::Ok(Some("Loading...".to_string())),
            inference_time: ColoredStatus::NotApplicable(Some("N/A".to_string())),

            simulation_connection: NetworkStatus::default(),

            game_state: GameState::default(),
            game_server_connection: NetworkStatus::default(),
            advanced_game_server: false,

            cv_location: None,
            target_path: vec![],

            gui_clients: 0,
            robots: RobotName::get_all().map(RobotStatus::new),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RobotStatus {
    pub name: RobotName,
    pub connection: NetworkStatus,
    pub ping: Option<Duration>,

    pub ota_current: OverTheAirStep,
    pub ota_completed: Vec<OverTheAirStepCompletion>,

    pub last_motor_status: (Duration, MotorControlStatus),
    pub utilization: [f32; 3],

    pub sim_position: Option<(Point2<f32>, Rotation2<f32>)>,

    pub imu_angle: Result<f32, String>,
    pub distance_sensors: [Result<Option<f32>, String>; 4],
    pub estimated_location: Option<Point2<f32>>,
    pub battery: Result<f32, ()>,

    pub display: Option<Vec<u128>>,
}

impl RobotStatus {
    pub fn new(name: RobotName) -> Self {
        Self {
            name,
            connection: NetworkStatus::default(),
            ping: None,

            ota_current: OverTheAirStep::GuiRequest,
            ota_completed: vec![],

            last_motor_status: Default::default(),
            utilization: [0.0; 3],

            sim_position: None,

            imu_angle: Err(String::new()),
            distance_sensors: [const { Err(String::new()) }; 4],
            estimated_location: None,
            battery: Err(()),

            display: None,
        }
    }
}

impl RobotStatus {
    #[cfg(feature = "egui-phosphor")]
    pub fn battery_status(&self) -> ColoredStatus {
        if self.connection != NetworkStatus::Connected {
            ColoredStatus::NotApplicable(Some("Not connected".to_string()))
        } else if let Ok(battery) = self.battery {
            let msg = Some(format!("{:.1}%", battery * 100.0));
            if battery > 0.5 {
                ColoredStatus::Ok(msg)
            } else if battery > 0.25 {
                ColoredStatus::Warn(msg)
            } else {
                ColoredStatus::Error(msg)
            }
        } else {
            ColoredStatus::Error(Some("ERR".to_string()))
        }
    }
}
