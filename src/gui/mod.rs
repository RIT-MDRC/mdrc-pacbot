//! Top-level GUI elements and functionality.

mod colors;
mod game;
mod physics;
pub mod replay_manager;
mod stopwatch;
pub mod transforms;
pub mod utils;

use std::ops::{Deref, DerefMut};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};

use eframe::egui;
use eframe::egui::{Align, Color32, Frame, Key, Pos2, RichText, Ui, WidgetText};
use egui_dock::{DockArea, DockState, Style};
use egui_phosphor::regular;
use pacbot_rs::game_engine::GameEngine;
use rapier2d::na::{Isometry2, Vector2};

use crate::constants::GUI_PARTICLE_FILTER_POINTS;
use crate::grid::standard_grids::StandardGrid;
use crate::grid::ComputedGrid;
use crate::gui::colors::{
    TRANSLUCENT_GREEN_COLOR, TRANSLUCENT_RED_COLOR, TRANSLUCENT_YELLOW_COLOR,
};
use crate::gui::game::{run_game, GameWidget, PacmanStateRenderInfo};
use crate::gui::physics::{run_physics, PhysicsRenderInfo};
use crate::gui::stopwatch::StopwatchWidget;
use crate::high_level::HighLevelContext;
use crate::network::{start_network_thread, NetworkCommand};
use crate::robot::Robot;
use crate::util::stopwatch::Stopwatch;

use self::transforms::Transform;

#[derive(Copy, Clone)]
pub enum Tab {
    Grid,
    Stopwatch,
    Unknown,
}

/// Thread where high level AI makes decisions.
fn run_high_level(
    pacman_state: Arc<RwLock<PacmanStateRenderInfo>>,
    target_pos: Arc<RwLock<(usize, usize)>>,
) {
    let mut hl_ctx = HighLevelContext::new("./checkpoints/q_net.safetensors");
    let std_grid = StandardGrid::Pacman.compute_grid();

    loop {
        // Use AI to indicate which direction to move.
        let pacman_state_render = pacman_state.read().unwrap();
        if !pacman_state_render.pacman_state.is_paused() {
            let state = pacman_state_render.pacman_state.get_state();
            let action = hl_ctx.step(state, &std_grid);
            let curr_pos = (state.pacman_loc.row as usize, state.pacman_loc.col as usize);
            drop(pacman_state_render); // Allow others to read this resource.
            let mut target_pos = target_pos.write().unwrap();
            *target_pos = match action {
                crate::high_level::HLAction::Stay => curr_pos,
                crate::high_level::HLAction::Left => (curr_pos.0, curr_pos.1 - 1),
                crate::high_level::HLAction::Right => (curr_pos.0, curr_pos.1 + 1),
                crate::high_level::HLAction::Up => (curr_pos.0 - 1, curr_pos.1),
                crate::high_level::HLAction::Down => (curr_pos.0 + 1, curr_pos.1),
            };
            drop(target_pos);
        } else {
            drop(pacman_state_render); // Allow others to read this resource.
        }

        // Sleep for 1/8th of a second.
        std::thread::sleep(std::time::Duration::from_secs_f32(1. / 8.));
    }
}

/// Thread where the velocity is modified to go to the target position.
/// The robot must already be near the target position
fn run_pos_to_target_vel(
    pacman_state: Arc<RwLock<PacmanStateRenderInfo>>,
    phys_render: Arc<RwLock<PhysicsRenderInfo>>,
    target_pos: Arc<RwLock<(usize, usize)>>,
    target_velocity: Arc<RwLock<(Vector2<f32>, f32)>>,
    ai_enabled: Arc<RwLock<bool>>,
) {
    loop {
        let is_paused = pacman_state.read().unwrap().pacman_state.is_paused();
        let ai_enabled = *ai_enabled.read().unwrap().deref();

        if !is_paused && ai_enabled {
            let curr_pos = phys_render
                .read()
                .unwrap()
                .pacbot_pos
                .translation
                .vector
                .xy();

            let target_pos = *target_pos.read().unwrap();
            let target_pos = Vector2::new(target_pos.0 as f32, target_pos.1 as f32);

            let max_speed = 40.;
            let mut delta_pos = target_pos - curr_pos;
            if delta_pos.magnitude() > max_speed {
                delta_pos = delta_pos.normalize() * max_speed;
            }
            delta_pos *= 2.;
            let mut target_velocity = target_velocity.write().unwrap();
            *target_velocity = (delta_pos, target_velocity.1);
            drop(target_velocity);
        } else {
            let mut target_velocity = target_velocity.write().unwrap();
            *target_velocity = (Vector2::zeros(), target_velocity.1);
            drop(target_velocity);
        }

        // Sleep for 1/30th of a second.
        std::thread::sleep(std::time::Duration::from_secs_f32(1. / 30.));
    }
}

