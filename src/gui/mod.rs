//! Top-level GUI elements and functionality.

pub mod transforms;

use std::sync::{Arc, RwLock};

use egui::accesskit::Point;
use egui::{Color32, Frame, Key, Painter, Pos2, Rect, Rounding, Stroke, Ui};
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

/// Stores state needed to render physics information.
#[derive(Default)]
pub struct PhysicsRenderInfo {
    /// The current position of the robot.
    pub pacbot_pos: Isometry2<f32>,
    /// An array of start and end points.
    pub primary_robot_rays: Vec<(Point2<f32>, Point2<f32>)>,
}

/// Thread where physics gets run.
fn run_physics(
    phys_render: Arc<RwLock<PhysicsRenderInfo>>,
    current_velocity: Arc<RwLock<Vector2<f32>>>,
) {
    let mut simulation = PacbotSimulation::new(
        ComputedGrid::try_from(standard_grids::GRID_PACMAN).unwrap(),
        Robot::default(),
        Isometry2::new(Vector2::new(14.0, 7.0), 0.0),
    );
    loop {
        // Run simulation one step
        simulation.step();

        // Update the current veloicty
        simulation.set_target_robot_velocity(*current_velocity.as_ref().read().unwrap());

        // Update our render state
        *phys_render.write().unwrap() = PhysicsRenderInfo {
            pacbot_pos: *simulation.get_primary_robot_position(),
            primary_robot_rays: simulation.get_primary_robot_rays().clone(),
        };

        // Sleep for 1/60th of a second
        std::thread::sleep(std::time::Duration::from_secs_f32(1. / 60.));
    }
}

struct App {
    selected_grid: StandardGrid,
    grid: ComputedGrid,
    pointer_pos: String,

    /// A read-only reference to info needed to render physics.
    phys_render: Arc<RwLock<PhysicsRenderInfo>>,
    target_velocity: Arc<RwLock<Vector2<f32>>>,
    robot: Robot,
}

impl Default for App {
    fn default() -> Self {
        // Set up physics thread
        let target_velocity: Arc<RwLock<Vector2<f32>>> = Arc::default();
        let phys_render: Arc<RwLock<PhysicsRenderInfo>> = Arc::default();
        let target_velocity_r = target_velocity.clone();
        let phys_render_w = phys_render.clone();
        std::thread::spawn(move || {
            run_physics(phys_render_w, target_velocity_r);
        });

        Self {
            selected_grid: StandardGrid::Pacman,
            grid: ComputedGrid::try_from(standard_grids::GRID_PACMAN).unwrap(),
            pointer_pos: "".to_string(),

            robot: Robot::default(),
            target_velocity,
            phys_render,
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

        self.update_target_velocity(ctx);

        self.draw_physics(world_to_screen, &painter)
    }

    fn update_target_velocity(&mut self, ctx: &egui::Context) {
        let mut target_velocity = self.target_velocity.write().unwrap();
        ctx.input(|i| {
            if i.key_pressed(Key::W) {
                target_velocity.y = 1.0;
            } else if i.key_released(Key::W) {
                target_velocity.y = 0.0;
            }
            if i.key_pressed(Key::S) {
                target_velocity.y = -1.0;
            } else if i.key_released(Key::S) {
                target_velocity.y = 0.0;
            }
            if i.key_pressed(Key::A) {
                target_velocity.x = -1.0;
            } else if i.key_released(Key::A) {
                target_velocity.x = 0.0;
            }
            if i.key_pressed(Key::D) {
                target_velocity.x = 1.0;
            } else if i.key_released(Key::D) {
                target_velocity.x = 0.0;
            }
        });
    }

    fn draw_physics(&mut self, world_to_screen: Transform, painter: &Painter) {
        let phys_render = self.phys_render.as_ref().read().unwrap();
        let pacbot_pos = phys_render.pacbot_pos;

        painter.circle_filled(
            world_to_screen.map_point(Pos2::new(
                pacbot_pos.translation.x,
                pacbot_pos.translation.y,
            )),
            world_to_screen.map_dist(self.robot.collider_radius),
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
            Stroke::new(2.0, Color32::BLUE),
        );

        let distance_sensor_rays = &phys_render.primary_robot_rays;

        for (s, f) in distance_sensor_rays.iter() {
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
        ctx.request_repaint();
    }
}
