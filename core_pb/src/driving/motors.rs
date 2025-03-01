use crate::drive_system::DriveSystem;
use crate::driving::data::SharedRobotData;
use crate::driving::RobotBehavior;
use crate::messages::{
    FrequentServerToRobot, MotorControlStatus, SensorData, Task, VelocityControl,
};
use crate::pure_pursuit::pure_pursuit;
use crate::util::utilization::UtilizationMonitor;
use crate::util::CrossPlatformInstant;
use core::sync::atomic::Ordering;
use core::time::Duration;
#[cfg(feature = "micromath")]
use micromath::F32Ext;
use nalgebra::{Rotation2, Vector2};
use pid::Pid;

/// Functionality that robots with motors must support
pub trait RobotMotorsBehavior {
    /// Set PWM for the given pin
    ///
    /// - 0 <= pin < 2*WHEELS
    /// - 0 <= to <= [`robot_definition.pwm_max`]
    async fn set_pwm(&mut self, pin: usize, to: u16);
}

struct MotorsData<const WHEELS: usize, M: RobotMotorsBehavior> {
    motors: M,

    config: FrequentServerToRobot,

    pid_controllers: [Pid<f32>; WHEELS],

    motor_speeds: [f32; 3],
    set_points: [f32; WHEELS],
    pwm: [[u16; 2]; WHEELS],
}

/// The "main" method for the motors task
pub async fn motors_task<R: RobotBehavior>(data: &SharedRobotData<R>, motors: R::Motors) -> ! {
    let status_sender = data.motor_control.sender();
    // Watch for config changes instead of just using .get() to enable detecting when no one is
    // setting new configs, and turn off motors
    let mut config_watch = data.config.receiver().unwrap();

    let robot = &data.robot_definition;
    let config = FrequentServerToRobot::new(data.name);
    let pid = config.pid;

    let pid_controllers = [0; 3].map(|_| {
        let mut pid_controller = Pid::new(0.0, robot.pwm_top as f32);
        pid_controller
            .p(pid[0], robot.pwm_top as f32)
            .i(pid[1], robot.pwm_top as f32)
            .d(pid[2], robot.pwm_top as f32);
        pid_controller
    });

    let mut motors_data = MotorsData {
        config,

        motors,
        pid_controllers,

        motor_speeds: [0.0; 3],
        set_points: Default::default(),
        pwm: Default::default(),
    };

    let mut last_command = R::Instant::default();

    let mut utilization_monitor: UtilizationMonitor<50, R::Instant> =
        UtilizationMonitor::new(0.0, 0.0);
    utilization_monitor.start();

    loop {
        if let Some(config) = config_watch.try_changed() {
            last_command = R::Instant::default();
            motors_data.config = config;
            for m in 0..3 {
                motors_data.pid_controllers[m]
                    .p(motors_data.config.pid[0], robot.pwm_top as f32)
                    .i(motors_data.config.pid[1], robot.pwm_top as f32)
                    .d(motors_data.config.pid[2], robot.pwm_top as f32);
            }
        }
        if last_command.elapsed() > Duration::from_millis(300) {
            // we might have disconnected, set all motors to stop
            motors_data.config = FrequentServerToRobot::new(data.name);
            motors_data.config.pwm_override = [[Some(0); 2]; 3];
        }
        if let Some(new_speeds) = data.sig_motor_speeds.try_take() {
            for (i, speed) in motors_data.motor_speeds.iter_mut().enumerate() {
                *speed = new_speeds[motors_data.config.encoder_config[i].0]
                    * if motors_data.config.encoder_config[i].1 {
                        -1.0
                    } else {
                        1.0
                    };
            }
        }

        motors_data
            .do_motors(&data.robot_definition.drive_system, &data.sensors.try_get())
            .await;

        status_sender.send(MotorControlStatus {
            pwm: motors_data.pwm,
            measured_speeds: motors_data.motor_speeds,
            speed_set_points: motors_data.set_points,
        });
        data.utilization[Task::Motors as usize]
            .store(utilization_monitor.utilization(), Ordering::Relaxed);

        utilization_monitor.stop();
        R::Instant::sleep(Duration::from_millis(30)).await;
        utilization_monitor.start();
    }
}

