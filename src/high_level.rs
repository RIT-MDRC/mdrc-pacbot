//! Defines the Pacman agent's high level AI.

use std::collections::HashSet;

use crate::grid::ComputedGrid;
use crate::grid::IntLocation;
use crate::grid::GRID_COLS;
use crate::grid::GRID_ROWS;
use crate::pathing::TargetPath;
use crate::util::stopwatch::Stopwatch;
use crate::{HighLevelStrategy, PacmanGameState, UserSettings};
use bevy::prelude::*;
use candle_core::D;
use candle_core::{Device, Module, Tensor};
use candle_nn as nn;
use ndarray::{s, Array};
use pacbot_rs::game_modes::GameMode;
use pacbot_rs::game_state::GameState;
use pacbot_rs::location::LocationState;
use pacbot_rs::variables;
use pacbot_rs::variables::GHOST_FRIGHT_STEPS;

/// Plugin for high level AI functionality.
pub struct HLPlugin;

impl Plugin for HLPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(AiStopwatch(Stopwatch::new(
            10,
            "AI".to_string(),
            30.0,
            40.0,
        )))
        .add_systems(Update, run_high_level)
        .init_non_send_resource::<HighLevelContext>()
        .init_resource::<ActionMask>();
    }
}

/// Stores latest action mask.
#[derive(Resource, Default)]
pub struct ActionMask(pub [bool; 5]);

/// Tracks the time AI takes to make decisions
#[derive(Resource)]
pub struct AiStopwatch(pub Stopwatch);

#[allow(clippy::too_many_arguments)]
pub fn run_high_level(
    game_state: Res<PacmanGameState>,
    mut target_path: ResMut<TargetPath>,
    mut action_mask: ResMut<ActionMask>,
    mut hl_ctx: NonSendMut<HighLevelContext>,
    std_grid: Local<ComputedGrid>,
    mut switched: Local<bool>,
    settings: Res<UserSettings>,
    mut ai_stopwatch: ResMut<AiStopwatch>,
) {
    if settings.high_level_strategy == HighLevelStrategy::ReinforcementLearning
        && game_state.0.is_paused()
    {
        *target_path = TargetPath(vec![]);
    }
    if settings.high_level_strategy == HighLevelStrategy::ReinforcementLearning
        && !game_state.0.is_paused()
        && game_state.is_changed()
    {
        // are super pellets gone and ghosts not frightened? then switch models
        if [(3, 1), (3, 26), (23, 1), (23, 26)]
            .into_iter()
            .all(|x| !game_state.0.get_state().pellet_at(x))
            && game_state
                .0
                .get_state()
                .ghosts
                .iter()
                .all(|g| !g.is_frightened())
        {
            if !*switched {
                info!("Switched to second AI!");
                *hl_ctx = HighLevelContext::new("./checkpoints/endgame.safetensors");
                *switched = true;
            }
        } else {
            if *switched {
                info!("Switched to first AI!");
                *hl_ctx = HighLevelContext::default();
                *switched = false;
            }
        }

        ai_stopwatch.0.start();

        let mut path_nodes = std::collections::HashSet::new();
        let mut sim_engine = game_state.0.clone();
        let mut curr_pos = IntLocation {
            row: sim_engine.get_state().pacman_loc.row,
            col: sim_engine.get_state().pacman_loc.col,
        };
        let curr_score = sim_engine.get_state().get_score();
        let mut mask = None;
        for _ in 0..6 {
            let (action, m) = hl_ctx.step(
                sim_engine.get_state(),
                &std_grid,
                settings.bot_update_period,
            );
            mask.get_or_insert(m);
            let target_pos = match action {
                HLAction::Stay => curr_pos,
                HLAction::Left => IntLocation {
                    row: curr_pos.row,
                    col: curr_pos.col - 1,
                },
                HLAction::Right => IntLocation {
                    row: curr_pos.row,
                    col: curr_pos.col + 1,
                },
                HLAction::Up => IntLocation {
                    row: curr_pos.row - 1,
                    col: curr_pos.col,
                },
                HLAction::Down => IntLocation {
                    row: curr_pos.row + 1,
                    col: curr_pos.col,
                },
            };
            sim_engine.set_pacman_location(LocationState {
                row: target_pos.row,
                col: target_pos.col,
                dir: 0,
            });
            sim_engine.step();
            curr_pos = target_pos;
            path_nodes.insert(target_pos);
            // if we go to a super pellet space, stop planning
            if sim_engine.is_paused()
                || ((target_pos.row == 3) || (target_pos.row == 23))
                    && ((target_pos.col == 1) || (target_pos.col == 26))
            {
                break;
            }
        }
        if let Some(mask) = mask {
            action_mask.0 = mask;
        }
        let new_score = sim_engine.get_state().get_score();

        // Construct minimum path
        // Path must have at least 2 nodes, otherwise just stay in place
        // If the score greatly increases (e.g. you ate a super pellet or a ghost), just use the normal path, since pathing becomes weird
        let pacman_loc = game_state.0.get_state().pacman_loc;
        let start_pos = IntLocation {
            row: pacman_loc.row,
            col: pacman_loc.col,
        };
        let mut curr_pos = start_pos;
        let mut path = Vec::new();
        path_nodes.remove(&curr_pos);
        for _ in 0..path_nodes.len() {
            if let Some(next_pos) = path_nodes
                .iter()
                .find(|p| ((p.col - curr_pos.col).abs() + (p.row - curr_pos.row).abs()) == 1)
            {
                curr_pos = *next_pos;
                path.push(curr_pos);
                path_nodes.remove(&curr_pos);
            } else {
                break;
            }
        }
        if ((new_score - curr_score) < variables::SUPER_PELLET_POINTS) && path.is_empty() {
            path = vec![start_pos];
        }
        target_path.0 = path;

        // If the 2nd stage AI is active, and there is a guaranteed-safe path that eats all the
        // remaining pellets, then just take that path.
        if *switched && game_state.0.get_state().get_num_pellets() <= 10 {
            let game_end_path =
                find_game_ending_path(&settings, game_state.0.get_state(), &std_grid);
            if let Some(game_end_path) = game_end_path {
                target_path.0 = game_end_path;
            }
        }

        ai_stopwatch.0.mark_segment("AI");
    }
}

