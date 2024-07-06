use crate::driving::RobotTask;
use core::fmt::Debug;
use embedded_graphics::prelude::DrawTarget;

pub trait RobotPeripheralsBehavior: RobotTask {
    type Display: DrawTarget;
    type Error: Debug;

    fn draw_display<F>(&mut self, draw: F)
    where
        F: FnOnce(&mut Self::Display);

    async fn flip_screen(&mut self);
}

pub async fn peripherals_task<T: RobotPeripheralsBehavior>(
    _peripherals: T,
) -> Result<(), T::Error> {
    Ok(())
}
