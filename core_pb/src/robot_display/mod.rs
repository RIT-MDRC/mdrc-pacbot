mod menu;

use crate::constants::{ROBOT_DISPLAY_HEIGHT, ROBOT_DISPLAY_WIDTH};
use crate::driving::network::DEFAULT_NETWORK;
use crate::messages::{NetworkStatus, RobotButton};
use crate::names::RobotName;
use crate::robot_display::menu::Page;
use crate::util::CrossPlatformInstant;
use embedded_graphics::mono_font::ascii::FONT_5X7;
use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;
use embedded_graphics::text::Text;
use embedded_graphics::Drawable;
use nalgebra::max;

#[derive(Copy, Clone, Debug, PartialOrd, PartialEq)]
enum Tab {
    Main,
    TypingTest(Option<usize>),
}

impl Tab {
    fn previous_tab(&self) -> Option<Self> {
        match self {
            Tab::Main => Some(Tab::TypingTest(None)),
            Tab::TypingTest(None) => Some(Tab::Main),
            _ => None,
        }
    }

    fn next_tab(&self) -> Option<Self> {
        match self {
            Tab::Main => Some(Tab::TypingTest(None)),
            Tab::TypingTest(None) => Some(Tab::Main),
            _ => None,
        }
    }
}

pub struct DisplayManager<I: CrossPlatformInstant + Default> {
    name: RobotName,
    started_at: I,

    page: Page,
    submenu_index: usize,

    tab: Tab,

    typing_tests: [TextInput<32>; 3],

    pub network_status: NetworkStatus,
    pub ip: Option<[u8; 4]>,
    pub ssid: Option<([u8; 32], usize)>,
    pub distances: [Result<Option<f32>, ()>; 4],
    pub imu_angle: Result<f32, ()>,
    pub joystick: (f32, f32),
}

impl<I: CrossPlatformInstant + Default> DisplayManager<I> {
    pub fn new(name: RobotName) -> Self {
        let mut ssid = [0; 32];
        for (i, ch) in DEFAULT_NETWORK.as_bytes().iter().enumerate().take(32) {
            ssid[i] = *ch;
        }
        Self {
            name,
            started_at: I::default(),

            page: Page::Main,
            submenu_index: 0,

            tab: Tab::Main,

            typing_tests: [TextInput::new(); 3],

            network_status: NetworkStatus::NotConnected,
            ssid: Some((ssid, DEFAULT_NETWORK.len().min(32))),
            ip: None,
            distances: [Err(()); 4],
            imu_angle: Err(()),
            joystick: (0.0, 0.0),
        }
    }

    pub fn draw<D: DrawTarget<Color = BinaryColor>>(&mut self, d: &mut D) -> Result<(), D::Error> {
        d.clear(BinaryColor::Off)?;
        self.draw_content(self.page, d)?;
        match self.tab {
            Tab::Main => self.main_content(d)?,
            Tab::TypingTest(_) => self.typing_test(d)?,
        }
        Ok(())
    }

    pub fn button_event(&mut self, button: RobotButton, pressed: bool) {
        if button == RobotButton::EastA && pressed {
            if let Some(tab) = self.tab.next_tab() {
                self.tab = tab;
                return;
            }
        }
        if button == RobotButton::WestY && pressed {
            if let Some(tab) = self.tab.previous_tab() {
                self.tab = tab;
                return;
            }
        }
    }

    fn text<D: DrawTarget<Color = BinaryColor>>(
        &mut self,
        text: &str,
        at: Point,
        d: &mut D,
    ) -> Result<(), D::Error> {
        Text::new(text, at, MonoTextStyle::new(&FONT_5X7, BinaryColor::On)).draw(d)?;
        Ok(())
    }

