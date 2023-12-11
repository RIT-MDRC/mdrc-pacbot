//! Top-level GUI elements and functionality.

mod colors;
mod game;
mod physics;
pub mod replay_manager;
pub mod transforms;
pub mod utils;

use std::ops::Deref;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex, RwLock};

use eframe::egui;
use eframe::egui::{Frame, Key, Pos2, Ui};
use rapier2d::na::{Isometry2, Vector2};

use crate::agent_setup::PacmanAgentSetup;
use crate::constants::GUI_PARTICLE_FILTER_POINTS;
use crate::game_state::PacmanState;
use crate::grid::ComputedGrid;
use crate::gui::game::{run_game, PacmanStateRenderInfo};
use crate::gui::physics::{run_physics, PhysicsRenderInfo};
use crate::robot::Robot;
use crate::standard_grids::StandardGrid;
use crate::util::stopwatch::Stopwatch;

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

/// Indicates whether the game state should be taken from our simulations or from the comp server
#[derive(Clone, Copy, PartialEq, Eq)]
enum GameServer {
    Simulated,
}

/// Indicates the current meta-state of the app
#[derive(Clone, Copy, PartialEq, Eq)]
enum AppMode {
    /// Using a game server with physics engine and recording the results to file
    Recording(GameServer),
    /// Playing information back from a file; no game server but physics should still run
    Playback,
}

struct App {
    mode: AppMode,

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

    replay_manager: replay_manager::ReplayManager,
    pacman_state_notify_recv: Receiver<()>,
    /// When in playback mode, the position of pacbot from the replay
    replay_pacman: Isometry2<f32>,
    save_pacbot_location: bool,

    pf_stopwatch: Arc<Mutex<Stopwatch>>,
    physics_stopwatch: Arc<Mutex<Stopwatch>>,
    gui_stopwatch: Stopwatch,
}

fn pretty_print_time_now() -> String {
    let date = chrono::Local::now();
    date.format("%Y_%m_%d__%H_%M_%S").to_string()
}

impl Default for App {
    fn default() -> Self {
        let (location_send, location_receive) = channel();
        let (pacman_state_notify_send, pacman_state_notify_recv) = channel();

        // Set up physics thread
        let target_velocity: Arc<RwLock<(Vector2<f32>, f32)>> = Arc::default();
        let phys_render: Arc<RwLock<PhysicsRenderInfo>> =
            Arc::new(RwLock::new(PhysicsRenderInfo {
                sleep: false,
                pacbot_pos: StandardGrid::Pacman.get_default_pacbot_isometry(),
                primary_robot_rays: vec![],
                pf_count: GUI_PARTICLE_FILTER_POINTS,
                pf_points: vec![],
            }));
        let target_velocity_r = target_velocity.clone();
        let phys_render_w = phys_render.clone();
        let (phys_restart_send, phys_restart_recv) = channel();

        // Set up game state thread
        let agent_setup = PacmanAgentSetup::default();
        let pacman_state = PacmanState::new(&agent_setup);
        let pacman_state_info = PacmanStateRenderInfo {
            pacman_state,
            agent_setup,
        };
        let pacman_render: Arc<RwLock<PacmanStateRenderInfo>> =
            Arc::new(RwLock::new(pacman_state_info));
        let pacman_state_rw = pacman_render.clone();
        let pacman_replay_commands = pacman_state_notify_send.clone();

        // Set up replay manager
        let filename = format!("replays/replay-{}.bin", pretty_print_time_now());

        // Set up stopwatches
        let gui_stopwatch = Stopwatch::new(30);
        let pf_stopwatch = Arc::new(Mutex::new(Stopwatch::new(10)));
        let physics_stopwatch = Arc::new(Mutex::new(Stopwatch::new(10)));

        let pf_stopwatch_ref = pf_stopwatch.clone();
        let physics_stopwatch_ref = physics_stopwatch.clone();

        // Spawn threads
        std::thread::spawn(move || {
            run_game(pacman_state_rw, location_receive, pacman_replay_commands)
        });
        std::thread::spawn(move || {
            run_physics(
                phys_render_w,
                target_velocity_r,
                location_send,
                phys_restart_recv,
                Arc::new(Mutex::new(vec![Some(0.0); 8])),
                pf_stopwatch_ref,
                physics_stopwatch_ref,
            );
        });

        let pacbot_pos = phys_render.read().unwrap().pacbot_pos;

        Self {
            mode: AppMode::Recording(GameServer::Simulated),

            selected_grid: StandardGrid::Pacman,
            grid: StandardGrid::Pacman.compute_grid(),
            pointer_pos: "".to_string(),

            robot: Robot::default(),
            target_velocity,
            phys_restart_send,
            phys_render,

            pacman_render,
            agent_setup: PacmanAgentSetup::default(),

            replay_manager: App::new_replay_manager(
                filename,
                StandardGrid::Pacman,
                PacmanAgentSetup::default(),
                PacmanState::default(),
                pacbot_pos,
            ),
            pacman_state_notify_recv,
            replay_pacman: Isometry2::default(),
            save_pacbot_location: false,

            gui_stopwatch,
            pf_stopwatch,
            physics_stopwatch,
        }
    }
}

