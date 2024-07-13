use std::fmt::Debug;
use std::future::{pending, Future};
use std::sync::{Arc, RwLock};
use std::thread::spawn;

use async_channel::unbounded;
use async_channel::{Receiver, Sender};
use bevy::tasks::block_on;
use embedded_graphics::mock_display::MockDisplay;
use embedded_graphics::pixelcolor::BinaryColor;
use futures::{join, select, FutureExt};

use core_pb::driving::motors::motors_task;
use core_pb::driving::network::network_task;
use core_pb::driving::peripherals::peripherals_task;
use core_pb::driving::{RobotInterTaskMessage, RobotTask, Task};
use core_pb::names::RobotName;

use crate::driving::motors::SimMotors;
use crate::driving::network::SimNetwork;
use crate::driving::peripherals::SimPeripherals;

mod motors;
mod network;
mod peripherals;

pub struct SimRobot {
    #[allow(unused)]
    pub name: RobotName,

    pub display: MockDisplay<BinaryColor>,
    pub display_ready: bool,

    pub thread_stopper: Sender<()>,
}

async fn handle_task<F, E: Debug>(task: F)
where
    F: Future<Output = Result<(), E>>,
{
    task.await.unwrap();
}

impl SimRobot {
    pub fn start(name: RobotName) -> Arc<RwLock<Self>> {
        let (thread_stopper_tx, thread_stopper_rx) = unbounded();

        let robot = Arc::new(RwLock::new(Self {
            name,

            display: MockDisplay::new(),
            display_ready: true,

            thread_stopper: thread_stopper_tx,
        }));

        let (motors, motors_rx, motors_tx) = TaskChannels::new();
        let (network, network_rx, network_tx) = TaskChannels::new();
        let (peripherals, peripherals_rx, peripherals_tx) = TaskChannels::new();

        let motors = SimMotors::new(motors);
        let network = SimNetwork::new(name, network);
        let peripherals = SimPeripherals::new(robot.clone(), peripherals);

        let f = Self::start_async(
            motors,
            network,
            peripherals,
            [motors_tx, network_tx, peripherals_tx],
            [motors_rx, network_rx, peripherals_rx],
            thread_stopper_rx,
        );

        spawn(|| block_on(f));

        robot
    }

    pub(crate) fn destroy(&mut self) {
        block_on(async { self.thread_stopper.send(()).await }).unwrap();
    }

    async fn handle_one_task_messages(
        receiver: Receiver<(RobotInterTaskMessage, Task)>,
        senders: [Sender<RobotInterTaskMessage>; 3],
    ) {
        loop {
            match receiver.recv().await {
                Ok((msg, to)) => {
                    match to {
                        Task::Wifi => &senders[0],
                        Task::Motors => &senders[1],
                        Task::Peripherals => &senders[2],
                    }
                    .send(msg)
                    .await
                    .unwrap();
                }
                Err(_) => break,
            }
        }
    }

    async fn start_async(
        motors: SimMotors,
        network: SimNetwork,
        peripherals: SimPeripherals,
        senders: [Sender<RobotInterTaskMessage>; 3],
        receivers: [Receiver<(RobotInterTaskMessage, Task)>; 3],
        thread_stopper: Receiver<()>,
    ) {
        let [r0, r1, r2] = receivers;
        select! {
            _ = thread_stopper.recv().fuse() => {}
            _ = {
                join!(
                    handle_task(motors_task(motors)),
                    handle_task(network_task(network)),
                    handle_task(peripherals_task(peripherals)),
                    Self::handle_one_task_messages(r0, senders.clone()),
                    Self::handle_one_task_messages(r1, senders.clone()),
                    Self::handle_one_task_messages(r2, senders.clone()),
                );
                pending::<()>()
            }.fuse() => {}
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
        let (from_tx, from_rx) = async_channel::unbounded();
        let (to_tx, to_rx) = async_channel::unbounded();

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

impl RobotTask for TaskChannels {
    async fn send_message(&mut self, message: RobotInterTaskMessage, to: Task) -> Result<(), ()> {
        self.tx.send((message, to)).await.map_err(|_| ())
    }

    async fn receive_message(&mut self) -> RobotInterTaskMessage {
        loop {
            if let Ok(m) = self.rx.recv().await {
                return m;
            }
        }
    }
}
