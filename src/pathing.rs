use std::collections::HashSet;

use crate::grid::{ComputedGrid, IntLocation};
use crate::physics::LightPhysicsInfo;
use crate::{HighLevelStrategy, UserSettings};
use bevy::prelude::*;
use rand::prelude::SliceRandom;
use rand::{distributions::WeightedIndex, seq::IteratorRandom};
use rand_distr::Distribution;
use rapier2d::na::Vector2;

/// Pacbot's desired path
#[derive(Default, Resource)]
pub struct TargetPath(pub Vec<IntLocation>);

/// The actual target velocity sent to the robot
#[derive(Default, Resource)]
pub struct TargetVelocity(pub Vector2<f32>, pub f32);

pub fn test_path_position_to_target_path(
    grid: Res<ComputedGrid>,
    phys_info: Res<LightPhysicsInfo>,
    mut target_path: ResMut<TargetPath>,
    settings: Res<UserSettings>,
) {
    if settings.high_level_strategy == HighLevelStrategy::ReinforcementLearning {
        return;
    }
    if let (Some(target_loc), Some(pf_pos)) = (settings.test_path_position, phys_info.pf_pos) {
        if let Some(current_loc) = grid.node_nearest(pf_pos.translation.x, pf_pos.translation.y) {
            if current_loc == target_loc {
                *target_path = TargetPath(vec![]);
                return;
            }
            // if we're already going there, just exit
            if let Some(first) = target_path.0.first() {
                if let Some(last) = target_path.0.last() {
                    // if the next one in the target path is adjacent to us
                    // and the destination is the target location
                    if grid.neighbors(&current_loc).contains(first) && *last == target_loc {
                        return;
                    }
                }
            }
            // we need to make a new path from current_loc to target_loc
            if let Some(mut path) = grid.bfs_path(current_loc, target_loc) {
                path.remove(0);
                *target_path = TargetPath(path);
            } else {
                *target_path = TargetPath(vec![]);
            }
        }
    }
}

pub fn target_path_to_target_vel(
    phys_info: Res<LightPhysicsInfo>,
    target_path: Res<TargetPath>,
    mut target_velocity: ResMut<TargetVelocity>,
    settings: Res<UserSettings>,
) {
    if let (Some(target_pos), Some(curr_pos)) = (target_path.0.first(), phys_info.pf_pos) {
        let curr_pos = curr_pos.translation.vector.xy();
        let target_pos = Vector2::new(target_pos.row as f32, target_pos.col as f32);

        // The final speed will be min(max_speed, base_speed + speed_mul * num_straight_moves)
        let base_speed = settings.speed_base;
        let speed_mul = settings.speed_multiplier;
        let max_speed = settings.speed_cap;
        let mut delta_pos = target_pos - curr_pos;

        // Check how many of the next moves are in the same direction.
        // For each one, we slightly increase the speed.
        let mut adj_nodes = 0;
        let mut prev_pos = curr_pos;
        for target_pos_next in &target_path.0.as_slice()[1..] {
            let target_pos_next =
                Vector2::new(target_pos_next.row as f32, target_pos_next.col as f32);
            let delta_pos_next = target_pos_next - prev_pos;
            if delta_pos_next.normalize().dot(&delta_pos.normalize()) <= 0.95 {
                break;
            }
            adj_nodes += 1;
            prev_pos = target_pos_next;
        }
        if delta_pos.magnitude_squared() > 0.1 {
            delta_pos = delta_pos.normalize()
                * f32::min(max_speed, base_speed + speed_mul * adj_nodes as f32);
        }

        target_velocity.0 = delta_pos;
    } else if settings.high_level_strategy == HighLevelStrategy::ReinforcementLearning {
        target_velocity.0 = Vector2::new(0.0, 0.0);
    }
}

#[derive(Resource, Default)]
pub struct GridSampleProbs(std::collections::HashMap<IntLocation, f32>);

