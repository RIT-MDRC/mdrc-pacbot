//! Top-level GUI elements and functionality.

pub mod transforms;

use egui::{Color32, Pos2, Rect, Rounding, Stroke, Ui};

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
    grid: ComputedGrid,
}

impl Default for App {
    fn default() -> Self {
        Self {
            grid: ComputedGrid::try_from(standard_grids::GRID_PACMAN).unwrap(),
        }
    }
}

impl App {
    fn draw_game(&mut self, ui: &mut Ui) {
        let (canvas_width, canvas_height) = (ui.available_width(), ui.available_height());
        let world_to_screen = Transform::new_letterboxed(
            Pos2::new(-1.0, 32.0),
            Pos2::new(32.0, -1.0),
            Pos2::new(0.0, 0.0),
            Pos2::new(canvas_width, canvas_height),
        );

        let wall_color = Color32::LIGHT_GRAY;

        let painter = ui.painter();
        for wall in self.grid.walls() {
            let (p1, p2) = world_to_screen.map_wall(wall);
            painter.rect(
                Rect::from_two_pos(p1, p2),
                Rounding::none(),
                wall_color,
                Stroke::new(1.0, wall_color),
            );
        }

        painter.circle_filled(
            world_to_screen.map_point(Pos2::new(14., 7.)),
            5.0,
            Color32::BLUE,
        )
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            self.draw_game(ui);
        });
    }
}
