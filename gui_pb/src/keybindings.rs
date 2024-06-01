use crate::App;
use core_pb::grid::standard_grid::StandardGrid;
use core_pb::messages::settings::{CvPositionSource, StrategyChoice};
use eframe::egui;
use eframe::egui::{Event, Key};

impl App {
    pub fn update_keybindings(&mut self, ctx: &egui::Context) {
        // don't activate keybindings if some element (text box, button) is focused
        if ctx.memory(|m| m.focused().is_some()) {
            return;
        }

        ctx.input(|i| {
            for event in &i.events {
                match event {
                    Event::Key {
                        key, pressed: true, ..
                    } => match key {
                        Key::Y => self.data.rotated_grid = !self.data.rotated_grid,
                        // Game state
                        Key::R => todo!("Reset game"),
                        Key::Space => todo!("Pause/unpause game"),
                        // Strategy
                        Key::Z => self.data.settings.driving.strategy = StrategyChoice::Manual,
                        Key::X => todo!("Reinforcement learning strategy"),
                        Key::C => self.data.settings.driving.strategy = StrategyChoice::TestUniform,
                        Key::V => self.data.settings.driving.strategy = StrategyChoice::TestForward,
                        // Driving
                        Key::P => todo!("Enable/disable pico"),
                        Key::M => {
                            self.data.settings.driving.commands_use_pf_angle =
                                !self.data.settings.driving.commands_use_pf_angle
                        }
                        // CV source
                        Key::G => {
                            self.data.settings.particle_filter.cv_position =
                                CvPositionSource::GameState
                        }
                        Key::H => {
                            self.data.settings.particle_filter.cv_position =
                                CvPositionSource::ParticleFilter
                        }
                        Key::T => {
                            if let Some(pos) = self.data.pointer_pos {
                                self.data.settings.particle_filter.cv_position =
                                    CvPositionSource::Constant(
                                        pos.x.round() as i8,
                                        pos.y.round() as i8,
                                    )
                            }
                        }
                        // Grid
                        Key::B => self.data.settings.grid = StandardGrid::Pacman,
                        Key::N => self.data.settings.grid = StandardGrid::Playground,
                        _ => {}
                    },
                    // Mouse buttons
                    // Event::PointerButton {
                    //     button: PointerButton::Primary,
                    //     pressed: true,
                    //     ..
                    // } => todo!("Set simulated pacman location"),
                    // Event::PointerButton {
                    //     button: PointerButton::Secondary,
                    //     pressed: true,
                    //     ..
                    // } => todo!("Set target path"),
                    _ => {}
                }
            }
        })
    }
}
