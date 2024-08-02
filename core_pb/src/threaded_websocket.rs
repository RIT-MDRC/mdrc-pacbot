#![allow(async_fn_in_trait)]
#![warn(missing_docs)]
//! See [`ThreadedSocket`], a simple poll-based wrapper around a socket (websocket or TCP) connection
//! that runs in a separate thread

use crate::messages::NetworkStatus;
use crate::{bin_decode, bin_encode, console_log};
use async_channel::{unbounded, Receiver, Sender};
use async_std::task::sleep;
use futures::executor::block_on;
use futures::future::{select, Either};
use futures::select;
use serde::de::DeserializeOwned;
use serde::Serialize;
#[allow(unused)]
use std::any::TypeId;
use std::fmt::Debug;
use std::future;
use std::pin::pin;
use std::time::Duration;
#[cfg(not(target_arch = "wasm32"))]
use {
    async_std::net::TcpStream,
    async_tungstenite::async_std::ConnectStream,
    async_tungstenite::tungstenite::Message,
    async_tungstenite::WebSocketStream,
    futures::SinkExt,
    futures::StreamExt,
    futures::{AsyncReadExt, AsyncWriteExt, FutureExt},
};
#[cfg(target_arch = "wasm32")]
use {
    futures::FutureExt,
    wasm_bindgen_futures::spawn_local,
    web_sys::wasm_bindgen::closure::Closure,
    web_sys::wasm_bindgen::JsCast,
    web_sys::WebSocket,
    web_sys::{js_sys, ErrorEvent, MessageEvent},
};

/// ipv4 address with port number
pub type Address = ([u8; 4], u16);

/// Simple poll-based wrapper around a socket (websocket or TCP) connection that runs in a separate thread
///
/// Associated methods return immediately even when (for some) the operation might not be completed.
/// Supports normal std environments as well as WASM.
///
/// Use [`ThreadedSocket::default`] for a websocket, or [`ThreadedSocket::new`] to specify another
/// socket type.
///
/// # Usage
///
/// ```no_run
/// use core_pb::threaded_websocket::{TextOrT, ThreadedSocket};
/// use std::thread::sleep;
/// use std::time::Duration;
/// use core_pb::messages::NetworkStatus;
///
/// // initialization
/// // by default, doesn't connect to anything
/// let mut connection: ThreadedSocket<usize, usize> = ThreadedSocket::with_name("test connection".to_string());
/// // try to connect to an address (with infinite retries)
/// connection.connect(Some(([127, 0, 0, 1], 20_000)));
/// // wait until connected
/// while connection.status() != NetworkStatus::Connected {
///     sleep(Duration::from_millis(100))
/// }
/// // send a message to the server (returns immediately)
/// connection.send(TextOrT::T(1));
/// connection.send(TextOrT::Text("hello".to_string()));
/// // wait for a message
/// loop {
///     // note: read() never blocks, and only returns
///     // some message when one is available
///     if let Some(msg) = connection.read() {
///         println!("Got a message: {msg:?}");
///         break;
///     }
///     sleep(Duration::from_millis(100))
/// }
/// // stop connecting to the server
/// connection.connect(None);
/// ```
pub struct ThreadedSocket<SendType: Debug, ReceiveType: Debug> {
    status: NetworkStatus,

    addr_sender: Sender<Option<Address>>,
    sender: Sender<TextOrT<SendType>>,
    status_receiver: Receiver<NetworkStatus>,
    receiver: Receiver<TextOrT<ReceiveType>>,
}

/// Represents data that is either the given type, or text
///
/// Used for websockets; the type is transferred using serialization in bytes, text is sent
/// as regular text
#[derive(Debug)]
pub enum TextOrT<T: Debug> {
    /// text
    Text(String),
    /// the type
    T(T),
    /// raw bytes
    Bytes(Vec<u8>),
}

