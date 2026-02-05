use std::collections::{HashMap, HashSet};

use iced::border::Radius;
use iced::{Alignment, Border, Color, Font, Length, Padding, Pixels, Shadow, Vector};
use iced::{Element, Task};
use iced::widget::{Column, Row, button, checkbox, container, mouse_area, scrollable, space, text, text_input};

use crate::icon;
use crate::message::SendMode;
use crate::messangers::Key;
use crate::send_categories::{NetworkInfo, SendCategory};
use crate::ui::{AppData, Message as MainMessage};
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
    Empty,
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
                self.new_category_name = None;
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
            Message::NetworkSerch(s) => {
                self.network_search = s
            },
            Message::GroupSearch(s) => {
                self.group_search = s
            },
            Message::Empty => {
                self.new_category_name = None;
                self.selected_category = None;
            }
        }

        Task::none()
    }

    fn category_list<'a>(&'a self, data: &'a AppData) -> Element<'a, Message> {
        let col = Column::new()
        .spacing(5)
        .push(
            text("Канали надсилання")
            .width(Length::Fill)
            .size(20)
            .center()
        )
        .push(
            space()
            .height(15)
        )
        .push(
            self.new_category_name.as_ref().map_or_else(
                || -> Element<'_, _> {
                    my_button(
                    "Додати",
                    Some(icon!(add).size(26)),
                    false
                    )
                    .width(Length::Fill)
                    .on_press(Message::AddCategory)
                    .into()
                },
                |name| Row::new()
                .spacing(3)
                .push(
                    text_input("Назва категорії", name)
                    .on_input(Message::EditNewName)
                    .on_submit(Message::AddCategory)
                    .id(self.edit_new_name_id.clone())
                )
                .push(
                    my_button(icon!(add), None::<Element<'_, Message>>, false)
                    .on_press(Message::AddCategory)
                )
                .into()
            )
        )
        .push(
            space()
            .height(15)
        );

        mouse_area(
            scrollable(
                data.categories.iter()
                .enumerate()
                .fold(
                    col,
                    |col, (index, category)| {
                        let selected = if let Some(sel_index) = self.selected_category && sel_index == index {
                            true
                        }
                        else {
                            false
                        };
                        col.push(
                            button(
                                Row::new()
                                .spacing(3)
                                .height(Length::Shrink)
                                .align_y(Alignment::Center)
                                .push(
                                    text(category.name())
                                    .width(Length::Fill)
                                    .center()
                                )
                                .push(
                                    button(
                                        // svg(svg::Handle::from_memory(icons::DELETE))
                                        icon!(delete)
                                        .size(28)
                                    )
                                    .style(|theme: &iced::Theme, status| button::Style {
                                        text_color: match status {
                                            button::Status::Active => theme.extended_palette().danger.base.color.into(),
                                            _ => theme.extended_palette().danger.weak.color.into(),
                                        },
                                        ..button::text(theme, status)
                                    })
                                    .padding(0)
                                    .on_press(Message::CategoryDelete(index))
                                )
                            )
                            .style(
                                button_style(selected)
                            )
                            .on_press(Message::CategoryToggled(index))
                        )
                    }
                )
            )
            .width(Length::Fill)
            .height(Length::Fill)
        )
        .on_press(Message::Empty)
        .into()
    }

    fn general_groups<'a>(&'a self, groups: &'a HashMap<Key, Group>) -> Element<'a, Message> {
        fn sort((k1, g1): &(&Key, &Group), (k2, g2): &(&Key, &Group)) -> std::cmp::Ordering {
            let v = g1.title.cmp(&g2.title);
            if v == std::cmp::Ordering::Equal {
                k1.cmp(k2)
            }
            else {
                v
            }
        }

        macro_rules! vec_to_col {
            ($name:ident, $map:expr) => {
                let $name = $name.into_iter().fold(
                    Column::new()
                    .spacing(5),
                    |col, (key, group)| col.push(($map)(key, group))
                );
            };
        }
        
        let mut added = Vec::new();
        let mut other = Vec::new();

        for (k, g) in groups.iter().filter(|(_, g)| g.title.contains(&self.group_search)) {
            if g.active() {
                added.push((k, g));
            }
            else {
                other.push((k, g));
            }
        }
        added.sort_unstable_by(sort);
        other.sort_unstable_by(sort);

        vec_to_col!(added, |key: &'a Key, group: &'a Group| -> container::Container<'a, Message> {
            container(
                Row::new()
                .width(Length::Fill)
                .spacing(5)
                .padding(5)
                .align_y(Alignment::Center)
                .push(
                    key.icon()
                    .height(22)
                    .width(Length::Shrink)
                )
                .push(
                    text(&group.title)
                    .width(Length::Fill)
                )
                .push(
                    button(
                        icon!(graphic_eq)
                        .size(22)
                    )
                    .padding(Padding::default().horizontal(3))
                    .style(
                        match group.send_mode {
                            SendMode::Frequency => Box::new(button_style(true)),
                            _ => Box::new(button::text) as Box<dyn Fn(&iced::Theme, button::Status) -> button::Style>,
                        }
                    )
                    .on_press(Message::ToggleGeneralGroup(key.clone(), match group.send_mode {
                            SendMode::Frequency => SendMode::Normal,
                            _ => SendMode::Frequency,
                        }))
                )
                .push(
                    button(
                        icon!(remove)
                        .size(22)
                    )
                    .padding(0)
                    .style(button::text)
                    .on_press(Message::ToggleGeneralGroup(key.clone(), SendMode::Off))
                )
            )
            .style(entry_style)
        });

        vec_to_col!(other, |key: &'a Key, group: &'a Group| -> container::Container<'a, Message> {
            container(
                Row::new()
                .width(Length::Fill)
                .padding(5)
                .spacing(5)
                .align_y(Alignment::Center)
                .push(
                    key.icon()
                    .height(22)
                    .width(Length::Shrink)
                )
                .push(
                    text(&group.title)
                    .width(Length::Fill)
                )
                .push(
                    button(
                        icon!(add)
                        .size(22)
                    )
                    .padding(0)
                    .style(button::text)
                    .on_press(Message::ToggleGeneralGroup(key.clone(), SendMode::Normal))
                )
            )
            .style(entry_style)
        });

        container(
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
                .style(text_input_style)
                .on_input(Message::GroupSearch)
                .width(Length::Fill)
            )
            .push(
                space()
                .height(15)
            )
            .push(
                scrollable(
                    Column::new()
                    .padding(Padding::default().horizontal(15))
                    .spacing(25)
                    .align_x(Alignment::Center)
                    .push("Відправляти")
                    .push(added)
                    .push("Не відправляти")
                    .push(other)
                )
            )
            .width(Length::Fill)
            .height(Length::Fill)
        )        
        .style(|theme: &iced::Theme| container::Style {
                background: Some(theme.extended_palette().background.weaker.color.into()),
                border: Border::default().rounded(20),
                shadow: Shadow { color: Color::BLACK.scale_alpha(0.2), blur_radius: 4.0, offset: Vector::new(0.0, 3.0) },
                ..Default::default()
        })
        .into()
    }

    fn category_groups<'a>(&'a self, index: usize, data: &'a AppData) -> Element<'a, Message> {
        fn sort((k1, g1): &(&Key, &Group), (k2, g2): &(&Key, &Group)) -> std::cmp::Ordering {
            let v = g1.title.cmp(&g2.title);
            if v == std::cmp::Ordering::Equal {
                k1.cmp(k2)
            }
            else {
                v
            }
        }

        macro_rules! vec_to_col {
            ($name:ident, $map:expr) => {
                let $name = $name.into_iter().fold(
                    Column::new()
                    .spacing(5),
                    |col, (key, group)| col.push(($map)(key, group))
                );
            };
        }
        
        let mut added = Vec::new();
        let mut other = Vec::new();
        let category = &data.categories[index];

        for (k, g) in data.groups.iter().filter(|(_, g)| g.title.contains(&self.group_search)) {
            if let Some(mode) = category.groups.get(k) && mode.active() {
                added.push((k, g));
            }
            else {
                other.push((k, g));
            }
        }
        added.sort_unstable_by(sort);
        other.sort_unstable_by(sort);

        vec_to_col!(added, |key: &'a Key, group: &'a Group| -> container::Container<'a, Message> {
            let mode = category.groups.get(key).map_or_else(Default::default, Clone::clone);
            container(
                Row::new()
                .width(Length::Fill)
                .spacing(5)
                .padding(5)
                .align_y(Alignment::Center)
                .push(
                    key.icon()
                    .height(22)
                    .width(Length::Shrink)
                )
                .push(
                    text(&group.title)
                    .width(Length::Fill)
                )
                .push(
                    button(
                        icon!(graphic_eq)
                        .size(22)
                    )
                    .padding(Padding::default().horizontal(3))
                    .style(
                        match mode {
                            SendMode::Frequency => Box::new(button_style(true)),
                            _ => Box::new(button::text) as Box<dyn Fn(&iced::Theme, button::Status) -> button::Style>,
                        }
                    )
                    .on_press(Message::ToggleGroup(index, key.clone(), match mode {
                            SendMode::Frequency => SendMode::Normal,
                            _ => SendMode::Frequency,
                        }))
                )
                .push(
                    button(
                        icon!(remove)
                        .size(22)
                    )
                    .padding(0)
                    .style(button::text)
                    .on_press(Message::ToggleGroup(index, key.clone(), SendMode::Off))
                )
            )
            .style(entry_style)
        });

        vec_to_col!(other, |key: &'a Key, group: &'a Group| -> container::Container<'a, Message> {
            container(
                Row::new()
                .width(Length::Fill)
                .padding(5)
                .spacing(5)
                .align_y(Alignment::Center)
                .push(
                    key.icon()
                    .height(22)
                    .width(Length::Shrink)
                )
                .push(
                    text(&group.title)
                    .width(Length::Fill)
                )
                .push(
                    button(
                        icon!(add)
                        .size(22)
                    )
                    .padding(0)
                    .style(button::text)
                    .on_press(Message::ToggleGroup(index, key.clone(), SendMode::Normal))
                )
            )
            .style(entry_style)
        });

        container(
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
                .style(text_input_style)
                .on_input(Message::GroupSearch)
                .width(Length::Fill)
            )
            .push(
                space()
                .height(15)
            )
            .push(
                scrollable(
                    Column::new()
                    .padding(Padding::default().horizontal(15))
                    .spacing(25)
                    .align_x(Alignment::Center)
                    .push("Відправляти")
                    .push(added)
                    .push("Не відправляти")
                    .push(other)
                )
            )
            .width(Length::Fill)
            .height(Length::Fill)
        )        
        .style(|theme: &iced::Theme| container::Style {
                background: Some(theme.extended_palette().background.weaker.color.into()),
                border: Border::default().rounded(20),
                shadow: Shadow { color: Color::BLACK.scale_alpha(0.2), blur_radius: 4.0, offset: Vector::new(0.0, 3.0) },
                ..Default::default()
        })
        .into()
    }

    fn category_networks<'a>(&'a self, index: usize, data: &'a AppData) -> Element<'a, Message> {
        let mut active = Vec::new();
        let mut other = Vec::new();

        macro_rules! vec_to_col {
            ($name:ident, $map:expr) => {
                let $name = $name.into_iter().fold(
                    Column::new()
                    .spacing(5),
                    |col, (key, group)| col.push(($map)(key, group))
                );
            };
        }

        let category = &data.categories[index];
        for (id, network) in data.networks.iter() {
            if category.networks.contains(id) {
                active.push((id, network))
            }
            else {
                other.push((id, network))
            }
        }

        vec_to_col!(active, |id: &'a u64, network: &'a NetworkInfo| -> container::Container<'a, _> {
            container(
                Row::new()
                .width(Length::Fill)
                .padding(5)
                .spacing(5)
                .align_y(Alignment::Center)
                .push(
                    text(&network.name)
                    .width(Length::Fill)
                )
                .push(
                    button(
                        icon!(remove)
                        .size(22)
                    )
                    .padding(0)
                    .style(button::text)
                    .on_press(Message::ToggleNetwork(index, *id, false))
                )
            )
            .style(entry_style)
        });

        vec_to_col!(other, |id: &'a u64, network: &'a NetworkInfo| -> container::Container<'a, _> {
            container(
                Row::new()
                .width(Length::Fill)
                .padding(5)
                .spacing(5)
                .align_y(Alignment::Center)
                .push(
                    text(&network.name)
                    .width(Length::Fill)
                )
                .push(
                    button(
                        icon!(add)
                        .size(22)
                    )
                    .padding(0)
                    .style(button::text)
                    .on_press(Message::ToggleNetwork(index, *id, true))
                )
            )
            .style(entry_style)
        });

        container(
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
                .style(text_input_style)
                .on_input(Message::NetworkSerch)
                .width(Length::Fill)
            )
            .push(
                scrollable(
                    Column::new()
                    .padding(Padding::default().horizontal(15))
                    .spacing(15)
                    .align_x(Alignment::Center)
                    .push("Відправляти")
                    .push(active)
                    .push("Не відправляти")
                    .push(other)
                )
            )
            .width(Length::Fill)
            .height(Length::Fill)
        )
        .style(|theme: &iced::Theme| container::Style {
                background: Some(theme.extended_palette().background.weaker.color.into()),
                border: Border::default().rounded(20),
                shadow: Shadow { color: Color::BLACK.scale_alpha(0.2), blur_radius: 4.0, offset: Vector::new(0.0, 3.0) },
                ..Default::default()
        })
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
                    text("Загальні")
                    .size(24)
                    .width(Length::Fill)
                    .center()
                )
                .push(
                    self.general_groups(&data.groups)
                )
                .width(Length::FillPortion(3))
            )
        }

        main_row.into()
    }
}


