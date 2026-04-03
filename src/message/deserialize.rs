use std::{
    fmt::{Display, Write},
    ops::Deref, sync::LazyLock,
};

use derive_more::Display;
use serde::Deserialize;

pub static TEST_MESSAGE: LazyLock<Message> = LazyLock::new(|| Message(MessageInner {
    message: MessageGroup(vec![
        IndividualMessage { datetime: "2026-04-03 13:55:56".to_owned(), message: CleanedMessage("1. Я тебе питаю як справи".to_owned()) },
        IndividualMessage { datetime: "2026-04-03 13:55:59".to_owned(), message: CleanedMessage("2. У мене все добре!".to_owned()) },
    ]),
    comment: Some("Щось дуже важливе".to_owned()),
    reciever: Name("Отримайко".to_owned()),
    sender: Name("Надсилайко".to_owned()),
    datetime: "24.02.2022 06:05:55".to_owned(),
    frequency: "123.456 МГц".to_owned(),
    location: "Десь там".to_owned(),
    title: "УКХ мережа таких".to_owned(),
    source: "Прийшло з апарату".to_owned(),
    network_id: None,
}));

#[derive(Deserialize, Display, Debug, Default)]
#[serde(from = "MessageOuter", default)]
pub struct Message(pub MessageInner);

impl Message {
    pub fn format(&self, formatting: Option<&super::Formatting>) -> String {
        match formatting {
            Some(formatting) => formatting.format_message(self),
            None => self.to_string()
        }
    }
}

impl From<MessageOuter> for Message {
    fn from(value: MessageOuter) -> Self {
        Self(value.message)
    }
}

impl Deref for Message {
    type Target = MessageInner;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Deserialize, Debug, Default)]
#[serde(default)]
pub struct MessageInner {
    pub message: MessageGroup,
    pub comment: Option<String>,
    #[serde(rename = "rUser")]
    pub reciever: Name,
    #[serde(rename = "tUser")]
    pub sender: Name,
    pub datetime: String,
    pub frequency: String,
    pub location: String,
    pub title: String,
    pub source: String,
    #[serde(rename = "radionetworkID")]
    pub network_id: Option<u64>,
}

impl Display for MessageInner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.title)?;
        f.write_char('\n')?;
        f.write_str(&self.location)?;
        f.write_char('\n')?;
        f.write_char('\n')?;
        f.write_str(&self.datetime)?;
        f.write_char('\n')?;
        f.write_char('\n')?;
        writeln!(f, "Отримувач: {}", self.reciever)?;
        writeln!(f, "Відправник: {}", self.sender)?;
        f.write_char('\n')?;
        write!(f, "{}", self.message)?;
        if let Some(comment) = &self.comment && !comment.is_empty() {
            f.write_char('\n')?;
            write!(f, "Коментар: {}", comment)?;
        }
        Ok(())
    }
}

#[derive(Deserialize, Default)]
struct MessageOuter {
    #[serde(rename = "Key")]
    _freq: String,
    #[serde(rename = "Value")]
    message: MessageInner,
}

#[derive(Deserialize, Debug,Default)]
#[serde(transparent)]
pub struct MessageGroup(Vec<IndividualMessage>);

impl Display for MessageGroup {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let last_index = self.0.len() - 1;
        for (i, m) in self.0.iter().enumerate() {
            f.write_str(m)?;
            if i != last_index {
                f.write_char('\n')?;
            }
        }
        Ok(())
    }
}

#[derive(Deserialize, Display, Debug, Default)]
#[display("{message}")]
struct IndividualMessage {
    #[serde(rename = "Key")]
    datetime: String,
    #[serde(rename = "Value")]
    message: CleanedMessage,
}

impl Deref for IndividualMessage {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.message.0
    }
}

#[derive(Debug, Deserialize, Display, Default)]
#[serde(from = "String")]
struct CleanedMessage(String);

impl From<String> for CleanedMessage {
    fn from(value: String) -> Self {
        Self(value.trim().to_owned())
    }
}

#[derive(Deserialize, Display, Debug, Default)]
#[serde(from = "Option<String>")]
pub struct Name(String);

impl Deref for Name {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}

impl From<Option<String>> for Name {
    fn from(value: Option<String>) -> Self {
        match value {
            Some(mut name) => if name.is_empty() {
                Self("НВ".to_owned())
            } else {
                name.remove_matches("\n");
                Self(name)
            },
            None => Self("НВ".to_owned())
        }
    }
}
