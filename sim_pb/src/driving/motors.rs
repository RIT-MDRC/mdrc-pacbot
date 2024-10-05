use crate::driving::TaskChannels;
use crate::RobotToSimulationMessage;
use async_channel::Sender;
use core_pb::drive_system::DriveSystem;
use core_pb::driving::motors::RobotMotorsBehavior;
use core_pb::driving::{RobotInterTaskMessage, RobotTask, Task};
use core_pb::names::RobotName;
use core_pb::util::StdInstant;
use std::sync::Arc;
use std::time::Duration;
use crate::SimRobot;
use crate::RwLock;


pub struct SimMotors {
    name: RobotName,
    drive_system: DriveSystem<3>,
    channels: TaskChannels,
    sim_robot:  Arc<RwLock<SimRobot>>,

    pwm_values: [[u16; 2]; 3],
    motor_speeds: [f32; 3],
    sim_tx: Sender<(RobotName, RobotToSimulationMessage)>,
}

impl SimMotors {
    pub fn new(
        name: RobotName,
        channels: TaskChannels,
        sim_tx: Sender<(RobotName, RobotToSimulationMessage)>,
        sim_robot: Arc<RwLock<SimRobot>>,
    ) -> Self {
        Self {
            name,
            drive_system: name.robot().drive_system,
            channels,
            pwm_values: Default::default(),
            motor_speeds: Default::default(),
            sim_tx,
            sim_robot
        }
    }
}

#[derive(Debug)]
pub enum SimMotorsError {}

impl RobotTask for SimMotors {
    fn send_or_drop(&mut self, message: RobotInterTaskMessage, to: Task) -> bool {
        self.channels.send_or_drop(message, to)
    }

    async fn send_blocking(&mut self, message: RobotInterTaskMessage, to: Task) {
        self.channels.send_blocking(message, to).await
    }

    async fn receive_message(&mut self) -> RobotInterTaskMessage {
        self.channels.receive_message().await
    }

    async fn receive_message_timeout(
        &mut self,
        timeout: Duration,
    ) -> Option<RobotInterTaskMessage> {
        self.channels.receive_message_timeout(timeout).await
    }
}

impl RobotMotorsBehavior for SimMotors {
    type Error = SimMotorsError;

    type Instant = StdInstant;

    async fn set_pwm(&mut self, pin: usize, to: u16) {
        let motor = pin / 2;
        if self.pwm_values[motor][pin % 2] != to {
            self.pwm_values[motor][pin % 2] = to;
            self.motor_speeds[motor] = 60.0
                * (self.pwm_values[motor][0] as f32 - self.pwm_values[motor][1] as f32)
                / self.name.robot().pwm_top as f32;
            let (lin, ang) = self.drive_system.get_actual_vel_omni(self.motor_speeds);
            self.sim_tx
                .send((
                    self.name,
                    RobotToSimulationMessage::SimulatedVelocity(lin, ang),
                ))
                .await
                .unwrap();
        }
    }
//TODO: grab velocity from sim robot, pass to get_motor_speed_omni from drive system, index into the array
    async fn get_motor_speed(&mut self, motor: usize) -> f32 {
        self.motor_speeds[motor]
    }
}
