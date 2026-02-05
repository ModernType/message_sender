use iced::{Alignment, Border, Color, Element, Font, Length, Padding, Shadow, Task, Vector, widget::{Column, Row, button, checkbox, column, container, pick_list, scrollable, text, text_input}};
use rfd::FileHandle;

use crate::{send_categories::parse_networks_data, ui::{AppData, theme::Theme}};

use super::Message as MainMessage;

#[derive(Debug, Clone)]
pub enum Message {
    ToggleMarkdown(bool),
    ToggleAutoupdateGroups(bool),
    ToggleMessageFile(bool),
    ToggleAutoSend(bool),
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
            Message::ToggleMarkdown(markdown) => {
                data.markdown = markdown;
            },
            Message::ToggleAutoupdateGroups(state) => {
                data.autoupdate_groups = state;
            },
            Message::ToggleMessageFile(state) => {
                data.message_file = state;
            },
            Message::ToggleAutoSend(state) => {
                data.autosend = state;
            }
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
        container(
            Column::new()
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(Padding::ZERO.horizontal(20).top(20))
        .spacing(20)
        .push(
            text("Налаштування")
            .width(Length::Fill)
            .center()
            .size(24)
        )
        .push(
            scrollable(
                Column::new()
                    .spacing(20)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .align_x(Alignment::Center)
                    .push(
                        container(
                            Column::new()
                            .spacing(20)
                            .width(Length::Fill)
                            .push(
                                Column::new()
                                .push(
                                    text("Адреса серверу для прийому повідомлень")
                                )
                                .push(
                                    text_input("Адреса серверу для прийому повідомлень", &self.recieve_address_edit)
                                    .style(|theme: &iced::Theme, status| {
                                        let mut default = text_input::Style {
                                            border: Border::default().rounded(10).color(theme.extended_palette().secondary.weak.color).width(1),
                                            ..text_input::default(theme, status)
                                        };
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
                                    .style(|theme: &iced::Theme, status| text_input::Style {
                                        border: Border::default().rounded(10).color(theme.extended_palette().secondary.weak.color).width(1),
                                        ..text_input::default(theme, status)
                                    })
                                    .on_input(Message::HistoryLenEdit)
                                ]
                            )
                        )
                        .padding(20)
                        .style(container_style)
                    )
                    .push(
                        container(
                            Column::new()
                            .width(Length::Fill)
                            .spacing(20)
                            .push(
                                checkbox(data.autosend)
                                .label("Автоматична відправка повідомлень")
                                .on_toggle(Message::ToggleAutoSend)
                            )
                            .push(
                                checkbox(data.message_file)
                                .label("Показувати функцію відправки повідомлення з файлу")
                                .on_toggle(Message::ToggleMessageFile)
                            )
                            .push(
                                checkbox(data.autoupdate_groups)
                                .label("Автоматично оновлювати список груп з месенджерів")
                                .on_toggle(Message::ToggleAutoupdateGroups)
                            )
                            .push(
                                checkbox(data.markdown)
                                .label("Використовувати форматування Markdown при надсиланні повідомлень")
                                .on_toggle(Message::ToggleMarkdown)
                            )
                        )
                        .padding(20)
                        .style(container_style)
                    )
                    .push(
                        container(
                            Row::new()
                            .spacing(5)
                            .align_y(Alignment::Center)
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
                                .style(|theme: &iced::Theme, status| {
                                    let palette = theme.extended_palette();
                                    pick_list::Style {
                                        border: Border::default().rounded(10).color(palette.secondary.weak.color).width(1),
                                        background: palette.background.base.color.into(),
                                        ..pick_list::default(theme, status)
                                    }
                                })
                            )
                        )
                        .padding(10)
                        .center_x(Length::Fill)
                        .style(container_style)
                    )
                    .push(
                        button("Завантажити список мереж")
                        .on_press(Message::ChooseNetworkFile)
                        .style(|theme: &iced::Theme, status| {
                            let border = Border::default().rounded(10);
                            button::Style {
                                border,
                                shadow: Shadow { color: Color::BLACK.scale_alpha(0.3), blur_radius: 4.0, offset: Vector::new(0.0, 2.0) },
                                ..button::primary(theme, status)
                            }
                        })
                    )
                )
                .width(Length::Fill)
                .height(Length::Fill)
            )
        )
        .style(|theme: &iced::Theme| container::Style {
            background: Some(theme.extended_palette().background.weaker.color.into()),
            ..Default::default()
        })
        .into()
    }
}

fn container_style(theme: &iced::Theme) -> container::Style {
    container::Style {
        background: Some(theme.extended_palette().background.weakest.color.into()),
        border: Border::default().rounded(20),
        shadow: Shadow { color: Color::BLACK.scale_alpha(0.2), offset: Vector::new(0.0, 2.0), blur_radius: 4.0 },
        ..Default::default()
    }
}
