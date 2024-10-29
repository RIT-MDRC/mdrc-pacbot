use crate::drive_system::DriveSystem;
use crate::driving::RobotInterTaskMessage;
use crate::driving::RobotTaskMessenger;
use crate::driving::Task;
use crate::messages::{
    FrequentServerToRobot, MotorControlStatus, RobotToServerMessage, SensorData,
};
use crate::names::RobotName;
use crate::pure_pursuit::pure_pursuit;
use crate::robot_definition::RobotDefinition;
use crate::util::utilization::UtilizationMonitor;
use crate::util::CrossPlatformInstant;
use core::fmt::Debug;
use core::time::Duration;
#[cfg(not(feature = "std"))]
use nalgebra::ComplexField;
use nalgebra::{Rotation2, Vector2};
use pid::Pid;

/// Functionality that robots with motors must support
pub trait RobotMotorsBehavior {
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

    config: FrequentServerToRobot,

    pid_controllers: [Pid<f32>; WHEELS],

    set_points: [f32; WHEELS],
    pwm: [[u16; 2]; WHEELS],
}

/// The "main" method for the motors task
pub async fn motors_task<T: RobotMotorsBehavior, M: RobotTaskMessenger>(
    name: RobotName,
    motors: T,
    mut msgs: M,
) -> Result<(), T::Error> {
    let robot = name.robot();
    let config = FrequentServerToRobot::new(name);
    let pid = config.pid;

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
        config,

        motors,
        pid_controllers,

        set_points: Default::default(),
        pwm: Default::default(),
    };

    let mut sensors: Option<SensorData> = None;

    let task_start = T::Instant::default();

    let mut last_motor_control_status = T::Instant::default();
    let run_pid_every = Duration::from_millis(30);

    let mut last_command = T::Instant::default();

    let mut utilization_monitor: UtilizationMonitor<50, T::Instant> =
        UtilizationMonitor::new(0.0, 0.0);
    utilization_monitor.start();

    loop {
        if data.config.follow_target_path {
            if let Some(sensors) = &sensors {
                let mut target_velocity = (Vector2::new(0.0, 0.0), 0.0);
                // maintain heading 0
                if let Ok(angle) = sensors.angle {
                    // ensure angle stays in range -pi <= angle < pi
                    let angle = Rotation2::new(angle).angle();
                    if angle.abs() > 1.5_f32.to_radians() {
                        const HEADING_CORRECTION_STRENGTH: f32 = 1.0 / 3.0;
                        target_velocity.1 = -angle * HEADING_CORRECTION_STRENGTH;
                    }
                    if angle.abs() < 5.0_f32.to_radians() {
                        // now that we've made sure we're facing the right way, try to follow the path
                        if let Some(vel) = pure_pursuit(sensors, &data.config.target_path, 0.5) {
                            target_velocity.0 = vel;
                        }
                    }
                }
                // calculate wheel velocities
                data.config.target_velocity = Some(target_velocity);
            }
        }

        if last_command.elapsed() > Duration::from_millis(300) {
            // we might have disconnected, set all motors to stop
            data.config = FrequentServerToRobot::new(data.name);
            data.config.pwm_override = [[Some(0); 2]; 3];
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
                if let Some((lin, ang)) = data.config.target_velocity {
                    data.set_points = data.drive_system.get_motor_speed_omni(lin, ang);
                }
                #[allow(clippy::needless_range_loop)]
                for m in 0..3 {
                    if let Some(motor_override) = data.config.motors_override[m] {
                        data.set_points[m] = motor_override;
                    }
                    // calculate pid
                    data.pid_controllers[m].setpoint(data.set_points[m]);
                    let output = if data.set_points[m] == 0.0 {
                        data.pid_controllers[m].reset_integral_term();
                        0.0
                    } else {
                        data.pid_controllers[m]
                            .next_control_output(measured_speeds[m])
                            .output
                    };

                    // set value to PWM on motors
                    if output > 0.0 {
                        data.pwm[m] = [output.abs().round() as u16, 0];
                    } else {
                        data.pwm[m] = [0, output.abs().round() as u16];
                    }
                    for p in 0..2 {
                        if let Some(pwm_override) = data.config.pwm_override[m][p] {
                            data.pwm[m][p] = pwm_override;
                        }
                        data.motors
                            .set_pwm(data.config.motor_config[m][p], data.pwm[m][p])
                            .await;
                    }
                }
                msgs.send_or_drop(
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
                msgs.send_or_drop(
                    RobotInterTaskMessage::Utilization(
                        utilization_monitor.utilization(),
                        Task::Motors,
                    ),
                    Task::Wifi,
                );
                last_motor_control_status = T::Instant::default();
                run_pid_every
                    .checked_sub(last_motor_control_status.elapsed())
                    .unwrap()
            }
            Some(t) => t,
        };

        utilization_monitor.stop();
        let event = msgs.receive_message_timeout(time_to_wait).await;
        utilization_monitor.start();

        match event {
            Some(RobotInterTaskMessage::FrequentServerToRobot(msg)) => {
                last_command = T::Instant::default();
                data.config = msg;
                for m in 0..3 {
                    data.pid_controllers[m]
                        .p(data.config.pid[0], robot.pwm_top as f32)
                        .i(data.config.pid[1], robot.pwm_top as f32)
                        .d(data.config.pid[2], robot.pwm_top as f32);
                }
            }
            Some(RobotInterTaskMessage::Sensors(new_sensors)) => {
                sensors = Some(new_sensors);
            }
            _ => {}
        }
    }
}
