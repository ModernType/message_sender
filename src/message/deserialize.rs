use std::{
    fmt::{Display, Write},
    ops::Deref,
};

use derive_more::Display;
use serde::Deserialize;

#[derive(Deserialize, Display, Debug, Default)]
#[serde(from = "MessageOuter", default)]
pub struct Message(pub MessageInner);

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
    #[serde(rename = "source")]
    pub _source: String,
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
        for m in self.0.iter() {
            f.write_str(m)?;
            f.write_char('\n')?;
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
