#[cfg(not(target_arch = "wasm32"))]
use std::io;
#[cfg(not(target_arch = "wasm32"))]
use std::net::TcpStream;
#[cfg(not(target_arch = "wasm32"))]
use tungstenite::{client, ClientHandshake, HandshakeError, Message, WebSocket};

pub struct CrossPlatformWebsocket {
    #[cfg(not(target_arch = "wasm32"))]
    ws: WebSocket<TcpStream>,
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug)]
#[allow(dead_code)]
pub enum WebsocketError {
    IoError(&'static str, io::Error),
    HandshakeError(HandshakeError<ClientHandshake<TcpStream>>),
    TungsteniteError(tungstenite::Error),
}

#[cfg(not(target_arch = "wasm32"))]
impl CrossPlatformWebsocket {
    pub fn connect(addr: String) -> Result<Self, WebsocketError> {
        let stream = TcpStream::connect(addr.clone())
            .map_err(|e| WebsocketError::IoError("error during TcpStream initialization", e))?;

        stream
            .set_nonblocking(true)
            .map_err(|e| WebsocketError::IoError("couldn't set stream to nonblocking", e))?;

        match client("ws://".to_string() + &addr, stream) {
            Ok((socket, _)) => {
                return Ok(Self { ws: socket });
            }
            Err(HandshakeError::Interrupted(mid)) => {
                let mut mid = mid;
                loop {
                    mid = match mid.handshake() {
                        Ok((socket, _)) => {
                            return Ok(Self { ws: socket });
                        }
                        Err(HandshakeError::Interrupted(mid_next)) => mid_next,
                        Err(HandshakeError::Failure(e)) => {
                            return Err(WebsocketError::HandshakeError(HandshakeError::Failure(e)))
                        }
                    }
                }
            }
            Err(HandshakeError::Failure(e)) => {
                return Err(WebsocketError::HandshakeError(HandshakeError::Failure(e)))
            }
        }
    }

    pub fn can_read(&self) -> bool {
        self.ws.can_read()
    }

    pub fn read(&mut self) -> Result<Message, WebsocketError> {
        self.ws
            .read()
            .map_err(|e| WebsocketError::TungsteniteError(e))
    }

    pub fn send(&mut self, bytes: Vec<u8>) -> Result<(), WebsocketError> {
        self.ws
            .send(Message::Binary(bytes))
            .map_err(|e| WebsocketError::TungsteniteError(e))
    }
}
