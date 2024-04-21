//! Top-level GUI elements and functionality.

mod colors;
pub mod game;
pub(crate) mod physics;
pub mod replay_manager;
mod robot;
mod settings;
mod stopwatch;
pub mod transforms;
pub mod utils;

use crate::grid::{ComputedGrid, IntLocation};
use bevy::app::{App, Startup};
use bevy::input::Input;
use bevy::prelude::{
    Axis, GamepadAxis, GamepadAxisType, GamepadButton, GamepadButtonType, Gamepads, Plugin, Update,
};
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
use crate::gui::robot::RobotWidget;
use crate::gui::settings::PacbotSettingsWidget;
use crate::gui::stopwatch::StopwatchWidget;
use crate::high_level::AiStopwatch;
use crate::network::{
    GSConnState, GameServerConn, LastMotorCommands, MotorRequest, PacbotSensors,
    PacbotSensorsRecvTime,
};
use crate::pathing::{TargetPath, TargetVelocity};
use crate::physics::{
    LightPhysicsInfo, PacbotSimulation, ParticleFilterStopwatch, PhysicsStopwatch,
};
use crate::replay_manager::{replay_playback, update_replay_manager_system, ReplayManager};
use crate::robot::Robot;
use crate::util::stopwatch::Stopwatch;
use crate::{
    HighLevelStrategy, PacmanGameState, ScheduleStopwatch, StandardGridResource, UserSettings,
};

use self::transforms::Transform;

/// Builds resources and systems related to the GUI
pub struct GuiPlugin;

