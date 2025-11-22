//! This module has all functionality to serve a message awaiting server

use std::{net::SocketAddrV4, sync::OnceLock};

use crate::{APP_STATE, signal_actions::SignalAction};
use axum::{Router, response::IntoResponse, routing::post};
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
        let send = {
            let state = APP_STATE.lock().unwrap();
            state.autosend
        };
        if send {
            _ = self
                .action_send
                .send(SignalAction::SendMessage(message))
                .await;
        } else {
            _ = self
                .app_handle
                .upgrade_in_event_loop(|app| app.invoke_set_message_text(message.into()));
        }
        "Recieved"
    }
}

static HANDLES: OnceLock<HandleContainer> = OnceLock::new();

pub fn start_server_thread(
    action_send: UnboundedSender<SignalAction>,
    app_handle: Weak<crate::App>,
) -> std::thread::JoinHandle<()> {
    std::thread::spawn(|| {
        HANDLES.get_or_init(move || HandleContainer::new(action_send, app_handle));

        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();
        let listener_addr = {
            let state = APP_STATE.lock().unwrap();
            state.recieve_address
        };

        rt.block_on(async move {
            match start_server(listener_addr).await {
                Ok(_) => info!("Message server finished successfuly"),
                Err(e) => log::error!("Message server error: {e}"),
            }
        });
    })
}

async fn start_server(addr: SocketAddrV4) -> anyhow::Result<()> {
    let listener = TcpListener::bind(addr).await?;

    let router = Router::new().route("/", post(message_post));

    axum::serve(listener, router).await?;
    Ok(())
}

async fn message_post(body: String) -> impl IntoResponse {
    HANDLES.get().unwrap().clone().message_post(body).await
}
