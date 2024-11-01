use crate::App;
use core_pb::grid::standard_grid::StandardGrid;
use core_pb::messages::settings::{CvLocationSource, StrategyChoice};
use core_pb::messages::{
    GameServerCommand, GuiToServerMessage, NetworkStatus, RobotButton, ServerToSimulationMessage,
};
use core_pb::pacbot_rs::location::Direction;
use core_pb::robot_definition::RobotDefinition;
use core_pb::threaded_websocket::TextOrT;
use eframe::egui;
use eframe::egui::{Event, Key, PointerButton};
use nalgebra::{Point2, Vector2};

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
                (Key::Q, (Vector2::new(0.0, 0.0), scale)),
                (Key::E, (Vector2::new(0.0, 0.0), -scale)),
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
                            Key::X => {
                                self.settings.driving.strategy =
                                    StrategyChoice::ReinforcementLearning
                            }
                            Key::C => self.settings.driving.strategy = StrategyChoice::TestUniform,
                            Key::V => self.settings.driving.strategy = StrategyChoice::TestForward,
                            // Driving
                            Key::P => {
                                self.settings.robots[self.ui_settings.selected_robot as usize]
                                    .connection
                                    .connect = !self.settings.robots
                                    [self.ui_settings.selected_robot as usize]
                                    .connection
                                    .connect
                            }
                            Key::U => {
                                self.settings.robots[self.ui_settings.selected_robot as usize]
                                    .config
                                    .pwm_override[0][0] = Some(
                                    RobotDefinition::new(self.ui_settings.selected_robot).pwm_top
                                        / 2,
                                )
                            }
                            Key::J => {
                                self.settings.robots[self.ui_settings.selected_robot as usize]
                                    .config
                                    .pwm_override[0][1] = Some(
                                    RobotDefinition::new(self.ui_settings.selected_robot).pwm_top
                                        / 2,
                                )
                            }
                            Key::I => {
                                self.settings.robots[self.ui_settings.selected_robot as usize]
                                    .config
                                    .pwm_override[1][0] = Some(
                                    RobotDefinition::new(self.ui_settings.selected_robot).pwm_top
                                        / 2,
                                )
                            }
                            Key::K => {
                                self.settings.robots[self.ui_settings.selected_robot as usize]
                                    .config
                                    .pwm_override[1][1] = Some(
                                    RobotDefinition::new(self.ui_settings.selected_robot).pwm_top
                                        / 2,
                                )
                            }
                            Key::O => {
                                self.settings.robots[self.ui_settings.selected_robot as usize]
                                    .config
                                    .pwm_override[2][0] = Some(
                                    RobotDefinition::new(self.ui_settings.selected_robot).pwm_top
                                        / 2,
                                )
                            }
                            Key::L => {
                                self.settings.robots[self.ui_settings.selected_robot as usize]
                                    .config
                                    .pwm_override[2][1] = Some(
                                    RobotDefinition::new(self.ui_settings.selected_robot).pwm_top
                                        / 2,
                                )
                            }
                            // CV source
                            Key::G => {
                                self.settings.cv_location_source = CvLocationSource::GameState
                            }
                            Key::H => {
                                self.settings.cv_location_source = CvLocationSource::Localization
                            }
                            Key::T => {
                                if let Some(pos) = self.pointer_pos {
                                    let pos = self.world_to_screen.inverse().map_point(pos);
                                    let p = Point2::new(pos.x.round() as i8, pos.y.round() as i8);
                                    if !self.grid.wall_at(&p) {
                                        self.settings.cv_location_source =
                                            CvLocationSource::Constant(Some(p))
                                    }
                                }
                            }
                            // Grid
                            Key::B => self.settings.standard_grid = StandardGrid::Pacman,
                            Key::N => self.settings.standard_grid = StandardGrid::Playground,
                            _ => {}
                        }
                    }
                    Event::Key {
                        key,
                        pressed: false,
                        ..
                    } => match key {
                        Key::U => {
                            self.settings.robots[self.ui_settings.selected_robot as usize]
                                .config
                                .pwm_override[0][0] = None
                        }
                        Key::J => {
                            self.settings.robots[self.ui_settings.selected_robot as usize]
                                .config
                                .pwm_override[0][1] = None
                        }
                        Key::I => {
                            self.settings.robots[self.ui_settings.selected_robot as usize]
                                .config
                                .pwm_override[1][0] = None
                        }
                        Key::K => {
                            self.settings.robots[self.ui_settings.selected_robot as usize]
                                .config
                                .pwm_override[1][1] = None
                        }
                        Key::O => {
                            self.settings.robots[self.ui_settings.selected_robot as usize]
                                .config
                                .pwm_override[2][0] = None
                        }
                        Key::L => {
                            self.settings.robots[self.ui_settings.selected_robot as usize]
                                .config
                                .pwm_override[2][1] = None
                        }
                        _ => {}
                    },
                    // Mouse buttons
                    Event::PointerButton {
                        button: PointerButton::Primary,
                        pressed,
                        pos,
                        ..
                    } => {
                        let pos2 = self.world_to_screen.inverse().map_point(*pos);
                        if *pressed {
                            if let Some(loc) = self.grid.node_nearest(pos2.x, pos2.y) {
                                self.send(GuiToServerMessage::SimulationCommand(
                                    ServerToSimulationMessage::Teleport(
                                        self.ui_settings.selected_robot,
                                        loc,
                                    ),
                                ))
                            }
                        }
                        let pos2 = self.robot_buttons_wts.inverse().map_point(*pos);
                        for (x, y, button) in [
                            (1.0, 8.0, RobotButton::NorthX),
                            (2.0, 9.0, RobotButton::EastA),
                            (2.0, 7.0, RobotButton::WestY),
                            (3.0, 8.0, RobotButton::SouthB),
                            (3.0, 4.3, RobotButton::LeftStart),
                            (3.0, 5.7, RobotButton::RightSelect),
                        ] {
                            let dist = ((pos2.x - x).powi(2) + (pos2.y - y).powi(2)).sqrt();
                            if dist < 0.4 {
                                self.send(GuiToServerMessage::SimulationCommand(
                                    ServerToSimulationMessage::RobotButton(
                                        self.ui_settings.selected_robot,
                                        (button, *pressed),
                                    ),
                                ))
                            }
                        }
                    }
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
