mod deserialize;
use std::collections::{HashMap, HashSet};

pub use deserialize::{NetworkInfo, parse_networks_data};
use serde::{Deserialize, Serialize};

use crate::{message::SendMode, messangers::Key};

pub type NetworksPool = HashMap<String, NetworkInfo>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendCategory {
    name: String,
    pub networks: HashMap<String, bool>,
    pub groups: HashMap<Key, SendMode>,
}

impl SendCategory {
    pub fn new(name: String) -> Self {
        Self {
            name,
            networks: HashMap::new(),
            groups: HashMap::new(),
        }
    }

    pub fn match_network_by_name(&self, name: &String) -> bool {
        self.networks.contains_key(name)
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}


