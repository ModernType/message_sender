use std::{
    collections::{HashMap, VecDeque}, fmt::Debug, fs::{File, OpenOptions}, net::SocketAddrV4, sync::{LazyLock, Mutex,}
};
use serde::{Serialize, Deserialize};
use presage::{Manager, manager::Registered};
use presage_store_sqlite::SqliteStore;
use slint::SharedString;

use crate::SendMode;

#[derive(Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct AppState {
    #[serde(skip)]
    manager: Option<Manager<SqliteStore, Registered>>,
    #[serde(skip)]
    pub cached_groups: HashMap<String, [u8; 32]>,
    #[serde(skip)]
    user_notification_queue: VecDeque<SharedString>,
    pub group_active: HashMap<SharedString, bool>,
    pub recieve_address: SocketAddrV4,
    pub autosend: bool,
    pub send_mode: SendMode,
    pub sync_interval: i32,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            manager: None,
            user_notification_queue: VecDeque::new(),
            cached_groups: HashMap::new(),
            group_active: HashMap::new(),
            recieve_address: "127.0.0.1:8000".parse().unwrap(),
            autosend: false,
            send_mode: SendMode::Standard,
            sync_interval: 60,
        }
    }
}

impl AppState {
    pub fn set_manager(&mut self, manager: Manager<SqliteStore, Registered>) {
        self.manager = Some(manager);
    }

    pub fn manager(&self) -> Option<&Manager<SqliteStore, Registered>> {
        self.manager.as_ref()
    }

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
