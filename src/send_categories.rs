mod deserialize;
use std::collections::HashMap;

pub use deserialize::{NetworkInfo, parse_networks_data};
use serde::{Deserialize, Serialize};

use crate::{message::SendMode, messangers::Key};

pub type NetworksPool = HashMap<u64, NetworkInfo>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendCategory {
    name: String,
    pub use_general: bool,
    pub networks: Vec<u64>,
    pub groups: HashMap<Key, SendMode>,
}

impl SendCategory {
    pub fn new(name: String) -> Self {
        Self {
            name,
            use_general: true,
            networks: Vec::new(),
            groups: HashMap::new(),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn contains_network(&self, id: &u64) -> bool {
        self.networks.contains(id)
    }
}


