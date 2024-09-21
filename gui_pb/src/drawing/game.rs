use core::f32;

use crate::colors::*;
use crate::App;
use core_pb::grid::standard_grid::StandardGrid;
use core_pb::names::RobotName;
use core_pb::pacbot_rs::ghost_state::GhostColor;
use core_pb::util::TRANSLUCENT_YELLOW_COLOR;
use eframe::egui::{Color32, Painter, Pos2, Rect, Rounding, Stroke};

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
            let rotation = pos.1.angle();
            painter.line_segment(
                [
                    center,
                    wts.map_point(Pos2::new(
                        pos.0.x + rotation.cos() * name.robot().radius,
                        pos.0.y + rotation.sin() * name.robot().radius,
                    )),
                ],
                Stroke::new(1.0, Color32::BLACK),
            );

            for (i, sensor) in app.server_status.robots[name as usize]
                .distance_sensors
                .into_iter()
                .enumerate()
            {
                if let Ok(Some(distance)) = sensor {
                    painter.line_segment(
                        [
                            wts.map_point(Pos2::new(
                                pos.0.x
                                    + name.robot().radius
                                        * f32::cos(
                                            pos.1.angle() + (i as f32) * f32::consts::FRAC_PI_2,
                                        ),
                                pos.0.y
                                    + name.robot().radius
                                        * f32::sin(
                                            pos.1.angle() + (i as f32) * f32::consts::FRAC_PI_2,
                                        ),
                            )),
                            wts.map_point(Pos2::new(
                                pos.0.x
                                    + (distance + name.robot().radius)
                                        * f32::cos(
                                            pos.1.angle() + (i as f32) * f32::consts::FRAC_PI_2,
                                        ),
                                pos.0.y
                                    + (distance + name.robot().radius)
                                        * f32::sin(
                                            pos.1.angle() + (i as f32) * f32::consts::FRAC_PI_2,
                                        ),
                            )),
                        ],
                        Stroke::new(1.0, Color32::GREEN),
                    );
                }
            }
        }
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

    // pacman
    painter.circle_filled(
        wts.map_point(Pos2::new(
            pacman_state.pacman_loc.row as f32,
            pacman_state.pacman_loc.col as f32,
        )),
        wts.map_dist(0.3),
        PACMAN_DISTANCE_SENSOR_RAY_COLOR,
    );

    // target path
    if let Some(target) = app.server_status.target_path.first() {
        painter.line_segment(
            [
                wts.map_point(Pos2::new(
                    pacman_state.pacman_loc.row as f32,
                    pacman_state.pacman_loc.col as f32,
                )),
                wts.map_point(Pos2::new(target.x as f32, target.y as f32)),
            ],
            Stroke::new(1.0, PACMAN_AI_TARGET_LOCATION_COLOR),
        );
    }
}
