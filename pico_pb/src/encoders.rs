use crate::SharedPicoRobotData;
use core_pb::driving::EmbassyInstant;
use core_pb::util::average_rate::AverageRate;
use embassy_futures::select::{select4, Either4};
use embassy_rp::gpio::Pull;
use embassy_rp::peripherals::PIO1;
use embassy_rp::pio;
use embassy_time::{Instant, Timer};
use fixed::traits::ToFixed;
use pio::{Common, Config, FifoJoin, Instance, PioPin, ShiftDirection, StateMachine};

#[embassy_executor::task]
pub async fn run_encoders(
    shared_data: &'static SharedPicoRobotData,
    mut encoders: (
        PioEncoder<'static, PIO1, 0>,
        PioEncoder<'static, PIO1, 1>,
        PioEncoder<'static, PIO1, 2>,
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
            Either4::First(_) => Some((2, encoders.0.ticks(), -encoders.0.average_rate())),
            Either4::Second(_) => Some((1, encoders.1.ticks(), -encoders.1.average_rate())),
            Either4::Third(_) => Some((0, encoders.2.ticks(), -encoders.2.average_rate())),
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
