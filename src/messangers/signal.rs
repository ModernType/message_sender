use std::{net::TcpStream, sync::Arc, time::{Duration, SystemTime}};

use futures::{SinkExt, StreamExt, channel::mpsc::{UnboundedReceiver, UnboundedSender}};
use log::{info, warn};
use presage::{libsignal_service::configuration::SignalServers, manager::Registered, proto::{DataMessage, EditMessage, GroupContextV2, data_message::Delete}, store::ContentsStore};
use presage_store_sqlite::{OnNewIdentity, SqliteConnectOptions, SqliteStore, SqliteStoreError};
use tokio::task::LocalSet;

use crate::{message::SendMode, messangers::Key, ui::{self, message_history::{GroupInfoSignal, SendMessageInfo, SendStatus}}};

type Manager = presage::Manager<SqliteStore, Registered>;

#[derive(Debug, Clone)]
pub enum SignalMessage {
    LinkBegin,
    Sync(Manager),
    // GetGroups,
    SendMessage(Manager, Arc<SendMessageInfo>, bool, bool),
    DeleteMessage(Manager, Arc<SendMessageInfo>),
    EditMessage(Manager, Arc<SendMessageInfo>, Vec<u64>, bool)
}

pub struct SignalWorker {
    signal_reciever: UnboundedReceiver<SignalMessage>,
    ui_message_sender: UnboundedSender<crate::ui::Message>,

}

impl SignalWorker {
    pub fn new(
        signal_reciever: UnboundedReceiver<SignalMessage>,
        ui_message_sender: UnboundedSender<crate::ui::Message>
    ) -> Self {
        Self { signal_reciever, ui_message_sender }
    }

    pub fn spawn_new(
        signal_reciever: UnboundedReceiver<SignalMessage>,
        ui_message_sender: UnboundedSender<crate::ui::Message>
    ) {
        std::thread::Builder::new().stack_size(8 * 1024 * 1024).spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("No runtime must be built");
            let worker = Self::new(signal_reciever, ui_message_sender);
            let local = LocalSet::new();
            local.spawn_local(worker.future());
            rt.block_on(local)
        }).unwrap();
    }

    async fn future(mut self) {
        while let Some(m) = self.signal_reciever.next().await {
            let ui_message_sender = self.ui_message_sender.clone();
            match m {
                SignalMessage::LinkBegin => {
                    match link(ui_message_sender.clone()).await {
                        Ok(mng) => send_ui_message(ui_message_sender.clone(), ui::Message::SetManager(mng)),
                        Err(_e) => send_ui_message(ui_message_sender.clone(), ui::main_screen::Message::SetSignalState(ui::main_screen::LinkState::Unlinked)),
                    }
                },
                SignalMessage::Sync(mng) => {
                    _ = tokio::task::spawn_local(sync(ui_message_sender.clone(), mng));
                },
                SignalMessage::SendMessage(manager, message, markdown, parallel) => {
                    send_message(ui_message_sender.clone(), manager, message, markdown, parallel).await;
                },
                SignalMessage::DeleteMessage(manager, message) => {
                    delete_message(ui_message_sender.clone(), manager, message).await;
                },
                SignalMessage::EditMessage(manager, message, timestamps, markdown) => {
                    _ = edit_message(ui_message_sender.clone(), manager, message, timestamps, markdown).await;
                }
            };
        }
    }
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

pub async fn get_groups(manager: Manager) -> anyhow::Result<Vec<(Key, String)>> {
    Ok(
        manager.store().groups().await?
        .flatten()
        .map(|(key, group)| {
            (Key::from(key), group.title)
        })
        .collect()
    )
}

async fn link(mut msg_send_channel: UnboundedSender<crate::ui::Message>) -> anyhow::Result<Manager> {
    send_ui_message(msg_send_channel.clone(), ui::main_screen::Message::SetSignalState(ui::main_screen::LinkState::Linking));
    loop {
        match TcpStream::connect("209.85.233.101:80") {
            Ok(_) => {
                break;
            },
            Err(_) => {
                send_ui_message(msg_send_channel.clone(), ui::Message::Notification("Немає підключення до інтернету".to_owned()));
                tokio::time::sleep(Duration::from_secs(3)).await;
            }
        }
    }

    let store = get_store().await?;
    info!("Registering from store");
    match Manager::load_registered(store).await {
        Ok(mng) => {
            // update_group_map(&mng).await;
            Ok(mng)
        }
        Err(e) => {
            warn!("{e}");
            let (tx, rx) = futures::channel::oneshot::channel();
            let store = get_store().await?;
            info!("Starting linking with device");
            let (mng_res, _) = futures::future::join(
                async move {
                    match presage::Manager::link_secondary_device(
                        store,
                        SignalServers::Production,
                        "message-sender".to_owned(),
                        tx,
                    )
                    .await
                    {
                        Ok(mng) => {
                            info!("Has manager");
                            Ok(mng)
                        }
                        Err(e) => {
                            warn!("Link failure: {e}");
                            Err(e)
                        },
                    }
                },
                async move {
                    match rx.await {
                        Ok(url) => {
                           msg_send_channel.send(crate::ui::main_screen::Message::SetRegisterUrl(url).into()).await.unwrap()
                        },
                        Err(_e) => {
                            
                        }
                    }
                },
            )
            .await;
            Ok(mng_res?)
        }
    }
}

