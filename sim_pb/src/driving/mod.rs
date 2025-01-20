use crate::driving::motors::SimMotors;
use crate::driving::network::SimNetwork;
use crate::driving::peripherals::{SimDisplay, SimPeripherals};
use async_channel::{bounded, Receiver, Sender};
use bevy::log::info;
use bevy::tasks::block_on;
use core_pb::driving::data::{SharedRobotData, NUM_WHEELS};
use core_pb::driving::motors::motors_task;
use core_pb::driving::network::network_task;
use core_pb::driving::peripherals::peripherals_task;
use core_pb::driving::RobotBehavior;
use core_pb::messages::RobotButton;
use core_pb::names::RobotName;
use core_pb::util::WebTimeInstant;
use futures::{select, FutureExt};
use std::collections::VecDeque;
use std::sync::{Arc, RwLock};
use std::thread::spawn;

mod motors;
mod network;
mod peripherals;

pub const CHANNEL_BUFFER_SIZE: usize = 64;

pub struct SimRobot<const WHEELS: usize> {
    pub name: RobotName,
    pub data: Arc<SharedRobotData<SimRobot<WHEELS>>>,

    pub display: SimDisplay,
    pub display_updated: bool,

    pub thread_stopper: Sender<()>,
    pub firmware_updated: bool,
    pub reboot: bool,

    pub wasd_motor_speeds: Option<[f32; WHEELS]>,
    pub requested_motor_speeds: [f32; WHEELS],

    pub button_events: VecDeque<(RobotButton, bool)>,
    pub joystick: Option<(f32, f32)>,
}

impl SimRobot<NUM_WHEELS> {
    pub fn start(name: RobotName, firmware_swapped: bool) -> Arc<RwLock<Self>> {
        let shared_data = Arc::new(SharedRobotData::new(name));
        let (thread_stopper_tx, thread_stopper_rx) = bounded(CHANNEL_BUFFER_SIZE);

        let robot = Arc::new(RwLock::new(Self {
            name,
            data: shared_data.clone(),

            display: SimDisplay::default(),
            display_updated: true,

            thread_stopper: thread_stopper_tx,
            firmware_updated: false,
            reboot: false,

            wasd_motor_speeds: None,
            requested_motor_speeds: [0.0; NUM_WHEELS],

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
                shared_data.clone(),
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
        motors: SimMotors<NUM_WHEELS>,
        network: SimNetwork,
        data: Arc<SharedRobotData<SimRobot<NUM_WHEELS>>>,
        peripherals: SimPeripherals,
        thread_stopper: Receiver<()>,
    ) {
        select! {
            _ = thread_stopper.recv().fuse() => {
                info!("{name} destroyed");
            }
            _ = network_task(&data, network).fuse() => {
                info!("{name} network task ended early");
            }
            _ = motors_task(&data, motors).fuse() => {
                info!("{name} motors task ended early");
            }
            _ = peripherals_task(&data, peripherals).fuse() => {
                info!("{name} peripherals task ended early");
            }
        }
    }
}

impl<const WHEELS: usize> RobotBehavior for SimRobot<WHEELS> {
    type Instant = WebTimeInstant;

    type Motors = SimMotors<NUM_WHEELS>;
    type Network = SimNetwork;
    type Peripherals = SimPeripherals;
}
