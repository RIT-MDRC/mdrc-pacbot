//! Top-level GUI elements and functionality.

pub mod transforms;

use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, RwLock};

use egui::{Color32, Frame, Key, Painter, Pos2, Rect, Rounding, Stroke, Ui};
use rand::rngs::ThreadRng;
use rapier2d::na::{Isometry2, Point2, Vector2};
use serde::{Deserialize, Serialize};

use crate::agent_setup::PacmanAgentSetup;
use crate::game_state::{GhostType, PacmanState};
use crate::grid::facing_direction;
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
#[derive(Default, Clone, Serialize, Deserialize)]
pub struct PhysicsRenderInfo {
    /// If true, the physics thread should not advance physics
    pub sleep: bool,
    /// The current position of the robot.
    pub pacbot_pos: Isometry2<f32>,
    /// An array of start and end points.
    pub primary_robot_rays: Vec<(Point2<f32>, Point2<f32>)>,
}

/// Stores state needed to render game state information
#[derive(Clone, Serialize, Deserialize)]
pub struct PacmanStateRenderInfo {
    /// If true, the game state thread should not advance the game state
    pub sleep: bool,
    /// Initial positions of Pacman, ghosts, etc.
    pub agent_setup: PacmanAgentSetup,
    /// Current game state
    pub pacman_state: PacmanState,
}

/// Thread where physics gets run.
fn run_physics(
    phys_render: Arc<RwLock<PhysicsRenderInfo>>,
    current_velocity: Arc<RwLock<(Vector2<f32>, f32)>>,
    location_send: Sender<Point2<u8>>,
    restart_recv: Receiver<(StandardGrid, Robot, Isometry2<f32>)>,
) {
    let mut simulation = PacbotSimulation::new(
        ComputedGrid::try_from(standard_grids::GRID_PACMAN).unwrap(),
        Robot::default(),
        Isometry2::new(Vector2::new(14.0, 7.0), 0.0),
    );

    let mut previous_pacbot_location = Point2::new(14, 7);

    loop {
        // Was a restart requested?
        if let Ok((grid, robot, isometry)) = restart_recv.try_recv() {
            simulation = PacbotSimulation::new(
                ComputedGrid::try_from(grid.get_grid()).unwrap(),
                robot,
                isometry,
            );
        }

        // Run simulation one step
        simulation.step();

        // Update the current velocity
        let target = *current_velocity.as_ref().read().unwrap();
        simulation.set_target_robot_velocity(target);

        // Update our render state
        *phys_render.write().unwrap() = PhysicsRenderInfo {
            sleep: false,
            pacbot_pos: *simulation.get_primary_robot_position(),
            primary_robot_rays: simulation.get_primary_robot_rays().clone(),
        };

        // Did pacbot's (rounded) position change? If so, send the new one to the game
        let pacbot_location = Point2::new(
            simulation
                .get_primary_robot_position()
                .translation
                .x
                .round() as u8,
            simulation
                .get_primary_robot_position()
                .translation
                .y
                .round() as u8,
        );

        if pacbot_location != previous_pacbot_location {
            location_send.send(pacbot_location).unwrap();
            previous_pacbot_location = pacbot_location;
        }

        // Sleep for 1/60th of a second
        std::thread::sleep(std::time::Duration::from_secs_f32(1. / 60.));
    }
}

fn run_game(
    pacman_render: Arc<RwLock<PacmanStateRenderInfo>>,
    location_receive: Receiver<Point2<u8>>,
) {
    let mut rng = ThreadRng::default();

    let mut previous_pacman_location = Point2::new(14u8, 7);

    loop {
        // {} block to make sure `game` goes out of scope and the RwLockWriteGuard is released
        {
            let mut state = pacman_render.write().unwrap();

            // fetch updated pacbot position
            while let Ok(pacbot_location) = location_receive.try_recv() {
                state.pacman_state.update_pacman(
                    pacbot_location,
                    facing_direction(&previous_pacman_location, &pacbot_location),
                );
                previous_pacman_location = pacbot_location;
            }

            let agent_setup = state.agent_setup.clone();

            // step the game
            if !state.pacman_state.paused {
                state.pacman_state.step(&agent_setup, &mut rng);
            }
        }

        // Sleep for 1/2 a second
        std::thread::sleep(std::time::Duration::from_secs_f32(1.0 / 2.5));
    }
}

