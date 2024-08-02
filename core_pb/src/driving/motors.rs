use crate::drive_system::DriveSystem;
use crate::driving::RobotInterTaskMessage;
use crate::driving::RobotTask;
use crate::driving::Task;
use crate::messages::{MotorControlStatus, RobotToServerMessage};
use crate::names::RobotName;
use crate::robot_definition::RobotDefinition;
use crate::util::CrossPlatformInstant;
use core::fmt::Debug;
use core::time::Duration;
#[cfg(not(feature = "std"))]
use nalgebra::ComplexField;
use nalgebra::Vector2;
use pid::Pid;

pub trait RobotMotorsBehavior: RobotTask {
    type Error: Debug;

    type Instant: CrossPlatformInstant + Default;

    /// Set PWM for the given pin
    ///
    /// - 0 <= pin < 2*WHEELS
    /// - 0 <= to <= [`robot_definition.pwm_max`]
    async fn set_pwm(&mut self, pin: usize, to: u16);

    async fn get_motor_speed(&mut self, motor: usize) -> f32;
}

#[allow(dead_code)]
struct MotorsData<const WHEELS: usize, T: RobotMotorsBehavior> {
    name: RobotName,
    robot: RobotDefinition<WHEELS>,
    drive_system: DriveSystem<WHEELS>,

    motors: T,
    config: [[usize; 2]; WHEELS],
    pid: [f32; 3],
    pid_controllers: [Pid<f32>; WHEELS],
    pwm_override: [[Option<u16>; 2]; WHEELS],
    motor_override: [Option<f32>; WHEELS],
    target_vel: Option<(Vector2<f32>, f32)>,

    set_points: [f32; WHEELS],
    pwm: [[u16; 2]; WHEELS],
}

pub async fn motors_task<T: RobotMotorsBehavior>(
    name: RobotName,
    motors: T,
) -> Result<(), T::Error> {
    let robot = name.robot();
    let config = robot.default_motor_config;
    let pid = robot.default_pid;

    let pid_controllers = [0; 3].map(|_| {
        let mut pid_controller = Pid::new(0.0, robot.pwm_top as f32);
        pid_controller
            .p(pid[0], robot.pwm_top as f32)
            .i(pid[1], robot.pwm_top as f32)
            .d(pid[2], robot.pwm_top as f32);
        pid_controller
    });

    let drive_system = robot.drive_system;

    let mut data = MotorsData {
        name,
        robot,
        drive_system,

        motors,
        config,
        pid,
        pid_controllers,
        pwm_override: Default::default(),
        motor_override: Default::default(),
        target_vel: Default::default(),

        set_points: Default::default(),
        pwm: Default::default(),
    };

    let task_start = T::Instant::default();

    let mut last_motor_control_status = T::Instant::default();
    let run_pid_every = Duration::from_millis(30);

    let mut last_command = T::Instant::default();

    loop {
        if last_command.elapsed() > Duration::from_millis(300) {
            // we might have disconnected, set all motors to stop
            data.pwm_override = Default::default();
            data.motor_override = Default::default();
            data.target_vel = None;
            for p in 0..6 {
                data.motors.set_pwm(p, 0).await;
            }
        }

        let time_to_wait = run_pid_every.checked_sub(last_motor_control_status.elapsed());

        let time_to_wait = match time_to_wait {
            None => {
                let measured_speeds = [
                    data.motors.get_motor_speed(0).await,
                    data.motors.get_motor_speed(1).await,
                    data.motors.get_motor_speed(2).await,
                ];
                data.set_points = [0.0; 3];
                data.pwm = [[0; 2]; 3];
                if let Some((lin, ang)) = data.target_vel {
                    data.set_points = data.drive_system.get_motor_speed_omni(lin, ang);
                }
                for m in 0..3 {
                    if let Some(motor_override) = data.motor_override[m] {
                        data.set_points[m] = motor_override;
                    }
                    // calculate pid
                    data.pid_controllers[m].setpoint(data.set_points[m]);
                    let output = data.pid_controllers[m].next_control_output(measured_speeds[m]);
                    let output = output.output;
                    if output > 0.0 {
                        data.pwm[m] = [output.abs().round() as u16, 0];
                    } else {
                        data.pwm[m] = [0, output.abs().round() as u16];
                    }
                    for p in 0..2 {
                        if let Some(pwm_override) = data.pwm_override[m][p] {
                            data.pwm[m][p] = pwm_override;
                        }
                        data.motors.set_pwm(data.config[m][p], data.pwm[m][p]).await;
                    }
                }
                data.motors.send_or_drop(
                    RobotInterTaskMessage::ToServer(RobotToServerMessage::MotorControlStatus((
                        task_start.elapsed(),
                        MotorControlStatus {
                            pwm: data.pwm,
                            measured_speeds,
                            speed_set_points: data.set_points,
                        },
                    ))),
                    Task::Wifi,
                );
                last_motor_control_status = T::Instant::default();
                run_pid_every
                    .checked_sub(last_motor_control_status.elapsed())
                    .unwrap()
            }
            Some(t) => t,
        };

        match data.motors.receive_message_timeout(time_to_wait).await {
            Some(RobotInterTaskMessage::TargetVelocity(vel)) => {
                last_command = T::Instant::default();
                data.target_vel = vel;
            }
            Some(RobotInterTaskMessage::MotorsOverride(overrides)) => {
                last_command = T::Instant::default();
                data.motor_override = overrides;
            }
            Some(RobotInterTaskMessage::PwmOverride(overrides)) => {
                last_command = T::Instant::default();
                data.pwm_override = overrides;
            }
            Some(RobotInterTaskMessage::MotorConfig(config)) => {
                data.config = config;
            }
            Some(RobotInterTaskMessage::Pid(pid)) => {
                data.pid = pid;
                for pid_controller in &mut data.pid_controllers {
                    pid_controller
                        .p(pid[0], robot.pwm_top as f32)
                        .i(pid[1], robot.pwm_top as f32)
                        .d(pid[2], robot.pwm_top as f32);
                }
            }
            _ => {}
        }
    }
}