impl<
        SendType: Serialize + Debug + Send + 'static,
        ReceiveType: DeserializeOwned + Debug + Send + 'static,
    > ThreadedSocket<SendType, ReceiveType>
{
    /// Specify an address to connect to (or None to suspend current connection and future attempts)
    ///
    /// See [`ThreadedSocket`] for full usage example
    pub fn connect(&mut self, addr: Option<Address>) {
        block_on(self.addr_sender.send(addr)).expect("ThreadedSocket address sender is closed");
    }

    /// Fetch the latest information about the status of the connection
    ///
    /// See [`ThreadedSocket`] for full usage example
    pub fn status(&mut self) -> NetworkStatus {
        while let Ok(status) = self.status_receiver.try_recv() {
            self.status = status
        }
        self.status
    }

    /// Queue something to be sent to the socket
    ///
    /// If the connection is not available, the data will be discarded
    ///
    /// See [`ThreadedSocket`] for full usage example
    pub fn send(&mut self, data: TextOrT<SendType>) {
        block_on(self.sender.send(data)).expect("ThreadedSocket data sender is closed");
    }

    /// Queue something to be sent to the socket (blocking await)
    ///
    /// If the connection is not available, the data will be discarded
    ///
    /// See [`ThreadedSocket`] for full usage example
    pub async fn async_send(&mut self, data: TextOrT<SendType>) {
        self.sender
            .send(data)
            .await
            .expect("ThreadedSocket data sender is closed");
    }

    /// Read new data from the socket, if it is available
    ///
    /// Expects to be called frequently
    ///
    /// See [`ThreadedSocket`] for full usage example
    pub fn read(&mut self) -> Option<TextOrT<ReceiveType>> {
        self.status();
        self.receiver.try_recv().ok()
    }

    /// Read new data from the socket (blocking await)
    ///
    /// Expects to be called frequently
    ///
    /// See [`ThreadedSocket`] for full usage example
    pub async fn async_read(&mut self) -> Either<TextOrT<ReceiveType>, NetworkStatus> {
        self.status();

        let data = self.receiver.recv();
        let status = self.status_receiver.recv();

        return match select(pin!(data), pin!(status)).await {
            Either::Left(x) => Either::Left(x.0.expect("ThreadedSocket data receiver is closed")),
            Either::Right(x) => {
                Either::Right(x.0.expect("ThreadedSocket status receiver is closed"))
            }
        };
    }

    /// Create a new [`ThreadedSocket`]
    ///
    /// # Usage
    ///
    /// ```no_run
    /// use core_pb::threaded_websocket::ThreadedSocket;
    ///
    /// // websocket for either std environment or WASM
    /// // let websocket = ThreadedSocket::with_name("test connection".to_string());
    /// // tcp socket to be supported in the future
    /// ```
    ///
    /// See [`ThreadedSocket`] for full usage example
    pub fn new<
        SocketType: ThreadableSocket<SendType, ReceiveType> + 'static,
        Serializer: Fn(SendType) -> Result<Vec<u8>, SerializeResult> + Send + 'static,
        SerializeResult: Debug + 'static,
        Deserializer: Fn(&[u8]) -> Result<ReceiveType, DeserializeResult> + Send + 'static,
        DeserializeResult: Debug + 'static,
    >(
        name: String,
        addr: Option<Address>,
        serializer: Serializer,
        deserializer: Deserializer,
    ) -> Self {
        console_log!("[threaded_websocket : {name}] Socket created with initial address {addr:?}");

        let (addr_sender, addr_rx) = unbounded();
        let (sender, sender_rx) = unbounded();
        let (status_tx, status_receiver) = unbounded();
        let (receiver_tx, receiver) = unbounded();

        block_on(addr_sender.send(addr)).unwrap();

        let name2 = name.clone();
        #[cfg(target_arch = "wasm32")]
        spawn_local(run_socket_forever::<_, _, SocketType, _, _, _, _>(
            name2,
            addr_rx,
            sender_rx,
            status_tx,
            receiver_tx,
            serializer,
            deserializer,
        ));
        #[cfg(not(target_arch = "wasm32"))]
        std::thread::spawn(|| {
            block_on(run_socket_forever::<_, _, SocketType, _, _, _, _>(
                name2,
                addr_rx,
                sender_rx,
                status_tx,
                receiver_tx,
                serializer,
                deserializer,
            ))
        });

        Self {
            status: NetworkStatus::NotConnected,

            addr_sender,
            sender,
            status_receiver,
            receiver,
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl<
        SendType: Serialize + Debug + Send + 'static,
        ReceiveType: DeserializeOwned + Debug + Send + 'static,
    > ThreadedSocket<SendType, ReceiveType>
{
    /// Create a new ThreadedSocket with a name for logging
    pub fn with_name(name: String) -> Self {
        Self::new::<WebSocketStream<ConnectStream>, _, _, _, _>(name, None, bin_encode, bin_decode)
    }
}

#[cfg(target_arch = "wasm32")]
impl<
        SendType: Serialize + Debug + Send + 'static,
        ReceiveType: DeserializeOwned + Debug + Send + 'static,
    > ThreadedSocket<SendType, ReceiveType>
{
    /// Create a new ThreadedSocket with a name for logging
    pub fn with_name(name: String) -> Self {
        Self::new::<WasmThreadableWebsocket, _, _, _, _>(name, None, bin_encode, bin_decode)
    }
}

/// Represents a type that is compatible with [`ThreadedSocket`]
pub trait ThreadableSocket<SendType, ReceiveType>: Sized {
    /// Try to connect to the address
    ///
    /// Do not do any retries, fail as soon as possible
    async fn my_connect(addr: Address) -> Result<Self, ()>;

    /// Send the data to the socket
    ///
    /// If this is impossible, simply drop the data
    async fn my_send(&mut self, data: TextOrT<Vec<u8>>);

    /// Try to read from the socket
    ///
    /// If the connection is no longer available, return Err(())
    async fn my_read(&mut self) -> Result<TextOrT<Vec<u8>>, ()>;

    /// Close the socket
    async fn my_close(self);
}

/// A future that yields the next message from the socket, or never if the socket is None
async fn socket_read_fut<T: ThreadableSocket<S, R>, S, R>(
    socket: &mut Option<T>,
) -> Result<TextOrT<Vec<u8>>, ()> {
    if let Some(socket) = socket {
        socket.my_read().await
    } else {
        future::pending().await
    }
}

/// Runs on a separate thread to babysit the socket
async fn run_socket_forever<
    OutgoingType: Serialize + Debug,
    IncomingType: DeserializeOwned + Debug,
    SocketType: ThreadableSocket<OutgoingType, IncomingType>,
    Serializer: Fn(OutgoingType) -> Result<Vec<u8>, SerializeResult>,
    SerializeResult: Debug,
    Deserializer: Fn(&[u8]) -> Result<IncomingType, DeserializeResult>,
    DeserializeResult: Debug,
>(
    name: String,
    addresses: Receiver<Option<Address>>,
    data_outgoing: Receiver<TextOrT<OutgoingType>>,
    statuses: Sender<NetworkStatus>,
    data_incoming: Sender<TextOrT<IncomingType>>,
    serializer: Serializer,
    deserializer: Deserializer,
) {
    let mut addr: Option<Address> = None;
    let mut socket: Option<SocketType> = None;

    loop {
        if socket.is_none() {
            if let Some(address) = addr {
                console_log!("[threaded_websocket : {name}] Connecting to {addr:?}...");
                statuses.send(NetworkStatus::Connecting).await.unwrap();
                select! {
                    new_addr = addresses.recv().fuse() => {
                        console_log!("[threaded_websocket : {name}] Address changed from {addr:?} to {new_addr:?}");
                        statuses.send(NetworkStatus::NotConnected).await.unwrap();
                        addr = new_addr.unwrap();
                    }
                    conn = SocketType::my_connect(address).fuse() => {
                        match conn {
                            Ok(s) => {
                                console_log!("[threaded_websocket : {name}] Connected to {addr:?}");
                                statuses.send(NetworkStatus::Connected).await.unwrap();
                                socket = Some(s);
                            }
                            Err(()) => {
                                console_log!(
                                    "[threaded_websocket : {name}] Connection failed to {addr:?}, retrying soon"
                                );
                                statuses
                                    .send(NetworkStatus::ConnectionFailed)
                                    .await
                                    .unwrap();
                            }
                        }
                    }
                }
            }
        }
        select! {
            _ = sleep(Duration::from_secs(1)).fuse() => {}
            new_addr = addresses.recv().fuse() => {
                if addr != new_addr.unwrap() {
                    console_log!("[threaded_websocket : {name}] Address changed from {addr:?} to {new_addr:?}");
                    statuses.send(NetworkStatus::NotConnected).await.unwrap();
                    if let Some(socket) = socket.take() {
                        console_log!("[threaded_websocket : {name}] Closing socket to {addr:?}");
                        socket.my_close().await
                    }
                    addr = new_addr.unwrap();
                }
            }
            incoming_data = socket_read_fut(&mut socket).fuse() => {
                if let Ok(incoming_data) = incoming_data {
                    // console_log!("[threaded_websocket : {name}] Received data from {addr:?}");
                    let incoming_data = match incoming_data {
                        TextOrT::T(data) => match deserializer(&data) {
                            Ok(data) => Some(TextOrT::T(data)),
                            Err(e) => {
                                console_log!("[threaded_websocket : {name}] Error deserializing data: {e:?}");
                                None
                            }
                        },
                        TextOrT::Text(text) => Some(TextOrT::Text(text)),
                        TextOrT::Bytes(data) => Some(TextOrT::Bytes(data))
                    };
                    if let Some(data) = incoming_data {
                        data_incoming.send(data).await.unwrap();
                    }
                } else {
                    console_log!("[threaded_websocket : {name}] Connection closed to {addr:?} due to error reading");
                    statuses.send(NetworkStatus::ConnectionFailed).await.unwrap();
                    if let Some(socket) = socket.take() {
                        console_log!("[threaded_websocket : {name}] Closing socket to {addr:?}");
                        socket.my_close().await
                    }
                }
            }
            outgoing_data = data_outgoing.recv().fuse() => {
                if let Some(socket) = &mut socket {
                    // console_log!("[threaded_websocket : {name}] Sending data to {addr:?}");
                    let outgoing_data = match outgoing_data.unwrap() {
                        TextOrT::T(data) => TextOrT::T(serializer(data).expect("failed to serialize data")),
                        TextOrT::Text(text) => TextOrT::Text(text),
                        TextOrT::Bytes(data) => TextOrT::Bytes(data)
                    };
                    socket.my_send(outgoing_data).await
                }
            }
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl<SendType: Serialize, ReceiveType: DeserializeOwned> ThreadableSocket<SendType, ReceiveType>
    for WebSocketStream<ConnectStream>
{
    async fn my_connect(addr: Address) -> Result<Self, ()> {
        let ([a, b, c, d], port) = addr;
        Ok(
            async_tungstenite::async_std::connect_async(format!("ws://{a}.{b}.{c}.{d}:{port}"))
                .await
                .map_err(|e| {
                    eprintln!(
                        "[threaded_websocket/WebSocketStream] Error connecting: {:?}",
                        e
                    )
                })?
                .0,
        )
    }

    async fn my_send(&mut self, data: TextOrT<Vec<u8>>) {
        if let Err(e) = match data {
            TextOrT::T(data) | TextOrT::Bytes(data) => self.send(Message::Binary(data)).await,
            TextOrT::Text(text) => self.send(Message::Text(text)).await,
        } {
            eprintln!(
                "[threaded_websocket/WebSocketStream] Error sending data: {:?}",
                e
            );
        }
    }

    async fn my_read(&mut self) -> Result<TextOrT<Vec<u8>>, ()> {
        match self.next().await {
            Some(Ok(Message::Binary(bytes))) => Ok(TextOrT::T(bytes)),
            Some(Ok(Message::Text(text))) => Ok(TextOrT::Text(text)),
            Some(Ok(Message::Close(_))) => {
                eprintln!("[threaded_websocket/WebSocketStream] Connection closing");
                Err(())
            }
            Some(Ok(msg)) => {
                eprintln!(
                    "[threaded_websocket/WebSocketStream] Unexpected message type: {:?}",
                    msg
                );
                Err(())
            }
            Some(err) => {
                eprintln!(
                    "[threaded_websocket/WebSocketStream] Error reading message: {:?}",
                    err
                );
                Err(())
            }
            _ => Err(()),
        }
    }

    async fn my_close(mut self) {
        if let Err(e) = self.close(None).await {
            eprintln!(
                "[threaded_websocket/WebSocketStream] Error closing websocket: {:?}",
                e
            );
        }
    }
}

#[cfg(target_arch = "wasm32")]
/// A WASM websocket compatible with [`ThreadedSocket`]
pub struct WasmThreadableWebsocket {
    ws: WebSocket,
    messages: Receiver<Result<TextOrT<Vec<u8>>, ()>>,
}

// https://rustwasm.github.io/wasm-bindgen/examples/websockets.html
#[cfg(target_arch = "wasm32")]
impl<SendType: Serialize, ReceiveType: DeserializeOwned> ThreadableSocket<SendType, ReceiveType>
    for WasmThreadableWebsocket
{
    async fn my_connect(addr: Address) -> Result<Self, ()> {
        let ([a, b, c, d], port) = addr;
        let addr = format!("{a}.{b}.{c}.{d}:{port}");

        let (msg_tx, msg_rx) = unbounded();
        let tx2 = msg_tx.clone();
        let tx3 = msg_tx.clone();

        let ws = WebSocket::new(&("ws://".to_string() + &addr)).map_err(|_| ())?;

        let onmessage_callback = Closure::<dyn FnMut(_)>::new(move |e: MessageEvent| {
            // Handle difference Text/Binary,...
            if let Ok(abuf) = e.data().dyn_into::<js_sys::ArrayBuffer>() {
                let array = js_sys::Uint8Array::new(&abuf);
                console_log!(
                    "[threaded_websocket/WasmThreadableWebsocket] message event, received Buf: {} bytes",
                    array.byte_length()
                );
                let _ = msg_tx.send(Ok(TextOrT::T(array.to_vec())));
            } else if let Ok(blob) = e.data().dyn_into::<web_sys::Blob>() {
                // console_log!("message event, received blob: {:?}", blob);
                let fr = web_sys::FileReader::new().unwrap();
                let fr_c = fr.clone();
                let msg_tx_c = msg_tx.clone();
                // create onLoadEnd callback
                let onloadend_cb =
                    Closure::<dyn FnMut(_)>::new(move |_e: web_sys::ProgressEvent| {
                        let array = js_sys::Uint8Array::new(&fr_c.result().unwrap());
                        // let len = array.byte_length() as usize;
                        // console_log!(
                        //     "[threaded_websocket/WasmThreadableWebsocket] message event, received Blob: {} bytes",
                        //     len
                        // );
                        let _ = block_on(msg_tx_c.send(Ok(TextOrT::T(array.to_vec()))));
                    });
                fr.set_onloadend(Some(onloadend_cb.as_ref().unchecked_ref()));
                fr.read_as_array_buffer(&blob)
                    .expect("[threaded_websocket/WasmThreadableWebsocket] blob not readable");
                onloadend_cb.forget();
            } else if let Ok(txt) = e.data().dyn_into::<js_sys::JsString>() {
                console_log!(
                    "[threaded_websocket/WasmThreadableWebsocket] message event, received Text: {:?}",
                    txt
                );
            } else {
                console_log!(
                    "[threaded_websocket/WasmThreadableWebsocket] message event, received Unknown: {:?}",
                    e.data()
                );
            }
        });

        // set message event handler on WebSocket
        ws.set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));
        // forget the callback to keep it alive
        onmessage_callback.forget();

        let onerror_callback = Closure::<dyn FnMut(_)>::new(move |e: ErrorEvent| {
            console_log!(
                "[threaded_websocket/WasmThreadableWebsocket] error event: {:?}",
                e
            );
            if let Err(e) = block_on(tx2.send(Err(()))) {
                console_log!(
                    "[threaded_websocket/WasmThreadableWebsocket] error sending error: {:?}",
                    e
                );
            }
        });
        ws.set_onerror(Some(onerror_callback.as_ref().unchecked_ref()));
        onerror_callback.forget();

        match msg_rx.recv().await {
            Ok(Ok(msg)) => {
                console_log!("[threaded_websocket/WasmThreadableWebsocket] Websocket received first valid message");
                // send this message around again so that it can be read by callers
                tx3.send(Ok(msg)).await.unwrap();
                Ok(Self {
                    ws,
                    messages: msg_rx,
                })
            }
            Ok(Err(_)) => {
                console_log!(
                    "[threaded_websocket/WasmThreadableWebsocket] Websocket had a javascript error, failed to connect"
                );
                Err(())
            }
            Err(e) => {
                console_log!("[threaded_websocket/WasmThreadableWebsocket] Channel could not receive data: {e:?}");
                Err(())
            }
        }
    }

    async fn my_send(&mut self, data: TextOrT<Vec<u8>>) {
        if let Err(e) = match data {
            TextOrT::T(data) => self.ws.send_with_u8_array(&data),
            TextOrT::Text(text) => self.ws.send_with_str(&text),
            TextOrT::Bytes(data) => self.ws.send_with_u8_array(&data),
        } {
            console_log!("[threaded_websocket/WasmThreadableWebsocket] Error sending data: {e:?}");
        }
    }

    async fn my_read(&mut self) -> Result<TextOrT<Vec<u8>>, ()> {
        match self.messages.recv().await {
            Ok(Ok(msg)) => match msg {
                TextOrT::Text(text) => Ok(TextOrT::Text(text)),
                TextOrT::T(data) => Ok(TextOrT::T(data)),
                TextOrT::Bytes(data) => Ok(TextOrT::Bytes(data)),
            },
            Ok(Err(_)) => {
                console_log!(
                    "[threaded_websocket/WasmThreadableWebsocket] Websocket had a javascript error"
                );
                Err(())
            }
            Err(e) => {
                panic!("[threaded_websocket/WasmThreadableWebsocket] Channel could not receive data: {e:?}");
            }
        }
    }

    async fn my_close(self) {
        if let Err(e) = self.ws.close() {
            console_log!(
                "[threaded_websocket/WasmThreadableWebsocket] Error closing websocket: {e:?}"
            );
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
/// A TCP socket compatible with [`ThreadedSocket`]
pub struct TcpStreamThreadableSocket {
    stream: TcpStream,
}

#[cfg(not(target_arch = "wasm32"))]
impl<SendType: Serialize, ReceiveType: DeserializeOwned> ThreadableSocket<SendType, ReceiveType>
    for TcpStreamThreadableSocket
{
    async fn my_connect(addr: Address) -> Result<Self, ()> {
        let ([a, b, c, d], port) = addr;
        match TcpStream::connect(format!("{a}.{b}.{c}.{d}:{port}")).await {
            Ok(stream) => Ok(Self { stream }),
            Err(e) => {
                eprintln!("[threaded_websocket/TcpStreamThreadableSocket] Error connecting: {e:?}");
                Err(())
            }
        }
    }

    async fn my_send(&mut self, data: TextOrT<Vec<u8>>) {
        if let TextOrT::T(bytes) | TextOrT::Bytes(bytes) = data {
            // first send the length
            let len = bytes.len() as u32;
            if let Err(e) = self.stream.write_all(&len.to_be_bytes()).await {
                eprintln!(
                    "[threaded_websocket/TcpStreamThreadableSocket] Error sending length: {e:?}"
                );
                return;
            }
            // then send the data
            if let Err(e) = self.stream.write_all(&bytes).await {
                eprintln!(
                    "[threaded_websocket/TcpStreamThreadableSocket] Error sending data: {e:?}"
                );
            }
        } else {
            eprintln!("[threaded_websocket/TcpStreamThreadableSocket] Cannot send text")
        }
    }

    async fn my_read(&mut self) -> Result<TextOrT<Vec<u8>>, ()> {
        // first read the length
        let mut len_buf = [0; 4];
        if let Err(e) = self.stream.read_exact(&mut len_buf).await {
            eprintln!("[threaded_websocket/TcpStreamThreadableSocket] Error reading length: {e:?}");
            return Err(());
        }
        let len = u32::from_be_bytes(len_buf) as usize;

        // then read the data
        let mut received = vec![0; len];
        if let Err(e) = self.stream.read_exact(&mut received).await {
            eprintln!("[threaded_websocket/TcpStreamThreadableSocket] Error reading data: {e:?}");
            return Err(());
        }

        Ok(TextOrT::T(received))
    }

    async fn my_close(mut self) {
        if let Err(e) = self.stream.close().await {
            eprintln!("[threaded_websocket/TcpStreamThreadableSocket] Error closing stream: {e:?}");
        }
    }
}
