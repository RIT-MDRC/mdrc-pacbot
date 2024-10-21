use crate::RobotToSimulationMessage;
use async_channel::Sender;
use core_pb::drive_system::DriveSystem;
use core_pb::driving::motors::RobotMotorsBehavior;
use core_pb::names::RobotName;
use core_pb::util::StdInstant;

pub struct SimMotors {
    name: RobotName,
    drive_system: DriveSystem<3>,

    pwm_values: [[u16; 2]; 3],
    motor_speeds: [f32; 3],
    sim_tx: Sender<(RobotName, RobotToSimulationMessage)>,
}

impl SimMotors {
    pub fn new(name: RobotName, sim_tx: Sender<(RobotName, RobotToSimulationMessage)>) -> Self {
        Self {
            name,
            drive_system: name.robot().drive_system,
            pwm_values: Default::default(),
            motor_speeds: Default::default(),
            sim_tx,
        }
    }
}

#[derive(Debug)]
pub enum SimMotorsError {}

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

    async fn get_motor_speed(&mut self, motor: usize) -> f32 {
        self.motor_speeds[motor]
    }
}
