use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Serialize, Deserialize)]
pub enum LocalizationAlgorithmSource {
    #[default]
    RegionLocalization,
    CVAdjust,
    CorridorPolicyChange,
}