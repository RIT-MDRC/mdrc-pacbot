#![allow(async_fn_in_trait)]

use crate::messages::NetworkStatus;
use crate::{bin_decode, bin_encode, console_log};
use async_channel::{unbounded, Receiver, Sender};
use async_std::task::sleep;
use futures::executor::block_on;
use futures::{select, FutureExt};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::future;
use std::time::Duration;
#[cfg(target_arch = "wasm32")]
use {
    crate::log,
    wasm_bindgen_futures::spawn_local,
    web_sys::wasm_bindgen::closure::Closure,
    web_sys::wasm_bindgen::JsCast,
    web_sys::WebSocket,
    web_sys::{js_sys, ErrorEvent, MessageEvent},
};
#[cfg(not(target_arch = "wasm32"))]
use {
    async_tungstenite::async_std::ConnectStream, async_tungstenite::tungstenite::Message,
    async_tungstenite::WebSocketStream, futures::SinkExt, futures::StreamExt,
};

pub type Address = ([u8; 4], u16);

pub struct ThreadedSocket<SendType, ReceiveType> {
    status: NetworkStatus,

    addr_sender: Sender<Option<Address>>,
    sender: Sender<SendType>,
    status_receiver: Receiver<NetworkStatus>,
    receiver: Receiver<ReceiveType>,
}

impl<SendType: Send + 'static, ReceiveType: Send + 'static> ThreadedSocket<SendType, ReceiveType> {
    /// specify an address to connect to (or None to suspend current connection and future attempts)
    pub fn connect(&mut self, addr: Option<Address>) {
        block_on(self.addr_sender.send(addr)).expect("ThreadedSocket address sender is closed");
    }

    /// fetch the latest information about the status of the connection
    pub fn status(&mut self) -> NetworkStatus {
        while let Ok(status) = self.status_receiver.try_recv() {
            self.status = status
        }
        self.status
    }

    /// queue something to be sent to the socket
    ///
    /// if the connection is not available, the data will be discarded
    pub fn send(&mut self, data: SendType) {
        block_on(self.sender.send(data)).expect("ThreadedSocket data sender is closed");
    }

    /// read new data from the socket, if it is available
    ///
    /// this should be called frequently
    pub fn read(&mut self) -> Option<ReceiveType> {
        self.status();
        self.receiver.try_recv().ok()
    }

