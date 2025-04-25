use crate::SharedPicoRobotData;
use core_pb::constants::PWM_SOFT_CAP;
use core_pb::driving::motors::RobotMotorsBehavior;
use defmt::Format;
use embassy_rp::peripherals::*;
use embassy_rp::pwm;
use embassy_rp::pwm::Pwm;
use fixed::types::extra::U4;
use fixed::FixedU16;

pub struct Motors<const WHEELS: usize> {
    pwm_pairs: [Pwm<'static>; WHEELS],
    pwm_configs: [pwm::Config; WHEELS],
}

impl Motors<3> {
    pub fn new(
        shared_data: &'static SharedPicoRobotData,
        pwm_pins: (PIN_2, PIN_3, PIN_6, PIN_7, PIN_10, PIN_11),
        pwm: (PWM_SLICE1, PWM_SLICE3, PWM_SLICE5),
    ) -> Self {
        let mut pwm_config = pwm::Config::default();
        pwm_config.top = shared_data.robot_definition.pwm_top;
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
        }
    }
}

#[derive(Debug, Format)]
pub enum MotorError {}

impl RobotMotorsBehavior for Motors<3> {
    async fn set_pwm(&mut self, pin: usize, to: u16) {
        let to = u16::min(to, PWM_SOFT_CAP);
        if pin % 2 == 0 {
            self.pwm_configs[pin / 2].compare_a = to;
        } else {
            self.pwm_configs[pin / 2].compare_b = to;
        }
        self.pwm_pairs[pin / 2].set_config(&self.pwm_configs[pin / 2]);
    }
}
