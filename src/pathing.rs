use crate::grid::IntLocation;
use crate::physics::LightPhysicsInfo;
use crate::{PacmanGameState, UserSettings};
use bevy::prelude::*;
use rapier2d::na::Vector2;

/// Pacbot's desired path
#[derive(Default, Resource)]
pub struct TargetPath(pub Vec<IntLocation>);

/// The actual target velocity sent to the robot
#[derive(Default, Resource)]
pub struct TargetVelocity(pub Vector2<f32>, pub f32);

pub fn target_path_to_target_vel(
    pacman_state: Res<PacmanGameState>,
    phys_info: Res<LightPhysicsInfo>,
    target_path: Res<TargetPath>,
    mut target_velocity: ResMut<TargetVelocity>,
    settings: Res<UserSettings>,
) {
    if settings.enable_ai {
        if pacman_state.0.is_paused() {
            target_velocity.0 = Vector2::new(0.0, 0.0);
            target_velocity.1 = 0.0;
        } else if let (Some(target_pos), Some(curr_pos)) =
            (target_path.0.first(), phys_info.real_pos)
        {
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
}
