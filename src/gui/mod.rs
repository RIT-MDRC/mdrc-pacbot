//! Top-level GUI elements and functionality.

mod colors;
mod game;
pub(crate) mod physics;
pub mod replay_manager;
mod stopwatch;
pub mod transforms;
pub mod utils;

use std::cell::RefMut;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use crate::grid::ComputedGrid;
use bevy::prelude::*;
use bevy_egui::EguiContexts;
use eframe::egui;
use eframe::egui::{Align, Color32, Frame, Pos2, RichText, Ui, WidgetText};
use egui_dock::{DockArea, DockState, Style};
use egui_phosphor::regular;
use pacbot_rs::game_engine::GameEngine;

use crate::grid::standard_grids::StandardGrid;
use crate::gui::replay_manager::ReplayManager;
use crate::physics::LightPhysicsInfo;
use crate::util::stopwatch::Stopwatch;
use crate::{PacmanGameState, PacmanReplayManager, StandardGridResource, UserSettings};

use self::transforms::Transform;

/// Tracks the performance of GUI rendering
#[derive(Default, Resource)]
pub struct GuiStopwatch(pub Stopwatch);

fn font_setup(mut contexts: EguiContexts) {
    let mut fonts = egui::FontDefinitions::default();
    egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);

    contexts.ctx_mut().set_fonts(fonts);
}

fn ui_system(
    mut contexts: EguiContexts,
    mut world: RefMut<World>,
    mut world_to_screen: Local<Option<Transform>>,
) {
    let mut app: Mut<App> = world.resource_mut();
    egui::Window::new("Pacbot simulation").show(contexts.ctx_mut(), |f| {
        app.update(
            contexts.ctx_mut(),
            f,
            &mut world.resource_mut::<PacmanGameState>().0,
            &*world.resource::<LightPhysicsInfo>(),
            &mut world_to_screen,
            &mut world.resource_mut::<StandardGridResource>().0,
            &mut world.resource_mut::<ComputedGrid>(),
            &mut world.resource_mut::<PacmanReplayManager>().0,
            &mut world.resource_mut::<UserSettings>(),
        )
    });
}

#[derive(Copy, Clone)]
pub enum Tab {
    Grid,
    Stopwatch,
    Unknown,
}

struct TabViewer<'a> {
    pointer_pos: Option<Pos2>,
    background_color: Color32,

    bevy_world: Option<&'a mut World>,
}

impl<'a> egui_dock::TabViewer for TabViewer<'a> {
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
                // ui.label("Particle Filter");
                // draw_stopwatch(&self.pf_stopwatch.read().unwrap(), ui, "pf_sw".to_string());
                // ui.separator();
                // ui.label("Physics");
                // draw_stopwatch(
                //     &self.physics_stopwatch.read().unwrap(),
                //     ui,
                //     "ph_sw".to_string(),
                // );
                // draw_stopwatch(&self.gui_stopwatch.read().unwrap(), ui);
            }
            _ => panic!("Widget did not declare a tab!"),
        }
    }
}

impl<'a> Default for TabViewer<'a> {
    fn default() -> Self {
        Self {
            pointer_pos: None,
            background_color: Color32::BLACK,

            world_to_screen: None,
            bevy_world: None,
        }
    }
}

