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
use eframe::egui;
use eframe::egui::{Align, Color32, Pos2};
use egui_dock::{DockArea, DockState, NodeIndex, Style};
// todo use native_dialog::FileDialog;
use crate::drawing::settings::UiSettings;
use core_pb::console_log;
#[cfg(target_arch = "wasm32")]
pub use core_pb::log;
use core_pb::messages::GuiToGameServerMessage;
use core_pb::threaded_websocket::{Address, ThreadedSocket};
use std::collections::HashMap;

// When compiling natively:
#[cfg(not(target_arch = "wasm32"))]
fn main() {
    console_log!("RIT Pacbot gui starting up");
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "RIT Pacbot",
        native_options,
        Box::new(|cc| Box::new(App::new(cc))),
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
                Box::new(|cc| Box::new(App::new(cc))),
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
    // replay_manager: ReplayManager,
    server_status: ServerStatus,
    network: (
        ThreadedSocket<GuiToGameServerMessage, ServerStatus>,
        Option<Address>,
    ),
    settings: PacbotSettings,
    ui_settings: UiSettings,

    rotated_grid: bool,
    settings_fields: Option<HashMap<&'static str, (String, String)>>,
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.pointer_pos = ctx.pointer_latest_pos();
        self.background_color = ctx.style().visuals.panel_fill;
        if *self.grid.standard_grid() != Some(self.settings.grid) {
            self.grid = self.settings.grid.compute_grid();
        }
        self.read_input(ctx);
        self.manage_network();

        self.draw_layout(ctx);

        ctx.request_repaint();
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
            // todo replay_manager: Default::default(),
            server_status: Default::default(),
            network: Default::default(),
            settings: Default::default(),
            ui_settings: Default::default(),

            rotated_grid: true,
            settings_fields: Some(HashMap::new()),
        }
    }

    pub fn manage_network(&mut self) {
        let new_addr = if self.ui_settings.connect_mdrc_server {
            Some((
                self.ui_settings.mdrc_server_ipv4,
                self.ui_settings.mdrc_server_ws_port,
            ))
        } else {
            None
        };
        if self.network.1 != new_addr {
            self.network.1 = new_addr;
            self.network.0.connect(new_addr)
        }
        if let Some(status) = self.network.0.read() {
            self.server_status = status;
            self.settings = self.server_status.settings.clone();
        }
        if self.server_status.settings != self.settings {
            self.network
                .0
                .send(GuiToGameServerMessage::Settings(self.settings.clone()));
            self.server_status.settings = self.settings.clone();
        }
    }

    /// Draw the main outer layout
    pub fn draw_layout(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("menu").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.with_layout(egui::Layout::left_to_right(Align::Center), |ui| {
                    // grid selector
                    egui::ComboBox::from_label("")
                        .selected_text(format!("{:?}", self.settings.grid))
                        .show_ui(ui, |ui| {
                            StandardGrid::get_all().iter().for_each(|grid| {
                                ui.selectable_value(
                                    &mut self.settings.grid,
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
