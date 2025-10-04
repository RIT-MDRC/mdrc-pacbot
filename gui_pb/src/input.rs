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
            for (key, (lin, ang), dir) in [
                (Key::W, (Vector2::new(0.0, scale), 0.0), Direction::Right),
                (Key::A, (Vector2::new(-scale, 0.0), 0.0), Direction::Up),
                (Key::D, (Vector2::new(scale, 0.0), 0.0), Direction::Down),
                (Key::S, (Vector2::new(0.0, -scale), 0.0), Direction::Left),
                (Key::Q, (Vector2::new(0.0, 0.0), scale), Direction::Stay),
                (Key::E, (Vector2::new(0.0, 0.0), -scale), Direction::Stay),
            ] {
                if self.settings.do_target_path == ShouldDoTargetPath::Yes
                    || self.settings.do_target_path == ShouldDoTargetPath::DoWhilePlayed
                        && !self.server_status.game_state.paused
                {
                    // instead of doing manual velocity, create a manual target path
                    if i.key_down(key) && dir != Direction::Stay {
                        // what is the earliest location in the current target path where we
                        // could start moving in this direction?
                        if let Some(curr_loc) = self.server_status.cv_location {
                            let next_loc = Point2::new(
                                curr_loc.x + dir.vector().0,
                                curr_loc.y + dir.vector().1,
                            );
                            if !self.grid.wall_at(&next_loc)
                                && !self.server_status.target_path.contains(&next_loc)
                            {
                                self.send(GuiToServerMessage::TargetLocation(next_loc));
                            } else if let Some(prev) = self
                                .server_status
                                .target_path
                                .iter()
                                .map(|loc| {
                                    Point2::new(loc.x + dir.vector().0, loc.y + dir.vector().1)
                                })
                                .filter(|loc| {
                                    !self.grid.wall_at(&loc)
                                        && !self.server_status.target_path.contains(&loc)
                                })
                                .next()
                            {
                                self.send(GuiToServerMessage::TargetLocation(prev));
                            }
                        }
                    }
                } else {
                    if i.key_down(key) {
                        target_vel.0 += lin;
                        target_vel.1 += ang;
                    }
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
                // Adjust scale with triggers
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

                let in_target_path_mode = self.settings.do_target_path == ShouldDoTargetPath::Yes
                    || (self.settings.do_target_path == ShouldDoTargetPath::DoWhilePlayed
                        && !self.server_status.game_state.paused);

                if in_target_path_mode {
                    // Handle left stick for target path directions
                    let deadzone = 0.5;
                    let left_x = gp
                        .axis_data(Axis::LeftStickX)
                        .map(|a| a.value())
                        .unwrap_or(0.0);
                    let left_y = gp
                        .axis_data(Axis::LeftStickY)
                        .map(|a| a.value())
                        .unwrap_or(0.0);

                    let dir = if left_x.abs() > deadzone || left_y.abs() > deadzone {
                        if left_x.abs() > left_y.abs() {
                            if left_x > deadzone {
                                Direction::Right
                            } else if left_x < -deadzone {
                                Direction::Left
                            } else {
                                Direction::Stay
                            }
                        } else {
                            if left_y > deadzone {
                                Direction::Up
                            } else if left_y < -deadzone {
                                Direction::Down
                            } else {
                                Direction::Stay
                            }
                        }
                    } else {
                        Direction::Stay
                    };

                    if dir != Direction::Stay {
                        if let Some(curr_loc) = self.server_status.cv_location {
                            let next_loc = Point2::new(
                                curr_loc.x + dir.vector().0,
                                curr_loc.y + dir.vector().1,
                            );
                            if !self.grid.wall_at(&next_loc)
                                && !self.server_status.target_path.contains(&next_loc)
                            {
                                self.send(GuiToServerMessage::TargetLocation(next_loc));
                            } else if let Some(prev) = self
                                .server_status
                                .target_path
                                .iter()
                                .map(|loc| {
                                    Point2::new(loc.x + dir.vector().0, loc.y + dir.vector().1)
                                })
                                .filter(|loc| {
                                    !self.grid.wall_at(loc)
                                        && !self.server_status.target_path.contains(loc)
                                })
                                .next()
                            {
                                self.send(GuiToServerMessage::TargetLocation(prev));
                            }
                        }
                    }
                } else {
                    // Existing velocity handling for gamepad
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
                            target_vel.1 += -right_x.value() * scale;
                        }
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
                            Key::Y => {
                                self.settings.do_target_path =
                                    if self.settings.do_target_path == ShouldDoTargetPath::Yes {
                                        ShouldDoTargetPath::DoWhilePlayed
                                    } else {
                                        ShouldDoTargetPath::Yes
                                    }
                            }
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
                                self.settings.cv_location_source = CvLocationSource::Simulation
                            }
                            Key::T => {
                                if let Some(pos) = self.pointer_pos {
                                    let pos = self.world_to_screen.inverse().map_point(pos);
                                    let p = Point2::new(pos.x.round() as i8, pos.y.round() as i8);
                                    // if !self.grid.wall_at(&p) {
                                    self.settings.cv_location_source =
                                        CvLocationSource::Constant(Some(p))
                                    // }
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
                            self.send(GuiToServerMessage::SimulationCommand(
                                ServerToSimulationMessage::Teleport(
                                    self.ui_settings.selected_robot,
                                    Point2::new(pos2.x.round() as i8, pos2.y.round() as i8),
                                ),
                            ))
                            // if let Some(loc) = self.grid.node_nearest(pos2.x, pos2.y) {
                            //     self.send(GuiToServerMessage::SimulationCommand(
                            //         ServerToSimulationMessage::Teleport(
                            //             self.ui_settings.selected_robot,
                            //             loc,
                            //         ),
                            //     ))
                            // }
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
