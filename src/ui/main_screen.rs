use std::{collections::{HashMap, VecDeque}, sync::Arc, time::{Duration, Instant}};

use iced::{Alignment, Animation, Border, Color, Element, Font, Length, Padding, Pixels, Task, alignment::Horizontal, border::Radius, mouse::Interaction, widget::{Column, Row, button, checkbox, container, mouse_area, qr_code, responsive, scrollable, svg, text, text_editor}};
use serde::{Deserialize, Serialize};

use crate::{code_point, icon, message::{OperatorMessage, SendMode}, message_server::AcceptedMessage, messangers::{Key, Messanger, signal::SignalMessage, whatsapp}, notification, ui::{AppData, main_screen, message_history::SendMessageInfo}};

use super::Message as MainMessage;
use super::ext::PushMaybe;


#[derive(Debug, Clone)]
pub enum Message {
    SetRegisterUrl(url::Url),
    SetSignalState(LinkState),
    SetWhatsappUrl(String),
    SetWhatsappState(LinkState),
    SetAutoSend(bool),
    TextEdit(text_editor::Action),
    SendMessage(String, Option<String>, Option<u64>),
    SendMessagePressed,
    LinkBegin,
    WhatsappLink,
    ToggleGroup(Key, SendMode),
    SetGroups(Vec<(Key, String)>),
    UpdateGroups,
    UpdateMessageHistory,
    Settings,
    ShowMessageHistory(bool),
    DeleteMessage(usize),
    EditMessage(usize),
    CancelEdit,
    ConfirmEdit,
    ShowMessanger(Messanger),
    MessageFile,
    NextMessage,
    Categories,
    Cancel(usize),
    CancelLink,
    RefreshMessage(usize)
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
    pub signal_state: LinkState,
    pub whatsapp_state: LinkState,
    message_content: text_editor::Content,
    pub message_history: VecDeque<Arc<SendMessageInfo>>,
    pub show_side_bar: Animation<bool>,
    pub edit: Option<Arc<SendMessageInfo>>,
    now: Instant,
    show_messanger: Messanger,
    pub message_queue: Vec<AcceptedMessage>,
    pub cur_message: Option<AcceptedMessage>
}

impl MainScreen {
    pub fn new() -> Self {
        Self {
            register_url: None,
            whatsapp_url: None,
            signal_state: Default::default(),
            whatsapp_state: Default::default(),
            message_content: Default::default(),
            message_history: Default::default(),
            show_side_bar: Animation::new(false)
                .quick()
                .easing(iced::animation::Easing::EaseInOut),
            edit: None,
            now: Instant::now(),
            show_messanger: Messanger::Signal,
            message_queue: Vec::new(),
            cur_message: None,
        }
    }

