use crate::colors::*;
use crate::AppData;
use eframe::egui::{Color32, Painter, Pos2, Rect, Rounding, Stroke};

pub fn draw_grid(app: &mut AppData, painter: &Painter) {
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
    for (p1, p2) in app.settings.grid.get_outside_soft_boundaries() {
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

pub fn draw_game(app: &mut AppData, painter: &Painter) {
    let wts = app.world_to_screen;
    let pacman_state = &app.game;

    // ghosts
    for ghost in &pacman_state.ghosts {
        painter.circle_filled(
            wts.map_point(Pos2::new(ghost.loc.row as f32, ghost.loc.col as f32)),
            wts.map_dist(0.45),
            match ghost.color {
                pacbot_rs::ghost_state::RED => GHOST_RED_COLOR,
                pacbot_rs::ghost_state::PINK => GHOST_PINK_COLOR,
                pacbot_rs::ghost_state::ORANGE => GHOST_ORANGE_COLOR,
                pacbot_rs::ghost_state::CYAN => GHOST_BLUE_COLOR,
                _ => panic!("Invalid ghost color!"),
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
}
