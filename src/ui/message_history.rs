use std::{sync::{Arc, Mutex, atomic::{AtomicBool, AtomicU8, AtomicU64, Ordering}}};

use iced::{Alignment, Border, Element, Length, Theme, widget::{Column, Row, button, container, progress_bar, svg, text}};
use wacore_binary::jid::Jid;

use crate::{icon, message::SendMode, messangers::Key, ui::icons};

#[derive(Debug)]
pub struct SendMessageInfo {
    pub content: String,
    pub freq: Option<String>,
    pub status: AtomicU8,
    pub groups_signal: Vec<GroupInfoSignal>,
    pub groups_whatsapp: Vec<GroupInfoWhatsapp>,
}

#[repr(u8)]
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum SendStatus {
    Pending,
    Sending,
    Sent,
    Failed,
    Deleted,
}

impl From<u8> for SendStatus {
    fn from(value: u8) -> Self {
        match value {
            0 => SendStatus::Pending,
            1 => SendStatus::Sending,
            2 => SendStatus::Sent,
            3 => SendStatus::Failed,
            4 => SendStatus::Deleted,
            _ => unreachable!("You should not use values outside of `SendStatus` enum"),
        }
    }
}

#[derive(Debug)]
pub struct GroupInfoSignal {
    pub key: [u8; 32],
    pub(super) timestamp: AtomicU64,
    pub send_mode: SendMode,
}

impl Clone for GroupInfoSignal {
    fn clone(&self) -> Self {
        Self {
            key: self.key,
            timestamp: AtomicU64::new(self.timestamp.load(Ordering::Relaxed)),
            send_mode: self.send_mode,
        }
    }
}

impl GroupInfoSignal {
    pub fn new(key: [u8; 32], send_mode: SendMode) -> Self {
        Self { key, timestamp: AtomicU64::new(0), send_mode }
    }

    pub fn set_timestamp(&self, timestamp: u64, ordering: std::sync::atomic::Ordering) {
        self.timestamp.store(timestamp, ordering);
    }

    pub fn timestamp(&self, ordering: std::sync::atomic::Ordering) -> Option<u64> {
        let timestamp = self.timestamp.load(ordering);
        if timestamp == 0 {
            None
        }
        else {
            Some(timestamp)
        }
    }

    pub fn sent(&self, ordering: std::sync::atomic::Ordering) -> bool {
        self.timestamp.load(ordering) != 0
    }
}

#[derive(Debug)]
pub struct GroupInfoWhatsapp {
    pub key: Jid,
    sent: AtomicBool,
    pub(super) sent_id: Mutex<String>,
    pub send_mode: SendMode,
}

impl Clone for GroupInfoWhatsapp {
    fn clone(&self) -> Self {
        Self {
            key: self.key.clone(),
            sent: AtomicBool::new(self.sent.load(Ordering::Relaxed)),
            sent_id: Mutex::new(self.sent_id.lock().unwrap().clone()),
            send_mode: self.send_mode
        }
    }
}

impl GroupInfoWhatsapp {
    pub fn new(key: Jid, send_mode: SendMode) -> Self {
        Self {
            key,
            sent: AtomicBool::new(false),
            sent_id: Mutex::new(String::new()),
            send_mode
        }
    }

    pub fn sent(&self, ordering: Ordering) -> bool {
        self.sent.load(ordering)
    }

    pub fn delete(&self, ordering: Ordering) {
        self.sent.store(false, ordering);
    }

    pub fn set_id(&self, id: String) {
        let mut lock = self.sent_id.lock().unwrap();
        *lock = id;
        self.sent.store(true, Ordering::Relaxed);
    }

    pub fn message_id(&self) -> Option<String> {
        let lock = self.sent_id.lock().unwrap();
        (!lock.is_empty()).then(|| lock.clone())
    }
}

impl SendMessageInfo {
    pub fn new(content: String, freq: Option<String>) -> Self {
        Self {
            content,
            freq,
            status: AtomicU8::new(SendStatus::Pending as u8),
            groups_signal: Vec::new(),
            groups_whatsapp: Vec::new(),
        }
    }

    pub fn push(&mut self, group_key: Key, send_mode: SendMode) {
        match group_key {
            Key::Signal(key) => self.groups_signal.push(GroupInfoSignal::new(key, send_mode)),
            Key::Whatsapp(key) => self.groups_whatsapp.push(GroupInfoWhatsapp::new(key, send_mode)),
        }
    }

    pub fn sent_count(&self) -> usize {
        self.groups_signal.iter()
        .map(|g| g.sent(Ordering::Relaxed))
        .chain(
            self.groups_whatsapp.iter()
            .map(|g| g.sent(Ordering::Relaxed))
        )
        .filter(|v| *v)
        .count()
    }

    // pub fn sent(&self) -> bool {
    //     self.groups.iter().all(|g| g.sent(Ordering::Relaxed))
    // }

    // pub fn message(&self) -> &str {
    //     &self.content
    // }

