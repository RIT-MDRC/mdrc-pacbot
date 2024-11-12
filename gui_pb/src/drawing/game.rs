use core::f32;

use crate::colors::*;
use crate::App;
use core_pb::constants::GU_PER_M;
use core_pb::grid::standard_grid::StandardGrid;
use core_pb::names::RobotName;
use core_pb::pacbot_rs::ghost_state::GhostColor;
use core_pb::region_localization::{get_all_regions, get_possible_regions};
use core_pb::robot_definition::RobotDefinition;
use core_pb::util::TRANSLUCENT_YELLOW_COLOR;
use eframe::egui::{Color32, Painter, Pos2, Rect, Rounding, Stroke};
use nalgebra::{Point2, Rotation2};

pub fn draw_grid(app: &mut App, painter: &Painter) {
    let wts = app.world_to_screen;

    // paint the solid walls
    for wall in app.grid.walls() {
        let (p1, p2) = wts.map_wall(wall);
        painter.rect(
            Rect::from_two_pos(p1, p2),
            Rounding::ZERO,
            WALL_COLOR,
            Stroke::new(1.0, WALL_COLOR),
        );
    }

    // make sure the area outside the soft boundary is not drawn on
    for (p1, p2) in app.settings.standard_grid.get_outside_soft_boundaries() {
        painter.rect(
            Rect::from_two_pos(wts.map_point2(p1), wts.map_point2(p2)),
            Rounding::ZERO,
            app.background_color,
            Stroke::new(1.0, app.background_color),
        );
    }

    // origin marker
    painter.circle_filled(wts.map_point(Pos2::new(-0.5, -0.5)), 2.0, Color32::RED);
}

