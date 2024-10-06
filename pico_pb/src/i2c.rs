use crate::{receive_timeout, send_blocking2, send_or_drop2, EmbassyInstant, I2cBus, Irqs};
use core::time::Duration;
use core_pb::driving::peripherals::RobotPeripheralsBehavior;
use core_pb::driving::{RobotInterTaskMessage, RobotTask, Task};
use defmt::{info, Format};
use embassy_embedded_hal::shared_bus::asynch::i2c::I2cDevice;
use embassy_rp::gpio::{AnyPin, Level, Output};
use embassy_rp::i2c::{Async, SclPin, SdaPin};
use embassy_rp::peripherals::I2C0;
use embassy_rp::{i2c, Peripheral};
use embassy_sync::blocking_mutex::raw::{NoopRawMutex, ThreadModeRawMutex};
use embassy_sync::channel::Channel;
use embassy_time::Timer;
use embedded_hal_1::delay::DelayNs;
use embedded_hal_async::i2c::I2c;
use futures::SinkExt;
use ssd1306::mode::{BufferedGraphicsMode, BufferedGraphicsModeAsync};
use ssd1306::prelude::{DisplayRotation, I2CInterface};
use ssd1306::size::{DisplaySize128x64, DisplaySizeAsync};
use ssd1306::{I2CDisplayInterface, Ssd1306Async};
use vl53l4cd::Vl53l4cd;

/// numbr of distance sensors on the robot
pub const NUM_DIST_SENSORS: usize = 8;
//// what I2C addresses to reassign each distance sensor to
pub const DIST_SENSOR_ADDRESSES: [u8; NUM_DIST_SENSORS] =
    [0x29, 0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37];

pub static PERIPHERALS_CHANNEL: Channel<ThreadModeRawMutex, RobotInterTaskMessage, 64> =
    Channel::new();

pub struct RobotPeripherals {
    display: Ssd1306Async<
        I2CInterface<I2cDevice<'static, NoopRawMutex, embassy_rp::i2c::I2c<'static, I2C0, Async>>>,
        DisplaySize128x64,
        BufferedGraphicsModeAsync<DisplaySize128x64>,
    >,
}

impl RobotPeripherals {
    pub fn new(bus: &'static I2cBus) -> Self {
        let disp_i2c = I2cDevice::new(bus);

        let interface = I2CInterface::new(disp_i2c, 0x01, 0);
        let display = Ssd1306Async::new(interface, DisplaySize128x64, DisplayRotation::Rotate0)
            .into_buffered_graphics_mode();

        Self { display }
    }
}

#[derive(Debug, Format)]
pub enum I2cError {}

impl RobotTask for RobotPeripherals {
    fn send_or_drop(&mut self, message: RobotInterTaskMessage, to: Task) -> bool {
        send_or_drop2(message, to)
    }

    async fn send_blocking(&mut self, message: RobotInterTaskMessage, to: Task) {
        send_blocking2(message, to).await
    }

    async fn receive_message(&mut self) -> RobotInterTaskMessage {
        PERIPHERALS_CHANNEL.receive().await
    }

    async fn receive_message_timeout(
        &mut self,
        timeout: Duration,
    ) -> Option<RobotInterTaskMessage> {
        receive_timeout(&PERIPHERALS_CHANNEL, timeout).await
    }
}

impl RobotPeripheralsBehavior for RobotPeripherals {
    type Display = Ssd1306Async<
        I2CInterface<I2cDevice<'static, NoopRawMutex, embassy_rp::i2c::I2c<'static, I2C0, Async>>>,
        DisplaySize128x64,
        BufferedGraphicsModeAsync<DisplaySize128x64>,
    >;
    type Instant = EmbassyInstant;
    type Error = ();

    fn draw_display<F>(&mut self, draw: F) -> Result<(), Self::Error>
    where
        F: FnOnce(&mut Self::Display) -> Result<(), display_interface::DisplayError>,
    {
        draw(&mut self.display).map_err(|_| ())
    }

    async fn flip_screen(&mut self) {
        let _ = self.display.flush().await;
    }

    async fn absolute_rotation(&mut self) -> Result<f32, Self::Error> {
        Err(())
    }

    async fn distance_sensor(&mut self, index: usize) -> Result<Option<f32>, Self::Error> {
        Err(())
    }

    async fn battery_level(&mut self) -> Result<f32, Self::Error> {
        Err(())
    }
}