    pub fn update(&mut self, message: Message, now: Instant, data: &mut AppData) -> Task<MainMessage> {
        self.now = now;

        match message {
            Message::Categories => {
                return Task::done(MainMessage::SetScreen(super::Screen::Categories))
            },
            Message::Cancel(idx) => {
                let message = &self.message_history[idx];
                message.set_status(super::message_history::SendStatus::Deleted, std::sync::atomic::Ordering::Relaxed);
                return Task::done(SignalMessage::Cancel.into());
            },
            Message::CancelLink => {
                return Task::done(SignalMessage::CancelLink.into())
            },
            Message::RefreshMessage(idx) => {
                let message = &self.message_history[idx];
                message.set_status(super::message_history::SendStatus::Sending, std::sync::atomic::Ordering::Relaxed);
            },
            Message::SetRegisterUrl(url) => {
                let url = url.as_ref();
                self.register_url = Some(
                    qr_code::Data::new(url).unwrap()
                );
                self.signal_state = LinkState::Linking;
                self.show_side_bar.go_mut(true, now);
            },
            Message::SetWhatsappUrl(data) => {
                self.whatsapp_url = Some(
                    qr_code::Data::new(data.as_bytes()).unwrap()
                );
                self.whatsapp_state = LinkState::Linking;
                self.show_side_bar.go_mut(true, now);
            },
            Message::SetWhatsappState(state) => {
                self.whatsapp_state = state;
                if state != LinkState::Linking {
                    self.whatsapp_url = None;
                }
                if state == LinkState::Linked {
                    data.whatsapp_logged = true;
                }
            }
            Message::SetSignalState(state) => {
                self.signal_state = state;
                if state != LinkState::Linking {
                    self.register_url = None;
                }
                if state == LinkState::Linked {
                    data.signal_logged = true;
                }
            },
            Message::SetAutoSend(autosend) => {
                data.autosend = autosend;
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
            Message::LinkBegin => {
                return Task::done(SignalMessage::LinkBegin.into())
            },
            Message::ShowMessanger(messanger) => {
                self.show_messanger = messanger;
            }
            Message::WhatsappLink => {
                self.whatsapp_state = LinkState::Linking;
                return Task::perform(whatsapp::start_whatsapp_task(), |_| MainMessage::None);
            },
            Message::ToggleGroup(key, send_mode) => {
                data.groups.get_mut(&key).unwrap().send_mode = send_mode;
            },
            Message::SetGroups(groups) => {
                for (key, title) in groups {
                    data.groups.entry(key)
                    .or_insert(Group { title, send_mode: SendMode::Off });
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
                }
                else {
                    self.message_content = text_editor::Content::new();
                }
            },
            Message::MessageFile => {
                return Task::future(async move {
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

        Task::none()
    }

    fn signal_list<'a>(&'a self, groups: &'a HashMap<Key, main_screen::Group>) -> Element<'a, Message> {
        match self.signal_state {
            LinkState::Linked => {
                let mut groups = groups.iter()
                .filter_map(|(key, group)| match key {
                    Key::Signal(key) => Some((key, group)),
                    _ => None
                })
                .collect::<Vec<_>>();
                groups.sort_unstable_by(|(_, prev), (_, next)| prev.title.cmp(&next.title));
                let col = groups.into_iter().fold(
                    Column::new().spacing(3).padding(10),
                    |col, (key, group)| col.push(
                    checkbox(group.active())
                    .label(&group.title)
                    .on_toggle(move |_| Message::ToggleGroup(Key::Signal(*key), group.send_mode.next()))
                    .icon(
                        checkbox::Icon {
                            font: crate::ui::icons::FONT,
                            code_point: if let SendMode::Frequency = group.send_mode { code_point!(graphic_eq) }
                                        else { code_point!(check) },
                            size: Some(Pixels::from(14)),
                            line_height: text::LineHeight::default(),
                            shaping: text::Shaping::Basic,
                        }
                    )
                ));
                scrollable(col)
                .style(|theme, status| {
                    let default = scrollable::default(theme, status);
                    let palette = theme.extended_palette();

                    scrollable::Style {
                        container: container::Style {
                            background: Some(palette.background.weaker.color.into()),
                            text_color: Some(palette.background.weaker.text.into()),
                            border: Border::default().rounded(Radius::default().bottom(15)),
                            ..Default::default()
                        },
                        ..default
                    }
                })
                .width(Length::Fill)
                .height(Length::Fill)
                .into()
            },
            LinkState::Linking => {
                button(
                    text("Відмінити")
                    .width(Length::Fill)
                    .center()
                )
                .style(button::subtle)
                .on_press(Message::CancelLink)
                .into()
            },
            LinkState::Unlinked => {
                button(
                    text("Приєднати пристрій")
                    .width(Length::Fill)
                    .center()
                )
                .on_press(Message::LinkBegin)
                .into()
            },
        }
    }

    fn whatsapp_list<'a>(&'a self, groups: &'a HashMap<Key, main_screen::Group>) -> Element<'a, Message> {
        match self.whatsapp_state {
            LinkState::Linked => {
                let mut groups = groups.iter()
                .filter_map(|(key, group)| match key {
                    Key::Whatsapp(key) => Some((key, group)),
                    _ => None
                })
                .collect::<Vec<_>>();
                groups.sort_unstable_by(|(_, prev), (_, next)| prev.title.cmp(&next.title));
                let col = groups.into_iter().fold(
                    Column::new().spacing(3).padding(10),
                    |col, (key, group)| col.push(
                    checkbox(group.active())
                    .label(&group.title)
                    .on_toggle(move |_| Message::ToggleGroup(Key::Whatsapp(key.clone()), group.send_mode.next()))
                    .icon(
                        checkbox::Icon {
                            font: crate::ui::icons::FONT,
                            code_point: if let SendMode::Frequency = group.send_mode { code_point!(graphic_eq) }
                                        else { code_point!(check) },
                            size: Some(Pixels::from(14)),
                            line_height: text::LineHeight::default(),
                            shaping: text::Shaping::Basic,
                        }
                    )
                ));
                scrollable(col)
                .style(|theme, status| {
                    let default = scrollable::default(theme, status);
                    let palette = theme.extended_palette();

                    scrollable::Style {
                        container: container::Style {
                            background: Some(palette.background.weaker.color.into()),
                            text_color: Some(palette.background.weaker.text.into()),
                            border: Border::default().rounded(Radius::default().bottom(15)),
                            ..Default::default()
                        },
                        ..default
                    }
                })
                .width(Length::Fill)
                .height(Length::Fill)
                .into()
            },
            LinkState::Linking => {
                button(
                    text("Відмінити")
                    .width(Length::Fill)
                    .center()
                )
                .style(button::subtle)
                // .on_press(Message::CancelLink)
                .into()
            },
            LinkState::Unlinked => {
                button(
                    text("Приєднати пристрій")
                    .width(Length::Fill)
                    .center()
                )
                .on_press(Message::WhatsappLink)
                .into()
            },
        }
    }

    fn group_list<'a>(&'a self, groups: &'a HashMap<Key, main_screen::Group>) -> Element<'a, Message> {
        Column::new()
        .push(
            Row::new()
            .push(
                button(
                    svg(svg::Handle::from_memory(include_bytes!("icons/signal.svg")))
                    .style(|theme: &iced::Theme, _status| {
                        let palette = theme.extended_palette();
                        svg::Style { color: match self.signal_state {
                            LinkState::Linked => None,
                            LinkState::Unlinked | LinkState::Linking => Some(palette.background.strong.color),
                        } }
                    })
                )
                .padding(Padding::default().horizontal(15).vertical(5))
                .style(messanger_button)
                .on_press_maybe((self.show_messanger != Messanger::Signal).then(|| Message::ShowMessanger(Messanger::Signal)))
            )
            .push(
                button(
                    svg(svg::Handle::from_memory(include_bytes!("icons/whatsapp.svg")))
                    .style(|theme: &iced::Theme, _status| {
                        let palette = theme.extended_palette();
                        svg::Style { color: match self.whatsapp_state {
                            LinkState::Linked => None,
                            LinkState::Unlinked | LinkState::Linking => Some(palette.background.strong.color),
                        } }
                    })
                )
                .padding(Padding::default().horizontal(15).vertical(5))
                .style(messanger_button)
                .on_press_maybe((self.show_messanger != Messanger::Whatsapp).then(|| Message::ShowMessanger(Messanger::Whatsapp)))
            )
        )
        .push(
            match self.show_messanger {
                Messanger::Signal => self.signal_list(groups),
                Messanger::Whatsapp => self.whatsapp_list(groups),
            }
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
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
                background: Some(theme.extended_palette().background.weaker.color.into()),
                ..Default::default()
            },
            ..scrollable::default(theme, status)
        })
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }

    fn side_bar(&self, message_file: bool) -> Element<'_, Message> {
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
                self.main_part(message_file)
            )
            .height(Length::Fill)
            .width(Length::Shrink)
        )
    }

