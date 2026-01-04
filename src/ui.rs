use std::{
    borrow::Cow, collections::HashMap, fmt::Debug, fs::{File, OpenOptions}, io::Write, net::SocketAddrV4, sync::{Arc, Mutex}, time::{Duration, Instant}
};
use derive_more::Display;
use futures::{SinkExt, Stream, StreamExt, channel::{mpsc::UnboundedSender}};
use iced::{Alignment, Animation, Border, Element, Length, Padding, Subscription, Task, animation::Easing, widget::{Stack, container, text, text_editor}};
use presage::{Manager, manager::Registered};
use presage_store_sqlite::SqliteStore;
use serde::{Serialize, Deserialize};
use crate::{message::OperatorMessage, message_server::{self, AcceptedMessage}, messangers::{Key, whatsapp}, send_categories::{NetworkInfo, NetworksPool, SendCategory}, ui::{category_screen::CategoryScreen, main_screen::{Group, LinkState}, theme::Theme}};

use crate::{messangers::signal::{SignalMessage, SignalWorker}, ui::{ext::ColorExt, main_screen::MainScreen, message_history::SendMessageInfo, settings_screen::SettingsScreen}};

pub mod main_screen;
pub mod settings_screen;
pub mod message_history;
pub mod category_screen;
mod ext;
mod theme;

const NOTIFICATION_SHOW_TIME: u64 = 3000;

