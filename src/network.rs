//! Network communications with the Pico and the game server.

use crate::pathing::TargetVelocity;
use crate::physics::LightPhysicsInfo;
use crate::{PacmanGameState, UserSettings};
use bevy::log::info;
use bevy::prelude::*;
use bincode;
use serde::{Deserialize, Serialize};
use std::f32::consts::{FRAC_PI_2, FRAC_PI_3};
use std::net::TcpStream;
use std::time::{Duration, Instant};
use std::{io, net::UdpSocket};
use tungstenite::stream::MaybeTlsStream;
use tungstenite::{connect, Message, WebSocket};

/// Stores data from Pacbot
#[derive(Resource, Copy, Clone, Debug, Serialize, Deserialize, Default)]
pub struct PacbotSensors {
    /// Distance sensor readings, mm
    pub distance_sensors: [u8; 8],
    /// Encoder positions
    pub encoders: [i64; 3],
    /// Velocity of the encoders
    pub encoder_velocities: [f32; 3],
    /// Output from PID
    pub pid_output: [f32; 3],
    /// PID setpoints
    pub pid_setpoints: [f32; 3],
}

/// Holds the last time when sensor information was received from Pacbot
#[derive(Resource, Default)]
pub struct PacbotSensorsRecvTime(pub(crate) Option<Instant>);

/// Stores connections for the NetworkPlugin
#[derive(Default, Resource)]
pub struct NetworkPluginData {
    /// Connection to the Pico
    pico: Option<PicoConnection>,
}

/// The current state of our game server connection.
#[allow(clippy::large_enum_variant)]
#[derive(Default)]
pub enum GSConnState {
    /// We are not connected, and we don't want to connect.
    #[default]
    Disconnected,
    /// We are currently trying to connect.
    Connecting,
    /// We are connected.
    Connected(WebSocket<MaybeTlsStream<TcpStream>>),
}

impl GSConnState {
    /// Returns true if we're connected.
    pub fn is_connected(&self) -> bool {
        matches!(self, GSConnState::Connected(_))
    }
}

/// Stores a connection to the game server.
#[derive(Default)]
pub struct GameServerConn {
    /// Stores the connection to the game server.
    pub client: GSConnState,
}

/// Polls the game server connection and updates the game state.
/// If we're currently connecting, it tries to connect to the server instead.
pub fn poll_gs(
    settings: Res<UserSettings>,
    mut gs_conn: NonSendMut<GameServerConn>,
    mut game_state: ResMut<PacmanGameState>,
) {
    match &mut gs_conn.client {
        GSConnState::Disconnected => (),
        GSConnState::Connecting => {
            if let Some(gs_address) = &settings.go_server_address {
                info!("Connecting to {gs_address}...");
                if let Ok(stream) = connect(format!("ws://{gs_address}")) {
                    gs_conn.client = GSConnState::Connected(stream.0);
                    info!("Connected!");
                } else {
                    warn!("Unable to connect. Retrying...");
                }
            } else {
                warn!("No address supplied.")
            }
        }
        GSConnState::Connected(client) => {
            if let Ok(msg) = client.read() {
                if let Message::Binary(msg) = msg {
                    game_state.0.update(&msg);
                }
            } else {
                warn!("Disconnected from game server. Attempting to reconnect...");
                gs_conn.client = GSConnState::Connecting;
            }
        }
    }
}

#[derive(Copy, Clone, Debug, Resource, Default)]
/// Holds information about the last velocities sent to the motors
pub struct LastMotorCommands {
    /// the last velocities sent to the motors
    pub motors: [f32; 3],
    /// the angle in radians
    pub angle: f32,
    /// the magnitude of the movement in gu/s
    pub mag: f32,
}