    pub fn new<SocketType: ThreadableSocket<SendType, ReceiveType> + 'static>(
        addr: Option<Address>,
    ) -> Self {
        console_log!("[threaded_websocket] Socket created with initial address {addr:?}");

        let (addr_sender, addr_rx) = unbounded();
        let (sender, sender_rx) = unbounded();
        let (status_tx, status_receiver) = unbounded();
        let (receiver_tx, receiver) = unbounded();

        block_on(addr_sender.send(addr)).unwrap();

        #[cfg(target_arch = "wasm32")]
        spawn_local(run_socket_forever::<SendType, ReceiveType, SocketType>(
            addr_rx,
            sender_rx,
            status_tx,
            receiver_tx,
        ));
        #[cfg(not(target_arch = "wasm32"))]
        std::thread::spawn(|| {
            block_on(run_socket_forever::<SendType, ReceiveType, SocketType>(
                addr_rx,
                sender_rx,
                status_tx,
                receiver_tx,
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

#[cfg(target_arch = "wasm32")]
impl<SendType: Serialize + Send + 'static, ReceiveType: DeserializeOwned + Send + 'static> Default
    for ThreadedSocket<SendType, ReceiveType>
{
    fn default() -> Self {
        Self::new::<WasmThreadableWebsocket>(None)
    }
}

pub trait ThreadableSocket<SendType, ReceiveType>: Sized {
    async fn my_connect(addr: Address) -> Result<Self, ()>;

    async fn my_send(&mut self, data: SendType);

    async fn my_read(&mut self) -> Result<ReceiveType, ()>;

    async fn my_close(self);
}

async fn socket_read_fut<T: ThreadableSocket<S, R>, S, R>(socket: &mut Option<T>) -> Result<R, ()> {
    if let Some(socket) = socket {
        socket.my_read().await
    } else {
        future::pending().await
    }
}

async fn run_socket_forever<
    OutgoingType,
    IncomingType,
    SocketType: ThreadableSocket<OutgoingType, IncomingType>,
>(
    addresses: Receiver<Option<Address>>,
    data_outgoing: Receiver<OutgoingType>,
    statuses: Sender<NetworkStatus>,
    data_incoming: Sender<IncomingType>,
) {
    let mut addr: Option<Address> = None;
    let mut socket: Option<SocketType> = None;

    loop {
        if socket.is_none() {
            if let Some(addr) = addr {
                console_log!("[threaded_websocket] Connecting to {addr:?}...");
                statuses.send(NetworkStatus::Connecting).await.unwrap();
                match SocketType::my_connect(addr).await {
                    Ok(s) => {
                        console_log!("[threaded_websocket] Connected to {addr:?}");
                        statuses.send(NetworkStatus::Connected).await.unwrap();
                        socket = Some(s);
                    }
                    Err(()) => {
                        console_log!(
                            "[threaded_websocket] Connection failed to {addr:?}, retrying soon"
                        );
                        statuses
                            .send(NetworkStatus::ConnectionFailed)
                            .await
                            .unwrap();
                    }
                }
            }
        }
        select! {
            _ = sleep(Duration::from_secs(1)).fuse() => {}
            new_addr = addresses.recv().fuse() => {
                if addr != new_addr.unwrap() {
                    console_log!("[threaded_websocket] Address changed from {addr:?} to {new_addr:?}");
                    statuses.send(NetworkStatus::NotConnected).await.unwrap();
                    if let Some(socket) = socket.take() {
                        console_log!("[threaded_websocket] Closing socket to {addr:?}");
                        socket.my_close().await
                    }
                    addr = new_addr.unwrap();
                }
            }
            incoming_data = socket_read_fut(&mut socket).fuse() => {
                if let Ok(incoming_data) = incoming_data {
                    console_log!("[threaded_websocket] Received data from {addr:?}");
                    data_incoming.send(incoming_data).await.unwrap();
                } else {
                    console_log!("[threaded_websocket] Connection closed to {addr:?} due to error reading");
                    statuses.send(NetworkStatus::ConnectionFailed).await.unwrap();
                    if let Some(socket) = socket.take() {
                        socket.my_close().await
                    }
                }
            }
            outgoing_data = data_outgoing.recv().fuse() => {
                if let Some(socket) = &mut socket {
                    console_log!("[threaded_websocket] Sending data to {addr:?}");
                    socket.my_send(outgoing_data.unwrap()).await
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
            async_tungstenite::async_std::connect_async(format!("{a}.{b}.{c}.{d}:{port}"))
                .await
                .map_err(|_| ())?
                .0,
        )
    }

    async fn my_send(&mut self, data: SendType) {
        let _ = self.send(Message::Binary(bin_encode(data).unwrap())).await;
    }

    async fn my_read(&mut self) -> Result<ReceiveType, ()> {
        match self.next().await {
            Some(Ok(Message::Binary(bytes))) => {
                if let Ok(data) = bin_decode(&bytes) {
                    return data.0;
                } else {
                    Err(())
                }
            }
            _ => Err(()),
        }
    }

    async fn my_close(mut self) {
        let _ = self.close(None).await;
    }
}

#[cfg(target_arch = "wasm32")]
pub struct WasmThreadableWebsocket {
    ws: WebSocket,
    messages: Receiver<Result<Vec<u8>, ()>>,
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
                    "[threaded_websocket] message event, received Buf: {} bytes",
                    array.byte_length()
                );
                let _ = msg_tx.send(Ok(array.to_vec()));
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
                        //     "[threaded_websocket] message event, received Blob: {} bytes",
                        //     len
                        // );
                        let _ = block_on(msg_tx_c.send(Ok(array.to_vec())));
                    });
                fr.set_onloadend(Some(onloadend_cb.as_ref().unchecked_ref()));
                fr.read_as_array_buffer(&blob)
                    .expect("[threaded_websocket] blob not readable");
                onloadend_cb.forget();
            } else if let Ok(txt) = e.data().dyn_into::<js_sys::JsString>() {
                console_log!(
                    "[threaded_websocket] message event, received Text: {:?}",
                    txt
                );
            } else {
                console_log!(
                    "[threaded_websocket] message event, received Unknown: {:?}",
                    e.data()
                );
            }
        });

        // set message event handler on WebSocket
        ws.set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));
        // forget the callback to keep it alive
        onmessage_callback.forget();

        let onerror_callback = Closure::<dyn FnMut(_)>::new(move |e: ErrorEvent| {
            console_log!("[threaded_websocket] error event: {:?}", e);
            if let Err(e) = block_on(tx2.send(Err(()))) {
                console_log!("[threaded_websocket] error sending error: {:?}", e);
            }
        });
        ws.set_onerror(Some(onerror_callback.as_ref().unchecked_ref()));
        onerror_callback.forget();

        match msg_rx.recv().await {
            Ok(Ok(msg)) => {
                console_log!("[threaded_websocket] Websocket received first valid message");
                // send this message around again so that it can be read by callers
                tx3.send(Ok(msg)).await.unwrap();
                Ok(Self {
                    ws,
                    messages: msg_rx,
                })
            }
            Ok(Err(_)) => {
                console_log!(
                    "[threaded_websocket] Websocket had a javascript error, failed to connect"
                );
                Err(())
            }
            Err(e) => {
                console_log!("[threaded_websocket] Channel could not receive data: {e:?}");
                Err(())
            }
        }
    }

    async fn my_send(&mut self, data: SendType) {
        if let Err(e) = self.ws.send_with_u8_array(&bin_encode(data).unwrap()) {
            console_log!("[threaded_websocket] Error sending data: {e:?}");
        }
    }

    async fn my_read(&mut self) -> Result<ReceiveType, ()> {
        match self.messages.recv().await {
            Ok(Ok(msg)) => Ok(bin_decode(&msg).unwrap().0),
            Ok(Err(_)) => {
                console_log!("[threaded_websocket] Websocket had a javascript error");
                Err(())
            }
            Err(e) => {
                panic!("[threaded_websocket] Channel could not receive data: {e:?}");
            }
        }
    }

    async fn my_close(self) {
        if let Err(e) = self.ws.close() {
            console_log!("[threaded_websocket] Error closing websocket: {e:?}");
        }
    }
}
