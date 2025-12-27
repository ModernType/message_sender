use std::{net::SocketAddrV4, time::Duration};

use axum::{Router, routing::post, http::StatusCode};
use futures::{SinkExt, channel::mpsc::UnboundedSender};
use log::info;
use tokio::net::TcpListener;

use crate::ui;

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

    let router = Router::new().route("/", post(move |s| async move {
        msg_send_channel.send(ui::Message::AcceptMessage(s)).await.unwrap();
        (StatusCode::OK, "Recieved".to_owned())
    }));

    info!("Starting serving");
    axum::serve(listener, router).await?;
    Ok(())
}