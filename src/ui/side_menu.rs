use std::time::Instant;
use derive_more::Display;
use iced::{Alignment, Animation, Border, Color, Element, Length, Padding, Task, widget::{Column, Row, Stack, button, container, opaque, space, svg, text, tooltip}};

use crate::{icon, message::OperatorMessage, message_server::AcceptedMessage, messangers::{signal::SignalMessage, whatsapp}, notification, ui::{AppData, Screen, ext::PushMaybe, icons::{GROUP_REFRESH_ICON, SIGNAL_ICON, WHATSAPP_ICON}}};

use super::Message as MainMessage;

#[derive(Debug, Clone)]
pub enum Message {
    LinkSignal,
    LinkWhatsapp,
    Categories,
    Settings,
    Main,
    Animate,
    ToggleSideMenu,
    SetSignalState(LinkState),
    SetWhatsappState(LinkState),
    UpdateGroups,
    MessageFile,
}

impl From<Message> for MainMessage {
    fn from(value: Message) -> Self {
        Self::SideMenuMessage(value)
    }
}

pub struct SideMenu {
    now: Instant,
    pub open: Animation<bool>,
    pub signal_state: LinkState,
    pub whatsapp_state: LinkState,
}

impl SideMenu {
    pub fn new() -> Self {
        Self {
            now: Instant::now(),
            open: Animation::new(false)
            .quick()
            .easing(iced::animation::Easing::EaseInOut),
            signal_state: Default::default(),
            whatsapp_state: Default::default(),
        }
    }

    pub fn update(&mut self, message: Message, now: Instant, data: &mut AppData) -> Task<MainMessage> {
        self.now = now;
        match message {
            Message::LinkSignal => Task::done(SignalMessage::LinkBegin.into()),
            Message::LinkWhatsapp => {
                self.whatsapp_state = LinkState::Linking;
                Task::future(whatsapp::start_whatsapp_task()).discard()
            },
            Message::Categories => Task::done(MainMessage::SetScreen(Screen::Categories)),
            Message::Settings => Task::done(MainMessage::SetScreen(Screen::Settings)),
            Message::Main => Task::done(MainMessage::SetScreen(Screen::Main)),
            Message::Animate => Task::none(),
            Message::ToggleSideMenu => {
                self.open.go_mut(!self.open.value(), now);
                Task::none()
            },
            Message::SetSignalState(state) => {
                self.signal_state = state;
                if state == LinkState::Linked {
                    data.signal_logged = true
                }
                if state != LinkState::Linking {
                    Task::done(super::main_screen::Message::SetRegisterUrl(None).into())
                }
                else {
                    Task::none()
                }
            },
            Message::SetWhatsappState(state) => {
                self.whatsapp_state = state;
                if state == LinkState::Linked {
                    data.whatsapp_logged = true
                }
                if state != LinkState::Linking {
                    Task::done(super::main_screen::Message::SetWhatsappUrl(None).into())
                }
                else {
                    Task::none()
                }
            },
            Message::UpdateGroups => {
                Task::done(MainMessage::UpdateGroupList)
            },
            Message::MessageFile => {
                Task::future(async move {
                    let path = match rfd::AsyncFileDialog::new()
                    .add_filter("Файл повідомлень", &["json"])
                    .set_title("Виберіть файл з повідомленнями")
                    .pick_file()
                    .await {
                        Some(path) => path,
                        None => return MainMessage::None,
                    };

                    let s = match tokio::fs::read_to_string(path.path()).await {
                        Ok(s) => s,
                        Err(e) => {
                            log::error!("File read error: {}", &e);
                            return notification!("Помилка зчитування файла: {}", e)
                        },
                    };

                    let message = match serde_json::from_str::<Vec<OperatorMessage>>(&s) {
                        Ok(msgs) => msgs.into_iter().map(AcceptedMessage::from).collect::<Vec<_>>(),
                        Err(e) => {
                            log::error!("Message parse error: {e}");
                            let mut message = AcceptedMessage::from(s);
                            message.autosend_overwrite = true;
                            vec![message]
                        }
                    };

                    MainMessage::AcceptMessage(message)
                })
            },
        }
    }

    pub fn minimized(&self, selected_screen: Screen, data: &AppData) -> Element<'_, Message> {
        const BUTTON_PADDING: u32 = 5;

