use crate::grid::IntLocation;
use bevy::prelude::Resource;
use rapier2d::na::Vector2;

/// Pacbot's desired path
#[derive(Default, Resource)]
pub struct TargetPath(pub Vec<IntLocation>);

/// The actual target velocity sent to the robot
#[derive(Default, Resource)]
pub struct TargetVelocity(pub Vector2<f32>, pub f32);
