use crate::driving::motors::SimMotors;
use crate::driving::network::SimNetwork;
use crate::driving::peripherals::SimPeripherals;
use async_channel::{Receiver, Sender};
use core_pb::driving::motors::motors_task;
use core_pb::driving::network::network_task;
use core_pb::driving::peripherals::peripherals_task;
use core_pb::driving::{RobotInterTaskMessage, RobotTask, Task};
use futures::join;
use std::fmt::Debug;
use std::future::Future;
use std::sync::{Arc, Mutex};

mod motors;
mod network;
mod peripherals;

pub struct SimRobot {}

async fn handle_task<F, E: Debug>(task: F)
where
    F: Future<Output = Result<(), E>>,
{
    task.await.unwrap();
}

#[allow(dead_code)]
impl SimRobot {
    pub fn start() -> (Arc<Mutex<Self>>, impl Future<Output = ()>) {
        let (motors, motors_rx, motors_tx) = TaskChannels::new();
        let (network, network_rx, network_tx) = TaskChannels::new();
        let (peripherals, peripherals_rx, peripherals_tx) = TaskChannels::new();

        let motors = SimMotors::new(motors);
        let network = SimNetwork::new(network);
        let peripherals = SimPeripherals::new(peripherals);

        let f = Self::start_async(
            motors,
            network,
            peripherals,
            [motors_tx, network_tx, peripherals_tx],
            [motors_rx, network_rx, peripherals_rx],
        );

        (Arc::new(Mutex::new(Self {})), f)
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
    ) {
        let [r0, r1, r2] = receivers;
        join!(
            handle_task(motors_task(motors)),
            handle_task(network_task(network)),
            handle_task(peripherals_task(peripherals)),
            Self::handle_one_task_messages(r0, senders.clone()),
            Self::handle_one_task_messages(r1, senders.clone()),
            Self::handle_one_task_messages(r2, senders.clone()),
        );
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
