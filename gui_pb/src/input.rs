use crate::App;
use core_pb::grid::standard_grid::StandardGrid;
use core_pb::messages::settings::StrategyChoice;
use eframe::egui;
use eframe::egui::{Event, Key};

impl App {
    pub fn read_input(&mut self, ctx: &egui::Context) {
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
                        Key::Y => self.rotated_grid = !self.rotated_grid,
                        // Game state
                        Key::R => todo!("Reset game"),
                        Key::Space => todo!("Pause/unpause game"),
                        // Strategy
                        Key::Z => self.settings.driving.strategy = StrategyChoice::Manual,
                        Key::X => todo!("Reinforcement learning strategy"),
                        Key::C => self.settings.driving.strategy = StrategyChoice::TestUniform,
                        Key::V => self.settings.driving.strategy = StrategyChoice::TestForward,
                        // Driving
                        Key::P => todo!("Enable/disable pico"),
                        Key::M => {
                            self.settings.driving.commands_use_pf_angle =
                                !self.settings.driving.commands_use_pf_angle
                        }
                        // CV source
                        // Key::G => {
                        //     self.settings.particle_filter.cv_position = CvPositionSource::GameState
                        // }
                        // Key::H => {
                        //     self.settings.particle_filter.cv_position =
                        //         CvPositionSource::ParticleFilter
                        // }
                        // Key::T => {
                        //     if let Some(pos) = self.pointer_pos {
                        //         self.settings.particle_filter.cv_position =
                        //             CvPositionSource::Constant(
                        //                 pos.x.round() as i8,
                        //                 pos.y.round() as i8,
                        //             )
                        //     }
                        // }
                        // Grid
                        Key::B => self.settings.standard_grid = StandardGrid::Pacman,
                        Key::N => self.settings.standard_grid = StandardGrid::Playground,
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
