//! Ghost behavior

use crate::agent_setup::{GhostSetup, PacmanAgentSetup};
use crate::game_state::{Agent, Ghost, GhostMode, GhostType};
use crate::grid::{ComputedGrid, Direction};
use rand::rngs::ThreadRng;
use rand::Rng;
use rapier2d::na::Point2;
use rapier2d::parry::utils::Array1;

impl Ghost {
    /// Have the ghost take one step
    #[allow(clippy::too_many_arguments)]
    pub fn step_ghost(
        &mut self,
        agent_setup: &PacmanAgentSetup,
        ghost_setup: &GhostSetup,
        mode: GhostMode,
        elapsed_time: u32,
        pacman: &Agent,
        red_ghost_location: &Point2<u8>,
        rng: &mut ThreadRng,
    ) {
        if self.frightened_counter > 0 {
            self.frightened_counter -= 1;
        }

        let mut destination;

        if (elapsed_time as usize) < ghost_setup.start_path.len() {
            destination = ghost_setup.start_path[elapsed_time as usize].0;
        } else if let Some(next_respawn_path_move) = self.get_respawn_path_move(agent_setup) {
            destination = next_respawn_path_move.to_owned();
            self.respawn_timer += 1;
        } else if let Some(next_swapped_state_move) =
            self.get_swapped_state_move(agent_setup, elapsed_time)
        {
            destination = next_swapped_state_move.to_owned();
        } else if self.frightened_counter > 0 {
            destination = self.get_frightened_move(rng, agent_setup.grid());
            self.frightened_counter -= 1;
        } else if mode == GhostMode::Chase {
            destination =
                self.get_next_chase_move(&pacman.location, pacman.direction, red_ghost_location);
        } else {
            destination = self.get_next_scatter_move(ghost_setup);
        }

        destination = self.get_move_based_on(&destination, ghost_setup, agent_setup.grid());

        let current_position = self.agent.location.to_owned();
        let direction = Self::direction(&current_position, &destination);

        self.previous_location = current_position;
        self.agent.location = destination;
        self.agent.direction = direction;
    }

    fn direction(start: &Point2<u8>, end: &Point2<u8>) -> Direction {
        if start.x < end.x {
            Direction::Right
        } else if start.x > end.x {
            Direction::Left
        } else if start.y < end.y {
            Direction::Up
        } else {
            Direction::Down
        }
    }

    /// Get Euclidean distance between two points
    fn distance(a: &Point2<u8>, b: &Point2<u8>) -> f32 {
        let dx = (a.x as f32) - (b.x as f32);
        let dy = (a.y as f32) - (b.y as f32);
        (dx * dx + dy * dy).sqrt()
    }

    fn get_move_based_on(
        &self,
        p: &Point2<u8>,
        ghost_setup: &GhostSetup,
        grid: &ComputedGrid,
    ) -> Point2<u8> {
        grid.neighbors(p)
            .iter()
            .min_by(|n1, n2| {
                let d1 = Self::distance(n1, &ghost_setup.scatter_point);
                let d2 = Self::distance(n2, &ghost_setup.scatter_point);
                d1.total_cmp(&d2)
            })
            .unwrap()
            .to_owned()
    }

    fn get_swapped_state_move(
        &self,
        agent_setup: &PacmanAgentSetup,
        elapsed_time: u32,
    ) -> Option<&Point2<u8>> {
        if agent_setup.state_swap_times().contains(&elapsed_time) {
            return Some(&self.previous_location);
        }
        None
    }

    fn get_respawn_path_move(&self, agent_setup: &PacmanAgentSetup) -> Option<Point2<u8>> {
        let p = agent_setup.ghost_respawn_path().get_at(self.respawn_timer);
        p?;
        Some(p.unwrap().0)
    }

    /// Frightened behavior - return a random legal move
    fn get_frightened_move(&self, rng: &mut ThreadRng, grid: &ComputedGrid) -> Point2<u8> {
        let moves = grid.neighbors(&self.agent.location);
        let index = rng.gen_range(0..moves.len());
        moves[index].to_owned()
    }

    fn get_next_scatter_move(&self, ghost_setup: &GhostSetup) -> Point2<u8> {
        ghost_setup.scatter_point.to_owned()
    }

    fn get_next_chase_move(
        &self,
        pacman_location: &Point2<u8>,
        pacman_direction: Direction,
        red_ghost_location: &Point2<u8>,
    ) -> Point2<u8> {
        match self.color {
            GhostType::Blue => {
                self.get_next_blue_chase_move(red_ghost_location, pacman_location, pacman_direction)
            }
            GhostType::Red => self.get_next_red_chase_move(pacman_location),
            GhostType::Pink => self.get_next_pink_chase_move(pacman_location, pacman_direction),
            GhostType::Orange => self.get_next_orange_chase_move(pacman_location),
        }
    }

    fn get_next_blue_chase_move(
        &self,
        red_ghost_location: &Point2<u8>,
        pacman_location: &Point2<u8>,
        pacman_direction: Direction,
    ) -> Point2<u8> {
        let target = match pacman_direction {
            Direction::Right => Point2::new(pacman_location.x + 2, pacman_location.y),
            Direction::Left => Point2::new(pacman_location.x - 2, pacman_location.y),
            Direction::Up => Point2::new(pacman_location.x - 2, pacman_location.y + 2),
            Direction::Down => Point2::new(pacman_location.x, pacman_location.y - 2),
        };

        let x = target.x + (target.x - red_ghost_location.x);
        let y = target.y + (target.y - red_ghost_location.y);

        Point2::new(x, y)
    }

    fn get_next_red_chase_move(&self, pacman_location: &Point2<u8>) -> Point2<u8> {
        pacman_location.to_owned()
    }

    /// Return the move closest to the space 4 tiles ahead of Pacman in the direction
    /// Pacman is currently facing.
    ///
    /// If Pacman is facing up, then we replicate a bug in
    /// the original game and return the move closest to the space 4 tiles above and
    /// 4 tiles to the left of Pacman.
    fn get_next_pink_chase_move(
        &self,
        pacman_location: &Point2<u8>,
        pacman_direction: Direction,
    ) -> Point2<u8> {
        let p = pacman_location;

        match pacman_direction {
            Direction::Up => Point2::new(p.x - 4, p.y + 4),
            Direction::Down => Point2::new(p.x, p.y - 4),
            Direction::Left => Point2::new(p.x - 4, p.y),
            Direction::Right => Point2::new(p.x + 4, p.y),
        }
    }

    fn get_next_orange_chase_move(&self, pacman_location: &Point2<u8>) -> Point2<u8> {
        if Self::distance(&self.agent.location, pacman_location) > 8.0 {
            return pacman_location.to_owned();
        }
        self.get_next_red_chase_move(pacman_location)
    }

    /// Teleport the ghost back to the home position, after it is eaten
    pub fn send_home(&mut self, ghost_home_pos: &(Point2<u8>, Direction)) {
        self.agent.location = ghost_home_pos.0;
        self.agent.direction = ghost_home_pos.1;

        self.previous_location = ghost_home_pos.0;
        self.respawn_timer = 0;
        self.frightened_counter = 0;
    }
}
