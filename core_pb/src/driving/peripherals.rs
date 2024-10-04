use crate::driving::{RobotInterTaskMessage, RobotTask, Task};
use crate::grid::standard_grid::StandardGrid;
use crate::localization::estimate_location;
use crate::messages::SensorData;
use crate::names::RobotName;
use crate::robot_definition::RobotDefinition;
use core::fmt::Debug;
use core::time::Duration;
use embedded_graphics::prelude::DrawTarget;
use nalgebra::Point2;

/// Functionality that robots with peripherals must support
pub trait RobotPeripheralsBehavior: RobotTask {
    type Display: DrawTarget;
    type Error: Debug;

    fn draw_display<F>(&mut self, draw: F)
    where
        F: FnOnce(&mut Self::Display);

    async fn flip_screen(&mut self);

    async fn absolute_rotation(&mut self) -> Result<f32, Self::Error>;

    async fn distance_sensor(&mut self, index: usize) -> Result<Option<f32>, Self::Error>;

    async fn battery_level(&mut self) -> Result<f32, Self::Error>;
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

    loop {
        let angle = peripherals.absolute_rotation().await.map_err(|_| ());
        let mut distances = [Err(()); 4];
        for (i, sensor) in distances.iter_mut().enumerate() {
            *sensor = peripherals.distance_sensor(i).await.map_err(|_| ());
        }
        let location = estimate_location(grid, cv_location, &distances, &robot);
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
            _ => {}
        }
    }
}
