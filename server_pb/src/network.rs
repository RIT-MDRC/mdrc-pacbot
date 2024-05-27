use crate::App;
use core_pb::constants::GUI_LISTENER_PORT;
use core_pb::messages::GuiToGameServerMessage;
use std::net::{TcpListener, TcpStream, UdpSocket};
use tungstenite::stream::MaybeTlsStream;
use tungstenite::{accept, connect, Message, WebSocket};

#[derive(Default)]
pub struct Sockets {
    pico_udp_tx: Option<UdpSocket>,
    pico_udp_rx: Option<UdpSocket>,
    pico_tcp: Option<TcpStream>,

    game_server: Option<WebSocket<MaybeTlsStream<TcpStream>>>,

    gui_listener: Option<TcpListener>,
    gui_clients: Vec<WebSocket<TcpStream>>,
}

pub fn reconnect_sockets(app: &mut App) {
    // reconnect pico sockets
    // if let Some(ip) = &app.settings.pico.ip {
    //     if app.sockets.pico_udp_tx.is_none() {
    //         if let Ok(socket) = UdpSocket::bind(format!("0.0.0.0:{PICO_TX_PORT}")) {
    //             app.sockets.pico_udp_tx = Some(socket);
    //         }
    //     }
    //     if app.sockets.pico_udp_rx.is_none() {
    //         if let Ok(socket) = UdpSocket::bind(format!("0.0.0.0:{PICO_RX_PORT}")) {
    //             app.sockets.pico_udp_rx = Some(socket);
    //         }
    //     }
    //     if app.sockets.pico_tcp.is_none() {
    //         if let Ok(socket) = TcpStream::connect(format!("{}:{}", ip, app.settings.pico.tcp_port))
    //         {
    //             app.sockets.pico_tcp = Some(socket);
    //         }
    //     }
    // } else {
    //     app.sockets.pico_udp_tx = None;
    //     app.sockets.pico_udp_rx = None;
    //     app.sockets.pico_tcp = None;
    // }
    //
    // reconnect game server
    if app.settings.game_server.connect {
        let [a, b, c, d] = app.settings.game_server.ipv4;
        let port = app.settings.game_server.ws_port;
        if let Ok((socket, _)) = connect(format!("ws://{a}.{b}.{c}.{d}:{port}")) {
            app.sockets.game_server = Some(socket);
        }
    } else {
        app.sockets.game_server = None;
    }

    // accept new gui clients
    if let Some(server) = &mut app.sockets.gui_listener {
        while let Ok((stream, _)) = server.accept() {
            if let Ok(ws) = accept(stream) {
                app.sockets.gui_clients.push(ws);
            }
        }
    } else {
        if let Ok(listener) = TcpListener::bind(format!("0.0.0.0:{GUI_LISTENER_PORT}")) {
            app.sockets.gui_listener = Some(listener);
        }
    }

    // get rid of old clients
    app.sockets.gui_clients.retain(|x| x.can_read());

    // accept messages from gui clients
    let mut new_settings = None;
    for client in &mut app.sockets.gui_clients {
        match client.read() {
            Err(e) => eprintln!("Error reading from gui client: {e:?}"),
            Ok(Message::Text(t)) => eprintln!("Unexpected text message from gui client: {t:?}"),
            Ok(Message::Binary(bytes)) => {
                match bincode::serde::decode_from_slice::<GuiToGameServerMessage, _>(
                    &bytes,
                    bincode::config::standard(),
                ) {
                    Err(e) => eprintln!("Error decoding message from gui client: {e:?}"),
                    Ok((msg, _)) => match msg {
                        GuiToGameServerMessage::Settings(new) => {
                            new_settings = Some(new);
                        }
                    },
                }
            }
            m => eprintln!("Unexpected message from gui client: {m:?}"),
        }
    }
    if let Some(new) = new_settings {
        let old = app.settings.clone();
        app.update_settings(&old, &new);
        app.settings = new;
    }
}
