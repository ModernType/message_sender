mod deserialize;
mod format;
mod compose;

#[allow(unused)]
pub use deserialize::{Message as OperatorMessage, MessageInner, TEST_MESSAGE};
pub use format::{parse_message_with_format, parse_message_with_whatsapp_format};
pub use compose::{FormatPart, Formatting};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum SendMode {
    #[default]
    Off,
    Normal,
    Frequency
}

impl SendMode {
    pub fn active(self) -> bool {
        !matches!(self, Self::Off)
    }

    pub fn update(&mut self, other: Self) {
        if other > *self {
            *self = other;
        }
    }
}