pub fn create_test_path_target_forward(
    settings: Res<UserSettings>,
    grid: Res<ComputedGrid>,
    mut path: ResMut<TargetPath>,
    phys_info: Res<LightPhysicsInfo>,
) {
    if settings.high_level_strategy == HighLevelStrategy::TestForward {
        if let Some(pf_pos) = phys_info.pf_pos {
            let current_space = grid
                .node_nearest(pf_pos.translation.x, pf_pos.translation.y)
                .expect("Invalid position!");
            if let Some(first_target) = path.0.first() {
                if *first_target == current_space {
                    path.0.remove(0);
                }
            }
            let mut rng = rand::thread_rng();
            if path.0.is_empty() {
                path.0.push(
                    *grid
                        .neighbors(&current_space)
                        .choose(&mut rng)
                        .expect("No neighbors!"),
                );
            }
            if path.0.len() == 1 {
                let first = path.0[0];
                let neighbor = grid
                    .neighbors(&first)
                    .into_iter()
                    .filter(|x| *x != current_space)
                    .next()
                    .expect("No exit!");
                path.0.push(neighbor);
            }
            if path.0.len() > 1 {
                while path.0.len() < 4 {
                    let prev = path.0[path.0.len() - 2];
                    let curr = path.0[path.0.len() - 1];
                    let neighbor = grid
                        .neighbors(&curr)
                        .into_iter()
                        .filter(|x| *x != prev)
                        .choose(&mut rng)
                        .expect("No exit!");
                    path.0.push(neighbor);
                }
            }
        }
    }
}

/// Generates a new target position when Pacbot reaches the current target.
pub fn create_test_path_target(
    mut settings: ResMut<UserSettings>,
    grid: Res<ComputedGrid>,
    path: Res<TargetPath>,
    mut grid_probs: ResMut<GridSampleProbs>,
    phys_info: Res<LightPhysicsInfo>,
) {
    if path.0.is_empty() {
        let mut rng = rand::thread_rng();
        let mut walkable = grid.walkable_nodes().clone();
        if let Some(pf_pos) = phys_info.pf_pos {
            if let Some(pos) = grid.node_nearest(pf_pos.translation.x, pf_pos.translation.y) {
                walkable = walkable
                    .into_iter()
                    .filter(|loc| grid.dist(&pos, &loc).is_some())
                    .collect::<Vec<IntLocation>>();
            }
        }
        match &settings.high_level_strategy {
            HighLevelStrategy::TestUniform => {
                if let Some(target) = walkable.iter().choose(&mut rng) {
                    settings.test_path_position = Some(*target);
                } else {
                    warn!("Tried to update path target, but grid returned None.");
                }
            }
            HighLevelStrategy::TestNonExplored => {
                // If our set of walkable cells are different, reinitialize probs
                let usable = walkable.iter().collect::<HashSet<_>>()
                    == grid_probs.0.keys().collect::<HashSet<_>>();
                if !usable {
                    grid_probs.0.clear();
                    for pos in walkable {
                        grid_probs.0.insert(pos, 1.);
                    }
                }

                // Increase the probability of selecting each cell
                for v in grid_probs.0.values_mut() {
                    *v = (*v + 0.1).min(1.);
                }

                // Sample new position from weighted index
                let keys: Vec<_> = grid_probs.0.keys().collect();
                let weights: Vec<_> = keys.iter().map(|k| grid_probs.0.get(k).unwrap()).collect();
                let index = WeightedIndex::new(weights).unwrap();
                let target = keys[index.sample(&mut rng)];
                settings.test_path_position = Some(*target);

                // Set the probability of the next target being on this path to 0
                let pf_pos = phys_info.pf_pos.unwrap();
                if let Some(current_loc) =
                    grid.node_nearest(pf_pos.translation.x, pf_pos.translation.y)
                {
                    let path = grid.bfs_path(current_loc, *target).unwrap();
                    for pos in path {
                        grid_probs.0.insert(pos, 0.);
                    }
                }
            }
            _ => (),
        };
    }
}
