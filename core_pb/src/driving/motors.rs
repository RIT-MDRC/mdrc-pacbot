use crate::drive_system::DriveSystem;
use crate::driving::info;
use crate::driving::RobotTask;
use crate::driving::{error, RobotInterTaskMessage};
use crate::names::RobotName;
use crate::robot_definition::RobotDefinition;
use core::fmt::Debug;
use core::time::Duration;

pub trait RobotMotorsBehavior: RobotTask {
    type Error: Debug;

    /// Whether this task should attempt to continuously compute PID for motors
    ///
    /// Generally, simulated robots should return false, while real robots should return true
    fn do_pid(&self) -> bool;

    /// Set PWM for the given pin
    ///
    /// - 0 <= pin < 2*WHEELS
    /// - 0 <= to <= [`robot_definition.pwm_max`]
    async fn set_pwm(&mut self, pin: usize, to: u16);
}

#[allow(dead_code)]
struct MotorsData<const WHEELS: usize, T: RobotMotorsBehavior> {
    name: RobotName,
    robot: RobotDefinition<WHEELS>,
    drive_system: DriveSystem<WHEELS>,

    motors: T,
    config: [[usize; 2]; 3],
}

pub async fn motors_task<T: RobotMotorsBehavior>(
    name: RobotName,
    motors: T,
) -> Result<(), T::Error> {
    let robot = RobotDefinition::default();
    let config = robot.default_motor_config;

    if motors.do_pid() {
        error!("PID not yet implemented!");
        todo!()
    }

    let drive_system = robot.drive_system;

    let mut data = MotorsData {
        name,
        robot,
        drive_system,

        motors,
        config,
    };

    loop {
        match data
            .motors
            .receive_message_timeout(Duration::from_millis(500))
            .await
        {
            Some(RobotInterTaskMessage::TargetVelocity(_lin, _ang)) => {
                // todo
            }
            Some(RobotInterTaskMessage::PwmOverride(overrides)) => {
                for m in 0..3 {
                    for i in 0..2 {
                        data.motors
                            .set_pwm(data.config[m][i], overrides[m][i].unwrap_or(0))
                            .await;
                    }
                }
            }
            Some(RobotInterTaskMessage::MotorConfig(config)) => {
                data.config = config;
            }
            None => {
                // sleep finished, set all motors to stop
                for p in 0..6 {
                    data.motors.set_pwm(p, 0).await;
                }
            }
        }
    }
}
