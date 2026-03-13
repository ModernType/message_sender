mod deserialize;
use std::collections::{HashMap, HashSet};

pub use deserialize::{NetworkInfo, parse_networks_data};
use serde::Serialize;
use serde_versioning::Deserialize;

use crate::{message::SendMode, messangers::Key};

pub type NetworksPool = HashMap<u64, NetworkInfo>;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[versioning(optimistic, previous_version = "SendCategoryOld")]
pub struct SendCategory {
    name: String,
    pub use_general: bool,
    pub parameters: Parameters,
    pub groups: HashMap<Key, SendMode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendCategoryOld {
    name: String,
    pub use_general: bool,
    pub networks: Vec<u64>,
    pub groups: HashMap<Key, SendMode>,
}

impl From<SendCategoryOld> for SendCategory {
    fn from(value: SendCategoryOld) -> Self {
        Self {
            name: value.name,
            use_general: value.use_general,
            parameters: Parameters::Networks(value.networks),
            groups: value.groups,
        }
    }
}

impl SendCategory {
    pub fn new(name: String) -> Self {
        Self {
            name,
            use_general: true,
            parameters: Parameters::Networks(Vec::new()),
            groups: HashMap::new(),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn contains_network(&self, id: &u64) -> bool {
        if let Parameters::Networks(networks) = &self.parameters {
            networks.contains(id)
        }
        else {
            false
        }
    }

    pub fn contains_source(&self, source: &String) -> bool {
        if let Parameters::Sources(sources) = &self.parameters {
            sources.contains(source)
        }
        else {
            false
        }
    }

    pub fn contains_comment(&self, comment: &String) -> bool {
        if let Parameters::Comments(comments) = &self.parameters {
            comments.contains(comment)
        }
        else {
            false
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Parameters {
    Networks(Vec<u64>),
    Sources(HashSet<String>),
    Comments(HashSet<String>),
}
