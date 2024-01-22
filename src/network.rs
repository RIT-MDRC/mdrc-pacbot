//! Network communications with the Pico and the game server.

use std::{io, net::UdpSocket, sync::OnceLock};

use eframe::epaint::mutex::Mutex;
use futures_util::StreamExt;
use rapier2d::na::Vector2;

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

    fn send_motors_message(&mut self, motor1: f32, motor2: f32, motor3: f32) -> io::Result<()> {
        let mut message = [0; 7];
        message[0] = MessageType::Motors as u8;
        message[1..3].copy_from_slice(&motor_speed_to_i16(motor1).to_le_bytes());
        message[3..5].copy_from_slice(&motor_speed_to_i16(motor2).to_le_bytes());
        message[5..7].copy_from_slice(&motor_speed_to_i16(motor3).to_le_bytes());
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

const MAX_SPEED: f32 = 10.0;

fn motor_speed_to_i16(speed: f32) -> i16 {
    let normalized_speed = (speed / MAX_SPEED).clamp(-1.0, 1.0);
    (normalized_speed * (i16::MAX as f32)) as i16
}

static PICO_CONNECTION: OnceLock<Mutex<PicoConnection>> = OnceLock::new();

/// Sends the given target velocity to the robot over the UDP connection.
/// The socket is created on the first call.
/// Panics if an error occurs. (TODO: make this more robust?)
pub fn set_target_robot_velocity(v: (Vector2<f32>, f32)) {
    // let mut pico_connection = PICO_CONNECTION
    //     .get_or_init(|| Mutex::new(PicoConnection::new(20001, "remote_address").unwrap()))
    //     .lock();
    //
    // // TODO: use *math* to convert the target velocity to the 3 actual motor velocities.
    // pico_connection
    //     .send_motors_message(v.0.x, v.0.y, v.1)
    //     .unwrap();
}
