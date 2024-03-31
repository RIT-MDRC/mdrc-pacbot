use crate::gui::transforms::Transform;
use crate::gui::{PacbotWidget, Tab, TabViewer};
use crate::robot::Robot;
use bevy_egui::egui::RichText;
use eframe::egui::{Align2, Color32, FontId, Pos2, Stroke, Ui};
use egui_phosphor::regular;
use rapier2d::math::Rotation;
use rapier2d::na::Point2;
use std::f32::consts::PI;

#[derive(Copy, Clone, Debug, Default)]
pub struct RobotWidget {}

impl PacbotWidget for RobotWidget {
    fn display_name(&self) -> &'static str {
        "Robot"
    }

    fn button_text(&self) -> RichText {
        RichText::new(regular::ROBOT.to_string())
    }

    fn tab(&self) -> Tab {
        Tab::Robot
    }
}

impl<'a> TabViewer<'a> {
    pub(super) fn draw_robot(&mut self, ui: &mut Ui) {
        if let Some(pf_pos) = self.phys_info.pf_pos {
            let robot = Robot::default();
            let motor_max_speed = 40.0;

            let rect = ui.max_rect();
            let world_to_screen = Transform::new_letterboxed(
                Pos2::new(-4.0, -4.0),
                Pos2::new(4.0, 4.0),
                Pos2::new(rect.top(), rect.left()),
                Pos2::new(rect.bottom(), rect.right()),
            );
            let painter = ui.painter_at(rect);

            // robot outline
            painter.circle_stroke(
                world_to_screen.map_point(Pos2::new(0.0, 0.0)),
                world_to_screen.map_dist(robot.collider_radius),
                Stroke::new(1.0, Color32::WHITE),
            );

            // distance sensors
            for (i, (a, b)) in self.phys_info.pf_pos_rays.iter().enumerate() {
                // sensor id
                painter.text(
                    world_to_screen.map_point(Pos2::new(
                        (a.x - pf_pos.translation.x) * 0.8,
                        (a.y - pf_pos.translation.y) * 0.8,
                    )),
                    Align2::CENTER_CENTER,
                    format!("{i}"),
                    FontId::default(),
                    Color32::GREEN,
                );
                // sensor line
                painter.line_segment(
                    [
                        world_to_screen.map_point(Pos2::new(
                            a.x - pf_pos.translation.x,
                            a.y - pf_pos.translation.y,
                        )),
                        world_to_screen.map_point(Pos2::new(
                            b.x - pf_pos.translation.x,
                            b.y - pf_pos.translation.y,
                        )),
                    ],
                    Stroke::new(1.0, Color32::GREEN),
                );
                // if hitting a wall, show contact point
                if self.sensors.distance_sensors[i] != 255 {
                    painter.circle_filled(
                        world_to_screen.map_point(Pos2::new(
                            b.x - pf_pos.translation.x,
                            b.y - pf_pos.translation.y,
                        )),
                        4.0,
                        Color32::GREEN,
                    )
                }
            }

            // motors
            for (i, angle) in [0.0, 2.0 * PI / 3.0, 4.0 * PI / 3.0].iter().enumerate() {
                let pos = Rotation::new(angle + pf_pos.rotation.angle())
                    .transform_point(&Point2::new(1.0, 0.0));
                // motor id
                painter.text(
                    world_to_screen.map_point(Pos2::new(pos.x * 0.35, pos.y * 0.35)),
                    Align2::CENTER_CENTER,
                    format!("{i}"),
                    FontId::default(),
                    Color32::RED,
                );
                // motor force origin point
                painter.circle_filled(
                    world_to_screen.map_point(Pos2::new(
                        pos.x * robot.collider_radius,
                        pos.y * robot.collider_radius,
                    )),
                    3.0,
                    Color32::RED,
                );
                // motor force indicator
                let motor_speed = self.last_motor_commands.motors[i];
                let distance = 3.0 * motor_speed / motor_max_speed;
                let other_pos = Rotation::new(angle + pf_pos.rotation.angle() + PI / -2.0)
                    .transform_point(&Point2::new(distance, 0.0));
                painter.line_segment(
                    [
                        world_to_screen.map_point(Pos2::new(
                            pos.x * robot.collider_radius,
                            pos.y * robot.collider_radius,
                        )),
                        world_to_screen.map_point(Pos2::new(
                            other_pos.x + pos.x * robot.collider_radius,
                            other_pos.y + pos.y * robot.collider_radius,
                        )),
                    ],
                    Stroke::new(1.0, Color32::RED),
                );
            }

            // ui.label("test");
        }
    }
}
