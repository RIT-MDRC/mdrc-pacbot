#![allow(async_fn_in_trait)]

use crate::messages::NetworkStatus;
use crate::{bin_decode, bin_encode};
use async_channel::{unbounded, Receiver, Sender};
use async_std::task::sleep;
use futures::executor::block_on;
use futures::{select, FutureExt};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::future;
use std::time::Duration;
use wasm_bindgen_futures::spawn_local;
#[cfg(not(target_arch = "wasm32"))]
use {
    async_tungstenite::async_std::ConnectStream, async_tungstenite::tungstenite::Message,
    async_tungstenite::WebSocketStream,
};
#[cfg(target_arch = "wasm32")]
use {
    web_sys::wasm_bindgen::closure::Closure,
    web_sys::wasm_bindgen::JsCast,
    web_sys::WebSocket,
    web_sys::{js_sys, ErrorEvent, MessageEvent},
};

pub type Address = ([u8; 4], u16);

pub struct ThreadedSocket<SendType, ReceiveType> {
    status: NetworkStatus,

    addr_sender: Sender<Option<Address>>,
    sender: Sender<SendType>,
    status_receiver: Receiver<NetworkStatus>,
    receiver: Receiver<ReceiveType>,
}

impl<SendType: 'static, ReceiveType: 'static> ThreadedSocket<SendType, ReceiveType> {
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
        let (addr_sender, addr_rx) = unbounded();
        let (sender, sender_rx) = unbounded();
        let (status_tx, status_receiver) = unbounded();
        let (receiver_tx, receiver) = unbounded();

        block_on(addr_sender.send(addr)).unwrap();

        let fut = run_socket_forever::<SendType, ReceiveType, SocketType>(
            addr_rx,
            sender_rx,
            status_tx,
            receiver_tx,
        );

        #[cfg(target_arch = "wasm32")]
        spawn_local(fut);
        #[cfg(not(target_arch = "wasm32"))]
        std::thread::spawn(|| block_on(fut));

        Self {
            status: NetworkStatus::NotConnected,

            addr_sender,
            sender,
            status_receiver,
            receiver,
        }
    }
}

pub trait ThreadableSocket<SendType, ReceiveType>: Sized {
    async fn my_connect(addr: Address) -> Result<Self, ()>;

    async fn my_send(&mut self, data: SendType);

    async fn my_read(&mut self) -> ReceiveType;

    async fn my_close(self);
}

async fn socket_read_fut<T: ThreadableSocket<S, R>, S, R>(socket: &mut Option<T>) -> R {
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
                statuses.send(NetworkStatus::Connecting).await.unwrap();
                match SocketType::my_connect(addr).await {
                    Ok(s) => {
                        statuses.send(NetworkStatus::Connected).await.unwrap();
                        socket = Some(s);
                    }
                    Err(()) => {
                        statuses
                            .send(NetworkStatus::ConnectionFailed)
                            .await
                            .unwrap();
                        sleep(Duration::from_secs(1)).await;
                    }
                }
            }
        }
        select! {
            new_addr = addresses.recv().fuse() => {
                if addr != new_addr.unwrap() {
                    statuses.send(NetworkStatus::NotConnected).await.unwrap();
                    if let Some(socket) = socket.take() {
                        socket.my_close().await
                    }
                    addr = new_addr.unwrap();
                }
            }
            incoming_data = socket_read_fut(&mut socket).fuse() => {
                data_incoming.send(incoming_data).await.unwrap();
            }
            outgoing_data = data_outgoing.recv().fuse() => {
                if let Some(socket) = &mut socket {
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

    async fn my_read(&mut self) -> ReceiveType {
        loop {
            match self.next().await {
                Some(Ok(Message::Binary(bytes))) => {
                    if let Ok(data) = bin_decode(&bytes) {
                        return data.0;
                    }
                }
                _ => {}
            }
        }
    }

    async fn my_close(mut self) {
        let _ = self.close(None).await;
    }
}

#[cfg(target_arch = "wasm32")]
pub struct WasmThreadableWebsocket {
    ws: WebSocket,
    status: NetworkStatus,
    messages: Receiver<Result<Vec<u8>, ()>>,
}

#[cfg(target_arch = "wasm32")]
impl<SendType: Serialize, ReceiveType: DeserializeOwned> ThreadableSocket<SendType, ReceiveType>
    for WasmThreadableWebsocket
{
    async fn my_connect(addr: Address) -> Result<Self, ()> {
        let ([a, b, c, d], port) = addr;
        let addr = format!("{a}.{b}.{c}.{d}:{port}");

        let (msg_tx, msg_rx) = unbounded();
        let tx2 = msg_tx.clone();

        let ws = WebSocket::new(&("ws://".to_string() + &addr)).map_err(|_| ())?;

        let onmessage_callback = Closure::<dyn FnMut(_)>::new(move |e: MessageEvent| {
            // Handle difference Text/Binary,...
            if let Ok(abuf) = e.data().dyn_into::<js_sys::ArrayBuffer>() {
                let array = js_sys::Uint8Array::new(&abuf);
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
                        let _ = block_on(msg_tx_c.send(Ok(array.to_vec())));
                    });
                fr.set_onloadend(Some(onloadend_cb.as_ref().unchecked_ref()));
                fr.read_as_array_buffer(&blob).expect("blob not readable");
                onloadend_cb.forget();
            }
        });

        // set message event handler on WebSocket
        ws.set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));
        // forget the callback to keep it alive
        onmessage_callback.forget();

        let onerror_callback = Closure::<dyn FnMut(_)>::new(move |_: ErrorEvent| {
            let _ = tx2.send(Err(()));
        });
        ws.set_onerror(Some(onerror_callback.as_ref().unchecked_ref()));
        onerror_callback.forget();

        if let Ok(Err(_)) = msg_rx.recv().await {
            return Err(());
        }

        Ok(Self {
            ws,
            status: NetworkStatus::Connecting,
            messages: msg_rx,
        })
    }

    async fn my_send(&mut self, data: SendType) {
        let _ = self.ws.send_with_u8_array(&bin_encode(data).unwrap());
    }

    async fn my_read(&mut self) -> ReceiveType {
        loop {
            match self.messages.recv().await {
                Ok(Ok(msg)) => {
                    self.status = NetworkStatus::Connected;
                    return bin_decode(&msg).unwrap().0;
                }
                _ => self.status = NetworkStatus::ConnectionFailed,
            }
        }
    }

    async fn my_close(self) {
        let _ = self.ws.close();
    }
}