fn find_game_ending_path(
    settings: &UserSettings,
    game_state: &GameState,
    grid: &ComputedGrid,
) -> Option<Vec<IntLocation>> {
    let mut cur_pos = IntLocation::new(game_state.pacman_loc.row, game_state.pacman_loc.col);
    let mut path = Vec::new();

    let mut remaining_pellets = (0..GRID_ROWS)
        .flat_map(|row| (0..GRID_COLS).map(move |col| IntLocation::new(row as i8, col as i8)))
        .filter(|&pos| game_state.pellet_at((pos.row, pos.col)))
        .collect::<HashSet<_>>();
    while let Some(&closest_pellet) = remaining_pellets
        .iter()
        .min_by_key(|&pellet_pos| grid.dist(&cur_pos, pellet_pos))
    {
        for path_pos in grid.bfs_path(cur_pos, closest_pellet)? {
            // If any ghosts are too close to this location (extrapolating ahead in time pessimistically),
            // then abort and return None.
            if game_state.ghosts.iter().any(|ghost| {
                // check if too close
                let ghost_pos = IntLocation::new(ghost.loc.row, ghost.loc.col);
                if let Some(dist_from_ghost) = grid.dist(&path_pos, &ghost_pos) {
                    let num_pacman_moves = path.len();
                    let num_ghost_moves = ((settings.bot_update_period as f32
                        / game_state.update_period as f32)
                        * num_pacman_moves as f32)
                        + 2.0;
                    (dist_from_ghost as f32) < num_ghost_moves
                } else {
                    false // no path from ghost to pacman
                }
            }) {
                return None;
            }

            let is_start_location = path.is_empty() && path_pos == cur_pos;
            let is_last_path_pos = path.last().is_some_and(|&last| last == path_pos);
            if !is_start_location && !is_last_path_pos {
                path.push(path_pos);
            }
        }

        if let Some(&last) = path.last() {
            cur_pos = last;
        }

        remaining_pellets.remove(&closest_pellet);
    }

    Some(path)
}

