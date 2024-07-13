use std::pin::pin;

use async_channel::{unbounded, Receiver, Sender};
use futures_util::future::{select_all, Either};
use futures_util::{select, FutureExt};
use tokio::spawn;

use core_pb::messages::{NetworkStatus, RobotToServerMessage, ServerToRobotMessage};
use core_pb::names::{RobotName, NUM_ROBOT_NAMES};
use core_pb::threaded_websocket::{Address, TextOrT, ThreadedSocket};

pub struct RobotsNetwork {
    pub outgoing: Sender<(RobotName, Either<ServerToRobotMessage, Option<Address>>)>,
    pub incoming: Receiver<(
        RobotName,
        Either<TextOrT<RobotToServerMessage>, NetworkStatus>,
    )>,
}

impl Default for RobotsNetwork {
    fn default() -> Self {
        let (tx1, rx1) = unbounded();
        let (tx2, rx2) = unbounded();

        let _ = spawn(run_robots(rx1, tx2));

        Self {
            outgoing: tx1,
            incoming: rx2,
        }
    }
}

async fn run_robots(
    outgoing: Receiver<(RobotName, Either<ServerToRobotMessage, Option<Address>>)>,
    incoming: Sender<(
        RobotName,
        Either<TextOrT<RobotToServerMessage>, NetworkStatus>,
    )>,
) {
    let mut sockets = RobotName::get_all().map(|_| ThreadedSocket::default());

    loop {
        let event: Either<
            (
                RobotName,
                Either<TextOrT<RobotToServerMessage>, NetworkStatus>,
            ),
            (RobotName, Either<ServerToRobotMessage, Option<Address>>),
        > = {
            let fut = pin!(robot_event_fut(&mut sockets));

            select! {
                incoming = fut.fuse() => {
                    Either::Left(incoming)
                }
                outgoing = outgoing.recv().fuse() => {
                    Either::Right(outgoing.unwrap())
                }
            }
        };

        match event {
            Either::Left(msg) => incoming.send(msg).await.unwrap(),
            Either::Right((name, Either::Left(msg))) => {
                sockets[name as usize].send(TextOrT::T(msg));
            }
            Either::Right((name, Either::Right(addr))) => sockets[name as usize].connect(addr),
        }
    }
}

async fn robot_event_fut(
    robots: &mut [ThreadedSocket<ServerToRobotMessage, RobotToServerMessage>; NUM_ROBOT_NAMES],
) -> (
    RobotName,
    Either<TextOrT<RobotToServerMessage>, NetworkStatus>,
) {
    let futures: Vec<_> = robots
        .iter_mut()
        .map(|robot| robot.async_read().boxed())
        .collect();
    let (result, index, _remaining) = select_all(futures).await;
    (RobotName::from(index), result)
}
