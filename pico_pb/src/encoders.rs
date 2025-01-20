use crate::EmbassyInstant;
use crate::SharedPicoRobotData;
use core_pb::util::average_rate::AverageRate;
use embassy_futures::select::{select4, Either4};
use embassy_rp::peripherals::PIO1;
use embassy_rp::pio::{Common, Instance, PioPin, StateMachine};
use embassy_rp::pio_programs::rotary_encoder::PioEncoderProgram;
use embassy_rp::pio_programs::rotary_encoder::{Direction, PioEncoder as EmbassyPioEncoder};
use embassy_time::{Instant, Timer};

#[embassy_executor::task]
pub async fn run_encoders(
    shared_data: &'static SharedPicoRobotData,
    mut encoders: (
        WrappedPioEncoder<PIO1, 0>,
        WrappedPioEncoder<PIO1, 1>,
        WrappedPioEncoder<PIO1, 2>,
    ),
) {
    let mut ticks = [0; 3];
    let mut velocities = [0.0; 3];
    let mut instants = [Instant::now(), Instant::now(), Instant::now()];

    let mut last_tick = Instant::now();
    let mut angle = 0.0;

    loop {
        if let Some((i, tick, velocity)) = match select4(
            encoders.0.read(),
            encoders.1.read(),
            encoders.2.read(),
            Timer::after_millis(10),
        )
        .await
        {
            Either4::First(_) => Some((0, encoders.0.ticks(), -encoders.0.average_rate())),
            Either4::Second(_) => Some((1, encoders.1.ticks(), -encoders.1.average_rate())),
            Either4::Third(_) => Some((2, encoders.2.ticks(), -encoders.2.average_rate())),
            _ => None,
        } {
            ticks[i] = tick;
            velocities[i] = velocity / 12.0 / 2.0;
            instants[i] = Instant::now();
        }

        for i in 0..3 {
            if instants[i].elapsed().as_millis() > 80 {
                velocities[i] = 0.0;
            }
        }
        shared_data.sig_motor_speeds.signal(velocities);

        let elapsed = last_tick.elapsed();
        if elapsed.as_micros() > 100 {
            last_tick = Instant::now();
            let rotational_velocity = shared_data
                .robot_definition
                .drive_system
                .get_actual_rotational_vel_omni(velocities);
            let s = elapsed.as_micros() as f32 / 1_000_000.0;
            angle += rotational_velocity * s;
            shared_data.sig_angle.signal(Ok(angle));
        }
    }
}

pub struct WrappedPioEncoder<T: Instance + 'static, const SM: usize> {
    pio_encoder: EmbassyPioEncoder<'static, T, SM>,
    ar: AverageRate<3, EmbassyInstant>,
    ticks: i64,
}

impl<T: Instance, const SM: usize> WrappedPioEncoder<T, SM> {
    pub fn new(
        common: &mut Common<'static, T>,
        sm: StateMachine<'static, T, SM>,
        pin_a: impl PioPin,
        pin_b: impl PioPin,
    ) -> Self {
        let prg = PioEncoderProgram::new(common);
        let pio_encoder = EmbassyPioEncoder::new(common, sm, pin_a, pin_b, &prg);

        Self {
            pio_encoder,
            ar: AverageRate::new(),
            ticks: 0,
        }
    }

    pub async fn read(&mut self) -> Direction {
        loop {
            let d = self.pio_encoder.read().await;
            match d {
                Direction::CounterClockwise => {
                    self.ar.tick(false);
                    self.ticks -= 1;
                }
                Direction::Clockwise => {
                    self.ar.tick(true);
                    self.ticks += 1;
                }
            };
            return d;
        }
    }

    pub fn average_rate(&self) -> f32 {
        self.ar.signed_ticks_per_second()
    }

    pub fn ticks(&self) -> i64 {
        self.ticks
    }
}
