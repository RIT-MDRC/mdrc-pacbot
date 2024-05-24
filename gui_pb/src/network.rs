use std::net::TcpStream;
use tungstenite::WebSocket;

pub struct NetworkData {
    game_server_socket: Option<WebSocket<TcpStream>>,
}
