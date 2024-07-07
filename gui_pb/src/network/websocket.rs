#[cfg(not(target_arch = "wasm32"))]
use std::io;
#[cfg(not(target_arch = "wasm32"))]
use std::net::TcpStream;
use std::sync::mpsc::{channel, Receiver};
use tungstenite::Message;
#[cfg(not(target_arch = "wasm32"))]
use tungstenite::{client, ClientHandshake, HandshakeError, WebSocket};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use web_sys::{ErrorEvent, MessageEvent, WebSocket};

#[cfg(not(target_arch = "wasm32"))]
pub struct CrossPlatformWebsocket {
    ws: WebSocket<TcpStream>,
}

#[cfg(target_arch = "wasm32")]
pub struct CrossPlatformWebsocket {
    ok: bool,
    ws: WebSocket,
    messages: Receiver<Result<Message, WebsocketError>>,
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug)]
#[allow(dead_code)]
pub enum WebsocketError {
    IoError(&'static str, io::Error),
    HandshakeError(HandshakeError<ClientHandshake<TcpStream>>),
    TungsteniteError(tungstenite::Error),
}

#[cfg(target_arch = "wasm32")]
#[derive(Debug)]
pub enum WebsocketError {
    JsValue(JsValue),
    WouldBlock,
    ErrorEvent(ErrorEvent),
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

#[cfg(target_arch = "wasm32")]
use crate::log;

#[cfg(target_arch = "wasm32")]
macro_rules! console_log {
    ($($t:tt)*) => (log(&format_args!($($t)*).to_string()))
}

#[cfg(target_arch = "wasm32")]
/// https://rustwasm.github.io/wasm-bindgen/examples/websockets.html
impl CrossPlatformWebsocket {
    pub fn connect(addr: String) -> Result<Self, WebsocketError> {
        let (msg_tx, msg_rx) = channel();
        let tx2 = msg_tx.clone();

        let ws = WebSocket::new(&("ws://".to_string() + &addr))
            .map_err(|e| WebsocketError::JsValue(e))?;

        let onmessage_callback = Closure::<dyn FnMut(_)>::new(move |e: MessageEvent| {
            // Handle difference Text/Binary,...
            if let Ok(abuf) = e.data().dyn_into::<js_sys::ArrayBuffer>() {
                console_log!("message event, received arraybuffer: {:?}", abuf);
                let array = js_sys::Uint8Array::new(&abuf);
                msg_tx.send(Ok(Message::Binary(array.to_vec()))).unwrap();
            } else if let Ok(blob) = e.data().dyn_into::<web_sys::Blob>() {
                console_log!("message event, received blob: {:?}", blob);
                let fr = web_sys::FileReader::new().unwrap();
                let fr_c = fr.clone();
                let msg_tx_c = msg_tx.clone();
                // create onLoadEnd callback
                let onloadend_cb =
                    Closure::<dyn FnMut(_)>::new(move |_e: web_sys::ProgressEvent| {
                        let array = js_sys::Uint8Array::new(&fr_c.result().unwrap());
                        let len = array.byte_length() as usize;
                        console_log!("Blob received {len} bytes");
                        msg_tx_c.send(Ok(Message::Binary(array.to_vec()))).unwrap();
                    });
                fr.set_onloadend(Some(onloadend_cb.as_ref().unchecked_ref()));
                fr.read_as_array_buffer(&blob).expect("blob not readable");
                onloadend_cb.forget();
            } else if let Ok(txt) = e.data().dyn_into::<js_sys::JsString>() {
                console_log!("message event, received Text: {:?}", txt);
            } else {
                console_log!("message event, received Unknown: {:?}", e.data());
            }
        });

        // set message event handler on WebSocket
        ws.set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));
        // forget the callback to keep it alive
        onmessage_callback.forget();

        let onerror_callback = Closure::<dyn FnMut(_)>::new(move |e: ErrorEvent| {
            console_log!("error event: {:?}", e);
            tx2.send(Err(WebsocketError::ErrorEvent(e))).unwrap();
        });
        ws.set_onerror(Some(onerror_callback.as_ref().unchecked_ref()));
        onerror_callback.forget();

        Ok(Self {
            ws,
            ok: true,
            messages: msg_rx,
        })
    }

    pub fn can_read(&self) -> bool {
        self.ok
    }

    pub fn read(&mut self) -> Result<Message, WebsocketError> {
        match self.messages.try_recv() {
            Ok(Ok(x)) => Ok(x),
            Ok(Err(e)) => {
                self.ok = false;
                Err(e)
            }
            Err(_) => Err(WebsocketError::WouldBlock),
        }
    }

    pub fn send(&mut self, bytes: Vec<u8>) -> Result<(), WebsocketError> {
        self.ws
            .send_with_u8_array(&bytes)
            .map_err(|e| WebsocketError::JsValue(e))
    }
}
