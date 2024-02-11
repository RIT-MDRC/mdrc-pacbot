use crate::grid::{facing_direction, ComputedGrid, IntLocation};
use crate::gui::colors::{
    GHOST_BLUE_COLOR, GHOST_ORANGE_COLOR, GHOST_PINK_COLOR, GHOST_RED_COLOR,
    PACMAN_DISTANCE_SENSOR_RAY_COLOR, PELLET_COLOR, SUPER_PELLET_COLOR, WALL_COLOR,
};
use crate::gui::transforms::Transform;
use crate::gui::{PacbotWidget, TabViewer};
use crate::UserSettings;
use eframe::egui::{Painter, Pos2, Rect, RichText, Rounding, Stroke};
use pacbot_rs::game_engine::GameEngine;
use pacbot_rs::location::LocationState;
use pacbot_rs::variables::PACMAN_SPAWN_LOC;
use serde::{Deserialize, Serialize};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, RwLock};

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

pub(super) fn run_game(
    pacman_render: Arc<RwLock<PacmanStateRenderInfo>>,
    location_receive: Receiver<IntLocation>,
    replay_send: Sender<()>,
) {
    let mut previous_pacman_location = IntLocation::new(PACMAN_SPAWN_LOC.row, PACMAN_SPAWN_LOC.col);
    {
        let pacman_render = pacman_render.clone();
        std::thread::spawn(move || {
            // fetch updated pacbot position
            while let Ok(pacbot_location) = location_receive.recv() {
                let mut state = pacman_render.write().unwrap();
                state.pacman_state.set_pacman_location(LocationState {
                    col: pacbot_location.col,
                    row: pacbot_location.row,
                    dir: facing_direction(&previous_pacman_location, &pacbot_location) as u8,
                });
                previous_pacman_location = pacbot_location;
                drop(state);
            }
        });
    }

    loop {
        // {} block to make sure `game` goes out of scope and the RwLockWriteGuard is released
        {
            let mut state = pacman_render.write().unwrap();

            // step the game
            if !state.pacman_state.is_paused() {
                state.pacman_state.force_step();
                replay_send.send(()).unwrap()
            }
        }

        // Sleep
        std::thread::sleep(std::time::Duration::from_secs_f32(1.0 / 2.5));
    }
}

impl<'a> TabViewer<'a> {
    pub(super) fn draw_grid(
        &mut self,
        world_to_screen: &Transform,
        painter: &Painter,
        grid: &ComputedGrid,
        settings: &UserSettings,
    ) {
        // paint the solid walls
        for wall in grid.walls() {
            let (p1, p2) = world_to_screen.map_wall(wall);
            painter.rect(
                Rect::from_two_pos(p1, p2),
                Rounding::ZERO,
                WALL_COLOR,
                Stroke::new(1.0, WALL_COLOR),
            );
        }

        // make sure the area outside the soft boundary is not drawn on
        for (p1, p2) in settings.standard_grid.get_outside_soft_boundaries() {
            painter.rect(
                Rect::from_two_pos(world_to_screen.map_point(p1), world_to_screen.map_point(p2)),
                Rounding::ZERO,
                self.background_color,
                Stroke::new(1.0, self.background_color),
            );
        }
    }

    pub(super) fn draw_pacman_state(
        &mut self,
        world_to_screen: &Transform,
        painter: &Painter,
        pacman_state: &GameEngine,
    ) {
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
