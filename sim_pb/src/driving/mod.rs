use crate::driving::motors::SimMotors;
use crate::driving::network::SimNetwork;
use crate::driving::peripherals::{SimDisplay, SimPeripherals};
use crate::RobotToSimulationMessage;
use async_channel::{bounded, Receiver, Sender, TrySendError};
use async_std::task::sleep;
use bevy::log::info;
use bevy::tasks::block_on;
use core_pb::driving::motors::motors_task;
use core_pb::driving::network::network_task;
use core_pb::driving::peripherals::peripherals_task;
use core_pb::driving::{RobotInterTaskMessage, RobotTaskMessenger};
use core_pb::messages::{RobotButton, Task};
use core_pb::names::RobotName;
use futures::future::{select, Either};
use futures::{select, FutureExt};
use std::collections::VecDeque;
use std::fmt::Debug;
use std::future::Future;
use std::pin::pin;
use std::sync::{Arc, RwLock};
use std::thread::spawn;
use std::time::Duration;

mod motors;
mod network;
mod peripherals;

pub const CHANNEL_BUFFER_SIZE: usize = 64;

pub struct SimRobot {
    #[allow(unused)]
    pub name: RobotName,

    pub display: SimDisplay,
    pub display_updated: bool,

    pub thread_stopper: Sender<()>,
    pub firmware_updated: bool,

    pub imu_angle: Result<f32, ()>,
    pub distance_sensors: [Result<Option<f32>, ()>; 4],

    pub button_events: VecDeque<(RobotButton, bool)>,
    pub joystick: Option<(f32, f32)>,
}

async fn handle_task<F, E: Debug>(task: F)
where
    F: Future<Output = Result<(), E>>,
{
    task.await.unwrap();
}

impl SimRobot {
    pub fn start(
        name: RobotName,
        firmware_swapped: bool,
        sim_tx: Sender<(RobotName, RobotToSimulationMessage)>,
    ) -> Arc<RwLock<Self>> {
        let (thread_stopper_tx, thread_stopper_rx) = bounded(CHANNEL_BUFFER_SIZE);

        let robot = Arc::new(RwLock::new(Self {
            name,

            display: SimDisplay::default(),
            display_updated: true,

            thread_stopper: thread_stopper_tx,
            firmware_updated: false,

            imu_angle: Err(()),
            distance_sensors: [Err(()); 4],

            button_events: VecDeque::new(),
            joystick: None,
        }));

        let (tc_motors, motors_rx, motors_tx) = TaskChannels::new();
        let (tc_network, network_rx, network_tx) = TaskChannels::new();
        let (tc_peripherals, peripherals_rx, peripherals_tx) = TaskChannels::new();

        let motors = SimMotors::new(name, sim_tx.clone());
        let network = SimNetwork::new(name, firmware_swapped, sim_tx.clone());
        let peripherals = SimPeripherals::new(robot.clone());

        spawn(move || {
            block_on(Self::start_async(
                name,
                motors,
                network,
                peripherals,
                [network_tx, motors_tx, peripherals_tx],
                [network_rx, motors_rx, peripherals_rx],
                [tc_network, tc_motors, tc_peripherals],
                thread_stopper_rx,
            ))
        });

        robot
    }

    pub(crate) fn destroy(&mut self) {
        block_on(async { self.thread_stopper.send(()).await }).unwrap();
    }

    async fn handle_one_task_messages(
        receiver: Receiver<(RobotInterTaskMessage, Task)>,
        senders: [Sender<RobotInterTaskMessage>; 3],
    ) {
        while let Ok((msg, to)) = receiver.recv().await {
            match to {
                Task::Wifi => &senders[0],
                Task::Motors => &senders[1],
                Task::Peripherals => &senders[2],
            }
            .send(msg)
            .await
            .unwrap();
        }
    }

    async fn start_async(
        name: RobotName,
        motors: SimMotors,
        network: SimNetwork,
        peripherals: SimPeripherals,
        senders: [Sender<RobotInterTaskMessage>; 3],
        receivers: [Receiver<(RobotInterTaskMessage, Task)>; 3],
        task_channels: [TaskChannels; 3],
        thread_stopper: Receiver<()>,
    ) {
        let [r0, r1, r2] = receivers;
        let [t0, t1, t2] = task_channels;
        select! {
            _ = thread_stopper.recv().fuse() => {
                info!("{name} destroyed");
            }
            _ = handle_task(network_task(network, t0)).fuse() => {
                info!("{name} network task ended early");
            }
            _ = handle_task(motors_task(name, motors, t1)).fuse() => {
                info!("{name} motors task ended early");
            }
            _ = handle_task(peripherals_task(name, peripherals, t2)).fuse() => {
                info!("{name} peripherals task ended early");
            }
            _ = Self::handle_one_task_messages(r0, senders.clone()).fuse() => {
                info!("{name} messages task ended early");
            }
            _ = Self::handle_one_task_messages(r1, senders.clone()).fuse() => {
                info!("{name} messages task ended early");
            }
            _ = Self::handle_one_task_messages(r2, senders.clone()).fuse() => {
                info!("{name} messages task ended early");
            }
        }
    }
}

pub struct TaskChannels {
    tx: Sender<(RobotInterTaskMessage, Task)>,
    rx: Receiver<RobotInterTaskMessage>,
}

impl TaskChannels {
    pub fn new() -> (
        Self,
        Receiver<(RobotInterTaskMessage, Task)>,
        Sender<RobotInterTaskMessage>,
    ) {
        let (from_tx, from_rx) = bounded(CHANNEL_BUFFER_SIZE);
        let (to_tx, to_rx) = bounded(CHANNEL_BUFFER_SIZE);

        (
            Self {
                tx: from_tx,
                rx: to_rx,
            },
            from_rx,
            to_tx,
        )
    }
}

impl RobotTaskMessenger for TaskChannels {
    fn send_or_drop(&mut self, message: RobotInterTaskMessage, to: Task) -> bool {
        match self.tx.try_send((message, to)) {
            Ok(_) => true,
            Err(TrySendError::Closed(_)) => unreachable!(),
            _ => false,
        }
    }

    async fn send_blocking(&mut self, message: RobotInterTaskMessage, to: Task) {
        self.tx.send((message, to)).await.unwrap();
    }

    async fn receive_message(&mut self) -> RobotInterTaskMessage {
        loop {
            if let Ok(m) = self.rx.recv().await {
                return m;
            }
        }
    }

    async fn receive_message_timeout(
        &mut self,
        timeout: Duration,
    ) -> Option<RobotInterTaskMessage> {
        match select(pin!(sleep(timeout)), pin!(self.rx.recv())).await {
            Either::Left(_) => None,
            Either::Right(msg) => Some(msg.0.unwrap()),
        }
    }
}
