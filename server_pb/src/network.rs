use crate::App;
use std::net::{TcpListener, TcpStream, UdpSocket};
use tungstenite::stream::MaybeTlsStream;
use tungstenite::{accept, connect, WebSocket};

const GUI_LISTENER_PORT: u16 = 20010;
const PICO_TX_PORT: u16 = 20011;
const PICO_RX_PORT: u16 = 20012;

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
    // // reconnect game server
    // if let Some(ip) = &app.settings.game_server.ip {
    //     if let Ok((socket, _)) =
    //         connect(format!("ws://{}:{}", ip, app.settings.game_server.ws_port))
    //     {
    //         app.sockets.game_server = Some(socket);
    //     }
    // } else {
    //     app.sockets.game_server = None;
    // }
    //
    // // accept new gui clients
    // if let Some(server) = &mut app.sockets.gui_listener {
    //     while let Ok((stream, _)) = server.accept() {
    //         if let Ok(ws) = accept(stream) {
    //             app.sockets.gui_clients.push(ws);
    //         }
    //     }
    // } else {
    //     if let Ok(listener) = TcpListener::bind(format!("0.0.0.0:{GUI_LISTENER_PORT}")) {
    //         app.sockets.gui_listener = Some(listener);
    //     }
    // }
}
