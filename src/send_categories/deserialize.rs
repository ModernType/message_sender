use std::collections::HashMap;

use serde::{Deserialize, Serialize};


pub fn parse_networks_data(s: &str) -> serde_json::Result<HashMap<u64, NetworkInfo>> {
    let infos = serde_json::from_str::<Vec<FullNetworkInfo>>(s)?;
    let mut map = HashMap::new();
    for info in infos {
        map.insert(info.id, info.into());
    }

    Ok(map)
}

#[derive(Debug, Clone, Hash, Serialize, Deserialize, PartialEq, Eq)]
pub struct NetworkInfo {
    pub id: u64,
    pub freq: String,
    pub crypt_mode: String,
    pub name: String,
}


impl NetworkInfo {
    pub fn new(id: u64, name: String) -> Self {
        Self {
            id, 
            name,
            crypt_mode: String::new(),
            freq: String::new(),
        }
    }
}

impl From<FullNetworkInfo> for NetworkInfo {
    fn from(value: FullNetworkInfo) -> Self {
        Self {
            id: value.id,
            freq: value.frequency_str,
            crypt_mode: value.crypt_mode_str,
            name: value.network_name,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
#[serde(rename_all = "PascalCase", default)]
struct FullNetworkInfo {
    #[serde(rename = "ID")]
    id: u64,
    frequency: u64,
    frequency_color_schema: u8,
    transmission: u8,
    crypt_mode2: u8,
    crypt_keys: Vec<String>,
    hardware_location: Option<String>,
    abonent_ids: Vec<String>,
    creative_param: Option<String>,
    network_name: String,
    source_location: String,
    /* ┌ // TODO: Use proper datetime types if needed later */
    /* | */ creating_date: String,
    /* | */ change_date: String,
    /* | */ last_session_date_time: String,
    /* └--------------------------------------------------  */
    frequency_str: String,
    frequency_color_string: String,
    transmission_str: String,
    crypt_mode_str: String,
}