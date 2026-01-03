use std::collections::HashMap;

use iced::{Alignment, Font, Length, Pixels};
use iced::{Element, Task};
use iced::widget::{Column, Row, button, checkbox, scrollable, space, text, text_input};

use crate::message::SendMode;
use crate::messangers::Key;
use crate::send_categories::{NetworksPool, SendCategory};
use crate::ui::Message as MainMessage;
use crate::ui::ext::PushMaybe;
use crate::ui::main_screen::Group;


pub struct CategoryScreen {
    pub networks: NetworksPool,
    pub categories: Vec<SendCategory>,
    pub new_category_name: Option<String>,
    pub selected_category: Option<usize>,
}

#[derive(Debug, Clone)]
pub enum Message {
    AddCategory,
    EditNewName(String),
    CategoryToggled(usize),
    ShowGeneral,
    ToggleGroup(usize, Key, SendMode),
    ToggleNetwork(usize, String, bool),
    ToggleGeneralGroup(Key, SendMode),
    Back
}

impl From<Message> for MainMessage {
    fn from(value: Message) -> Self {
        Self::CategoriesScrMessage(value)
    }
}

impl CategoryScreen {
    pub fn new(categories: Vec<SendCategory>, networks: NetworksPool) -> Self {
        Self {
            networks,
            categories,
            new_category_name: None,
            selected_category: None,
        }
    }

    pub fn update(&mut self, message: Message, general_groups: &mut HashMap<Key, Group>) -> Task<MainMessage> {
        match message {
            Message::AddCategory => {
                match self.new_category_name.take() {
                    Some(name) if !name.is_empty() => self.categories.push(SendCategory::new(name)),
                    None => self.new_category_name = Some(String::new()),
                    _ => {}
                }
            },
            Message::EditNewName(new_name) => {
                if let Some(name) = self.new_category_name.as_mut() {
                    *name = new_name;
                }
            },
            Message::CategoryToggled(index) => {
                if let Some(cur_index) = self.selected_category && cur_index == index {
                    self.selected_category = None
                }
                else {
                    self.selected_category = Some(index)
                }
            },
            Message::ShowGeneral => {
                self.selected_category = None;
            },
            Message::ToggleGeneralGroup(key, send_mode) => {
                general_groups.get_mut(&key).unwrap().send_mode = send_mode;
            },
            Message::ToggleGroup(index, key, send_mode) => {
                let category = &mut self.categories[index];
                category.groups.insert(key, send_mode);
            },
            Message::ToggleNetwork(index, network, state) => {
                let category = &mut self.categories[index];
                category.networks.insert(network, state);
            },
            Message::Back => {
                return Task::done(MainMessage::SetScreen(super::Screen::Main));
            }
        }

        Task::none()
    }

    fn category_list(&self) -> Element<'_, Message> {
        let col = Column::new()
        .width(Length::Fill)
        .spacing(5)
        .push(
            button(
                text("<- Категорії відправки")
                .center()
                .size(20)
            )
            .style(button::text)
            .on_press(Message::Back)
        )
        .push(
            button(
                text("+")
                .width(Length::Fill)
                .center()
            )
            .on_press(Message::AddCategory)
        )
        .push_maybe(
            self.new_category_name.as_ref()
            .map(
                |name| text_input("Назва категорії", name)
                .on_input(Message::EditNewName)
                .on_submit(Message::AddCategory)
            )
        )
        .push(
            space()
            .height(25)
        )
        .push(
            button(
                text("Загальна")
                .width(Length::Fill)
                .center()
            )
            .on_press(Message::ShowGeneral)
            .style(
                if self.selected_category.is_none() {
                    button::primary
                }
                else {
                    button::secondary
                }
            )
        );