    pub fn set_status(&self, status: SendStatus, ordering: Ordering) {
        let sent_count = self.sent_count();
        if SendStatus::Deleted == status && sent_count != 0 
        || SendStatus::Sent == status && sent_count != self.len()
        {
            return;
        }
        self.status.store(status as u8, ordering);
    }

    pub fn len(&self) -> usize {
        self.groups_signal.len() + self.groups_whatsapp.len()
    }

    pub fn view<'a>(self: &'a Arc<Self>, idx: usize) -> Element<'a, super::main_screen::Message, Theme> {
        let status_color = match self.status.load(Ordering::Relaxed) {
            0 => Some(iced::Color::from_rgb(0.3, 0.3, 0.3)),
            1 => None,
            2 => Some(iced::Color::from_rgb(0.0, 0.6, 0.0)),
            3 | 4 => Some(iced::Color::from_rgb(0.6, 0.0, 0.0)),
            _ => unreachable!("You should not use values outside of `SendStatus` enum"),
        };
        let sent_count = self.sent_count();
        let status = SendStatus::from(self.status.load(Ordering::Relaxed));
        container(
            Row::new()
            .spacing(5)
            .padding(5)
            .align_y(Alignment::Center)
            .push(
                Column::new()
                .spacing(5)
                .padding(5)
                .push(
                    text(
                        self.content.lines()
                        .filter(|l| l.len() > 1)
                        .take(3)
                        .collect::<Vec<_>>()
                        .join("\n")
                    )
                    .center()
                    .wrapping(text::Wrapping::None)
                    .size(14)
                    .width(Length::Fill)
                )
                .push(
                    match status {
                        SendStatus::Pending => Element::from(
                            text("Pending...")
                            .color(status_color.unwrap())
                            .font(iced::Font { style: iced::font::Style::Italic, ..Default::default() })
                            .center()
                            .width(Length::Fill)
                        ),
                        SendStatus::Deleted if sent_count == 0 => Element::from(
                            text("Видалено")
                            .style(|theme: &Theme| text::Style { color: Some(theme.extended_palette().danger.base.color) })
                            .center()
                            .width(Length::Fill)
                        ),
                        SendStatus::Sent if sent_count == self.len() => Element::from(
                            text("Відправлено")
                            .style(|theme: &Theme| text::Style { color: Some(theme.extended_palette().success.strong.color) })
                            .center()
                            .width(Length::Fill)
                        ),
                        _ => Element::from(
                            Column::new()
                            .push(
                                text(format!("{}/{}", sent_count, self.groups_signal.len() + self.groups_whatsapp.len()))
                                .color_maybe(status_color)
                                .center()
                                .width(Length::Fill)
                            )
                            .push(
                                progress_bar(0.0 ..= (self.groups_signal.len() + self.groups_whatsapp.len()) as f32, sent_count as f32)
                                .length(Length::Fill)
                                .girth(5)
                                .style(move |theme: &Theme| {
                                    let palette = theme.extended_palette();
                                    progress_bar::Style {
                                        bar: status_color.map(iced::Background::Color).unwrap_or_else(|| palette.background.base.text.into()),
                                        background: palette.background.base.color.into(),
                                        border: Border::default().rounded(2.5),
                                    }
                                })
                            )
                        )
                    }
                )
            )
            .push(
                Column::new()
                .spacing(3)
                .push(
                    button(
                        icon!(delete)
                    )
                    .style(button::danger)
                    .on_press_maybe(match status {
                        SendStatus::Sent => Some(super::main_screen::Message::DeleteMessage(idx)),
                        SendStatus::Sending | SendStatus::Failed  => Some(super::main_screen::Message::Cancel(idx)),
                        _ => None,
                    })
                )
                .push(
                    button(
                        match status {
                            SendStatus::Pending | SendStatus::Sending | SendStatus::Failed => icon!(refresh),
                            _ => icon!(edit),
                        }
                    )
                    .style(button::secondary)
                    .on_press_maybe(match status {
                        SendStatus::Sent => Some(super::main_screen::Message::EditMessage(idx)),
                        SendStatus::Sending | SendStatus::Failed => Some(super::main_screen::Message::RefreshMessage(idx)),
                        _ => None,
                    })
                )
            )
        )
        .padding(3)
        .width(Length::Fill)
        .style(move |theme: &Theme| {
            let palette = theme.extended_palette();
            let background = match status {
                SendStatus::Deleted | SendStatus::Failed => palette.danger.base.color.scale_alpha(if palette.is_dark { 0.10 } else { 0.4 }),
                SendStatus::Sending => palette.background.base.color,
                SendStatus::Sent => palette.success.base.color.scale_alpha(if palette.is_dark { 0.10 } else { 0.4 }),
                SendStatus::Pending => palette.background.base.color.scale_alpha(if palette.is_dark { 0.05 } else { 0.4 }),
            };
            container::Style {
                background: Some(background.into()),
                border: Border{
                    width: 0.,
                    radius: 10.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            }
        })
        .into()
    }
}
