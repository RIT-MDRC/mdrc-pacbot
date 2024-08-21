use crate::App;
use core_pb::grid::standard_grid::StandardGrid;
use core_pb::messages::settings::StrategyChoice;
use core_pb::messages::{GameServerCommand, GuiToServerMessage, NetworkStatus};
use core_pb::pacbot_rs::location::Direction;
use core_pb::threaded_websocket::TextOrT;
use eframe::egui;
use eframe::egui::{Event, Key, PointerButton};
use nalgebra::Vector2;

impl App {
    pub fn set_target_vel(&mut self, target_vel: Option<(Vector2<f32>, f32)>) {
        if target_vel != self.target_vel {
            self.target_vel = target_vel;
            self.network
                .0
                .send(TextOrT::T(GuiToServerMessage::RobotVelocity(
                    self.ui_settings.selected_robot,
                    self.target_vel,
                )))
        }
    }

    pub fn read_input(&mut self, ctx: &egui::Context) {
        // don't activate keybindings if some element (text box, button) is focused
        if ctx.memory(|m| m.focused().is_some()) {
            self.set_target_vel(None);
            return;
        }

        ctx.input(|i| {
            let mut target_vel = (Vector2::new(0.0, 0.0), 0.0);
            let scale = self.settings.target_speed;
            for (key, (lin, ang)) in [
                (Key::W, (Vector2::new(0.0, scale), 0.0)),
                (Key::A, (Vector2::new(-scale, 0.0), 0.0)),
                (Key::D, (Vector2::new(scale, 0.0), 0.0)),
                (Key::S, (Vector2::new(0.0, -scale), 0.0)),
                (Key::Q, (Vector2::new(0.0, 0.0), scale / 3.0)),
                (Key::E, (Vector2::new(0.0, 0.0), -scale / 3.0)),
            ] {
                if i.key_down(key) {
                    target_vel.0 += lin;
                    target_vel.1 += ang;
                }
            }
            self.set_target_vel(if target_vel == (Vector2::new(0.0, 0.0), 0.0) {
                None
            } else {
                Some(target_vel)
            });

            // if the currently selected robot isn't connected, but the game server is, then
            // interpret WASD presses as an attempt to manually play Pacman
            if self.server_status.robots[self.ui_settings.selected_robot as usize].connection
                == NetworkStatus::NotConnected
                && self.server_status.game_server_connection == NetworkStatus::Connected
            {
                for (key, dir1, dir2) in [
                    (Key::W, Direction::Right, Direction::Up),
                    (Key::A, Direction::Up, Direction::Left),
                    (Key::S, Direction::Left, Direction::Down),
                    (Key::D, Direction::Down, Direction::Right),
                ] {
                    if i.key_pressed(key) {
                        self.send(GuiToServerMessage::GameServerCommand(
                            GameServerCommand::Direction(if self.rotated_grid {
                                dir1
                            } else {
                                dir2
                            }),
                        ));
                    }
                }
                if i.key_pressed(Key::W) {}
            }

            for event in &i.events {
                match event {
                    Event::Key {
                        key, pressed: true, ..
                    } => {
                        match key {
                            Key::Y => self.rotated_grid = !self.rotated_grid,
                            // Game state
                            Key::R => self.send(GuiToServerMessage::GameServerCommand(
                                GameServerCommand::Reset,
                            )),
                            Key::Space => {
                                if self.server_status.game_state.paused {
                                    self.send(GuiToServerMessage::GameServerCommand(
                                        GameServerCommand::Unpause,
                                    ))
                                } else {
                                    self.send(GuiToServerMessage::GameServerCommand(
                                        GameServerCommand::Pause,
                                    ))
                                }
                            }
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
                        }
                    }
                    // Mouse buttons
                    // Event::PointerButton {
                    //     button: PointerButton::Primary,
                    //     pressed: true,
                    //     ..
                    // } => todo!("Set simulated pacman location"),
                    Event::PointerButton {
                        button: PointerButton::Secondary,
                        pressed: true,
                        pos,
                        ..
                    } => {
                        let pos = self.world_to_screen.inverse().map_point(*pos);
                        if let Some(loc) = self.grid.node_nearest(pos.x, pos.y) {
                            self.send(GuiToServerMessage::TargetLocation(loc))
                        }
                    }
                    _ => {}
                }
            }
        })
    }
}
