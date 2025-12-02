use std::{
    rc::Rc,
    time::{Duration, SystemTime},
};

use futures::{
    StreamExt,
    channel::mpsc::UnboundedReceiver,
    future::Either,
    pin_mut,
};
use image::Rgb;
use log::{info, warn};
use presage::{
    Manager, libsignal_service::{configuration::SignalServers, content::ContentBody}, manager::Registered, proto::{DataMessage, GroupContextV2}, store::ContentsStore
};
use presage_store_sqlite::{OnNewIdentity, SqliteConnectOptions, SqliteStore, SqliteStoreError};
use slint::{ModelRc, SharedPixelBuffer, SharedString, ToSharedString, VecModel, Weak};
use tokio::task::LocalSet;

use crate::{app_state::APP_STATE, App, Group};

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum SignalAction {
    LinkBegin,
    Sync,
    GetGroups,
    SendMessage(String),
}

pub fn start_signal_thread(
    app_handle: Weak<App>,
    mut rx: UnboundedReceiver<SignalAction>,
) -> std::thread::JoinHandle<()> {
    let runtime_builder = std::thread::Builder::new().stack_size(32 * 1024 * 1024);
    runtime_builder
        .spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("None runtimes shall be constructed before");

            let local = LocalSet::new();
            let mut manager: Option<Manager<SqliteStore, Registered>> = None;
            
            local.spawn_local(async move {
                while let Some(action) = rx.next().await {
                    let app_loop = app_handle.clone();
                    let mng  = manager.clone();
                    _ = match action {
                        SignalAction::LinkBegin => {
                            let mng = link(app_loop.clone()).await;
                            match mng {
                                Ok(mng) => {
                                    manager = Some(mng.clone());
                                    tokio::task::spawn_local(sync(app_loop, mng));
                                },
                                Err(e) => {
                                    log::error!("Linking error: {e}");
                                    // _ = app_loop.upgrade_in_event_loop(|app| {
                                    //     app.invoke_cancel_link();
                                    // });
                                }
                            };
                        },
                        SignalAction::Sync => {},
                        SignalAction::GetGroups => { tokio::task::spawn_local(get_groups(app_loop)); },
                        SignalAction::SendMessage(message) => {
                            if let Some(manager) = mng {
                                _ = send_message(message, app_loop, manager).await;
                            }
                        },
                    };
                }
            });

            rt.block_on(local)
        })
        .expect("Failed to initiate Signal worker thread")
}

async fn get_store() -> Result<SqliteStore, SqliteStoreError> {
    let store = SqliteStore::open_with_options(
        SqliteConnectOptions::default()
            .filename("signal_data.db")
            .create_if_missing(true),
        OnNewIdentity::Trust,
    )
    .await?;
    // Clearing all messages to free space
    // _ = store.clear_messages().await;
    Ok(store)
}

async fn update_group_map(manager: &Manager<SqliteStore, Registered>) {
    let mut state = APP_STATE.lock().unwrap();
    let group_map = &mut state.cached_groups;
    for (key, group) in manager.store().groups().await.unwrap().flatten() {
        group_map.insert(group.title, key);
    }
}

async fn link(app_handle: Weak<App>) -> anyhow::Result<Manager<SqliteStore, Registered>> {
    let store = get_store().await?;
    info!("Registering from store");
    match Manager::load_registered(store).await {
        Ok(mng) => {
            app_handle
                .clone()
                .upgrade_in_event_loop(move |app| {
                    app.invoke_linked();
                })
                .unwrap();
            update_group_map(&mng).await;
            Ok(mng)
        }
        Err(e) => {
            warn!("{e}");
            let (tx, rx) = futures::channel::oneshot::channel();
            let store = get_store().await?;
            info!("Starting linking with device");
            let app_handle2 = app_handle.clone();
            let (mng_res, _) = futures::future::join(
                async move {
                    match Manager::link_secondary_device(
                        store,
                        SignalServers::Production,
                        "message-sender".to_owned(),
                        tx,
                    )
                    .await
                    {
                        Ok(mng) => {
                            info!("Has manager");
                            update_group_map(&mng).await;
                            app_handle
                                .clone()
                                .upgrade_in_event_loop(|app| {
                                    app.invoke_linked();
                                })
                                .unwrap();
                            Ok(mng)
                        }
                        Err(e) => {
                            warn!("Link failure: {e}");
                            return Err(e)
                        },
                    }
                },
                async move {
                    match rx.await {
                        Ok(mut url) => {
                            url.set_host(Some("linkdevice")).unwrap();
                            let qr = qrcode::QrCode::new(url.as_str().as_bytes()).unwrap();
                            let image = qr.render::<Rgb<u8>>().build();
                            let (width, height) = (image.width(), image.height());
                            info!("Making QR code");
                            app_handle2
                                .upgrade_in_event_loop(move |app| {
                                    let s_image = slint::Image::from_rgb8(
                                        SharedPixelBuffer::clone_from_slice(
                                            image.as_ref(),
                                            width,
                                            height,
                                        ),
                                    );
                                    app.invoke_set_qr(s_image);
                                })
                                .unwrap();
                        }
                        Err(e) => {
                            warn!("QR code future error: {e}")
                        }
                    }
                },
            )
            .await;
            Ok(mng_res?)
        }
    }
}

