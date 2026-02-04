use std::{collections::{HashMap, VecDeque}, sync::Arc, time::Instant};

use iced::{Alignment, Animation, Border, Color, Element, Length, Task, alignment::Horizontal, border::Radius, widget::{Column, Row, button, container, qr_code, responsive, scrollable, space, text, text_editor}};
use serde::{Deserialize, Serialize};

use crate::{icon, message::SendMode, message_server::AcceptedMessage, messangers::{Key, signal::SignalMessage}, ui::{AppData, message_history::SendMessageInfo}};

use super::Message as MainMessage;
use super::ext::PushMaybe;


#[derive(Debug, Clone)]
pub enum Message {
    SetRegisterUrl(Option<url::Url>),
    SetWhatsappUrl(Option<String>),
    TextEdit(text_editor::Action),
    SendMessage(String, Option<String>, Option<u64>),
    SendMessagePressed,
    SetGroups(Vec<(Key, String)>),
    UpdateMessageHistory,
    ShowMessageHistory(bool),
    DeleteMessage(usize),
    EditMessage(usize),
    CancelEdit,
    ConfirmEdit,
    NextMessage,
    Cancel(usize),
    RefreshMessage(usize),
    SendMessageDirect(Arc<SendMessageInfo>),
}

impl From<Message> for MainMessage {
    fn from(value: Message) -> Self {
        Self::MainScrMessage(value)
    }
}

#[derive(Debug)]
pub(super) struct MainScreen {
    register_url: Option<qr_code::Data>,
    whatsapp_url: Option<qr_code::Data>,
    message_content: text_editor::Content,
    pub message_history: VecDeque<Arc<SendMessageInfo>>,
    pub show_side_bar: Animation<bool>,
    pub edit: Option<Arc<SendMessageInfo>>,
    now: Instant,
    pub message_queue: Vec<AcceptedMessage>,
    pub cur_message: Option<AcceptedMessage>
}

impl MainScreen {
    pub fn new() -> Self {
        Self {
            register_url: None,
            whatsapp_url: None,
            message_content: Default::default(),
            message_history: Default::default(),
            show_side_bar: Animation::new(false)
                .quick()
                .easing(iced::animation::Easing::EaseInOut),
            edit: None,
            now: Instant::now(),
            message_queue: Vec::new(),
            cur_message: None,
        }
    }