/// Represents an action the AI can choose to perform.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum HLAction {
    /// The agent should stay in place.
    Stay,
    /// The agent should move left.
    Left,
    /// The agent should move right.
    Right,
    /// The agent should move up.
    Up,
    /// The agent should move down.
    Down,
}

const OBS_SHAPE: (usize, usize, usize) = (17, 28, 31);

/// Handles executing high level AI.
pub struct HighLevelContext {
    net: QNetV2,
    // These `cached` variables contain the last observed positions.
    // Once these cached positions are different from the next observed positions, the `last_variables`
    // are updated with these.
    // "cached" and "last" variables have their second coordinate flipped from the original game state,
    // so you shouldn't need to transform it again.
    pos_cached: Option<(usize, usize)>,
    ghost_pos_cached: Vec<Option<(usize, usize)>>,
    last_pos: Option<(usize, usize)>,
    last_ghost_pos: Vec<Option<(usize, usize)>>,
}

impl Default for HighLevelContext {
    fn default() -> Self {
        Self::new("./checkpoints/q_net.safetensors")
    }
}

impl HighLevelContext {
    /// Creates a new instance of the high level AI.
    pub fn new(weights_path: &str) -> Self {
        let mut vm = nn::VarMap::new();
        let vb =
            nn::VarBuilder::from_varmap(&vm, candle_core::DType::F32, &candle_core::Device::Cpu);
        let net = QNetV2::new(
            candle_core::Shape::from_dims(&[OBS_SHAPE.0, OBS_SHAPE.1, OBS_SHAPE.2]),
            5,
            vb,
        )
        .unwrap();
        vm.load(weights_path).unwrap();

        Self {
            net,
            last_pos: None,
            last_ghost_pos: vec![None; 4],
            pos_cached: None,
            ghost_pos_cached: vec![None; 4],
        }
    }

