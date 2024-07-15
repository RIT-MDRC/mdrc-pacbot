use crate::driving::{error, info};
use crate::driving::{RobotInterTaskMessage, RobotTask};
use crate::names::RobotName;
use core::fmt::Debug;

pub trait RobotMotorsBehavior: RobotTask {
    type Error: Debug;

    /// Whether this task should attempt to continuously compute PID for motors
    ///
    /// Generally, simulated robots should return false, while real robots should return true
    fn do_pid(&self) -> bool;

    /// Set the given motor to the speed in rad/s; only called if do_pid is false
    async fn set_motor_speed(&mut self, index: usize, to: f32);
}

pub async fn motors_task<T: RobotMotorsBehavior>(
    name: RobotName,
    mut motors: T,
) -> Result<(), T::Error> {
    if motors.do_pid() {
        error!("PID not yet implemented!");
        todo!()
    }

    let drive_system = name.robot().drive_system;

    loop {
        #[allow(irrefutable_let_patterns)]
        if let RobotInterTaskMessage::TargetVelocity(lin, ang) = motors.receive_message().await {
            info!("{} received new motor velocities", name);
            let outputs = drive_system.get_motor_speed_omni(lin, ang);

            for (i, v) in outputs.iter().enumerate() {
                motors.set_motor_speed(i, *v).await;
            }
        }
    }
}
