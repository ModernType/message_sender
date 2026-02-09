use std::{collections::VecDeque, net::TcpStream, sync::Arc, time::{Duration, SystemTime}};

use futures::{FutureExt, SinkExt, StreamExt, channel::mpsc::{UnboundedReceiver, UnboundedSender}};
use log::{info, warn};
use presage::{libsignal_service::configuration::SignalServers, manager::Registered, proto::{DataMessage, EditMessage, GroupContextV2, data_message::Delete}, store::ContentsStore};
use presage_store_sqlite::{OnNewIdentity, SqliteConnectOptions, SqliteStore, SqliteStoreError};
use tokio::task::{AbortHandle, LocalSet};

use crate::{message::SendMode, messangers::Key, ui::{self, message_history::{GroupInfoSignal, SendMessageInfo, SendStatus}, side_menu::LinkState}};

type Manager = presage::Manager<SqliteStore, Registered>;

#[derive(Debug)]
pub enum SignalMessage {
    LinkBegin,
    Linked(Result<Manager, anyhow::Error>),
    CancelLink,
    Disconnect,
    GetGroups,
    SendMessage(Arc<SendMessageInfo>, bool, bool),
    DeleteMessage(Arc<SendMessageInfo>),
    EditMessage(Arc<SendMessageInfo>, Vec<u64>, bool),
    Cancel,
    Finished,
}

pub struct SignalWorker {
    signal_reciever: UnboundedReceiver<SignalMessage>,
    ui_message_sender: UnboundedSender<crate::ui::Message>,
    message_queue: VecDeque<SignalMessage>,
    abort_handle: Option<AbortHandle>,
    signal_sender: UnboundedSender<SignalMessage>,
    link_abort: Option<AbortHandle>,
    manager: Option<Manager>,
    has_connected: bool,
}

impl SignalWorker {
    pub fn new(
        signal_reciever: UnboundedReceiver<SignalMessage>,
        ui_message_sender: UnboundedSender<crate::ui::Message>,
        signal_sender: UnboundedSender<SignalMessage>
    ) -> Self {
        Self {
            signal_reciever,
            ui_message_sender,
            signal_sender,
            message_queue: VecDeque::new(),
            abort_handle: None,
            link_abort: None,
            manager: None,
            has_connected: false,
        }
    }

