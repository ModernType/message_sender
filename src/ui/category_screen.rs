use std::collections::HashMap;

use iced::{Element, Task};
use iced::widget::Row;

use crate::messangers::Key;
use crate::send_categories::SendCategory;
use crate::ui::Message as MainMessage;
use crate::ui::main_screen::Group;


pub struct CategoryScreen {
    pub categories: Vec<SendCategory>,
    pub new_category_name: Option<String>,
}

pub enum Message {
    AddCategory,
    EditNewName(String),

}

impl From<Message> for MainMessage {
    fn from(value: Message) -> Self {
        Self::CategoriesScrMessage(value)
    }
}

impl CategoryScreen {
    pub fn new(categories: Vec<SendCategory>) -> Self {
        Self {
            categories,
            new_category_name: None,
        }
    }

    pub fn update(&mut self, message: Message) -> Task<MainMessage> {


        Task::none()
    }

    pub fn view(&self) -> Element<'_, Message> {
        Row::new()
        .padding(10)
        .into()
    }
}