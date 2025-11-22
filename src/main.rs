use std::{
    collections::HashMap,
    fmt::Debug,
    fs::{File, OpenOptions},
    net::SocketAddrV4,
    str::FromStr,
    sync::{LazyLock, Mutex},
};

use futures::SinkExt;
use log::info;
use presage::{Manager, manager::Registered};
use presage_store_sqlite::SqliteStore;
use serde::{Deserialize, Serialize};
use simplelog::Config;
use slint::{SharedString, Timer, ToSharedString};

use crate::{accept_server::start_server_thread, signal_actions::start_signal_thread};
use signal_actions::SignalAction;

mod accept_server;
mod message;
mod observable;
mod signal_actions;
#[cfg(test)]
mod test;

slint::include_modules!();

#[derive(Debug, Serialize, Deserialize)]
#[serde(default)]
struct AppState {
    #[serde(skip)]
    manager: Option<Manager<SqliteStore, Registered>>,
    #[serde(skip)]
    cached_groups: HashMap<String, [u8; 32]>,
    #[serde(skip)]
    #[allow(dead_code)]
    error_queue: Vec<String>,
    group_active: HashMap<SharedString, bool>,
    recieve_address: SocketAddrV4,
    autosend: bool,
    send_mode: SendMode,
    sync_interval: i32,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            manager: None,
            error_queue: Vec::new(),
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

static APP_STATE: LazyLock<Mutex<AppState>> =
    LazyLock::new(|| Mutex::new(AppState::load().unwrap_or_default()));

fn main() {
    let log_file = File::create("sender.log").unwrap();
    simplelog::CombinedLogger::init(vec![
        simplelog::TermLogger::new(
            log::LevelFilter::Info,
            Config::default(),
            simplelog::TerminalMode::Mixed,
            simplelog::ColorChoice::Auto,
        ),
        simplelog::WriteLogger::new(log::LevelFilter::Info, Config::default(), log_file),
    ])
    .unwrap();
    let app = App::new().unwrap();

    let (tx, rx) = futures::channel::mpsc::unbounded::<SignalAction>();

    let app_handle = app.as_weak();
    let _runtime_thread = start_signal_thread(app_handle, rx);

    let tx_clone = tx.clone();
    let app_handle = app.as_weak();
    let _server_runtime = start_server_thread(tx_clone, app_handle);

    let mut tx_clone = tx.clone();
    app.on_start_link(move || {
        futures::executor::block_on(tx_clone.send(SignalAction::LinkBegin)).unwrap()
    });
    app.invoke_start_link();

    let mut tx_clone = tx.clone();
    app.on_sync(move || {
        info!("Sending sync signal");
        futures::executor::block_on(tx_clone.send(SignalAction::Sync)).unwrap()
    });
    let mut tx_clone = tx.clone();
    app.on_get_groups(move || {
        info!("Sending get_groups signal");
        futures::executor::block_on(tx_clone.send(SignalAction::GetGroups)).unwrap()
    });
    app.on_group_edited(|group, state| {
        let mut app_state = APP_STATE.lock().unwrap();
        app_state.group_active.insert(group, state);
    });
    let mut tx_clone = tx.clone();
    app.on_send_message(move |message| {
        info!("Sending send_message signal");
        futures::executor::block_on(tx_clone.send(SignalAction::SendMessage(message.to_string())))
            .unwrap()
    });
    app.on_check_ip_correct(|text| match SocketAddrV4::from_str(text.as_str()) {
        Ok(addr) => {
            let mut state = APP_STATE.lock().unwrap();
            state.recieve_address = addr;
            true
        }
        Err(_) => false,
    });
    app.on_autosend_change(|check| {
        let mut state = APP_STATE.lock().unwrap();
        state.autosend = check;
    });
    app.on_send_mode_change(|mode| {
        let mut state = APP_STATE.lock().unwrap();
        state.send_mode = mode;
    });
    app.on_sync_interval_changed(|interval| {
        let mut state = APP_STATE.lock().unwrap();
        state.sync_interval = interval;
    });

    // Set initial ip address in field from save. Use scope to automatically drop MutexGuard
    {
        let state = APP_STATE.lock().unwrap();
        let ip = state.recieve_address.to_shared_string();
        app.set_listener_ip(ip);
        app.set_autosend(state.autosend);
        app.set_send_mode(state.send_mode);
        app.set_sync_interval(state.sync_interval);
    }

    _ = app.run();
    tx.close_channel();
    let state = APP_STATE.lock().unwrap();
    _ = state.save();
}