struct TabViewer {
    mode: AppMode,
    ai_enable: Arc<RwLock<bool>>,

    grid_widget: GridWidget,
    game_widget: GameWidget,
    stopwatch_widget: StopwatchWidget,
    ai_widget: AiWidget,
    sensors_widget: PacbotSensorsWidget,

    selected_grid: StandardGrid,
    grid: ComputedGrid,
    pointer_pos: String,
    background_color: Color32,

    /// A read-only reference to info needed to render physics.
    phys_render: Arc<RwLock<PhysicsRenderInfo>>,
    target_velocity: Arc<RwLock<(Vector2<f32>, f32)>>,
    target_pos: Arc<RwLock<(usize, usize)>>,
    phys_restart_send: Sender<(StandardGrid, Robot, Isometry2<f32>)>,
    robot: Robot,

    pacman_render: Arc<RwLock<PacmanStateRenderInfo>>,
    world_to_screen: Option<Transform>,

    replay_manager: replay_manager::ReplayManager,
    pacman_state_notify_recv: Receiver<()>,
    /// When in playback mode, the position of pacbot from the replay
    replay_pacman: Isometry2<f32>,
    save_pacbot_location: bool,

    network_command_send: tokio::sync::mpsc::Sender<NetworkCommand>,

    pf_stopwatch: Arc<RwLock<Stopwatch>>,
    physics_stopwatch: Arc<RwLock<Stopwatch>>,
}

impl egui_dock::TabViewer for TabViewer {
    type Tab = Tab;

    fn title(&mut self, tab: &mut Self::Tab) -> WidgetText {
        match tab {
            Tab::Grid => WidgetText::from("Main Grid"),
            Tab::Stopwatch => WidgetText::from("Stopwatch"),
            _ => panic!("Widget did not declare a tab!"),
        }
    }

    fn ui(&mut self, ui: &mut Ui, tab: &mut Self::Tab) {
        match tab {
            Tab::Grid => self.grid_ui(ui),
            Tab::Stopwatch => {
                ui.label("Particle Filter");
                draw_stopwatch(&self.pf_stopwatch.read().unwrap(), ui, "pf_sw".to_string());
                ui.separator();
                ui.label("Physics");
                draw_stopwatch(
                    &self.physics_stopwatch.read().unwrap(),
                    ui,
                    "ph_sw".to_string(),
                );
                // draw_stopwatch(&self.gui_stopwatch.read().unwrap(), ui);
            }
            _ => panic!("Widget did not declare a tab!"),
        }
    }
}

