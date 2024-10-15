use crate::driving::{RobotInterTaskMessage, RobotTask, Task};
use crate::grid::standard_grid::StandardGrid;
use crate::localization::estimate_location;
use crate::messages::{RobotButton, SensorData};
use crate::names::RobotName;
use crate::robot_definition::RobotDefinition;
use crate::robot_display::DisplayManager;
use crate::util::CrossPlatformInstant;
use core::fmt::Debug;
use core::time::Duration;
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::DrawTarget;
use nalgebra::Point2;

/// Functionality that robots with peripherals must support
pub trait RobotPeripheralsBehavior: RobotTask {
    type Display: DrawTarget<Color = BinaryColor>;
    type Instant: CrossPlatformInstant + Default;
    type Error: Debug;

    async fn draw_display<F>(&mut self, draw: F) -> Result<(), Self::Error>
    where
        F: FnOnce(&mut Self::Display) -> Result<(), <Self::Display as DrawTarget>::Error>;

    async fn flip_screen(&mut self) -> Result<(), Self::Error>;

    async fn absolute_rotation(&mut self) -> Result<f32, Self::Error>;

    async fn distance_sensor(&mut self, index: usize) -> Result<Option<f32>, Self::Error>;

    async fn battery_level(&mut self) -> Result<f32, Self::Error>;

    async fn read_button_event(&mut self) -> Option<(RobotButton, bool)>;

    async fn read_joystick(&mut self) -> Option<(f32, f32)>;
}

/// The "main" method for the peripherals task
pub async fn peripherals_task<T: RobotPeripheralsBehavior>(
    mut peripherals: T,
    name: RobotName,
) -> Result<(), T::Error> {
    let mut grid = StandardGrid::default();
    let mut cv_location = Some(Point2::new(
        pacbot_rs::variables::PACMAN_SPAWN_LOC.get_coords().0,
        pacbot_rs::variables::PACMAN_SPAWN_LOC.get_coords().1,
    ));

    let robot = RobotDefinition::new(name);

    let mut display_manager: DisplayManager<T::Instant> = DisplayManager::new(name);
    peripherals.draw_display(|d| display_manager.draw(d))?;
    peripherals.flip_screen().await;

    loop {
        while let Some((button, pressed)) = peripherals.read_button_event().await {
            display_manager.button_event(button, pressed);
        }
        if let Some(joystick) = peripherals.read_joystick().await {
            display_manager.joystick = joystick;
        }
        peripherals.draw_display(|d| display_manager.draw(d))?;
        peripherals.flip_screen().await;

        let angle = peripherals.absolute_rotation().await.map_err(|_| ());
        let mut distances = [Err(()); 4];
        for (i, sensor) in distances.iter_mut().enumerate() {
            *sensor = peripherals.distance_sensor(i).await.map_err(|_| ());
        }
        let location = estimate_location(grid, cv_location, &distances, &robot);
        display_manager.imu_angle = angle;
        display_manager.distances = distances;
        let sensors = SensorData {
            angle,
            distances,
            location,
            battery: peripherals.battery_level().await.map_err(|_| ()),
        };
        peripherals.send_or_drop(RobotInterTaskMessage::Sensors(sensors.clone()), Task::Wifi);
        peripherals.send_or_drop(RobotInterTaskMessage::Sensors(sensors), Task::Motors);
        match peripherals
            .receive_message_timeout(Duration::from_millis(10))
            .await
        {
            Some(RobotInterTaskMessage::Grid(new_grid)) => grid = new_grid,
            Some(RobotInterTaskMessage::FrequentServerToRobot(data)) => {
                cv_location = data.cv_location
            }
            Some(RobotInterTaskMessage::NetworkStatus(status, ip)) => {
                display_manager.network_status = status;
                display_manager.ip = ip;
            }
            _ => {}
        }
    }
}
