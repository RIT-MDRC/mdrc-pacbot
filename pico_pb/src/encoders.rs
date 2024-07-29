use crate::EmbassyInstant;
use core_pb::util::average_rate::AverageRate;
use embassy_futures::select::{select3, Either3};
use embassy_rp::gpio::Pull;
use embassy_rp::peripherals::PIO1;
use embassy_rp::pio;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::signal::Signal;
use fixed::traits::ToFixed;
use pio::{Common, Config, FifoJoin, Instance, PioPin, ShiftDirection, StateMachine};

pub static ENCODER_VELOCITIES: Signal<CriticalSectionRawMutex, [f32; 3]> = Signal::new();
pub static FULL_ENCODER_INFO: Signal<CriticalSectionRawMutex, ([i64; 3], [f32; 3])> = Signal::new();

#[embassy_executor::task]
pub async fn run_encoders(
    mut encoders: (
        PioEncoder<'static, PIO1, 0>,
        PioEncoder<'static, PIO1, 1>,
        PioEncoder<'static, PIO1, 2>,
    ),
) {
    let mut ticks = [0; 3];
    let mut velocities = [0.0; 3];
    loop {
        let (i, tick, velocity) =
            match select3(encoders.0.read(), encoders.1.read(), encoders.2.read()).await {
                Either3::First(_) => (0, encoders.0.ticks(), encoders.0.average_rate()),
                Either3::Second(_) => (1, encoders.1.ticks(), encoders.1.average_rate()),
                Either3::Third(_) => (2, encoders.2.ticks(), encoders.2.average_rate()),
            };
        ticks[i] = tick;
        velocities[i] = -velocity / 12.0 / 2.0;
        ENCODER_VELOCITIES.signal(velocities);
        FULL_ENCODER_INFO.signal((ticks, velocities));
    }
}

pub struct PioEncoder<'d, T: Instance, const SM: usize> {
    sm: StateMachine<'d, T, SM>,
    ar: AverageRate<3, EmbassyInstant>,
    ticks: i64,
}

impl<'d, T: Instance, const SM: usize> PioEncoder<'d, T, SM> {
    pub fn new(
        pio: &mut Common<'d, T>,
        mut sm: StateMachine<'d, T, SM>,
        pin_a: impl PioPin,
        pin_b: impl PioPin,
    ) -> Self {
        let mut pin_a = pio.make_pio_pin(pin_a);
        let mut pin_b = pio.make_pio_pin(pin_b);
        pin_a.set_pull(Pull::Up);
        pin_b.set_pull(Pull::Up);
        sm.set_pin_dirs(pio::Direction::In, &[&pin_a, &pin_b]);

        let prg = pio_proc::pio_asm!("wait 1 pin 1", "wait 0 pin 1", "in pins, 2", "push",);

        let mut cfg = Config::default();
        cfg.set_in_pins(&[&pin_a, &pin_b]);
        cfg.fifo_join = FifoJoin::RxOnly;
        cfg.shift_in.direction = ShiftDirection::Left;
        cfg.clock_divider = 10_000.to_fixed();
        cfg.use_program(&pio.load_program(&prg.program), &[]);
        sm.set_config(&cfg);
        sm.set_enable(true);
        Self {
            sm,
            ar: AverageRate::new(),
            ticks: 0,
        }
    }

    pub async fn read(&mut self) -> Direction {
        loop {
            match self.sm.rx().wait_pull().await {
                0 => {
                    self.ar.tick(false);
                    self.ticks -= 1;
                    return Direction::CounterClockwise;
                }
                1 => {
                    self.ar.tick(true);
                    self.ticks += 1;
                    return Direction::Clockwise;
                }
                _ => {}
            }
        }
    }

    pub fn average_rate(&self) -> f32 {
        self.ar.signed_ticks_per_second()
    }

    pub fn ticks(&self) -> i64 {
        self.ticks
    }
}

pub enum Direction {
    Clockwise,
    CounterClockwise,
}
