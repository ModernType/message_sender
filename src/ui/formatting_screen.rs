use iced::{Alignment, Border, Color, Element, Length, Shadow, Task, Vector, border::Radius, widget::{Column, Row, container, text, text_editor}};

use crate::{appdata::AppData, message::{Formatting, TEST_MESSAGE}};

use super::Message as MainMessage;

const HELP: &str = "Спеціальні коди, які можна вписати:

%частота% - Вставляє частоту (незалежно увімкнена вона чи ні)
%текст% - Вставляє текст самого повідомлення
%хто% - Всталяє відправника
%кому% - Вставляє отримувача
%заголовок% - Вставляє заголовок (назву мережі)
%район% - Вставляє привʼязку по місцевості
%дата% - Вставляє дату повідомлення
%час% - Вставляє час повідомлення
%джерело% - Вставляє джерело отримання повідомлення
%коментар% - Вставляє коментар (за наявності)";

pub struct FormattingScreen {
    pub editor: text_editor::Content,
}

#[derive(Debug, Clone)]
pub enum Message {
    TextEditor(text_editor::Action),
}

impl From<Message> for MainMessage {
    fn from(value: Message) -> Self {
        MainMessage::FormattingScrMessage(value)
    }
}

impl FormattingScreen {
    pub fn new(formatting: Option<&Formatting>) -> Self {
        let content = match formatting {
            Some(f) => text_editor::Content::with_text(&f.to_string()),
            None => text_editor::Content::new(),
        };

        Self {
            editor: content,
        }
    }

    pub fn update(&mut self, message: Message, _data: &mut AppData) -> Task<MainMessage> {
        match message {
            Message::TextEditor(action) => {
                self.editor.perform(action);
            },
        }
        Task::none()
    }

    pub fn formatting(&self) -> Formatting {
        Formatting::parse(&self.editor.text())
    }

    pub fn view(&self) -> Element<'_, Message> {
        container(
            Row::new()
            .padding(15)
            .spacing(10)
            .height(Length::Fill)
            .width(Length::Fill)
            .align_y(Alignment::Center)
            .push(
                text_editor(&self.editor)
                .height(Length::Fill)
                .on_action(Message::TextEditor)
            )
            .push(
                container(
                    Column::new()
                    .padding(15)
                    .spacing(40)
                    .push(text(HELP))
                    .push(text(self.formatting().format_message(&TEST_MESSAGE)))
                )
                .style(|theme: &iced::Theme| {
                    let palette = theme.extended_palette();
                    container::Style {
                        text_color: Some(palette.background.base.text),
                        background: Some(palette.background.base.color.into()),
                        border: Border::default().rounded(Radius::new(15)),
                        shadow: Shadow { color: Color::BLACK.scale_alpha(0.15), offset: Vector::ZERO, blur_radius: 10.0 },
                        ..Default::default()
                    }
                })
            )
        )
        .style(|theme: &iced::Theme| {
            let palette = theme.extended_palette();
            container::Style {
                text_color: Some(palette.background.weaker.text),
                background: Some(palette.background.weaker.color.into()),
                ..Default::default()
            }
        })
        .into()
    }
}