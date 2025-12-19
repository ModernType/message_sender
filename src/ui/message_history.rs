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
    timestamp: AtomicU64,
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

    pub fn edit(self: &mut Arc<Self>, new_content: String) -> (Arc<Self>, Vec<GroupInfo>) {
        let mut info = SendMessageInfo::new(new_content);
        for group in self.groups.iter() {
            info.push(group.key, group.title.clone());
        }

        let groups = self.groups.clone();
        *self = Arc::new(info);

        (Arc::clone(&self), groups)
    }

    pub fn view<'a>(self: &'a Arc<Self>) -> Element<'a, super::main_screen::Message, Theme> {
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
            .spacing(3)
            .padding(5)
            .push(
                text(
                    self.content.graphemes(true).take(23).collect::<String>()
                )
                .center()
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
                        .color(status_color.unwrap())
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
                            .girth(4)
                            .style(move |theme: &Theme| progress_bar::Style {
                                bar: status_color.map(iced::Background::Color).unwrap_or_else(|| theme.palette().text.into()),
                                background: theme.palette().background.into(),
                                border: Default::default(),
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
                    .on_press_maybe({status == SendStatus::Sent}.then_some(super::main_screen::Message::DeleteMessage(Arc::clone(self))))
                )
                .push(
                    button(
                        svg(svg::Handle::from_memory(include_bytes!("icons/edit.svg")))
                    )
                    .style(button::primary)
                    .on_press_maybe({status == SendStatus::Sent}.then_some(super::main_screen::Message::EditMessage(Arc::clone(self))))
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
