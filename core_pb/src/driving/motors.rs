use crate::driving::RobotTask;
use core::fmt::Debug;

pub trait RobotMotorsBehavior: RobotTask {
    type Error: Debug;
}

pub async fn motors_task<T: RobotMotorsBehavior>(mut motors: T) -> Result<(), T::Error> {
    let _ = motors.receive_message().await;

    Ok(())
}
