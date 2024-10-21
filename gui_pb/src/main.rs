mod colors;

mod input;
mod replay;

mod drawing;
mod transform;

use crate::drawing::tab::Tab;
use crate::transform::Transform;
use anyhow::Error;
use core_pb::grid::computed_grid::ComputedGrid;
use core_pb::grid::standard_grid::StandardGrid;
use core_pb::messages::server_status::ServerStatus;
use core_pb::messages::settings::PacbotSettings;
use core_pb::pacbot_rs::game_state::GameState;
use eframe::egui;
use eframe::egui::{Align, Color32, Pos2, Visuals};
use egui_dock::{DockArea, DockState, NodeIndex, Style};
// todo use native_dialog::FileDialog;
use crate::drawing::motors::MotorStatusGraphFrames;
use crate::drawing::settings::UiSettings;
use crate::drawing::widgets::draw_widgets;
use core_pb::console_log;
#[cfg(target_arch = "wasm32")]
pub use core_pb::log;
use core_pb::messages::{GameServerCommand, GuiToServerMessage, NetworkStatus, ServerToGuiMessage};
use core_pb::threaded_websocket::{Address, TextOrT, ThreadedSocket};
use core_pb::util::stopwatch::Stopwatch;
#[cfg(not(target_arch = "wasm32"))]
use core_pb::util::StdInstant;
#[cfg(target_arch = "wasm32")]
use core_pb::util::WebTimeInstant as StdInstant;
use nalgebra::Vector2;
use std::collections::HashMap;
use std::time::Duration;

// When compiling natively:
#[cfg(not(target_arch = "wasm32"))]
fn main() {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .init();
    console_log!("RIT Pacbot gui starting up");
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "RIT Pacbot",
        native_options,
        Box::new(|cc| {
            let style = egui::Style {
                visuals: Visuals::dark(),
                ..egui::Style::default()
            };
            cc.egui_ctx.set_style(style);
            Box::new(App::new(cc))
        }),
    )
    .expect("Failed to start egui app!");
}

// When compiling to web using trunk:
#[cfg(target_arch = "wasm32")]
fn main() {
    console_log!("WASM gui starting up");

    // Redirect `log` message to `console.log` and friends:
    eframe::WebLogger::init(log::LevelFilter::Debug).ok();

    let web_options = eframe::WebOptions::default();

    wasm_bindgen_futures::spawn_local(async {
        let start_result = eframe::WebRunner::new()
            .start(
                "the_canvas_id", // hardcode it
                web_options,
                Box::new(|cc| {
                    let style = egui::Style {
                        visuals: Visuals::dark(),
                        ..egui::Style::default()
                    };
                    cc.egui_ctx.set_style(style);
                    Box::new(App::new(cc))
                }),
            )
            .await;
        let loading_text = web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.get_element_by_id("loading_text"));
        match start_result {
            Ok(_) => {
                loading_text.map(|e| e.remove());
            }
            Err(e) => {
                loading_text.map(|e| {
                    e.set_inner_html(
                        "<p> The app has crashed. See the developer console for details. </p>",
                    )
                });
                panic!("failed to start eframe: {e:?}");
            }
        }
    });
}

/// Stores all the data needed for the application
pub struct App {
    dock_state: Option<DockState<Tab>>,

    grid: ComputedGrid,
    pointer_pos: Option<Pos2>,
    background_color: Color32,
    world_to_screen: Transform,
    robot_buttons_wts: Transform,
    // replay_manager: ReplayManager,
    server_status: ServerStatus,
    saved_game_state: Option<GameState>,
    network: (
        ThreadedSocket<GuiToServerMessage, ServerToGuiMessage>,
        Option<Address>,
    ),
    old_settings: PacbotSettings,
    settings: PacbotSettings,
    ui_settings: UiSettings,
    target_vel: Option<(Vector2<f32>, f32)>,
    motor_status_frames: MotorStatusGraphFrames<3>,
    gui_stopwatch: Stopwatch<5, 30, StdInstant>,
    rotated_grid: bool,
    settings_fields: Option<HashMap<String, (String, String)>>,
    pacbot_server_connection_status: NetworkStatus,
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.gui_stopwatch.start();

