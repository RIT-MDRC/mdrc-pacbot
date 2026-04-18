use crate::EmbassyInstant;
use crate::SharedPicoRobotData;
use core::sync::atomic::Ordering;
use core_pb::util::average_rate::AverageRate;
use embassy_futures::select::{select, Either};
use embassy_rp::peripherals::PIO1;
use embassy_rp::pio::{Common, Instance, PioPin, StateMachine};
use embassy_rp::pio_programs::rotary_encoder::PioEncoderProgram;
use embassy_rp::pio_programs::rotary_encoder::{Direction, PioEncoder as EmbassyPioEncoder};
use embassy_time::{Instant, Timer};

#[embassy_executor::task]
pub async fn run_encoder_0(
    shared_data: &'static SharedPicoRobotData,
    mut encoder: WrappedPioEncoder<PIO1, 0>,
) {
    let mut last_tick = Instant::now();
    loop {
        match select(encoder.read(), Timer::after_millis(20)).await {
            Either::First(_) => {
                let velocity = -encoder.average_rate() / 12.0 / 2.0;
                shared_data.sig_motor_speeds[0].store(velocity, Ordering::Relaxed);
                last_tick = Instant::now();
            }
            Either::Second(_) => {
                if last_tick.elapsed().as_millis() > 80 {
                    shared_data.sig_motor_speeds[0].store(0.0, Ordering::Relaxed);
                } else {
                    let velocity = -encoder.average_rate() / 12.0 / 2.0;
                    shared_data.sig_motor_speeds[0].store(velocity, Ordering::Relaxed);
                }
            }
        }
    }
}

#[embassy_executor::task]
pub async fn run_encoder_1(
    shared_data: &'static SharedPicoRobotData,
    mut encoder: WrappedPioEncoder<PIO1, 1>,
) {
    let mut last_tick = Instant::now();
    loop {
        match select(encoder.read(), Timer::after_millis(20)).await {
            Either::First(_) => {
                let velocity = -encoder.average_rate() / 12.0 / 2.0;
                shared_data.sig_motor_speeds[1].store(velocity, Ordering::Relaxed);
                last_tick = Instant::now();
            }
            Either::Second(_) => {
                if last_tick.elapsed().as_millis() > 80 {
                    shared_data.sig_motor_speeds[1].store(0.0, Ordering::Relaxed);
                } else {
                    let velocity = -encoder.average_rate() / 12.0 / 2.0;
                    shared_data.sig_motor_speeds[1].store(velocity, Ordering::Relaxed);
                }
            }
        }
    }
}

#[embassy_executor::task]
pub async fn run_encoder_2(
    shared_data: &'static SharedPicoRobotData,
    mut encoder: WrappedPioEncoder<PIO1, 2>,
) {
    let mut last_tick = Instant::now();
    loop {
        match select(encoder.read(), Timer::after_millis(20)).await {
            Either::First(_) => {
                let velocity = -encoder.average_rate() / 12.0 / 2.0;
                shared_data.sig_motor_speeds[2].store(velocity, Ordering::Relaxed);
                last_tick = Instant::now();
            }
            Either::Second(_) => {
                if last_tick.elapsed().as_millis() > 80 {
                    shared_data.sig_motor_speeds[2].store(0.0, Ordering::Relaxed);
                } else {
                    let velocity = -encoder.average_rate() / 12.0 / 2.0;
                    shared_data.sig_motor_speeds[2].store(velocity, Ordering::Relaxed);
                }
            }
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
        d
    }

    pub fn average_rate(&self) -> f32 {
        self.ar.signed_ticks_per_second()
    }

    pub fn ticks(&self) -> i64 {
        self.ticks
    }
}
