//! Defines the Pacman agent's high level AI.

use crate::grid::ComputedGrid;
use crate::grid::IntLocation;
use crate::pathing::TargetPath;
use crate::util::stopwatch::Stopwatch;
use crate::{PacmanGameState, UserSettings};
use bevy::prelude::*;
use candle_core::D;
use candle_core::{Device, Module, Tensor};
use candle_nn as nn;
use ndarray::{s, Array};
use pacbot_rs::game_engine::GameEngine;
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
        .init_non_send_resource::<HighLevelContext>();
    }
}

/// Tracks the time AI takes to make decisions
#[derive(Resource)]
pub struct AiStopwatch(pub Stopwatch);

pub fn run_high_level(
    game_state: Res<PacmanGameState>,
    mut target_path: ResMut<TargetPath>,
    mut hl_ctx: NonSendMut<HighLevelContext>,
    std_grid: Local<ComputedGrid>,
    settings: Res<UserSettings>,
    mut ai_stopwatch: ResMut<AiStopwatch>,
) {
    if settings.enable_ai && !game_state.0.is_paused() && game_state.is_changed() {
        // If ghosts are in an invalid state, don't run the AI
        if game_state
            .0
            .get_state()
            .ghosts
            .iter()
            .map(|g| g.read().unwrap().loc)
            .any(|loc| loc.col > 28 || loc.row > 31)
        {
            return;
        }

        ai_stopwatch.0.start();

        let mut path = vec![];
        let sim_engine =
            bincode::serde::encode_to_vec(&game_state.0, bincode::config::standard()).unwrap();
        let mut sim_engine: GameEngine =
            bincode::serde::decode_from_slice(&sim_engine, bincode::config::standard())
                .unwrap()
                .0;
        let mut curr_pos = IntLocation {
            row: sim_engine.get_state().pacman_loc.row,
            col: sim_engine.get_state().pacman_loc.col,
        };
        for _ in 0..6 {
            let action = hl_ctx.step(sim_engine.get_state(), &std_grid);
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
            curr_pos = target_pos;
            path.push(target_pos);
        }
        target_path.0 = path;

        ai_stopwatch.0.mark_segment("AI");
    }
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

const OBS_SHAPE: (usize, usize, usize) = (15, 28, 31);

/// Handles executing high level AI.
pub struct HighLevelContext {
    net: QNetV2,
    // These `cached` variables contain the last observed positions.
    // Once these cached positions are different from the next observed positions, the `last_variables`
    // are updated with these.
    pos_cached: (usize, usize),
    ghost_pos_cached: Vec<(usize, usize)>,
    last_pos: (usize, usize),
    last_ghost_pos: Vec<(usize, usize)>,
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
            last_pos: (0, 0),
            last_ghost_pos: vec![(0, 0), (0, 0), (0, 0), (0, 0)],
            pos_cached: (0, 0),
            ghost_pos_cached: vec![(0, 0), (0, 0), (0, 0), (0, 0)],
        }
    }

    /// Runs one step of the high level AI.
    /// Returns the action the AI has decided to take.
    // Currently, this implements a DQN approach.
    fn step(&mut self, game_state: &GameState, grid: &ComputedGrid) -> HLAction {
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

        // Account for old walls
        for row in 0..9 {
            for col in 0..5 {
                wall[(col, 12 + row)] = 0.;
                wall[(31 - 9 + 1 + col, 12 + row)] = 0.;
            }
        }
        for row in 27..28 {
            for col in 3..5 {
                wall[(col, row)] = 0.;
                wall[(28 - 1 - col, row)] = 0.;
            }
        }
        for row in 27..28 {
            for col in 8..11 {
                wall[(col, row)] = 0.;
                wall[(28 - 1 - col, row)] = 0.;
            }
        }

        let pac_pos = game_state.pacman_loc;

        // Save last positions.
        let new_pos_cached = (pac_pos.col as usize, 31 - pac_pos.row as usize - 1);
        let new_ghost_pos_cached: Vec<_> = game_state
            .ghosts
            .iter()
            .map(|g| g.read().unwrap())
            .map(|g| (g.loc.col as usize, 31 - g.loc.row as usize - 1))
            .collect();

        if self.pos_cached == (32, 32) {
            self.last_pos = new_pos_cached;
            self.pos_cached = new_pos_cached;
        }

        if self.ghost_pos_cached.contains(&(32, 32)) {
            self.last_ghost_pos = new_ghost_pos_cached.clone();
            self.ghost_pos_cached = new_ghost_pos_cached.clone();
        }

        if new_pos_cached != self.pos_cached {
            self.last_pos = self.pos_cached;
            self.pos_cached = new_pos_cached;
        }

        if new_ghost_pos_cached != self.ghost_pos_cached {
            self.last_ghost_pos = self.ghost_pos_cached.clone();
            self.ghost_pos_cached = new_ghost_pos_cached;
        }

        // I think (32, 32) is the shadow realm
        if pac_pos.col != 32 && self.last_pos.0 != 32 {
            pacman[(0, self.last_pos.0, self.last_pos.1)] = 1.0;
            pacman[(1, pac_pos.col as usize, 31 - pac_pos.row as usize - 1)] = 1.0;

            for (i, g) in game_state.ghosts.iter().enumerate() {
                let g = g.read().unwrap();
                let pos = g.loc;
                let col = pos.col as usize;
                let row = 31 - pos.row as usize - 1;
                if pos.col != 32 {
                    ghost[(i, col, row)] = 1.0;
                    if g.is_frightened() {
                        state[(2, col, row)] = g.fright_steps as f32 / GHOST_FRIGHT_STEPS as f32;
                        reward[(col, row)] += 1.;
                    } else {
                        let state_index = if game_state.mode == GameMode::CHASE {
                            1
                        } else {
                            0
                        };
                        state[(state_index, col, row)] = 1.0;
                    }
                }
            }
        }

        for (i, pos) in self.last_ghost_pos.iter().enumerate() {
            if pos.0 != 32 {
                last_ghost[(i, pos.0, pos.1)] = 1.0;
            }
        }

        // Create action mask.
        let mut action_mask = [false, false, false, false, false];
        if let Some(valid_actions) = grid.valid_actions(IntLocation::new(pac_pos.row, pac_pos.col))
        {
            // The order of valid actions is stay, up, left, down, right
            action_mask = [
                !valid_actions[0],
                !valid_actions[1],
                !valid_actions[3],
                !valid_actions[2],
                !valid_actions[4],
            ];
        }
        let action_mask =
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

        let q_vals = ((q_vals * (1. - &action_mask).unwrap()).unwrap()
            + (&action_mask * -999.).unwrap())
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
            HLAction::Up,
            HLAction::Down,
            HLAction::Left,
            HLAction::Right,
        ];
        actions[q_vals
            .argmax(candle_core::D::Minus1)
            .unwrap()
            .to_scalar::<u32>()
            .unwrap() as usize]
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