async fn sync(_app_handle: Weak<App>, mut manager: Manager<SqliteStore, Registered>) -> anyhow::Result<()> {
    let reciever = manager.receive_messages().await?;
    pin_mut!(reciever);
    while let Some(msg) = reciever.next().await {
        match msg {
            presage::model::messages::Received::Contacts => {
                info!("Got contacts");
            }
            presage::model::messages::Received::Content(_) => {
                info!("Got message");
            }
            presage::model::messages::Received::QueueEmpty => {
                update_group_map(&manager).await;
                // _ = app_handle.upgrade_in_event_loop(|app| {
                //     app.invoke_synced();
                // })
                // tokio::time::sleep(Duration::from_millis(5000)).await;
            }
        }
    }
    log::error!("Sync suspended");
    Ok(())
}

async fn get_groups(app_handle: Weak<App>) -> anyhow::Result<()> {
    let state = APP_STATE.lock().unwrap();
    let mut groups = state
        .cached_groups
        .keys()
        .map(SharedString::from)
        .collect::<Vec<_>>();
    groups.sort();
    let groups = groups
        .into_iter()
        .map(|title| {
            let state = state.group_active.get(&title).cloned().unwrap_or(false);
            Group { title, state }
        })
        .collect::<Vec<_>>();
    drop(state);
    app_handle
        .clone()
        .upgrade_in_event_loop(move |app| {
            let v_model = VecModel::from(groups);
            let model = ModelRc::from(Rc::new(v_model));
            app.set_groups(model)
        })
        .unwrap();

    Ok(())
}

async fn send_message(message: String, app_handle: Weak<App>, manager: Manager<SqliteStore, Registered>) -> anyhow::Result<()> {
    let state = APP_STATE.lock().unwrap();
    let key_iter = state
        .group_active
        .iter()
        .filter_map(|(title, send)| send.then(|| state.cached_groups.get(&title.to_string())))
        .flatten()
        .cloned()
        .collect::<Vec<_>>();
    let markdown = state.markdown;
    // We should drop mutex lock before any await point
    drop(state);

    let timestamp = std::time::SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
    dbg!(&message);
    let message = if markdown {
        let (message, ranges) = crate::message::parse_message_with_format(&message)?;
        DataMessage {
            body: Some(message),
            timestamp: Some(timestamp),
            body_ranges: ranges,
            ..Default::default()
        }
    }
    else {
        DataMessage {
            body: Some(message),
            timestamp: Some(timestamp),
            ..Default::default()
        }
    };
    
    for key in key_iter {
        let app_handle = app_handle.clone();
        let mut message = message.clone();
        let mut manager =  manager.clone();
        async move {
            message.group_v2 = Some(GroupContextV2 {
                master_key: Some(key.to_vec()),
                revision: Some(0),
                ..Default::default()
            });
            let send_timeout = {
                let state = APP_STATE.lock().unwrap();
                state.send_timeout
            };
            
            // We setup this loop to continuosly try to send message with given timeout
            loop {
                let timeout = tokio::time::sleep(Duration::from_secs(send_timeout as u64));
                let send = manager.send_message_to_group(
                    key.as_slice(),
                    ContentBody::DataMessage(message.clone()),
                    timestamp.into(),
                );
                pin_mut!(timeout);
                pin_mut!(send);
                match futures::future::select(send, timeout).await {
                    Either::Left((send_res, _)) => {
                        match send_res {
                            Ok(_) => {
                                _ = app_handle.upgrade_in_event_loop(|app| app.invoke_report("Повідомлення надіслано".to_shared_string()));
                                break;
                            },
                            Err(e) => { _ = app_handle.upgrade_in_event_loop(move |app| app.invoke_report(slint::format!("Помилка відправки, повтор: {e}"))); },
                        }
                    }
                    _ => {
                        _ = app_handle.upgrade_in_event_loop(|app| app.invoke_report("Повтор відправки повідомлення...".to_shared_string()));
                    }
                }
            }
        }.await;
    }

    info!("{:?}", APP_STATE.lock().unwrap());

    Ok(())
}