impl App {
    fn update_target_velocity(&mut self, ctx: &egui::Context) {
        let mut target_velocity = self.target_velocity.write().unwrap();
        target_velocity.0.x = 0.0;
        target_velocity.0.y = 0.0;
        target_velocity.1 = 0.0;
        ctx.input(|i| {
            let target_speed = if i.modifiers.shift { 2.0 } else { 0.8 };
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
                        self.grid = grid.compute_grid();
                        self.phys_render.write().unwrap().pacbot_pos =
                            self.selected_grid.get_default_pacbot_isometry();
                        self.phys_restart_send
                            .send((
                                self.selected_grid,
                                Robot::default(),
                                self.selected_grid.get_default_pacbot_isometry(),
                            ))
                            .unwrap();
                        self.reset_replay();
                    }
                });
            });
    }
}

fn draw_stopwatch(stopwatch: &Stopwatch, ctx: &egui::Context, name: &str) {
    egui::Window::new(name).show(ctx, |ui| {
        ui.label(format!(
            "Total: {:.2}",
            stopwatch.average_process_time() * 1000.0
        ));
        ui.separator();
        egui::Grid::new("")
            .num_columns(2)
            .striped(true)
            .show(ui, |ui| {
                let segment_times = stopwatch.average_segment_times();
                for (name, time) in segment_times {
                    ui.label(format!("{}", name));
                    ui.label(format!("{:.2}", time * 1000.0));
                    ui.end_row();
                }
            });
    });
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.gui_stopwatch.start();
        self.update_target_velocity(ctx);

        self.update_replay_manager()
            .expect("Error updating replay manager");
        self.gui_stopwatch.mark_segment("Update replay manager");

        egui::TopBottomPanel::top("menu").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    self.add_grid_variants(ui);
                    egui::menu::bar(ui, |ui| {
                        ui.menu_button("Replay", |ui| {
                            if ui.button("Save").clicked() {
                                self.save_replay().expect("Failed to save replay!");
                            }
                            if ui.button("Load").clicked() {
                                self.load_replay().expect("Failed to load replay!");
                            }
                            if ui
                                .add(
                                    egui::Button::new("Save Pacbot Location")
                                        .selected(self.save_pacbot_location),
                                )
                                .clicked()
                            {
                                self.save_pacbot_location = !self.save_pacbot_location;
                            }
                        });
                        ui.menu_button("Game", |ui| {
                            if ui.button("Reset").clicked() {
                                self.pacman_render
                                    .write()
                                    .unwrap()
                                    .pacman_state
                                    .reset(&self.agent_setup, true);
                            }
                        });
                    })
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
                    self.draw_replay_ui(ctx, ui);
                });
        }
        self.gui_stopwatch.mark_segment("Draw replay UI");

        egui::CentralPanel::default()
            .frame(Frame::none().fill(ctx.style().visuals.panel_fill))
            .show(ctx, |ui| {
                let rect = ui.max_rect();
                let (src_p1, src_p2) = self.selected_grid.get_soft_boundaries();

                let world_to_screen = Transform::new_letterboxed(
                    src_p1,
                    src_p2,
                    Pos2::new(rect.left(), rect.top()),
                    Pos2::new(rect.right(), rect.bottom()),
                );
                let painter = ui.painter_at(rect);

                self.draw_grid(ctx, &world_to_screen, &painter);
                self.gui_stopwatch.mark_segment("Draw grid");

                if self.selected_grid == StandardGrid::Pacman {
                    self.draw_pacman_state(ctx, &world_to_screen, &painter);
                }
                self.gui_stopwatch.mark_segment("Draw pacman state");

                self.draw_simulation(&world_to_screen, &painter);
                self.gui_stopwatch.mark_segment("Draw simulation");
            });

        draw_stopwatch(&self.gui_stopwatch, ctx, "GUI Time");
        draw_stopwatch(
            &self.physics_stopwatch.lock().unwrap().deref(),
            ctx,
            "Physics Time",
        );
        draw_stopwatch(&self.pf_stopwatch.lock().unwrap().deref(), ctx, "PF Time");
        self.gui_stopwatch.mark_segment("Draw stopwatches");

        ctx.request_repaint();
    }
}
