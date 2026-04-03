use std::{collections::{HashMap, HashSet}, fs::{File, OpenOptions}, io::Write, net::{IpAddr, Ipv4Addr, SocketAddrV4}, path::{Path, PathBuf}, sync::LazyLock};

use crate::{message::Formatting, ui::theme::Theme};
use local_ip_address::local_ip;
use ron::ser::PrettyConfig;
use serde::Serialize;
use serde_versioning::Deserialize;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::{messangers::Key, send_categories::{NetworksPool, SendCategory}, ui::{main_screen, message_history::SaveMessageInfo}};


#[derive(Debug, Serialize, Deserialize, Clone)]
struct AppData1 {
    pub groups: HashMap<Key, main_screen::Group>,
    pub recieve_address: SocketAddrV4,
    pub autosend: bool,
    pub sync_interval: u64,
    pub send_timeout: u64,
    pub markdown: bool,
    pub history_len: u32,
    pub signal_logged: bool,
    pub whatsapp_logged: bool,
    pub theme: Theme,
    pub categories: Vec<SendCategory>,
    pub networks: NetworksPool,
    pub sources: HashSet<String>,
    pub comments: HashSet<String>,
    pub show_groups: bool,
    pub autoupdate_groups: bool,
    pub message_file: bool,
    pub saved_messages: Vec<SaveMessageInfo>,
}

#[derive(Debug, Serialize, Deserialize, Clone, better_default::Default)]
#[serde(default)]
#[versioning(optimistic, previous_version = "AppData1")]
pub struct AppData {
    /// Stores group keys mapped to group info
    pub groups: HashMap<Key, main_screen::Group>,
    /// The address and port the application listens on for incoming messages
    #[default(default_address())]
    pub recieve_address: SocketAddrV4,
    /// Automatically send messages upon receiving them without user confirmation
    pub autosend: bool,
    /// Use markdown formatting for messages
    #[default(true)]
    pub markdown: bool,
    /// Length of message history
    #[default(50)]
    pub history_len: u32,
    /// Has user logged to signal
    pub signal_logged: bool,
    /// Has user logged to whatsapp
    pub whatsapp_logged: bool,
    /// App theme
    pub theme: Theme,
    /// List of categories (channels) user set up
    pub categories: Vec<SendCategory>,
    /// Map of network ids mapped to network info
    pub networks: NetworksPool,
    /// Set of sources saved from sent messages
    pub sources: HashSet<String>,
    /// Set of comments saved from sent messages
    pub comments: HashSet<String>,
    /// Whether to show "Message from file" button
    pub message_file: bool,
    /// Not sent messages saved after close up
    pub saved_messages: Vec<SaveMessageInfo>,
    /// Formatting used to send messages
    pub formatting: Option<Formatting>,
}

impl From<AppData1> for AppData {
    fn from(value: AppData1) -> Self {
        Self {
            groups: value.groups,
            recieve_address: value.recieve_address,
            autosend: value.autosend,
            markdown: value.markdown,
            history_len: value.history_len,
            signal_logged: value.signal_logged,
            whatsapp_logged: value.whatsapp_logged,
            theme: value.theme,
            categories: value.categories,
            networks: value.networks,
            sources: value.sources,
            comments: value.comments,
            message_file: value.message_file,
            saved_messages: value.saved_messages,
            ..Default::default()
        }
    }
}

static SETTINGS_PATH: LazyLock<PathBuf> = LazyLock::new(
    || match std::env::home_dir() {
        Some(path) => path.join(".sender/data.ron"),
        None => PathBuf::from("data.ron"),
    }
);

impl AppData {
    pub fn load() -> anyhow::Result<Self> {
        let data = File::open(SETTINGS_PATH.as_path())?;
        let state = ron::de::from_reader(data)?;
        Ok(state)
    }

    pub async fn load_from(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let mut file = tokio::fs::File::open(path).await?;
        let mut content = String::new();
        file.read_to_string(&mut content).await?;
        Ok(ron::de::from_str(&content)?)
    }

    pub async fn save_to(&self, path: impl AsRef<Path>) -> std::io::Result<()> {
        let mut file = tokio::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(path)
            .await?;
        let s = ron::ser::to_string_pretty(
            self,
            PrettyConfig::default(),
        ).unwrap();

        file.write_all(s.as_bytes()).await?;
        Ok(())
    }

    pub fn new() -> Self {
        Self::load().unwrap_or_default()
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let mut config_file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(SETTINGS_PATH.as_path())?;
        let s = ron::ser::to_string_pretty(
            self,
            PrettyConfig::default(),
        )?;
        config_file.write_all(s.as_bytes())?;
        Ok(())
    }
}

fn default_address() -> SocketAddrV4 {
    SocketAddrV4::new(
        match local_ip() {
            Ok(IpAddr::V4(ip)) => ip,
            _ => Ipv4Addr::new(127, 0, 0, 1),
        },
        8000
    )
}
