use std::{
    collections::HashMap, fmt::Debug, fs::{File, OpenOptions}, io::Write, net::SocketAddrV4, sync::Arc, time::{Duration, Instant}
};
use futures::{SinkExt, Stream, StreamExt, channel::{mpsc::UnboundedSender}};
use iced::{Alignment, Animation, Border, Element, Length, Padding, Subscription, Task, animation::Easing, keyboard, widget::{Stack, container, text}};
use presage::{Manager, manager::Registered};
use presage_store_sqlite::SqliteStore;
use ron::ser::PrettyConfig;
use serde::{Serialize, Deserialize};
use crate::{message_server::{self, AcceptedMessage}, messangers::{Key, whatsapp}, send_categories::{NetworkInfo, NetworksPool, SendCategory}, ui::{category_screen::CategoryScreen, main_screen::LinkState, theme::Theme}};

use crate::{messangers::signal::{SignalMessage, SignalWorker}, ui::{ext::ColorExt, main_screen::MainScreen, message_history::SendMessageInfo, settings_screen::SettingsScreen}};

pub mod main_screen;
pub mod settings_screen;
pub mod message_history;
pub mod category_screen;
mod icons;
mod ext;
mod theme;

const NOTIFICATION_SHOW_TIME: u64 = 3000;

#[derive(Debug, Clone, Copy)]
pub enum Screen {
    Main,
    Settings,
    Categories
}

pub enum Message {
    MainScrMessage(main_screen::Message),
    SettingsScrMessage(settings_screen::Message),
    CategoriesScrMessage(category_screen::Message),
    SignalMessage(SignalMessage),
    SetManager(Manager<SqliteStore, Registered>),
    SetWhatsappClient(Option<Arc<whatsapp_rust::Client>>),
    SetupSignalWorker(UnboundedSender<Message>),
    SendMessage(Arc<SendMessageInfo>),
    DeleteMessage(Arc<SendMessageInfo>),
    EditMessage(Arc<SendMessageInfo>, Vec<u64>, Vec<String>),
    SetScreen(Screen),
    AcceptMessage(Vec<AcceptedMessage>),
    ThemeChange(Theme),
    OnClose,
    UpdateGroupList,
    Synced,
    Notification(String),
    NotificationClose,
    RecivedNetworks(HashMap<u64, NetworkInfo>),
    Keyboard(keyboard::Event),
    None,
}

#[macro_export]
macro_rules! notification {
    ($s:expr) => {
        $crate::ui::Message::Notification($s.to_string())
    };
    ($s:literal $(, $v:expr)* $(,)?) => {
        $crate::ui::Message::Notification(format!($s $(, $v)*))
    };
}

impl Debug for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ui::Message").finish_non_exhaustive()
    }
}

impl From<SignalMessage> for Message {
    fn from(value: SignalMessage) -> Self {
        Self::SignalMessage(value)
    }
}

pub struct App {
    data: AppData,
    cur_screen: Screen,
    manager: Option<Manager<SqliteStore, Registered>>,
    whatsapp_client: Option<Arc<whatsapp_rust::Client>>,
    main_scr: MainScreen,
    sett_scr: SettingsScreen,
    category_scr: CategoryScreen,
    signal_task_send: Option<UnboundedSender<SignalMessage>>,
    now: Instant,
    notification: Notification,
    signal_logged: bool,
    whatsapp_logged: bool,
}

impl<M: Into<Message>> From<anyhow::Result<M>> for Message {
    fn from(value: anyhow::Result<M>) -> Self {
        match value {
            Ok(m) => m.into(),
            Err(e) => Self::Notification(e.to_string())
        }
    }
}

impl App {
    pub fn new() -> (Self, Task<Message>) {
        let data = AppData::new();
        let start_task = if data.theme.is_system() {
            iced::system::theme().map(|mode| Message::ThemeChange(mode.into()))
        }
        else {
            Task::none()
        };

        (
            Self {
                main_scr: MainScreen::new(),
                sett_scr: SettingsScreen::new(&data),
                category_scr: CategoryScreen::new(),
                signal_logged: data.signal_logged,
                whatsapp_logged: data.whatsapp_logged,
                data,
                manager: None,
                whatsapp_client: None,
                cur_screen: Screen::Main,
                signal_task_send: None,
                now: Instant::now(),
                notification: Notification::new(),
            },
            start_task
        )
    }

    fn save(&self) -> anyhow::Result<()> {
        self.data.save()
    }

    #[inline]
    fn whatsapp_registered(&self) -> bool {
        self.main_scr.whatsapp_state == LinkState::Linked
    }

