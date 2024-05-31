mod colors;
mod game;
mod keybindings;
mod network;
mod replay;
mod replay_manager;
mod settings;
mod tab;
mod transform;

use crate::replay::Replay;
use crate::replay_manager::ReplayManager;
use crate::tab::Tab;
use crate::transform::Transform;
use anyhow::Error;
use core_pb::grid::computed_grid::ComputedGrid;
use core_pb::grid::standard_grid::StandardGrid;
use core_pb::messages::settings::PacbotSettings;
use core_pb::pacbot_rs::game_state::GameState;
use eframe::egui;
use eframe::egui::{Align, Color32, Id, Pos2};
use egui_dock::{DockArea, DockState, NodeIndex, Style};
use native_dialog::FileDialog;
use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io::{Read, Write};

fn main() {
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "RIT Pacbot",
        native_options,
        Box::new(|cc| Box::new(App::new(cc))),
    )
    .expect("Failed to start egui app!");
}

pub struct App {
    dock_state: DockState<Tab>,
    data: AppData,
}

pub struct AppData {
    game: GameState,
    grid: ComputedGrid,
    pointer_pos: Option<Pos2>,
    background_color: Color32,
    world_to_screen: Transform,
    replay_manager: ReplayManager,
    settings: PacbotSettings,
    ui_settings: UiSettings,

    rotated_grid: bool,
    settings_fields: Option<HashMap<Id, (String, String)>>,
}

pub struct UiSettings {
    mdrc_server_collapsed: bool,
    game_server_collapsed: bool,
    robot_collapsed: bool,
}

impl Default for UiSettings {
    fn default() -> Self {
        Self {
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
        Self {
            game: Default::default(),
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
            replay_manager: Default::default(),
            settings: Default::default(),
            ui_settings: Default::default(),

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

        self.draw_layout(ctx);
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
        let path = FileDialog::new()
            .add_filter("Pacbot Replay", &["pb"])
            .set_filename("replay.pb")
            .show_save_single_file()?;

        if let Some(path) = path {
            let bytes = self.data.replay_manager.replay.to_bytes()?;
            let mut file = fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(path)?;
            file.write_all(&bytes)?;
        }

        Ok(())
    }

    /// Load a replay from file
    pub fn load_replay(&mut self) -> Result<(), Error> {
        let path = FileDialog::new()
            .add_filter("Pacbot Replay", &["pb"])
            .show_open_single_file()?;

        if let Some(path) = path {
            let mut file = File::open(&path)?;
            let metadata = fs::metadata(&path).expect("unable to read metadata");
            let mut buffer = vec![0; metadata.len() as usize];
            file.read_exact(&mut buffer)?;

            let replay = Replay::from_bytes(&buffer)?;

            // self.settings.mode = AppMode::Playback;
            self.data.replay_manager.replay = replay.0;
            self.data.replay_manager.playback_paused = true;
        }

        Ok(())
    }
}
