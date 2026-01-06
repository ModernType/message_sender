use std::collections::HashMap;

use iced::{Alignment, Color, Font, Length, Pixels};
use iced::{Element, Task};
use iced::widget::{Column, Row, button, checkbox, scrollable, space, text, text_input};

use crate::message::SendMode;
use crate::messangers::Key;
use crate::send_categories::{NetworksPool, SendCategory};
use crate::ui::{AppData, Message as MainMessage};
use crate::ui::ext::PushMaybe;
use crate::ui::main_screen::Group;


pub struct CategoryScreen {
    pub new_category_name: Option<String>,
    pub selected_category: Option<usize>,
    edit_new_name_id: iced::widget::Id,
}

#[derive(Debug, Clone)]
pub enum Message {
    AddCategory,
    EditNewName(String),
    CategoryToggled(usize),
    ShowGeneral,
    ToggleGroup(usize, Key, SendMode),
    ToggleNetwork(usize, u64, bool),
    ToggleGeneralGroup(Key, SendMode),
    Back
}

impl From<Message> for MainMessage {
    fn from(value: Message) -> Self {
        Self::CategoriesScrMessage(value)
    }
}

impl CategoryScreen {
    pub fn new() -> Self {
        Self {
            new_category_name: None,
            selected_category: None,
            edit_new_name_id: iced::widget::Id::unique()
        }
    }

    pub fn update(&mut self, message: Message, data: &mut AppData) -> Task<MainMessage> {
        match message {
            Message::AddCategory => {
                match self.new_category_name.take() {
                    Some(name) if !name.is_empty() => data.categories.push(SendCategory::new(name)),
                    None => {
                        self.new_category_name = Some(String::new());
                        return iced::widget::operation::focus(self.edit_new_name_id.clone())
                    },
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
                data.groups.get_mut(&key).unwrap().send_mode = send_mode;
            },
            Message::ToggleGroup(index, key, send_mode) => {
                let category = &mut data.categories[index];
                category.groups.insert(key, send_mode);
            },
            Message::ToggleNetwork(index, network, state) => {
                let category = &mut data.categories[index];
                if state {
                    category.networks.push(network);
                }
                else {
                    category.networks.retain(|v| *v != network);
                }
            },
            Message::Back => {
                data.categories.iter_mut().for_each(SendCategory::shrink);
                return Task::done(MainMessage::SetScreen(super::Screen::Main));
            }
        }

        Task::none()
    }

    fn category_list<'a>(&'a self, data: &'a AppData) -> Element<'a, Message> {
        let col = Column::new()
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
                .id(self.edit_new_name_id.clone())
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
            data.categories.iter()
            .enumerate()
            .fold(
                col,
                |col, (index, category)| {
                    col.push(
                        button(
                            text(category.name())
                            .width(Length::Fill)
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
        .width(Length::Fill)
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
        )
        .width(Length::FillPortion(4))
        .height(Length::Fill)
        .into()
    }

    fn category_groups<'a>(&'a self, index: usize, data: &'a AppData) -> Element<'a, Message> {
        let category = &data.categories[index];
        scrollable(
            Column::new()
            .spacing(7)
            .push(
                text("Signal").width(Length::Fill).center()
            )
            .push({
                let mut groups = data.groups.iter()
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
                let mut groups = data.groups.iter()
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
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }

    fn category_networks<'a>(&'a self, index: usize, data: &'a AppData) -> Element<'a, Message> {
        let mut all_networks = data.networks.keys().collect::<Vec<_>>();
        all_networks.sort_unstable();
        let category = &data.categories[index];

        scrollable(
            all_networks.into_iter()
            .fold(
                Column::new()
                .spacing(7),
                |col, id| col.push(
                    checkbox(category.networks.contains(id))
                    .label(&data.networks.get(id).unwrap().name)
                    .on_toggle(move |state| Message::ToggleNetwork(index, *id, state))
                )
            )
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }

    pub fn view<'a>(&'a self, data: &'a AppData) -> Element<'a, Message> {
        let mut main_row = Row::new()
        .padding(10)
        .spacing(10)
        .width(Length::Fill)
        .height(Length::Fill)
        .push(
            self.category_list(data)
        );

        if let Some(index) = self.selected_category {
            main_row = main_row.push(
                Column::new()
                .width(Length::FillPortion(4))
                .spacing(20)
                .push(
                    text(data.categories[index].name())
                    .width(Length::Fill)
                    .size(24)
                    .center()
                )
                .push(
                    Row::new()
                    .width(Length::Fill)
                    .push(
                        self.category_groups(index, data)
                    )
                    .push(
                        self.category_networks(index, data)
                    )
                )
            )
        }
        else {
            main_row = main_row.push(
                self.general_groups(&data.groups)
            )
        }

        main_row.into()
    }
}