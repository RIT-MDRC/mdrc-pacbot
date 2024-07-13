use std::collections::HashMap;
use std::pin::pin;
use std::time::{Duration, Instant};

use async_tungstenite::async_std::ConnectStream;
use async_tungstenite::WebSocketStream;
use futures_util::future::Either;
use futures_util::select;
use futures_util::FutureExt;
use simple_websockets::{Event, Message};
use tokio::time::sleep;

use core_pb::constants::GUI_LISTENER_PORT;
use core_pb::messages::settings::PacbotSettings;
use core_pb::messages::{
    GuiToServerMessage, NetworkStatus, ServerToGuiMessage, ServerToSimulationMessage,
    GAME_SERVER_MAGIC_NUMBER,
};
use core_pb::pacbot_rs::game_state::GameState;
use core_pb::threaded_websocket::{TextOrT, ThreadedSocket};
use core_pb::{bin_decode, bin_encode};

use crate::App;

// todo mod robots;

pub async fn manage_network() {
    let mut app = App {
        status: Default::default(),
        settings: Default::default(),

        last_status_update: Instant::now(),
        settings_update_needed: false,

        client_http_host_process: None,
        sim_game_engine_process: None,

        game_server_socket: ThreadedSocket::new::<WebSocketStream<ConnectStream>, _, _, _, _>(
            None,
            bin_encode,
            |bytes| Ok::<_, ()>(bytes.iter().copied().collect()),
        ),
        simulation_socket: ThreadedSocket::default(),

        gui_clients: HashMap::new(),

        grid: Default::default(),
    };

    let gui_client_event_hub = simple_websockets::launch(GUI_LISTENER_PORT).unwrap();

    println!("Listening on 0.0.0.0:{GUI_LISTENER_PORT}");

    // apply default settings to the app
    app.update_settings(&PacbotSettings::default(), PacbotSettings::default())
        .await;

    loop {
        // if necessary, send updated settings to clients
        if app.settings_update_needed {
            app.settings_update_needed = false;
            let msg = Message::Binary(
                bin_encode(ServerToGuiMessage::Settings(app.settings.clone())).unwrap(),
            );
            for (_, r) in &mut app.gui_clients {
                r.send(msg.clone());
            }
        }
        // if necessary, send updated status to clients
        if app.last_status_update.elapsed() > Duration::from_millis(40) {
            app.last_status_update = Instant::now();
            let msg = Message::Binary(
                bin_encode(ServerToGuiMessage::Status(app.status.clone())).unwrap(),
            );
            for (_, r) in &mut app.gui_clients {
                r.send(msg.clone());
            }
        }

        // note: this pin! is why gui_client_event_hub is not part of App
        let gui_event_fut = pin!(gui_client_event_hub.poll_async());

        select! {
            // we should send a new status message every so often
            _ = sleep(Duration::from_millis(100)).fuse() => {}
            // handle connections/messages from GUIs
            gui_event = gui_event_fut.fuse() => {
                handle_gui_event(&mut app, gui_event).await;
            }
            // handle status/messages from game server
            game_server_msg = app.game_server_socket.async_read().fuse() => {
                match game_server_msg {
                    Either::Left(TextOrT::Text(text)) => eprintln!("Unexpected text from game server: {text}"),
                    Either::Left(TextOrT::T(bytes)) => {
                        if bytes == GAME_SERVER_MAGIC_NUMBER.to_vec() {
                            app.status.advanced_game_server = true;
                        } else {
                        let mut g = GameState::new();
                            match g.update(&bytes) {
                                Ok(()) => app.status.game_state = g,
                                Err(e) => eprintln!("Error updating game state: {e:?}"),
                            }
                        }
                    }
                    Either::Right(new_status) => {
                        if new_status != NetworkStatus::Connected {
                            // assume the game server is not advanced until proven otherwise
                            app.status.advanced_game_server = false;
                        }
                        app.status.game_server_connection_status = new_status
                    }
                }
            }
            // handle status/messages from simulation
            simulation_msg = app.simulation_socket.async_read().fuse() => {
                match simulation_msg {
                    Either::Left(msg) => println!("Message from simulation: {msg:?}"),
                    Either::Right(new_status) => {
                        app.status.simulation_connection_status = new_status
                    }
                }
            }
        }
    }
}

