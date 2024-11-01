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
    /// Simulated robot controls
    RobotButtonPanel,
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
            Tab::RobotButtonPanel => "Robot Button Panel",
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
                    Pos2::new(-1.0, 1.0),
                    Pos2::new(
                        ROBOT_DISPLAY_HEIGHT as f32 + 1.0,
                        ROBOT_DISPLAY_WIDTH as f32 + 1.0,
                    ),
                    Pos2::new(rect.top(), rect.left()),
                    Pos2::new(rect.bottom(), rect.right()),
                    false,
                );

                let painter = ui.painter_at(rect);

                painter.rect(
                    Rect::from_two_pos(
                        wts.map_point(Pos2::new(-1.0, 1.0)),
                        wts.map_point(Pos2::new(
                            ROBOT_DISPLAY_HEIGHT as f32 + 1.0,
                            ROBOT_DISPLAY_WIDTH as f32 + 1.0,
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
            Tab::RobotButtonPanel => {
                let rect = ui.max_rect();

                let wts = Transform::new_letterboxed(
                    Pos2::new(0.0, 0.0),
                    Pos2::new(4.0, 10.0),
                    Pos2::new(rect.top(), rect.left()),
                    Pos2::new(rect.bottom(), rect.right()),
                    false,
                );
                self.robot_buttons_wts = wts;

                let painter = ui.painter_at(rect);

                painter.rect(
                    Rect::from_two_pos(
                        wts.map_point(Pos2::new(0.0, 0.0)),
                        wts.map_point(Pos2::new(4.0, 10.0)),
                    ),
                    Rounding::ZERO,
                    Color32::BLACK,
                    Stroke::NONE,
                );

                for (x, y) in [
                    (8.0, 1.0),
                    (9.0, 2.0),
                    (7.0, 2.0),
                    (8.0, 3.0),
                    (4.3, 3.0),
                    (5.7, 3.0),
                ] {
                    painter.circle_filled(
                        wts.map_point(Pos2::new(y, x)),
                        wts.map_dist(0.4),
                        Color32::WHITE,
                    );
                }
            }
            Tab::Keybindings => {
                ui.label("General");
                ui.label("[Left click] Set simulated robot position");
                ui.label("[Right click] Set target position");
                ui.label("[Y] Toggle rotated grid");
                ui.label("[P] Toggle selected robot connection");
                ui.separator();
                ui.label("Movement, relative to estimated location");
                ui.label("[W] Up");
                ui.label("[A] Left");
                ui.label("[S] Down");
                ui.label("[D] Right");
                ui.label("[Q] Rotate counterclockwise");
                ui.label("[E] Rotate clockwise");
                ui.separator();
                ui.label("Test raw motor control");
                ui.label("[U] First motor forwards");
                ui.label("[J] First motor backwards");
                ui.label("[I] Second motor forwards");
                ui.label("[K] Second motor backwards");
                ui.label("[O] Third motor forwards");
                ui.label("[L] Third motor backwards");
                ui.separator();
                ui.label("Gameplay");
                ui.label("[space] Pause/unpause");
                ui.label("[R] Reset pacman game");
                ui.separator();
                ui.label("Strategy");
                ui.label("[Z] Manual");
                ui.label("[X] AI");
                ui.label("[C] Test uniform");
                ui.label("[V] Test forwards");
                ui.separator();
                ui.label("Grid");
                ui.label("[B] Pacman grid");
                ui.label("[N] Playground grid");
                ui.separator();
                ui.label("CV position source");
                ui.label("[T] Mouse pointer");
                ui.label("[G] Game state");
                ui.label("[H] Particle filter");
                // ui.separator();
                // ui.label("Replay controls");
                // ui.label("[shift + left] Go to beginning");
                // ui.label("[left] Previous frame");
                // ui.label("[space] Pause/unpause");
                // ui.label("[right] Next frame");
                // ui.label("[shift + right] Go to end");
            }
            _ => {
                ui.label(self.title(tab));
            }
        };
    }
}