    fn main_part(&self, message_file: bool) -> Element<'_, Message> {
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
                                    (self.signal_state == LinkState::Linked || self.whatsapp_state == LinkState::Linked)
                                    && !self.message_content.is_empty()
                                ).then_some(Message::SendMessagePressed)
                            )
                        )
                        .push_maybe(
                            message_file.then(||
                                button(
                                    text("Надіслати з файлу")
                                    .center()
                                    .width(Length::Fill)
                                    .font(iced::Font {
                                        weight: iced::font::Weight::Bold,
                                        ..iced::Font::DEFAULT
                                    })
                                )
                                .style(button::subtle)
                                .on_press(Message::MessageFile)
                            )
                        )
                    )
                }
            )
        };

        container(col)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(|theme: &iced::Theme| container::background(theme.extended_palette().background.weaker.color))
        .into()
    }

    pub fn view<'a>(&'a self, data: &'a AppData) -> Element<'a, Message> {
        Row::new()
        .spacing(7)
        .push(
            Column::new()
            .width(180)
            .spacing(20)
            .padding(10)
            .align_x(Horizontal::Center)
            .push(
                text("MODERN SENDER")
                .size(26)
                .center()
                .width(Length::Fill)
            )
            .push(
                Row::new()
                .spacing(8)
                .align_y(Alignment::Center)
                .push(
                    button(
                        icon!(settings)
                        .size(26)
                    )
                    .on_press(Message::Settings)
                    .style(|theme, status| {
                        let mut style = button::subtle(theme, status);
                        style.border = Border::default().rounded(20);
                        style
                    })
                )
                .push(
                    button(
                        text("Категорії надсилання")
                        .width(Length::Fill)
                        .center()
                    )
                    .style(|theme, status| {
                        let mut style = button::subtle(theme, status);
                        style.border = Border::default().rounded(20);
                        style
                    })
                    .on_press(Message::Categories)
                )
            )
            .push_maybe(
                (
                    (self.signal_state == LinkState::Linked || self.whatsapp_state == LinkState::Linked)
                    && !data.autoupdate_groups
                )
                .then(||
                    button(
                        text("Оновити групи").width(Length::Fill).center()
                    )
                    .on_press(Message::UpdateGroups)
                    .style(
                        |theme: &iced::Theme, status| {
                            let palette = theme.extended_palette();
                            button::Style {
                                background: match status {
                                    button::Status::Active => Some(palette.background.strong.color.into()),
                                    button::Status::Hovered => Some(palette.background.stronger.color.into()),
                                    button::Status::Pressed | button::Status::Disabled => Some(palette.background.strongest.color.into())
                                },
                                text_color: palette.background.strong.text,
                                border: Border::default().rounded(5),
                                ..Default::default()
                            }
                        }
                    )
                )
            )
            .push_maybe(
                data.show_groups.then(|| self.group_list(&data.groups))
            )
        )
        .push(
            self.message_history()
        )
        .push(
            self.side_bar(data.message_file)
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

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy)]
pub enum LinkState {
    #[default]
    Unlinked,
    Linking,
    Linked
}

fn messanger_button(theme: &iced::Theme, status: button::Status) -> button::Style {
    let palette = theme.extended_palette();
    button::Style {
        background: Some(
            if matches!(status, button::Status::Active | button::Status::Hovered) { palette.background.base.color }
            else { palette.background.weaker.color } .into()
        ),
        border: Border::default().rounded(Radius::default().top(10)),
        ..Default::default()
    }
}
