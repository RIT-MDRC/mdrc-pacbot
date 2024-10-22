use crate::encoders::ENCODER_VELOCITIES;
use crate::EmbassyInstant;
use core_pb::driving::motors::RobotMotorsBehavior;
use core_pb::driving::RobotInterTaskMessage;
use core_pb::robot_definition::RobotDefinition;
use defmt::Format;
use embassy_rp::peripherals::*;
use embassy_rp::pwm;
use embassy_rp::pwm::Pwm;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::channel::Channel;
use embassy_time::Instant;
use fixed::types::extra::U4;
use fixed::FixedU16;

pub static MOTORS_CHANNEL: Channel<ThreadModeRawMutex, RobotInterTaskMessage, 64> = Channel::new();

pub struct Motors<const WHEELS: usize> {
    pwm_pairs: [Pwm<'static>; WHEELS],
    pwm_configs: [pwm::Config; WHEELS],
    motor_speeds: ([f32; WHEELS], Instant),
}

impl Motors<3> {
    pub fn new(
        robot: RobotDefinition<3>,
        pwm_pins: (PIN_2, PIN_3, PIN_6, PIN_7, PIN_10, PIN_11),
        pwm: (PWM_SLICE1, PWM_SLICE3, PWM_SLICE5),
    ) -> Self {
        let mut pwm_config = pwm::Config::default();
        pwm_config.top = robot.pwm_top;
        pwm_config.divider = FixedU16::<U4>::from_num(0.7);

        let pins = [
            Pwm::new_output_ab(pwm.0, pwm_pins.0, pwm_pins.1, pwm_config.clone()),
            Pwm::new_output_ab(pwm.1, pwm_pins.2, pwm_pins.3, pwm_config.clone()),
            Pwm::new_output_ab(pwm.2, pwm_pins.4, pwm_pins.5, pwm_config.clone()),
        ];

        let pwm_configs = [0; 3].map(|_| pwm_config.clone());

        Self {
            pwm_pairs: pins,
            pwm_configs,
            motor_speeds: ([0.0; 3], Instant::now()),
        }
    }
}

#[derive(Debug, Format)]
pub enum MotorError {}

impl RobotMotorsBehavior for Motors<3> {
    type Error = MotorError;
    type Instant = EmbassyInstant;

    async fn set_pwm(&mut self, pin: usize, to: u16) {
        if pin % 2 == 0 {
            self.pwm_configs[pin / 2].compare_a = to;
        } else {
            self.pwm_configs[pin / 2].compare_b = to;
        }
        self.pwm_pairs[pin / 2].set_config(&self.pwm_configs[pin / 2]);
    }

    async fn get_motor_speed(&mut self, motor: usize) -> f32 {
        if let Some(speeds) = ENCODER_VELOCITIES.try_take() {
            self.motor_speeds = speeds;
        }
        if self.motor_speeds.1.elapsed() > embassy_time::Duration::from_millis(80) {
            0.0
        } else {
            self.motor_speeds.0[motor]
        }
    }
}
