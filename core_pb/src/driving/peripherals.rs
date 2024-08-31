use crate::driving::RobotTask;
use core::fmt::Debug;
use embedded_graphics::prelude::DrawTarget;

/// Functionality that robots with peripherals must support
pub trait RobotPeripheralsBehavior: RobotTask {
    type Display: DrawTarget;
    type Error: Debug;

    fn draw_display<F>(&mut self, draw: F)
    where
        F: FnOnce(&mut Self::Display);

    async fn flip_screen(&mut self);
}

/// The "main" method for the peripherals task
pub async fn peripherals_task<T: RobotPeripheralsBehavior>(
    mut peripherals: T,
) -> Result<(), T::Error> {
    loop {
        let _ = peripherals.receive_message().await;
    }
}
