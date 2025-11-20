use std::{
    rc::Rc, time::{Duration, SystemTime}
};

use futures::{StreamExt, future::Either, pin_mut};
use image::Rgb;
use log::{info, warn};
use presage::{
    Manager, libsignal_service::{configuration::SignalServers, content::ContentBody}, proto::{DataMessage, GroupContextV2}, store::ContentsStore
};
use presage_store_sqlite::{OnNewIdentity, SqliteConnectOptions, SqliteStore, SqliteStoreError};
use slint::{ModelRc, SharedPixelBuffer, SharedString, VecModel, Weak};

use crate::{APP_STATE, App, Group};

pub async fn get_store() -> Result<SqliteStore, SqliteStoreError> {
    let mut store = SqliteStore::open_with_options(
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

async fn update_group_map() {
    let mut state = APP_STATE.lock().unwrap();
    let manager = state.manager().unwrap().clone();
    let group_map = &mut state.cached_groups;
    for (key, group) in manager.store().groups().await.unwrap().flatten() {
        group_map.insert(group.title, key);
    }
}

pub async fn link(app_handle: Weak<App>) -> anyhow::Result<()> {
    let store = get_store().await?;
    info!("Registering from store");
    match Manager::load_registered(store).await {
        Ok(mng) => {
            APP_STATE.lock().unwrap().set_manager(mng);
            update_group_map().await;
            app_handle
                .clone()
                .upgrade_in_event_loop(move |app| {
                    app.invoke_linked();
                })
                .unwrap()
        }
        Err(e) => {
            warn!("{e}");
            let (tx, rx) = futures::channel::oneshot::channel();
            let store = get_store().await?;
            info!("Starting linking with device");
            let app_handle2 = app_handle.clone();
            let _res = futures::future::join(
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
                            APP_STATE.lock().unwrap().set_manager(mng);
                            update_group_map().await;
                            app_handle
                                .clone()
                                .upgrade_in_event_loop(|app| {
                                    app.invoke_linked();
                                })
                                .unwrap()
                        }
                        Err(e) => warn!("Link failure: {e}"),
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
        }
    };
    info!("Linking finished");
    Ok(())
}

pub async fn sync(app_handle: Weak<App>) -> anyhow::Result<()> {
    let state = APP_STATE.lock().unwrap();
    let mut manager = state.manager().unwrap().clone();
    drop(state);
    let reciever = manager.receive_messages().await?;
    pin_mut!(reciever);
    let mut msg_count = 0;
    while let Some(msg) = reciever.next().await {
        match msg {
            presage::model::messages::Received::Contacts => {
                info!("Got contacts");
            }
            presage::model::messages::Received::Content(_) => {
                msg_count += 1;
                info!("Got message {msg_count}");
            }
            presage::model::messages::Received::QueueEmpty => {
                info!("New messages ended");
                break;
            }
        }
    }
    let mut state = APP_STATE.lock().unwrap();
    let group_map = &mut state.cached_groups;
    for (key, group) in manager.store().groups().await?.flatten() {
        group_map.insert(group.title, key);
    }
    app_handle
        .clone()
        .upgrade_in_event_loop(|app| {
            app.invoke_synced();
        })
        .unwrap();
    Ok(())
}

pub async fn get_groups(app_handle: Weak<App>) -> anyhow::Result<()> {
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

pub async fn send_message(message: String) -> anyhow::Result<()> {
    let state = APP_STATE.lock().unwrap();
    let key_iter = state
        .group_active
        .iter()
        .filter_map(|(title, send)| send.then(|| state.cached_groups.get(&title.to_string())))
        .flatten()
        .cloned()
        .collect::<Vec<_>>();
    // We should drop mutex lock before any await point
    drop(state);

    let timestamp = std::time::SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
    // let local = LocalSet::new();
    for key in key_iter {
        let message = message.clone();
        tokio::task::spawn_local(async move {
            let message = DataMessage {
                body: Some(message),
                timestamp: Some(timestamp),
                group_v2: Some(GroupContextV2 {
                    master_key: Some(key.to_vec()),
                    revision: Some(0),
                    ..Default::default()
                }),
                // body_ranges: vec![
                //     BodyRange {
                //         start: Some(0),
                //         length: Some(5),
                //         associated_value: Some(presage::proto::body_range::AssociatedValue::Style(Style::Bold as i32))
                //     }
                // ],
                ..Default::default()
            };
            let mut manager = {
                let state = APP_STATE.lock().unwrap();
                match state.manager() {
                    Some(mng) => mng.clone(),
                    None => return,
                }
            };

            // We setup this loop to continuosly try to send message with given timeout
            loop {
                let timeout = tokio::time::sleep(Duration::from_millis(5000));
                let send = manager
                .send_message_to_group(
                    key.as_slice(),
                    ContentBody::DataMessage(message.clone()),
                    timestamp.into(),
                );
                pin_mut!(timeout);
                pin_mut!(send);
                match futures::future::select(send, timeout).await {
                    Either::Left((send_res, _)) => {
                        match send_res {
                            Ok(_) => info!("Message sent!"),
                            Err(e) => warn!("Message send error: {e}"),
                        }
                        break;
                    },
                    _ => {
                        info!("Retrying to send the message...")
                    }
                }
            }
        });
    }

    // local.await;
    Ok(())
}
