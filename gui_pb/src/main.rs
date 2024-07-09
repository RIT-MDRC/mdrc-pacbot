mod colors;
mod game;
mod keybindings;
mod network;
mod replay;
mod replay_manager;
mod settings;
mod tab;
mod transform;

use crate::tab::Tab;
use crate::transform::Transform;
use anyhow::Error;
use core_pb::constants::GUI_LISTENER_PORT;
use core_pb::grid::computed_grid::ComputedGrid;
use core_pb::grid::standard_grid::StandardGrid;
use core_pb::messages::server_status::ServerStatus;
use core_pb::messages::settings::PacbotSettings;
use eframe::egui;
use eframe::egui::{Align, Color32, Pos2};
use egui_dock::{DockArea, DockState, NodeIndex, Style};
// todo use native_dialog::FileDialog;
use core_pb::messages::GuiToGameServerMessage;
use core_pb::threaded_websocket::ThreadedSocket;
use std::collections::HashMap;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::wasm_bindgen;
// use std::fs;
// use std::fs::File;

// When compiling natively:
#[cfg(not(target_arch = "wasm32"))]
fn main() {
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "RIT Pacbot",
        native_options,
        Box::new(|cc| Box::new(App::new(cc))),
    )
    .expect("Failed to start egui app!");
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    pub fn log(s: &str);
}

// When compiling to web using trunk:
#[cfg(target_arch = "wasm32")]
fn main() {
    // Redirect `log` message to `console.log` and friends:
    eframe::WebLogger::init(log::LevelFilter::Debug).ok();

    let web_options = eframe::WebOptions::default();

    wasm_bindgen_futures::spawn_local(async {
        let start_result = eframe::WebRunner::new()
            .start(
                "the_canvas_id", // hardcode it
                web_options,
                Box::new(|cc| Box::new(App::new(cc))),
            )
            .await;
        let loading_text = eframe::web_sys::window()
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

pub struct App {
    dock_state: DockState<Tab>,
    data: AppData,
}

pub struct AppData {
    grid: ComputedGrid,
    pointer_pos: Option<Pos2>,
    background_color: Color32,
    world_to_screen: Transform,
    // replay_manager: ReplayManager,
    server_status: ServerStatus,
    network: ThreadedSocket<GuiToGameServerMessage, ServerStatus>,
    settings: PacbotSettings,
    ui_settings: UiSettings,

    rotated_grid: bool,
    settings_fields: Option<HashMap<&'static str, (String, String)>>,
}

pub struct UiSettings {
    connect_mdrc_server: bool,
    mdrc_server_ipv4: [u8; 4],
    mdrc_server_ws_port: u16,

    mdrc_server_collapsed: bool,
    game_server_collapsed: bool,
    robot_collapsed: bool,
}

impl Default for UiSettings {
    fn default() -> Self {
        Self {
            connect_mdrc_server: true,
            mdrc_server_ipv4: [127, 0, 0, 1],
            mdrc_server_ws_port: GUI_LISTENER_PORT,

            mdrc_server_collapsed: true,
            game_server_collapsed: true,
            robot_collapsed: true,
        }
    }
}

impl App {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut fonts = egui::FontDefinitions::default();
        egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);

        cc.egui_ctx.set_fonts(fonts);

        let mut dock_state = DockState::new(vec![Tab::Grid, Tab::Robot]);
        let surface = dock_state.main_surface_mut();
        surface.split_right(NodeIndex::root(), 0.75, vec![Tab::Settings]);
        surface.split_left(NodeIndex::root(), 0.15, vec![Tab::Keybindings]);

        Self {
            dock_state,
            data: AppData::default(),
        }
    }
}

impl Default for AppData {
    fn default() -> Self {
        let ui_settings = UiSettings::default();
        let mut network = ThreadedSocket::default();
        network.connect(Some((
            ui_settings.mdrc_server_ipv4,
            ui_settings.mdrc_server_ws_port,
        )));

        Self {
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
            // todo replay_manager: Default::default(),
            server_status: Default::default(),
            network,
            settings: Default::default(),
            ui_settings,

            rotated_grid: true,
            settings_fields: Some(HashMap::new()),
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.data.pointer_pos = ctx.pointer_latest_pos();
        self.data.background_color = ctx.style().visuals.panel_fill;
        self.data.grid = self.data.settings.grid.compute_grid();
        self.update_keybindings(ctx);
        self.manage_network();

        self.draw_layout(ctx);

        ctx.request_repaint();
    }
}

impl App {
    /// Draw the main outer layout
    pub fn draw_layout(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("menu").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.with_layout(egui::Layout::left_to_right(Align::Center), |ui| {
                    // grid selector
                    egui::ComboBox::from_label("")
                        .selected_text(format!("{:?}", self.data.settings.grid))
                        .show_ui(ui, |ui| {
                            StandardGrid::get_all().iter().for_each(|grid| {
                                ui.selectable_value(
                                    &mut self.data.settings.grid,
                                    *grid,
                                    format!("{:?}", grid),
                                );
                            });
                        });
                    // top left buttons
                    egui::menu::bar(ui, |ui| {
                        ui.menu_button("Replay", |ui| {
                            if ui.button("Save").clicked() {
                                self.save_replay().expect("Failed to save replay!");
                            }
                            if ui.button("Load").clicked() {
                                self.load_replay().expect("Failed to load replay!");
                            }
                        });
                    });
                    // TODO widgets
                });
                ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                    ui.label(
                        &(match self.data.pointer_pos {
                            None => "".to_string(),
                            Some(pos) => {
                                let pos = self.data.world_to_screen.inverse().map_point(pos);
                                format!("({:.1}, {:.1})", pos.x, pos.y)
                            }
                        }),
                    );
                });
            });
        });

        DockArea::new(&mut self.dock_state)
            .style(Style::from_egui(ctx.style().as_ref()))
            .show(ctx, &mut self.data)
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
