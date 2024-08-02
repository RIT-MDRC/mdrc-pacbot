use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Indicates the last completed action
#[derive(Copy, Clone, Debug, Default, Serialize, Deserialize, PartialOrd, PartialEq)]
#[repr(usize)]
pub enum OverTheAirStep {
    #[default]
    GuiRequest = 0,
    RobotReadyConfirmation = 1,
    FetchBinary = 2,
    DataTransfer {
        received: usize,
        total: usize,
    } = 3,
    HashConfirmation = 4,
    GuiConfirmation = 5,
    MarkUpdateReady = 6,
    Reboot = 7,
    CheckFirmwareSwapped = 8,
    FinalGuiConfirmation = 9,
    MarkUpdateBooted = 10,
    Finished = 11,
    Failed = 12,
}

impl From<OverTheAirStep> for usize {
    fn from(value: OverTheAirStep) -> Self {
        match value {
            OverTheAirStep::GuiRequest => 0,
            OverTheAirStep::RobotReadyConfirmation => 1,
            OverTheAirStep::FetchBinary => 2,
            OverTheAirStep::DataTransfer { .. } => 3,
            OverTheAirStep::HashConfirmation => 4,
            OverTheAirStep::GuiConfirmation => 5,
            OverTheAirStep::MarkUpdateReady => 6,
            OverTheAirStep::Reboot => 7,
            OverTheAirStep::CheckFirmwareSwapped => 8,
            OverTheAirStep::FinalGuiConfirmation => 9,
            OverTheAirStep::MarkUpdateBooted => 10,
            OverTheAirStep::Finished => 11,
            OverTheAirStep::Failed => 12,
        }
    }
}

impl From<usize> for OverTheAirStep {
    fn from(value: usize) -> Self {
        match value {
            0 => OverTheAirStep::GuiRequest,
            1 => OverTheAirStep::RobotReadyConfirmation,
            2 => OverTheAirStep::FetchBinary,
            3 => OverTheAirStep::DataTransfer {
                received: 0,
                total: 0,
            },
            4 => OverTheAirStep::HashConfirmation,
            5 => OverTheAirStep::GuiConfirmation,
            6 => OverTheAirStep::MarkUpdateReady,
            7 => OverTheAirStep::Reboot,
            8 => OverTheAirStep::CheckFirmwareSwapped,
            9 => OverTheAirStep::FinalGuiConfirmation,
            10 => OverTheAirStep::MarkUpdateBooted,
            11 => OverTheAirStep::Finished,
            12 => OverTheAirStep::Failed,
            _ => OverTheAirStep::default(),
        }
    }
}

impl OverTheAirStep {
    pub fn terminated(&self) -> bool {
        match self {
            OverTheAirStep::Failed | OverTheAirStep::Finished => true,
            _ => false,
        }
    }

    pub fn message(&self) -> String {
        match self {
            OverTheAirStep::GuiRequest => "GUI request".into(),
            OverTheAirStep::RobotReadyConfirmation => "Robot prepare update".into(),
            OverTheAirStep::FetchBinary => "Fetch binary".into(),
            OverTheAirStep::DataTransfer { received, total } => format!(
                "Upload ({received}/{total}, {:.1}%)",
                100.0 * *received as f32 / *total as f32
            ),
            OverTheAirStep::HashConfirmation => "Matching hash (NOT CHECKED)".into(),
            OverTheAirStep::GuiConfirmation => "Gui go-ahead".into(),
            OverTheAirStep::MarkUpdateReady => "Mark update ready".into(),
            OverTheAirStep::Reboot => "Reboot robot".into(),
            OverTheAirStep::CheckFirmwareSwapped => "Check successful swap".into(),
            OverTheAirStep::FinalGuiConfirmation => "Final gui confirmation".into(),
            OverTheAirStep::MarkUpdateBooted => "Mark update booted".into(),
            OverTheAirStep::Finished => "Finished".into(),
            OverTheAirStep::Failed => "Failed".into(),
        }
    }
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize, PartialOrd, PartialEq)]
pub struct OverTheAirStepCompletion {
    pub step: OverTheAirStep,
    pub since_beginning: Duration,
    pub success: Option<bool>,
}