async fn sync(mut msg_send_channel: UnboundedSender<crate::ui::Message>, mut manager: Manager) -> anyhow::Result<()> {
    let reciever = manager.receive_messages().await?;
    let mut reciever = Box::pin(reciever);
    while let Some(msg) = reciever.next().await {
        match msg {
            presage::model::messages::Received::Contacts => {
                info!("Got contacts");
            }
            presage::model::messages::Received::Content(_) => {
                info!("Got message");
            }
            presage::model::messages::Received::QueueEmpty => {
                msg_send_channel.send(crate::ui::Message::Synced).await.unwrap();
            }
        }
    }
    log::error!("Sync suspended");
    Ok(())
}

async fn send_message(
    msg_send_channel: UnboundedSender<crate::ui::Message>,
    manager: Manager,
    message: Arc<SendMessageInfo>,
    markdown: bool,
    parallel: bool,
) {
    message.set_status(SendStatus::Sending, std::sync::atomic::Ordering::Relaxed);
    send_ui_message(msg_send_channel.clone(), ui::main_screen::Message::UpdateMessageHistory);
    if !parallel {
        for group in message.groups_signal.iter() {
            loop {
                match send_message_inner(
                    manager.clone(),
                    group,
                    &message.content,
                    message.freq.as_deref(),
                    markdown,
                ).await {
                    Ok(_) => {
                        message.set_status(SendStatus::Sending, std::sync::atomic::Ordering::Relaxed);
                        send_ui_message(msg_send_channel.clone(), ui::main_screen::Message::UpdateMessageHistory);
                        break;
                    }
                    Err(e) => {
                        message.set_status(SendStatus::Failed, std::sync::atomic::Ordering::Relaxed);
                        send_ui_message(msg_send_channel.clone(), ui::Message::Notification(e.to_string()));
                    }
                }
            }
        }
    }
    else {
        futures::future::join_all(
            message.groups_signal.iter()
            .map(|group| async {
                loop {
                    match send_message_inner(
                        manager.clone(),
                        group,
                        &message.content,
                        message.freq.as_deref(),
                        markdown,
                    ).await {
                        Ok(_) => {
                            message.set_status(SendStatus::Sending, std::sync::atomic::Ordering::Relaxed);
                            send_ui_message(msg_send_channel.clone(), ui::main_screen::Message::UpdateMessageHistory);
                            break;
                        }
                        Err(e) => {
                            message.set_status(SendStatus::Failed, std::sync::atomic::Ordering::Relaxed);
                            send_ui_message(msg_send_channel.clone(), ui::Message::Notification(e.to_string()));
                        }
                    }
                }
            })
        ).await;
    }
    message.set_status(SendStatus::Sent, std::sync::atomic::Ordering::Relaxed);
    send_ui_message(msg_send_channel.clone(), ui::main_screen::Message::UpdateMessageHistory);
}

async fn send_message_inner(
    mut manager: Manager,
    group: &GroupInfoSignal,
    message: &str,
    freq: Option<&str>,
    markdown: bool,
) -> anyhow::Result<()> {
    let mut message = if markdown {
        let (message, ranges) = crate::message::parse_message_with_format(message)?;
        let message = if let SendMode::Frequency = group.send_mode && let Some(freq) = freq {
            format!("{}\n{}", freq, message)
        }
        else {
            message
        };

        DataMessage {
            body: Some(message),
            body_ranges: ranges,
            ..Default::default()
        }
    }
    else {
        DataMessage {
            body: Some(
                if let SendMode::Frequency = group.send_mode && let Some(freq) = freq {
                    format!("{}\n{}", freq, message)
                }
                else {
                    message.to_owned()
                }
            ),
            ..Default::default()
        }
    };
    let timestamp = std::time::SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
    message.group_v2 = Some(GroupContextV2 {
        master_key: Some(group.key.to_vec()),
        revision: Some(0),
        ..Default::default()
    });
    message.timestamp = Some(timestamp);

    manager.send_message_to_group(
        &group.key,
        message,
        timestamp
    ).await?;

    group.set_timestamp(timestamp, std::sync::atomic::Ordering::Relaxed);

    Ok(())
}