        container(
            Column::new()
            .align_x(Alignment::Center)
            .padding(Padding::default().horizontal(5).top(22))
            .push(
                sidebar_tooltip(button(
                        icon!(menu)
                        .size(26)
                    )
                    .style(button::subtle)
                    .on_press(Message::ToggleSideMenu)
                    .padding(Padding::default().horizontal(5)),
                    "Меню"
                )
            )
            .push(
                space()
                .height(36)
            )
            .push(
                Column::new()
                .spacing(20)
                .padding(Padding::default().bottom(15))
                .align_x(Alignment::Center)
                .push(
                    sidebar_tooltip(
                        button(
                            svg(svg::Handle::from_memory(SIGNAL_ICON))
                            .style(move |theme: &iced::Theme, _status| svg::Style {
                                color: match self.signal_state {
                                    LinkState::Linked => None,
                                    LinkState::Disconnected => Some(Color { r: 0.8, ..Color::BLACK }),
                                    _ => Some(theme.extended_palette().background.weaker.text),
                                },
                            })
                            .height(30)
                            .width(Length::Shrink)
                        )
                        .on_press_maybe((self.signal_state == LinkState::Unlinked).then_some(Message::LinkSignal))
                        .style(button::subtle)
                        .height(Length::Shrink)
                        .padding(Padding::default().vertical(BUTTON_PADDING).horizontal(5)),
                        text(format!("Signal: {}", self.signal_state))
                    )
                )
                .push(
                    sidebar_tooltip(
                        button(
                            svg(svg::Handle::from_memory(WHATSAPP_ICON))
                            .style(move |theme: &iced::Theme, _status| svg::Style {
                                color: match self.whatsapp_state {
                                    LinkState::Linked => None,
                                    LinkState::Disconnected => Some(Color { r: 0.8, ..Color::BLACK }),
                                    _ => Some(theme.extended_palette().background.weaker.text),
                                },
                            })
                            .height(30)
                            .width(Length::Shrink)
                        )
                        .on_press_maybe((self.whatsapp_state == LinkState::Unlinked).then_some(Message::LinkWhatsapp))
                        .style(button::subtle)
                        .height(Length::Shrink)
                        .padding(Padding::default().vertical(BUTTON_PADDING).horizontal(5)),
                        text(format!("Whatsapp: {}", self.whatsapp_state))
                    )
                )
                .push_maybe(
                    (!data.autoupdate_groups).then(|| sidebar_tooltip(
                        button(
                            svg(svg::Handle::from_memory(GROUP_REFRESH_ICON))
                            .style(move |theme: &iced::Theme, _status| svg::Style {
                                color: Some(theme.extended_palette().background.weaker.text),
                            })
                            .height(30)
                            .width(Length::Shrink)
                        )
                        .on_press(Message::UpdateGroups)
                        .style(button::subtle)
                        .height(Length::Shrink)
                        .padding(Padding::default().vertical(BUTTON_PADDING).horizontal(5)),
                        "Оновити групи"
                    ))
                )
                .push(
                    container(
                        space()
                        .width(Length::Fill)
                        .height(3)
                    )
                    .style(|theme: &iced::Theme| container::Style {
                        background: Some(theme.extended_palette().background.neutral.color.into()),
                        border: Border::default().rounded(1.5),
                        ..Default::default()
                    })
                )
                .push(
                    sidebar_tooltip(
                        button(
                            icon!(chat)
                            .size(28)
                        )
                        .on_press(Message::Main)
                        .style(menu_button_style(selected_screen == Screen::Main))
                        .padding(Padding::default().vertical(BUTTON_PADDING).horizontal(5)),
                        "Повідомлення"
                    )
                )
                .push(
                    sidebar_tooltip(
                        button(
                            icon!(group)
                            .size(28)
                        )
                        .on_press(Message::Categories)
                        .style(menu_button_style(selected_screen == Screen::Categories))
                        .padding(Padding::default().vertical(BUTTON_PADDING).horizontal(5)),
                        "Канали надсилання"    
                    )
                )
                .push_maybe(
                    data.message_file.then(|| container(
                        space()
                        .width(Length::Fill)
                        .height(3)
                    )
                    .style(|theme: &iced::Theme| container::Style {
                        background: Some(theme.extended_palette().background.neutral.color.into()),
                        border: Border::default().rounded(1.5),
                        ..Default::default()
                    }))
                )
                .push_maybe(
                    data.message_file.then(|| sidebar_tooltip(
                        button(
                            icon!(upload_file)
                            .size(28)
                        )
                        .on_press(Message::MessageFile)
                        .style(menu_button_style(false))
                        .padding(Padding::default().vertical(BUTTON_PADDING).horizontal(5)),
                        "Надіслати з файлу"    
                    ))
                )
                .push(
                    space()
                    .height(Length::Fill)
                )
                .push(
                    sidebar_tooltip(
                        button(
                            icon!(settings)
                            .size(28)
                        )
                        .on_press(Message::Settings)
                        .style(menu_button_style(selected_screen == Screen::Settings))
                        .padding(Padding::default().horizontal(5)),
                        "Налаштування"
                    )
                )
            )
            .height(Length::Fill)
            .width(50)
        )
        .style(|theme: &iced::Theme| container::Style {
            background: Some(theme.extended_palette().background.weakest.color.into()),
            ..Default::default()
        })
        .into()
    }

    pub fn menu_content(&self, selected_screen: Screen, data: &AppData) -> Element<'_, Message> {
        Column::new()
        .padding(Padding::default().horizontal(5).vertical(10))
        .spacing(20)
        .push(
            Row::new()
            .align_y(Alignment::Center)
            .spacing(20)
            .push(
                button(
                    icon!(menu)
                    .size(26)
                )
                .style(button::subtle)
                .on_press(Message::ToggleSideMenu)
                .padding(Padding::default().bottom(3).horizontal(5))
            )
            .push(
                text("Modern Sender")
                .size(24)
                .center()
            )
        )
        .push(
            menu_button(
                svg(svg::Handle::from_memory(SIGNAL_ICON))
                    .style(move |theme: &iced::Theme, _status| svg::Style {
                        color: match self.signal_state {
                            LinkState::Linked => None,
                            LinkState::Disconnected => Some(Color { r: 0.8, ..Color::BLACK }),
                            _ => Some(theme.extended_palette().background.weaker.text),
                        },
                    })
                    .content_fit(iced::ContentFit::Contain)
                    .height(30)
                    .width(30), 
                text("Signal")
                    .style(move |_| text::Style {
                        color: (self.signal_state == LinkState::Linked).then_some(Color::from_rgb(0.0, 0.0, 0.7)),
                    })
                    .height(Length::Fill)
                    .align_y(Alignment::Center)
                    .size(self.open.interpolate(0.1, 16.0, self.now)),
                (self.signal_state == LinkState::Unlinked).then_some(Message::LinkSignal),
                false
            )
        )
        .push(
            menu_button(
                svg(svg::Handle::from_memory(WHATSAPP_ICON))
                    .style(move |theme: &iced::Theme, _status| svg::Style {
                        color: match self.whatsapp_state {
                            LinkState::Linked => None,
                            LinkState::Disconnected => Some(Color { r: 0.8, ..Color::BLACK }),
                            _ => Some(theme.extended_palette().background.weaker.text),
                        },
                    })
                    .width(30)
                    .height(30), 
                text("Whatsapp")
                    .style(move |_| text::Style {
                        color: (self.whatsapp_state == LinkState::Linked).then_some(Color::from_rgb(0.0, 0.7, 0.0)),
                    })
                    .height(Length::Fill)
                    .align_y(Alignment::Center)
                    .size(self.open.interpolate(0.1, 16.0, self.now)), 
                (self.whatsapp_state == LinkState::Unlinked).then_some(Message::LinkWhatsapp),
                false
            )
        )
        .push_maybe(
            (!data.autoupdate_groups).then(|| menu_button(
                svg(svg::Handle::from_memory(GROUP_REFRESH_ICON))
                    .style(move |theme: &iced::Theme, _status| svg::Style {
                        color: Some(theme.extended_palette().background.weaker.text),
                    })
                    .width(30)
                    .height(30), 
                text("Оновити групи")
                    .height(Length::Fill)
                    .align_y(Alignment::Center)
                    .size(self.open.interpolate(0.1, 16.0, self.now)), 
                Some(Message::UpdateGroups),
                false
            ))
        )
        .push(
            container(
                space()
                .width(Length::Fill)
                .height(3)
            )
            .style(|theme: &iced::Theme| container::Style {
                background: Some(theme.extended_palette().background.neutral.color.into()),
                border: Border::default().rounded(1.5),
                ..Default::default()
            })
        )
        .push(
            menu_button(
                icon!(chat)
                .size(28), 
                text("Повідомлення")
                .height(Length::Fill)
                .align_y(Alignment::Center)
                .size(self.open.interpolate(0.1, 16.0, self.now)), 
                Some(Message::Main),
                selected_screen == Screen::Main
            )
        )
        .push(
            menu_button(
                icon!(group)
                    .size(28)
                    .align_y(Alignment::Center),
                text("Канали надсилання")
                .height(Length::Fill)
                .align_y(Alignment::Center)
                .size(self.open.interpolate(0.1, 16.0, self.now)),
                Some(Message::Categories),
                selected_screen == Screen::Categories
            )
        )
        .push_maybe(
            data.message_file.then(|| container(
                space()
                .width(Length::Fill)
                .height(3)
            )
            .style(|theme: &iced::Theme| container::Style {
                background: Some(theme.extended_palette().background.neutral.color.into()),
                border: Border::default().rounded(1.5),
                ..Default::default()
            })
            // .padding(Padding::default().horizontal(horizontal))
        )
            
        )
        .push_maybe(
            data.message_file.then(|| menu_button(
                icon!(upload_file)
                    .size(28)
                    .align_y(Alignment::Center),
                text("Відправити з файлу")
                .height(Length::Fill)
                .align_y(Alignment::Center)
                .size(self.open.interpolate(0.1, 16.0, self.now)),
                Some(Message::MessageFile),
                false
            ))
        )
        .push(
            space()
            .height(Length::Fill)
        )
        .push(
            menu_button(
                icon!(settings)
                    .size(28),
                text("Налаштування")
                    .height(Length::Fill)
                    .align_y(Alignment::Center)
                    .size(self.open.interpolate(0.1, 16.0, self.now)),
                Some(Message::Settings),
                selected_screen == Screen::Settings
            )
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }

    pub fn view(&self, selected_screen: Screen, data: &AppData) -> Element<'_, Message> {
        Stack::new()
        .push(
            container(
                space()
                .width(Length::Fill)
                .height(Length::Fill)
            )
            .style(|_| container::Style {
                background: Some(Color { a: self.open.interpolate(0.0, 0.4, self.now), ..Color::BLACK }.into()),
                ..Default::default()
            })
        )
        .push(
            opaque(
                container(
                    self.menu_content(selected_screen, data)
                )
                .width(
                    if self.open.is_animating(self.now) || self.open.value() {
                        self.open.interpolate(60.0, 200.0, self.now)
                    }
                    else {
                        0.0
                    }
                )
                .height(Length::Fill)
                .style(|theme: &iced::Theme| container::Style {
                    background: Some(theme.extended_palette().background.weakest.color.into()),
                    ..Default::default()
                })
            )
        )
        .into()
    }

    pub fn is_animating(&self, now: Instant) -> bool {
        self.open.is_animating(now)
    }
}