    fn text_ticker<D: DrawTarget<Color = BinaryColor>>(
        &mut self,
        text: &str,
        at: Point,
        max_len_px: usize,
        d: &mut D,
    ) -> Result<(), D::Error> {
        let start_t = 1500;
        let px_transition_t = 40;
        let end_t = 1500;

        let num_px_transitions = (text.len() * 5).saturating_sub(max_len_px) as i32 + 1;
        let cycle_t = start_t + px_transition_t * num_px_transitions + end_t;

        let phase = (self.started_at.elapsed().as_millis() % cycle_t as u128) as i32;
        let first_char_loc_px = if phase < start_t {
            0
        } else if phase > start_t + px_transition_t * num_px_transitions {
            -num_px_transitions
        } else {
            -(phase - start_t) / px_transition_t
        };

        for i in 0..text.len() {
            // if the entire character fits within the ticker, draw the character
            let x_offset = first_char_loc_px + 5 * i as i32;
            if 0 <= x_offset && x_offset + 5 < max_len_px as i32 {
                Text::new(
                    &text[i..(i + 1)],
                    Point::new(at.x + x_offset, at.y),
                    MonoTextStyle::new(&FONT_5X7, BinaryColor::On),
                )
                .draw(d)?;
            } else if x_offset + 5 > max_len_px as i32 {
                break;
            }
        }
        Ok(())
    }

    fn alternating_interval(&self, on_ms: u128, off_ms: u128, on_offset: u128) -> bool {
        let elapsed_ms = self.started_at.elapsed().as_millis();
        let cycle_length = on_ms + off_ms;
        let offset_in_cycle = on_offset % cycle_length;
        let phase = (elapsed_ms + cycle_length - offset_in_cycle) % cycle_length;
        phase < on_ms
    }

    fn main_content<D: DrawTarget<Color = BinaryColor>>(
        &mut self,
        d: &mut D,
    ) -> Result<(), D::Error> {
        // test ticker
        Pixel(Point::new(29, 20), BinaryColor::On).draw(d)?;
        Pixel(Point::new(62, 20), BinaryColor::On).draw(d)?;
        self.text_ticker("this is a very long", Point::new(30, 20), 31, d)?;
        // name top left
        self.text(self.name.get_str(), Point::new(3, 6), d)?;
        let mut buf = [0; 20];
        // ip top right
        let ip_str = match self.network_status {
            NetworkStatus::NotConnected => "NO CONNECTION",
            NetworkStatus::ConnectionFailed => "WIFI FAILED",
            NetworkStatus::Connecting => "CONNECTING",
            NetworkStatus::Connected => {
                if let Some([a, b, c, d]) = self.ip {
                    format_no_std::show(&mut buf, format_args!("{a}.{b}.{c}.{d}"))
                        .unwrap_or("FORMAT ERR")
                } else {
                    "IP UNKNOWN"
                }
            }
        };
        self.text(
            ip_str,
            Point::new(ROBOT_DISPLAY_WIDTH as i32 - (5 * ip_str.len() as i32), 6),
            d,
        )?;
        // alive indicator
        self.pacman_animation2(10 + (15 - ip_str.len() as u128) * 5, 50, 38, 0, d)?;
        // network below ip
        let net_str = match self.ssid {
            None => "UNKNOWN NETWORK",
            Some((name, len)) => format_no_std::show(
                &mut buf,
                format_args!(
                    "{:>19}",
                    core::str::from_utf8(&name[..usize::min(len, 19)]).unwrap_or("FORMAT ERR1")
                ),
            )
            .unwrap_or("FORMAT ERR2"),
        };
        self.text(net_str, Point::new(33, 13), d)?;
        // distance sensors
        self.text(
            "dist",
            Point::new(3, ROBOT_DISPLAY_HEIGHT as i32 - (7 * 7)),
            d,
        )?;
        for i in 0..4 {
            let s = match self.distances[i] {
                Err(_) => " ERR",
                Ok(None) => "NONE",
                Ok(Some(d)) => {
                    format_no_std::show(&mut buf, format_args!("{:>4.1}", d)).unwrap_or("?")
                }
            };
            self.text(
                s,
                Point::new(3, ROBOT_DISPLAY_HEIGHT as i32 + 1 - (7 * (6 - i as i32))),
                d,
            )?;
        }
        // angle
        self.text(
            "angl",
            Point::new(3, ROBOT_DISPLAY_HEIGHT as i32 - 4 - 7),
            d,
        )?;
        let s = match self.imu_angle {
            Err(_) => " ERR",
            Ok(a) => format_no_std::show(&mut buf, format_args!("{:>4}", a.to_degrees() as i32))
                .unwrap_or("?"),
        };
        self.text(s, Point::new(3, ROBOT_DISPLAY_HEIGHT as i32 - 3), d)?;
        Ok(())
    }

