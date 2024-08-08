use core_pb::messages::settings::KnownRLModel;
use core_pb::pacbot_rs::game_state::GameState;
use core_pb::pacbot_rs::location::Direction;
use rl_pb::candle_inference::CandleInference;
use rl_pb::env::PacmanGymConfiguration;
use std::collections::HashMap;

pub fn model_configuration(_model: KnownRLModel) -> PacmanGymConfiguration {
    PacmanGymConfiguration {
        random_start: false,
        random_ticks: false,
        randomize_ghosts: false,
        remove_super_pellets: false,
    }
}

#[derive(Default)]
pub struct ReinforcementLearningManager {
    models: HashMap<KnownRLModel, CandleInference>,
}

impl ReinforcementLearningManager {
    pub fn do_inference(
        &mut self,
        model: KnownRLModel,
        game_state: GameState,
        advanced_action_mask: bool,
        ticks_per_step: u32,
    ) -> Direction {
        let candle_inference = self
            .models
            .entry(model)
            .or_insert_with(|| CandleInference::new(model.path(), model_configuration(model)));
        let action_mask = if advanced_action_mask {
            Some(CandleInference::complex_action_mask(&game_state, 3))
        } else {
            None
        };
        candle_inference
            .get_actions(game_state, action_mask, ticks_per_step)
            .0
    }

    pub fn hybrid_strategy(&mut self, game_state: GameState) -> Direction {
        // todo make this switch between models
        self.do_inference(KnownRLModel::Endgame, game_state, true, 6)
    }
}