#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, Display)]
pub enum LinkState {
    #[default]
    #[display("Не прив'язано")]
    Unlinked,
    #[display("Підключення...")]
    Linking,
    #[display("Прив'язано")]
    Linked,
    #[display("Відключено")]
    Disconnected,
}


fn menu_button<'a>(icon: impl Into<Element<'a, Message>>, text: text::Text<'a>, on_press_maybe: Option<Message>, selected: bool) -> button::Button<'a, Message> {
    button(
        Row::new()
        .spacing(8)
        .width(Length::Fill)
        .push(
            icon
        )
        .push(
            text
        )
    )
    .style(menu_button_style(selected))
    .on_press_maybe(on_press_maybe)
    .padding(5)
    .height(Length::Shrink)
}

fn menu_button_style(selected: bool) -> impl Fn(&iced::Theme, button::Status) -> button::Style {
    move |theme, status| {
        let mut style = button::Style {
            border: Border::default().rounded(10),
            ..button::subtle(theme, status)
        };
        if selected {
            style.background = Some(theme.extended_palette().background.weak.color.into());
        }
        style
    }
}

fn sidebar_tooltip<'a>(content: impl Into<Element<'a, Message>>, text: impl Into<Element<'a, Message>>) -> tooltip::Tooltip<'a, Message> {
    tooltip(
        content,
        container(
            text
        )
        .padding(Padding::default().horizontal(3))
        .style(|theme: &iced::Theme| container::Style {
            background: Some(theme.extended_palette().background.weaker.color.into()),
            border: Border::default().rounded(2),
            ..Default::default()
        }),
        tooltip::Position::Right
    )
    .gap(0)
    // .delay(Duration::from_millis(500))
}