    pub fn spawn_new(
        signal_reciever: UnboundedReceiver<SignalMessage>,
        ui_message_sender: UnboundedSender<crate::ui::Message>,
        signal_sender: UnboundedSender<SignalMessage>
    ) {
        std::thread::Builder::new().stack_size(8 * 1024 * 1024).spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("No runtime must be built");
            let worker = Self::new(signal_reciever, ui_message_sender, signal_sender);
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
                    let mut finish_send = self.signal_sender.clone();
                    let handle = tokio::task::spawn_local(link(ui_message_sender.clone()).then(move |res| async move {
                        _ = finish_send.send(SignalMessage::Linked(res)).await;
                    }));
                    self.link_abort = Some(handle.abort_handle());
                },
                SignalMessage::Linked(mng_res) => {
                    match mng_res {
                        Ok(mng) => {
                            tokio::task::spawn_local(sync(self.ui_message_sender.clone(), mng.clone()));
                            self.manager = Some(mng);
                            self.has_connected = true;
                            self.execute_next_maybe();
                            send_ui_message(ui_message_sender.clone(), ui::side_menu::Message::SetSignalState(LinkState::Linked))
                        },
                        Err(_e) => send_ui_message(ui_message_sender.clone(), if self.has_connected {
                            ui::side_menu::Message::SetSignalState(LinkState::Disconnected)
                        }
                        else {
                            ui::side_menu::Message::SetSignalState(LinkState::Unlinked)
                        }),
                    }
                },
                SignalMessage::Disconnect => {
                    self.manager = None;
                    if let Some(handle) = self.abort_handle.take() {
                        handle.abort();
                    }
                },
                SignalMessage::CancelLink => {
                    if let Some(handle) = self.abort_handle.take() {
                        handle.abort();
                    }
                    send_ui_message(ui_message_sender.clone(), ui::side_menu::Message::SetSignalState(LinkState::Unlinked))
                },
                SignalMessage::Cancel => {
                    if let Some(handle) = self.abort_handle.take() {
                        handle.abort();
                    }
                    if self.manager.is_some() {
                        self.execute_next_maybe();
                    }
                },
                SignalMessage::Finished => {
                    self.abort_handle = None;
                    _ = self.message_queue.pop_front();
                    self.execute_next_maybe();
                },
                SignalMessage::GetGroups => {
                    if let Some(manager) = &self.manager {
                        let manager = manager.clone();
                        tokio::task::spawn_local(async move {
                            let groups = get_groups(manager).await;
                            _ = send_ui_message(ui_message_sender, groups.map(ui::main_screen::Message::SetGroups));
                        });
                    }
                }
                message => {
                    self.message_queue.push_back(message);
                    self.execute_next_maybe();
                },
            };


        }
    }

    fn execute_next_maybe(&mut self) {
        if self.abort_handle.is_none() && self.manager.is_some() && let Some(message) = self.message_queue.front() {
            let handle = self.execute_task(message);
            self.abort_handle = Some(handle);
        }
    }

    fn execute_task(&self, message: &SignalMessage) -> AbortHandle {
        /// This macro wraps any future, attaches finish sending message, starts its execution and returns abort handle
        macro_rules! message_task {
            ($e:expr, $finish_send:ident) => {
                {
                    let handle = tokio::task::spawn_local(
                        $e.then(move |_| async move { $finish_send.send(SignalMessage::Finished).await.unwrap(); })
                    );
                    handle.abort_handle()
                }
            };
        }

        let ui_message_sender = self.ui_message_sender.clone();
        let mut finish_send = self.signal_sender.clone();
        let abort_handle = match message {
            SignalMessage::SendMessage(message, markdown, parallel) => message_task!(send_message(ui_message_sender, self.manager.as_ref().unwrap().clone(), message.clone(), *markdown, *parallel), finish_send),
            SignalMessage::DeleteMessage(message) => message_task!(delete_message(ui_message_sender, self.manager.as_ref().unwrap().clone(), message.clone()), finish_send),
            SignalMessage::EditMessage(message, timestamps, markdown) => message_task!(edit_message(ui_message_sender, self.manager.as_ref().unwrap().clone(), message.clone(), timestamps.clone(), *markdown), finish_send),
            _m => panic!("Other messages should not be here!")
        };
    
        abort_handle
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
    send_ui_message(msg_send_channel.clone(), ui::side_menu::Message::SetSignalState(LinkState::Linking));
    loop {
        // Ping to google.com at start
        match TcpStream::connect("209.85.233.101:80") {
            Ok(_) => {
                break;
            },
            Err(_) => {
                send_ui_message(msg_send_channel.clone(), ui::side_menu::Message::SetSignalState(LinkState::Disconnected));
                send_ui_message(msg_send_channel.clone(), ui::Message::Notification("Немає підключення до інтернету".to_owned()));
                tokio::time::sleep(Duration::from_secs(5)).await;
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
                           msg_send_channel.send(crate::ui::main_screen::Message::SetRegisterUrl(Some(url)).into()).await.unwrap()
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
                _ = msg_send_channel.send(crate::ui::Message::Synced).await;
            }
        }
    }
    log::error!("Sync suspended");
    _ = msg_send_channel.send(ui::Message::SignalDisconnected).await;

    #[allow(unreachable_code)]
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
                    Err(_e) => {
                        message.set_status(SendStatus::Failed, std::sync::atomic::Ordering::Relaxed);
                        // send_ui_message(msg_send_channel.clone(), ui::Message::Notification(e.to_string()));
                        tokio::time::sleep(Duration::from_millis(2000)).await;
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
        let (message, ranges) = crate::message::parse_message_with_format(message).unwrap_or_else(|_| (message.to_owned(), Vec::new()));
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
                Err(_e) => {
                    message.set_status(SendStatus::Failed, std::sync::atomic::Ordering::Relaxed);
                    // send_ui_message(msg_send_channel.clone(), ui::Message::Notification(e.to_string()));
                    tokio::time::sleep(Duration::from_millis(2000)).await;
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
) {
    message.set_status(SendStatus::Sending, std::sync::atomic::Ordering::Relaxed);
    send_ui_message(msg_send_channel.clone(), ui::main_screen::Message::UpdateMessageHistory);
    for (group, timestamp) in message.groups_signal.iter().zip(timestamps) {
        let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
        let freq = message.freq.as_ref();

        let mut data_message = if markdown {
            let (message, ranges) = crate::message::parse_message_with_format(&message.content).unwrap_or_else(|_| (message.content.clone(), Vec::new()));
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
                Err(_e) => {
                    message.set_status(SendStatus::Failed, std::sync::atomic::Ordering::Relaxed);
                    // send_ui_message(msg_send_channel.clone(), ui::Message::Notification(e.to_string()));
                    tokio::time::sleep(Duration::from_millis(2000)).await;
                }
            }
        }

        group.set_timestamp(now, std::sync::atomic::Ordering::Relaxed);
    }

    message.set_status(SendStatus::Sent, std::sync::atomic::Ordering::Relaxed);
    send_ui_message(msg_send_channel.clone(), ui::main_screen::Message::UpdateMessageHistory);
}

fn send_ui_message(
    mut msg_send_channel: UnboundedSender<crate::ui::Message>,
    message: impl Into<ui::Message> + 'static + Send
) {
    tokio::spawn(async move { msg_send_channel.send(message.into()).await.unwrap() });
}