    #[allow(dead_code)]
    fn arrow_animation<D: DrawTarget<Color = BinaryColor>>(
        &mut self,
        d: &mut D,
    ) -> Result<(), D::Error> {
        let indicator_speed = 30;
        for row in 1i32..=6 {
            for col in 35i32..=50 {
                let add_delay = match row {
                    1 | 6 => 6,
                    2 | 5 => 4,
                    _ => 2,
                };
                let color = self.alternating_interval(
                    indicator_speed * 6,
                    indicator_speed * 20,
                    indicator_speed * (col as u128 - 35 + add_delay as u128),
                );
                Pixel(Point::new(col, row), color.into()).draw(d)?;
            }
        }
        Ok(())
    }

    #[allow(dead_code)]
    fn pacman_animation<D: DrawTarget<Color = BinaryColor>>(
        &mut self,
        d: &mut D,
    ) -> Result<(), D::Error> {
        let x = 37;
        let y = 0;

        let tot_ms = 1700;
        let inc_ms = 55;

        for (x2, y2, delay) in [
            (0, 3, 0),
            (0, 2, 1),
            (0, 4, 1),
            (1, 1, 2),
            (1, 5, 2),
            (2, 0, 3),
            (2, 6, 3),
            (3, 0, 4),
            (3, 6, 4),
            (4, 0, 5),
            (4, 6, 5),
            (5, 1, 6),
            (5, 5, 6),
            (4, 2, 7),
            (4, 4, 7),
            (3, 3, 8),
            (6, 3, 11),
            (8, 3, 14),
            (10, 3, 17),
        ] {
            let color = self.alternating_interval(1400, tot_ms - inc_ms, inc_ms * delay);
            Pixel(Point::new(x + x2, y + y2), color.into()).draw(d)?;
        }

        Ok(())
    }

    #[allow(dead_code)]
    fn pacman_animation2<D: DrawTarget<Color = BinaryColor>>(
        &mut self,
        len_px: u128,
        speed: u128,
        x: i32,
        y: i32,
        d: &mut D,
    ) -> Result<(), D::Error> {
        let tot_ms = 2 * speed * len_px + 1200;
        let pellet_on_ms = speed * (len_px + 8);

        let dir = self.started_at.elapsed().as_millis() % (tot_ms * 2) < tot_ms;

        // pellets
        for i in (2..len_px).step_by(2) {
            let color = self.alternating_interval(pellet_on_ms, tot_ms - pellet_on_ms, speed * i);
            let x2 = if dir {
                i as i32
            } else {
                len_px as i32 - i as i32
            };
            Pixel(Point::new(x + x2, y + 3), color.into()).draw(d)?;
        }

        // pacman
        let phase = self.started_at.elapsed().as_millis() % tot_ms / speed;
        let pacman_start_x = phase as i32 - 10 - len_px as i32;
        for (x2, y2) in [
            (0, 3),
            (0, 2),
            (0, 4),
            (1, 1),
            (1, 5),
            (2, 0),
            (2, 6),
            (3, 0),
            (3, 6),
            (4, 0),
            (4, 6),
            (5, 1),
            (5, 5),
            (4, 2),
            (4, 4),
            (3, 3),
        ] {
            if 0 <= x2 + pacman_start_x && x2 + pacman_start_x <= len_px as i32 {
                let x2 = if dir {
                    x2 + pacman_start_x
                } else {
                    len_px as i32 - (x2 + pacman_start_x)
                };
                Pixel(Point::new(x + x2, y + y2), BinaryColor::On).draw(d)?;
            }
        }

        Ok(())
    }

    fn typing_test<D: DrawTarget<Color = BinaryColor>>(
        &mut self,
        d: &mut D,
    ) -> Result<(), D::Error> {
        self.text("Typing Test", Point::new(10, 10), d)?;
        Ok(())
    }
}

#[derive(Copy, Clone, Debug)]
struct TextInput<const L: usize> {
    buf: [u8; L],
    len: usize,
    selected: Option<usize>,
}

impl<const L: usize> TextInput<L> {
    pub fn new() -> Self {
        Self {
            buf: [32; L],
            len: 1,
            selected: None,
        }
    }
}
