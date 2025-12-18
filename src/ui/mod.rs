use std::{
    borrow::Cow, collections::HashMap, fmt::Debug, fs::{File, OpenOptions}, io::Write, net::SocketAddrV4, sync::Arc
};
use derive_more::Display;
use futures::{SinkExt, Stream, StreamExt, channel::{mpsc::UnboundedSender}};
use iced::{Element, Subscription, Task};
use presage::{Manager, manager::Registered};
use presage_store_sqlite::SqliteStore;
use ron::ser::PrettyConfig;
use serde::{Serialize, Deserialize};

use crate::{signal::{SignalMessage, SignalWorker}, ui::{main_screen::MainScreen, message_history::SendMessageInfo, settings_screen::SettingsScreen}};

pub mod main_screen;
pub mod settings_screen;
pub mod message_history;
mod ext;

#[derive(Debug, Clone)]
enum Screen {
    Main,
    Settings,
}

#[derive(Debug, Clone)]
pub enum Message {
    MainScrMessage(main_screen::Message),
    SettingsScrMessage(settings_screen::Message),
    SignalMessage(SignalMessage),
    SetManager(Manager<SqliteStore, Registered>),
    SetupSignalWorker(UnboundedSender<Message>),
    SendMessage(Arc<SendMessageInfo>),
    SetScreen(Screen),
    OnClose,
    UpdateGroupList,
    Synced,
    Notification(String),
    None,
}

impl From<SignalMessage> for Message {
    fn from(value: SignalMessage) -> Self {
        Self::SignalMessage(value)
    }
}

#[derive(Debug)]
pub struct App {
    cur_screen: Screen,
    manager: Option<Manager<SqliteStore, Registered>>,
    main_scr: MainScreen,
    sett_scr: SettingsScreen,
    signal_task_send: Option<UnboundedSender<SignalMessage>>,
    sync_interval: u64,
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
    pub fn new() -> Self {
        let data = AppData::new();
        let groups = data.cached_groups.into_owned();
        Self {
            manager: None,
            cur_screen: Screen::Main,
            main_scr: MainScreen::new(data.autosend, groups),
            sett_scr: SettingsScreen::new(data.markdown, data.parallel, data.recieve_address),
            signal_task_send: None,
            sync_interval: data.sync_interval,
        }
    }

    fn save(&self) -> anyhow::Result<()> {
        let data = AppData {
            cached_groups: Cow::Borrowed(self.main_scr.groups()),
            autosend: self.main_scr.autosend(),
            sync_interval: self.sync_interval,
            markdown: self.sett_scr.markdown,
            parallel: self.sett_scr.parallel,
            ..Default::default()
        };
        data.save()
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::MainScrMessage(m) => self.main_scr.update(m),
            Message::SettingsScrMessage(m) => self.sett_scr.update(m),
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
                Task::batch([
                    Task::done(main_screen::Message::SetLinkState(main_screen::LinkState::Linked).into()),
                    Task::done(SignalMessage::Sync(mng).into())   
                ])
            },
            Message::Synced => {
                Task::done(main_screen::Message::UpdateGroups.into())
            }
            Message::SetupSignalWorker(tx) => {
                let (task_tx, task_rx) = futures::channel::mpsc::unbounded();
                SignalWorker::spawn_new(task_rx, tx);
                self.signal_task_send = Some(task_tx);

                Task::done(SignalMessage::LinkBegin.into())
            },
            Message::UpdateGroupList => {
                let manager = self.manager.as_ref().unwrap().clone();
                Task::perform(crate::signal::get_groups(manager), |v| v.map(main_screen::Message::SetGroups).into())
            },
            Message::SendMessage(message) => {
                Task::done(SignalMessage::SendMessage(self.manager.as_ref().unwrap().clone(), message, self.sett_scr.markdown, self.sett_scr.parallel).into())
            },
            Message::OnClose => {
                log::warn!("Closing application, saving data...");
                self.save().unwrap_or_else(|e| log::error!("Failed to save data: {e}"));
                iced::exit()
            }
            Message::Notification(e) => {
                log::warn!("{e}");
                Task::none()
            },
            Message::None => Task::none(),
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        match self.cur_screen {
            Screen::Main => self.main_scr.view().map(Into::into),
            Screen::Settings => self.sett_scr.view().map(Into::into),
        }
    }

    pub fn subscription(&self) -> Subscription<Message> {
        Subscription::batch([
            Subscription::run(Self::setup_subscription),
            iced::time::every(std::time::Duration::from_secs(self.sync_interval)).map(|_| Message::UpdateGroupList),
            iced::window::close_requests().map(|_| Message::OnClose),
        ])
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
        match dark_light::detect().unwrap_or(dark_light::Mode::Unspecified) {
            dark_light::Mode::Light => iced::Theme::CatppuccinLatte,
            dark_light::Mode::Dark => iced::Theme::Dracula,
            dark_light::Mode::Unspecified => iced::Theme::CatppuccinLatte,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct AppData<'a> {
    pub cached_groups: Cow<'a, HashMap<[u8; 32], main_screen::Group>>,
    pub recieve_address: SocketAddrV4,
    pub autosend: bool,
    pub send_mode: SendMode,
    pub sync_interval: u64,
    pub send_timeout: u64,
    pub markdown: bool,
    pub parallel: bool,
}

impl Default for AppData<'_> {
    fn default() -> Self {
        Self {
            cached_groups: Cow::Owned(HashMap::new()),
            recieve_address: "127.0.0.1:8000".parse().unwrap(),
            autosend: false,
            send_mode: SendMode::Standard,
            sync_interval: 10,
            send_timeout: 90,
            markdown: true,
            parallel: false,
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

#[derive(Debug, Serialize, Deserialize, Display, Clone, Copy, PartialEq, Eq)]
pub enum SendMode {
    Standard,
    Frequency,
    Plain
}