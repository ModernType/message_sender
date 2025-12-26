mod deserialize;
mod format;

#[allow(unused)]
pub use deserialize::{Message as OperatorMessage, MessageInner};
pub use format::parse_message_with_format;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub enum SendMode {
    #[default]
    Off,
    Normal,
    Frequency
}

impl SendMode {
    pub fn next(self) -> Self {
        match self {
            SendMode::Off => Self::Normal,
            SendMode::Normal => Self::Frequency,
            SendMode::Frequency => Self::Off,
        }
    }

    pub fn active(self) -> bool {
        if let Self::Off = self {
            false
        }
        else {
            true
        }
    }
}