struct App {
    selected_grid: StandardGrid,
    grid: ComputedGrid,
    pointer_pos: String,

    /// A read-only reference to info needed to render physics.
    phys_render: Arc<RwLock<PhysicsRenderInfo>>,
    target_velocity: Arc<RwLock<(Vector2<f32>, f32)>>,
    phys_restart_send: Sender<(StandardGrid, Robot, Isometry2<f32>)>,
    robot: Robot,

    pacman_render: Arc<RwLock<PacmanStateRenderInfo>>,
    agent_setup: PacmanAgentSetup,
}

impl Default for App {
    fn default() -> Self {
        let (location_send, location_receive) = channel();

        // Set up physics thread
        let target_velocity: Arc<RwLock<(Vector2<f32>, f32)>> = Arc::default();
        let phys_render: Arc<RwLock<PhysicsRenderInfo>> = Arc::default();
        let target_velocity_r = target_velocity.clone();
        let phys_render_w = phys_render.clone();
        let (phys_restart_send, phys_restart_recv) = channel();
        std::thread::spawn(move || {
            run_physics(
                phys_render_w,
                target_velocity_r,
                location_send,
                phys_restart_recv,
            );
        });

        let agent_setup = PacmanAgentSetup::default();
        let pacman_state = PacmanState::new(&agent_setup);
        let pacman_state_info = PacmanStateRenderInfo {
            sleep: false,
            pacman_state,
            agent_setup,
        };
        let pacman_render: Arc<RwLock<PacmanStateRenderInfo>> =
            Arc::new(RwLock::new(pacman_state_info));
        let pacman_state_rw = pacman_render.clone();
        std::thread::spawn(move || run_game(pacman_state_rw, location_receive));

        Self {
            selected_grid: StandardGrid::Pacman,
            grid: ComputedGrid::try_from(standard_grids::GRID_PACMAN).unwrap(),
            pointer_pos: "".to_string(),

            robot: Robot::default(),
            target_velocity,
            phys_restart_send,
            phys_render,

            pacman_render,
            agent_setup: PacmanAgentSetup::default(),
        }
    }
}

impl App {
    fn draw_grid(&mut self, ctx: &egui::Context, ui: &mut Ui) {
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

        if self.selected_grid == StandardGrid::Pacman {
            self.draw_pacman_state(&world_to_screen, &painter);
        }

        self.draw_simulation(&world_to_screen, &painter)
    }

    fn update_target_velocity(&mut self, ctx: &egui::Context) {
        let mut target_velocity = self.target_velocity.write().unwrap();
        target_velocity.0.x = 0.0;
        target_velocity.0.y = 0.0;
        target_velocity.1 = 0.0;
        ctx.input(|i| {
            let target_speed = if i.modifiers.shift { 10.0 } else { 4.0 };
            if i.key_down(Key::S) {
                target_velocity.0.y = -target_speed;
            }
            if i.key_down(Key::W) {
                target_velocity.0.y = target_speed;
            }
            if i.key_down(Key::A) {
                target_velocity.0.x = -target_speed;
            }
            if i.key_down(Key::D) {
                target_velocity.0.x = target_speed;
            }
            if i.key_down(Key::E) {
                target_velocity.1 = -target_speed;
            }
            if i.key_down(Key::Q) {
                target_velocity.1 = target_speed;
            }
        });
    }