    /// Runs one step of the high level AI.
    /// Returns the action the AI has decided to take and the action masks.
    /// Currently, this implements a DQN approach.
    fn step(
        &mut self,
        game_state: &GameState,
        grid: &ComputedGrid,
        bot_update_period: usize,
    ) -> (HLAction, [bool; 5]) {
        // Convert the current game state into an agent observation.
        let mut obs_array = Array::zeros(OBS_SHAPE);
        let (mut wall, mut reward, mut pacman, mut ghost, mut last_ghost, mut state) = obs_array
            .multi_slice_mut((
                s![0, .., ..],
                s![1, .., ..],
                s![2..4, .., ..],
                s![4..8, .., ..],
                s![8..12, .., ..],
                s![12..15, .., ..],
            ));

        for row in 0..31 {
            for col in 0..28 {
                let obs_row = 31 - row - 1;
                wall[(col, obs_row)] = grid.grid()[row][col] as u8 as f32;
                reward[(col, obs_row)] = if game_state.pellet_at((row as i8, col as i8)) {
                    if ((row == 3) || (row == 23)) && ((col == 1) || (col == 26)) {
                        variables::SUPER_PELLET_POINTS
                    } else {
                        variables::PELLET_POINTS
                    }
                } else if game_state.fruit_exists()
                    && col == game_state.fruit_loc.col as usize
                    && row == game_state.fruit_loc.row as usize
                {
                    variables::FRUIT_POINTS
                } else {
                    0
                } as f32
                    / variables::COMBO_MULTIPLIER as f32;
            }
        }

        // Compute new pacman and ghost positions
        let new_pos_cached = {
            let pac_pos = game_state.pacman_loc;
            if pac_pos.col != 32 && pac_pos.row != 32 {
                Some((pac_pos.col as usize, (31 - pac_pos.row - 1) as usize))
            } else {
                None
            }
        };
        let new_ghost_pos_cached: Vec<_> = game_state
            .ghosts
            .iter()
            .map(|g| {
                if g.loc.col != 32 && g.loc.row != 32 {
                    Some((g.loc.col as usize, ((31 - g.loc.row - 1) as usize)))
                } else {
                    None
                }
            })
            .collect();

        // Save last positions.
        if self.pos_cached.is_none() {
            self.last_pos = new_pos_cached;
            self.pos_cached = new_pos_cached;
        }

        for (i, ghost) in self.ghost_pos_cached.iter_mut().enumerate() {
            if ghost.is_none() {
                self.last_ghost_pos[i] = new_ghost_pos_cached[i];
                *ghost = new_ghost_pos_cached[i];
            }
        }

        if new_pos_cached != self.pos_cached {
            self.last_pos = self.pos_cached;
            self.pos_cached = new_pos_cached;
        }

        for (i, ghost) in self.ghost_pos_cached.iter_mut().enumerate() {
            if new_ghost_pos_cached[i] != *ghost {
                self.last_ghost_pos[i] = *ghost;
                *ghost = new_ghost_pos_cached[i];
            }
        }

        if let Some(last_pos) = self.last_pos {
            pacman[(0, last_pos.0, last_pos.1)] = 1.0;
        }
        if let Some(new_pos_cached) = new_pos_cached {
            pacman[(1, new_pos_cached.0, new_pos_cached.1)] = 1.0;
        }

        for (i, g) in game_state.ghosts.iter().enumerate() {
            if let Some((col, row)) = new_ghost_pos_cached[i] {
                ghost[(i, col, row)] = 1.0;
                if g.is_frightened() {
                    state[(2, col, row)] = g.fright_steps as f32 / GHOST_FRIGHT_STEPS as f32;
                    reward[(col, row)] += 2_i32.pow(game_state.ghost_combo as u32) as f32;
                } else {
                    let state_index = if game_state.mode == GameMode::CHASE {
                        1
                    } else {
                        0
                    };
                    state[(state_index, col, row)] =
                        game_state.get_mode_steps() as f32 / GameMode::CHASE.duration() as f32;
                }
            }
        }

        for (i, pos) in self.last_ghost_pos.iter().enumerate() {
            if let Some(pos) = pos {
                last_ghost[(i, pos.0, pos.1)] = 1.0;
            }
        }

        obs_array
            .slice_mut(s![15, .., ..])
            .fill(bot_update_period as f32 / game_state.get_update_period() as f32);

        // Super pellet map
        for row in 0..31 {
            for col in 0..28 {
                let obs_row = 31 - row - 1;
                if game_state.pellet_at((row as i8, col as i8))
                    && ((row == 3) || (row == 23))
                    && ((col == 1) || (col == 26))
                {
                    obs_array[(16, col, obs_row)] = 1.;
                }
            }
        }

        // Create action mask.
        let mut action_mask = [false, false, false, false, false];
        let ghost_within = |row: i8, col: i8, distance: i8| {
            game_state.ghosts.iter().any(|g| {
                (g.loc.row - row).abs() + (g.loc.col - col).abs() <= distance && !g.is_frightened()
            })
        };
        let super_pellet_within = |row: i8, col: i8, distance: i8| {
            [(3, 1), (3, 26), (23, 1), (23, 26)]
                .iter()
                .any(|(p_row, p_col)| {
                    (p_row - row).abs() + (p_col - col).abs() <= distance
                        && game_state.pellet_at((*p_row, *p_col))
                })
        };
        if grid
            .valid_actions(IntLocation::new(
                game_state.pacman_loc.row,
                game_state.pacman_loc.col,
            ))
            .is_some()
        {
            for ghost_deny_distance in (0..=3).rev() {
                let row = game_state.pacman_loc.row;
                let col = game_state.pacman_loc.col;
                action_mask = [
                    (row, col),
                    (row + 1, col),
                    (row - 1, col),
                    (row, col - 1),
                    (row, col + 1),
                ]
                .map(|(target_row, target_col)| {
                    !game_state.wall_at((target_row, target_col))
                        && (!ghost_within(target_row, target_col, ghost_deny_distance)
                            || super_pellet_within(target_row, target_col, 0))
                });
                action_mask[0] = true;
                // if any movement is possible, and there is a ghost nearby, you must move
                if action_mask.iter().filter(|x| **x).count() > 1 && ghost_within(row, col, 1) {
                    action_mask[0] = false;
                }

                if action_mask != [true, false, false, false, false] {
                    break;
                }
            }
        }
        let action_mask_arr =
            Tensor::from_slice(&action_mask.map(|b| b as u8 as f32), 5, &Device::Cpu).unwrap(); // 1 if masked, 0 if not

        // Run observation through model and generate action.
        let obs_flat = obs_array.as_slice().unwrap();
        let obs_tensor = Tensor::from_slice(obs_flat, OBS_SHAPE, &Device::Cpu)
            .unwrap()
            .unsqueeze(0)
            .unwrap()
            .to_dtype(candle_core::DType::F32)
            .unwrap();

        let q_vals = self.net.forward(&obs_tensor).unwrap().squeeze(0).unwrap();

        let q_vals = ((q_vals * &action_mask_arr).unwrap()
            + ((1. - &action_mask_arr).unwrap() * -999.).unwrap())
        .unwrap();
        let argmax_idx = q_vals
            .argmax(D::Minus1)
            .unwrap()
            .to_scalar::<u32>()
            .unwrap() as usize;
        let mut argmax = [0.; 5];
        argmax[argmax_idx] = 1.;

        let actions = [
            HLAction::Stay,
            HLAction::Down,
            HLAction::Up,
            HLAction::Left,
            HLAction::Right,
        ];
        let action = actions[q_vals
            .argmax(candle_core::D::Minus1)
            .unwrap()
            .to_scalar::<u32>()
            .unwrap() as usize];
        (action, action_mask)
    }
}

