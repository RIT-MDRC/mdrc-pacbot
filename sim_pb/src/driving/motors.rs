use crate::RwLock;
use crate::SimRobot;
use core_pb::driving::motors::RobotMotorsBehavior;
use core_pb::names::RobotName;
use core_pb::util::StdInstant;
use std::sync::Arc;

pub struct SimMotors {
    name: RobotName,
    sim_robot: Arc<RwLock<SimRobot>>,

    pwm_values: [[u16; 2]; 3],
}

impl SimMotors {
    pub fn new(name: RobotName, sim_robot: Arc<RwLock<SimRobot>>) -> Self {
        Self {
            name,
            pwm_values: Default::default(),
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
            self.sim_robot.write().unwrap().requested_motor_speeds[motor] = 60.0
                * (self.pwm_values[motor][0] as f32 - self.pwm_values[motor][1] as f32)
                / self.name.robot().pwm_top as f32;
        }
    }

    async fn get_motor_speed(&mut self, motor: usize) -> f32 {
        self.sim_robot.read().unwrap().actual_motor_speeds[motor]
    }
}
