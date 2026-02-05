use derive_more::From;
use iced::widget::svg;
use serde::{Deserialize, Serialize};
use wacore_binary::jid::Jid;

use crate::ui::icons::{SIGNAL_ICON, WHATSAPP_ICON};

pub mod signal;
pub mod whatsapp;


#[derive(Debug, From, Serialize, Deserialize, PartialEq, Eq, Hash, Clone)]
pub enum Key {
    Signal([u8; 32]),
    Whatsapp(Jid),
}

impl PartialOrd for Key {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        fn val(v: &Key) -> usize {
            match v {
                Key::Signal(_) => 1,
                Key::Whatsapp(_) => 0,
            }
        }

        Some(val(self).cmp(&val(other)))
    }
}

impl Ord for Key {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.partial_cmp(other).unwrap()
    }
}

impl Key {
    pub fn icon(&self) -> svg::Svg<'static> {
        let bytes = match self {
            Self::Signal(_) => SIGNAL_ICON,
            Self::Whatsapp(_) => WHATSAPP_ICON,
        };
        svg(svg::Handle::from_memory(bytes))
    }
}
