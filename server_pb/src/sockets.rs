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
use core_pb::threaded_websocket::{Address, TcpStreamThreadableSocket, TextOrT, ThreadedSocket};
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

#[allow(clippy::large_enum_variant)]
pub enum Incoming {
    Status(NetworkStatus),
    Text(String),
    Bytes(Vec<u8>),
    SleepFinished,
    FromSimulation(SimulationToServerMessage),
    FromRobot(RobotToServerMessage),
    GuiConnected(u64),
    GuiDisconnected(u64),
    FromGui(GuiToServerMessage),
    FromGameServer(Vec<u8>),
}

#[allow(clippy::large_enum_variant)]
#[allow(dead_code)]
pub enum Outgoing {
    Address(Option<Address>),
    Text(String),
    RawBytes(Vec<u8>),
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
    pub async fn spawn() -> Self {
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
) -> Result<(), ()> {
    // game server
    let (gs_tx, gs_rx) = unbounded();
    let _ = tokio::spawn(manage_threaded_socket(
        GameServer,
        ThreadedSocket::new::<WebSocketStream<ConnectStream>, _, _, _, _>(
            "server[game_server]".to_string(),
            None,
            bin_encode,
            |bytes| Ok::<_, ()>(bytes.to_vec()),
        ),
        gs_rx,
        incoming_tx.clone(),
        Incoming::FromGameServer,
    ));

    // simulation
    let (sim_tx, sim_rx) = unbounded();
    let _ = tokio::spawn(manage_threaded_socket(
        Simulation,
        ThreadedSocket::with_name("server[simulation]".to_string()),
        sim_rx,
        incoming_tx.clone(),
        Incoming::FromSimulation,
    ));

    // robots
    let robots = RobotName::get_all().map(|name| {
        let incoming_tx = incoming_tx.clone();
        let (robot_tx, robot_rx) = unbounded();
        let _ = tokio::spawn(manage_threaded_socket(
            Robot(name),
            ThreadedSocket::new::<TcpStreamThreadableSocket, _, _, _, _>(
                format!("server[{name}]"),
                None,
                bin_encode,
                bin_decode,
            ),
            robot_rx,
            incoming_tx,
            Incoming::FromRobot,
        ));
        robot_tx
    });

    // gui clients
    let (gui_tx, gui_rx) = unbounded();
    let _ = tokio::spawn(manage_gui_clients(incoming_tx.clone(), gui_rx));

    let _ = tokio::spawn(repeat_sleep(incoming_tx.clone(), Duration::from_millis(40)));

    loop {
        let (dest, msg) = outgoing_rx.recv().await.map_err(|_| ())?;

        if let Outgoing::Address(addr) = msg {
            match dest {
                GameServer => gs_tx.send(Left(addr)).await.map_err(|_| ())?,
                Simulation => sim_tx.send(Left(addr)).await.map_err(|_| ())?,
                Robot(name) => robots[name as usize]
                    .send(Left(addr))
                    .await
                    .map_err(|_| ())?,
                _ => eprintln!("Invalid destination {dest:?} for address {addr:?}"),
            }
        } else if let Outgoing::Text(text) = msg {
            match dest {
                GameServer => gs_tx
                    .send(Right(TextOrT::Text(text)))
                    .await
                    .map_err(|_| ())?,
                Simulation => sim_tx
                    .send(Right(TextOrT::Text(text)))
                    .await
                    .map_err(|_| ())?,
                _ => eprintln!("Invalid destination {dest:?} for text {text}"),
            }
        } else {
            match (dest, msg) {
                (GameServer, Outgoing::ToGameServer(cmd)) => {
                    gs_tx.send(Right(TextOrT::T(cmd))).await.map_err(|_| ())?
                }
                (Simulation, Outgoing::ToSimulation(cmd)) => {
                    sim_tx.send(Right(TextOrT::T(cmd))).await.map_err(|_| ())?
                }
                (Robot(name), Outgoing::ToRobot(cmd)) => robots[name as usize]
                    .send(Right(TextOrT::T(cmd)))
                    .await
                    .map_err(|_| ())?,
                (Robot(name), Outgoing::RawBytes(data)) => robots[name as usize]
                    .send(Right(TextOrT::Bytes(data)))
                    .await
                    .map_err(|_| ())?,
                (GuiClients, Outgoing::ToGui(cmd)) => gui_tx.send(cmd).await.map_err(|_| ())?,
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

async fn repeat_sleep(
    incoming_tx: Sender<(Destination, Incoming)>,
    delay: Duration,
) -> Result<(), ()> {
    loop {
        sleep(delay).await;
        incoming_tx
            .send((NotApplicable, Incoming::SleepFinished))
            .await
            .map_err(|_| ())?;
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
) -> Result<(), ()> {
    loop {
        select! {
            msg = rx.recv() => {
                match msg.map_err(|_| ())? {
                    Left(addr) => threaded_socket.connect(addr),
                    Right(s) => threaded_socket.send(s),
                }
            },
            msg = threaded_socket.async_read() => {
                match msg {
                    Left(r) => {
                        match r {
                            TextOrT::T(r) => tx.send((destination, r_to_inc(r))).await.map_err(|_| ())?,
                            TextOrT::Text(text) => tx.send((destination, Incoming::Text(text))).await.map_err(|_| ())?,
                            TextOrT::Bytes(data) => tx.send((destination, Incoming::Bytes(data))).await.map_err(|_| ())?,
                        }
                    },
                    Right(status) => {
                        tx.send((destination, Incoming::Status(status))).await.map_err(|_| ())?;
                    },
                }
            }
        }
    }
}

async fn manage_gui_clients(
    tx: Sender<(Destination, Incoming)>,
    rx: Receiver<ServerToGuiMessage>,
) -> Result<(), ()> {
    let event_hub = simple_websockets::launch(GUI_LISTENER_PORT).map_err(|_| ())?;
    let mut responders: HashMap<u64, Responder> = HashMap::new();

    loop {
        select! {
            outgoing = rx.recv() => {
                let msg = Message::Binary(bin_encode(outgoing.map_err(|_| ())?).unwrap());
                for r in responders.values_mut() {
                    r.send(msg.clone());
                }
            }
            event = event_hub.poll_async() => {
                match event {
                    Event::Connect(id, responder) => {
                        responders.insert(id, responder);
                        tx.send((GuiClients, GuiConnected(id))).await.map_err(|_| ())?;
                    }
                    Event::Disconnect(id) => {
                        responders.remove(&id);
                        tx.send((GuiClients, GuiDisconnected(id))).await.map_err(|_| ())?;
                    }
                    Event::Message(id, msg) => match msg {
                        Message::Binary(bytes) => match bin_decode(&bytes) {
                            Ok(msg) => tx.send((GuiClients, Incoming::FromGui(msg))).await.map_err(|_| ())?,
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
