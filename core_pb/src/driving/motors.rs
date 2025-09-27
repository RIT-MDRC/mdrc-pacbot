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
use nalgebra::Vector2;
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

    // PID controllers configured with P, I, and D constants from the config/GUI.
    pid_controllers: [Pid<f32>; WHEELS],

    // Measurements of current motor speeds.
    motor_speeds: [f32; WHEELS],
    // Measurements of current target velocities.
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
    let pid = config.pidsv;

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

    let mut cv_over_time = motors_data.config.cv_location;
    let mut cv_over_time_time = R::Instant::default();
    let mut last_motor_speeds = [0, 1, 2].map(|i| data.sig_motor_speeds[i].load(Ordering::Relaxed));

    loop {
        if let Some(config) = config_watch.try_changed() {
            last_command = R::Instant::default();
            if config.cv_location != cv_over_time {
                cv_over_time = config.cv_location;
                cv_over_time_time = R::Instant::default();
            }
            motors_data.config = config;
            for m in 0..3 {
                motors_data.pid_controllers[m]
                    .p(motors_data.config.pidsv[0], robot.pwm_top as f32)
                    .i(motors_data.config.pidsv[1], robot.pwm_top as f32)
                    .d(motors_data.config.pidsv[2], robot.pwm_top as f32);
            }
        }
        if last_command.elapsed() > Duration::from_millis(300) {
            // we might have disconnected, set all motors to stop
            motors_data.config = FrequentServerToRobot::new(data.name);
            motors_data.config.pwm_override = [[Some(0); 2]; 3];
        }
        let new_speeds = [0, 1, 2].map(|i| data.sig_motor_speeds[i].load(Ordering::Relaxed));
        if new_speeds != last_motor_speeds {
            last_motor_speeds = new_speeds;
            for (i, speed) in motors_data.motor_speeds.iter_mut().enumerate() {
                *speed = new_speeds[motors_data.config.encoder_config[i].0]
                    * if motors_data.config.encoder_config[i].1 {
                        -1.0
                    } else {
                        1.0
                    };
            }
        }

        let is_enabled = cv_over_time.is_some()
            && motors_data.config.follow_target_path
            && motors_data.config.target_path.len() > 0;
        if !is_enabled {
            cv_over_time_time = R::Instant::default();
        }
        let stuck = cv_over_time_time.elapsed() > Duration::from_secs(3) && is_enabled;
        // if cv_over_time_time.elapsed() > Duration::from_secs(2) {
        //     cv_over_time_time = R::Instant::default();
        // }
        // data.set_extra_bool_indicator(1, stuck);
        // data.set_extra_i32_indicator(
        //     1,
        //     if !stuck {
        //         0
        //     } else {
        //         cv_over_time_time.elapsed().as_secs() as i32
        //     },
        // );
        motors_data
            .do_motors(
                &data.robot_definition.drive_system,
                &data.sensors.try_get(),
                if !stuck {
                    0
                } else {
                    cv_over_time_time.elapsed().as_secs()
                },
                3.0, // todo make this into an option
                2.0,
                0.05,
                0.0,
            )
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

fn adjust_ang_vel(curr_ang: f32, desired_ang: f32, p: f32, tol: f32, offset: f32) -> f32 {
    // Calculate the difference between desired angle and current angle
    let mut angle_diff = desired_ang - curr_ang;

    // account for angles that cross discontinuity
    if angle_diff > core::f32::consts::PI {
        angle_diff -= 2.0 * core::f32::consts::PI;
    } else if angle_diff < -core::f32::consts::PI {
        angle_diff += 2.0 * core::f32::consts::PI;
    }

    // clamp if within tol rads
    if angle_diff.abs() < tol {
        0.0
    } else {
        angle_diff * p + offset * angle_diff.signum()
    }
}

impl<M: RobotMotorsBehavior> MotorsData<3, M> {
    pub async fn do_motors(
        &mut self,
        drive_system: &DriveSystem<3>,
        sensors: &Option<SensorData>,
        stuck_time: u64,
        snapping_multiplier: f32,
        angle_p: f32,
        angle_tol: f32, // rad
        angle_snapping_offset: f32,
    ) {
        if self.config.follow_target_path {
            if let Some(sensors) = sensors {
                let mut target_velocity = (Vector2::new(0.0, 0.0), 0.0);
                // maintain heading 0
                if let Ok(angle) = sensors.angle {
                    target_velocity.1 =
                        adjust_ang_vel(angle, 0.0, angle_p, angle_tol, angle_snapping_offset);
                    // let angle = Rotation2::new(angle).angle();
                    // if angle.abs() < 20.0_f32.to_radians() {
                    // now that we've made sure we're facing the right way, try to follow the path
                    if let Some(vel) = pure_pursuit(
                        sensors,
                        &self.config.target_path,
                        if stuck_time > 2 {
                            self.config.lookahead_dist * 0.1
                        } else {
                            self.config.lookahead_dist
                        },
                        self.config.robot_speed,
                        self.config.snapping_dist,
                        snapping_multiplier,
                        self.config.cv_location,
                    ) {
                        target_velocity.0 = vel;
                        if stuck_time % 6 > 3 {
                            // let phase = stuck_time % 4;
                            // let speed = 2.5;
                            // let angle = (180.0 as f32).to_radians();
                            // let x_speed = angle.cos() * speed;
                            // let y_speed = angle.sin() * speed;
                            // target_velocity.0 = Vector2::new(x_speed, y_speed);
                            target_velocity.0 = -target_velocity.0;
                            target_velocity.1 = -target_velocity.1;
                        }
                    }
                    // }
                }
                // calculate wheel velocities
                self.config.target_velocity =
                    VelocityControl::LinVelAngVel(target_velocity.0, target_velocity.1);
            }
        }

        self.set_points = [0.0; 3];
        self.pwm = [[0; 2]; 3];
        if let Some((mut lin, ang)) = match self.config.target_velocity {
            VelocityControl::None | VelocityControl::AssistedDriving(_) => None,
            VelocityControl::Stop => Some((Vector2::new(0.0, 0.0), 0.0)),
            VelocityControl::LinVelAngVel(lin, ang) => Some((lin, ang)),
            VelocityControl::LinVelFixedAng(lin, set_ang) => sensors
                .as_ref()
                .and_then(|s| s.angle.clone().ok())
                .map(|cur_ang| {
                    (
                        lin,
                        adjust_ang_vel(cur_ang, set_ang, angle_p, angle_tol, angle_snapping_offset),
                    )
                }),
            VelocityControl::LinVelFaceForward(lin) => sensors
                .as_ref()
                .and_then(|s| s.angle.clone().ok())
                .map(|cur_ang| {
                    (
                        lin,
                        if lin.magnitude() < 0.01 {
                            0.0
                        } else {
                            adjust_ang_vel(
                                cur_ang,
                                f32::atan2(lin.y, lin.x),
                                angle_p,
                                angle_tol,
                                angle_snapping_offset,
                            )
                        },
                    )
                }),
        } {
            if let Some(SensorData {
                angle: Ok(angle), ..
            }) = &sensors
            {
                lin = Vector2::new(
                    lin.x * (-angle).cos() - lin.y * (-angle).sin(),
                    lin.x * (-angle).sin() + lin.y * (-angle).cos(),
                );
            }
            // let lin_x = lin.x *
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
                // TODO: Check this, but I think this makes more sense
                let target_velocity = self.set_points[m];

                let s = self.config.pidsv[3];
                let v = self.config.pidsv[4];

                // Get output from PID
                let pid_output = self.pid_controllers[m]
                    .next_control_output(self.motor_speeds[m])
                    .output;

                // Calculate static friction (basically just whether or not the bot is moving)
                let static_friction = if target_velocity.abs() > 0.0 { s } else { 0.0 };

                // Add the feedforward
                let velocity_feedforward = v * target_velocity;

                // Output = PID + S + V = PIDSV
                pid_output + static_friction + velocity_feedforward
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
