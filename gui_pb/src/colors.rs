#![allow(dead_code)]

use core_pb::messages::NetworkStatus;
use eframe::egui::Color32;

pub const WALL_COLOR: Color32 = Color32::LIGHT_GRAY;
pub const PELLET_COLOR: Color32 = Color32::BLUE;
pub const SUPER_PELLET_COLOR: Color32 = Color32::BLUE;

pub const PACMAN_COLOR: Color32 = Color32::YELLOW;
pub const PACMAN_GUESS_COLOR: Color32 = Color32::GREEN;
pub const PACMAN_PARTICLE_FILTER_COLOR: Color32 = Color32::RED;
pub const PACMAN_REPLAY_COLOR: Color32 = Color32::from_rgba_premultiplied(88, 88, 0, 25);
pub const PACMAN_FACING_INDICATOR_COLOR: Color32 = Color32::BLUE;
pub const PACMAN_DISTANCE_SENSOR_RAY_COLOR: Color32 = Color32::GREEN;
pub const PACMAN_AI_TARGET_LOCATION_COLOR: Color32 =
    Color32::from_rgba_premultiplied(128, 0, 128, 255);

pub const GHOST_RED_COLOR: Color32 = Color32::RED;
pub const GHOST_PINK_COLOR: Color32 = Color32::from_rgb(255, 192, 203);
pub const GHOST_ORANGE_COLOR: Color32 = Color32::from_rgb(255, 140, 0);
pub const GHOST_BLUE_COLOR: Color32 = Color32::BLUE;
pub const GHOST_FRIGHTENED_COLOR: Color32 = Color32::LIGHT_YELLOW;

pub const TRANSLUCENT_GREEN_COLOR: Color32 = Color32::from_rgba_premultiplied(0, 50, 0, 50);
pub const TRANSLUCENT_YELLOW_COLOR: Color32 = Color32::from_rgba_premultiplied(50, 50, 0, 50);
pub const TRANSLUCENT_RED_COLOR: Color32 = Color32::from_rgba_premultiplied(50, 0, 0, 50);

pub fn network_status_to_color(value: NetworkStatus) -> Color32 {
    match value {
        NetworkStatus::NotConnected => Color32::DARK_GRAY,
        NetworkStatus::ConnectionFailed => TRANSLUCENT_RED_COLOR,
        NetworkStatus::Connecting => TRANSLUCENT_YELLOW_COLOR,
        NetworkStatus::Connected => TRANSLUCENT_GREEN_COLOR,
    }
}
