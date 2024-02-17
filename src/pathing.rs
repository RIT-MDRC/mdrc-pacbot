use crate::grid::IntLocation;
use crate::physics::LightPhysicsInfo;
use crate::{PacmanGameState, UserSettings};
use bevy_ecs::prelude::*;
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
    if settings.enable_ai && false {
        if !pacman_state.0.is_paused() {
            if let Some(target_pos) = target_path.0.get(0) {
                if let Some(curr_pos) = phys_info.real_pos {
                    let curr_pos = curr_pos.translation.vector.xy();
                    let target_pos = Vector2::new(target_pos.row as f32, target_pos.col as f32);

                    let max_speed = 20.;
                    let mut delta_pos = target_pos - curr_pos;
                    if delta_pos.magnitude() > max_speed {
                        delta_pos = delta_pos.normalize() * max_speed;
                    }
                    delta_pos *= 2.;

                    target_velocity.0 = delta_pos;
                    return;
                }
            }
        }
    }
    target_velocity.0 = Vector2::new(0.0, 0.0);
    target_velocity.1 = 0.0;
}
