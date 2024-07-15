use crate::driving::TaskChannels;
use crate::RobotToSimulationMessage;
use async_channel::Sender;
use core_pb::drive_system::DriveSystem;
use core_pb::driving::motors::RobotMotorsBehavior;
use core_pb::driving::{RobotInterTaskMessage, RobotTask, Task};
use core_pb::names::RobotName;

pub struct SimMotors {
    name: RobotName,
    drive_system: DriveSystem<3>,
    channels: TaskChannels,

    motor_speeds: [f32; 3],
    sim_tx: Sender<(RobotName, RobotToSimulationMessage)>,
}

impl SimMotors {
    pub fn new(
        name: RobotName,
        channels: TaskChannels,
        sim_tx: Sender<(RobotName, RobotToSimulationMessage)>,
    ) -> Self {
        Self {
            name,
            drive_system: name.robot().drive_system,
            channels,
            motor_speeds: Default::default(),
            sim_tx,
        }
    }
}

#[derive(Debug)]
pub enum SimMotorsError {}

impl RobotTask for SimMotors {
    async fn send_message(&mut self, message: RobotInterTaskMessage, to: Task) -> Result<(), ()> {
        self.channels.send_message(message, to).await
    }

    async fn receive_message(&mut self) -> RobotInterTaskMessage {
        self.channels.receive_message().await
    }
}

impl RobotMotorsBehavior for SimMotors {
    type Error = SimMotorsError;

    fn do_pid(&self) -> bool {
        false
    }

    async fn set_motor_speed(&mut self, index: usize, to: f32) {
        self.motor_speeds[index] = to;
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