    pub fn update(&mut self, message: Message, now: Instant) -> Task<Message> {
        self.now = now;

        match message {
            Message::MainScrMessage(m) => self.main_scr.update(m, now, &mut self.data),
            Message::SettingsScrMessage(m) => self.sett_scr.update(m, &mut self.data),
            Message::CategoriesScrMessage(m) => self.category_scr.update(m, &mut self.data),
            Message::SignalMessage(m) => {
                if let Some(channel) = self.signal_task_send.as_ref() {
                    let mut channel = channel.clone();
                    Task::perform(
                        async move { channel.send(m).await },
                        |_| Message::None
                    )
                }
                else {
                    Task::none()
                }
            },
            Message::SetScreen(screen) => {
                self.cur_screen = screen;
                if let Screen::Main = screen {
                    if let Err(e) = self.save() {
                        log::error!("Error saving data: {e}");
                        Task::done(Message::Notification(format!("Error saving data: {e}")))
                    }
                    else {
                        Task::none()
                    }
                }
                else {
                    Task::none()
                }
            },
            Message::Keyboard(event) => {
                if let keyboard::Event::KeyPressed{ key, modifiers, ..} = event {
                    #[allow(clippy::single_match, clippy::collapsible_match)]
                    match key {
                        keyboard::Key::Named(named) => {
                            match named {
                                keyboard::key::Named::Escape => match self.cur_screen {
                                    Screen::Categories | Screen::Settings => return Task::done(Message::SetScreen(Screen::Main)),
                                    _ => ()
                                }
                                keyboard::key::Named::Enter if modifiers.command() => {
                                    if self.main_scr.edit.is_some() {
                                        return self.main_scr.update(main_screen::Message::ConfirmEdit, now, &mut self.data)
                                    }
                                    else {
                                        return self.main_scr.update(main_screen::Message::SendMessagePressed, now, &mut self.data)
                                    }
                                },
                                _ => ()
                            }
                        },
                        _ => ()
                    }
                }

                Task::none()
            },
            Message::SetManager(mng) => {
                self.manager = Some(mng.clone());
                self.signal_logged = true;
                Task::batch([
                    Task::done(main_screen::Message::SetSignalState(main_screen::LinkState::Linked).into()),
                    Task::done(SignalMessage::Sync(mng).into())   
                ])
            },
            Message::SetWhatsappClient(maybe_client) => {
                match maybe_client {
                    Some(client) => {
                        self.whatsapp_client = Some(client);
                        self.whatsapp_logged = true;
                        Task::done(main_screen::Message::SetWhatsappState(LinkState::Linked).into())
                    },
                    None if !self.whatsapp_registered() => {
                        Task::done(main_screen::Message::SetWhatsappState(LinkState::Unlinked).into())
                    },
                    _ => {
                        Task::none()
                    }
                }
            },
            Message::Synced => {
                Task::done(main_screen::Message::UpdateGroups.into())
            }
            Message::SetupSignalWorker(tx) => {
                let (task_tx, task_rx) = futures::channel::mpsc::unbounded();
                SignalWorker::spawn_new(task_rx, tx.clone(), task_tx.clone());
                self.signal_task_send = Some(task_tx);
                whatsapp::UI_MESSAGE_SENDER.set(tx.clone()).unwrap();

                Task::batch([
                    if self.signal_logged { Task::done(SignalMessage::LinkBegin.into()) } else { Task::none() },
                    if self.whatsapp_logged { Task::perform(whatsapp::start_whatsapp_task(), |_| Message::None) } else { Task::none() },
                    Task::perform(message_server::start_server(self.data.recieve_address, tx), |_| Message::Notification("Message server stopped working".to_owned()))
                ])
            },
            Message::UpdateGroupList => {
                let mut task_list = Vec::with_capacity(2);
                if let Some(manager) = self.manager.as_ref() {
                    task_list.push(
                        Task::perform(crate::messangers::signal::get_groups(manager.clone()), |v| v.map(main_screen::Message::SetGroups).into())
                    );
                }
                if let Some(client) = self.whatsapp_client.as_ref() {
                    task_list.push(
                        Task::perform(whatsapp::get_groups(client.clone()), |v| v.map(main_screen::Message::SetGroups).into())
                    );
                }
                Task::batch(task_list)
            },
            Message::SendMessage(message) => {
                let mut task_list = Vec::with_capacity(2);
                if let Some(manager) = self.manager.as_ref()  {
                    task_list.push(
                        Task::done(SignalMessage::SendMessage(manager.clone(), message.clone(), self.data.markdown, self.data.parallel).into())
                    );
                }
                else if !message.groups_signal.is_empty() {
                    message.set_status(message_history::SendStatus::Deleted, std::sync::atomic::Ordering::Relaxed);
                    return Task::done(notification!("Прив'яжіть, будь ласка, Modern Sender до Signal"));
                }
                if let Some(client) = self.whatsapp_client.as_ref() {
                    task_list.push(
                        Task::perform(whatsapp::send_message(client.clone(), message, self.data.markdown), |_| Message::None)
                    );
                }
                else if !message.groups_whatsapp.is_empty() {
                    message.set_status(message_history::SendStatus::Deleted, std::sync::atomic::Ordering::Relaxed);
                    return Task::done(notification!("Прив'яжіть, будь ласка, Modern Sender до Whatsapp"));
                }
                Task::batch(task_list)
            },
            Message::DeleteMessage(message) => {
                message.set_status(message_history::SendStatus::Pending, std::sync::atomic::Ordering::Relaxed);
                let mut task_list = Vec::with_capacity(2);
                if let Some(manager) = self.manager.as_ref()  {
                    task_list.push(
                        Task::done(SignalMessage::DeleteMessage(manager.clone(), message.clone()).into())
                    );
                }
                if let Some(client) = self.whatsapp_client.as_ref() {
                    task_list.push(
                        Task::perform(whatsapp::delete_message(client.clone(), message), |_| Message::None)
                    );
                }
                Task::batch(task_list)
            },
            Message::EditMessage(message, timestamps, whatsapp_ids) => {
                let mut task_list = Vec::with_capacity(2);
                if let Some(manager) = self.manager.as_ref()  {
                    task_list.push(
                        Task::done(SignalMessage::EditMessage(manager.clone(), message.clone(), timestamps, self.data.markdown).into())
                    );
                }
                if let Some(client) = self.whatsapp_client.as_ref() {
                    task_list.push(
                        Task::perform(whatsapp::edit_message(client.clone(), message, whatsapp_ids, self.data.markdown), |_| Message::None)
                    );
                }
                Task::batch(task_list)
            },
            Message::AcceptMessage(messages) => {
                let autosend = messages.iter().fold(self.data.autosend, |autosend, msg| autosend && !msg.autosend_overwrite);

                if autosend {
                    Task::batch(
                        messages
                        .into_iter()
                        .map(|msg| Task::done(main_screen::Message::SendMessage(msg.text, msg.freq, msg.network).into()))
                    )
                }
                else {
                    self.main_scr.message_queue.extend(messages);
                    if self.main_scr.cur_message.is_none() {
                        Task::done(main_screen::Message::NextMessage.into())
                    }
                    else {
                        Task::none()
                    }
                }
            },
            Message::OnClose => {
                log::warn!("Closing application, saving data...");
                self.save().unwrap_or_else(|e| log::error!("Failed to save data: {e}"));
                iced::exit()
            }
            Message::Notification(e) => {
                log::warn!("{e}");
                self.notification.set_text(e);
                self.notification.show(now);
                Task::perform(tokio::time::sleep(Duration::from_millis(NOTIFICATION_SHOW_TIME)), |_| Message::NotificationClose)
            },
            Message::NotificationClose => {
                // if !self.notification.is_animating(now) && self.notification.is_open() {
                if self.notification.is_open() {
                    self.notification.close(now);
                }
                Task::none()
            },
            Message::ThemeChange(theme) => {
                log::info!("Changing theme to: {}", &theme);
                match theme {
                    Theme::System => {
                        iced::system::theme().map(|mode| Message::ThemeChange(mode.into()))
                    },
                    Theme::Light if self.data.theme.is_system() => {
                        self.data.theme = Theme::Light;
                        Task::none()
                    }
                    Theme::Dark if self.data.theme.is_system() => {
                        self.data.theme = Theme::Dark;
                        Task::none()
                    }
                    Theme::Selected(theme) => {
                        self.data.theme = Theme::Selected(theme);
                        Task::none()
                    }
                    _ => {
                        Task::none()
                    },
                }
            },
            Message::RecivedNetworks(networks) => {
                self.data.networks.extend(networks);
                Task::done(Message::Notification("Нові мережі додані!".to_owned()))
            },
            Message::None => Task::done(main_screen::Message::UpdateMessageHistory.into()),
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        Stack::new()
        .width(Length::Fill)
        .height(Length::Fill)
        .push(
            match self.cur_screen {
                Screen::Main => self.main_scr.view(&self.data).map(Into::into),
                Screen::Settings => self.sett_scr.view(&self.data).map(Into::into),
                Screen::Categories => self.category_scr.view(&self.data).map(Into::into)
            }
        )
        .push(
            self.notification.view(self.now)
        )
        .into()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        Subscription::batch([
            iced::system::theme_changes().map(|mode| Message::ThemeChange(mode.into())),
            Subscription::run(Self::setup_subscription),
            if self.data.autoupdate_groups { iced::time::every(std::time::Duration::from_secs(10)).map(|_| Message::UpdateGroupList) } else { Subscription::none() },
            iced::window::close_requests().map(|_| Message::OnClose),
            if self.is_animating() { iced::window::frames().map(|_| Message::None) } else { Subscription::none() },
            iced::keyboard::listen().map(Message::Keyboard),
        ])
    }

    pub fn is_animating(&self) -> bool {
        self.main_scr.show_message_history.is_animating(self.now) ||
        self.notification.is_animating(self.now)
    }

    fn setup_subscription() -> impl Stream<Item = Message> {
        iced::stream::channel(10, async |mut sender| {
            let (tx, mut rx) = futures::channel::mpsc::unbounded::<Message>();
            _ = sender.send(Message::SetupSignalWorker(tx)).await;

            while let Some(m) = rx.next().await {
                _ = sender.send(m).await;
            }
        })
    }

    pub fn theme(&self) -> iced::Theme {
        self.data.theme.as_theme().clone()
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct AppData {
    pub groups: HashMap<Key, main_screen::Group>,
    pub recieve_address: SocketAddrV4,
    pub autosend: bool,
    pub sync_interval: u64,
    pub send_timeout: u64,
    pub markdown: bool,
    pub parallel: bool,
    pub history_len: u32,
    pub signal_logged: bool,
    pub whatsapp_logged: bool,
    pub theme: Theme,
    pub categories: Vec<SendCategory>,
    pub networks: NetworksPool,
    pub show_groups: bool,
    pub autoupdate_groups: bool,
    pub message_file: bool,
}

impl Default for AppData {
    fn default() -> Self {
        Self {
            groups: HashMap::new(),
            recieve_address: "127.0.0.1:8000".parse().unwrap(),
            autosend: false,
            sync_interval: 10,
            send_timeout: 90,
            markdown: true,
            parallel: false,
            history_len: 50,
            signal_logged: false,
            whatsapp_logged: false,
            theme: Theme::None,
            categories: Vec::new(),
            networks: HashMap::new(),
            show_groups: true,
            autoupdate_groups: true,
            message_file: false,
        }
    }
}

impl AppData {
    pub fn load() -> anyhow::Result<Self> {
        let data = File::open("data.ron")?;
        let state = ron::de::from_reader(data)?;
        Ok(state)
    }

    pub fn new() -> Self {
        Self::load().unwrap_or_default()
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let mut config_file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open("data.ron")?;
        let s = ron::ser::to_string_pretty(
            self,
            PrettyConfig::default(),
        )?;
        config_file.write_all(s.as_bytes())?;
        Ok(())
    }
}

#[derive(Debug)]
struct Notification {
    text: String,
    open: Animation<bool>
}

impl Notification {
    pub fn new() -> Self {
        Self {
            text: String::new(),
            // open: false,
            open: Animation::new(false)
                  .very_quick()
                  .easing(Easing::EaseInOut)
        }
    }

    pub fn set_text(&mut self, text: String) {
        self.text = text
    }

    pub fn is_animating(&self, now: Instant) -> bool {
        self.open.is_animating(now)
    }

    pub fn is_open(&self) -> bool {
        self.open.value()
        // self.open
    }

    pub fn show(&mut self, now: Instant) {
        self.open.go_mut(true, now);
        // self.open = true;
    }

    pub fn close(&mut self, now: Instant) {
        self.open.go_mut(false, now);
        // self.open = false;
    }

    pub fn view(&self, now: Instant) -> Element<'_, Message> {
        container(
            container(
                text(&self.text)
                .align_y(Alignment::Center)
                .width(Length::Fill)
                .height(Length::Fill)
            )
            .padding(Padding::ZERO.horizontal(10))
            .height(self.open.interpolate(0.0, 40.0, now))
            // .height(if self.open { 40 } else { 0 })
            .width(Length::Fill)
            .style(|theme: &iced::Theme| {
                let palette = theme.palette();
                container::Style { text_color: Some(palette.text),
                    background: Some(palette.background.lighter(0.25).into()),
                    border: Border::default().rounded(5),
                    ..Default::default()
                }
            })
        )
        .padding(10)
        .align_bottom(Length::Fill)
        .width(Length::Fill)
        .into()
    }
}
