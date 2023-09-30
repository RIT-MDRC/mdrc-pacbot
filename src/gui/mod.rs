//! Top-level GUI elements and functionality.

pub mod transforms;

use egui::{Color32, Frame, Painter, Pos2, Rect, Rounding, Stroke, Ui};
use rapier2d::math::Rotation;
use rapier2d::na::{Isometry2, Point2, Vector2};

use crate::robot::Robot;
use crate::simulation::PacbotSimulation;
use crate::standard_grids::StandardGrid;
use crate::{grid::ComputedGrid, standard_grids};

use self::transforms::Transform;

/// Launches the GUI application. Blocks until the application has quit.
pub fn run_gui() {
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "PacBot simulation",
        native_options,
        Box::new(|_cc| Box::<App>::default()),
    )
    .expect("eframe::run_native error");
}

struct App {
    selected_grid: StandardGrid,
    grid: ComputedGrid,
    pointer_pos: String,

    simulation: PacbotSimulation,
}

impl Default for App {
    fn default() -> Self {
        Self {
            selected_grid: StandardGrid::Pacman,
            grid: ComputedGrid::try_from(standard_grids::GRID_PACMAN).unwrap(),
            pointer_pos: "".to_string(),

            simulation: PacbotSimulation::new(
                ComputedGrid::try_from(standard_grids::GRID_PACMAN).unwrap(),
                Robot::default(),
                Isometry2::new(Vector2::new(14.0, 7.0), 0.0),
            ),
        }
    }
}

impl App {
    fn draw_game(&mut self, ctx: &egui::Context, ui: &mut Ui) {
        let rect = ui.max_rect();

        let world_to_screen = Transform::new_letterboxed(
            Pos2::new(-1.0, 32.0),
            Pos2::new(32.0, -1.0),
            Pos2::new(rect.left(), rect.top()),
            Pos2::new(rect.right(), rect.bottom()),
        );

        self.pointer_pos = match ctx.pointer_latest_pos() {
            None => "".to_string(),
            Some(pos) => {
                let pos = world_to_screen.inverse().map_point(pos);
                format!("({:.1}, {:.1})", pos.x, pos.y)
            }
        };

        let wall_color = Color32::LIGHT_GRAY;

        let painter = ui.painter_at(rect);
        for wall in self.grid.walls() {
            let (p1, p2) = world_to_screen.map_wall(wall);
            painter.rect(
                Rect::from_two_pos(p1, p2),
                Rounding::none(),
                wall_color,
                Stroke::new(1.0, wall_color),
            );
        }

        self.draw_physics(world_to_screen, &painter)
    }

    fn draw_physics(&mut self, world_to_screen: Transform, painter: &Painter) {
        self.simulation.step();

        let pacbot_pos = self.simulation.get_primary_robot_position();

        println!("{}", pacbot_pos.rotation.angle());

        painter.circle_filled(
            world_to_screen.map_point(Pos2::new(
                pacbot_pos.translation.x,
                pacbot_pos.translation.y,
            )),
            2.0,
            Color32::RED,
        );

        let pacbot_front = pacbot_pos.rotation.transform_point(&Point2::new(0.45, 0.0));

        painter.line_segment(
            [
                world_to_screen.map_point(Pos2::new(
                    pacbot_pos.translation.x,
                    pacbot_pos.translation.y,
                )),
                world_to_screen.map_point(Pos2::new(
                    pacbot_front.x + pacbot_pos.translation.x,
                    pacbot_front.y + pacbot_pos.translation.y,
                )),
            ],
            Stroke::new(1.0, Color32::YELLOW),
        );

        let distance_sensor_rays = self.simulation.get_primary_robot_rays();

        for (s, f) in distance_sensor_rays.iter().flatten() {
            painter.line_segment(
                [
                    world_to_screen.map_point(Pos2::new(s.x, s.y)),
                    world_to_screen.map_point(Pos2::new(f.x, f.y)),
                ],
                Stroke::new(1.0, Color32::GREEN),
            );
        }
    }

    fn add_grid_variants(&mut self, ui: &mut Ui) {
        egui::ComboBox::from_label("")
            .selected_text(format!("{:?}", self.selected_grid))
            .show_ui(ui, |ui| {
                StandardGrid::get_all().iter().for_each(|grid| {
                    if ui
                        .selectable_value(&mut self.selected_grid, *grid, format!("{:?}", grid))
                        .clicked()
                    {
                        self.grid = ComputedGrid::try_from(grid.get_grid()).unwrap();
                    }
                });
            });
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("menu").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    self.add_grid_variants(ui);
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(&self.pointer_pos);
                });
            });
        });
        egui::CentralPanel::default()
            .frame(Frame::none().fill(ctx.style().visuals.panel_fill))
            .show(ctx, |ui| {
                self.draw_game(ctx, ui);
            });
    }
}
