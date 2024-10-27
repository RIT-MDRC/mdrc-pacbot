use crate::RobotToSimulationMessage;
use crate::RwLock;
use crate::SimRobot;
use async_channel::Sender;
use core_pb::driving::motors::RobotMotorsBehavior;
use core_pb::names::RobotName;
use core_pb::util::StdInstant;
use std::sync::Arc;

pub struct SimMotors {
    name: RobotName,
    sim_robot: Arc<RwLock<SimRobot>>,

    pwm_values: [[u16; 2]; 3],
    motor_speeds: [f32; 3],
    sim_tx: Sender<(RobotName, RobotToSimulationMessage)>,
}

impl SimMotors {
    pub fn new(
        name: RobotName,
        sim_tx: Sender<(RobotName, RobotToSimulationMessage)>,
        sim_robot: Arc<RwLock<SimRobot>>,
    ) -> Self {
        Self {
            name,
            pwm_values: Default::default(),
            motor_speeds: Default::default(),
            sim_tx,
            sim_robot,
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
            //converts pid output to simulator velocity
            self.motor_speeds[motor] = 60.0
                * (self.pwm_values[motor][0] as f32 - self.pwm_values[motor][1] as f32)
                / self.name.robot().pwm_top as f32;
            self.sim_tx
                .send((
                    self.name,
                    RobotToSimulationMessage::SimulatedMotors(self.motor_speeds),
                ))
                .await
                .unwrap();
        }
    }

    async fn get_motor_speed(&mut self, motor: usize) -> f32 {
        self.sim_robot.read().unwrap().actual_motor_speeds[motor]
    }
}