        scrollable(
            self.categories.iter()
            .enumerate()
            .fold(
                col,
                |col, (index, category)| {
                    col.push(
                        button(
                            text(category.name())
                            .center()
                        )
                        .style(
                            if let Some(sel_index) = self.selected_category && sel_index == index {
                                button::primary
                            }
                            else {
                                button::secondary
                            }
                        )
                        .width(Length::Fill)
                        .on_press(Message::CategoryToggled(index))
                    )
                }
            )
        )
        .width(Length::FillPortion(1))
        .height(Length::Fill)
        .into()
    }

    fn general_groups<'a>(&'a self, groups: &'a HashMap<Key, Group>) -> Element<'a, Message> {
        Column::new()
        .push(
            text("Загальна")
            .width(Length::Fill)
            .size(24)
            .center()
        )
        .push(
            scrollable(
                Column::new()
                .spacing(7)
                .push(
                    text("Signal").width(Length::Fill).center()
                )
                .push({
                    let mut groups = groups.iter()
                    .filter_map(|(key, group)| match key {
                        Key::Signal(key) => Some((key, group)),
                        _ => None
                    })
                    .collect::<Vec<_>>();
                    groups.sort_unstable_by(|(_, prev), (_, next)| prev.title.cmp(&next.title));
                    groups.into_iter().fold(Column::new().spacing(3), |col, (key, group)| col.push(
                        checkbox(group.active())
                        .label(&group.title)
                        .on_toggle(move |_| Message::ToggleGeneralGroup(Key::Signal(*key), group.send_mode.next()))
                        .icon(checkbox::Icon {
                            font: Font::with_name("Material Icons"),
                            code_point: if let SendMode::Frequency = group.send_mode { '\u{e1b8}' }
                                        else { '\u{e5ca}' },
                            size: Some(Pixels::from(14)),
                            line_height: text::LineHeight::default(),
                            shaping: text::Shaping::Basic,
                        })
                    ))
                })
                .push(
                        text("Whatsapp").width(Length::Fill).center()
                )
                .push({
                    let mut groups = groups.iter()
                    .filter_map(|(key, group)| match key {
                        Key::Whatsapp(key) => Some((key, group)),
                        _ => None
                    })
                    .collect::<Vec<_>>();
                    groups.sort_unstable_by(|(_, prev), (_, next)| prev.title.cmp(&next.title));
                    groups.into_iter().fold(Column::new().spacing(3), |col, (key, group)| col.push(
                        checkbox(group.active())
                        .label(&group.title)
                        .on_toggle(move |_| Message::ToggleGeneralGroup(Key::Whatsapp(key.clone()), group.send_mode.next()))
                        .icon(checkbox::Icon {
                            font: Font::with_name("Material Icons"),
                            code_point: if let SendMode::Frequency = group.send_mode { '\u{e1b8}' }
                                        else { '\u{e5ca}' },
                            size: Some(Pixels::from(14)),
                            line_height: text::LineHeight::default(),
                            shaping: text::Shaping::Basic,
                        })
                    ))
                })
            )
            .width(Length::FillPortion(4))
            .height(Length::Fill)
        )
        .into()
    }

    fn category_groups<'a>(&'a self, index: usize, groups: &'a HashMap<Key, Group>) -> Element<'a, Message> {
        let category = &self.categories[index];
        scrollable(
            Column::new()
            .spacing(7)
            .align_x(Alignment::Center)
            .push(
                text("Signal").width(Length::Fill).center()
            )
            .push({
                let mut groups = groups.iter()
                .filter_map(|(key, group)| match key {
                    Key::Signal(key) => Some((key, group)),
                    _ => None
                })
                .collect::<Vec<_>>();
                groups.sort_unstable_by(|(_, prev), (_, next)| prev.title.cmp(&next.title));
                groups.into_iter().fold(Column::new().spacing(3), |col, (key, group)| col.push(
                    {
                        let send_mode = category.groups.get(&Key::Signal(*key)).cloned().unwrap_or_default();
                        checkbox(send_mode.active())
                        .label(&group.title)
                        .on_toggle(move |_| Message::ToggleGroup(index, Key::Signal(*key), send_mode.next()))
                        .icon(checkbox::Icon {
                            font: Font::with_name("Material Icons"),
                            code_point: if let SendMode::Frequency = send_mode { '\u{e1b8}' }
                                        else { '\u{e5ca}' },
                            size: Some(Pixels::from(14)),
                            line_height: text::LineHeight::default(),
                            shaping: text::Shaping::Basic,
                        })
                    }
                ))
            })
            .push(
                    text("Whatsapp").width(Length::Fill).center()
            )
            .push({
                let mut groups = groups.iter()
                .filter_map(|(key, group)| match key {
                    Key::Whatsapp(key) => Some((key, group)),
                    _ => None
                })
                .collect::<Vec<_>>();
                groups.sort_unstable_by(|(_, prev), (_, next)| prev.title.cmp(&next.title));
                groups.into_iter().fold(Column::new().spacing(3), |col, (key, group)| col.push(
                    {
                        let send_mode = category.groups.get(&Key::Whatsapp(key.clone())).cloned().unwrap_or_default();
                        checkbox(send_mode.active())
                        .label(&group.title)
                        .on_toggle(move |_| Message::ToggleGroup(index, Key::Whatsapp(key.clone()), send_mode.next()))
                        .icon(checkbox::Icon {
                            font: Font::with_name("Material Icons"),
                            code_point: if let SendMode::Frequency = send_mode { '\u{e1b8}' }
                                        else { '\u{e5ca}' },
                            size: Some(Pixels::from(14)),
                            line_height: text::LineHeight::default(),
                            shaping: text::Shaping::Basic,
                        })
                    }
                ))
            })
        )
        .width(Length::FillPortion(2))
        .height(Length::Fill)
        .into()
    }

    fn category_networks(&self, index: usize) -> Element<'_, Message> {
        let mut all_networks = self.networks.keys().collect::<Vec<_>>();
        all_networks.sort_unstable();
        let category = &self.categories[index];

        scrollable(
            all_networks.into_iter()
            .fold(
                Column::new()
                .spacing(7),
                |col, name| col.push(
                    checkbox(*category.networks.get(name).unwrap())
                    .label(name)
                    .on_toggle(move |state| Message::ToggleNetwork(index, name.clone(), state))
                )
            )
        )
        .width(Length::FillPortion(2))
        .height(Length::Fill)
        .into()
    }

    pub fn view<'a>(&'a self, groups: &'a HashMap<Key, Group>) -> Element<'a, Message> {
        let mut main_row = Row::new()
        .padding(10)
        .spacing(10)
        .push(
            self.category_list()
        );

        if let Some(index) = self.selected_category {
            main_row = main_row.push(
                self.category_groups(index, groups)
            )
            .push(
                self.category_networks(index)
            )
        }
        else {
            main_row = main_row.push(
                self.general_groups(groups)
            )
        }

        main_row.into()
    }
}