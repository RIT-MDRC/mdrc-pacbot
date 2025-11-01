use crate::constants::INCHES_PER_GU;
use crate::driving::data::SharedRobotData;
use crate::driving::RobotBehavior;
use crate::messages::settings::LocalizationAlgorithmSource;
use crate::messages::{RobotButton, SensorData, Task, MAX_SENSOR_ERR_LEN};
// use crate::region_localization::estimate_location_2;
use crate::region_localization;
use crate::localization;
use crate::robot_display::DisplayManager;
use crate::util::utilization::UtilizationMonitor;
use crate::util::CrossPlatformInstant;
use array_init::array_init;
use core::fmt::Debug;
use core::sync::atomic::Ordering;
use core::time::Duration;
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::DrawTarget;
#[cfg(feature = "micromath")]
use micromath::F32Ext;
use nalgebra::{Point2, Vector2};

/// Functionality that robots with peripherals must support
pub trait RobotPeripheralsBehavior {
    type Display: DrawTarget<Color = BinaryColor>;
    type Error: Debug;

    async fn draw_display<F>(&mut self, draw: F)
    where
        F: FnOnce(&mut Self::Display) -> Result<(), <Self::Display as DrawTarget>::Error>;

    async fn flip_screen(&mut self);

    async fn read_button_event(&mut self) -> Option<(RobotButton, bool)>;

    async fn read_joystick(&mut self) -> Option<(f32, f32)>;
}