        self.pointer_pos = ctx.pointer_latest_pos();
        self.background_color = ctx.style().visuals.panel_fill;
        if *self.grid.standard_grid() != Some(self.settings.standard_grid) {
            self.grid = self.settings.standard_grid.compute_grid();
        }
        self.gui_stopwatch.mark_completed("Initialization").unwrap();
        self.read_input(ctx);
        self.gui_stopwatch.mark_completed("Read input").unwrap();
        self.manage_network();
        self.gui_stopwatch.mark_completed("Manage network").unwrap();

        self.draw_layout(ctx);
        self.gui_stopwatch.mark_completed("Draw graphics").unwrap();

        ctx.request_repaint();
        self.gui_stopwatch
            .mark_completed("Request repaint")
            .unwrap();
    }
}

impl App {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut fonts = egui::FontDefinitions::default();
        egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);

        cc.egui_ctx.set_fonts(fonts);

        let mut dock_state =
            DockState::new(vec![Tab::Grid, Tab::Motors, Tab::Robot, Tab::Stopwatch]);
        let surface = dock_state.main_surface_mut();
        surface.split_right(NodeIndex::root(), 0.75, vec![Tab::Settings]);
        let [_, left] = surface.split_left(
            NodeIndex::root(),
            0.15,
            vec![Tab::OverTheAirProgramming, Tab::Keybindings],
        );
        let [_, below] = surface.split_below(left, 0.6, vec![Tab::RobotDisplay]);
        surface.split_below(below, 0.6, vec![Tab::RobotButtonPanel]);

        let ui_settings: UiSettings = Default::default();

        Self {
            dock_state: Some(dock_state),

            grid: Default::default(),
            pointer_pos: None,
            background_color: Color32::BLACK,
            world_to_screen: Transform::new_letterboxed(
                Pos2::new(0.0, 0.0),
                Pos2::new(0.0, 1.0),
                Pos2::new(0.0, 0.0),
                Pos2::new(0.0, 1.0),
                false,
            ),
            robot_buttons_wts: Transform::new_letterboxed(
                Pos2::new(0.0, 0.0),
                Pos2::new(0.0, 1.0),
                Pos2::new(0.0, 0.0),
                Pos2::new(0.0, 1.0),
                false,
            ),
            // todo replay_manager: Default::default(),
            server_status: Default::default(),
            saved_game_state: Option::None,
            network: (
                ThreadedSocket::with_name("gui[server]".to_string()),
                Default::default(),
            ),
            old_settings: Default::default(),
            settings: Default::default(),
            motor_status_frames: MotorStatusGraphFrames::new(ui_settings.selected_robot),
            ui_settings,
            target_vel: None,
            gui_stopwatch: Stopwatch::new(
                "Gui",
                Duration::from_millis(15),
                Duration::from_millis(20),
                0.8,
                0.9,
            ),

            rotated_grid: true,
            settings_fields: Some(HashMap::new()),
            pacbot_server_connection_status: NetworkStatus::NotConnected,
        }
    }

    pub fn send(&self, message: GuiToServerMessage) {
        self.network.0.send(TextOrT::T(message))
    }

    pub fn manage_network(&mut self) {
        let new_addr = if self.ui_settings.mdrc_server.connect {
            Some((
                self.ui_settings.mdrc_server.ipv4,
                self.ui_settings.mdrc_server.port,
            ))
        } else {
            None
        };
        if self.network.1 != new_addr {
            self.network.1 = new_addr;
            self.network.0.connect(new_addr)
        }
        // we must check for changed settings before updating them from the server
        if self.old_settings != self.settings {
            self.send(GuiToServerMessage::Settings(self.settings.clone()));
        }
        while let Some(TextOrT::T(msg)) = self.network.0.read() {
            match msg {
                ServerToGuiMessage::Settings(settings) => {
                    if self.pacbot_server_connection_status != NetworkStatus::Connected
                        && self.network.0.status() == NetworkStatus::Connected
                        && self.settings != PacbotSettings::default()
                    {
                        // send our settings to hopefully replace the server's
                        self.send(GuiToServerMessage::Settings(self.settings.clone()));
                    }
                    self.settings = settings.clone();
                    self.old_settings = settings
                }
                ServerToGuiMessage::Status(status) => self.server_status = status,
            }
        }
        self.pacbot_server_connection_status = self.network.0.status();
    }

    /// Draw the main outer layout
    pub fn draw_layout(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("menu").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.with_layout(egui::Layout::left_to_right(Align::Center), |ui| {
                    // grid selector
                    egui::ComboBox::from_label("")
                        .selected_text(format!("{:?}", self.settings.standard_grid))
                        .show_ui(ui, |ui| {
                            StandardGrid::get_all().iter().for_each(|grid| {
                                ui.selectable_value(
                                    &mut self.settings.standard_grid,
                                    *grid,
                                    format!("{:?}", grid),
                                );
                            });
                        });
                    // top left buttons
                    egui::menu::bar(ui, |ui| {
                        if ui.button("Save").clicked() {
                            self.saved_game_state = Some(self.server_status.game_state.clone());
                        }
                        if ui.button("Load").clicked() {
                            if let Some(x) = &self.saved_game_state {
                                let mut x = x.clone();
                                x.paused = self.server_status.game_state.paused;
                                self.send(GuiToServerMessage::GameServerCommand(
                                    GameServerCommand::SetState(x),
                                ))
                            }
                        }

                        ui.menu_button("Replay", |ui| {
                            if ui.button("Save").clicked() {
                                self.save_replay().expect("Failed to save replay!");
                            }
                            if ui.button("Load").clicked() {
                                self.load_replay().expect("Failed to load replay!");
                            }
                        });
                        draw_widgets(self, ui)
                    });
                });
                ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                    ui.label(
                        &(match self.pointer_pos {
                            None => "".to_string(),
                            Some(pos) => {
                                let pos = self.world_to_screen.inverse().map_point(pos);
                                format!("({:.1}, {:.1})", pos.x, pos.y)
                            }
                        }),
                    );
                });
            });
        });

        // take out dock_state to pass it to DockArea::new and allow tabs to use data from App
        let mut dock_state = self.dock_state.take().unwrap();
        DockArea::new(&mut dock_state)
            .style(Style::from_egui(ctx.style().as_ref()))
            .show(ctx, self);
        self.dock_state = Some(dock_state);
    }

    /// Save the current replay to file
    pub fn save_replay(&self) -> Result<(), Error> {
        todo!()
        // let path = FileDialog::new()
        //     .add_filter("Pacbot Replay", &["pb"])
        //     .set_filename("replay.pb")
        //     .show_save_single_file()?;
        //
        // if let Some(path) = path {
        //     let bytes = self.data.replay_manager.replay.to_bytes()?;
        //     let mut file = fs::OpenOptions::new()
        //         .write(true)
        //         .create(true)
        //         .truncate(true)
        //         .open(path)?;
        //     file.write_all(&bytes)?;
        // }
        //
        // Ok(())
    }

    /// Load a replay from file
    pub fn load_replay(&mut self) -> Result<(), Error> {
        todo!()
        // let path = FileDialog::new()
        //     .add_filter("Pacbot Replay", &["pb"])
        //     .show_open_single_file()?;
        //
        // if let Some(path) = path {
        //     let mut file = File::open(&path)?;
        //     let metadata = fs::metadata(&path).expect("unable to read metadata");
        //     let mut buffer = vec![0; metadata.len() as usize];
        //     file.read_exact(&mut buffer)?;
        //
        //     let replay = Replay::from_bytes(&buffer)?;
        //
        //     // self.settings.mode = AppMode::Playback;
        //     self.data.replay_manager.replay = replay.0;
        //     self.data.replay_manager.playback_paused = true;
        // }
        //
        // Ok(())
    }
}
