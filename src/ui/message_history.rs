use std::sync::atomic::{AtomicU8, AtomicU64, Ordering};

use iced::{Border, Element, Font, Length, Theme, widget::{Column, Row, button, container, progress_bar, svg, text}};


impl<'a> From<&'a SendMessageInfo> for Element<'a, super::main_screen::Message, Theme> {
    fn from(value: &'a SendMessageInfo) -> Self {
        let status_color = match value.status.load(Ordering::Relaxed) {
            0 => Some(iced::Color::from_rgb(0.3, 0.3, 0.3)),
            1 => None,
            2 => Some(iced::Color::from_rgb(0.0, 0.6, 0.0)),
            3 => Some(iced::Color::from_rgb(0.6, 0.0, 0.0)),
            _ => unreachable!("You should not use values outside of `SendStatus` enum"),
        };
        let sent_count = value.sent_count();
        let sent = value.status.load(Ordering::Relaxed) == SendStatus::Sent as u8;
        container(
            Column::new()
            .spacing(3)
            .padding(5)
            .push(
                text(
                    if value.content.len() < 23 {
                        value.content.clone()
                    }
                    else {
                        format!("{}...", &value.content[..20])
                    }
                )
                .center()
                .width(Length::Fill)
            )
            .push(
                progress_bar(0.0 ..= value.groups.len() as f32, sent_count as f32)
                .length(Length::Fill)
                .girth(4)
                .style(move |theme: &Theme| progress_bar::Style {
                    bar: status_color.map(iced::Background::Color).unwrap_or_else(|| theme.palette().text.into()),
                    background: theme.palette().background.into(),
                    border: Default::default(),
                })
            )
            .push(
                text(format!("{}/{}", sent_count, value.groups.len()))
                .center()
                .width(Length::Fill)
                .style(move |_theme| {
                    text::Style { color: status_color }
                })
                .font_maybe(status_color.map(|_| Font {style: iced::font::Style::Italic, ..Font::default()}))
            )
            .push(
                Row::new()
                .spacing(3)
                .push(
                    button(
                        svg(svg::Handle::from_memory(include_bytes!("icons/delete.svg")))
                    )
                    .style(button::secondary)
                )
                .push(
                    button(
                        svg(svg::Handle::from_memory(include_bytes!("icons/edit.svg")))
                    )
                    .style(button::secondary)
                )
            )
        )
        .padding(3)
        .width(Length::Fill)
        .style(move |theme: &Theme| {
            let color = if let Some(c) = status_color {
                c
            } else {
                theme.palette().background
            };
            container::Style {
                border: Border{
                    width: 1.5,
                    color,
                    radius: 10.0.into(),
                },
                ..Default::default()
            }
        })
        .into()
    }
}

#[derive(Debug)]
pub struct SendMessageInfo {
    pub content: String,
    pub status: AtomicU8,
    pub groups: Vec<GroupInfo>,
}

#[repr(u8)]
pub enum SendStatus {
    Pending,
    Sending,
    Sent,
    Failed,
}

#[derive(Debug)]
pub struct GroupInfo {
    pub key: [u8; 32],
    pub title: String,
    timestamp: AtomicU64,
}

impl GroupInfo {
    pub fn new(key: [u8; 32], title: String) -> Self {
        Self { key, title, timestamp: AtomicU64::new(0) }
    }

    pub fn set_timestamp(&self, timestamp: u64, ordering: std::sync::atomic::Ordering) {
        self.timestamp.store(timestamp, ordering);
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

    pub fn sent(&self) -> bool {
        self.groups.iter().all(|g| g.sent(Ordering::Relaxed))
    }

    pub fn message(&self) -> &str {
        &self.content
    }

    pub fn set_status(&self, status: SendStatus, ordering: Ordering) {
        self.status.store(status as u8, ordering);
    }
}