/// The "main" method for the peripherals task
pub async fn peripherals_task<R: RobotBehavior>(
    data: &SharedRobotData<R>,
    mut peripherals: R::Peripherals,
) {
    let mut sensors = SensorData {
        angle: Err("unknown".try_into().unwrap()),
        distances: array_init(|_| Err("unknown".try_into().unwrap())),
        location: None,
        battery: Err("unknown".try_into().unwrap()),
    };

    let sensors_sender = data.sensors.sender();
    let mut config = data.config.receiver().unwrap();
    println!("{}", config.get().await.angle_offset);

    let mut display_manager = DisplayManager::new(data);

    let mut utilization_monitor: UtilizationMonitor<50, R::Instant> =
        UtilizationMonitor::new(0.0, 0.0);
    utilization_monitor.start();

    let mut last_display_time = R::Instant::default();
    let mut dead_reckoning_loc = Point2::new(0.0f32, 0.0);
    let mut dead_reckoning_time = R::Instant::default();

    loop {
        // used to control the sleep between loop iterations
        let loop_start_time = R::Instant::default();
        // if any sensors changed, recompute the estimated position
        let mut something_changed = false;

        if let Some(r) = data.sig_angle.try_take() {
            sensors.angle = handle_err(r);
            if let Ok(ang) = &mut sensors.angle {
                *ang = *ang - config.get().await.angle_offset;
            }
            something_changed = true;
        }
        // https://docs.google.com/spreadsheets/d/1IAJgFo2dWXHccGbES6QZTsN5cONJF3FZhfny_6V-3S4/edit?gid=0#gid=0
        for (i, sensor) in data.sig_distances.iter().enumerate() {
            let index = config.get().await.dist_sensor_config[i];
            if let Some(r) = sensor.try_take() {
                sensors.distances[index] = handle_err(r).map(|x| {
                    x.map(|x| {
                        f32::max(
                            0.0,
                            if data.name.is_simulated() {
                                x
                            } else {
                                match index {
                                    0 => (0.0402 * x - 0.826) / INCHES_PER_GU,
                                    1 => (0.0417 * x - 1.47) / INCHES_PER_GU,
                                    2 => (0.0403 * x - 0.942) / INCHES_PER_GU,
                                    _ => (0.0403 * x - 0.819) / INCHES_PER_GU,
                                }
                            },
                        )
                    })
                });
                something_changed = true;
            }
        }
        if let Some(r) = data.sig_battery.try_take() {
            sensors.battery = handle_err(r);
            something_changed = true;
        }

        if last_display_time.elapsed()
            > Duration::from_millis(data.display_loop_interval.load(Ordering::Relaxed))
        {
            last_display_time = R::Instant::default();
            if let Some((button, pressed)) = peripherals.read_button_event().await {
                display_manager.button_event(button, pressed);
            }
            if let Some(joystick) = peripherals.read_joystick().await {
                display_manager.joystick = joystick;
            }
            peripherals.draw_display(|d| display_manager.draw(d)).await;
            peripherals.flip_screen().await;
        }

        // compute dead reckoning location
        if let Ok(angle) = sensors.angle {
            let motor_speeds = [0, 1, 2].map(|i| data.sig_motor_speeds[i].load(Ordering::Relaxed));
            let t = dead_reckoning_time.elapsed().as_secs_f32();
            if t != 0.0 {
                dead_reckoning_time = R::Instant::default();
                let (lin, _) = data
                    .robot_definition
                    .drive_system
                    .get_actual_vel_omni(motor_speeds);
                // data.set_extra_f32_indicator(2, lin.x);
                // data.set_extra_f32_indicator(3, lin.y);
                if !lin.x.is_nan() && !lin.y.is_nan() {
                    // transform linear velocity by the current angle
                    let vel = Vector2::new(
                        lin.x * angle.cos() - lin.y * angle.sin(),
                        lin.x * angle.sin() + lin.y * angle.cos(),
                    );
                    dead_reckoning_loc += vel * t;
                    // data.set_extra_f32_indicator(0, dead_reckoning_loc.x);
                    // data.set_extra_f32_indicator(1, dead_reckoning_loc.y);
                    // data.set_extra_f32_indicator(2, t);
                    // data.set_extra_f32_indicator(3, angle);
                }
            }
        }

        if something_changed {
            sensors.location = match config.get().await.localization_algorithm {
                LocalizationAlgorithmSource::RegionLocalization => region_localization::estimate_location_2(
                    config.get().await.grid,
                    config.get().await.cv_location,
                    &sensors.distances,
                    &data.robot_definition,
                    config.get().await.follow_target_path,
                ),
                LocalizationAlgorithmSource::CVAdjust => localization::estimate_location(
                    config.get().await.grid,
                    config.get().await.cv_location,
                    &sensors.distances,
                    &data.robot_definition,
                    config.get().await.cv_error,
                ),
                // TODO: not implemented yet
                _ => None,
            };
            sensors_sender.send(sensors.clone());
        }

        data.utilization[Task::Peripherals as usize]
            .store(utilization_monitor.utilization(), Ordering::Relaxed);

        utilization_monitor.stop();

        // The peripherals loop tends to use a significant percentage of its loop time doing I/O
        // Peripherals should always sleep for at least a little bit in order to give other tasks
        // a chance to run
        let min_wait_time = Duration::from_millis(5);
        // Ideally, peripherals runs at a consistent rate
        let ideal_loop_interval = Duration::from_millis(15);
        let this_loop_time = loop_start_time.elapsed();
        if this_loop_time > ideal_loop_interval {
            // This is bad; the peripherals loop took longer to run than its ideal interval
            // This will manifest in a drop in utilization_monitor's hz() result
            R::Instant::sleep(min_wait_time).await;
        } else {
            // Make sure to sleep for at least min_wait_time
            R::Instant::sleep(Duration::max(
                ideal_loop_interval.saturating_sub(this_loop_time),
                min_wait_time,
            ))
            .await;
        }
        // After sleeping, activity continues at the start of the loop
        utilization_monitor.start();
    }
}

/// Converts Results from sensors into heapless::String Results to be sent to the GUI
fn handle_err<T, E: Debug>(r: Result<T, E>) -> Result<T, heapless::String<MAX_SENSOR_ERR_LEN>> {
    let mut fmt_buf = [0; 100];
    match r {
        Ok(x) => Ok(x),
        Err(e) => {
            let s = format_no_std::show(&mut fmt_buf, format_args!("{:?}", e)).unwrap_or("?");
            Err(
                heapless::String::try_from(&s[..usize::min(MAX_SENSOR_ERR_LEN, s.len())])
                    .unwrap_or(heapless::String::new()),
            )
        }
    }
}