    pub fn update(&mut self, message: Message, now: Instant, data: &mut AppData) -> Task<MainMessage> {
        self.now = now;

        match message {
            Message::Cancel(idx) => {
                let message = &self.message_history[idx];
                message.set_status(super::message_history::SendStatus::Deleted, std::sync::atomic::Ordering::Relaxed);
                return Task::done(SignalMessage::Cancel.into());
            },
            Message::RefreshMessage(idx) => {
                let message = &self.message_history[idx];
                message.set_status(super::message_history::SendStatus::Sending, std::sync::atomic::Ordering::Relaxed);
            },
            Message::SetRegisterUrl(url) => {
                if let Some(url) = url {
                    let url = url.as_ref();
                    self.register_url = Some(
                        qr_code::Data::new(url).unwrap()
                    );
                    self.show_side_bar.go_mut(true, now);
                }
                else {
                    self.register_url = None
                }

                self.maybe_hide();
            },
            Message::SetWhatsappUrl(data) => {
                if let Some(data) = data {
                    self.whatsapp_url = Some(
                        qr_code::Data::new(data.as_bytes()).unwrap()
                    );
                    self.show_side_bar.go_mut(true, now);
                }
                else {
                    self.whatsapp_url = None
                }

                self.maybe_hide();
            },
            Message::TextEdit(action) => {
                self.message_content.perform(action);
            },
            Message::SendMessagePressed => {
                let (text, freq, network) = if let Some(mut message) = self.cur_message.take() {
                    message.text = self.message_content.text();
                    (message.text, message.freq, message.network)
                }
                else {
                    (self.message_content.text(), None, None)
                };

                self.message_content = text_editor::Content::new();

                return Task::batch([
                    Task::done(Message::NextMessage.into()),
                    Task::done(Message::SendMessage(text, freq, network).into()),
                ])
            }
            Message::SendMessage(message, freq, network) => {
                let mut message = SendMessageInfo::new(message, freq);

                if let Some(network) = network {
                    log::info!("Has network {}", &network);
                    let mut groups: HashMap<&Key, SendMode> = HashMap::new();
                    let mut use_general = false;

                    for category in data.categories.iter() {
                        if category.contains_network(&network) {
                            for (key, mode) in category.groups.iter() {
                                groups.entry(key)
                                .and_modify(|mode| mode.update(*mode))
                                .or_insert(*mode);
                            }
                            use_general |= category.use_general;
                        }
                    }

                    if !groups.is_empty() {
                        log::info!("Has in category");
                        if use_general {
                            for (key, group) in data.groups.iter() {
                                if group.active() {
                                    groups.entry(key)
                                    .and_modify(|mode| mode.update(group.send_mode))
                                    .or_insert(group.send_mode);
                                }
                            }
                        }
                        for(key, mode) in groups {
                            message.push(key.clone(), mode);
                        }
                    }
                    else {
                        log::info!("Getting general");
                        for (key, group) in data.groups.iter() {
                            if group.active() {
                                message.push(key.clone(), group.send_mode);
                            }
                        }
                    }
                }
                else {
                    log::info!("Getting general");
                    for (key, group) in data.groups.iter() {
                        if group.active() {
                            message.push(key.clone(), group.send_mode);
                        }
                    }
                }
                
                let message = Arc::new(message);

                if self.message_history.len() >= data.history_len as usize {
                    self.message_history.pop_back();
                }
                self.message_history.push_front(message.clone());


                return Task::batch([
                    Task::done(MainMessage::Notification("Початок відправки повідомлення".to_owned())),
                    Task::done(MainMessage::SendMessage(
                        message
                    )),
                ])
            },
            Message::SendMessageDirect(message) => {
                message.set_status(super::message_history::SendStatus::Pending, std::sync::atomic::Ordering::Relaxed);
                return Task::done(MainMessage::SendMessage(message))
            },
            Message::SetGroups(groups) => {
                for (key, title) in groups {
                    data.groups.entry(key)
                    .or_insert(Group { title, send_mode: SendMode::Off });
                }
            },
            Message::UpdateMessageHistory => {
                // Makes window redraw to display actual information
            },
            Message::ShowMessageHistory(state) => {
                self.show_side_bar.go_mut(state, self.now);
            },
            Message::DeleteMessage(idx) => {
                return Task::done(MainMessage::DeleteMessage(Arc::clone(&self.message_history[idx])))
            },
            Message::EditMessage(idx) => {
                self.show_side_bar.go_mut(true, now);
                match self.edit {
                    Some(ref mut editing_message) => {
                        std::mem::swap(
                            editing_message,
                            self.message_history.get_mut(idx).unwrap()
                        );
                        let content = text_editor::Content::with_text(&editing_message.content);
                        self.message_content = content;
                    },
                    None => {
                        let message = self.message_history.remove(idx).unwrap();
                        let content = text_editor::Content::with_text(&message.content);
                        self.message_content = content;
                        self.edit = Some(message);
                    }
                }
            },
            Message::CancelEdit => {
                self.message_content = text_editor::Content::new();
                if self.message_history.len() >= data.history_len as usize {
                    self.message_history.pop_back();
                }
                self.message_history.push_front(
                    self.edit.take().unwrap()
                );
                self.show_side_bar.go_mut(false, now);
            },
            Message::ConfirmEdit => {
                let mut arc_message = self.edit.take().unwrap();
                // We are making it mut, because we know that it already finished sending and is available only in `self.edit`
                let message = Arc::get_mut(&mut arc_message).unwrap();
                
                let timestamps = message.groups_signal.iter().map(|group| group.timestamp.swap(0, std::sync::atomic::Ordering::Relaxed)).collect();
                let whatsapp_ids = message.groups_whatsapp.iter_mut()
                .map(|msg| {
                    let mut id = msg.sent_id.lock().unwrap();
                    std::mem::take(&mut *id)
                })
                .collect::<Vec<_>>();

                let new_message = self.message_content.text();
                self.message_content = text_editor::Content::new();

                message.content = new_message;
                message.set_status(super::message_history::SendStatus::Pending, std::sync::atomic::Ordering::Relaxed);

                if self.message_history.len() >= data.history_len as usize {
                    self.message_history.pop_back();
                }
                self.message_history.push_front(arc_message.clone());

                self.show_side_bar.go_mut(false, now);

                return Task::done(MainMessage::EditMessage(arc_message, timestamps, whatsapp_ids));
            },
            Message::NextMessage => {
                self.cur_message = self.message_queue.pop();
                if let Some(message) = &self.cur_message {
                    self.message_content = text_editor::Content::with_text(&message.text);
                    self.show_side_bar.go_mut(true, now);
                }
                else {
                    self.message_content = text_editor::Content::new();
                    self.show_side_bar.go_mut(false, now);
                }
            },
        }

        Task::none()
    }

