//! This module has all functionality to serve a message awaiting server

use std::{net::SocketAddrV4, sync::OnceLock, time::Duration};

use crate::{app_state::APP_STATE, SendMode, message::OperatorMessage as Message, signal_actions::SignalAction};
use axum::{Router, http::StatusCode, response::IntoResponse, routing::post};
use futures::{SinkExt, channel::mpsc::UnboundedSender};
use log::info;
use slint::Weak;
use tokio::net::TcpListener;

#[derive(Clone)]
struct HandleContainer {
    action_send: UnboundedSender<SignalAction>,
    app_handle: Weak<crate::App>,
}

impl HandleContainer {
    pub fn new(action_send: UnboundedSender<SignalAction>, app_handle: Weak<crate::App>) -> Self {
        Self {
            action_send,
            app_handle,
        }
    }

    // pub fn sender(&self) -> UnboundedSender<SignalAction> {
    //     self.action_send.clone()
    // }

    // pub fn app_handle(&self) -> Weak<crate::App> {
    //     self.app_handle.clone()
    // }

    pub async fn message_post(mut self, message: String) -> impl IntoResponse {
        let (autosend, send_mode) = {
            let state = APP_STATE.lock().unwrap();
            (state.autosend, state.send_mode)
        };
        let messages = match send_mode {
            SendMode::Standard => {
                match serde_json::from_str::<Vec<Message>>(&message) {
                    Ok(messages) => messages.iter().map(Message::to_string).collect(),
                    Err(e) => {
                        log::error!("Error to parse messages: {}\nOriginal body: {}", &e, message);
                        return (StatusCode::BAD_REQUEST, e.to_string())
                    }
                }
            },
            SendMode::Frequency => {
                match serde_json::from_str::<Vec<Message>>(&message) {
                    Ok(messages) => messages.iter().map(Message::with_frequency).collect(),
                    Err(e) => {
                        log::error!("Error to parse messages: {}\nOriginal body: {}", &e, message);
                        return (StatusCode::BAD_REQUEST, e.to_string())
                    }
                }
            }
            SendMode::Debug => {
                vec![message]
            }
        };
        if autosend {
            _ = self.app_handle.clone().upgrade_in_event_loop(|app| {
                app.invoke_report("Отримано повідомлення. Відправка...".into());
            });
            for message in messages {
                _ = self
                    .action_send
                    .send(SignalAction::SendMessage(message))
                    .await;
            }
        }
        else {
            let message = messages.join("\n");
            _ = self
            .app_handle
            .upgrade_in_event_loop(|app| app.invoke_set_message_text(message.into()));
        }
        (StatusCode::OK, "Recieved".to_owned())
    }
}

static HANDLES: OnceLock<HandleContainer> = OnceLock::new();

pub fn start_server_thread(
    action_send: UnboundedSender<SignalAction>,
    app_handle: Weak<crate::App>,
) -> std::thread::JoinHandle<()> {
    std::thread::spawn(|| {
        let app_handle2 = app_handle.clone();
        HANDLES.get_or_init(move || HandleContainer::new(action_send, app_handle2));

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let listener_addr = {
            let state = APP_STATE.lock().unwrap();
            state.recieve_address
        };

        rt.block_on(async move {
            match start_server(listener_addr, app_handle).await {
                Ok(_) => info!("Message server finished successfuly"),
                Err(e) => panic!("Message server error: {e}"),
            }
        });
    })
}

async fn start_server(addr: SocketAddrV4, app_handle: Weak<crate::App>) -> anyhow::Result<()> {
    info!("Binding on addr {}", &addr);
    let listener = loop {
        match TcpListener::bind(addr).await {
            Ok(listener) => {
                _ = app_handle.upgrade_in_event_loop(move |app| {
                    app.invoke_report("Сервер для отримання повідомлень запущено".into());
                });
                break listener
            },
            Err(_e) => {
                _ = app_handle.upgrade_in_event_loop(move |app| {
                    app.invoke_report(slint::format!("Не можу запустити сервер на адресі {}. Може вона вже зайнята. Змініть її у налаштуваннях", &addr));
                });
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        }
    };

    let router = Router::new().route("/", post(message_post));

    info!("Starting serving");
    axum::serve(listener, router).await?;
    Ok(())
}

async fn message_post(body: String) -> impl IntoResponse {
    HANDLES.get().unwrap().clone().message_post(body).await
}
