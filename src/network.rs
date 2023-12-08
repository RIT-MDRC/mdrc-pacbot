//! Network communications with the Pico and the game server.

use std::{io, net::UdpSocket};

use futures_util::StreamExt;

/// Starts the network thread that communicates with the Pico and game server.
/// This function does not block.
pub fn start_network_thread() {
    std::thread::Builder::new()
        .name("network thread".into())
        .spawn(move || {
            let async_runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("error creating tokio runtime");

            async_runtime.block_on(network_thread_main());
        })
        .unwrap();
}

/// The function that runs on the network thread.
async fn network_thread_main() {
    let server_ip = "localhost";
    let websocket_port = 3002;
    let url = format!("ws://{server_ip}:{websocket_port}");

    // Establish the WebSocket connection.
    println!("Connecting to {url}");
    let (mut socket, response) = tokio_tungstenite::connect_async(url)
        .await
        .expect("error connecting to game server");
    println!("Connected; status = {}", response.status());

    // Handle incoming messages.
    loop {
        tokio::select! {
            message = socket.next() => {
                match message {
                    Some(message) => {
                        println!("GOT MESSAGE:  {message:?}");
                    },
                    None => break, // This case means the WebSocket is closed.
                }
            }
        };
    }
}

/// Types of messages sent to the Pico.
#[repr(u8)]
enum MessageType {
    Motors = 1,
    Sleep = 2,
    AccelOffset = 3,
    DistanceOffset = 4,
}

struct PicoConnection {
    socket: UdpSocket,
    next_ack: u16,
}

impl PicoConnection {
    fn new(local_port: u16, remote_address: &str) -> io::Result<Self> {
        let socket = UdpSocket::bind(("0.0.0.0", local_port))?;
        socket.connect(remote_address)?;
        Ok(Self {
            socket,
            next_ack: 1,
        })
    }

    fn send_message(&mut self, message: &[u8]) -> io::Result<()> {
        self.socket.send(message)?;
        Ok(())
    }

    fn send_motors_message(&mut self, motor1: i16, motor2: i16, motor3: i16) -> io::Result<()> {
        let mut message = [0; 7];
        message[0] = MessageType::Motors as u8;
        message[1..3].copy_from_slice(&motor1.to_le_bytes());
        message[3..5].copy_from_slice(&motor2.to_le_bytes());
        message[5..7].copy_from_slice(&motor3.to_le_bytes());
        self.send_message(&message)
    }

    fn send_sleep_message(&mut self, sleep: bool) -> io::Result<()> {
        let mut message = [0; 4];
        message[0] = MessageType::Sleep as u8;
        message[1..3].copy_from_slice(&self.next_ack.to_le_bytes());
        self.next_ack += 1;
        message[3] = sleep as u8;
        self.send_message(&message)
    }

    fn send_accel_offset_message(&mut self, offset: i16) -> io::Result<()> {
        let mut message = [0; 5];
        message[0] = MessageType::AccelOffset as u8;
        message[1..3].copy_from_slice(&self.next_ack.to_le_bytes());
        self.next_ack += 1;
        message[3..5].copy_from_slice(&offset.to_le_bytes());
        self.send_message(&message)
    }

    fn send_distance_offset_message(&mut self, which_sensor: u8, offset: i16) -> io::Result<()> {
        let mut message = [0; 6];
        message[0] = MessageType::DistanceOffset as u8;
        message[1..3].copy_from_slice(&self.next_ack.to_le_bytes());
        self.next_ack += 1;
        message[3] = which_sensor;
        message[4..6].copy_from_slice(&offset.to_le_bytes());
        self.send_message(&message)
    }
}