async fn handle_gui_event(app: &mut App, event: Event) {
    match event {
        Event::Connect(id, responder) => {
            println!("Gui client #{id} connected");
            app.status.gui_clients += 1;
            println!("{} gui client(s) are connected", app.status.gui_clients);
            app.settings_update_needed = true;
            app.gui_clients.insert(id, responder);
        }
        Event::Disconnect(id) => {
            println!("Gui client #{id} disconnected");
            app.status.gui_clients -= 1;
            println!("{} gui client(s) remaining", app.status.gui_clients);
            app.gui_clients.remove(&id);
        }
        Event::Message(id, msg) => {
            println!("Received a message from gui client {}", id);
            match msg {
                Message::Binary(bytes) => match bin_decode(&bytes) {
                    Ok(msg) => match msg {
                        GuiToServerMessage::Settings(settings) => {
                            let old_settings = app.settings.clone();
                            app.update_settings(&old_settings, settings).await;
                        }
                        GuiToServerMessage::GameServerCommand(command) => match command.text() {
                            Some(text) => {
                                app.game_server_socket
                                    .async_send(TextOrT::Text(text.into()))
                                    .await
                            }
                            None => {
                                if app.status.advanced_game_server {
                                    app.game_server_socket.async_send(TextOrT::T(command)).await;
                                }
                            }
                        },
                        GuiToServerMessage::RobotVelocity(robot, vel) => {
                            app.simulation_socket.send(TextOrT::T(
                                ServerToSimulationMessage::RobotVelocity(robot, vel),
                            ))
                        }
                    },
                    Err(e) => eprintln!(
                        "Error decoding message from {id}: {e:?}, {} bytes",
                        bytes.len()
                    ),
                },
                Message::Text(text) => {
                    eprintln!("Received strange message from gui client {id}: {text}")
                }
            }
        }
    }
}

// async fn handle_game_server_message(app: &mut App, msg: )

// pub fn reconnect_sockets(app: &mut App) {
//     // reconnect pico sockets
//     // if let Some(ip) = &app.settings.pico.ip {
//     //     if app.sockets.pico_udp_tx.is_none() {
//     //         if let Ok(socket) = UdpSocket::bind(format!("0.0.0.0:{PICO_TX_PORT}")) {
//     //             app.sockets.pico_udp_tx = Some(socket);
//     //         }
//     //     }
//     //     if app.sockets.pico_udp_rx.is_none() {
//     //         if let Ok(socket) = UdpSocket::bind(format!("0.0.0.0:{PICO_RX_PORT}")) {
//     //             app.sockets.pico_udp_rx = Some(socket);
//     //         }
//     //     }
//     //     if app.sockets.pico_tcp.is_none() {
//     //         if let Ok(socket) = TcpStream::connect(format!("{}:{}", ip, app.settings.pico.tcp_port))
//     //         {
//     //             app.sockets.pico_tcp = Some(socket);
//     //         }
//     //     }
//     // } else {
//     //     app.sockets.pico_udp_tx = None;
//     //     app.sockets.pico_udp_rx = None;
//     //     app.sockets.pico_tcp = None;
//     // }
//
//     let status = bincode::serde::encode_to_vec(app.status, bincode::config::standard())
//         .expect("Failed to encode status");
//
//     // accept messages from gui clients, send status
//     let mut new_settings = None;
//     for client in &mut app.sockets.gui_clients {
//         match client.read() {
//             Err(tungstenite::Error::Io(e)) if e.kind() == io::ErrorKind::WouldBlock => {}
//             Err(e) => eprintln!("Error reading from gui client: {e:?}"),
//             Ok(Message::Text(t)) => eprintln!("Unexpected text message from gui client: {t:?}"),
//             Ok(Message::Binary(bytes)) => {
//                 match bincode::serde::decode_from_slice::<GuiToGameServerMessage, _>(
//                     &bytes,
//                     bincode::config::standard(),
//                 ) {
//                     Err(e) => eprintln!("Error decoding message from gui client: {e:?}"),
//                     Ok((msg, _)) => match msg {
//                         GuiToGameServerMessage::Settings(new) => {
//                             new_settings = Some(new);
//                         }
//                     },
//                 }
//             }
//             m => eprintln!("Unexpected message from gui client: {m:?}"),
//         }
//         // send server status
//         match client.send(Message::Binary(status.clone())) {
//             Ok(()) => {}
//             Err(e) => eprintln!("Error sending to gui client: {e:?}"),
//         }
//     }
//     if let Some(new) = new_settings {
//         let old = app.settings.clone();
//         app.update_settings(&old, &new);
//         app.settings = new;
//     }
// }
