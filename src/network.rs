//! Network communications with the Pico and the game server.

use crate::pathing::TargetVelocity;
use crate::physics::LightPhysicsInfo;
use crate::UserSettings;
use bevy::log::info;
use bevy_ecs::prelude::*;
use bincode;
use serde::{Deserialize, Serialize};
use std::f32::consts::FRAC_PI_3;
use std::time::{Duration, Instant};
use std::{io, net::UdpSocket, thread};

/// Stores data from Pacbot
#[derive(Resource, Copy, Clone, Debug, Serialize, Deserialize)]
pub struct PacbotSensors {
    /// Distance sensor readings, mm
    pub distance_sensors: [u8; 8],
    /// Encoder positions
    pub encoders: [i64; 3],
    /// Velocity of the encoders
    pub encoder_velocities: [f32; 3],
    /// Output from PID
    pub pid_output: [f32; 3],
}

/// Holds the last time when sensor information was received from Pacbot
#[derive(Resource, Default)]
pub struct PacbotSensorsRecvTime(pub(crate) Option<Instant>);

impl Default for PacbotSensors {
    fn default() -> Self {
        Self {
            distance_sensors: [0; 8],
            encoders: [0; 3],
            encoder_velocities: [0.0; 3],
            pid_output: [0.0; 3],
        }
    }
}

/// Stores connections for the NetworkPlugin
#[derive(Default, Resource)]
pub struct NetworkPluginData {
    /// Connection to the Pico
    pico: Option<PicoConnection>,
}

/// Sends current motor commands to the pico
pub fn send_motor_commands(
    mut network_data: ResMut<NetworkPluginData>,
    target_velocity: Res<TargetVelocity>,
    phys_info: Res<LightPhysicsInfo>,
) {
    if let Some(pico) = &mut network_data.pico {
        if let Some(loc) = phys_info.pf_pos {
            let x = target_velocity.0.x;
            let y = target_velocity.0.y;

            let current_angle = loc.rotation.angle();

            // use x and y to find the desired angle
            let angle = y.atan2(x);
            let angle = angle - current_angle;

            let mut scale = (x.powi(2) + y.powi(2)).sqrt();
            if scale != 0.0 {
                scale = 30.0;
            }

            let motor_angles = [
                angle.cos(),
                (angle - (2.0 * FRAC_PI_3)).cos(),
                (angle + (2.0 * FRAC_PI_3)).cos(),
            ];

            let rotate_adjust = if target_velocity.1 > 0.0 {
                -10.0
            } else if target_velocity.1 < 0.0 {
                10.0
            } else {
                0.0
            };

            let motors_i16 = [
                // constant is like max speed - can go up to 255.0
                -(motor_angles[0] * scale + rotate_adjust),
                (motor_angles[1] * scale + rotate_adjust),
                -(motor_angles[2] * scale + rotate_adjust),
            ];

            let mut motors = [0.0; 3];
            for i in 0..3 {
                motors[i] = motors_i16[i];
            }

            if let Err(e) = pico.send_motors_message(motors) {
                eprintln!("{:?}", e);
                network_data.pico = None;
            }
        } else {
            let motors = [0.0; 3];

            if let Err(e) = pico.send_motors_message(motors) {
                eprintln!("{:?}", e);
                network_data.pico = None;
            }
        }
    }
}

/// Attempts to reconnect to the pico if not currently connected
pub fn reconnect_pico(mut network_data: ResMut<NetworkPluginData>, settings: Res<UserSettings>) {
    if settings.pico_address.is_none() {
        network_data.pico = None;
    }
    if network_data.pico.is_none() {
        if let Some(pico_address) = &settings.pico_address {
            if pico_address.len() == 0 {
                return;
            }
            let try_conn = PicoConnection::new(20002, &pico_address);
            if let Err(ref e) = try_conn {
                info!("{:?}", e);
            }
            network_data.pico = try_conn.ok();
            if let Some(pico) = &mut network_data.pico {
                if let Err(e) = pico.socket.set_nonblocking(true) {
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
        while let Ok(size) = pico.socket.recv(&mut bytes) {
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
    }
    if recv_time.0.unwrap_or(Instant::now()).elapsed() > Duration::from_secs(1) {
        network_data.pico = None;
    }
}

struct PicoConnection {
    socket: UdpSocket,
}

#[allow(dead_code)]
impl PicoConnection {
    fn new(local_port: u16, remote_address: &str) -> io::Result<Self> {
        let socket = UdpSocket::bind(("0.0.0.0", local_port))?;
        socket.connect(remote_address)?;
        Ok(Self { socket })
    }

    fn send_message(&mut self, message: &[u8]) -> io::Result<()> {
        self.socket.send(message)?;
        Ok(())
    }

    fn send_motors_message(&mut self, motors: [f32; 3]) -> io::Result<()> {
        let message = PacbotCommand {
            velocities: motors,
            pid: [5.0, 0.1, 0.0],
            pid_limits: [10000.0, 10000.0, 10000.0],
        };
        self.socket.set_nonblocking(false).unwrap();
        let r = self.send_message(
            &bincode::serde::encode_to_vec(&message, bincode::config::standard()).unwrap(),
        );
        self.socket.set_nonblocking(true).unwrap();
        r
    }
}

#[derive(Copy, Clone, Serialize)]
struct PacbotCommand {
    pub velocities: [f32; 3],
    pub pid: [f32; 3],
    pub pid_limits: [f32; 3],
}
