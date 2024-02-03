//! Network communications with the Pico and the game server.

use rapier2d::na::Vector2;
use std::f32::consts::FRAC_PI_3;
use std::ops::DerefMut;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use std::{io, net::UdpSocket};
use tokio::sync::mpsc::Receiver;
use crate::gui::physics::PhysicsRenderInfo;

/// Starts the network thread that communicates with the Pico and game server.
/// This function does not block.
pub fn start_network_thread(
    receiver: Receiver<NetworkCommand>,
    sensors: Arc<RwLock<(bool, [u8; 8], [i64; 3], Instant)>>,
    target_velocity: Arc<RwLock<(Vector2<f32>, f32)>>,
    phys_render: Arc<RwLock<PhysicsRenderInfo>>,
) {
    std::thread::Builder::new()
        .name("network thread".into())
        .spawn(move || {
            let async_runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("error creating tokio runtime");

            async_runtime.block_on(network_thread_main(receiver, sensors, target_velocity, phys_render));
        })
        .unwrap();
}

#[derive(Clone, Debug)]
pub enum NetworkCommand {
    NewPacbotAddress(String),
}

/// The function that runs on the network thread.
async fn network_thread_main(
    mut receiver: Receiver<NetworkCommand>,
    sensors: Arc<RwLock<(bool, [u8; 8], [i64; 3], Instant)>>,
    target_velocity: Arc<RwLock<(Vector2<f32>, f32)>>,
    phys_render: Arc<RwLock<PhysicsRenderInfo>>,
) {
    // let server_ip = "localhost";
    // let websocket_port = 3002;
    // let url = format!("ws://{server_ip}:{websocket_port}");

    // Establish the WebSocket connection.
    // let mut socket;
    // if false {
    //     println!("Connecting to {url}");
    //     let (new_socket, response) = tokio_tungstenite::connect_async(url)
    //         .await
    //         .expect("error connecting to game server");
    //     socket = Some(new_socket);
    //     println!("Connected; status = {}", response.status());
    // } else {
    //     socket = None;
    // }

    let mut pico_address = "192.168.4.16:20001".to_string();

    // Timer to ping pico
    let mut pico_timer = tokio::time::interval(Duration::from_millis(10));
    let mut pico_reconnection_timer = tokio::time::interval(Duration::from_millis(500));
    // This is a terrible way to do this
    let mut pico_recv_timer = tokio::time::interval(Duration::from_millis(1));
    let pico_connection = Arc::new(RwLock::new(None::<PicoConnection>));

    let mut last_motors = [0i16; 3];
    let mut last_sent = Instant::now();

    // Handle incoming messages.
    loop {
        tokio::select! {
            // message = socket.unwrap().next(), if socket.is_some() => {
            //     match message {
            //         Some(message) => {
            //             println!("GOT MESSAGE:  {message:?}");
            //         },
            //         None => break, // This case means the WebSocket is closed.
            //     }
            // }
            _ = pico_timer.tick() => {
                let mut pico_connection = pico_connection.write().unwrap();
                if let Some(pico) = &mut pico_connection.deref_mut() {
                    let target_velocity = *target_velocity.read().unwrap();
                    let x = target_velocity.0.x;
                    let y = target_velocity.0.y;

                    // use x and y to find the desired angle
                    let angle = y.atan2(x);

                    let current_angle = phys_render.read().unwrap().pacbot_pos.rotation.angle();
                    let angle = angle - current_angle;

                    let mut scale = (x.powi(2) + y.powi(2)).sqrt();
                    if scale != 0.0 {
                        scale = 7.0;
                    }

                    let motor_angles = [
                        angle.cos(),
                        (angle - (2.0 * FRAC_PI_3)).cos(),
                        (angle + (2.0 * FRAC_PI_3)).cos(),
                    ];

                    let rotate_adjust = if target_velocity.1 > 0.0 {
                        3.0
                    } else if target_velocity.1 < 0.0 {
                        -3.0
                    } else { 0.0 };

                    let motors_i16 = [
                        // constant is like max speed - can go up to 255.0
                        (motor_angles[0] * (255.0/11.6) * scale + rotate_adjust * (255.0/11.6)) as i16,
                        (motor_angles[1] * (255.0/11.6) * scale + rotate_adjust * (255.0/11.6)) as i16,
                        (motor_angles[2] * (255.0/11.6) * scale + rotate_adjust * (255.0/11.6)) as i16,
                    ];

                    if last_motors != motors_i16 || last_sent.elapsed() > Duration::from_millis(100) || true {
                        // don't repeat motor commands too often
                        last_motors = motors_i16;
                        last_sent = Instant::now();

                        let mut motors = [(0, true); 3];
                        for i in 0..3 {
                            motors[i].0 = motors_i16[i].abs() as u8;
                            motors[i].1 = motors_i16[i] >= 0;
                        }

                        if let Err(e) = pico.send_motors_message(motors) {
                            println!("{:?}", e);
                            *pico_connection = None;
                        }
                    }
                }
            }
            _ = pico_reconnection_timer.tick() => {
                let mut pico_connection = pico_connection.write().unwrap();
                if pico_connection.is_none() {
                    let try_conn = PicoConnection::new(20001, &pico_address);
                    if let Err(ref e) = try_conn {
                        println!("{:?}", e);
                    }
                    *pico_connection = try_conn.ok();
                    if let Some(pico) = pico_connection.deref_mut() {
                        if let Err(e) = pico.socket.set_nonblocking(true) {
                            println!("{:?}", e);
                            *pico_connection = None;
                        }
                    }
                }
            }
            _ = pico_recv_timer.tick() => {
                let mut pico_connection = pico_connection.write().unwrap();
                if let Some(pico) = pico_connection.deref_mut() {
                    let mut bytes = [0; 30];
                    while let Ok(size) = pico.socket.recv(&mut bytes) {
                        if size == 20 {
                            let mut sensors = sensors.write().unwrap();
                            for i in 0..8 {
                                sensors.1[i] = bytes[i];
                            }
                            sensors.3 = Instant::now();
                            for i in 0..3 {
                                sensors.2[i] = i32::from_le_bytes([bytes[i*4 + 8], bytes[i*4 + 9], bytes[i*4 + 10], bytes[i*4 + 11]]) as i64;
                            }
                        }
                    }
                }
            }
            Some(command) = receiver.recv() => {
                match command {
                    NetworkCommand::NewPacbotAddress(s) => {
                        pico_address = s;
                        let mut pico_connection = pico_connection.write().unwrap();
                        *pico_connection = None;
                    }
                }
            }
        }
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

#[allow(dead_code)]
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

    fn send_motors_message(&mut self, motors: [(u8, bool); 3]) -> io::Result<()> {
        let mut message = [0; 7];
        message[0] = MessageType::Motors as u8;
        message[1] = motors[0].0;
        message[2] = motors[1].0;
        message[3] = motors[2].0;
        message[4] = if motors[0].1 { 2 } else { 0 };
        message[5] = if motors[1].1 { 2 } else { 0 };
        message[6] = if motors[2].1 { 2 } else { 0 };
        self.socket.set_nonblocking(false).unwrap();
        let r = self.send_message(&message);
        self.socket.set_nonblocking(true).unwrap();
        r
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
