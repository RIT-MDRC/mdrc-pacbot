//! Top-level GUI elements and functionality.

mod colors;
pub mod game;
pub(crate) mod physics;
pub mod replay_manager;
mod settings;
mod stopwatch;
pub mod transforms;
pub mod utils;

use crate::grid::{ComputedGrid, IntLocation};
use bevy::app::{App, Startup};
use bevy::prelude::{Plugin, Update};
use bevy_ecs::prelude::*;
use bevy_egui::EguiContexts;
use eframe::egui;
use eframe::egui::{Align, Color32, Frame, Key, Pos2, RichText, Ui, WidgetText};
use egui_dock::{DockArea, DockState, NodeIndex, Style};
use egui_phosphor::regular;
use pacbot_rs::game_engine::GameEngine;
use std::ops::Deref;
use std::time::Duration;

use crate::grid::standard_grids::StandardGrid;
use crate::gui::colors::{
    TRANSLUCENT_GREEN_COLOR, TRANSLUCENT_RED_COLOR, TRANSLUCENT_YELLOW_COLOR,
};
use crate::gui::game::GameWidget;
use crate::gui::settings::PacbotSettingsWidget;
use crate::gui::stopwatch::StopwatchWidget;
use crate::high_level::AiStopwatch;
use crate::network::{GSConnState, GameServerConn, PacbotSensors, PacbotSensorsRecvTime};
use crate::pathing::{TargetPath, TargetVelocity};
use crate::physics::{LightPhysicsInfo, ParticleFilterStopwatch, PhysicsStopwatch};
use crate::replay_manager::{replay_playback, update_replay_manager_system, ReplayManager};
use crate::util::stopwatch::Stopwatch;
use crate::{PacmanGameState, ScheduleStopwatch, StandardGridResource, UserSettings};

use self::transforms::Transform;

/// Builds resources and systems related to the GUI
pub struct GuiPlugin;

impl Plugin for GuiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GuiApp>()
            .insert_resource(GuiStopwatch(Stopwatch::new(
                10,
                "GUI".to_string(),
                1.0,
                2.0,
            )))
            .add_systems(Startup, font_setup)
            .add_systems(
                Update,
                (ui_system, update_replay_manager_system, replay_playback),
            );
    }
}

/// Tracks the performance of GUI rendering
#[derive(Resource)]
pub struct GuiStopwatch(pub Stopwatch);

/// Adds Phosphor to Egui for icons
fn font_setup(mut contexts: EguiContexts) {
    let mut fonts = egui::FontDefinitions::default();
    egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);

    contexts.ctx_mut().set_fonts(fonts);
}

/// Updates Egui and any actions from the user
pub fn ui_system(
    mut contexts: EguiContexts,
    mut app: Local<GuiApp>,
    world_to_screen: Local<Option<Transform>>,
    pacman_state: ResMut<PacmanGameState>,
    phys_info: ResMut<LightPhysicsInfo>,
    selected_grid: ResMut<StandardGridResource>,
    grid: ResMut<ComputedGrid>,
    replay_manager: ResMut<ReplayManager>,
    settings: ResMut<UserSettings>,
    target_velocity: ResMut<TargetVelocity>,
    target_path: Res<TargetPath>,
    stopwatches: (
        ResMut<ParticleFilterStopwatch>,
        ResMut<PhysicsStopwatch>,
        ResMut<GuiStopwatch>,
        ResMut<ScheduleStopwatch>,
        ResMut<AiStopwatch>,
    ),
    sensors: (Res<PacbotSensors>, Res<PacbotSensorsRecvTime>),
    mut gs_conn: NonSendMut<GameServerConn>,
) {
    let ctx = contexts.ctx_mut();

    let mut tab_viewer = TabViewer {
        pointer_pos: ctx.pointer_latest_pos(),
        background_color: ctx.style().visuals.panel_fill,

        pacman_state,
        phys_info,
        world_to_screen,
        replay_manager,
        settings,
        target_velocity,
        target_path,
        grid,
        selected_grid,
        pf_stopwatch: stopwatches.0,
        physics_stopwatch: stopwatches.1,
        gui_stopwatch: stopwatches.2,
        schedule_stopwatch: stopwatches.3,
        ai_stopwatch: stopwatches.4,
        sensors: sensors.0,
        sensors_recv_time: sensors.1,

        connected: gs_conn.client.is_connected(),
        reconnect: false,
    };

    tab_viewer.gui_stopwatch.0.start();

    app.update_target_velocity(&ctx, &mut tab_viewer);

    tab_viewer
        .gui_stopwatch
        .0
        .mark_segment("Update target velocity");

    app.update(&ctx, &mut tab_viewer);

    if tab_viewer.reconnect {
        gs_conn.client = GSConnState::Connecting;
    }
}