async fn delete_message(
    msg_send_channel: UnboundedSender<crate::ui::Message>,
    mut manager: Manager,
    message: Arc<SendMessageInfo>,
) {
    message.set_status(SendStatus::Sending, std::sync::atomic::Ordering::Relaxed);
    send_ui_message(msg_send_channel.clone(), ui::main_screen::Message::UpdateMessageHistory);
    
    for group in message.groups_signal.iter() {
        let target_timestamp = group.timestamp(std::sync::atomic::Ordering::Relaxed).unwrap();
        let cur_timestamp = std::time::SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;

        let delete_message = DataMessage {
            delete: Some(Delete {
                target_sent_timestamp: Some(target_timestamp),
            }),
            group_v2: Some(GroupContextV2 {
                master_key: Some(group.key.to_vec()),
                revision: Some(0),
                ..Default::default()
            }),
            timestamp: Some(cur_timestamp),
            ..Default::default()
        };

        loop {
            match manager.send_message_to_group(
                &group.key,
                delete_message.clone(),
                cur_timestamp
            ).await {
                Ok(_) => {
                    message.set_status(SendStatus::Sending, std::sync::atomic::Ordering::Relaxed);
                    group.set_timestamp(0, std::sync::atomic::Ordering::Relaxed);
                    send_ui_message(msg_send_channel.clone(), ui::main_screen::Message::UpdateMessageHistory);
                    break;
                }
                Err(e) => {
                    message.set_status(SendStatus::Failed, std::sync::atomic::Ordering::Relaxed);
                    send_ui_message(msg_send_channel.clone(), ui::Message::Notification(e.to_string()));
                }
            }
        }
    }

    message.set_status(SendStatus::Deleted, std::sync::atomic::Ordering::Relaxed);
    send_ui_message(msg_send_channel.clone(), ui::main_screen::Message::UpdateMessageHistory);
}

async fn edit_message(
    msg_send_channel: UnboundedSender<crate::ui::Message>,
    mut manager: Manager,
    message: Arc<SendMessageInfo>,
    timestamps: Vec<u64>,
    markdown: bool,
) -> anyhow::Result<()> {
    message.set_status(SendStatus::Sending, std::sync::atomic::Ordering::Relaxed);
    send_ui_message(msg_send_channel.clone(), ui::main_screen::Message::UpdateMessageHistory);
    for (group, timestamp) in message.groups_signal.iter().zip(timestamps) {
        let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
        let freq = message.freq.as_ref();

        let mut data_message = if markdown {
            let (message, ranges) = crate::message::parse_message_with_format(&message.content)?;
            DataMessage {
                body: Some(if let SendMode::Frequency = group.send_mode && let Some(freq) = freq {
                    format!("{}\n{}", freq, message)
                }
                else {
                    message
                }),
                body_ranges: ranges,
                ..Default::default()
            }
        }
        else {
            DataMessage {
                body: Some(if let SendMode::Frequency = group.send_mode && let Some(freq) = freq {
                    format!("{}\n{}", freq, message.content)
                }
                else {
                    message.content.clone()
                }),
                ..Default::default()
            }
        };
        data_message.group_v2 = Some(GroupContextV2 {
            master_key: Some(group.key.to_vec()),
            revision: Some(0),
            ..Default::default()
        });
        data_message.timestamp = Some(now);
        
        let edit_message = EditMessage {
            target_sent_timestamp: Some(timestamp),
            data_message: Some(data_message)
        };
        
        loop {
            match manager.send_message_to_group(
                &group.key,
                edit_message.clone(),
                now
            ).await {
                Ok(_) => {
                    message.set_status(SendStatus::Sending, std::sync::atomic::Ordering::Relaxed);
                    send_ui_message(msg_send_channel.clone(), ui::main_screen::Message::UpdateMessageHistory);
                    break;
                }
                Err(e) => {
                    message.set_status(SendStatus::Failed, std::sync::atomic::Ordering::Relaxed);
                    send_ui_message(msg_send_channel.clone(), ui::Message::Notification(e.to_string()));
                }
            }
        }

        group.set_timestamp(now, std::sync::atomic::Ordering::Relaxed);
    }

    message.set_status(SendStatus::Sent, std::sync::atomic::Ordering::Relaxed);
    send_ui_message(msg_send_channel.clone(), ui::main_screen::Message::UpdateMessageHistory);
    Ok(())
}

fn send_ui_message(
    mut msg_send_channel: UnboundedSender<crate::ui::Message>,
    message: impl Into<ui::Message> + 'static + Send
) {
    tokio::spawn(async move { msg_send_channel.send(message.into()).await.unwrap() });
}