impl Default for TabViewer {
    fn default() -> Self {
        let (location_send, location_receive) = channel();
        let (pacman_state_notify_send, pacman_state_notify_recv) = channel();

        // Set up physics thread
        let target_velocity: Arc<RwLock<(Vector2<f32>, f32)>> = Arc::default();
        let target_pos: Arc<RwLock<(usize, usize)>> = Arc::new(RwLock::new((23, 13)));
        let phys_render: Arc<RwLock<PhysicsRenderInfo>> =
            Arc::new(RwLock::new(PhysicsRenderInfo {
                sleep: false,
                pacbot_pos: StandardGrid::Pacman.get_default_pacbot_isometry(),
                pacbot_pos_guess: StandardGrid::Pacman.get_default_pacbot_isometry(),
                primary_robot_rays: vec![],
                pf_count: GUI_PARTICLE_FILTER_POINTS,
                pf_points: vec![],
            }));
        let target_velocity_r = target_velocity.clone();
        let target_pos_rw = target_pos.clone();
        let phys_render_w = phys_render.clone();
        let (phys_restart_send, phys_restart_recv) = channel();

        // Set up game state thread
        let pacman_state = GameEngine::default();
        let pacman_state_info = PacmanStateRenderInfo { pacman_state };
        let pacman_render: Arc<RwLock<PacmanStateRenderInfo>> =
            Arc::new(RwLock::new(pacman_state_info));
        let pacman_state_rw = pacman_render.clone();
        let hl_game_state = pacman_state_rw.clone();
        let pacman_replay_commands = pacman_state_notify_send.clone();

        // Set up replay manager
        let filename = format!("replays/replay-{}.bin", pretty_print_time_now());

        // Set up stopwatches
        let (stopwatch_widget, stopwatches) = StopwatchWidget::new();
        let pf_stopwatch = stopwatches[2].clone();
        let physics_stopwatch = stopwatches[1].clone();

        let pf_stopwatch_ref = pf_stopwatch.clone();
        let physics_stopwatch_ref = physics_stopwatch.clone();

        let ai_enabled = Arc::new(RwLock::new(true));
        let sensors = Arc::new(RwLock::new((false, [0; 8], [0; 3], Instant::now())));

        // Spawn threads
        std::thread::spawn(move || {
            run_game(pacman_state_rw, location_receive, pacman_replay_commands)
        });
        {
            let hl_game_state = hl_game_state.clone();
            let phys_render_r = phys_render_w.clone();
            let target_pos_rw = target_pos_rw.clone();
            let target_velocity_w = target_velocity_r.clone();
            let ai_enabled_r = ai_enabled.clone();
            std::thread::spawn(move || {
                run_pos_to_target_vel(
                    hl_game_state,
                    phys_render_r,
                    target_pos_rw,
                    target_velocity_w,
                    ai_enabled_r,
                );
            });
        }
        {
            let target_velocity_r = target_velocity_r.clone();
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
        }
        std::thread::spawn(move || {
            run_high_level(hl_game_state, target_pos_rw);
        });
        let (network_command_send, network_recv) = tokio::sync::mpsc::channel(10);
        start_network_thread(network_recv, sensors.clone());

        let pacbot_pos = phys_render.read().unwrap().pacbot_pos;

        Self {
            mode: AppMode::Recording,
            ai_enable: ai_enabled.clone(),

            grid_widget: GridWidget {},
            game_widget: GameWidget {
                state: pacman_render.clone(),
            },
            stopwatch_widget,
            ai_widget: AiWidget { ai_enabled },
            sensors_widget: PacbotSensorsWidget::new(sensors),

            selected_grid: StandardGrid::Pacman,
            grid: StandardGrid::Pacman.compute_grid(),
            pointer_pos: "".to_string(),
            background_color: Color32::BLACK,

            robot: Robot::default(),
            target_velocity,
            target_pos,
            phys_restart_send,
            phys_render,

            pacman_render,
            world_to_screen: None,

            replay_manager: Self::new_replay_manager(
                filename,
                StandardGrid::Pacman,
                GameEngine::default(),
                pacbot_pos,
            ),
            pacman_state_notify_recv,
            replay_pacman: Isometry2::default(),
            save_pacbot_location: false,

            network_command_send,

            pf_stopwatch,
            physics_stopwatch,
        }
    }
}

impl TabViewer {
    fn grid_ui(&mut self, ui: &mut Ui) {
        let rect = ui.max_rect();
        let (src_p1, src_p2) = self.selected_grid.get_soft_boundaries();

        let world_to_screen = Transform::new_letterboxed(
            src_p1,
            src_p2,
            Pos2::new(rect.top(), rect.left()),
            Pos2::new(rect.bottom(), rect.right()),
        );
        self.world_to_screen = Some(world_to_screen);
        let painter = ui.painter_at(rect);

        self.draw_grid(&world_to_screen, &painter);

        if self.selected_grid == StandardGrid::Pacman {
            self.draw_pacman_state(&world_to_screen, &painter);
        }

        self.draw_simulation(&world_to_screen, &painter);
    }
}

