use crate::constants::GUI_PARTICLE_FILTER_POINTS;
use crate::grid::standard_grids::StandardGrid;
use crate::grid::PLocation;
use crate::gui::colors::{
    PACMAN_COLOR, PACMAN_DISTANCE_SENSOR_RAY_COLOR, PACMAN_FACING_INDICATOR_COLOR,
    PACMAN_GUESS_COLOR, PACMAN_PARTICLE_FILTER_COLOR, PACMAN_REPLAY_COLOR,
};
use crate::gui::transforms::Transform;
use crate::gui::{App, AppMode};
use crate::physics::PacbotSimulation;
use crate::robot::Robot;
use crate::util::stopwatch::Stopwatch;
use eframe::egui::{Painter, Pos2, Stroke};
use pacbot_rs::variables::PACMAN_SPAWN_LOC;
use rapier2d::na::{Isometry2, Point2, Vector2};
use serde::{Deserialize, Serialize};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex, RwLock};

/// Stores state needed to render physics information.
#[derive(Default, Clone, Serialize, Deserialize)]
pub struct PhysicsRenderInfo {
    /// If true, the physics thread should not advance physics
    pub sleep: bool,
    /// The current position of the robot.
    pub pacbot_pos: Isometry2<f32>,
    /// The particle filter's current best guess
    pub pacbot_pos_guess: Isometry2<f32>,
    /// An array of start and end points.
    pub primary_robot_rays: Vec<(Point2<f32>, Point2<f32>)>,
    /// The number of best particle filter points to save
    pub pf_count: usize,
    /// The best pf_count particle filter points
    pub pf_points: Vec<Isometry2<f32>>,
}

/// Thread where physics gets run.
pub(super) fn run_physics(
    phys_render: Arc<RwLock<PhysicsRenderInfo>>,
    current_velocity: Arc<RwLock<(Vector2<f32>, f32)>>,
    location_send: Sender<PLocation>,
    restart_recv: Receiver<(StandardGrid, Robot, Isometry2<f32>)>,
    distance_sensors: Arc<Mutex<Vec<Option<f32>>>>,
    pf_stopwatch: Arc<Mutex<Stopwatch>>,
    physics_stopwatch: Arc<Mutex<Stopwatch>>,
) {
    let grid = StandardGrid::Pacman.compute_grid();

    let distance_sensors_ref = distance_sensors.clone();

    let mut simulation = PacbotSimulation::new(
        grid.to_owned(),
        Robot::default(),
        StandardGrid::Pacman.get_default_pacbot_isometry(),
        distance_sensors_ref,
    );

    let mut previous_pacbot_location = PLocation::new(PACMAN_SPAWN_LOC.row, PACMAN_SPAWN_LOC.col);

    loop {
        // Was a restart requested?
        if let Ok((grid, robot, isometry)) = restart_recv.try_recv() {
            simulation = PacbotSimulation::new(
                grid.compute_grid(),
                robot,
                isometry,
                distance_sensors.clone(),
            );
        }

        // Run simulation one step
        physics_stopwatch.lock().unwrap().start();
        simulation.step();
        physics_stopwatch
            .lock()
            .unwrap()
            .mark_segment("Step simulation");

        // Estimate game location
        let estimated_location = grid
            .node_nearest(
                simulation.get_primary_robot_position().translation.x,
                simulation.get_primary_robot_position().translation.y,
            )
            .unwrap_or(PLocation::new(1, 1));

        // Update distance sensors
        let rays = simulation.get_primary_robot_rays();
        {
            let mut d = distance_sensors.lock().unwrap();
            for i in 0..d.len() {
                if let Some((a, b)) = rays.get(i) {
                    d[i] = Some(((a.x - b.x).powi(2) + (a.y - b.y).powi(2)).sqrt())
                }
            }
        }

        // Update particle filter
        simulation.pf_update(estimated_location, &pf_stopwatch);
        physics_stopwatch
            .lock()
            .unwrap()
            .mark_segment("Update particle filter");

        // Update the current velocity
        let target = *current_velocity.as_ref().read().unwrap();
        simulation.set_target_robot_velocity(target);

        // Update our render state
        *phys_render.write().unwrap() = PhysicsRenderInfo {
            sleep: false,
            pacbot_pos: *simulation.get_primary_robot_position(),
            pacbot_pos_guess: simulation.pf_best_guess(),
            primary_robot_rays: simulation.get_primary_robot_rays().clone(),
            pf_count: GUI_PARTICLE_FILTER_POINTS,
            pf_points: simulation.pf_points(GUI_PARTICLE_FILTER_POINTS),
        };

        // Did pacbot's (rounded) position change? If so, send the new one to the game
        let pacbot_location = grid
            .node_nearest(
                simulation.get_primary_robot_position().translation.x,
                simulation.get_primary_robot_position().translation.y,
            )
            .unwrap();

        if pacbot_location != previous_pacbot_location {
            location_send.send(pacbot_location).unwrap();
            previous_pacbot_location = pacbot_location;
        }
    }
}

