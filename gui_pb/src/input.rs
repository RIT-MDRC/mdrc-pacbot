use crate::drawing::motors::MotorStatusGraphFrames;
use crate::drawing::settings::VelocityControlAngleBehavior;
use crate::App;
use core_pb::grid::standard_grid::StandardGrid;
use core_pb::messages::settings::{CvLocationSource, ShouldDoTargetPath, StrategyChoice};
use core_pb::messages::{
    GameServerCommand, GuiToServerMessage, NetworkStatus, RobotButton, ServerToSimulationMessage,
    VelocityControl,
};
use core_pb::pacbot_rs::location::Direction;
use core_pb::robot_definition::RobotDefinition;
use core_pb::threaded_websocket::TextOrT;
use eframe::egui;
use eframe::egui::{Event, Key, PointerButton};
use gilrs::{Axis, Button, EventType};
use log::info;
use nalgebra::{Point2, Vector2};

impl App {
    pub fn set_target_vel(&mut self, target_vel: VelocityControl) {
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
            self.set_target_vel(VelocityControl::None);
            return;
        }

        ctx.input(|i| {
            let mut target_vel = (Vector2::new(0.0, 0.0), 0.0);
            let mut scale = self.settings.target_speed;
            if i.modifiers.shift {
                scale *= 1.5;
            }
            if i.modifiers.ctrl || i.modifiers.command {
                scale /= 3.0;
            }
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
            while let Some(gilrs::Event { event, .. }) = self.gilrs.next_event() {
                match event {
                    EventType::ButtonPressed(b, _) => match b {
                        Button::East => {
                            self.ui_settings.angle_behavior = VelocityControlAngleBehavior::Free
                        }
                        Button::West => {
                            self.ui_settings.angle_behavior = VelocityControlAngleBehavior::Locked(
                                self.server_status.robots[self.ui_settings.selected_robot as usize]
                                    .clone()
                                    .imu_angle
                                    .unwrap_or(0.0),
                            )
                        }
                        Button::North => {
                            self.ui_settings.angle_behavior =
                                VelocityControlAngleBehavior::FaceForward
                        }

                        Button::South => {
                            self.ui_settings.angle_behavior =
                                VelocityControlAngleBehavior::AssistedDriving
                        }
                        _ => {}
                    },
                    EventType::Connected => {
                        info!("Gamepad connected")
                    }
                    EventType::Disconnected => {
                        info!("Gamepad disconnected")
                    }
                    _ => {}
                }
            }
            if let Some((_, gp)) = self.gilrs.gamepads().next() {
                if let Some(t) = gp.button_data(Button::LeftTrigger2) {
                    if t.is_pressed() {
                        scale /= 3.0;
                    }
                }
                if let Some(t) = gp.button_data(Button::RightTrigger2) {
                    if t.is_pressed() {
                        scale *= 1.5;
                    }
                }
                if let Some(left_x) = gp.axis_data(Axis::LeftStickX) {
                    if left_x.value() != 0.0 {
                        target_vel.0 += Vector2::new(left_x.value() * scale, 0.0);
                    }
                }
                if let Some(left_y) = gp.axis_data(Axis::LeftStickY) {
                    if left_y.value() != 0.0 {
                        target_vel.0 += Vector2::new(0.0, left_y.value() * scale);
                    }
                }
                if let Some(right_x) = gp.axis_data(Axis::RightStickX) {
                    if right_x.value() != 0.0 {
                        target_vel.1 += -right_x.value() * scale
                    }
                }
            }
            let (lin, ang) = target_vel;
            let v = match self.ui_settings.angle_behavior {
                VelocityControlAngleBehavior::Free => VelocityControl::LinVelAngVel(lin, ang),
                VelocityControlAngleBehavior::Locked(angle) => {
                    if ang.abs() < 0.01 {
                        VelocityControl::LinVelFixedAng(lin, angle)
                    } else {
                        self.ui_settings.angle_behavior = VelocityControlAngleBehavior::Locked(
                            self.server_status.robots[self.ui_settings.selected_robot as usize]
                                .clone()
                                .imu_angle
                                .unwrap_or(0.0),
                        );
                        VelocityControl::LinVelAngVel(lin, ang)
                    }
                }
                VelocityControlAngleBehavior::FaceForward => {
                    VelocityControl::LinVelFaceForward(lin)
                }
                VelocityControlAngleBehavior::AssistedDriving => {
                    VelocityControl::AssistedDriving(lin)
                }
            };
            self.set_target_vel(v);

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
                                if !self.distance_recording.0 {
                                    self.distance_recording.0 = true;
                                    self.distance_recording.1 += 1;
                                    info!("Start recording segment {}", self.distance_recording.1);
                                }
                                // if self.server_status.game_state.paused {
                                //     self.send(GuiToServerMessage::GameServerCommand(
                                //         GameServerCommand::Unpause,
                                //     ))
                                // } else {
                                //     self.send(GuiToServerMessage::GameServerCommand(
                                //         GameServerCommand::Pause,
                                //     ))
                                // }
                            }
                            Key::Escape => {
                                if self.distance_recording.0 {
                                    self.distance_recording.0 = false;
                                    info!("End recording segment {}", self.distance_recording.1);
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
                            Key::M => self.settings.standard_grid = StandardGrid::Open,
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