fn button_style(selected: bool) -> impl Fn(&iced::Theme, button::Status) -> button::Style {
    move |theme, status| {
        let palette = theme.extended_palette();
        let border = Border::default().rounded(10);
        if selected {
            button::Style {
                border,
                // shadow: Shadow { color: Color::BLACK.scale_alpha(0.3), blur_radius: 2.0, offset: Vector::new(0.0, 2.0) },
                background: Some(palette.secondary.weak.color.into()),
                text_color: palette.secondary.weak.text,
                ..Default::default()
            }
        }
        else {
            button::Style {
                border,
                shadow: Shadow { color: Color::BLACK.scale_alpha(0.3), blur_radius: 2.0, offset: Vector::new(0.0, 2.0) },
                ..button::subtle(theme, status)
            }
        }
    }
}

fn my_button<'a, Message: 'a>(content: impl Into<Element<'a, Message>>, icon: Option<impl Into<Element<'a, Message>>>, selected: bool) -> button::Button<'a, Message> {
    let content = if let Some(icon) = icon {
        Row::new()
        .align_y(Alignment::Center)
        .spacing(5)
        .push(icon)
        .push(content)
        .into()
    }
    else {
        content.into()
    };
    button(content)
    .style(button_style(selected))
}

fn entry_style(theme: &iced::Theme) -> container::Style {
    let palette = theme.extended_palette();
    container::Style {
        background: Some(palette.background.base.color.into()),
        border: Border::default().rounded(10),
        shadow: Shadow { color: Color::BLACK.scale_alpha(0.3), blur_radius: 2.0, offset: Vector::new(0.0, 2.0) },
        ..Default::default()
    }
}

fn text_input_style(theme: &iced::Theme, status: text_input::Status) -> text_input::Style {
    text_input::Style {
        border: Border {
            color: theme.extended_palette().secondary.weak.color,
            width: 1.0,
            radius: Radius::new(5),
        },
        ..text_input::default(theme, status)
    }
}
