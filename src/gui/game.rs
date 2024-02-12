use crate::gui::colors::{
    GHOST_BLUE_COLOR, GHOST_ORANGE_COLOR, GHOST_PINK_COLOR, GHOST_RED_COLOR,
    PACMAN_DISTANCE_SENSOR_RAY_COLOR, PELLET_COLOR, SUPER_PELLET_COLOR, WALL_COLOR,
};
use crate::gui::transforms::Transform;
use crate::gui::{PacbotWidget, TabViewer};
use crate::PacmanGameState;
use bevy::prelude::*;
use eframe::egui::{Painter, Pos2, Rect, RichText, Rounding, Stroke};
use pacbot_rs::game_engine::GameEngine;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

/// Stores state needed to render game state information
#[derive(Clone, Serialize, Deserialize)]
pub struct PacmanStateRenderInfo {
    /// Current game state
    pub pacman_state: GameEngine,
}

#[derive(Clone)]
pub struct GameWidget {
    pub state: Arc<RwLock<PacmanStateRenderInfo>>,
}

impl PacbotWidget for GameWidget {
    fn display_name(&self) -> &'static str {
        "Game (Click to Reset)"
    }

    fn button_text(&self) -> RichText {
        RichText::new(format!(
            "{} {} {} {} {} {}",
            egui_phosphor::regular::HEART,
            self.state
                .read()
                .unwrap()
                .pacman_state
                .get_state()
                .curr_lives,
            egui_phosphor::regular::TROPHY,
            self.state
                .read()
                .unwrap()
                .pacman_state
                .get_state()
                .curr_score,
            egui_phosphor::regular::TIMER,
            self.state
                .read()
                .unwrap()
                .pacman_state
                .get_state()
                .curr_ticks
        ))
    }
}

pub fn update_game(
    mut pacman_state: ResMut<PacmanGameState>,
    mut last_update: Local<Option<Instant>>,
) {
    let last_update = last_update.get_or_insert(Instant::now());
    if last_update.elapsed() > Duration::from_secs_f32(1.0 / 2.5) {
        *last_update = Instant::now();
        pacman_state.0.force_step()
    }
}

impl<'a> TabViewer<'a> {
    pub(super) fn draw_grid(&mut self, world_to_screen: &Transform, painter: &Painter) {
        // paint the solid walls
        for wall in self.grid.walls() {
            let (p1, p2) = world_to_screen.map_wall(wall);
            painter.rect(
                Rect::from_two_pos(p1, p2),
                Rounding::ZERO,
                WALL_COLOR,
                Stroke::new(1.0, WALL_COLOR),
            );
        }

        // make sure the area outside the soft boundary is not drawn on
        for (p1, p2) in self.selected_grid.0.get_outside_soft_boundaries() {
            painter.rect(
                Rect::from_two_pos(world_to_screen.map_point(p1), world_to_screen.map_point(p2)),
                Rounding::ZERO,
                self.background_color,
                Stroke::new(1.0, self.background_color),
            );
        }
    }

    pub(super) fn draw_pacman_state(&mut self, world_to_screen: &Transform, painter: &Painter) {
        let pacman_state = &self.pacman_state.0;

        // ghosts
        for ghost in &pacman_state.get_state().ghosts {
            let ghost = ghost.read().unwrap();
            painter.circle_filled(
                world_to_screen.map_point(Pos2::new(ghost.loc.row as f32, ghost.loc.col as f32)),
                world_to_screen.map_dist(0.45),
                match ghost.color {
                    pacbot_rs::ghost_state::RED => GHOST_RED_COLOR,
                    pacbot_rs::ghost_state::PINK => GHOST_PINK_COLOR,
                    pacbot_rs::ghost_state::ORANGE => GHOST_ORANGE_COLOR,
                    pacbot_rs::ghost_state::CYAN => GHOST_BLUE_COLOR,
                    _ => panic!("Invalid ghost color!"),
                },
            )
        }

        // pellets
        for row in 0..32 {
            for col in 0..32 {
                if pacman_state.get_state().pellet_at((row, col)) {
                    let super_pellet = ((row == 3) || (row == 23)) && ((col == 1) || (col == 26));
                    if super_pellet {
                        painter.circle_filled(
                            world_to_screen.map_point(Pos2::new(row as f32, col as f32)),
                            6.0,
                            SUPER_PELLET_COLOR,
                        )
                    } else {
                        painter.circle_filled(
                            world_to_screen.map_point(Pos2::new(row as f32, col as f32)),
                            3.0,
                            PELLET_COLOR,
                        )
                    }
                }
            }
        }

        // pacman
        painter.circle_filled(
            world_to_screen.map_point(Pos2::new(
                pacman_state.get_state().pacman_loc.row as f32,
                pacman_state.get_state().pacman_loc.col as f32,
            )),
            world_to_screen.map_dist(0.3),
            PACMAN_DISTANCE_SENSOR_RAY_COLOR,
        )
    }
}
