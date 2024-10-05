use crate::drawing::game::{draw_game, draw_grid};
use crate::drawing::motors::draw_motors;
use crate::drawing::over_the_air::draw_over_the_air;
use crate::drawing::settings::draw_settings;
use crate::drawing::timings::draw_timings;
use crate::transform::Transform;
use crate::App;
use core_pb::constants::{ROBOT_DISPLAY_HEIGHT, ROBOT_DISPLAY_WIDTH};
use eframe::egui::{Color32, Pos2, Rect, Rounding, Stroke, Ui, WidgetText};
use egui_dock::TabViewer;

pub enum Tab {
    /// Main game grid
    Grid,
    /// Detailed timings
    Stopwatch,
    /// User settings
    Settings,
    /// Motor configuration and testing
    Motors,
    /// Robot view
    Robot,
    /// Keybindings
    Keybindings,
    /// Status of OTA programming
    OverTheAirProgramming,
    /// For widgets that don't have corresponding tabs
    Unknown,
    /// Simulated robot display
    RobotDisplay,
}

impl TabViewer for App {
    type Tab = Tab;

    fn title(&mut self, tab: &mut Self::Tab) -> WidgetText {
        match tab {
            Tab::Grid => "Main Grid",
            Tab::Stopwatch => "Stopwatch",
            Tab::Settings => "Settings",
            Tab::Motors => "Motors",
            Tab::Robot => "Robot",
            Tab::Keybindings => "Keybindings",
            Tab::OverTheAirProgramming => "OTA Programming",
            Tab::RobotDisplay => "Robot Display",
            Tab::Unknown => "?",
        }
        .into()
    }

    fn ui(&mut self, ui: &mut Ui, tab: &mut Self::Tab) {
        match tab {
            Tab::Grid => {
                let rect = ui.max_rect();
                let (src_p1, src_p2) = self.settings.standard_grid.get_soft_boundaries();

                self.world_to_screen = if self.rotated_grid {
                    Transform::new_letterboxed(
                        Pos2::new(src_p1.x, src_p2.y),
                        Pos2::new(src_p2.x, src_p1.y),
                        Pos2::new(rect.left(), rect.top()),
                        Pos2::new(rect.right(), rect.bottom()),
                        true,
                    )
                } else {
                    Transform::new_letterboxed(
                        Pos2::new(src_p1.x, src_p1.y),
                        Pos2::new(src_p2.x, src_p2.y),
                        Pos2::new(rect.top(), rect.left()),
                        Pos2::new(rect.bottom(), rect.right()),
                        false,
                    )
                };

                let painter = ui.painter_at(rect);
                draw_grid(self, &painter);
                draw_game(self, &painter);
            }
            Tab::Stopwatch => draw_timings(self, ui),
            Tab::Settings => draw_settings(self, ui),
            Tab::OverTheAirProgramming => draw_over_the_air(self, ui),
            Tab::Motors => draw_motors(self, ui),
            Tab::RobotDisplay => {
                let rect = ui.max_rect();

                let wts = Transform::new_letterboxed(
                    Pos2::new(0.0, 0.0),
                    Pos2::new(ROBOT_DISPLAY_HEIGHT as f32, ROBOT_DISPLAY_WIDTH as f32),
                    Pos2::new(rect.top(), rect.left()),
                    Pos2::new(rect.bottom(), rect.right()),
                    false,
                );

                let painter = ui.painter_at(rect);

                painter.rect(
                    Rect::from_two_pos(
                        wts.map_point(Pos2::new(0.0, 0.0)),
                        wts.map_point(Pos2::new(
                            ROBOT_DISPLAY_HEIGHT as f32,
                            ROBOT_DISPLAY_WIDTH as f32,
                        )),
                    ),
                    Rounding::ZERO,
                    Color32::BLACK,
                    Stroke::new(1.0, Color32::RED),
                );

                if let Some(display) =
                    &self.server_status.robots[self.ui_settings.selected_robot as usize].display
                {
                    for (y, row) in display.iter().enumerate() {
                        for x in 0u128..128 {
                            if (row >> x) % 2 == 1 {
                                painter.rect(
                                    Rect::from_two_pos(
                                        wts.map_point(Pos2::new(y as f32, x as f32)),
                                        wts.map_point(Pos2::new(y as f32 + 1.0, x as f32 + 1.0)),
                                    ),
                                    Rounding::ZERO,
                                    Color32::WHITE,
                                    Stroke::NONE,
                                );
                            }
                        }
                    }
                }
            }
            _ => {
                ui.label(self.title(tab));
            }
        };
    }
}
