use crate::grid::{facing_direction, PLocation};
use crate::gui::colors::{
    GHOST_BLUE_COLOR, GHOST_ORANGE_COLOR, GHOST_PINK_COLOR, GHOST_RED_COLOR, PELLET_COLOR,
    SUPER_PELLET_COLOR, WALL_COLOR,
};
use crate::gui::transforms::Transform;
use crate::gui::App;
use eframe::egui;
use eframe::egui::{Painter, Pos2, Rect, Rounding, Stroke};
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

pub(super) fn run_game(
    pacman_render: Arc<RwLock<PacmanStateRenderInfo>>,
    location_receive: Receiver<PLocation>,
    replay_send: Sender<()>,
) {
    let mut previous_pacman_location = PLocation::new(PACMAN_SPAWN_LOC.row, PACMAN_SPAWN_LOC.col);

    loop {
        // {} block to make sure `game` goes out of scope and the RwLockWriteGuard is released
        {
            let mut state = pacman_render.write().unwrap();

            // fetch updated pacbot position
            while let Ok(pacbot_location) = location_receive.try_recv() {
                state.pacman_state.set_pacman_location(LocationState {
                    col: pacbot_location.col,
                    row: pacbot_location.row,
                    dir: facing_direction(&previous_pacman_location, &pacbot_location) as u8,
                });
                previous_pacman_location = pacbot_location;
            }

            // step the game
            if !state.pacman_state.is_paused() {
                state.pacman_state.step();
                replay_send.send(()).unwrap()
            }
        }

        // Sleep
        std::thread::sleep(std::time::Duration::from_secs_f32(1.0 / 2.5));
    }
}

impl App {
    pub(super) fn draw_grid(
        &mut self,
        ctx: &egui::Context,
        world_to_screen: &Transform,
        painter: &Painter,
    ) {
        self.pointer_pos = match ctx.pointer_latest_pos() {
            None => "".to_string(),
            Some(pos) => {
                let pos = world_to_screen.inverse().map_point(pos);
                format!("({:.1}, {:.1})", pos.x, pos.y)
            }
        };

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
        for (p1, p2) in self.selected_grid.get_outside_soft_boundaries() {
            painter.rect(
                Rect::from_two_pos(world_to_screen.map_point(p1), world_to_screen.map_point(p2)),
                Rounding::ZERO,
                ctx.style().visuals.panel_fill,
                Stroke::new(1.0, ctx.style().visuals.panel_fill),
            );
        }
    }

    pub(super) fn draw_pacman_state(
        &mut self,
        ctx: &egui::Context,
        world_to_screen: &Transform,
        painter: &Painter,
    ) {
        let pacman_state_info = self.pacman_render.read().unwrap();
        let pacman_state = &pacman_state_info.pacman_state;

        egui::Window::new("Pacman").show(ctx, |ui| {
            ui.label(format!("Score: {}", pacman_state.get_state().curr_score));
            ui.label(format!("Lives: {}", pacman_state.get_state().curr_lives));
            ui.label(format!("Frame: {}", pacman_state.get_state().curr_ticks));
        });

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
    }
}