impl<'a> TabViewer<'a> {
    fn grid_ui(&mut self, ui: &mut Ui) {
        // let rect = ui.max_rect();
        // let (src_p1, src_p2) = self.selected_grid.get_soft_boundaries();
        //
        // let world_to_screen = Transform::new_letterboxed(
        //     src_p1,
        //     src_p2,
        //     Pos2::new(rect.top(), rect.left()),
        //     Pos2::new(rect.bottom(), rect.right()),
        // );
        // self.world_to_screen = Some(world_to_screen);
        // let painter = ui.painter_at(rect);
        //
        // self.draw_grid(&world_to_screen, &painter);
        //
        // if self.selected_grid == StandardGrid::Pacman {
        //     self.draw_pacman_state(&world_to_screen, &painter);
        // }

        // self.draw_simulation(&world_to_screen, &painter);
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

/// Indicates the current meta-state of the app
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum AppMode {
    /// Using a game server with physics engine and recording the results to file
    Recording,
    /// Playing information back from a file; no game server but physics should still run
    Playback,
}

#[derive(Resource)]
struct App {
    tree: DockState<Tab>,
}

fn pretty_print_time_now() -> String {
    let date = chrono::Local::now();
    date.format("%Y_%m_%d__%H_%M_%S").to_string()
}

impl Default for App {
    fn default() -> Self {
        Self {
            tree: DockState::new(vec![Tab::Grid]),
        }
    }
}

impl App {
    // TODO
    // fn update_target_velocity(&mut self, ctx: &egui::Context) {
    //     let ai_enabled = *self.tab_viewer.ai_enable.read().unwrap().deref();
    //     if !ai_enabled {
    //         let mut target_velocity = self.tab_viewer.target_velocity.write().unwrap();
    //         target_velocity.0.x = 0.0;
    //         target_velocity.0.y = 0.0;
    //         target_velocity.1 = 0.0;
    //         ctx.input(|i| {
    //             let target_speed = if i.modifiers.shift { 2.0 } else { 0.8 };
    //             if i.key_down(Key::S) {
    //                 target_velocity.0.x = target_speed;
    //             }
    //             if i.key_down(Key::W) {
    //                 target_velocity.0.x = -target_speed;
    //             }
    //             if i.key_down(Key::A) {
    //                 target_velocity.0.y = -target_speed;
    //             }
    //             if i.key_down(Key::D) {
    //                 target_velocity.0.y = target_speed;
    //             }
    //             if i.key_down(Key::E) {
    //                 target_velocity.1 = -1.0;
    //             }
    //             if i.key_down(Key::Q) {
    //                 target_velocity.1 = 1.0;
    //             }
    //         });
    //     }
    // }

    fn add_grid_variants(
        &mut self,
        ui: &mut Ui,
        pacman_state: &mut GameEngine,
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
                        pacman_state.pause();
                        *computed_grid = grid.compute_grid();
                        // todo unwrap
                        replay_manager.reset_replay(
                            *grid,
                            pacman_state,
                            phys_info.real_pos.unwrap(),
                        );
                    }
                });
            });
    }

    fn draw_widget_icons(&mut self, ui: &mut Ui) {
        // TODO
        // let widgets: Vec<Box<&mut dyn PacbotWidget>> = vec![
        //     Box::new(&mut self.tab_viewer.grid_widget),
        //     Box::new(&mut self.tab_viewer.game_widget),
        //     Box::new(&mut self.tab_viewer.stopwatch_widget),
        //     Box::new(&mut self.tab_viewer.ai_widget),
        //     Box::new(&mut self.tab_viewer.sensors_widget),
        // ];
        // for mut widget in widgets {
        //     widget.deref_mut().update();
        //     let mut button = ui.add(egui::Button::new(widget.button_text()).fill(
        //         match widget.overall_status() {
        //             PacbotWidgetStatus::Ok => TRANSLUCENT_GREEN_COLOR,
        //             PacbotWidgetStatus::Warn(_) => TRANSLUCENT_YELLOW_COLOR,
        //             PacbotWidgetStatus::Error(_) => TRANSLUCENT_RED_COLOR,
        //             PacbotWidgetStatus::NotApplicable => Color32::TRANSPARENT,
        //         },
        //     ));
        //     button = button.on_hover_ui(|ui| {
        //         ui.label(widget.display_name());
        //         for msg in widget.messages() {
        //             ui.label(
        //                 RichText::new(format!(
        //                     "{} {}",
        //                     match msg.1 {
        //                         PacbotWidgetStatus::Ok => regular::CHECK,
        //                         PacbotWidgetStatus::Warn(_) => regular::WARNING,
        //                         PacbotWidgetStatus::Error(_) => regular::X,
        //                         PacbotWidgetStatus::NotApplicable => regular::CHECK,
        //                     },
        //                     msg.0.to_owned()
        //                 ))
        //                 .color(match msg.1 {
        //                     PacbotWidgetStatus::Ok => Color32::GREEN,
        //                     PacbotWidgetStatus::Warn(_) => Color32::YELLOW,
        //                     PacbotWidgetStatus::Error(_) => Color32::RED,
        //                     PacbotWidgetStatus::NotApplicable => Color32::GREEN,
        //                 }),
        //             );
        //         }
        //     });
        //     if button.clicked() {
        //         match widget.display_name() {
        //             "Game (Click to Reset)" => {
        //                 self.tab_viewer.pacman_render.write().unwrap().pacman_state =
        //                     GameEngine::default()
        //             }
        //             "AI" => {
        //                 let val = *self.tab_viewer.ai_enable.read().unwrap();
        //                 *self.tab_viewer.ai_enable.write().unwrap() = !val;
        //             }
        //             "Sensors" => {}
        //             _ => self.tree.push_to_focused_leaf(widget.tab()),
        //         }
        //     }
        // }
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

impl App {
    fn update(
        &mut self,
        ctx: &egui::Context,
        _frame: &mut eframe::Frame,
        pacman_state: &mut GameEngine,
        phys_info: &LightPhysicsInfo,
        world_to_screen: &mut Option<Transform>,
        selected_grid: &mut StandardGrid,
        grid: &mut ComputedGrid,
        replay_manager: &mut ReplayManager,
        settings: &mut UserSettings,
    ) {
        let mut tab_viewer = TabViewer {
            pointer_pos: ctx.pointer_latest_pos(),
            background_color: ctx.style().visuals.panel_fill,
            bevy_world: None,
        };

        // TODO
        // self.update_target_velocity(ctx);

        // TODO
        // self.tab_viewer
        //     .update_replay_manager()
        //     .expect("Error updating replay manager");

        egui::TopBottomPanel::top("menu").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.with_layout(egui::Layout::left_to_right(Align::Center), |ui| {
                    self.add_grid_variants(
                        ui,
                        pacman_state,
                        phys_info,
                        replay_manager,
                        selected_grid,
                        grid,
                    );
                    egui::menu::bar(ui, |ui| {
                        ui.menu_button("Replay", |ui| {
                            if ui.button("Save").clicked() {
                                tab_viewer
                                    .save_replay(replay_manager)
                                    .expect("Failed to save replay!");
                            }
                            if ui.button("Load").clicked() {
                                tab_viewer
                                    .load_replay(pacman_state, replay_manager, settings)
                                    .expect("Failed to load replay!");
                            }
                            if ui
                                .add(
                                    egui::Button::new("Save Pacbot Location")
                                        .selected(settings.replay_save_location),
                                )
                                .clicked()
                            {
                                settings.replay_save_location = !settings.replay_save_location;
                            }
                        });

                        self.draw_widget_icons(ui);
                    })
                });
                ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                    ui.label(
                        &(match tab_viewer.pointer_pos {
                            None => "".to_string(),
                            Some(pos) => {
                                let pos = world_to_screen.inverse().map_point(pos);
                                format!("({:.1}, {:.1})", pos.x, pos.y)
                            }
                        }),
                    );
                });
            });
        });
        if *selected_grid == StandardGrid::Pacman {
            egui::TopBottomPanel::bottom("playback_controls")
                .frame(
                    Frame::none()
                        .fill(ctx.style().visuals.panel_fill)
                        .inner_margin(5.0),
                )
                .show(ctx, |ui| {
                    tab_viewer.draw_replay_ui(ctx, ui, pacman_state, replay_manager, settings);
                });
        }
        DockArea::new(&mut self.tree)
            .style(Style::from_egui(ctx.style().as_ref()))
            .show(ctx, &mut tab_viewer);

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
            for i in 0..3 {
                self.messages.push((
                    format!("Encoder {i}: {}", sensors.2[i]),
                    PacbotWidgetStatus::Ok,
                ));
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
