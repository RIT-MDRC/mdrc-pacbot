use crate::{constants, game_state::PacmanState, grid::GridValue, standard_grids::GRID_PACMAN};
use candle_core::{Device, Module, Tensor};
use candle_nn as nn;
use ndarray::{s, Array};

/// Represents an action the AI can choose to perform.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum HLAction {
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
    last_pos: (usize, usize),
    last_ghost_pos: Vec<(usize, usize)>,
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
        }
    }

    /// Runs one step of the high level AI.
    /// Returns the action the AI has decided to take.
    // Currently, this implements a DQN approach.
    pub fn step(&mut self, game_state: &PacmanState) -> HLAction {
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

        for ((grid_value, reward_value), wall_value) in GRID_PACMAN
            .iter()
            .flatten()
            .zip(reward.iter_mut())
            .zip(wall.iter_mut())
        {
            *wall_value = (*grid_value == GridValue::I || *grid_value == GridValue::n) as u8 as f32;
            *reward_value = match grid_value {
                GridValue::o => constants::PELLET_SCORE,
                GridValue::O => constants::POWER_PELLET_SCORE,
                GridValue::c => constants::CHERRY_SCORE,
                _ => 0,
            } as f32
                / constants::GHOST_SCORE as f32;
        }

        let pac_pos = game_state.pacman.location;
        pacman[(0, self.last_pos.0, self.last_pos.1)] = 1.0;
        pacman[(1, pac_pos.x as usize, pac_pos.y as usize)] = 1.0;

        for (i, g) in game_state.ghosts.iter().enumerate() {
            let pos = g.agent.location;
            ghost[(i, pos.x as usize, pos.y as usize)] = 1.0;
            let is_frightened = g.frightened_counter > 0; // TODO: Check if this is correct
            if is_frightened {
                state[(2, pos.x as usize, pos.y as usize)] =
                    g.frightened_counter as f32 / constants::FRIGHTENED_LENGTH as f32;
            } else {
                // let state_index = if game_state.game_state == GameStateState::Chase {
                //     1
                // } else {
                //     0
                // };
                // TODO: Implement checking for chase state
                let state_index = 1;
                state[(state_index, pos.x as usize, pos.y as usize)] = 1.0;
            }
        }

        for (i, pos) in self.last_ghost_pos.iter().enumerate() {
            last_ghost[(i, pos.0, pos.1)] = 1.0;
        }

        // Save last positions
        self.last_pos = (pac_pos.x as usize, pac_pos.y as usize);
        self.last_ghost_pos = game_state
            .ghosts
            .iter()
            .map(|g| (g.agent.location.x as usize, g.agent.location.y as usize))
            .collect();

        // Run observation through model and generate action.
        let obs_flat = obs_array.as_slice().unwrap();
        let obs_tensor = Tensor::from_slice(obs_flat, OBS_SHAPE, &Device::Cpu)
            .unwrap()
            .unsqueeze(0)
            .unwrap();
        let q_vals = self.net.forward(&obs_tensor).unwrap().squeeze(0).unwrap();
        let actions = [
            HLAction::Stay,
            HLAction::Down,
            HLAction::Up,
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
        .add(nn::func(|x| x.max_pool2d(2)))
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
            .add(nn::func(|x| x.flatten(1, candle_core::D::Minus1)))
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
        let value = self.value_head.forward(&backbone_features)?;
        let advantages = self.advantage_head.forward(&backbone_features)?;
        value.broadcast_sub(&advantages.mean_keepdim(1)?.broadcast_add(&advantages)?)
    }
}