/// Options for different kinds of tabs
#[derive(Copy, Clone)]
pub enum Tab {
    /// Main game grid
    Grid,
    /// Detailed timings
    Stopwatch,
    /// User settings
    Settings,
    /// For widgets that don't have corresponding tabs
    Unknown,
}

struct TabViewer<'a> {
    pointer_pos: Option<Pos2>,
    background_color: Color32,

    pacman_state: ResMut<'a, PacmanGameState>,
    phys_info: ResMut<'a, LightPhysicsInfo>,
    world_to_screen: Local<'a, Option<Transform>>,
    replay_manager: ResMut<'a, ReplayManager>,
    settings: ResMut<'a, UserSettings>,
    target_velocity: ResMut<'a, TargetVelocity>,
    target_path: Res<'a, TargetPath>,
    grid: ResMut<'a, ComputedGrid>,
    selected_grid: ResMut<'a, StandardGridResource>,
    sensors: Res<'a, PacbotSensors>,
    sensors_recv_time: Res<'a, PacbotSensorsRecvTime>,

    pf_stopwatch: ResMut<'a, ParticleFilterStopwatch>,
    physics_stopwatch: ResMut<'a, PhysicsStopwatch>,
    gui_stopwatch: ResMut<'a, GuiStopwatch>,
    schedule_stopwatch: ResMut<'a, ScheduleStopwatch>,
    ai_stopwatch: ResMut<'a, AiStopwatch>,

    reconnect: bool,
    connected: bool,
}

impl<'a> egui_dock::TabViewer for TabViewer<'a> {
    type Tab = Tab;

    fn title(&mut self, tab: &mut Self::Tab) -> WidgetText {
        match tab {
            Tab::Grid => WidgetText::from("Main Grid"),
            Tab::Stopwatch => WidgetText::from("Stopwatch"),
            Tab::Settings => WidgetText::from("Settings"),
            _ => panic!("Widget did not declare a tab!"),
        }
    }

    fn ui(&mut self, ui: &mut Ui, tab: &mut Self::Tab) {
        match tab {
            Tab::Grid => self.grid_ui(ui),
            Tab::Stopwatch => {
                ui.label("Particle Filter");
                draw_stopwatch(&self.pf_stopwatch.0, ui, "pf_sw".to_string());
                ui.separator();
                ui.label("Physics");
                draw_stopwatch(&self.physics_stopwatch.0, ui, "ph_sw".to_string());
                ui.separator();
                ui.label("GUI");
                draw_stopwatch(&self.gui_stopwatch.0, ui, "ui_sw".to_string());
                ui.separator();
                ui.label("AI");
                draw_stopwatch(&self.ai_stopwatch.0, ui, "ai_sw".to_string());
                ui.separator();
                ui.label("Schedule");
                draw_stopwatch(&self.schedule_stopwatch.0, ui, "sch_sw".to_string());
            }
            Tab::Settings => self.draw_settings(ui),
            _ => panic!("Widget did not declare a tab!"),
        }
    }
}

