use crate::messages::RobotButton;
use crate::robot_display::{DisplayManager, TextInput};
use crate::util::CrossPlatformInstant;
use embedded_graphics::draw_target::DrawTarget;
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::Point;
use embedded_graphics::{Drawable, Pixel};
use pacbot_rs::game_state::GameState;
use pacbot_rs::location::Direction;
use pacbot_rs::variables::{MAZE_COLS, MAZE_ROWS};

pub const MAX_TITLE_LEN: usize = 32;

#[derive(Copy, Clone, Debug, PartialOrd, PartialEq)]
pub enum Page {
    Main,

    Settings,
    SettingsCheckbox1,
    SettingsInput2,

    TypingTest,
    TypingTestEdit(usize),

    Pacman,
}

use crate::constants::ROBOT_DISPLAY_WIDTH;
use Page::*;

impl Page {
    pub fn submenu(&self) -> &'static [Page] {
        match self {
            Settings => &[SettingsCheckbox1, SettingsInput2, Pacman],
            TypingTest => &[TypingTestEdit(0), TypingTestEdit(1), TypingTestEdit(2)],
            _ => &[],
        }
    }

    pub fn title<'a>(&self, buf: &'a mut [u8; MAX_TITLE_LEN]) -> &'a str {
        match self {
            Main => "",
            Settings => "Settings",
            SettingsCheckbox1 => "Setting 1 this is a very long setting name",
            SettingsInput2 => "Setting 2",
            TypingTest => "Typing Test",
            TypingTestEdit(i) => {
                format_no_std::show(buf, format_args!("{i}")).unwrap_or("format err")
            }
            Pacman => "Pacman",
        }
    }

    pub fn upper_menu(&self) -> Option<(Self, usize)> {
        match self {
            SettingsCheckbox1 => Some((Settings, 0)),
            SettingsInput2 => Some((Settings, 1)),
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
        if !page.submenu().is_empty() {
            self.scroll_menu(page, d)
        } else {
            match page {
                Main => self.main_content(d),
                SettingsCheckbox1 => Ok(()),
                SettingsInput2 => Ok(()),
                TypingTestEdit(_) => Ok(()),
                Pacman => self.pacman(d),
                _ => unreachable!(),
            }
        }
    }

    pub fn consume(&mut self, button: RobotButton, pressed: bool) {
        let old_page = self.page;
        match (self.page, !self.page.submenu().is_empty(), button, pressed) {
            // anywhere, start
            (_, _, RobotButton::LeftStart, true) => self.page = Main,
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
            // any submenu, select
            (_, true, RobotButton::RightSelect, true) => {
                self.page = self.page.submenu()[self.submenu_index];
                self.submenu_index = 0;
            }
            // pacman
            (Pacman, _, button, true) => match button {
                RobotButton::RightSelect => {
                    self.game_state =
                        GameState::new_with_seed(self.initial_time.elapsed().as_micros() as u64);
                    self.game_state.paused = false;
                    self.game_state.step();
                }
                RobotButton::NorthX => self.game_state.move_pacman_dir(Direction::Left),
                RobotButton::EastA => self.game_state.move_pacman_dir(Direction::Down),
                RobotButton::SouthB => self.game_state.move_pacman_dir(Direction::Right),
                RobotButton::WestY => self.game_state.move_pacman_dir(Direction::Up),
                _ => {}
            },
            _ => {
                // if this page is a text edit field, send the input to it
                if let Some(_text_edit) = self.page.clone().text_edit(self) {
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
        if self.page != old_page {
            self.animation_timer = I::default();
            if !self.page.submenu().is_empty() {
                self.submenu_index %= self.page.submenu().len();
            }
        }
    }

    fn scroll_menu<D: DrawTarget<Color = BinaryColor>>(
        &mut self,
        page: Page,
        d: &mut D,
    ) -> Result<(), D::Error> {
        let mut buf = [0; 32];
        self.text_ticker(
            page.title(&mut buf),
            Point::new(3, 6),
            ROBOT_DISPLAY_WIDTH - 5,
            d,
        )?;
        for (i, submenu) in page.submenu().iter().enumerate() {
            self.text_ticker(
                submenu.title(&mut buf),
                Point::new(3 + 5, 13 + i as i32 * 7),
                ROBOT_DISPLAY_WIDTH - 5 - 3,
                d,
            )?;
        }
        self.text(">", Point::new(3, 13 + 7 * self.submenu_index as i32), d)?;
        Ok(())
    }

    fn pacman<D: DrawTarget<Color = BinaryColor>>(&mut self, d: &mut D) -> Result<(), D::Error> {
        let base_x = 60;
        let base_y = 4;
        if self.last_game_state_step.elapsed().as_millis() > (1000 / 24) {
            self.last_game_state_step = I::default();
            self.game_state.step();
        }
        // walls/pellets
        for row in 0..(MAZE_ROWS as i8 + 1) {
            for col in 0..(MAZE_COLS as i8 + 1) {
                if self.pixel_is_pacman_wall(row, col) {
                    Pixel(
                        Point::new(row as i32 * 2 + base_x, col as i32 * 2 + base_y),
                        BinaryColor::On,
                    )
                    .draw(d)?;
                    if self.pixel_is_pacman_wall(row + 1, col) && row < MAZE_ROWS as i8 {
                        Pixel(
                            Point::new(row as i32 * 2 + base_x + 1, col as i32 * 2 + base_y),
                            BinaryColor::On,
                        )
                        .draw(d)?;
                    }
                    if self.pixel_is_pacman_wall(row, col + 1) && col < MAZE_COLS as i8 {
                        Pixel(
                            Point::new(row as i32 * 2 + base_x, col as i32 * 2 + base_y + 1),
                            BinaryColor::On,
                        )
                        .draw(d)?;
                    }
                    if self.pixel_is_pacman_wall(row, col + 1)
                        && self.pixel_is_pacman_wall(row + 1, col)
                        && self.pixel_is_pacman_wall(row + 1, col + 1)
                        && col < MAZE_COLS as i8
                        && row < MAZE_ROWS as i8
                    {
                        Pixel(
                            Point::new(row as i32 * 2 + base_x + 1, col as i32 * 2 + base_y + 1),
                            BinaryColor::On,
                        )
                        .draw(d)?;
                    }
                }
                // pellet
                if self.game_state.pellet_at((row, col)) {
                    Pixel(
                        Point::new(row as i32 * 2 + base_x + 1, col as i32 * 2 + base_y + 1),
                        BinaryColor::On,
                    )
                    .draw(d)?;
                }
            }
        }
        // pacman
        let (pac_row, pac_col) = (
            self.game_state.pacman_loc.row,
            self.game_state.pacman_loc.col,
        );
        for (r, c) in [(1, 0), (0, 1), (1, 1), (2, 1), (1, 2)] {
            Pixel(
                Point::new(
                    base_x + pac_row as i32 * 2 + r,
                    base_y + pac_col as i32 * 2 + c,
                ),
                BinaryColor::On,
            )
            .draw(d)?;
        }
        // ghosts
        for ghost in &self.game_state.ghosts {
            if ghost.is_frightened() && self.alternating_interval(150, 150, 0) {
                continue;
            }
            let (ghost_row, ghost_col) = (ghost.loc.row, ghost.loc.col);
            for (r, c) in [(1, 0), (0, 1), (1, 1), (2, 1), (0, 2), (2, 2)] {
                Pixel(
                    Point::new(
                        base_x + ghost_row as i32 * 2 + r,
                        base_y + ghost_col as i32 * 2 + c,
                    ),
                    BinaryColor::On,
                )
                .draw(d)?;
            }
        }
        // text
        let mut buf = [0; 32];
        self.text(
            format_no_std::show(
                &mut buf,
                format_args!("Score {}", self.game_state.curr_score),
            )
            .unwrap_or("format err"),
            Point::new(3, 6),
            d,
        )?;
        self.text(
            format_no_std::show(
                &mut buf,
                format_args!("Lives {}", self.game_state.curr_lives),
            )
            .unwrap_or("format err"),
            Point::new(3, 13),
            d,
        )?;
        self.text(
            format_no_std::show(
                &mut buf,
                format_args!("Level {}", self.game_state.curr_level),
            )
            .unwrap_or("format err"),
            Point::new(3, 20),
            d,
        )?;
        Ok(())
    }

    fn pixel_is_pacman_wall(&self, row: i8, col: i8) -> bool {
        self.game_state.wall_at((row, col))
            && self.game_state.wall_at((row - 1, col))
            && self.game_state.wall_at((row, col - 1))
            && self.game_state.wall_at((row - 1, col - 1))
    }
}
