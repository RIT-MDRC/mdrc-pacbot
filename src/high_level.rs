use candle_core::{Module, Tensor};
use candle_nn as nn;

use crate::{agent_setup::PacmanAgentSetup, game_state::PacmanState};

/// Represents an action the AI can choose to perform.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum HLAction {
    Left,
    Right,
    Up,
    Down,
}

/// Handles executing high level AI.
pub struct HighLevelContext {
    net: QNetV2,
}

impl HighLevelContext {
    /// Creates a new instance of the high level AI.
    pub fn new() -> Self {
        let vm = nn::VarMap::new();
        let vb =
            nn::VarBuilder::from_varmap(&vm, candle_core::DType::F32, &candle_core::Device::Cpu);
        let net = QNetV2::new(candle_core::Shape::from_dims(&[3, 32, 28]), 4, vb).unwrap();
        Self { net }
    }

    /// Runs one step of the high level AI.
    /// Returns the action the AI has decided to take.
    pub fn step(&self, game_state: &PacmanState) -> HLAction {
        // Currently, this implements a DQN approach.
        // TODO: Actually do this.
        HLAction::Down
    }
}

impl Default for HighLevelContext {
    fn default() -> Self {
        Self::new()
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
            vb.pp("conv"),
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
        let backbone = nn::seq()
            .add(nn::conv2d(
                obs_channels,
                16,
                5,
                nn::Conv2dConfig {
                    padding: 2,
                    ..Default::default()
                },
                vb.pp("conv1"),
            )?)
            .add(nn::Activation::Silu)
            .add(conv_block_pool(16, 32, vb.pp("conv2"))?)
            .add(conv_block_pool(32, 64, vb.pp("conv3"))?)
            .add(conv_block_pool(64, 128, vb.pp("conv4"))?)
            .add(nn::conv2d(
                128,
                128,
                3,
                nn::Conv2dConfig {
                    padding: 1,
                    groups: 128 / 16,
                    ..Default::default()
                },
                vb.pp("conv5"),
            )?)
            // TODO: Figure out how to get this operation working
            // nn.AdaptiveMaxPool2d((1, 1)),
            .add(nn::func(|x| x.flatten(1, candle_core::D::Minus1)))
            .add(nn::Activation::Silu)
            .add(nn::linear(128, 256, vb.pp("l1"))?)
            .add(nn::Activation::Silu);
        let value_head = nn::seq().add(nn::linear(256, 1, vb.pp("val"))?);
        let advantage_head = nn::seq().add(nn::linear(256, action_count, vb.pp("adv"))?);

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
        value - advantages.mean_keepdim(1)? + advantages
    }
}
