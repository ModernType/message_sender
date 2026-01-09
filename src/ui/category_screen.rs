use std::collections::{HashMap, HashSet};

use iced::{Alignment, Border, Font, Length, Pixels};
use iced::{Element, Task};
use iced::widget::{Column, Row, button, checkbox, container, scrollable, space, svg, text, text_input};

use crate::message::SendMode;
use crate::messangers::Key;
use crate::send_categories::SendCategory;
use crate::ui::{AppData, Message as MainMessage, icons};
use crate::ui::ext::PushMaybe;
use crate::ui::main_screen::Group;


pub struct CategoryScreen {
    pub new_category_name: Option<String>,
    pub selected_category: Option<usize>,
    edit_new_name_id: iced::widget::Id,
    network_search: String,
    group_search: String,
}

#[derive(Debug, Clone)]
pub enum Message {
    AddCategory,
    EditNewName(String),
    CategoryToggled(usize),
    CategoryDelete(usize),
    ShowGeneral,
    ToggleGroup(usize, Key, SendMode),
    ToggleNetwork(usize, u64, bool),
    ToggleGeneralGroup(Key, SendMode),
    ToggleUseGeneral(usize, bool),
    NetworkSerch(String),
    GroupSearch(String),
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
            edit_new_name_id: iced::widget::Id::unique(),
            network_search: String::new(),
            group_search: String::new(),
        }
    }

    pub fn update(&mut self, message: Message, data: &mut AppData) -> Task<MainMessage> {
        match message {
            Message::AddCategory => {
                match self.new_category_name.take() {
                    Some(name) if !name.is_empty() => {
                        data.categories.push(SendCategory::new(name));
                        self.selected_category = Some(data.categories.len() - 1);
                    },
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
            Message::CategoryDelete(index) => {
                if self.selected_category.is_some() {
                    self.selected_category = None
                }
                data.categories.remove(index);
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
            Message::ToggleUseGeneral(index, state) => {
                let cat = &mut data.categories[index];
                cat.use_general = state;
            },
            Message::Back => {
                data.categories.iter_mut().for_each(SendCategory::shrink);
                return Task::done(MainMessage::SetScreen(super::Screen::Main));
            },
            Message::NetworkSerch(s) => {
                self.network_search = s
            },
            Message::GroupSearch(s) => {
                self.group_search = s
            },
        }

        Task::none()
    }

    fn category_list<'a>(&'a self, data: &'a AppData) -> Element<'a, Message> {
        let col = Column::new()
        .spacing(5)
        .push(
            Row::new()
            .spacing(3)
            .align_y(Alignment::Center)
            .push(
                button(
                    svg(svg::Handle::from_memory(icons::ARROW_BACK))
                    .width(Length::Shrink)
                )
                .on_press(Message::Back)
                .style(button::secondary)
            )
            .push(
                text("Категорії надсилання")
                .width(Length::Fill)
                .size(20)
                .center()
            )
        )
        .push(
            space()
            .height(15)
        )
        .push(
            button(
                text("Додати")
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
            .height(15)
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
                    button::subtle
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
                        Row::new()
                        .spacing(3)
                        .height(Length::Shrink)
                        .push(
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
                                    button::subtle
                                }
                            )
                            .width(Length::Fill)
                            .on_press(Message::CategoryToggled(index))
                        )
                        .push(
                            button(
                                // svg(svg::Handle::from_memory(icons::DELETE))
                                text("X")
                                .width(Length::Shrink)
                            )
                            .width(Length::Shrink)
                            .style(button::subtle)
                            .on_press(Message::CategoryDelete(index))
                        )
                    )
                }
            )
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }

    fn general_groups<'a>(&'a self, groups: &'a HashMap<Key, Group>) -> Element<'a, Message> {
        scrollable(
            Column::new()
            .spacing(7)
            .padding(10)
            .push(
                text("Групи")
                .center()
                .width(Length::Fill)
            )
            .push(
                text_input("Пошук", &self.group_search)
                .on_input(Message::GroupSearch)
                .width(Length::Fill)
            )
            .push(
                text("Signal").width(Length::Fill).center()
            )
            .push({
                let mut groups = groups.iter()
                .filter_map(|(key, group)| match key {
                    Key::Signal(key) if group.title.contains(&self.group_search) => Some((key, group)),
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
                    Key::Whatsapp(key) if group.title.contains(&self.group_search) => Some((key, group)),
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
        .width(Length::Fill)
        .height(Length::Fill)
        .style(|theme: &iced::Theme, status| scrollable::Style {
            container: container::Style {
                background: Some(theme.extended_palette().background.weakest.color.into()),
                border: Border::default().rounded(20),
                ..Default::default()
            },
            ..scrollable::default(theme, status)
        })
        .into()
    }

    fn general_networks<'a>(&'a self, data: &'a AppData) -> Element<'a, Message> {
        let col = Column::new()
        .spacing(3)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(10)
        .push(
            text("Мережі")
            .center()
            .width(Length::Fill)
        )
        .push(
            text_input("Пошук", &self.network_search)
            .on_input(Message::NetworkSerch)
            .width(Length::Fill)
        );
        
        let mut checks = data.networks.keys().collect::<HashSet<_>>();

        for cat in data.categories.iter() {
            checks.retain(|id| !cat.networks.contains(id));
        }

        let networks = if self.network_search.is_empty() {
            data.networks.values()
            .map(|net| (net.id, &net.name))
            .collect::<Vec<_>>()
        }
        else {
            data.networks.values()
            .filter(|net| net.name.contains(&self.network_search))
            .map(|net| (net.id, &net.name))
            .collect::<Vec<_>>()
        };

        scrollable(
            networks.into_iter().fold(
                col,
                |col, (id, name)|
                col.push(
                    checkbox(checks.contains(&id))
                    .label(name)
                )
            )
        )
        .style(|theme: &iced::Theme, status| scrollable::Style {
            container: container::Style {
                background: Some(theme.extended_palette().background.weakest.color.into()),
                border: Border::default().rounded(20),
                ..Default::default()
            },
            ..scrollable::default(theme, status)
        })
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }

    fn category_groups<'a>(&'a self, index: usize, data: &'a AppData) -> Element<'a, Message> {
        let category = &data.categories[index];
        scrollable(
            Column::new()
            .spacing(7)
            .padding(10)
            .push(
                text("Групи")
                .center()
                .width(Length::Fill)
            )
            .push(
                text_input("Пошук", &self.group_search)
                .on_input(Message::GroupSearch)
                .width(Length::Fill)
            )
            .push(
                text("Signal").width(Length::Fill).center()
            )
            .push({
                let mut groups = data.groups.iter()
                .filter_map(|(key, group)| match key {
                    Key::Signal(key) if group.title.contains(&self.group_search) => Some((key, group)),
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
                    Key::Whatsapp(key) if group.title.contains(&self.group_search) => Some((key, group)),
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
        .style(|theme: &iced::Theme, status| scrollable::Style {
            container: container::Style {
                background: Some(theme.extended_palette().background.weakest.color.into()),
                border: Border::default().rounded(20),
                ..Default::default()
            },
            ..scrollable::default(theme, status)
        })
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }

    fn category_networks<'a>(&'a self, index: usize, data: &'a AppData) -> Element<'a, Message> {
        let mut all_networks = data.networks.iter()
        .filter(|(_, info)| self.network_search.is_empty() || info.name.contains(&self.network_search))
        .collect::<Vec<_>>();
        all_networks.sort_unstable_by(|(_, info1), (_, info2)| info1.name.cmp(&info2.name));
        let category = &data.categories[index];

        scrollable(
            all_networks.into_iter()
            .fold(
                Column::new()
                .spacing(7)
                .padding(10)
                .push(
                    text("Мережі")
                    .center()
                    .width(Length::Fill)
                )
                .push(
                    text_input("Пошук", &self.network_search)
                    .on_input(Message::NetworkSerch)
                    .width(Length::Fill)
                ),
                |col, (id, _)| col.push(
                    checkbox(category.networks.contains(id))
                    .label(&data.networks.get(id).unwrap().name)
                    .on_toggle(move |state| Message::ToggleNetwork(index, *id, state))
                )
            )
        )
        .style(|theme: &iced::Theme, status| scrollable::Style {
            container: container::Style {
                background: Some(theme.extended_palette().background.weakest.color.into()),
                border: Border::default().rounded(20),
                ..Default::default()
            },
            ..scrollable::default(theme, status)
        })
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }

    pub fn view<'a>(&'a self, data: &'a AppData) -> Element<'a, Message> {
        let mut main_row = Row::new()
        .padding(20)
        .spacing(20)
        .width(Length::Fill)
        .height(Length::Fill)
        .push(
            self.category_list(data)
        );

        if let Some(index) = self.selected_category {
            main_row = main_row.push(
                Column::new()
                .width(Length::FillPortion(3))
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
                    .spacing(20)
                    .push(
                        self.category_groups(index, data)
                    )
                    .push(
                        self.category_networks(index, data)
                    )
                )
                .push(
                    container(
                        checkbox(data.categories[index].use_general)
                        .label("Відправляти у загальні групи")
                        .on_toggle(move |state| Message::ToggleUseGeneral(index, state))
                    )
                    .center_x(Length::Fill)
                )
            )
        }
        else {
            main_row = main_row.push(
                Column::new()
                .spacing(20)
                .push(
                    text("Загальна")
                    .size(24)
                    .width(Length::Fill)
                    .center()
                )
                .push(
                    Row::new()
                    .spacing(20)
                    .push(self.general_groups(&data.groups))
                    .push(self.general_networks(data))
                )
                .width(Length::FillPortion(3))
            )
        }

        main_row.into()
    }
}