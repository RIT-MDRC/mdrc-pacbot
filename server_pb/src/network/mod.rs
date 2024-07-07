use crate::network::game_server::manage_game_server;
use crate::network::gui_clients::listen_for_gui_clients;
use crate::App;
use core_pb::messages::server_status::ServerStatus;
use core_pb::messages::{GameServerCommand, GuiToGameServerMessage};
use core_pb::pacbot_rs::game_state::GameState;
use futures_channel::mpsc::{unbounded, UnboundedReceiver, UnboundedSender};
use std::sync::{Arc, Mutex};

mod game_server;
mod gui_clients;
// todo mod robots;

pub struct Sockets {
    // pico_udp_tx: Option<UdpSocket>,
    // pico_udp_rx: Option<UdpSocket>,
    // pico_tcp: Option<TcpStream>,
    pub game_states: UnboundedReceiver<GameState>,
    pub game_server_commands: UnboundedSender<GameServerCommand>,

    pub commands_from_gui: UnboundedReceiver<GuiToGameServerMessage>,
    pub gui_outgoing: UnboundedSender<ServerStatus>,
}

impl Sockets {
    pub fn spawn(app: Arc<Mutex<App>>) -> Self {
        let (gs_inc_tx, gs_inc_rx) = unbounded();
        let (gs_out_tx, gs_out_rx) = unbounded();

        let (gui_incoming_tx, gui_incoming_rx) = unbounded();
        let (gui_outgoing_tx, gui_outgoing_rx) = unbounded();

        tokio::spawn(listen_for_gui_clients(
            app.clone(),
            gui_incoming_tx,
            gui_outgoing_rx,
        ));
        tokio::spawn(manage_game_server(app.clone(), gs_inc_tx, gs_out_rx));

        Sockets {
            game_states: gs_inc_rx,
            game_server_commands: gs_out_tx,

            commands_from_gui: gui_incoming_rx,
            gui_outgoing: gui_outgoing_tx,
        }
    }
}

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
