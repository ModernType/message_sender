use std::{collections::{HashMap, VecDeque}, sync::Arc};

use iced::{Alignment, Border, Color, Element, Length, Task, alignment::Horizontal, border::Radius, widget::{Column, Row, button, checkbox, container, qr_code, scrollable, svg, text, text_editor}};
use serde::{Deserialize, Serialize};

use crate::{signal::SignalMessage, ui::{ext::ColorExt, message_history::SendMessageInfo}};

use super::Message as MainMessage;
use super::ext::PushMaybe;

type Key = [u8; 32];

#[derive(Debug, Clone)]
pub enum Message {
    SetRegisterUrl(url::Url),
    SetLinkState(LinkState),
    SetAutoSend(bool),
    TextEdit(text_editor::Action),
    SendMessage(String),
    SendMessagePressed,
    LinkBegin,
    ToggleGroup(Key, bool),
    SetGroups(Vec<(Key, String)>),
    UpdateGroups,
    UpdateMessageHistory,
    Settings,
    ShowMessageHistory(bool),
    DeleteMessage(Arc<SendMessageInfo>),
    EditMessage(Arc<SendMessageInfo>),
}

impl From<Message> for MainMessage {
    fn from(value: Message) -> Self {
        Self::MainScrMessage(value)
    }
}

#[derive(Debug, Default)]
pub(super) struct MainScreen {
    register_url: Option<qr_code::Data>,
    link_state: LinkState,
    autosend: bool,
    message_content: text_editor::Content,
    groups: HashMap<[u8; 32], Group>,
    message_history: VecDeque<Arc<SendMessageInfo>>,
    show_message_history: bool,
}

impl MainScreen {
    pub fn new(autosend: bool, groups: HashMap<[u8; 32], Group>) -> Self {
        Self {
            autosend,
            groups,
            ..Default::default()
        }
    }

    pub fn autosend(&self) -> bool {
        self.autosend
    }

    pub fn groups(&self) -> &HashMap<[u8; 32], Group> {
        &self.groups
    }

    pub fn update(&mut self, message: Message) -> Task<MainMessage> {
        match message {
            Message::SetRegisterUrl(url) => {
                let url = url.as_ref();
                self.register_url = Some(
                    qr_code::Data::new(url).unwrap()
                )
            },
            Message::SetLinkState(state) => {
                self.link_state = state;
                if state == LinkState::Linked {
                    self.register_url = None;
                }
            },
            Message::SetAutoSend(autosend) => {
                self.autosend = autosend;
            },
            Message::TextEdit(action) => {
                self.message_content.perform(action);
            },
            Message::SendMessagePressed => {
                let text = self.message_content.text();
                self.message_content = text_editor::Content::new();
                return Task::done(Message::SendMessage(text).into())
            }
            Message::SendMessage(message) => {
                let mut message = SendMessageInfo::new(message);
                
                for (key, group) in self.groups.iter() {
                    if group.active {
                        message.push(*key, group.title.clone());
                    }
                }
                let message = Arc::new(message);
                self.message_history.push_front(message.clone());

                return Task::batch([
                    Task::done(MainMessage::Notification("Початок відправки повідомлення".to_owned())),
                    Task::done(MainMessage::SendMessage(
                        message
                    ).into()),
                ])
            },
            Message::LinkBegin => {
                return Task::done(SignalMessage::LinkBegin.into())
            },
            Message::ToggleGroup(key, active) => {
                self.groups.get_mut(&key).unwrap().active = active;
            },
            Message::SetGroups(groups) => {
                for (key, title) in groups {
                    self.groups.entry(key)
                    .or_insert(Group { title, active: false });
                }
            },
            Message::UpdateGroups => {
                return Task::done(MainMessage::UpdateGroupList);
            },
            Message::UpdateMessageHistory => {
                // Makes window redraw to display actual information
            },
            Message::Settings => {
                return Task::done(MainMessage::SetScreen(super::Screen::Settings));
            },
            Message::ShowMessageHistory(state) => {
                self.show_message_history = state;
            },
            Message::DeleteMessage(message) => {
                return Task::done(MainMessage::DeleteMessage(message))
            },
            Message::EditMessage(_message) => {

            }
        }

        Task::none()
    }

    fn group_list(&self) -> Element<'_, Message> {
        scrollable(
            self.groups.iter().fold(Column::new()
            .spacing(3),
            |col, (key, group)| {
                col.push(
                    checkbox(group.active)
                    .label(&group.title)
                    .on_toggle(move |state| Message::ToggleGroup(key.clone(), state))
                )
            })
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }

    pub fn view(&self) -> Element<'_, Message> {
        Row::new()
        .spacing(10)
        .push(
            Column::new()
            .width(Length::FillPortion(2))
            .spacing(20)
            .padding(10)
            .align_x(Horizontal::Center)
            .push(
                text("Modern Sender")
                .size(30)
                .center()
                .width(Length::Fill)
                .color(
                    match self.link_state {
                        LinkState::Linked => Color::from_rgb(0.0, 0.8, 0.0),
                        LinkState::Unlinked => Color::from_rgb(0.8, 0.0, 0.0),
                        LinkState::Linking => Color::from_rgb(0.8, 0.8, 0.0)
                    }
                )
            )
            .push(
                Row::new()
                .push(
                    button(
                        svg(svg::Handle::from_memory(include_bytes!("icons/settings.svg")))
                    )
                    .style(button::text)
                    .on_press(Message::Settings)
                    .width(Length::Shrink)
                )
                .push(
                    match self.link_state {
                        LinkState::Unlinked => button(text("Приєднати пристрій").width(Length::Fill).center()).on_press(Message::LinkBegin),
                        LinkState::Linking => button(text("Приєднати пристрій").width(Length::Fill).center()).on_press_maybe(None),
                        LinkState::Linked => button(text("Оновити групи").width(Length::Fill).center()).on_press(Message::UpdateGroups),
                    }
                )
            )
            .push_maybe(
                self.register_url.as_ref().map(
                    |data| qr_code(data).style(|_| qr_code::Style { cell: Color::BLACK, background: Color::WHITE })
                )
            )
            .push(
                self.group_list()
            )
        )
        .push(
            Column::new()
            .width(Length::FillPortion(6))
            .height(Length::Fill)
            .align_x(Horizontal::Center)
            .spacing(10)
            .padding(10)
            .push(
                text_editor(&self.message_content)
                .placeholder("Введіть повідомлення")
                .height(Length::Fill)
                .on_action(Message::TextEdit)
            )
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
                    (self.link_state == LinkState::Linked).then_some(Message::SendMessagePressed)
                )
            )
            .push(
                checkbox(self.autosend)
                .label("Автоматична відправка")
                .on_toggle(Message::SetAutoSend)
            )
            
        )
        .push(
            if self.show_message_history {
                Element::new(
                    Row::new()
                    .width(Length::FillPortion(3))
                    .align_y(Alignment::Center)
                    .push(
                        button(text(">").center())
                        .height(80)
                        .style(|theme: &iced::Theme, _status| {
                            let palette = theme.palette();
                            button::Style {
                                background: Some(palette.background.darker(0.1).into()),
                                border: Border {
                                    width: 0.0,
                                    color: palette.text,
                                    radius: Radius {
                                        top_left: 10.0,
                                        bottom_left: 10.0,
                                        top_right: 0.0,
                                        bottom_right: 0.0,
                                    },
                                    ..Default::default()
                                },
                                text_color: palette.text,
                                ..Default::default()
                            }
                        })
                        .on_press(Message::ShowMessageHistory(false))   
                    )
                    .push(
                        scrollable(
                            self.message_history.iter().fold(
                                Column::new()
                                .padding(10)
                                .spacing(3)
                                .width(Length::Fill)
                                .height(Length::Fill),
                                |col, message_info| {
                                    col.push(
                                        message_info.view()
                                    )
                                }
                            )
                        )
                        .style(|theme, status| scrollable::Style {
                            container: container::Style {
                                background: Some(theme.palette().background.darker(0.1).into()),
                                ..Default::default()
                            },
                            ..scrollable::default(theme, status)
                        })
                        .width(Length::Fill)
                        .height(Length::Fill)
                    )
                )
            }
            else {
                container(
                    button(text("<").center())
                    .height(80)
                    .style(|theme: &iced::Theme, _status| {
                        let palette = theme.palette();
                        button::Style {
                            background: Some(palette.background.darker(0.1).into()),
                            border: Border {
                                width: 0.0,
                                color: palette.text,
                                radius: Radius {
                                    top_left: 10.0,
                                    bottom_left: 10.0,
                                    top_right: 0.0,
                                    bottom_right: 0.0,
                                },
                                ..Default::default()
                            },
                            text_color: palette.text,
                            ..Default::default()
                        }
                    })
                    .on_press(Message::ShowMessageHistory(true))
                )
                .center_y(Length::Fill)
                .into()
            }
        )
        .into()
    }
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct Group {
    pub title: String,
    pub active: bool,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy)]
pub enum LinkState {
    #[default]
    Unlinked,
    Linking,
    Linked
}
