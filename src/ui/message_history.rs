use std::sync::{Arc, atomic::{AtomicU8, AtomicU64, Ordering}};

use iced::{Border, Element, Length, Theme, widget::{Column, Row, button, container, progress_bar, svg, text}};
use unicode_segmentation::UnicodeSegmentation;

#[derive(Debug)]
pub struct SendMessageInfo {
    pub content: String,
    pub status: AtomicU8,
    pub groups: Vec<GroupInfo>,
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
pub struct GroupInfo {
    pub key: [u8; 32],
    pub title: String,
    pub(super) timestamp: AtomicU64,
}

impl Clone for GroupInfo {
    fn clone(&self) -> Self {
        Self {
            key: self.key,
            title: self.title.clone(),
            timestamp: AtomicU64::new(self.timestamp.load(Ordering::Relaxed)),
        }
    }
}

impl GroupInfo {
    pub fn new(key: [u8; 32], title: String) -> Self {
        Self { key, title, timestamp: AtomicU64::new(0) }
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

impl SendMessageInfo {
    pub fn new(content: String) -> Self {
        Self {
            content,
            status: AtomicU8::new(SendStatus::Pending as u8),
            groups: Vec::new(),
        }
    }

    pub fn push(&mut self, group_key: [u8; 32], title: String) {
        self.groups.push(GroupInfo::new(group_key, title));
    }

    pub fn sent_count(&self) -> usize {
        self.groups.iter().filter(|g| g.sent(Ordering::Relaxed)).count()
    }

    // pub fn sent(&self) -> bool {
    //     self.groups.iter().all(|g| g.sent(Ordering::Relaxed))
    // }

    // pub fn message(&self) -> &str {
    //     &self.content
    // }

    pub fn set_status(&self, status: SendStatus, ordering: Ordering) {
        self.status.store(status as u8, ordering);
    }

    pub fn view<'a>(self: &'a Arc<Self>, idx: usize) -> Element<'a, super::main_screen::Message, Theme> {
        let status_color = match self.status.load(Ordering::Relaxed) {
            0 => Some(iced::Color::from_rgb(0.3, 0.3, 0.3)),
            1 => None,
            2 => Some(iced::Color::from_rgb(0.0, 0.6, 0.0)),
            3 | 4=> Some(iced::Color::from_rgb(0.6, 0.0, 0.0)),
            _ => unreachable!("You should not use values outside of `SendStatus` enum"),
        };
        let sent_count = self.sent_count();
        let status = SendStatus::from(self.status.load(Ordering::Relaxed));
        container(
            Column::new()
            .spacing(5)
            .padding(5)
            .push(
                text(
                    self.content.graphemes(true).take(23).collect::<String>()
                )
                .center()
                .wrapping(text::Wrapping::None)
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
                    SendStatus::Deleted => Element::from(
                        text("Видалено")
                        .style(|theme: &Theme| text::Style { color: Some(theme.extended_palette().danger.base.color) })
                        .center()
                        .width(Length::Fill)
                    ),
                    SendStatus::Sent => Element::from(
                        text("Відправлено")
                        .style(|theme: &Theme| text::Style { color: Some(theme.extended_palette().success.strong.color) })
                        .center()
                        .width(Length::Fill)
                    ),
                    _ => Element::from(
                        Column::new()
                        .push(
                            text(format!("{}/{}", sent_count, self.groups.len()))
                            .color_maybe(status_color)
                            .center()
                            .width(Length::Fill)
                        )
                        .push(
                            progress_bar(0.0 ..= self.groups.len() as f32, sent_count as f32)
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
            .push(
                Row::new()
                .spacing(3)
                .push(
                    button(
                        svg(svg::Handle::from_memory(include_bytes!("icons/delete.svg")))
                    )
                    .style(button::danger)
                    .on_press_maybe({status == SendStatus::Sent}.then_some(super::main_screen::Message::DeleteMessage(idx)))
                )
                .push(
                    button(
                        svg(svg::Handle::from_memory(include_bytes!("icons/edit.svg")))
                    )
                    .style(button::secondary)
                    .on_press_maybe({status == SendStatus::Sent}.then_some(super::main_screen::Message::EditMessage(idx)))
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
