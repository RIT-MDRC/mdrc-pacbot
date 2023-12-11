use crate::agent_setup::PacmanAgentSetup;
use crate::game_state::{GhostType, PacmanState};
use crate::grid::facing_direction;
use crate::gui::colors::{
    GHOST_BLUE_COLOR, GHOST_ORANGE_COLOR, GHOST_PINK_COLOR, GHOST_RED_COLOR, PELLET_COLOR,
    SUPER_PELLET_COLOR, WALL_COLOR,
};
use crate::gui::transforms::Transform;
use crate::gui::App;
use eframe::egui;
use eframe::egui::{Painter, Pos2, Rect, Rounding, Stroke};
use rand::prelude::ThreadRng;
use rapier2d::na::Point2;
use serde::{Deserialize, Serialize};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, RwLock};

/// Stores state needed to render game state information
#[derive(Clone, Serialize, Deserialize)]
pub struct PacmanStateRenderInfo {
    /// Initial positions of Pacman, ghosts, etc.
    pub agent_setup: PacmanAgentSetup,
    /// Current game state
    pub pacman_state: PacmanState,
}

pub(super) fn run_game(
    pacman_render: Arc<RwLock<PacmanStateRenderInfo>>,
    location_receive: Receiver<Point2<u8>>,
    replay_send: Sender<()>,
) {
    let mut rng = ThreadRng::default();

    let mut previous_pacman_location = Point2::new(14u8, 7);

    loop {
        // {} block to make sure `game` goes out of scope and the RwLockWriteGuard is released
        {
            let mut state = pacman_render.write().unwrap();

            // fetch updated pacbot position
            while let Ok(pacbot_location) = location_receive.try_recv() {
                state.pacman_state.update_pacman(
                    pacbot_location,
                    facing_direction(&previous_pacman_location, &pacbot_location),
                );
                previous_pacman_location = pacbot_location;
            }

            let agent_setup = state.agent_setup.clone();

            // step the game
            if !state.pacman_state.paused {
                state.pacman_state.step(&agent_setup, &mut rng, true);
                replay_send.send(()).unwrap()
            }
        }

        // Sleep for 1/2 a second
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
            ui.label(format!("Score: {}", pacman_state.score));
            ui.label(format!("Lives: {}", pacman_state.lives));
            ui.label(format!("Frame: {}", pacman_state.elapsed_time));
        });

        // ghosts
        for i in 0..pacman_state.ghosts.len() {
            painter.circle_filled(
                world_to_screen.map_point(Pos2::new(
                    pacman_state.ghosts[i].agent.location.x as f32,
                    pacman_state.ghosts[i].agent.location.y as f32,
                )),
                world_to_screen.map_dist(0.45),
                match pacman_state.ghosts[i].color {
                    GhostType::Red => GHOST_RED_COLOR,
                    GhostType::Pink => GHOST_PINK_COLOR,
                    GhostType::Orange => GHOST_ORANGE_COLOR,
                    GhostType::Blue => GHOST_BLUE_COLOR,
                },
            )
        }

        // pellets
        for i in 0..pacman_state.pellets.len() {
            if pacman_state.pellets[i] {
                painter.circle_filled(
                    world_to_screen.map_point(Pos2::new(
                        self.agent_setup.grid().walkable_nodes()[i].x as f32,
                        self.agent_setup.grid().walkable_nodes()[i].y as f32,
                    )),
                    3.0,
                    PELLET_COLOR,
                )
            }
        }

        // super pellets
        for super_pellet in &pacman_state.power_pellets {
            painter.circle_filled(
                world_to_screen.map_point(Pos2::new(super_pellet.x as f32, super_pellet.y as f32)),
                6.0,
                SUPER_PELLET_COLOR,
            )
        }
    }
}