/// Sends current motor commands to the pico
pub fn send_motor_commands(
    mut network_data: ResMut<NetworkPluginData>,
    target_velocity: Res<TargetVelocity>,
    phys_info: Res<LightPhysicsInfo>,
    settings: Res<UserSettings>,
    mut last_motor_commands: ResMut<LastMotorCommands>,
) {
    if let Some(pwm_override) = settings.pwm_override {
        last_motor_commands.motors = [0.0; 3];
        if let Some(pico) = &mut network_data.pico {
            if let Err(e) = pico.send_pwm_message(pwm_override, settings.max_accel, &settings) {
                eprintln!("{:?}", e);
                network_data.pico = None;
            }
        }
        return;
    }
    if let Some(loc) = phys_info.pf_pos {
        let x = target_velocity.0.x;
        let y = target_velocity.0.y;

        let mut current_angle = loc.rotation.angle();
        if settings.motors_ignore_phys_angle {
            current_angle = FRAC_PI_2;
        }

        // use x and y to find the desired angle
        let angle = y.atan2(x);
        let angle = angle - current_angle;

        let scale = (x.powi(2) + y.powi(2)).sqrt();

        let motor_angles = [
            angle.sin(),
            (angle + (4.0 * FRAC_PI_3)).sin(),
            (angle + (2.0 * FRAC_PI_3)).sin(),
        ];

        let rotate_adjust = target_velocity.1;

        let motors = [
            // constant is like max speed - can go up to 255.0
            motor_angles[0] * scale + rotate_adjust,
            motor_angles[1] * scale + rotate_adjust,
            motor_angles[2] * scale + rotate_adjust,
        ];

        last_motor_commands.motors = motors;
        last_motor_commands.angle = angle;
        last_motor_commands.mag = scale;

        if let Some(pico) = &mut network_data.pico {
            if let Err(e) = pico.send_motors_message(motors, angle, scale, &settings) {
                eprintln!("{:?}", e);
                network_data.pico = None;
            }
        }
    }
}

/// Attempts to reconnect to the pico if not currently connected
pub fn reconnect_pico(
    mut network_data: ResMut<NetworkPluginData>,
    mut recv_time: ResMut<PacbotSensorsRecvTime>,
    settings: Res<UserSettings>,
) {
    if settings.pico_address.is_none() {
        network_data.pico = None;
    }
    if network_data.pico.is_none() {
        if let Some(pico_address) = &settings.pico_address {
            if pico_address.is_empty() {
                return;
            }
            let try_conn = PicoConnection::new(pico_address);
            recv_time.0 = None;
            if let Err(ref e) = try_conn {
                info!("{:?}", e);
            }
            network_data.pico = try_conn.ok();
            if let Some(pico) = &mut network_data.pico {
                if let Err(e) = pico.tx_socket.set_nonblocking(true) {
                    info!("{:?}", e);
                    network_data.pico = None;
                }
            }
        }
    }
}

/// Attempts to receive data from the pico connection if any is available
pub fn recv_pico(
    mut network_data: ResMut<NetworkPluginData>,
    mut sensors: ResMut<PacbotSensors>,
    mut recv_time: ResMut<PacbotSensorsRecvTime>,
    settings: Res<UserSettings>,
) {
    if let Some(pico) = &mut network_data.pico {
        let mut bytes = [0; 90];
        while let Ok((size, _)) = pico.rx_socket.recv_from(&mut bytes) {
            if settings.sensors_from_robot {
                if let Ok((message, _)) = bincode::serde::decode_from_slice::<PacbotSensors, _>(
                    &bytes,
                    bincode::config::standard(),
                ) {
                    *sensors = message;
                    recv_time.0 = Some(Instant::now());
                } else {
                    eprintln!("Invalid message from Pico: {size}");
                }
            }
        }
        if recv_time.0.unwrap_or(Instant::now()).elapsed() > Duration::from_millis(50) {
            info!("pico timeout!");
            network_data.pico = None;
        }
    }
}

struct PicoConnection {
    rx_socket: UdpSocket,
    tx_socket: UdpSocket,
    remote_address: String,
}

#[allow(dead_code)]
impl PicoConnection {
    fn new(remote_address: &str) -> io::Result<Self> {
        let rx_socket = UdpSocket::bind(("0.0.0.0", 20001))?;
        rx_socket.set_nonblocking(true).unwrap();

        let tx_socket = UdpSocket::bind(("0.0.0.0", 20002))?;
        Ok(Self {
            rx_socket,
            tx_socket,
            remote_address: remote_address.to_string(),
        })
    }

    fn send_message(&mut self, message: &[u8]) -> io::Result<()> {
        self.tx_socket
            .send_to(message, self.remote_address.as_str())?;
        Ok(())
    }

