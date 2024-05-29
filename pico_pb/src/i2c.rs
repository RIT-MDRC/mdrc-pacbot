use embedded_hal_async::i2c::I2c;

pub async fn write_u8<T: I2c>(
    address: u8,
    i2c: &mut T,
    location: u16,
    byte: u8,
) -> Result<(), T::Error> {
    i2c.write(
        address,
        &[
            ((location >> 8) & 0xFF) as u8,
            (location & 0xFF) as u8,
            byte,
        ],
    )
    .await
}

pub async fn write_u16<T: I2c>(
    address: u8,
    i2c: &mut T,
    location: u16,
    word: u16,
) -> Result<(), T::Error> {
    i2c.write(
        address,
        &[
            ((location >> 8) & 0xFF) as u8,
            (location & 0xFF) as u8,
            (word >> 8) as u8,
            (word & 0xFF) as u8,
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
