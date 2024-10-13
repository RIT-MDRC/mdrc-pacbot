use crate::messages::RobotButton;
use crate::robot_display::{DisplayManager, TextInput};
use crate::util::CrossPlatformInstant;
use embedded_graphics::draw_target::DrawTarget;
use embedded_graphics::pixelcolor::BinaryColor;

pub const MAX_TITLE_LEN: usize = 32;

#[derive(Copy, Clone, Debug, PartialOrd, PartialEq)]
pub enum Page {
    Main,

    Settings,
    SettingsCheckbox1,
    SettingsCheckbox2,

    TypingTest,
    TypingTestEdit(usize),
}

use Page::*;

impl Page {
    pub fn submenu(&self) -> &'static [Page] {
        match self {
            Settings => &[SettingsCheckbox1, SettingsCheckbox2],
            TypingTest => &[TypingTestEdit(0), TypingTestEdit(1), TypingTestEdit(2)],
            _ => &[],
        }
    }

    pub fn title<'a>(&self, buf: &'a mut [u8; MAX_TITLE_LEN]) -> &'a str {
        match self {
            Main => "",
            Settings => "Settings",
            SettingsCheckbox1 => "Setting 1",
            SettingsCheckbox2 => "Setting 2",
            TypingTest => "Typing Test",
            TypingTestEdit(i) => {
                format_no_std::show(buf, format_args!("{i}")).unwrap_or("format err")
            }
        }
    }

    pub fn upper_menu(&self) -> Option<(Self, usize)> {
        match self {
            SettingsCheckbox1 => Some((Settings, 0)),
            SettingsCheckbox2 => Some((Settings, 1)),
            TypingTestEdit(i) => Some((TypingTest, *i)),
            _ => None,
        }
    }

    fn text_edit<'a, I: CrossPlatformInstant + Default>(
        &self,
        dm: &'a mut DisplayManager<I>,
    ) -> Option<&'a mut TextInput<32>> {
        match self {
            TypingTestEdit(i) => Some(&mut dm.typing_tests[*i]),
            _ => None,
        }
    }
}

impl<I: CrossPlatformInstant + Default> DisplayManager<I> {
    pub fn draw_content<D: DrawTarget<Color = BinaryColor>>(
        &mut self,
        page: Page,
        d: &mut D,
    ) -> Result<(), D::Error> {
        if page.submenu().len() > 0 {
            self.scroll_menu(page, d)
        } else {
            match page {
                Main => self.main_content(d),
                SettingsCheckbox1 => Ok(()),
                SettingsCheckbox2 => Ok(()),
                TypingTestEdit(_) => Ok(()),
                _ => unreachable!(),
            }
        }
    }

    pub fn consume(&mut self, button: RobotButton, pressed: bool) {
        match (self.page, self.page.submenu().len() > 0, button, pressed) {
            // outer menu pages, forwards
            (Main, _, RobotButton::EastA, true) => self.page = Settings,
            (Settings, _, RobotButton::EastA, true) => self.page = TypingTest,
            (TypingTest, _, RobotButton::EastA, true) => self.page = Main,
            // outer menu pages, backwards
            (Main, _, RobotButton::WestY, true) => self.page = TypingTest,
            (Settings, _, RobotButton::WestY, true) => self.page = Main,
            (TypingTest, _, RobotButton::WestY, true) => self.page = Settings,
            // any submenu, down
            (_, true, RobotButton::SouthB, true) => {
                self.submenu_index = (self.submenu_index + 1) % self.page.submenu().len()
            }
            // any submenu, up
            (_, true, RobotButton::NorthX, true) => {
                let len = self.page.submenu().len();
                self.submenu_index = (self.submenu_index + len - 1) % len
            }
            _ => {
                // if this page is a text edit field, send the input to it
                if let Some(text_edit) = self.page.clone().text_edit(self) {
                    // todo send button to text_edit
                    if (button == RobotButton::LeftStart || button == RobotButton::RightSelect)
                        && pressed
                    {
                        if let Some((upper_menu, submenu_index)) = self.page.upper_menu() {
                            self.page = upper_menu;
                            self.submenu_index = submenu_index;
                        }
                    }
                }
            }
        }
    }

    fn scroll_menu<D: DrawTarget<Color = BinaryColor>>(
        &mut self,
        page: Page,
        d: &mut D,
    ) -> Result<(), D::Error> {
        todo!()
    }
}
