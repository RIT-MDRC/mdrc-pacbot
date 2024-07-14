use std::collections::HashMap;
use std::fmt::Debug;
use std::time::Duration;

use async_channel::{unbounded, Receiver, Sender};
use async_tungstenite::async_std::ConnectStream;
use async_tungstenite::WebSocketStream;
use futures_util::future::Either;
use futures_util::future::Either::{Left, Right};
use serde::de::DeserializeOwned;
use serde::Serialize;
use simple_websockets::{Event, Message, Responder};
use tokio::select;
use tokio::time::sleep;

use core_pb::constants::GUI_LISTENER_PORT;
use core_pb::messages::{
    GameServerCommand, GuiToServerMessage, NetworkStatus, RobotToServerMessage, ServerToGuiMessage,
    ServerToRobotMessage, ServerToSimulationMessage, SimulationToServerMessage,
};
use core_pb::names::RobotName;
use core_pb::threaded_websocket::{Address, TextOrT, ThreadedSocket};
use core_pb::{bin_decode, bin_encode};
use Destination::*;

use crate::sockets::Incoming::{GuiConnected, GuiDisconnected};

#[derive(Copy, Clone, Debug, PartialOrd, PartialEq)]
pub enum Destination {
    GuiClients,
    GameServer,
    Robot(RobotName),
    Simulation,
    NotApplicable,
}

pub enum Incoming {
    Status(NetworkStatus),
    Text(String),
    SleepFinished,
    FromSimulation(SimulationToServerMessage),
    FromRobot(RobotToServerMessage),
    GuiConnected(u64),
    GuiDisconnected(u64),
    FromGui(GuiToServerMessage),
    FromGameServer(Vec<u8>),
}

pub enum Outgoing {
    Address(Option<Address>),
    Text(String),
    ToSimulation(ServerToSimulationMessage),
    ToRobot(ServerToRobotMessage),
    ToGui(ServerToGuiMessage),
    ToGameServer(GameServerCommand),
}

// external api
pub struct Sockets {
    pub outgoing: Sender<(Destination, Outgoing)>,
    pub incoming: Receiver<(Destination, Incoming)>,
}

impl Sockets {
    pub fn spawn() -> Self {
        let (outgoing_tx, outgoing_rx) = unbounded();
        let (incoming_tx, incoming_rx) = unbounded();

        let _ = tokio::spawn(receive_outgoing(incoming_tx, outgoing_rx));

        Self {
            outgoing: outgoing_tx,
            incoming: incoming_rx,
        }
    }
}

async fn receive_outgoing(
    incoming_tx: Sender<(Destination, Incoming)>,
    outgoing_rx: Receiver<(Destination, Outgoing)>,
) {
    // game server
    let (gs_tx, gs_rx) = unbounded();
    let _ = tokio::spawn(manage_threaded_socket(
        GameServer,
        ThreadedSocket::new::<WebSocketStream<ConnectStream>, _, _, _, _>(
            "server[game_server]".to_string(),
            None,
            bin_encode,
            |bytes| Ok::<_, ()>(bytes.iter().copied().collect()),
        ),
        gs_rx,
        incoming_tx.clone(),
        |msg| Incoming::FromGameServer(msg),
    ));

    // simulation
    let (sim_tx, sim_rx) = unbounded();
    let _ = tokio::spawn(manage_threaded_socket(
        Simulation,
        ThreadedSocket::with_name("server[simulation]".to_string()),
        sim_rx,
        incoming_tx.clone(),
        |msg| Incoming::FromSimulation(msg),
    ));

    // robots
    let robots = RobotName::get_all().map(|name| {
        let (robot_tx, robot_rx) = unbounded();
        let _ = tokio::spawn(manage_threaded_socket(
            Robot(name),
            ThreadedSocket::with_name(format!("server[{name}]")),
            robot_rx,
            incoming_tx.clone(),
            |msg| Incoming::FromRobot(msg),
        ));
        robot_tx
    });

    // gui clients
    let (gui_tx, gui_rx) = unbounded();
    let _ = tokio::spawn(manage_gui_clients(incoming_tx.clone(), gui_rx));

    let _ = tokio::spawn(repeat_sleep(incoming_tx.clone(), Duration::from_millis(40)));

    loop {
        let (dest, msg) = outgoing_rx.recv().await.unwrap();

        if let Outgoing::Address(addr) = msg {
            match dest {
                GameServer => gs_tx.send(Left(addr)).await.unwrap(),
                Simulation => sim_tx.send(Left(addr)).await.unwrap(),
                Robot(name) => robots[name as usize].send(Left(addr)).await.unwrap(),
                _ => eprintln!("Invalid destination {dest:?} for address {addr:?}"),
            }
        } else if let Outgoing::Text(text) = msg {
            match dest {
                GameServer => gs_tx.send(Right(TextOrT::Text(text))).await.unwrap(),
                Simulation => sim_tx.send(Right(TextOrT::Text(text))).await.unwrap(),
                _ => eprintln!("Invalid destination {dest:?} for text {text}"),
            }
        } else {
            match (dest, msg) {
                (GameServer, Outgoing::ToGameServer(cmd)) => {
                    gs_tx.send(Right(TextOrT::T(cmd))).await.unwrap()
                }
                (Simulation, Outgoing::ToSimulation(cmd)) => {
                    sim_tx.send(Right(TextOrT::T(cmd))).await.unwrap()
                }
                (Robot(name), Outgoing::ToRobot(cmd)) => robots[name as usize]
                    .send(Right(TextOrT::T(cmd)))
                    .await
                    .unwrap(),
                (GuiClients, Outgoing::ToGui(cmd)) => gui_tx.send(cmd).await.unwrap(),
                (NotApplicable, _)
                | (GameServer, _)
                | (Simulation, _)
                | (Robot(_), _)
                | (GuiClients, _) => {
                    eprintln!("Invalid destination: {dest:?}")
                }
            }
        }
    }
}

