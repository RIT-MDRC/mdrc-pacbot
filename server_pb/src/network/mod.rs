use std::sync::{Arc, Mutex};

use core_pb::messages::server_status::ServerStatus;
use core_pb::messages::{GameServerCommand, GuiToGameServerMessage};
use futures_channel::mpsc::{unbounded, UnboundedReceiver, UnboundedSender};
use futures_util::future::join;

use crate::network::game_server::manage_game_server;
use crate::network::gui_clients::listen_for_gui_clients;
use crate::App;

mod game_server;
mod gui_clients;

pub struct Sockets {
    // pico_udp_tx: Option<UdpSocket>,
    // pico_udp_rx: Option<UdpSocket>,
    // pico_tcp: Option<TcpStream>,
    pub game_server_commands: UnboundedSender<GameServerCommand>,

    pub gui_incoming: UnboundedReceiver<GuiToGameServerMessage>,
    pub gui_outgoing: UnboundedSender<ServerStatus>,
}

pub struct Network {
    gs_rx: UnboundedReceiver<GameServerCommand>,

    gui_incoming: UnboundedSender<GuiToGameServerMessage>,
    gui_outgoing: UnboundedReceiver<ServerStatus>,
}

impl Network {
    pub fn new() -> (Self, Sockets) {
        let (gs_tx, gs_rx) = unbounded();

        let (gui_incoming_tx, gui_incoming_rx) = unbounded();
        let (gui_outgoing_tx, gui_outgoing_rx) = unbounded();

        let sockets = Sockets {
            game_server_commands: gs_tx,

            gui_incoming: gui_incoming_rx,
            gui_outgoing: gui_outgoing_tx,
        };

        let s = Self {
            gs_rx,

            gui_incoming: gui_incoming_tx,
            gui_outgoing: gui_outgoing_rx,
        };

        (s, sockets)
    }

    pub async fn run(self, app: Arc<Mutex<App>>) -> ! {
        join(
            listen_for_gui_clients(self.gui_incoming, self.gui_outgoing),
            manage_game_server(app.clone(), self.gs_rx),
        )
        .await;

        unreachable!("All network futures ended! This shouldn't happen.")
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