#[derive(Clone, Debug)]
pub enum PacbotWidgetStatus {
    Ok,
    Warn(String),
    Error(String),
    NotApplicable,
}

pub trait PacbotWidget {
    fn update(&mut self) {}
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

/// Launches the GUI application. Blocks until the application has quit.
pub fn run_gui() {
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "PacBot simulation",
        native_options,
        Box::new(|cc| {
            let mut fonts = egui::FontDefinitions::default();
            egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);

            cc.egui_ctx.set_fonts(fonts);
            Box::<App>::default()
        }),
    )
    .expect("eframe::run_native error");
}

/// Indicates the current meta-state of the app
#[derive(Clone, Copy, PartialEq, Eq)]
enum AppMode {
    /// Using a game server with physics engine and recording the results to file
    Recording,
    /// Playing information back from a file; no game server but physics should still run
    Playback,
}

struct App {
    tree: DockState<Tab>,
    tab_viewer: TabViewer,
}

fn pretty_print_time_now() -> String {
    let date = chrono::Local::now();
    date.format("%Y_%m_%d__%H_%M_%S").to_string()
}

impl Default for App {
    fn default() -> Self {
        Self {
            tree: DockState::new(vec![Tab::Grid]),
            tab_viewer: TabViewer::default(),
        }
    }
}

impl App {
    fn update_target_velocity(&mut self, ctx: &egui::Context) {
        let ai_enabled = *self.tab_viewer.ai_enable.read().unwrap().deref();
        if !ai_enabled {
            let mut target_velocity = self.tab_viewer.target_velocity.write().unwrap();
            target_velocity.0.x = 0.0;
            target_velocity.0.y = 0.0;
            target_velocity.1 = 0.0;
            ctx.input(|i| {
                let target_speed = if i.modifiers.shift { 2.0 } else { 0.8 };
                if i.key_down(Key::S) {
                    target_velocity.0.x = target_speed;
                }
                if i.key_down(Key::W) {
                    target_velocity.0.x = -target_speed;
                }
                if i.key_down(Key::A) {
                    target_velocity.0.y = -target_speed;
                }
                if i.key_down(Key::D) {
                    target_velocity.0.y = target_speed;
                }
                if i.key_down(Key::E) {
                    target_velocity.1 = -target_speed;
                }
                if i.key_down(Key::Q) {
                    target_velocity.1 = target_speed;
                }
            });
        }
    }

    fn add_grid_variants(&mut self, ui: &mut Ui) {
        egui::ComboBox::from_label("")
            .selected_text(format!("{:?}", self.tab_viewer.selected_grid))
            .show_ui(ui, |ui| {
                StandardGrid::get_all().iter().for_each(|grid| {
                    if ui
                        .selectable_value(
                            &mut self.tab_viewer.selected_grid,
                            *grid,
                            format!("{:?}", grid),
                        )
                        .clicked()
                    {
                        self.tab_viewer
                            .pacman_render
                            .write()
                            .unwrap()
                            .pacman_state
                            .pause();
                        self.tab_viewer.grid = grid.compute_grid();
                        self.tab_viewer.phys_render.write().unwrap().pacbot_pos =
                            self.tab_viewer.selected_grid.get_default_pacbot_isometry();
                        self.tab_viewer
                            .phys_restart_send
                            .send((
                                self.tab_viewer.selected_grid,
                                Robot::default(),
                                self.tab_viewer.selected_grid.get_default_pacbot_isometry(),
                            ))
                            .unwrap();
                        self.tab_viewer.reset_replay();
                    }
                });
            });
    }