pub fn draw_game(app: &mut App, painter: &Painter) {
    let wts = app.world_to_screen;
    let pacman_state = &app.server_status.game_state;

    // sim robot positions
    for name in RobotName::get_all() {
        // estimated pos
        if let Some(orig_estimated_location) =
            app.server_status.robots[name as usize].estimated_location
        {
            let estimated_location: Pos2 = wts.map_point(Pos2::new(
                orig_estimated_location.x,
                orig_estimated_location.y,
            ));
            let angle = app.server_status.robots[name as usize]
                .imu_angle
                .clone()
                .unwrap_or(0.0);
            painter.circle(
                estimated_location,
                wts.map_dist(name.robot().radius),
                Color32::TRANSPARENT,
                Stroke::new(1.0, Color32::GREEN),
            );
            for (i, sensor) in app.server_status.robots[name as usize]
                .distance_sensors
                .iter()
                .enumerate()
            {
                if let Ok(Some(distance)) = sensor {
                    painter.line_segment(
                        [
                            wts.map_point(Pos2::new(
                                orig_estimated_location.x
                                    + name.robot().radius
                                        * f32::cos(angle + (i as f32) * f32::consts::FRAC_PI_2),
                                orig_estimated_location.y
                                    + name.robot().radius
                                        * f32::sin(angle + (i as f32) * f32::consts::FRAC_PI_2),
                            )),
                            wts.map_point(Pos2::new(
                                orig_estimated_location.x
                                    + (distance + name.robot().radius)
                                        * f32::cos(angle + (i as f32) * f32::consts::FRAC_PI_2),
                                orig_estimated_location.y
                                    + (distance + name.robot().radius)
                                        * f32::sin(angle + (i as f32) * f32::consts::FRAC_PI_2),
                            )),
                        ],
                        Stroke::new(1.0, Color32::GREEN),
                    );
                } else {
                    let distance = RobotDefinition::new(name).sensor_distance * GU_PER_M;
                    painter.line_segment(
                        [
                            wts.map_point(Pos2::new(
                                orig_estimated_location.x
                                    + name.robot().radius
                                        * f32::cos(angle + (i as f32) * f32::consts::FRAC_PI_2),
                                orig_estimated_location.y
                                    + name.robot().radius
                                        * f32::sin(angle + (i as f32) * f32::consts::FRAC_PI_2),
                            )),
                            wts.map_point(Pos2::new(
                                orig_estimated_location.x
                                    + (distance + name.robot().radius)
                                        * f32::cos(angle + (i as f32) * f32::consts::FRAC_PI_2),
                                orig_estimated_location.y
                                    + (distance + name.robot().radius)
                                        * f32::sin(angle + (i as f32) * f32::consts::FRAC_PI_2),
                            )),
                        ],
                        Stroke::new(1.0, Color32::RED),
                    );
                }
            }
        }
        if let Some(pos) = app.server_status.robots[name as usize].sim_position {
            let center = wts.map_point(Pos2::new(pos.0.x, pos.0.y));
            painter.circle_filled(
                center,
                wts.map_dist(name.robot().radius),
                if name == app.ui_settings.selected_robot {
                    Color32::YELLOW
                } else {
                    TRANSLUCENT_YELLOW_COLOR
                },
            );
            // draw a line to show the direction the robot is facing
            // shortcut since these values are already pre-computed in the rotation matrix
            let rot_cos = pos.1.matrix()[(0, 0)];
            let rot_sin = pos.1.matrix()[(1, 0)];
            painter.line_segment(
                [
                    center,
                    wts.map_point(Pos2::new(
                        pos.0.x + rot_cos * name.robot().radius,
                        pos.0.y + rot_sin * name.robot().radius,
                    )),
                ],
                Stroke::new(1.0, Color32::BLACK),
            );

            let estimated_location = app.server_status.robots[name as usize].estimated_location;
            if let Some(orig_estimated_location) = estimated_location {
                let estimated_location: Pos2 = wts.map_point(Pos2::new(
                    orig_estimated_location.x,
                    orig_estimated_location.y,
                ));
                // painter.circle(
                //     estimated_location,
                //     wts.map_dist(name.robot().radius),
                //     Color32::TRANSPARENT,
                //     Stroke::new(1.0, Color32::GREEN),
                // );
                if let Ok(angle) = app.server_status.robots[name as usize].imu_angle {
                    let rot_cos = Rotation2::new(angle).matrix()[(0, 0)];
                    let rot_sin = Rotation2::new(angle).matrix()[(1, 0)];
                    painter.line_segment(
                        [
                            estimated_location,
                            wts.map_point(Pos2::new(
                                orig_estimated_location.x + rot_cos * name.robot().radius,
                                orig_estimated_location.y + rot_sin * name.robot().radius,
                            )),
                        ],
                        Stroke::new(1.0, Color32::GREEN),
                    );
                }
            }
        }
    }

    // pacman
    if let Some(cv_loc) = &app.server_status.cv_location {
        painter.circle_filled(
            wts.map_point(Pos2::new(cv_loc.x as f32, cv_loc.y as f32)),
            wts.map_dist(0.3),
            Color32::GREEN,
        );
    }

    // target path
    for i in 0..app.server_status.target_path.len() {
        let first = wts.map_point(if i == 0 {
            let p = app.server_status.cv_location.unwrap_or(Point2::new(0, 0));
            Pos2::new(p.x as f32, p.y as f32)
        } else {
            Pos2::new(
                app.server_status.target_path[i - 1].x as f32,
                app.server_status.target_path[i - 1].y as f32,
            )
        });
        let second = wts.map_point(Pos2::new(
            app.server_status.target_path[i].x as f32,
            app.server_status.target_path[i].y as f32,
        ));
        painter.line_segment(
            [first, second],
            Stroke::new(1.0, PACMAN_AI_TARGET_LOCATION_COLOR),
        );
    }

    // draw possible region boundaries
    for region in get_possible_regions(
        *app.grid.grid(),
        app.server_status.robots[app.ui_settings.selected_robot as usize]
            .distance_sensors
            .clone()
            .map(|x| {
                x.map_err(|_| ()).map(|x| {
                    x.unwrap_or(
                        RobotDefinition::new(app.ui_settings.selected_robot).sensor_distance,
                    )
                })
            }),
        RobotDefinition::new(app.ui_settings.selected_robot).sensor_distance,
        RobotDefinition::new(app.ui_settings.selected_robot).radius,
    ) {
        painter.rect(
            Rect::from_two_pos(
                wts.map_point2(region.low_xy.map(|x| x as f32)),
                wts.map_point2(region.high_xy.map(|x| x as f32)),
            ),
            Rounding::ZERO,
            Color32::from_rgba_unmultiplied(100, 0, 0, 25),
            Stroke::new(1.0, Color32::DARK_GRAY),
        );
    }

    if app.settings.standard_grid != StandardGrid::Pacman {
        return;
    }

    // ghosts
    for ghost in &pacman_state.ghosts {
        painter.circle_filled(
            wts.map_point(Pos2::new(ghost.loc.row as f32, ghost.loc.col as f32)),
            wts.map_dist(0.45),
            match ghost.color {
                GhostColor::Red => GHOST_RED_COLOR,
                GhostColor::Pink => GHOST_PINK_COLOR,
                GhostColor::Cyan => GHOST_BLUE_COLOR,
                GhostColor::Orange => GHOST_ORANGE_COLOR,
            },
        );
        if ghost.fright_steps > 0 {
            painter.circle_stroke(
                wts.map_point(Pos2::new(ghost.loc.row as f32, ghost.loc.col as f32)),
                wts.map_dist(0.45),
                Stroke::new(2.0, GHOST_FRIGHTENED_COLOR),
            );
        }
    }

    // pellets
    for row in 0..32 {
        for col in 0..32 {
            if pacman_state.pellet_at((row, col)) {
                let super_pellet = ((row == 3) || (row == 23)) && ((col == 1) || (col == 26));
                if super_pellet {
                    painter.circle_filled(
                        wts.map_point(Pos2::new(row as f32, col as f32)),
                        6.0,
                        SUPER_PELLET_COLOR,
                    );
                } else {
                    painter.circle_filled(
                        wts.map_point(Pos2::new(row as f32, col as f32)),
                        3.0,
                        PELLET_COLOR,
                    );
                }
            }
        }
    }
}
