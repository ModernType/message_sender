use std::{net::SocketAddrV4, time::Duration};

use axum::{Router, routing::post, http::StatusCode};
use futures::{SinkExt, channel::mpsc::UnboundedSender};
use log::info;
use tokio::net::TcpListener;

use crate::{message::OperatorMessage, ui};

pub async fn start_server(addr: SocketAddrV4, mut msg_send_channel: UnboundedSender<crate::ui::Message>) -> anyhow::Result<()> {
    info!("Binding on addr {}", &addr);
    let listener = loop {
        match TcpListener::bind(&addr).await {
            Ok(listener) => {
                msg_send_channel.send(ui::Message::Notification(format!("Прийом повідомлень запущено на {}", &addr))).await.unwrap();
                break listener
            },
            Err(_e) => {
                msg_send_channel.send(ui::Message::Notification(format!("Не можу запустити сервер на адресі {}. Може вона вже зайнята. Змініть її, будь ласка, у налаштуваннях", &addr))).await.unwrap();
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        }
    };

    let router = Router::new().route("/", post(move |s: String| async move {
        let message = match serde_json::from_str::<Vec<OperatorMessage>>(&s) {
            Ok(msgs) => msgs.into_iter().map(AcceptedMessage::from).collect::<Vec<_>>(),
            Err(e) => {
                log::error!("Message parse error: {e}");
                let mut message = AcceptedMessage::from(s);
                message.autosend_overwrite = true;
                vec![message]
            }
        };

        msg_send_channel.send(ui::Message::AcceptMessage(message)).await.unwrap();

        (StatusCode::OK, "Recieved".to_owned())
    }));

    info!("Starting serving");
    axum::serve(listener, router).await?;
    Ok(())
}

#[derive(Debug)]
pub struct AcceptedMessage {
    pub text: String,
    pub freq: Option<String>,
    pub network: Option<String>,
    pub autosend_overwrite: bool,
}

impl From<OperatorMessage<'_>> for AcceptedMessage {
    fn from(value: OperatorMessage) -> Self {
        let freq = Some(value.0.frequency.to_string());
        let network = Some(value.0.title.to_string());

        Self {
            text: value.to_string(),
            freq,
            network,
            autosend_overwrite: false
        }
    }
}

impl From<String> for AcceptedMessage {
    fn from(value: String) -> Self {
        Self {
            text: value,
            freq: None,
            network: None,
            autosend_overwrite: false
        }
    }
}