async fn repeat_sleep(incoming_tx: Sender<(Destination, Incoming)>, delay: Duration) {
    loop {
        sleep(delay).await;
        incoming_tx
            .send((NotApplicable, Incoming::SleepFinished))
            .await
            .unwrap();
    }
}

async fn manage_threaded_socket<
    S: Debug + Serialize + Send + 'static,
    R: Debug + DeserializeOwned + Send + 'static,
    F: Fn(R) -> Incoming,
>(
    destination: Destination,
    mut threaded_socket: ThreadedSocket<S, R>,
    rx: Receiver<Either<Option<Address>, TextOrT<S>>>,
    tx: Sender<(Destination, Incoming)>,
    r_to_inc: F,
) {
    loop {
        select! {
            msg = rx.recv() => {
                match msg.unwrap() {
                    Left(addr) => threaded_socket.connect(addr),
                    Right(s) => threaded_socket.send(s),
                }
            },
            msg = threaded_socket.async_read() => {
                match msg {
                    Left(r) => {
                        match r {
                            TextOrT::T(r) => tx.send((destination, r_to_inc(r))).await.unwrap(),
                            TextOrT::Text(text) => tx.send((destination, Incoming::Text(text))).await.unwrap(),
                        }
                    },
                    Right(status) => {
                        tx.send((destination, Incoming::Status(status))).await.unwrap();
                    },
                }
            }
        }
    }
}

async fn manage_gui_clients(tx: Sender<(Destination, Incoming)>, rx: Receiver<ServerToGuiMessage>) {
    let event_hub = simple_websockets::launch(GUI_LISTENER_PORT).unwrap();
    let mut responders: HashMap<u64, Responder> = HashMap::new();

    loop {
        select! {
            outgoing = rx.recv() => {
                let msg = Message::Binary(bin_encode(outgoing.unwrap()).unwrap());
                for (_, r) in &mut responders {
                    r.send(msg.clone());
                }
            }
            event = event_hub.poll_async() => {
                match event {
                    Event::Connect(id, responder) => {
                        responders.insert(id, responder);
                        tx.send((GuiClients, GuiConnected(id))).await.unwrap();
                    }
                    Event::Disconnect(id) => {
                        responders.remove(&id);
                        tx.send((GuiClients, GuiDisconnected(id))).await.unwrap();
                    }
                    Event::Message(id, msg) => match msg {
                        Message::Binary(bytes) => match bin_decode(&bytes) {
                            Ok(msg) => tx.send((GuiClients, Incoming::FromGui(msg))).await.unwrap(),
                            Err(e) => eprintln!(
                                "Failed to decode bytes from gui client {id} ({} bytes): {e:?}",
                                bytes.len()
                            ),
                        },
                        Message::Text(text) => eprintln!("Unexpected text from gui client {id}: {text}"),
                    },
                }
            }
        }
    }
}
