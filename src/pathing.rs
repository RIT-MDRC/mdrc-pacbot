use crate::grid::{ComputedGrid, IntLocation};
use crate::physics::LightPhysicsInfo;
use crate::UserSettings;
use bevy::prelude::*;
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
    if settings.enable_ai {
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
) {
    if let (Some(target_pos), Some(curr_pos)) = (target_path.0.first(), phys_info.pf_pos) {
        let curr_pos = curr_pos.translation.vector.xy();
        let target_pos = Vector2::new(target_pos.row as f32, target_pos.col as f32);

        let base_speed = 4.;
        let speed_mul = 1.5;
        let mut delta_pos = target_pos - curr_pos;

        // Check how many of the next moves are in the same direction
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
            delta_pos = delta_pos.normalize() * (base_speed + speed_mul * adj_nodes as f32);
        }

        target_velocity.0 = delta_pos;
    }
}