    fn draw_simulation(&mut self, world_to_screen: &Transform, painter: &Painter) {
        let phys_render = self.phys_render.as_ref().read().unwrap();
        let pacbot_pos = phys_render.pacbot_pos;

        painter.circle_filled(
            world_to_screen.map_point(Pos2::new(
                pacbot_pos.translation.x,
                pacbot_pos.translation.y,
            )),
            world_to_screen.map_dist(self.robot.collider_radius),
            Color32::YELLOW,
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

    fn draw_pacman_state(&mut self, world_to_screen: &Transform, painter: &Painter) {
        let pacman_state_info = self.pacman_render.read().unwrap();
        let pacman_state = &pacman_state_info.pacman_state;

        // ghosts
        for i in 0..self.agent_setup.ghosts().len() {
            painter.circle_filled(
                world_to_screen.map_point(Pos2::new(
                    pacman_state.ghosts[i].agent.location.x as f32,
                    pacman_state.ghosts[i].agent.location.y as f32,
                )),
                world_to_screen.map_dist(0.45),
                match pacman_state.ghosts[i].color {
                    GhostType::Red => Color32::RED,
                    GhostType::Pink => Color32::from_rgb(255, 192, 203),
                    GhostType::Orange => Color32::from_rgb(255, 140, 0),
                    GhostType::Blue => Color32::BLUE,
                },
            )
        }

        // pellets
        for i in 0..pacman_state.pellets.len() {
            if pacman_state.pellets[i] {
                painter.circle_filled(
                    world_to_screen.map_point(Pos2::new(
                        self.agent_setup.grid().walkable_nodes()[i].x as f32,
                        self.agent_setup.grid().walkable_nodes()[i].y as f32,
                    )),
                    3.0,
                    Color32::BLUE,
                )
            }
        }

        // super pellets
        for super_pellet in &pacman_state.power_pellets {
            painter.circle_filled(
                world_to_screen.map_point(Pos2::new(super_pellet.x as f32, super_pellet.y as f32)),
                6.0,
                Color32::BLUE,
            )
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
                        self.pacman_render.write().unwrap().pacman_state.pause();
                        self.grid = ComputedGrid::try_from(grid.get_grid()).unwrap();
                        self.phys_restart_send
                            .send((
                                self.selected_grid,
                                Robot::default(),
                                self.selected_grid.get_default_pacbot_isometry(),
                            ))
                            .unwrap();
                    }
                });
            });
    }

    fn draw_playback_controls(&mut self, ctx: &egui::Context, ui: &mut Ui) {
        let mut game = self.pacman_render.write().unwrap();

        let space_pressed = ctx.input(|i| i.key_pressed(Key::Space));
        let arrow_right_pressed = ctx.input(|i| i.key_pressed(Key::ArrowRight));

        ui.horizontal(|ui| {
            if !game.pacman_state.paused {
                if ui.button("||").on_hover_text("Pause").clicked() || space_pressed {
                    game.pacman_state.paused = true;
                }
            } else {
                if game.pacman_state.lives == 0 {
                    if ui.button("Restart").clicked() || space_pressed {
                        game.pacman_state = PacmanState::new(&game.agent_setup);
                    }
                } else {
                    if ui.button("|>").on_hover_text("Play").clicked() || space_pressed {
                        game.pacman_state.paused = false;
                    }
                    if ui.button(">").on_hover_text("Advance one frame").clicked()
                        || arrow_right_pressed
                    {
                        game.pacman_state.resume();
                        game.pacman_state
                            .step(&self.agent_setup, &mut ThreadRng::default());
                        game.pacman_state.pause();
                    }
                }
            }
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
        if self.selected_grid == StandardGrid::Pacman {
            egui::TopBottomPanel::bottom("playback_controls")
                .frame(
                    Frame::none()
                        .fill(ctx.style().visuals.panel_fill)
                        .inner_margin(5.0),
                )
                .show(ctx, |ui| {
                    self.draw_playback_controls(ctx, ui);
                });
        }
        egui::CentralPanel::default()
            .frame(Frame::none().fill(ctx.style().visuals.panel_fill))
            .show(ctx, |ui| {
                self.draw_grid(ctx, ui);
            });
        ctx.request_repaint();
    }
}
