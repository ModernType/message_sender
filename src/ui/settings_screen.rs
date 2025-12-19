use std::net::SocketAddrV4;

use iced::{Alignment, Element, Length, Task, widget::{Column, button, checkbox, column, scrollable, svg, text, text_input}};

use super::Message as MainMessage;

#[derive(Debug, Clone)]
pub enum Message {
    ToMainScreen,
    ToggleMarkdown(bool),
    ToggleParallel(bool),
    RecieveAddressEditChanged(String),
}

impl From<Message> for MainMessage {
    fn from(value: Message) -> Self {
        Self::SettingsScrMessage(value)
    }
}

#[derive(Debug)]
pub(super) struct SettingsScreen {
    pub markdown: bool,
    pub parallel: bool,
    recieve_address_edit: String,
    pub recieve_address: SocketAddrV4
}

impl SettingsScreen {
    pub fn new(markdown: bool, parallel: bool, recieve_address: SocketAddrV4) -> Self {
        Self {
            markdown,
            parallel,
            recieve_address,
            recieve_address_edit: recieve_address.to_string(),
        }
    }

    pub fn update(&mut self, message: Message) -> Task<MainMessage> {
        match message {
            Message::ToMainScreen => return Task::done(MainMessage::SetScreen(super::Screen::Main)),
            Message::ToggleMarkdown(markdown) => {
                self.markdown = markdown;
            },
            Message::ToggleParallel(parallel) => {
                self.parallel = parallel;
            },
            Message::RecieveAddressEditChanged(recieve_address_edit) => {
                if let Ok(addr) = recieve_address_edit.parse() {
                    self.recieve_address = addr;
                }
                self.recieve_address_edit = recieve_address_edit;
            }
        }

        Task::none()
    }

    pub fn view(&self) -> Element<'_, Message> {
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
                    column![
                        text("Адреса серверу для прийому повідомлень"),
                        text_input("Адреса серверу для прийому повідомлень", &self.recieve_address_edit)
                        .on_input(Message::RecieveAddressEditChanged)
                    ]
                )
                .push(
                    checkbox(self.markdown)
                    .label("Використовувати форматування markdown при відправці повідомлень")
                    .on_toggle(Message::ToggleMarkdown)
                )
                .push(
                    checkbox(self.parallel)
                    .label("Здійснювати відправку повідомлень паралельно (ЕКСПЕРИМЕНТАЛЬНО!!!)")
                    .on_toggle(Message::ToggleParallel)
                )
            )
            .width(Length::Fill)
            .height(Length::Fill)
        )
        .into()
    }
}