/// Returns a convolutional block.
fn conv_block_pool(
    in_channels: usize,
    out_channels: usize,
    vb: nn::VarBuilder,
) -> candle_core::Result<nn::Sequential> {
    Ok(nn::seq()
        .add(nn::conv2d(
            in_channels,
            out_channels,
            3,
            nn::Conv2dConfig {
                padding: 1,
                ..Default::default()
            },
            vb,
        )?)
        .add(nn::func(|x| {
            let (_, _, w, h) = x.shape().dims4()?;
            let pad_w = w % 2;
            let pad_h = h % 2;
            x.pad_with_same(2, 0, pad_w)?
                .pad_with_same(3, 0, pad_h)?
                .max_pool2d(2)
        }))
        .add(nn::Activation::Silu))
}

/// The Q network.
struct QNetV2 {
    backbone: nn::Sequential,
    value_head: nn::Sequential,
    advantage_head: nn::Sequential,
}

impl QNetV2 {
    pub fn new(
        obs_shape: candle_core::Shape,
        action_count: usize,
        vb: nn::VarBuilder,
    ) -> candle_core::Result<Self> {
        let (obs_channels, _, _) = obs_shape.dims3().unwrap();
        let b_vb = vb.pp("backbone");
        let backbone = nn::seq()
            .add(nn::conv2d(
                obs_channels,
                16,
                5,
                nn::Conv2dConfig {
                    padding: 2,
                    ..Default::default()
                },
                b_vb.pp("0"),
            )?)
            .add(nn::Activation::Silu)
            .add(conv_block_pool(16, 32, b_vb.pp("2"))?)
            .add(conv_block_pool(32, 64, b_vb.pp("5"))?)
            .add(conv_block_pool(64, 128, b_vb.pp("8"))?)
            .add(nn::conv2d(
                128,
                128,
                3,
                nn::Conv2dConfig {
                    padding: 1,
                    groups: 128 / 16,
                    ..Default::default()
                },
                b_vb.pp("11"),
            )?)
            .add_fn(|xs| xs.max(candle_core::D::Minus1)?.max(candle_core::D::Minus1))
            .add(nn::Activation::Silu)
            .add(nn::linear(128, 256, b_vb.pp("15"))?)
            .add(nn::Activation::Silu);
        let value_head = nn::seq().add(nn::linear(256, 1, vb.pp("value_head").pp("0"))?);
        let advantage_head = nn::seq().add(nn::linear(
            256,
            action_count,
            vb.pp("advantage_head").pp("0"),
        )?);

        Ok(Self {
            backbone,
            value_head,
            advantage_head,
        })
    }
}

impl Module for QNetV2 {
    fn forward(&self, input_batch: &Tensor) -> candle_core::Result<Tensor> {
        let backbone_features = self.backbone.forward(input_batch)?;
        let values = self.value_head.forward(&backbone_features)?;
        let advantages = self.advantage_head.forward(&backbone_features)?;
        values
            .broadcast_sub(&advantages.mean(D::Minus1)?)?
            .broadcast_add(&advantages)
    }
}
