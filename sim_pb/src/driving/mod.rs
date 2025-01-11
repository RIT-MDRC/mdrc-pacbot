use crate::driving::motors::SimMotors;
use crate::driving::network::SimNetwork;
use crate::driving::peripherals::{SimDisplay, SimPeripherals};
use async_channel::{bounded, Receiver, Sender};
use bevy::log::info;
use bevy::tasks::block_on;
use bevy_rapier2d::na::Vector2;
use core_pb::driving::data::SharedRobotData;
use core_pb::driving::motors::motors_task;
use core_pb::driving::network::network_task;
use core_pb::driving::peripherals::peripherals_task;
use core_pb::driving::RobotBehavior;
use core_pb::messages::RobotButton;
use core_pb::names::{RobotName, NUM_ROBOT_NAMES};
use core_pb::util::WebTimeInstant;
use futures::{select, FutureExt};
use std::collections::VecDeque;
use std::sync::{Arc, OnceLock, RwLock};
use std::thread::spawn;

mod motors;
mod network;
mod peripherals;

pub const CHANNEL_BUFFER_SIZE: usize = 64;

pub struct SimRobot {
    pub name: RobotName,

    pub display: SimDisplay,
    pub display_updated: bool,

    pub thread_stopper: Sender<()>,
    pub firmware_updated: bool,
    pub reboot: bool,

    pub imu_angle: Result<f32, ()>,
    pub velocity: Vector2<f32>,
    pub ang_velocity: f32,
    pub distance_sensors: [Result<Option<f32>, ()>; 4],

    pub wasd_motor_speeds: Option<[f32; 3]>,
    pub requested_motor_speeds: [f32; 3],
    pub actual_motor_speeds: [f32; 3],

    pub button_events: VecDeque<(RobotButton, bool)>,
    pub joystick: Option<(f32, f32)>,
}

impl SimRobot {
    pub fn start(name: RobotName, firmware_swapped: bool) -> Arc<RwLock<Self>> {
        let (thread_stopper_tx, thread_stopper_rx) = bounded(CHANNEL_BUFFER_SIZE);

        let robot = Arc::new(RwLock::new(Self {
            name,

            display: SimDisplay::default(),
            display_updated: true,

            thread_stopper: thread_stopper_tx,
            firmware_updated: false,
            reboot: false,

            imu_angle: Err(()),
            velocity: Vector2::new(0.0, 0.0),
            ang_velocity: 0.0,
            distance_sensors: [Err(()); 4],

            wasd_motor_speeds: None,
            requested_motor_speeds: [0.0; 3],
            actual_motor_speeds: [0.0; 3],

            button_events: VecDeque::new(),
            joystick: None,
        }));

        let motors = SimMotors::new(name, robot.clone());
        let network = SimNetwork::new(name, firmware_swapped, robot.clone());
        let peripherals = SimPeripherals::new(robot.clone());

        spawn(move || {
            block_on(Self::start_async(
                name,
                motors,
                network,
                SimRobot::get(name),
                peripherals,
                thread_stopper_rx,
            ))
        });

        robot
    }

    pub(crate) fn destroy(&mut self) {
        block_on(async { self.thread_stopper.send(()).await }).unwrap();
    }

    #[allow(clippy::too_many_arguments)]
    async fn start_async(
        name: RobotName,
        motors: SimMotors,
        network: SimNetwork,
        data: &'static SharedRobotData<SimRobot>,
        peripherals: SimPeripherals,
        thread_stopper: Receiver<()>,
    ) {
        select! {
            _ = thread_stopper.recv().fuse() => {
                info!("{name} destroyed");
            }
            _ = network_task(data, network).fuse() => {
                info!("{name} network task ended early");
            }
            _ = motors_task(data, motors).fuse() => {
                info!("{name} motors task ended early");
            }
            _ = peripherals_task(data, peripherals).fuse() => {
                info!("{name} peripherals task ended early");
            }
        }
    }
}

impl RobotBehavior for SimRobot {
    type Instant = WebTimeInstant;

    type Motors = SimMotors;
    type Network = SimNetwork;
    type Peripherals = SimPeripherals;
}

static ROBOT_DATA: [OnceLock<SharedRobotData<SimRobot>>; NUM_ROBOT_NAMES] = [
    OnceLock::new(),
    OnceLock::new(),
    OnceLock::new(),
    OnceLock::new(),
    OnceLock::new(),
    OnceLock::new(),
];

impl SimRobot {
    pub fn get(name: RobotName) -> &'static SharedRobotData<SimRobot> {
        ROBOT_DATA[name as usize].get_or_init(|| SharedRobotData::new(name))
    }
}
