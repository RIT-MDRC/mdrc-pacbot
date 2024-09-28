use crate::drawing::game::{draw_game, draw_grid};
use crate::drawing::motors::draw_motors;
use crate::drawing::over_the_air::draw_over_the_air;
use crate::drawing::settings::draw_settings;
use crate::drawing::timings::draw_timings;
use crate::transform::Transform;
use crate::App;
use eframe::egui::{Pos2, Ui, WidgetText};
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
            _ => {
                ui.label(match tab {
                    Tab::Grid => "Main Grid",
                    Tab::Stopwatch => "Stopwatch",
                    Tab::Settings => "Settings",
                    Tab::Robot => "Robot",
                    Tab::Motors => "Motors",
                    Tab::Keybindings => "Keybindings",
                    Tab::OverTheAirProgramming => "OTA Programming",
                    Tab::Unknown => "?",
                });
            }
        };
    }
}
