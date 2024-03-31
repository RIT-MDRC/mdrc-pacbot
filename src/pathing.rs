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
    target_velocity.0 = Vector2::new(0.0, 0.0);
    target_velocity.1 = 0.0;
    if let (Some(target_pos), Some(curr_pos)) = (target_path.0.first(), phys_info.pf_pos) {
        let curr_pos = curr_pos.translation.vector.xy();
        let target_pos = Vector2::new(target_pos.row as f32, target_pos.col as f32);

        let max_speed = 6.;
        let mut delta_pos = target_pos - curr_pos;

        let mut barrel_through = false;
        if let Some(target_pos_next) = target_path.0.get(1) {
            // Barrel through if next position is in the same direction
            let delta_pos_next =
                Vector2::new(target_pos_next.row as f32, target_pos_next.col as f32) - curr_pos;
            if delta_pos_next.normalize().dot(&delta_pos.normalize()) > 0.95 {
                barrel_through = true;
            }
        }
        if barrel_through || delta_pos.magnitude() > max_speed {
            delta_pos = delta_pos.normalize() * max_speed;
        } else {
            delta_pos *= 4.;
        }

        target_velocity.0 = delta_pos;
    }
}