impl<'a> TabViewer<'a> {
    fn grid_ui(&mut self, ui: &mut Ui) {
        let rect = ui.max_rect();
        let (src_p1, src_p2) = self.selected_grid.0.get_soft_boundaries();

        let world_to_screen = Transform::new_letterboxed(
            src_p1,
            src_p2,
            Pos2::new(rect.top(), rect.left()),
            Pos2::new(rect.bottom(), rect.right()),
        );
        *self.world_to_screen = Some(world_to_screen);
        let painter = ui.painter_at(rect);

        self.draw_grid(&world_to_screen, &painter);

        if self.selected_grid.0 == StandardGrid::Pacman {
            self.draw_pacman_state(&world_to_screen, &painter);
        }

        self.draw_simulation(&painter);
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
    fn update(&mut self, _tab_viewer: &TabViewer) {}
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

/// Indicates the current meta-state of the app
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum AppMode {
    /// Using a game server with physics engine and recording the results to file
    Recording,
    /// Playing information back from a file; no game server but physics should still run
    Playback,
}

/// Holds information about the tabs and widgets over time
#[derive(Resource)]
pub struct GuiApp {
    /// Tab configuration
    tree: DockState<Tab>,

    /// grid
    grid_widget: GridWidget,
    /// game
    game_widget: GameWidget,
    /// timings
    stopwatch_widget: StopwatchWidget,
    /// ai
    ai_widget: AiWidget,
    /// sensors
    sensors_widget: PacbotSensorsWidget,
    /// settings
    settings_widget: PacbotSettingsWidget,
}

impl Default for GuiApp {
    fn default() -> Self {
        let mut dock_state = DockState::new(vec![Tab::Grid]);
        let surface = dock_state.main_surface_mut();
        surface.split_right(NodeIndex::root(), 0.75, vec![Tab::Settings]);

        Self {
            tree: dock_state,

            grid_widget: GridWidget::default(),
            game_widget: GameWidget::default(),
            stopwatch_widget: StopwatchWidget::new(),
            ai_widget: AiWidget::default(),
            sensors_widget: PacbotSensorsWidget::new(),
            settings_widget: PacbotSettingsWidget::default(),
        }
    }
}

impl GuiApp {
    fn update_target_velocity(&mut self, ctx: &egui::Context, tab_viewer: &mut TabViewer) {
        let ai_enabled = tab_viewer.settings.enable_ai;
        if !ai_enabled {
            tab_viewer.target_velocity.0.x = 0.0;
            tab_viewer.target_velocity.0.y = 0.0;
            tab_viewer.target_velocity.1 = 0.0;
            ctx.input(|i| {
                let target_speed = if i.modifiers.shift { 4.0 } else { 10.0 };
                if i.key_down(Key::S) {
                    tab_viewer.target_velocity.0.x = target_speed;
                }
                if i.key_down(Key::W) {
                    tab_viewer.target_velocity.0.x = -target_speed;
                }
                if i.key_down(Key::A) {
                    tab_viewer.target_velocity.0.y = -target_speed;
                }
                if i.key_down(Key::D) {
                    tab_viewer.target_velocity.0.y = target_speed;
                }
                if i.key_down(Key::E) {
                    tab_viewer.target_velocity.1 = -1.0;
                }
                if i.key_down(Key::Q) {
                    tab_viewer.target_velocity.1 = 1.0;
                }
            });
        }
    }

    fn add_grid_variants(
        &mut self,
        ui: &mut Ui,
        pacman_state: &mut ResMut<PacmanGameState>,
        phys_info: &LightPhysicsInfo,
        replay_manager: &mut ReplayManager,
        standard_grid: &mut StandardGrid,
        computed_grid: &mut ComputedGrid,
    ) {
        egui::ComboBox::from_label("")
            .selected_text(format!("{:?}", standard_grid))
            .show_ui(ui, |ui| {
                StandardGrid::get_all().iter().for_each(|grid| {
                    if ui
                        .selectable_value(standard_grid, *grid, format!("{:?}", grid))
                        .clicked()
                    {
                        pacman_state.0.pause();
                        *computed_grid = grid.compute_grid();
                        replay_manager.reset_replay(
                            *grid,
                            &pacman_state.0,
                            phys_info
                                .real_pos
                                .unwrap_or(grid.get_default_pacbot_isometry()),
                        );
                    }
                });
            });
    }

    fn draw_widget_icons(&mut self, ui: &mut Ui, tab_viewer: &mut TabViewer) {
        let mut widgets: [Box<&mut dyn PacbotWidget>; 6] = [
            Box::new(&mut self.grid_widget),
            Box::new(&mut self.game_widget),
            Box::new(&mut self.stopwatch_widget),
            Box::new(&mut self.ai_widget),
            Box::new(&mut self.sensors_widget),
            Box::new(&mut self.settings_widget),
        ];
        for widget in &mut widgets {
            widget.update(tab_viewer);
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
                    "Game (Click to Reset)" => tab_viewer.pacman_state.0 = GameEngine::default(),
                    "AI" => {
                        tab_viewer.settings.enable_ai = !tab_viewer.settings.enable_ai;
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

impl GuiApp {
    fn update(&mut self, ctx: &egui::Context, tab_viewer: &mut TabViewer) {
        egui::TopBottomPanel::top("menu").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.with_layout(egui::Layout::left_to_right(Align::Center), |ui| {
                    self.add_grid_variants(
                        ui,
                        &mut tab_viewer.pacman_state,
                        &tab_viewer.phys_info,
                        &mut tab_viewer.replay_manager,
                        &mut tab_viewer.selected_grid.0,
                        &mut tab_viewer.grid,
                    );
                    egui::menu::bar(ui, |ui| {
                        ui.menu_button("Replay", |ui| {
                            if ui.button("Save").clicked() {
                                tab_viewer.save_replay().expect("Failed to save replay!");
                            }
                            if ui.button("Load").clicked() {
                                tab_viewer.load_replay().expect("Failed to load replay!");
                            }
                            if ui
                                .add(
                                    egui::Button::new("Save Pacbot Location")
                                        .selected(tab_viewer.settings.replay_save_location),
                                )
                                .clicked()
                            {
                                tab_viewer.settings.replay_save_location =
                                    !tab_viewer.settings.replay_save_location;
                            }
                        });

                        self.draw_widget_icons(ui, tab_viewer);
                    })
                });
                ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                    if let Some(world_to_screen) = tab_viewer.world_to_screen.deref() {
                        ui.label(
                            &(match tab_viewer.pointer_pos {
                                None => "".to_string(),
                                Some(pos) => {
                                    let pos = world_to_screen.inverse().map_point(pos);
                                    format!("({:.1}, {:.1})", pos.x, pos.y)
                                }
                            }),
                        );
                        if ctx.input(|i| i.pointer.primary_clicked()) {
                            if let Some(pos) = tab_viewer.pointer_pos {
                                let pos = world_to_screen.inverse().map_point(pos);
                                let int_pos = IntLocation {
                                    row: pos.x.round() as i8,
                                    col: pos.y.round() as i8,
                                };
                                if !tab_viewer.grid.wall_at(&int_pos) {
                                    tab_viewer.settings.kidnap_position = Some(int_pos);
                                }
                            }
                        }
                    }
                });
            });
        });

        tab_viewer.gui_stopwatch.0.mark_segment("Draw top bar");

        if tab_viewer.selected_grid.0 == StandardGrid::Pacman {
            egui::TopBottomPanel::bottom("playback_controls")
                .frame(
                    Frame::none()
                        .fill(ctx.style().visuals.panel_fill)
                        .inner_margin(5.0),
                )
                .show(ctx, |ui| {
                    tab_viewer.draw_replay_ui(ctx, ui);
                });
        }

        tab_viewer.gui_stopwatch.0.mark_segment("Draw replay UI");

        DockArea::new(&mut self.tree)
            .style(Style::from_egui(ctx.style().as_ref()))
            .show(ctx, tab_viewer);

        tab_viewer.gui_stopwatch.0.mark_segment("Draw tabs");

        ctx.request_repaint();
    }
}