impl Plugin for GuiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GuiApp>()
            .insert_resource(GuiStopwatch(Stopwatch::new(
                10,
                "GUI".to_string(),
                3.0,
                4.0,
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
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn ui_system(
    mut contexts: EguiContexts,
    mut app: Local<GuiApp>,
    world_to_screen: Local<Option<Transform>>,
    pacman_state: ResMut<PacmanGameState>,
    simulation: (ResMut<PacbotSimulation>, ResMut<LightPhysicsInfo>),
    selected_grid: ResMut<StandardGridResource>,
    grid: ResMut<ComputedGrid>,
    replay_manager: ResMut<ReplayManager>,
    settings: ResMut<UserSettings>,
    target_velocity: ResMut<TargetVelocity>,
    target_path: Res<TargetPath>,
    gamepad: (
        Res<Gamepads>,
        Res<Axis<GamepadAxis>>,
        Res<Input<GamepadButton>>,
    ),
    stopwatches: (
        ResMut<ParticleFilterStopwatch>,
        ResMut<PhysicsStopwatch>,
        ResMut<GuiStopwatch>,
        ResMut<ScheduleStopwatch>,
        ResMut<AiStopwatch>,
    ),
    sensors: (Res<PacbotSensors>, Res<PacbotSensorsRecvTime>),
    last_motor_commands: Res<LastMotorCommands>,
    mut gs_conn: NonSendMut<GameServerConn>,
) {
    let ctx = contexts.ctx_mut();

    let mut tab_viewer = TabViewer {
        pointer_pos: ctx.pointer_latest_pos(),
        background_color: ctx.style().visuals.panel_fill,

        gamepad: gamepad.0,
        gamepad_input: gamepad.1,
        gamepad_buttons: gamepad.2,
        pacman_state,
        simulation: simulation.0,
        phys_info: simulation.1,
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
        last_motor_commands,

        connected: gs_conn.client.is_connected(),
        reconnect: false,
    };

    tab_viewer.gui_stopwatch.0.start();

    app.update_target_velocity(ctx, &mut tab_viewer);

    tab_viewer
        .gui_stopwatch
        .0
        .mark_segment("Update target velocity");

    app.update(ctx, &mut tab_viewer);

    if tab_viewer.reconnect {
        gs_conn.client = GSConnState::Connecting;
    }

    if tab_viewer.settings.go_server_address.is_none() {
        gs_conn.client = GSConnState::Disconnected;
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
    /// Robot view
    Robot,
    /// For widgets that don't have corresponding tabs
    Unknown,
}

struct TabViewer<'a> {
    pointer_pos: Option<Pos2>,
    background_color: Color32,

    gamepad: Res<'a, Gamepads>,
    gamepad_input: Res<'a, Axis<GamepadAxis>>,
    gamepad_buttons: Res<'a, Input<GamepadButton>>,
    pacman_state: ResMut<'a, PacmanGameState>,
    simulation: ResMut<'a, PacbotSimulation>,
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
    last_motor_commands: Res<'a, LastMotorCommands>,

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
            Tab::Robot => WidgetText::from("Robot"),
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
            Tab::Robot => self.draw_robot(ui),
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
    /// robot
    robot_widget: RobotWidget,
}

impl Default for GuiApp {
    fn default() -> Self {
        let mut dock_state = DockState::new(vec![Tab::Grid, Tab::Robot]);
        let surface = dock_state.main_surface_mut();
        surface.split_right(NodeIndex::root(), 0.75, vec![Tab::Settings]);

        Self {
            tree: dock_state,

            grid_widget: GridWidget::default(),
            game_widget: GameWidget::default(),
            stopwatch_widget: StopwatchWidget::new(),
            ai_widget: AiWidget::default(),
            sensors_widget: PacbotSensorsWidget::default(),
            settings_widget: PacbotSettingsWidget,
            robot_widget: RobotWidget::default(),
        }
    }
}

impl GuiApp {
    fn update_target_velocity(&mut self, ctx: &egui::Context, tab_viewer: &mut TabViewer) {
        let high_level_strategy = tab_viewer.settings.high_level_strategy;
        if high_level_strategy == HighLevelStrategy::Manual && tab_viewer.target_path.0.is_empty() {
            tab_viewer.target_velocity.0.x = 0.0;
            tab_viewer.target_velocity.0.y = 0.0;
            tab_viewer.target_velocity.1 = 0.0;
            ctx.input(|i| {
                let target_speed = if i.modifiers.shift {
                    4.0
                } else {
                    tab_viewer.settings.manual_speed
                };
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
                    tab_viewer.target_velocity.1 = -tab_viewer.settings.manual_rotate_speed;
                }
                if i.key_down(Key::Q) {
                    tab_viewer.target_velocity.1 = tab_viewer.settings.manual_rotate_speed;
                }

                if let Some(gamepad) = tab_viewer.gamepad.iter().next() {
                    let left_stick_x = tab_viewer
                        .gamepad_input
                        .get(GamepadAxis::new(gamepad, GamepadAxisType::LeftStickX))
                        .unwrap();
                    let left_stick_y = tab_viewer
                        .gamepad_input
                        .get(GamepadAxis::new(gamepad, GamepadAxisType::LeftStickY))
                        .unwrap();

                    let left_trigger = tab_viewer
                        .gamepad_buttons
                        .pressed(GamepadButton::new(gamepad, GamepadButtonType::LeftTrigger));
                    let right_trigger = tab_viewer
                        .gamepad_buttons
                        .pressed(GamepadButton::new(gamepad, GamepadButtonType::RightTrigger));

                    let speed_multiplier = match (left_trigger, right_trigger) {
                        (true, true) | (false, false) => 1.0,
                        (true, false) => 0.5,
                        (false, true) => 2.0,
                    };

                    if left_stick_x.abs() > 0.1 {
                        tab_viewer.target_velocity.0.y =
                            target_speed * left_stick_x * speed_multiplier;
                    }
                    if left_stick_y.abs() > 0.1 {
                        tab_viewer.target_velocity.0.x =
                            -target_speed * left_stick_y * speed_multiplier;
                    }
                    let right_stick = tab_viewer
                        .gamepad_input
                        .get(GamepadAxis::new(gamepad, GamepadAxisType::RightStickX))
                        .unwrap();
                    if right_stick.abs() > 0.1 {
                        tab_viewer.target_velocity.1 = -right_stick
                            * tab_viewer.settings.manual_rotate_speed
                            * speed_multiplier.powi(2);
                    }
                }

                let mut pwm_override_motor: Option<usize> = None;
                let mut pwm_override_forward = false;
                if i.key_down(Key::U) {
                    pwm_override_motor = Some(0);
                    pwm_override_forward = true;
                }
                if i.key_down(Key::J) {
                    pwm_override_motor = Some(0);
                }
                if i.key_down(Key::I) {
                    pwm_override_motor = Some(1);
                    pwm_override_forward = true;
                }
                if i.key_down(Key::K) {
                    pwm_override_motor = Some(1);
                }
                if i.key_down(Key::O) {
                    pwm_override_motor = Some(2);
                    pwm_override_forward = true;
                }
                if i.key_down(Key::L) {
                    pwm_override_motor = Some(2);
                }
                if let Some(motor) = pwm_override_motor {
                    let mut command = [
                        MotorRequest::Pwm(0, 0),
                        MotorRequest::Pwm(0, 0),
                        MotorRequest::Pwm(0, 0),
                    ];
                    command[motor] = if pwm_override_forward {
                        MotorRequest::Pwm(0x8000, 0)
                    } else {
                        MotorRequest::Pwm(0, 0x8000)
                    };
                    tab_viewer.settings.pwm_override = Some(command);
                } else {
                    tab_viewer.settings.pwm_override = None;
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
        simulation: &mut PacbotSimulation,
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
                        *simulation = PacbotSimulation::new(
                            grid.compute_grid(),
                            Robot::default(),
                            grid.get_default_pacbot_isometry(),
                        );
                    }
                });
            });
    }

    fn draw_widget_icons(&mut self, ui: &mut Ui, tab_viewer: &mut TabViewer) {
        let mut widgets: [Box<&mut dyn PacbotWidget>; 7] = [
            Box::new(&mut self.grid_widget),
            Box::new(&mut self.game_widget),
            Box::new(&mut self.robot_widget),
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
                        tab_viewer.settings.high_level_strategy = match tab_viewer
                            .settings
                            .high_level_strategy
                        {
                            HighLevelStrategy::ReinforcementLearning => HighLevelStrategy::Manual,
                            _ => HighLevelStrategy::ReinforcementLearning,
                        }
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
                        &mut tab_viewer.simulation,
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
                        if ctx
                            .input(|i| i.pointer.primary_clicked() || i.pointer.secondary_clicked())
                        {
                            if let Some(pos) = tab_viewer.pointer_pos {
                                let pos = world_to_screen.inverse().map_point(pos);
                                let int_pos = IntLocation {
                                    row: pos.x.round() as i8,
                                    col: pos.y.round() as i8,
                                };
                                if !tab_viewer.grid.wall_at(&int_pos) {
                                    if ctx.input(|i| i.pointer.primary_clicked()) {
                                        tab_viewer.settings.kidnap_position = Some(int_pos);
                                    } else if tab_viewer.settings.high_level_strategy
                                        == HighLevelStrategy::Manual
                                    {
                                        tab_viewer.settings.test_path_position = Some(int_pos);
                                    }
                                }
                            }
                        }
                        // reset test_path_position if necessary
                        if let Some(Some(curr_pos)) = tab_viewer.phys_info.pf_pos.map(|x| {
                            tab_viewer
                                .grid
                                .node_nearest(x.translation.x, x.translation.y)
                        }) {
                            if Some(curr_pos) == tab_viewer.settings.test_path_position {
                                tab_viewer.settings.test_path_position = None;
                            }
                        } else {
                            tab_viewer.settings.test_path_position = None;
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
        RichText::new(regular::GRID_FOUR.to_string())
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
        self.ai_enabled =
            tab_viewer.settings.high_level_strategy == HighLevelStrategy::ReinforcementLearning;
    }

    fn display_name(&self) -> &'static str {
        "AI"
    }

    fn button_text(&self) -> RichText {
        RichText::new(regular::BRAIN.to_string())
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

impl Default for PacbotSensorsWidget {
    fn default() -> Self {
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
                    format!("Last data age: {:.2?}", t.elapsed()),
                    PacbotWidgetStatus::Error("".to_string()),
                ));
                self.overall_status =
                    PacbotWidgetStatus::Error(format!("Last data age: {:.2?}", t.elapsed()));
            } else {
                self.messages.push((
                    format!("Last data age: {:.2?}", t.elapsed()),
                    PacbotWidgetStatus::Ok,
                ));
            }
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
                self.messages.push((
                    format!("Velocity {i}: {:.2}", sensors.encoder_velocities[i]),
                    PacbotWidgetStatus::Ok,
                ));
                self.messages.push((
                    format!("PID {i}: {:.2}", sensors.pid_output[i]),
                    PacbotWidgetStatus::Ok,
                ));
            }
        }
    }

    fn display_name(&self) -> &'static str {
        "Sensors"
    }

    fn button_text(&self) -> RichText {
        RichText::new(regular::RULER.to_string())
    }

    fn overall_status(&self) -> &PacbotWidgetStatus {
        &self.overall_status
    }

    fn messages(&self) -> &[(String, PacbotWidgetStatus)] {
        &self.messages
    }
}