    fn draw_widget_icons(&mut self, ui: &mut Ui) {
        let widgets: Vec<Box<&mut dyn PacbotWidget>> = vec![
            Box::new(&mut self.tab_viewer.grid_widget),
            Box::new(&mut self.tab_viewer.game_widget),
            Box::new(&mut self.tab_viewer.stopwatch_widget),
            Box::new(&mut self.tab_viewer.ai_widget),
            Box::new(&mut self.tab_viewer.sensors_widget),
        ];
        for mut widget in widgets {
            widget.deref_mut().update();
            let mut button = ui.add(egui::Button::new(widget.button_text()).fill(
                match widget.overall_status() {
                    PacbotWidgetStatus::Ok => TRANSLUCENT_GREEN_COLOR,
                    PacbotWidgetStatus::Warn(_) => TRANSLUCENT_YELLOW_COLOR,
                    PacbotWidgetStatus::Error(_) => TRANSLUCENT_RED_COLOR,
                    PacbotWidgetStatus::NotApplicable => Color32::TRANSPARENT,
                },
            ));
            button = button.on_hover_ui(|ui| {
                ui.label(widget.display_name());
                for msg in widget.messages() {
                    ui.label(
                        RichText::new(format!(
                            "{} {}",
                            match msg.1 {
                                PacbotWidgetStatus::Ok => regular::CHECK,
                                PacbotWidgetStatus::Warn(_) => regular::WARNING,
                                PacbotWidgetStatus::Error(_) => regular::X,
                                PacbotWidgetStatus::NotApplicable => regular::CHECK,
                            },
                            msg.0.to_owned()
                        ))
                        .color(match msg.1 {
                            PacbotWidgetStatus::Ok => Color32::GREEN,
                            PacbotWidgetStatus::Warn(_) => Color32::YELLOW,
                            PacbotWidgetStatus::Error(_) => Color32::RED,
                            PacbotWidgetStatus::NotApplicable => Color32::GREEN,
                        }),
                    );
                }
            });
            if button.clicked() {
                match widget.display_name() {
                    "Game (Click to Reset)" => {
                        self.tab_viewer.pacman_render.write().unwrap().pacman_state =
                            GameEngine::default()
                    }
                    "AI" => {
                        let val = *self.tab_viewer.ai_enable.read().unwrap();
                        *self.tab_viewer.ai_enable.write().unwrap() = !val;
                    }
                    "Sensors" => {}
                    _ => self.tree.push_to_focused_leaf(widget.tab()),
                }
            }
        }
    }
}

fn draw_stopwatch(stopwatch: &Stopwatch, ui: &mut Ui, id: String) {
    ui.label(format!(
        "Total: {:.2}",
        stopwatch.average_process_time() * 1000.0
    ));
    ui.separator();
    egui::Grid::new(id)
        .num_columns(2)
        .striped(true)
        .show(ui, |ui| {
            let segment_times = stopwatch.average_segment_times();
            for (name, time) in segment_times {
                ui.label(name);
                ui.label(format!("{:.2}", time * 1000.0));
                ui.end_row();
            }
        });
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if let Some(world_to_screen) = &self.tab_viewer.world_to_screen {
            self.tab_viewer.pointer_pos = match ctx.pointer_latest_pos() {
                None => "".to_string(),
                Some(pos) => {
                    let pos = world_to_screen.inverse().map_point(pos);
                    format!("({:.1}, {:.1})", pos.x, pos.y)
                }
            };
        }
        self.tab_viewer.background_color = ctx.style().visuals.panel_fill;

        self.update_target_velocity(ctx);

        self.tab_viewer
            .update_replay_manager()
            .expect("Error updating replay manager");

        egui::TopBottomPanel::top("menu").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.with_layout(egui::Layout::left_to_right(Align::Center), |ui| {
                    self.add_grid_variants(ui);
                    egui::menu::bar(ui, |ui| {
                        ui.menu_button("Replay", |ui| {
                            if ui.button("Save").clicked() {
                                self.tab_viewer
                                    .save_replay()
                                    .expect("Failed to save replay!");
                            }
                            if ui.button("Load").clicked() {
                                self.tab_viewer
                                    .load_replay()
                                    .expect("Failed to load replay!");
                            }
                            if ui
                                .add(
                                    egui::Button::new("Save Pacbot Location")
                                        .selected(self.tab_viewer.save_pacbot_location),
                                )
                                .clicked()
                            {
                                self.tab_viewer.save_pacbot_location =
                                    !self.tab_viewer.save_pacbot_location;
                            }
                        });

                        self.draw_widget_icons(ui);
                    })
                });
                ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                    ui.label(&self.tab_viewer.pointer_pos);
                });
            });
        });
        if self.tab_viewer.selected_grid == StandardGrid::Pacman {
            egui::TopBottomPanel::bottom("playback_controls")
                .frame(
                    Frame::none()
                        .fill(ctx.style().visuals.panel_fill)
                        .inner_margin(5.0),
                )
                .show(ctx, |ui| {
                    self.tab_viewer.draw_replay_ui(ctx, ui);
                });
        }
        DockArea::new(&mut self.tree)
            .style(Style::from_egui(ctx.style().as_ref()))
            .show(ctx, &mut self.tab_viewer);

        ctx.request_repaint();
    }
}

