use crate::drive_system::DriveSystem;
use crate::driving::RobotTask;
use crate::driving::Task;
use crate::driving::{error, RobotInterTaskMessage};
use crate::messages::{MotorControlStatus, RobotToServerMessage};
use crate::names::RobotName;
use crate::robot_definition::RobotDefinition;
use core::fmt::Debug;
use core::time::Duration;

pub trait RobotMotorsBehavior: RobotTask {
    type Error: Debug;

    type Instant;
    fn now(&self) -> Self::Instant;
    fn elapsed(&self, instant: &Self::Instant) -> Duration;

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
    config: [[usize; 2]; WHEELS],
    pwm: [[u16; 2]; WHEELS],
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
        pwm: Default::default(),
    };

    let task_start = data.motors.now();

    let mut last_motor_control_status = data.motors.now();
    let run_pid_every = Duration::from_millis(30);

    let mut last_command = data.motors.now();

    loop {
        if data.motors.elapsed(&last_command) > Duration::from_millis(400) {
            // we might have disconnected, set all motors to stop
            data.pwm = Default::default();
            for p in 0..6 {
                data.motors.set_pwm(p, 0).await;
            }
        }

        let time_to_wait =
            run_pid_every.checked_sub(data.motors.elapsed(&last_motor_control_status));

        let time_to_wait = match time_to_wait {
            None => {
                // just skip it if network buffer is full
                data.motors.send_or_drop(
                    RobotInterTaskMessage::ToServer(RobotToServerMessage::MotorControlStatus((
                        data.motors.elapsed(&task_start),
                        MotorControlStatus { pwm: data.pwm },
                    ))),
                    Task::Wifi,
                );
                last_motor_control_status = data.motors.now();
                run_pid_every
                    .checked_sub(data.motors.elapsed(&last_motor_control_status))
                    .unwrap()
            }
            Some(t) => t,
        };

        match data.motors.receive_message_timeout(time_to_wait).await {
            Some(RobotInterTaskMessage::TargetVelocity(_lin, _ang)) => {
                last_command = data.motors.now();
                // todo
            }
            Some(RobotInterTaskMessage::PwmOverride(overrides)) => {
                last_command = data.motors.now();
                for m in 0..3 {
                    for i in 0..2 {
                        data.pwm[m][i] = overrides[m][i].unwrap_or(0);
                        data.motors.set_pwm(data.config[m][i], data.pwm[m][i]).await;
                    }
                }
            }
            Some(RobotInterTaskMessage::MotorConfig(config)) => {
                data.config = config;
            }
            _ => {}
        }
    }
}
