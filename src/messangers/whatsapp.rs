use std::sync::{Arc, OnceLock};

use futures::{SinkExt, channel::mpsc::UnboundedSender};
use whatsapp_rust::{Client, bot::Bot, types::events::Event};
use whatsapp_rust_sqlite_storage::SqliteStore;
use whatsapp_rust_tokio_transport::TokioWebSocketTransportFactory;
use whatsapp_rust_ureq_http_client::UreqHttpClient;
use waproto::whatsapp as wa;

use crate::{message::{SendMode, parse_message_with_whatsapp_format}, messangers::Key, ui::{self, main_screen::LinkState, message_history::{SendMessageInfo, SendStatus}}};

pub static UI_MESSAGE_SENDER: OnceLock<UnboundedSender<ui::Message>> = OnceLock::new();

async fn send_ui_message(messsage: impl Into<ui::Message>) {
    UI_MESSAGE_SENDER.get().unwrap().send(messsage.into()).await.unwrap()
}

pub async fn start_whatsapp_task() {
    match start_whatsapp_task_inner().await {
        Ok(_bot_handle) => {
            // TODO: Maybe use `bot_handle` for termination
        },
        Err(e) => {
            UI_MESSAGE_SENDER.get().unwrap().send(ui::main_screen::Message::SetWhatsappState(LinkState::Unlinked).into()).await.unwrap();
            UI_MESSAGE_SENDER.get().unwrap().send(ui::Message::Notification(format!("Error linking to Whatsapp: {e}"))).await.unwrap();
        },
    }
}

async fn start_whatsapp_task_inner() -> anyhow::Result<tokio::task::JoinHandle<()>> {
    let store = Arc::new(SqliteStore::new("whatsapp_data.db").await?);
    
    let transport = TokioWebSocketTransportFactory::new();
    let client = UreqHttpClient::new();


    let mut bot = Bot::builder()
    .with_backend(store)
    .with_transport_factory(transport)
    .with_http_client(client)
    .on_event(|event, client| async {
        match event {
            // TODO: Maybe use timeout to communicate to UI
            #[allow(unused_variables)]
            Event::PairingQrCode { code, timeout } => {
                UI_MESSAGE_SENDER.get().unwrap().send(ui::main_screen::Message::SetWhatsappUrl(code).into()).await.unwrap();
            },
            Event::Connected(_) => {
                println!("Connected to whatsapp");
                UI_MESSAGE_SENDER.get().unwrap().send(ui::Message::SetWhatsappClient(Some(client))).await.unwrap();
            },
            _other_event => {
                
            }
        }
    })
    .build()
    .await?;

    bot.run().await
}

pub async fn get_groups(client: Arc<Client>) -> anyhow::Result<Vec<(Key, String)>> {
    Ok(
        client.groups().get_participating().await?.values()
        .map(|meta| (meta.id.clone().into(), meta.subject.clone()))
        .collect()
    )
}

pub async fn send_message(client: Arc<Client>, message: Arc<SendMessageInfo>, markdown: bool) {
    message.set_status(SendStatus::Sending, std::sync::atomic::Ordering::Relaxed);

    let content = if markdown {
        parse_message_with_whatsapp_format(&message.content).unwrap_or(message.content.clone())
    }
    else {
        message.content.clone()
    };
    
    for group in message.groups_whatsapp.iter() {
        let wa_message = wa::Message {
            conversation: Some(
                if let SendMode::Frequency = group.send_mode && let Some(ref freq) = message.freq {
                    format!("{}\n{}", freq, &content)
                }
                else {
                    content.clone()
                }
            ),
            ..Default::default()
        };

        let message_id = loop {
            match client.send_message(
                group.key.clone(),
                wa_message.clone()
            ).await {
                Ok(msg) => {
                    message.set_status(SendStatus::Sending, std::sync::atomic::Ordering::Relaxed);
                    send_ui_message(ui::main_screen::Message::UpdateMessageHistory).await;
                    break msg;
                }
                Err(e) => {
                    message.set_status(SendStatus::Failed, std::sync::atomic::Ordering::Relaxed);
                    send_ui_message(ui::Message::Notification(e.to_string())).await;
                }
            }
        };

        group.set_id(message_id);
    }

    message.set_status(SendStatus::Sent, std::sync::atomic::Ordering::Relaxed);
}

pub async fn edit_message(client: Arc<Client>, message: Arc<SendMessageInfo>, message_ids: Vec<String>, markdown: bool) {
    message.set_status(SendStatus::Sending, std::sync::atomic::Ordering::Relaxed);
    let content = if markdown {
        parse_message_with_whatsapp_format(&message.content).unwrap_or(message.content.clone())
    }
    else {
        message.content.clone()
    };
    
    for (group, message_id) in message.groups_whatsapp.iter().zip(message_ids.into_iter()) {
        let wa_message = wa::Message {
            conversation: Some(if let SendMode::Frequency = group.send_mode && let Some(ref freq) = message.freq {
                    format!("{}\n{}", freq, &content)
                }
                else {
                    content.clone()
                }),
            ..Default::default()
        };
        let message_id = loop {
            match client.edit_message(
                group.key.clone(),
                message_id.clone(),
                wa_message.clone()
            ).await {
                Ok(msg) => {
                    message.set_status(SendStatus::Sending, std::sync::atomic::Ordering::Relaxed);
                    send_ui_message(ui::main_screen::Message::UpdateMessageHistory).await;
                    break msg;
                }
                Err(e) => {
                    message.set_status(SendStatus::Failed, std::sync::atomic::Ordering::Relaxed);
                    send_ui_message(ui::Message::Notification(e.to_string())).await;
                }
            }
        };

        group.set_id(message_id);
    }

    message.set_status(SendStatus::Sent, std::sync::atomic::Ordering::Relaxed);
}

// TODO: Figure out how to delete messages in whatsapp
pub async fn delete_message(_client: Arc<Client>, message: Arc<SendMessageInfo>) {
    message.set_status(SendStatus::Sending, std::sync::atomic::Ordering::Relaxed);
    
    send_ui_message(ui::Message::Notification("Повідомлення не буде видалене у Whatsapp".to_owned())).await;

    for group in message.groups_whatsapp.iter() {
        group.delete(std::sync::atomic::Ordering::Relaxed);
    }

    message.set_status(SendStatus::Deleted, std::sync::atomic::Ordering::Relaxed);
    send_ui_message(ui::main_screen::Message::UpdateMessageHistory).await;
}
