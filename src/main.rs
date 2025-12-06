use std::{
    fs::File,
    net::SocketAddrV4,
    str::FromStr,
    time::Duration,
};

use futures::SinkExt;
use log::info;
use simplelog::Config;
use slint::{Timer, ToSharedString};

use crate::{accept_server::start_server_thread, signal_actions::start_signal_thread};
use signal_actions::SignalAction;
use app_state::APP_STATE;

#[macro_use]
mod helpers;
mod accept_server;
mod message;
mod observable;
mod signal_actions;
mod app_state;
#[cfg(test)]
mod test;

slint::include_modules!();


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

    // Setup snackbar callbacks
    let app_weak = app.as_weak();
    let timer = Timer::default();
    app.on_report_close_prepare(move |dur| {
        let app_weak = app_weak.clone();
        let dur = Duration::from_millis(dur as u64);
        timer.start(slint::TimerMode::SingleShot, dur, move || app_weak.unwrap().invoke_report_close());
    });

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
        app_state.cached_groups.get_mut(&group.to_string()).unwrap().active = state;
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
    app.on_send_timeout_changed(|timeout| {
        let mut state = APP_STATE.lock().unwrap();
        state.send_timeout = timeout;
    });
    app.on_markdown_change(|markdown| {
        let mut state = APP_STATE.lock().unwrap();
        state.markdown = markdown;
    });
    app.on_parallel_send_changed(|par_send| {
        let mut state = APP_STATE.lock().unwrap();
        state.parallel_send = par_send;
    });

    // Set initial ip address in field from save. Use scope to automatically drop MutexGuard
    {
        let state = APP_STATE.lock().unwrap();
        let ip = state.recieve_address.to_shared_string();
        app.set_listener_ip(ip);
        app.set_autosend(state.autosend);
        app.set_send_mode(state.send_mode);
        app.set_sync_interval(state.sync_interval);
        app.set_send_timeout(state.send_timeout);
        app.set_markdown(state.markdown);
        app.set_parallel_send(state.parallel_send);
    }

    _ = app.run();
    tx.close_channel();
    let state = APP_STATE.lock().unwrap();
    _ = state.save();
}
