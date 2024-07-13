use crate::drawing::game::{draw_game, draw_grid};
use crate::drawing::settings::draw_settings;
use crate::transform::Transform;
use crate::App;
use core_pb::grid::standard_grid::StandardGrid;
use eframe::egui::{Pos2, RichText, Ui, WidgetText};
use egui_dock::TabViewer;

pub enum Tab {
    /// Main game grid
    Grid,
    /// Detailed timings
    Stopwatch,
    /// User settings
    Settings,
    /// Robot view
    Robot,
    /// Keybindings
    Keybindings,
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
            Tab::Robot => "Robot",
            Tab::Keybindings => "Keybindings",
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
                if self.settings.standard_grid == StandardGrid::Pacman {
                    draw_game(self, &painter);
                }
            }
            Tab::Settings => draw_settings(self, ui),
            _ => {
                ui.label(match tab {
                    Tab::Grid => "Main Grid",
                    Tab::Stopwatch => "Stopwatch",
                    Tab::Settings => "Settings",
                    Tab::Robot => "Robot",
                    Tab::Keybindings => "Keybindings",
                    Tab::Unknown => "?",
                });
            }
        };
    }
}

/// A generic status indication
#[derive(Clone, Debug)]
pub enum PacbotWidgetStatus {
    /// Green
    Ok,
    /// Yellow
    Warn(String),
    /// Red
    Error(String),
    /// Grey
    NotApplicable,
}

trait PacbotWidget {
    fn update(&mut self, _tab_viewer: &App) {}
    fn display_name(&self) -> &'static str;
    fn button_text(&self) -> RichText;
    fn tab(&self) -> Tab {
        Tab::Unknown
    }
    fn overall_status(&self) -> &PacbotWidgetStatus {
        &PacbotWidgetStatus::NotApplicable
    }

    fn messages(&self) -> &[(String, PacbotWidgetStatus)] {
        &[]
    }
}