    fn send_motors_message(
        &mut self,
        motors: [f32; 3],
        angle: f32,
        mag: f32,
        settings: &UserSettings,
    ) -> io::Result<()> {
        let message = PacbotCommand {
            motors: [
                MotorRequest::Velocity(motors[0]),
                MotorRequest::Velocity(motors[1]),
                MotorRequest::Velocity(motors[2]),
            ],
            pid: settings.pid,
            pid_limits: [0x8000 as f32; 3],
            max_accel: settings.max_accel,
            angle,
            mag,
            collision_avoidance: settings.collision_avoidance,
            collision_distance_threshold: settings.collision_distance_threshold,
            collision_avoidance_stop: settings.collision_distance_stop,
            sensor_continuous_interval: settings.sensor_range_interval,
            gui_ip: settings.gui_addr,
        };
        self.tx_socket.set_nonblocking(false).unwrap();
        let r = self.send_message(
            &bincode::serde::encode_to_vec(
                GuiToRobotMsg::Command(message),
                bincode::config::standard(),
            )
            .unwrap(),
        );
        self.tx_socket.set_nonblocking(true).unwrap();
        r
    }

    fn send_pwm_message(
        &mut self,
        motors: [MotorRequest; 3],
        max_accel: f32,
        settings: &UserSettings,
    ) -> io::Result<()> {
        let message = PacbotCommand {
            motors,
            pid: [5.0, 0.1, 0.0],
            pid_limits: [10000.0, 10000.0, 10000.0],
            max_accel,
            angle: 0.0,
            mag: 0.0,
            collision_avoidance: settings.collision_avoidance,
            collision_distance_threshold: settings.collision_distance_threshold,
            collision_avoidance_stop: settings.collision_distance_stop,
            sensor_continuous_interval: settings.sensor_range_interval,
            gui_ip: settings.gui_addr,
        };
        self.tx_socket.set_nonblocking(false).unwrap();
        let r = self.send_message(
            &bincode::serde::encode_to_vec(
                GuiToRobotMsg::Command(message),
                bincode::config::standard(),
            )
            .unwrap(),
        );
        self.tx_socket.set_nonblocking(true).unwrap();
        r
    }
}

#[derive(Copy, Clone, Serialize)]
#[allow(dead_code)]
enum GuiToRobotMsg {
    Config(PacbotConfig),
    Command(PacbotCommand),
}

/// Holds the order of the motor/encoder pins
#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
struct PacbotConfig {
    distance_sensors: [usize; 8],
    encoders: [(usize, usize); 3],
    motors: [(usize, usize); 3],

    /// If some, will attempt to request the (static ip) from the (gateway)
    /// Only has an effect in code on robot side, cannot be updated through message
    #[allow(clippy::type_complexity)]
    pub static_ip: Option<((u8, u8, u8, u8), (u8, u8, u8, u8))>,
    pub udp_port: usize,
    pub i2c_enabled: bool,
    pub motors_enabled: bool,
}

impl Default for PacbotConfig {
    fn default() -> Self {
        Self {
            distance_sensors: [0, 1, 2, 3, 4, 5, 6, 7],
            encoders: [(0, 1), (2, 3), (4, 5)],
            motors: [(0, 1), (2, 3), (4, 5)],
            static_ip: Some(((192, 168, 4, 209), (192, 168, 4, 1))),
            udp_port: 20002,
            i2c_enabled: true,
            motors_enabled: true,
        }
    }
}

/// Messages from the client
#[derive(Copy, Clone, Serialize)]
pub struct PacbotCommand {
    motors: [MotorRequest; 3],
    pid: [f32; 3],
    pid_limits: [f32; 3],
    max_accel: f32,
    angle: f32,
    mag: f32,
    collision_avoidance: bool,
    collision_distance_threshold: u8,
    collision_avoidance_stop: u8,
    sensor_continuous_interval: u8,
    gui_ip: [u8; 4],
}

/// The way the client wants the motor to be controlled
#[derive(Copy, Clone, Debug, PartialOrd, PartialEq, Serialize)]
pub enum MotorRequest {
    /// Use PID to move the motor to this velocity
    Velocity(f32),
    /// Set PWM to these values directly
    Pwm(u16, u16),
}
