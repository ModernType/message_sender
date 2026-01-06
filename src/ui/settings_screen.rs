use std::net::SocketAddrV4;

use iced::{Alignment, Element, Font, Length, Task, widget::{Column, Row, button, checkbox, column, pick_list, rich_text, scrollable, span, svg, text, text_input}};
use rfd::FileHandle;

use crate::{send_categories::parse_networks_data, ui::{AppData, main_screen, theme::Theme}};

use super::Message as MainMessage;

#[derive(Debug, Clone)]
pub enum Message {
    ToMainScreen,
    ToggleMarkdown(bool),
    ToggleParallel(bool),
    ToggleShowGroups(bool),
    RecieveAddressEditChanged(String),
    HistoryLenEdit(String),
    ThemeSelected(Theme),
    ChooseNetworkFile,
    NetworkFileChoosen(Option<FileHandle>),
}

impl From<Message> for MainMessage {
    fn from(value: Message) -> Self {
        Self::SettingsScrMessage(value)
    }
}

#[derive(Debug)]
pub(super) struct SettingsScreen {
    recieve_address_edit: String,
    address_correct: bool,
}

impl SettingsScreen {
    pub fn new(data: &AppData) -> Self {
        Self {
            recieve_address_edit: data.recieve_address.to_string(),
            address_correct: true,
        }
    }

    pub fn update(&mut self, message: Message, data: &mut AppData) -> Task<MainMessage> {
        match message {
            Message::ToMainScreen => return Task::done(MainMessage::SetScreen(super::Screen::Main)),
            Message::ToggleMarkdown(markdown) => {
                data.markdown = markdown;
            },
            Message::ToggleParallel(parallel) => {
                data.parallel = parallel;
            },
            Message::ToggleShowGroups(state) => {
                data.show_groups = state;
            },
            Message::RecieveAddressEditChanged(recieve_address_edit) => {
                match recieve_address_edit.parse() {
                    Ok(addr) => {
                        data.recieve_address = addr;
                        self.address_correct = true;
                    },
                    Err(_) => self.address_correct = false,
                }
                self.recieve_address_edit = recieve_address_edit;
            },
            Message::HistoryLenEdit(text) => {
                if let Ok(num) = text.parse::<u32>() {
                    data.history_len = num;
                }
            },
            Message::ThemeSelected(theme) => {
                data.theme = theme.clone();
                return Task::done(MainMessage::ThemeChange(theme));
            },
            Message::ChooseNetworkFile => {
                return Task::perform(
                    rfd::AsyncFileDialog::new()
                    .add_filter("Файл мереж", &["json"])
                    .set_title("Виберіть файл з даними мереж")
                    .set_directory(std::path::absolute(".").unwrap())
                    .pick_file(),
                    Message::NetworkFileChoosen
                )
                .map(Into::into)
            },
            Message::NetworkFileChoosen(path) => {
                if let Some(path) = path {
                    return Task::future(async move {
                        let res: anyhow::Result<_> = async move {
                            let s = tokio::fs::read_to_string(path.path()).await?;
                            Ok(parse_networks_data(&s)?)
                        }.await;
                        match res {
                            Ok(networks) => MainMessage::RecivedNetworks(networks),
                            Err(e) => {
                                log::error!("Error parsing networks: {}", &e);
                                MainMessage::Notification(format!("Помилка зчитування мереж: {e}"))
                            }
                        }
                    })
                }
            },
            
        }

        Task::none()
    }

    pub fn view<'a>(&'a self, data: &'a AppData) -> Element<'a, Message> {
        Column::new()
        .width(Length::Fill)
        .height(Length::Fill)
        .push(
            button(
                svg(svg::Handle::from_memory(include_bytes!("icons/arrow_back.svg")))
            )
            .width(Length::Shrink)
            .height(Length::Shrink)
            .on_press(Message::ToMainScreen)
        )
        .push(
            scrollable(
                Column::new()
                .padding(20)
                .spacing(20)
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(Alignment::Center)
                .push(
                    Column::new()
                    .push(
                        text("Адреса серверу для прийому повідомлень")
                    )
                    .push(
                        text_input("Адреса серверу для прийому повідомлень", &self.recieve_address_edit)
                        .style(|theme, status| {
                            let mut default = text_input::default(theme, status);
                            if !self.address_correct {
                                default.border.color = theme.palette().danger;
                            }
                            default
                        })
                        .on_input(Message::RecieveAddressEditChanged)
                    )
                    .push(
                        text("Програму потрібно перезапустити, щоб зміни вступили у силу").font(Font { style:iced::font::Style::Italic, ..Default::default() })
                    )
                )
                .push(
                    column![
                        text("Кількість повідомлень в історії"),
                        text_input("Кількість повідомлень в історії", &data.history_len.to_string())
                        .on_input(Message::HistoryLenEdit)
                    ]
                )
                .push(
                    checkbox(data.show_groups)
                    .label("Показувати список груп на головному екрані")
                    .on_toggle(Message::ToggleShowGroups)
                )
                .push(
                    checkbox(data.markdown)
                    .label("Використовувати форматування markdown при відправці повідомлень")
                    .on_toggle(Message::ToggleMarkdown)
                )
                .push(
                    checkbox(data.parallel)
                    .label("Здійснювати відправку повідомлень паралельно (ЕКСПЕРИМЕНТАЛЬНО!!!)")
                    .on_toggle(Message::ToggleParallel)
                )
                .push(
                    Row::new()
                    .spacing(5)
                    .push(
                        text("Тема додатку")
                        .center()
                    )
                    .push(
                        pick_list(
                            Theme::ALL, 
                            Some(&data.theme),
                            Message::ThemeSelected
                        )
                    )
                )
                .push(
                    button("Завантажити список мереж")
                    .on_press(Message::ChooseNetworkFile)
                )
            )
            .width(Length::Fill)
            .height(Length::Fill)
        )
        .into()
    }
}