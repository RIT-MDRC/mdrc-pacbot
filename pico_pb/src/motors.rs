use crate::send;
use core_pb::driving::motors::RobotMotorsBehavior;
use core_pb::driving::{RobotInterTaskMessage, RobotTask, Task};
use defmt::Format;
use embassy_rp::peripherals::{
    PIN_14, PIN_15, PIN_6, PIN_7, PIN_8, PIN_9, PWM_SLICE3, PWM_SLICE4, PWM_SLICE7,
};
use embassy_rp::pwm;
use embassy_rp::pwm::Pwm;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::channel::Channel;

pub static MOTORS_CHANNEL: Channel<ThreadModeRawMutex, RobotInterTaskMessage, 64> = Channel::new();

pub struct Motors<const WHEELS: usize> {
    pwm_top: u16,
    motor_io: MotorIO<'static, WHEELS>,
}

impl Motors<3> {
    pub fn new(
        motor_pins: (PIN_6, PIN_7, PIN_8, PIN_9, PIN_14, PIN_15),
        pwm: (PWM_SLICE3, PWM_SLICE4, PWM_SLICE7),
    ) -> Self {
        let pwm_top: u16 = 0x8000;
        let mut pwm_config = pwm::Config::default();
        pwm_config.top = pwm_top;

        let motors = [
            Pwm::new_output_ab(pwm.0, motor_pins.0, motor_pins.1, pwm_config.clone()),
            Pwm::new_output_ab(pwm.1, motor_pins.2, motor_pins.3, pwm_config.clone()),
            Pwm::new_output_ab(pwm.2, motor_pins.4, motor_pins.5, pwm_config.clone()),
        ];

        let motor_io = MotorIO::new([(4, 5), (3, 2), (0, 1)], motors, pwm_config);

        Self { pwm_top, motor_io }
    }
}

#[derive(Debug, Format)]
pub enum MotorError {}

impl<const WHEELS: usize> RobotTask for Motors<WHEELS> {
    async fn send_message(&mut self, message: RobotInterTaskMessage, to: Task) -> Result<(), ()> {
        send(message, to).await.map_err(|_| ())
    }

    async fn receive_message(&mut self) -> RobotInterTaskMessage {
        MOTORS_CHANNEL.receive().await
    }
}

impl<const WHEELS: usize> RobotMotorsBehavior for Motors<WHEELS> {
    type Error = ();

    fn do_pid(&self) -> bool {
        true
    }

    async fn set_motor_speed(&mut self, index: usize, to: f32) {
        if to == 0.0 {
            self.motor_io
                .set_motor_speeds(index, self.pwm_top, self.pwm_top)
        } else if to > 0.0 {
            self.motor_io.set_motor_speeds(index, self.pwm_top, 0)
        } else {
            self.motor_io.set_motor_speeds(index, 0, self.pwm_top)
        }
    }
}

/// Holds all the configurable motors/encoders
struct MotorIO<'a, const WHEELS: usize> {
    config: [(usize, usize); WHEELS],
    motors: [Pwm<'a>; WHEELS],
    motor_configs: [pwm::Config; WHEELS],
}

#[allow(dead_code)]
impl<'a, const WHEELS: usize> MotorIO<'a, WHEELS> {
    /// Create a new MotorIO
    fn new(
        config: [(usize, usize); WHEELS],
        motors: [Pwm<'a>; WHEELS],
        motor_config: pwm::Config,
    ) -> Self {
        Self {
            config,
            motors,
            motor_configs: [0; WHEELS].map(|_| motor_config.clone()),
        }
    }

    /// Applies changes to motor_configs to the pins
    fn set_pwm_config(&mut self, id: usize) {
        self.motors[id].set_config(&self.motor_configs[id]);
    }

    /// Sets the "top" value for all PWMs
    pub fn set_pwm_top(&mut self, top: u16) {
        for i in 0..WHEELS {
            self.motor_configs[i].top = top;
            self.motors[i].set_config(&self.motor_configs[i]);
        }
    }

    /// Get the current output on the given pin
    pub fn get_pin_pwm(&self, id: usize) -> u16 {
        if id % 2 == 0 {
            self.motor_configs[id / 2].compare_a
        } else {
            self.motor_configs[id / 2].compare_b
        }
    }

    /// Set the output value for a given pin
    pub fn set_pin_pwm(&mut self, id: usize, compare: u16) {
        if id % 2 == 0 {
            self.motor_configs[id / 2].compare_a = compare;
        } else {
            self.motor_configs[id / 2].compare_b = compare;
        }
        self.set_pwm_config(id / 2)
    }

    /// Get the current PWM outputs for the given motor
    pub fn get_motor_speeds(&self, id: usize) -> (u16, u16) {
        let (a, b) = self.config[id];
        (self.get_pin_pwm(a), self.get_pin_pwm(b))
    }

    /// Set the PWM outputs for the given motor
    pub fn set_motor_speeds(&mut self, id: usize, compare_a: u16, compare_b: u16) {
        let (a, b) = self.config[id];
        self.set_pin_pwm(a, compare_a);
        self.set_pin_pwm(b, compare_b);
    }
}
