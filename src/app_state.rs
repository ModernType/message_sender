use std::{
    collections::{HashMap, VecDeque}, fmt::Debug, fs::{File, OpenOptions}, net::SocketAddrV4, sync::{LazyLock, Mutex}
};
use serde::{Serialize, Deserialize};
use slint::SharedString;

use crate::SendMode;

#[derive(Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct AppState {
    pub cached_groups: HashMap<String, GroupData>,
    #[serde(skip)]
    user_notification_queue: VecDeque<SharedString>,
    pub recieve_address: SocketAddrV4,
    pub autosend: bool,
    pub send_mode: SendMode,
    pub sync_interval: i32,
    pub send_timeout: i32,
    pub markdown: bool,
}

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct GroupData {
    #[serde(skip)]
    pub key: Option<[u8; 32]>,
    pub active: bool,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            user_notification_queue: VecDeque::new(),
            cached_groups: HashMap::new(),
            recieve_address: "127.0.0.1:8000".parse().unwrap(),
            autosend: false,
            send_mode: SendMode::Standard,
            sync_interval: 60,
            send_timeout: 30,
            markdown: true,
        }
    }
}

impl AppState {
    pub fn load() -> anyhow::Result<Self> {
        let data = File::open("data.json")?;
        let state = serde_json::from_reader(data)?;
        Ok(state)
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let config_file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open("data.json")?;
        serde_json::to_writer_pretty(config_file, self)?;
        Ok(())
    }
}

pub static APP_STATE: LazyLock<Mutex<AppState>> =
    LazyLock::new(|| Mutex::new(AppState::load().unwrap_or_default()));