fn adjust_ang_vel(curr_ang: f32, desired_ang: f32, p: f32, tol: f32) -> f32 {
    // Calculate the difference between desired angle and current angle
    let mut angle_diff = desired_ang - curr_ang;

    // account for angles that cross discontinuity
    if angle_diff > core::f32::consts::PI {
        angle_diff -= 2.0 * core::f32::consts::PI;
    } else if angle_diff < -core::f32::consts::PI {
        angle_diff += 2.0 * core::f32::consts::PI;
    }

    // clamp if within tol rads
    angle_diff = if angle_diff.abs() < tol {
        0.0
    } else {
        angle_diff
    };

    angle_diff * p
}

impl<M: RobotMotorsBehavior> MotorsData<3, M> {
    pub async fn do_motors(&mut self, drive_system: &DriveSystem<3>, sensors: &Option<SensorData>) {
        // TODO: make this a tunable param
        let angle_p = 2.0;
        let angle_tol = 0.03; // rad

        if self.config.follow_target_path {
            if let Some(sensors) = sensors {
                let mut target_velocity = (Vector2::new(0.0, 0.0), 0.0);
                // maintain heading 0
                if let Ok(angle) = sensors.angle {
                    target_velocity.1 = adjust_ang_vel(angle, 0.0, angle_p, angle_tol);
                    let angle = Rotation2::new(angle).angle();
                    if angle.abs() < 20.0_f32.to_radians() {
                        // now that we've made sure we're facing the right way, try to follow the path
                        if let Some(vel) = pure_pursuit(
                            sensors,
                            &self.config.target_path,
                            self.config.lookahead_dist,
                            self.config.robot_speed,
                            self.config.snapping_dist,
                        ) {
                            target_velocity.0 = vel;
                        }
                    }
                }
                // calculate wheel velocities
                self.config.target_velocity =
                    VelocityControl::LinVelAngVel(target_velocity.0, target_velocity.1);
            }
        }

        self.set_points = [0.0; 3];
        self.pwm = [[0; 2]; 3];
        if let Some((lin, ang)) = match self.config.target_velocity {
            VelocityControl::None | VelocityControl::AssistedDriving(_) => None,
            VelocityControl::Stop => Some((Vector2::new(0.0, 0.0), 0.0)),
            VelocityControl::LinVelAngVel(lin, ang) => Some((lin, ang)),
            VelocityControl::LinVelFixedAng(lin, set_ang) => sensors
                .as_ref()
                .and_then(|s| s.angle.clone().ok())
                .map(|cur_ang| (lin, adjust_ang_vel(cur_ang, set_ang, angle_p, angle_tol))),
            VelocityControl::LinVelFaceForward(lin) => sensors
                .as_ref()
                .and_then(|s| s.angle.clone().ok())
                .map(|cur_ang| {
                    (
                        lin,
                        if lin.magnitude() < 0.01 {
                            0.0
                        } else {
                            crate::driving::motors::adjust_ang_vel(
                                cur_ang,
                                f32::atan2(lin.y, lin.x),
                                angle_p,
                                angle_tol,
                            )
                        },
                    )
                }),
        } {
            self.set_points = drive_system.get_motor_speed_omni(lin, ang);
        }
        #[allow(clippy::needless_range_loop)]
        for m in 0..3 {
            if let Some(motor_override) = self.config.motors_override[m] {
                self.set_points[m] = motor_override;
            }
            // calculate pid
            self.pid_controllers[m].setpoint(self.set_points[m]);
            let output = if self.set_points[m] == 0.0 {
                self.pid_controllers[m].reset_integral_term();
                0.0
            } else {
                self.pid_controllers[m]
                    .next_control_output(self.motor_speeds[m])
                    .output
            };

            // set value to PWM on motors
            if output > 0.0 {
                self.pwm[m] = [output.abs().round() as u16, 0];
            } else {
                self.pwm[m] = [0, output.abs().round() as u16];
            }
            for p in 0..2 {
                if let Some(pwm_override) = self.config.pwm_override[m][p] {
                    self.pwm[m][p] = pwm_override;
                }
                self.motors
                    .set_pwm(self.config.motor_config[m][p], self.pwm[m][p])
                    .await;
            }
        }
    }
}
