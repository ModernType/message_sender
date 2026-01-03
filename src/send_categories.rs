mod deserialize;
use std::collections::{HashMap, HashSet};

pub use deserialize::{NetworkInfo, parse_networks_data};
use serde::{Deserialize, Serialize};

use crate::{message::SendMode, ui::main_screen::Group};

pub type NetworksPool = HashMap<u64, NetworkInfo>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendCategory {
    name: String,
    pub networks: HashSet<NetworkInfo>,
    pub groups: HashSet<Group>,
}

impl SendCategory {
    pub fn new(name: String) -> Self {
        Self {
            name,
            networks: HashSet::new(),
            groups: HashSet::new(),
        }
    }
}


