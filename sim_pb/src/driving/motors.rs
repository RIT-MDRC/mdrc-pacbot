use crate::RwLock;
use crate::SimRobot;
use core_pb::driving::motors::RobotMotorsBehavior;
use core_pb::names::RobotName;
use std::sync::Arc;

pub struct SimMotors<const WHEELS: usize> {
    name: RobotName,
    sim_robot: Arc<RwLock<SimRobot<WHEELS>>>,

    pwm_values: [[u16; 2]; 3],
}

impl<const WHEELS: usize> SimMotors<WHEELS> {
    pub fn new(name: RobotName, sim_robot: Arc<RwLock<SimRobot<WHEELS>>>) -> Self {
        Self {
            name,
            pwm_values: Default::default(),
            sim_robot,
        }
    }
}

impl<const WHEELS: usize> RobotMotorsBehavior for SimMotors<WHEELS> {
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
}