#[derive(Copy, Clone, Debug)]
pub struct GridWidget {}

impl PacbotWidget for GridWidget {
    fn display_name(&self) -> &'static str {
        "Grid"
    }

    fn button_text(&self) -> RichText {
        RichText::new(format!("{}", regular::GRID_FOUR,))
    }

    fn tab(&self) -> Tab {
        Tab::Grid
    }
}

#[derive(Clone, Debug)]
pub struct AiWidget {
    pub ai_enabled: Arc<RwLock<bool>>,
}

impl PacbotWidget for AiWidget {
    fn display_name(&self) -> &'static str {
        "AI"
    }

    fn button_text(&self) -> RichText {
        RichText::new(format!("{}", regular::BRAIN,))
    }

    fn overall_status(&self) -> &PacbotWidgetStatus {
        if *self.ai_enabled.read().unwrap() {
            &PacbotWidgetStatus::Ok
        } else {
            &PacbotWidgetStatus::NotApplicable
        }
    }
}

#[derive(Clone, Debug)]
pub struct PacbotSensorsWidget {
    pub sensors: Arc<RwLock<(bool, [u8; 8], [i64; 3], Instant)>>,

    pub overall_status: PacbotWidgetStatus,
    pub messages: Vec<(String, PacbotWidgetStatus)>,
}

impl PacbotSensorsWidget {
    pub fn new(sensors: Arc<RwLock<(bool, [u8; 8], [i64; 3], Instant)>>) -> Self {
        Self {
            sensors,

            overall_status: PacbotWidgetStatus::Ok,
            messages: vec![],
        }
    }
}

impl PacbotWidget for PacbotSensorsWidget {
    fn update(&mut self) {
        let sensors = self.sensors.read().unwrap();

        self.messages = vec![];
        self.overall_status = PacbotWidgetStatus::Ok;

        if sensors.0 {
            self.messages
                .push(("Sensors enabled".to_string(), PacbotWidgetStatus::Ok));
        } else {
            self.messages.push((
                "Sensors disabled".to_string(),
                PacbotWidgetStatus::Warn("".to_string()),
            ));
        }

        if sensors.3.elapsed() > Duration::from_secs(1) {
            self.messages.push((
                format!("Last data age: {:?}", sensors.3.elapsed()),
                PacbotWidgetStatus::Error("".to_string()),
            ));
            self.overall_status =
                PacbotWidgetStatus::Error(format!("Last data age: {:?}", sensors.3.elapsed()));
        } else {
            for i in 0..8 {
                if sensors.1[i] == 0 {
                    self.messages.push((
                        format!("Sensor {i} unresponsive"),
                        PacbotWidgetStatus::Error("".to_string()),
                    ));
                    self.overall_status =
                        PacbotWidgetStatus::Error(format!("Sensor {i} unresponsive"));
                }
                self.messages.push((
                    format!("{i} => {}", sensors.1[i]),
                    match sensors.1[i] {
                        0 => PacbotWidgetStatus::Error("".to_string()),
                        255 => PacbotWidgetStatus::Warn("".to_string()),
                        _ => PacbotWidgetStatus::Ok,
                    },
                ))
            }
        }
    }

    fn display_name(&self) -> &'static str {
        "Sensors"
    }

    fn button_text(&self) -> RichText {
        RichText::new(format!("{}", regular::RULER,))
    }

    fn overall_status(&self) -> &PacbotWidgetStatus {
        &self.overall_status
    }

    fn messages(&self) -> &[(String, PacbotWidgetStatus)] {
        &self.messages
    }
}