    fn maybe_hide(&mut self) {
        if self.register_url.is_none()
        && self.whatsapp_url.is_none()
        && self.edit.is_none()
        && self.message_queue.is_empty()
        && self.message_content.is_empty()
        {
            self.show_side_bar.go_mut(false, self.now);
        }
    }

    fn message_history(&self) -> Element<'_, Message> {
        scrollable(
            self.message_history.iter().enumerate().fold(
                Column::new()
                .padding(10)
                .spacing(3)
                .width(Length::Fill)
                .height(Length::Fill),
                |col, (idx, message_info)| {
                    col.push(
                        message_info.view(idx)
                    )
                }
            )
        )
        .style(|theme, status| scrollable::Style {
            container: container::Style {
                background: Some(theme.extended_palette().background.base.color.into()),
                ..Default::default()
            },
            ..scrollable::default(theme, status)
        })
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }

    fn side_bar(&self) -> Element<'_, Message> {
        Element::new(
            Row::new()
            .align_y(Alignment::Center)
            .push(
                button(if self.show_side_bar.value() { icon!(arrow_right) } else { icon!(arrow_left) }.center().size(24))
                .padding(3)
                .height(80)
                .style(|theme: &iced::Theme, button_status| {
                    let palette = theme.extended_palette();
                    button::Style {
                        background: match button_status {
                            button::Status::Active => Some(palette.background.weaker.color.into()),
                            _ => Some(palette.background.neutral.color.into())
                        },
                        border: Border {
                            radius: Radius {
                                top_left: 10.0,
                                bottom_left: 10.0,
                                top_right: 0.0,
                                bottom_right: 0.0,
                            },
                            ..Default::default()
                        },
                        text_color: palette.background.weaker.text,
                        ..Default::default()
                    }
                })
                .on_press(Message::ShowMessageHistory(!self.show_side_bar.value()))   
            )
            .push(
                self.main_part()
            )
            .height(Length::Fill)
        )
    }

    fn main_part(&self) -> Element<'_, Message> {
        let mut col = Column::new()
        .width(self.show_side_bar.interpolate(0., 450., self.now))
        .height(Length::Fill)
        .align_x(Horizontal::Center)
        .spacing(10)
        .padding(10);
        
        col = if self.register_url.is_some() || self.whatsapp_url.is_some() {
            col.push_maybe(
                self.register_url.as_ref().map(
                    |data| responsive(
                        |size| qr_code(data)
                        .style(|theme: &iced::Theme| {
                            let palette = theme.extended_palette();
                            qr_code::Style {
                                cell: Color::BLACK,
                                background: if palette.is_dark {
                                    Color::WHITE
                                }
                                else {
                                    palette.background.base.color
                                }
                            }
                        })
                        .total_size(size.height.min(size.width))
                        .into()
                    )
                )
            )
            .push_maybe(
                self.whatsapp_url.as_ref().map(
                    |data| responsive(
                        |size| qr_code(data)
                        .style(|theme: &iced::Theme| {
                            let palette = theme.extended_palette();
                            qr_code::Style {
                                cell: Color::BLACK,
                                background: if palette.is_dark {
                                    Color::WHITE
                                }
                                else {
                                    palette.background.base.color
                                }
                            }
                        })
                        .total_size(size.height.min(size.width))
                        .into()
                    )
                )
            )
        }
        else {
            col.push(
                text_editor(&self.message_content)
                .placeholder("Введіть повідомлення")
                .height(Length::Fill)
                .on_action(Message::TextEdit)
                .highlight("md", iced::highlighter::Theme::SolarizedDark)
                .style(|theme, status| {
                    let mut style = text_editor::default(theme, status);
                    style.background = theme.extended_palette().background.weakest.color.into();
                    style
                })
            )
            .push(
                if self.edit.is_some() {
                    Element::from(
                        Row::new()
                        .spacing(5)
                        .push(
                            button(
                                text("Відмінити")
                                .center()
                                .width(Length::Fill)
                            )
                            .style(button::secondary)
                            .on_press(Message::CancelEdit)
                        )
                        .push(
                            button(
                                text("Редагувати")
                                .center()
                                .width(Length::Fill)
                            )
                            .on_press(Message::ConfirmEdit)
                        )
                    )
                }
                else if self.cur_message.is_some() {
                    Element::from(
                        Row::new()
                        .spacing(5)
                        .push(
                            button(
                                text("Відмінити")
                                .center()
                                .width(Length::Fill)
                            )
                            .style(button::secondary)
                            .on_press(Message::NextMessage)
                        )
                        .push(
                            button(
                                text("Надіслати повідомлення")
                                .center()
                                .width(Length::Fill)
                            )
                            .on_press(Message::SendMessagePressed)
                        )
                    )
                }
                else {
                    Element::from(
                        Column::new()
                        .spacing(5)
                        .push(
                            button(
                                text("Надіслати повідомлення")
                                .center()
                                .width(Length::Fill)
                                .font(iced::Font {
                                    weight: iced::font::Weight::Bold,
                                    ..iced::Font::DEFAULT
                                })
                            )
                            .on_press_maybe(
                                (
                                    !self.message_content.is_empty()
                                ).then_some(Message::SendMessagePressed)
                            )
                        )
                    )
                }
            )
        };

        container(col)
        .style(|theme: &iced::Theme| container::background(theme.extended_palette().background.weaker.color))
        .into()
    }

    pub fn tutorial(&self) -> Element<'_, Message> {
        Column::new()
        .padding(15)
        .push(
            space()
            .height(100)
        )
        .push(
            Row::new()
            .spacing(10)
            .align_y(Alignment::Center)
            .push(
                icon!(arrow_back)
                .size(28)
            )
            .push(
                "Для початку роботи прив'яжіть акаунт месенджера до програми"
            )
        )
        .push(
            space()
            .height(Length::Fill)
        )
        .push(
            Column::new()
            .push(
                "Налаштуйте IP-адресу для прийому повідомлень у налаштуваннях"
            )
            .push(
                icon!(subdirectory_arrow_left)
                .size(28)
            )
        )
        .height(Length::Fill)
        .into()
    }

    pub fn view<'a>(&'a self, tutorial: bool) -> Element<'a, Message> {
        Row::new()
        .spacing(7)
        .push(
            if tutorial {
                self.tutorial()
            }
            else {
                self.message_history()
            }
        )
        .push_maybe(
            (self.show_side_bar.is_animating(self.now) || self.show_side_bar.value()).then(|| {
                #[cfg(debug_assertions)]
                let el = self.side_bar();
                #[cfg(not(debug_assertions))]
                let el = self.main_part();
                el
            })
        )
        .into()
    }
}

#[derive(Debug, Serialize, Deserialize, Default, Clone, PartialEq, Eq, Hash)]
pub struct Group {
    pub title: String,
    pub send_mode: SendMode,
}

impl Group {
    pub fn active(&self) -> bool {
        self.send_mode.active()
    }
}