#[derive(Debug, Clone)]
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
    None,
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
    cur_screen: Screen,
    manager: Option<Manager<SqliteStore, Registered>>,
    whatsapp_client: Option<Arc<whatsapp_rust::Client>>,
    main_scr: MainScreen,
    sett_scr: SettingsScreen,
    category_scr: CategoryScreen,
    signal_task_send: Option<UnboundedSender<SignalMessage>>,
    sync_interval: u64,
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
        let groups = data.cached_groups.into_owned();
        let start_task = if data.theme.is_system() {
            iced::system::theme().map(|mode| Message::ThemeChange(mode.into()))
        }
        else {
            Task::none()
        };

        (
            Self {
                manager: None,
                whatsapp_client: None,
                cur_screen: Screen::Main,
                main_scr: MainScreen::new(data.autosend, groups, data.history_len),
                sett_scr: SettingsScreen::new(data.markdown, data.parallel, data.recieve_address, data.history_len, data.theme),
                category_scr: CategoryScreen::new(data.categories, data.networks.into_owned()),
                signal_task_send: None,
                sync_interval: data.sync_interval,
                now: Instant::now(),
                notification: Notification::new(),
                signal_logged: data.signal_logged,
                whatsapp_logged: data.whatsapp_logged,
            },
            start_task
        )
    }

    fn save(&self) -> anyhow::Result<()> {
        let data = AppData {
            cached_groups: Cow::Borrowed(self.main_scr.groups()),
            autosend: self.main_scr.autosend(),
            sync_interval: self.sync_interval,
            markdown: self.sett_scr.markdown,
            parallel: self.sett_scr.parallel,
            history_len: self.sett_scr.history_len,
            recieve_address: self.sett_scr.recieve_address,
            signal_logged: self.signal_logged,
            whatsapp_logged: self.whatsapp_logged,
            theme: self.sett_scr.theme_selected.clone(),
            send_timeout: 90,
            categories: self.category_scr.categories.clone(),
            networks: Cow::Borrowed(&self.category_scr.networks)
        };
        data.save()
    }

    #[inline]
    fn whatsapp_registered(&self) -> bool {
        self.main_scr.whatsapp_state == LinkState::Linked
    }

    pub fn update(&mut self, message: Message, now: Instant) -> Task<Message> {
        self.now = now;

        match message {
            Message::MainScrMessage(m) => self.main_scr.update(m, now, &self.category_scr.categories),
            Message::SettingsScrMessage(m) => self.sett_scr.update(m),
            Message::CategoriesScrMessage(m) => self.category_scr.update(m, &mut self.main_scr.groups),
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
                Task::none()
            }
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
                SignalWorker::spawn_new(task_rx, tx.clone());
                self.signal_task_send = Some(task_tx);
                whatsapp::UI_MESSAGE_SENDER.set(tx.clone()).unwrap();

                Task::batch([
                    if self.signal_logged { Task::done(SignalMessage::LinkBegin.into()) } else { Task::none() },
                    if self.whatsapp_logged { Task::perform(whatsapp::start_whatsapp_task(), |_| Message::None) } else { Task::none() },
                    Task::perform(message_server::start_server(self.sett_scr.recieve_address, tx), |_| Message::Notification("Message server stopped working".to_owned()))
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
                        Task::done(SignalMessage::SendMessage(manager.clone(), message.clone(), self.sett_scr.markdown, self.sett_scr.parallel).into())
                    );
                }
                if let Some(client) = self.whatsapp_client.as_ref() {
                    task_list.push(
                        Task::perform(whatsapp::send_message(client.clone(), message, self.sett_scr.markdown), |_| Message::None)
                    );
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
                        Task::done(SignalMessage::EditMessage(manager.clone(), message.clone(), timestamps, self.sett_scr.markdown).into())
                    );
                }
                if let Some(client) = self.whatsapp_client.as_ref() {
                    task_list.push(
                        Task::perform(whatsapp::edit_message(client.clone(), message, whatsapp_ids, self.sett_scr.markdown), |_| Message::None)
                    );
                }
                Task::batch(task_list)
            },
            Message::AcceptMessage(messages) => {
                let autosend = messages.iter().fold(self.main_scr.autosend(), |autosend, msg| autosend && !msg.autosend_overwrite);
                for msg in messages.iter() {
                    if let Some(network) = &msg.network && !self.category_scr.networks.contains_key(network) {
                        self.category_scr.networks.insert(network.clone(), NetworkInfo::new(0, network.clone()));
                    }
                }

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
                    Theme::Light if self.sett_scr.theme_selected.is_system() => {
                        self.sett_scr.theme_selected = Theme::Light;
                        Task::none()
                    }
                    Theme::Dark if self.sett_scr.theme_selected.is_system() => {
                        self.sett_scr.theme_selected = Theme::Dark;
                        Task::none()
                    }
                    Theme::Selected(theme) => {
                        self.sett_scr.theme_selected = Theme::Selected(theme);
                        Task::none()
                    }
                    _ => {
                        Task::none()
                    },
                }
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
                Screen::Main => self.main_scr.view().map(Into::into),
                Screen::Settings => self.sett_scr.view().map(Into::into),
                Screen::Categories => self.category_scr.view(self.main_scr.groups()).map(Into::into)
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
            iced::time::every(std::time::Duration::from_secs(self.sync_interval)).map(|_| Message::UpdateGroupList),
            iced::window::close_requests().map(|_| Message::OnClose),
            if self.is_animating() { iced::window::frames().map(|_| Message::None) } else { Subscription::none() },
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
        self.sett_scr.theme_selected.as_theme().clone()
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct AppData<'a> {
    pub cached_groups: Cow<'a, HashMap<Key, main_screen::Group>>,
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
    pub networks: Cow<'a, NetworksPool>,
}

impl Default for AppData<'_> {
    fn default() -> Self {
        Self {
            cached_groups: Cow::Owned(HashMap::new()),
            recieve_address: "127.0.0.1:8000".parse().unwrap(),
            autosend: false,
            sync_interval: 10,
            send_timeout: 90,
            markdown: true,
            parallel: false,
            history_len: 20,
            signal_logged: false,
            whatsapp_logged: false,
            theme: Theme::None,
            categories: Vec::new(),
            networks: Cow::Owned(HashMap::new()),
        }
    }
}

impl AppData<'_> {
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
        let s = ron::ser::to_string(self)?;
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
