use embedded_hal_async::i2c::I2c;

pub mod bno08x;
pub mod ltc2943;
pub mod seesaw_gamepad_qt;
pub mod ssd1306;
pub mod vl53l1x;
pub mod vl53l4cd;
pub mod vl6180x;

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
