use crate::driving::{RobotInterTaskMessage, RobotTaskMessenger, Task};
use crate::grid::standard_grid::StandardGrid;
use crate::messages::{RobotButton, SensorData, MAX_SENSOR_ERR_LEN};
use crate::names::RobotName;
use crate::region_localization::estimate_location_2;
use crate::robot_definition::RobotDefinition;
use crate::robot_display::DisplayManager;
use crate::util::utilization::UtilizationMonitor;
use crate::util::CrossPlatformInstant;
use core::fmt::Debug;
use core::time::Duration;
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::DrawTarget;
use nalgebra::Point2;

/// Functionality that robots with peripherals must support
pub trait RobotPeripheralsBehavior {
    type Display: DrawTarget<Color = BinaryColor>;
    type Instant: CrossPlatformInstant + Default;
    type Error: Debug;

    async fn draw_display<F>(&mut self, draw: F)
    where
        F: FnOnce(&mut Self::Display) -> Result<(), <Self::Display as DrawTarget>::Error>;

    async fn flip_screen(&mut self);

    async fn absolute_rotation(&mut self) -> Result<f32, Self::Error>;

    async fn distance_sensor(&mut self, index: usize) -> Result<Option<f32>, Self::Error>;

    async fn battery_level(&mut self) -> Result<f32, Self::Error>;

    async fn read_button_event(&mut self) -> Option<(RobotButton, bool)>;

    async fn read_joystick(&mut self) -> Option<(f32, f32)>;
}

/// The "main" method for the peripherals task
pub async fn peripherals_task<T: RobotPeripheralsBehavior, M: RobotTaskMessenger>(
    name: RobotName,
    mut peripherals: T,
    mut msgs: M,
) -> Result<(), T::Error> {
    let mut grid = StandardGrid::default();
    let mut cv_location = Some(Point2::new(
        pacbot_rs::variables::PACMAN_SPAWN_LOC.get_coords().0,
        pacbot_rs::variables::PACMAN_SPAWN_LOC.get_coords().1,
    ));

    let robot = RobotDefinition::new(name);
    let mut last_success_angle = 0.0;
    let mut angle_offset = 0.0;

    let mut display_manager: DisplayManager<T::Instant> = DisplayManager::new(name);
    peripherals.draw_display(|d| display_manager.draw(d)).await;
    peripherals.flip_screen().await;

    let mut utilization_monitor: UtilizationMonitor<50, T::Instant> =
        UtilizationMonitor::new(0.0, 0.0);
    utilization_monitor.start();

    let mut last_send_time = T::Instant::default();

    loop {
        if last_send_time.elapsed() > Duration::from_millis(30) {
            last_send_time = T::Instant::default();
            while let Some((button, pressed)) = peripherals.read_button_event().await {
                display_manager.button_event(button, pressed);
            }
            if let Some(joystick) = peripherals.read_joystick().await {
                display_manager.joystick = joystick;
            }
            peripherals.draw_display(|d| display_manager.draw(d)).await;
            peripherals.flip_screen().await;

            fn handle_err<T, E: Debug>(
                r: Result<T, E>,
            ) -> Result<T, heapless::String<MAX_SENSOR_ERR_LEN>> {
                let mut fmt_buf = [0; 100];
                match r {
                    Ok(x) => Ok(x),
                    Err(e) => {
                        let s = format_no_std::show(&mut fmt_buf, format_args!("{:?}", e))
                            .unwrap_or("?");
                        Err(heapless::String::try_from(
                            &s[..usize::min(MAX_SENSOR_ERR_LEN, s.len())],
                        )
                        .unwrap_or(heapless::String::new()))
                    }
                }
            }

            let angle = match handle_err(peripherals.absolute_rotation().await) {
                Ok(a) => {
                    last_success_angle = a;
                    let a = a - angle_offset;
                    Ok(a)
                }
                e => e,
            };

            let mut distances = [const { Err(heapless::String::new()) }; 4];
            for (i, sensor) in distances.iter_mut().enumerate() {
                *sensor = handle_err(peripherals.distance_sensor(i).await);
            }
            let location = estimate_location_2(grid, cv_location, &distances, &robot);
            display_manager.imu_angle = angle.clone();
            display_manager.distances = distances.clone();
            let sensors = SensorData {
                angle,
                distances,
                location,
                battery: peripherals.battery_level().await.map_err(|_| ()),
            };
            msgs.send_or_drop(RobotInterTaskMessage::Sensors(sensors.clone()), Task::Wifi);
            msgs.send_or_drop(RobotInterTaskMessage::Sensors(sensors), Task::Motors);
            msgs.send_or_drop(
                RobotInterTaskMessage::Utilization(
                    utilization_monitor.utilization(),
                    Task::Peripherals,
                ),
                Task::Wifi,
            );
        }

        utilization_monitor.stop();
        let event = msgs
            .receive_message_timeout(Duration::from_millis(10))
            .await;
        utilization_monitor.start();

        match event {
            Some(RobotInterTaskMessage::FrequentServerToRobot(data)) => {
                grid = data.grid;
                cv_location = data.cv_location
            }
            Some(RobotInterTaskMessage::NetworkStatus(status, ip)) => {
                display_manager.network_status = status;
                display_manager.ip = ip;
            }
            Some(RobotInterTaskMessage::ResetAngle) => {
                angle_offset = last_success_angle;
            }
            _ => {}
        }
    }
}
