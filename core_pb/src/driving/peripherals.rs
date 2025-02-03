use crate::driving::data::SharedRobotData;
use crate::driving::RobotBehavior;
use crate::messages::{RobotButton, SensorData, Task, MAX_SENSOR_ERR_LEN};
use crate::region_localization::estimate_location_2;
use crate::robot_display::DisplayManager;
use crate::util::utilization::UtilizationMonitor;
use crate::util::CrossPlatformInstant;
use array_init::array_init;
use core::fmt::Debug;
use core::sync::atomic::Ordering;
use core::time::Duration;
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::DrawTarget;

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

    let mut display_manager = DisplayManager::new(data);

    let mut utilization_monitor: UtilizationMonitor<50, R::Instant> =
        UtilizationMonitor::new(0.0, 0.0);
    utilization_monitor.start();

    let mut last_display_time = R::Instant::default();

    loop {
        // used to control the sleep between loop iterations
        let loop_start_time = R::Instant::default();
        // if any sensors changed, recompute the estimated position
        let mut something_changed = false;

        if let Some(r) = data.sig_angle.try_take() {
            sensors.angle = handle_err(r);
            something_changed = true;
        }
        for (i, sensor) in sensors.distances.iter_mut().enumerate() {
            if let Some(r) = data.sig_distances[i].try_take() {
                *sensor = handle_err(r);
                something_changed = true;
            }
        }
        if let Some(r) = data.sig_battery.try_take() {
            sensors.battery = handle_err(r);
            something_changed = true;
        }

        if last_display_time.elapsed() > Duration::from_millis(120) {
            last_display_time = R::Instant::default();
            while let Some((button, pressed)) = peripherals.read_button_event().await {
                display_manager.button_event(button, pressed);
            }
            if let Some(joystick) = peripherals.read_joystick().await {
                display_manager.joystick = joystick;
            }
            peripherals.draw_display(|d| display_manager.draw(d)).await;
            peripherals.flip_screen().await;
        }

        if something_changed {
            sensors.location = estimate_location_2(
                config.get().await.grid,
                config.get().await.cv_location,
                &sensors.distances,
                &data.robot_definition,
            );
            sensors_sender.send(sensors.clone());
            data.utilization[Task::Peripherals as usize]
                .store(utilization_monitor.utilization(), Ordering::Relaxed);
        }

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
                this_loop_time - ideal_loop_interval,
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
