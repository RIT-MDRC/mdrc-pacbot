use crate::{receive_timeout, send};
use core::time::Duration;
use core_pb::driving::motors::RobotMotorsBehavior;
use core_pb::driving::{RobotInterTaskMessage, RobotTask, Task};
use core_pb::robot_definition::RobotDefinition;
use defmt::Format;
use embassy_rp::peripherals::{
    PIN_14, PIN_15, PIN_6, PIN_7, PIN_8, PIN_9, PWM_SLICE3, PWM_SLICE4, PWM_SLICE7,
};
use embassy_rp::pwm;
use embassy_rp::pwm::Pwm;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::channel::Channel;
use embassy_time::Instant;

pub static MOTORS_CHANNEL: Channel<ThreadModeRawMutex, RobotInterTaskMessage, 64> = Channel::new();

pub struct Motors<const WHEELS: usize> {
    pwm_pairs: [Pwm<'static>; WHEELS],
    pwm_configs: [pwm::Config; WHEELS],
}

impl Motors<3> {
    pub fn new(
        robot: RobotDefinition<3>,
        pwm_pins: (PIN_6, PIN_7, PIN_8, PIN_9, PIN_14, PIN_15),
        pwm: (PWM_SLICE3, PWM_SLICE4, PWM_SLICE7),
    ) -> Self {
        let mut pwm_config = pwm::Config::default();
        pwm_config.top = robot.pwm_top;

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

impl<const WHEELS: usize> RobotTask for Motors<WHEELS> {
    async fn send_message(&mut self, message: RobotInterTaskMessage, to: Task) -> Result<(), ()> {
        send(message, to).await.map_err(|_| ())
    }

    async fn receive_message(&mut self) -> RobotInterTaskMessage {
        MOTORS_CHANNEL.receive().await
    }

    async fn receive_message_timeout(
        &mut self,
        timeout: Duration,
    ) -> Option<RobotInterTaskMessage> {
        receive_timeout(&MOTORS_CHANNEL, timeout).await
    }
}

impl<const WHEELS: usize> RobotMotorsBehavior for Motors<WHEELS> {
    type Error = MotorError;

    type Instant = Instant;
    fn elapsed(&self, instant: &Self::Instant) -> Duration {
        instant.elapsed().into()
    }
    fn now(&self) -> Self::Instant {
        Instant::now()
    }

    fn do_pid(&self) -> bool {
        false
    }

    async fn set_pwm(&mut self, pin: usize, to: u16) {
        if pin % 2 == 0 {
            self.pwm_configs[pin / 2].compare_a = to;
        } else {
            self.pwm_configs[pin / 2].compare_b = to;
        }
        self.pwm_pairs[pin / 2].set_config(&self.pwm_configs[pin / 2]);
    }
}
