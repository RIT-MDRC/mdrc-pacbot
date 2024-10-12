use crate::driving::{RobotInterTaskMessage, RobotTask, Task};
use crate::grid::standard_grid::StandardGrid;
use crate::localization::estimate_location;
use crate::messages::SensorData;
use crate::names::RobotName;
use crate::robot_definition::RobotDefinition;
use crate::util::CrossPlatformInstant;
use core::fmt::Debug;
use core::time::Duration;
use embedded_graphics::mono_font::ascii::FONT_6X10;
use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::{DrawTarget, Point, Primitive, Size};
use embedded_graphics::primitives::{PrimitiveStyleBuilder, Rectangle};
use embedded_graphics::text::Text;
use embedded_graphics::{Drawable, Pixel};
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

    // testing screen
    peripherals
        .draw_display(|d| {
            Pixel(Point::new(0, 0), BinaryColor::On).draw(d)?;
            Pixel(Point::new(127, 0), BinaryColor::On).draw(d)?;
            Pixel(Point::new(127, 63), BinaryColor::On).draw(d)?;
            Pixel(Point::new(0, 63), BinaryColor::On).draw(d)?;
            Text::new(
                name.get_str(),
                Point::new(2, 8),
                MonoTextStyle::new(&FONT_6X10, BinaryColor::On),
            )
            .draw(d)?;
            Ok(())
        })
        .await?;
    let _ = peripherals.flip_screen().await;

    let mut last_display_change = T::Instant::default();
    let mut last_display_state = false;

    loop {
        if last_display_change.elapsed() > Duration::from_millis(500) {
            last_display_change = T::Instant::default();
            last_display_state = !last_display_state;
            let color = if last_display_state {
                BinaryColor::On
            } else {
                BinaryColor::Off
            };
            let rectangle_style = PrimitiveStyleBuilder::new()
                .fill_color(color)
                .stroke_color(color)
                .stroke_width(1)
                .build();
            peripherals
                .draw_display(|d| {
                    Rectangle::new(Point::new(20, 20), Size::new(2, 2))
                        .into_styled(rectangle_style)
                        .draw(d)?;
                    Ok(())
                })
                .await?;
            let _ = peripherals.flip_screen().await;
        }

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