#[derive(Copy, Clone, Debug, Default)]
struct GridWidget {}

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

/// Shows information about the AI
#[derive(Clone, Debug, Default)]
pub struct AiWidget {
    ai_enabled: bool,
}

impl PacbotWidget for AiWidget {
    fn update(&mut self, tab_viewer: &TabViewer) {
        self.ai_enabled = tab_viewer.settings.enable_ai;
    }

    fn display_name(&self) -> &'static str {
        "AI"
    }

    fn button_text(&self) -> RichText {
        RichText::new(format!("{}", regular::BRAIN,))
    }

    fn overall_status(&self) -> &PacbotWidgetStatus {
        if self.ai_enabled {
            &PacbotWidgetStatus::Ok
        } else {
            &PacbotWidgetStatus::NotApplicable
        }
    }
}

/// Displays information about pacbot sensors
#[derive(Clone, Debug)]
pub struct PacbotSensorsWidget {
    /// Status of the connection/sensors
    pub overall_status: PacbotWidgetStatus,
    /// Messages for each sensor
    pub messages: Vec<(String, PacbotWidgetStatus)>,
}

impl PacbotSensorsWidget {
    /// Make a new PacbotSensorsWidget
    pub fn new() -> Self {
        Self {
            overall_status: PacbotWidgetStatus::Ok,
            messages: vec![],
        }
    }
}

impl PacbotWidget for PacbotSensorsWidget {
    fn update(&mut self, tab_viewer: &TabViewer) {
        let sensors = &tab_viewer.sensors;

        self.messages = vec![];
        self.overall_status = PacbotWidgetStatus::Ok;

        if let Some(t) = tab_viewer.sensors_recv_time.0 {
            if t.elapsed() > Duration::from_secs(1) {
                self.messages.push((
                    format!("Last data age: {:?}", t.elapsed()),
                    PacbotWidgetStatus::Error("".to_string()),
                ));
                self.overall_status =
                    PacbotWidgetStatus::Error(format!("Last data age: {:?}", t.elapsed()));
            } else {
                for i in 0..8 {
                    if sensors.distance_sensors[i] == 0 {
                        self.messages.push((
                            format!("Sensor {i} unresponsive"),
                            PacbotWidgetStatus::Error("".to_string()),
                        ));
                        self.overall_status =
                            PacbotWidgetStatus::Error(format!("Sensor {i} unresponsive"));
                    }
                    self.messages.push((
                        format!("{i} => {}", sensors.distance_sensors[i]),
                        match sensors.distance_sensors[i] {
                            0 => PacbotWidgetStatus::Error("".to_string()),
                            255 => PacbotWidgetStatus::Warn("".to_string()),
                            _ => PacbotWidgetStatus::Ok,
                        },
                    ))
                }
                for i in 0..3 {
                    self.messages.push((
                        format!("Encoder {i}: {}", sensors.encoders[i]),
                        PacbotWidgetStatus::Ok,
                    ));
                }
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