impl App {
    pub(super) fn draw_simulation(&mut self, world_to_screen: &Transform, painter: &Painter) {
        let phys_render = self.phys_render.as_ref().read().unwrap();
        let pacbot_pos = phys_render.pacbot_pos;

        // pacbot real position
        painter.circle_filled(
            world_to_screen.map_point(Pos2::new(
                pacbot_pos.translation.x,
                pacbot_pos.translation.y,
            )),
            world_to_screen.map_dist(self.robot.collider_radius),
            PACMAN_COLOR,
        );

        // pacbot best estimate position
        let best_guess = phys_render.pacbot_pos_guess;
        painter.circle_stroke(
            world_to_screen.map_point(Pos2::new(
                best_guess.translation.x,
                best_guess.translation.y,
            )),
            world_to_screen.map_dist(self.robot.collider_radius),
            Stroke::new(2.0, PACMAN_GUESS_COLOR),
        );

        let pacbot_front = pacbot_pos.rotation.transform_point(&Point2::new(0.45, 0.0));

        // pacbot facing indicator
        painter.line_segment(
            [
                world_to_screen.map_point(Pos2::new(
                    pacbot_pos.translation.x,
                    pacbot_pos.translation.y,
                )),
                world_to_screen.map_point(Pos2::new(
                    pacbot_front.x + pacbot_pos.translation.x,
                    pacbot_front.y + pacbot_pos.translation.y,
                )),
            ],
            Stroke::new(2.0, PACMAN_FACING_INDICATOR_COLOR),
        );

        // pacbot from the replay
        if matches!(self.mode, AppMode::Playback) {
            painter.circle_filled(
                world_to_screen.map_point(Pos2::new(
                    self.replay_pacman.translation.x,
                    self.replay_pacman.translation.y,
                )),
                world_to_screen.map_dist(self.robot.collider_radius),
                PACMAN_REPLAY_COLOR,
            );

            let pacbot_front = self
                .replay_pacman
                .rotation
                .transform_point(&Point2::new(0.45, 0.0));

            painter.line_segment(
                [
                    world_to_screen.map_point(Pos2::new(
                        self.replay_pacman.translation.x,
                        self.replay_pacman.translation.y,
                    )),
                    world_to_screen.map_point(Pos2::new(
                        pacbot_front.x + self.replay_pacman.translation.x,
                        pacbot_front.y + self.replay_pacman.translation.y,
                    )),
                ],
                Stroke::new(2.0, PACMAN_FACING_INDICATOR_COLOR),
            );
        }

        // pacbot best guess distance sensor rays
        let distance_sensor_rays = &phys_render.primary_robot_rays;

        for (s, f) in distance_sensor_rays.iter() {
            painter.line_segment(
                [
                    world_to_screen.map_point(Pos2::new(s.x, s.y)),
                    world_to_screen.map_point(Pos2::new(f.x, f.y)),
                ],
                Stroke::new(1.0, PACMAN_DISTANCE_SENSOR_RAY_COLOR),
            );
        }

        // particle filter
        let pf_points = &phys_render.pf_points;

        for p in pf_points {
            painter.circle_filled(
                world_to_screen.map_point(Pos2::new(p.translation.x, p.translation.y)),
                1.0,
                PACMAN_PARTICLE_FILTER_COLOR,
            );
        }
    }
}
