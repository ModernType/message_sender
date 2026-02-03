use derive_more::From;
use serde::{Deserialize, Serialize};
use wacore_binary::jid::Jid;

pub mod signal;
pub mod whatsapp;


#[derive(Debug, From, Serialize, Deserialize, PartialEq, Eq, Hash, Clone)]
pub enum Key {
    Signal([u8; 32]),
    Whatsapp(Jid),
}