#[embassy_executor::task]
pub async fn read_distance_sensors(
    bus: &'static I2cBus<'static>,
    xshut: [AnyPin; NUM_DIST_SENSORS],
) {
    let mut xshut = xshut.map(|p| Output::new(p, Level::Low));
    let mut dist_sensors = [0, 1, 2, 3, 4, 5, 6, 7].map(|_| None);

    // initialize all sensors
    for i in 0..NUM_DIST_SENSORS {
        xshut[i].set_high();
        // this will error out since there are no sensors on 0x00 and will eventually be replaced
        // with the correct one.
        // TODO: figure out correct type annotations for dist_sensors
        // let i2c_inst = bus.acquire_i2c();
        let i2c_inst = I2cDevice::new(bus);
        let sensor = Vl53l4cd::with_addr(i2c_inst, 0x00, embassy_time::Delay, vl53l4cd::wait::Poll);

        dist_sensors[i] = Some(sensor);
        xshut[i].set_low();
    }

    let addresses: [u8; NUM_DIST_SENSORS] = [0x29, 0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37];
    let mut old_dist = [0; NUM_DIST_SENSORS];

    loop {
        // fetch new values
        let mut changed = false;
        for i in 1..NUM_DIST_SENSORS {
            let mut sensor_error = false;
            if let Some(sensor) = &mut dist_sensors[i] {
                if let Ok(measurement) = sensor.measure().await {
                    // info!("range {} {}", range, sensor.continuous_mode_enabled(&mut i2c).await);
                    if measurement.distance != old_dist[i] {
                        old_dist[i] = measurement.distance;
                        changed = true;
                    }
                    if measurement.distance == 0 {
                        // info!("Range status error {:?}", measurement.status);
                    }
                } else {
                    sensor_error = true;
                }
            } else {
                // the sensor isn't connected - try to connect to it
                info!("Trying to connect to sensor {}", i);
                xshut[i].set_high();
                info!("Sensor {} set high", i);
                Timer::after_millis(300).await;

                // create new inst since the sensor library takes ownership
                let i2c_inst = I2cDevice::new(bus);
                // if let Err(e) = Vl53l4cd::new(&mut i2c, 0x29).await {
                let mut sensor =
                    Vl53l4cd::with_addr(i2c_inst, 0x29, embassy_time::Delay, vl53l4cd::wait::Poll);

                match sensor.init().await {
                    Ok(_) => {}
                    Err(e) => {
                        // info!("{:?}", e);
                    }
                }
            }
        }
    }
    //             if let Ok(mut sensor) = VL6180X::new(&mut i2c, 0x29).await {
    //                 // sensor.set_address(&mut i2c, addresses[i]).await;
    //                 if let Ok(_) = sensor.set_address(&mut i2c, addresses[i]).await {
    //                     Timer::after_millis(300).await;
    //                     if let Ok(sensor) = VL6180X::new(&mut i2c, addresses[i]).await {
    //                         Timer::after_millis(300).await;
    //                         if let Ok(_) = sensor.start_range_continuous(&mut i2c, 1).await {
    //                             sensors[i] = Some(sensor)
    //                         } else {
    //                             info!("start cont err");
    //                             sensor_error = true;
    //                         }
    //                     } else {
    //                         info!("remake device err");
    //                         sensor_error = true;
    //                     }
    //                 } else {
    //                     info!("set addr err");
    //                     if let Err(e) = sensor.set_address(&mut i2c, addresses[i]).await {
    //                         info!("{:?}", e);
    //                     }
    //                     sensor_error = true;
    //                 }
    //             } else {
    //                 info!("make sensor err");
    //                 // something went wrong - in case this sensor wakes up, force it to shut down for now
    //                 sensor_error = true;
    //             }
    //         }
    //         if sensor_error {
    //             xshut[i].set_low();
    //             old_dist[i] = 0;
    //             sensors[i] = None;
    //             changed = true;
    //         }
    //     }
    //     if changed {
    //         let _ = sender.try_send(old_dist);
    //     }
    //     Timer::after_millis(1).await;
    // }
}

pub async fn write_u8<T: I2c>(
    address: u8,
    i2c: &mut T,
    location: u16,
    data: u8,
) -> Result<(), T::Error> {
    i2c.write(
        address,
        &[
            ((location >> 8) & 0xFF) as u8,
            (location & 0xFF) as u8,
            data,
        ],
    )
    .await
}

pub async fn write_u16<T: I2c>(
    address: u8,
    i2c: &mut T,
    location: u16,
    data: u16,
) -> Result<(), T::Error> {
    i2c.write(
        address,
        &[
            ((location >> 8) & 0xFF) as u8,
            (location & 0xFF) as u8,
            (data >> 8) as u8,
            (data & 0xFF) as u8,
        ],
    )
    .await
}

pub async fn write_u32<T: I2c>(
    address: u8,
    i2c: &mut T,
    location: u16,
    data: u32,
) -> Result<(), T::Error> {
    i2c.write(
        address,
        &[
            ((location >> 8) & 0xFF) as u8,
            (location & 0xFF) as u8,
            ((data >> 24) & 0xFF) as u8,
            ((data >> 16) & 0xFF) as u8,
            ((data >> 8) & 0xFF) as u8,
            (data & 0xFF) as u8,
        ],
    )
    .await
}

pub async fn read_u8<T: I2c>(address: u8, i2c: &mut T, location: u16) -> Result<u8, T::Error> {
    let mut buf = [0];
    i2c.write_read(
        address,
        &[((location >> 8) & 0xFF) as u8, (location & 0xFF) as u8],
        &mut buf,
    )
    .await?;
    Ok(buf[0])
}

pub async fn read_u16<T: I2c>(address: u8, i2c: &mut T, location: u16) -> Result<u16, T::Error> {
    let mut buf = [0; 2];
    i2c.write_read(
        address,
        &[((location >> 8) & 0xFF) as u8, (location & 0xFF) as u8],
        &mut buf,
    )
    .await?;
    Ok(u16::from_be_bytes([buf[0], buf[1]]))
}

pub async fn read_u32<T: I2c>(address: u8, i2c: &mut T, location: u16) -> Result<u32, T::Error> {
    let mut buf = [0; 4];
    i2c.write_read(
        address,
        &[((location >> 8) & 0xFF) as u8, (location & 0xFF) as u8],
        &mut buf,
    )
    .await?;
    Ok(u32::from_be_bytes([buf[0], buf[1], buf[2], buf[3]]))
}
