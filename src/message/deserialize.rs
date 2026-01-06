use std::{
    fmt::{Display, Write},
    ops::Deref,
};

use derive_more::Display;
use serde::Deserialize;

#[derive(Deserialize, Display, Debug, Default)]
#[serde(from = "MessageOuter", bound = "'de: 'a", default)]
pub struct Message<'a>(pub MessageInner<'a>);

impl<'a> From<MessageOuter<'a>> for Message<'a> {
    fn from(value: MessageOuter<'a>) -> Self {
        Self(value.message)
    }
}

impl<'a> Deref for Message<'a> {
    type Target = MessageInner<'a>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Deserialize, Debug, Default)]
#[serde(default)]
pub struct MessageInner<'a> {
    pub message: MessageGroup,
    pub comment: Option<&'a str>,
    #[serde(rename = "rUser")]
    pub reciever: Name<'a>,
    #[serde(rename = "tUser")]
    pub sender: Name<'a>,
    pub datetime: &'a str,
    pub frequency: &'a str,
    pub location: &'a str,
    pub title: &'a str,
    #[serde(rename = "source")]
    pub _source: &'a str,
    #[serde(rename = "networkID")]
    pub network_id: Option<u64>,
}

impl<'a> Message<'a> {
    pub fn with_frequency(&self) -> String {
        format!("{}\n{}", self.0.frequency, self)
    }
}

impl<'a> Display for MessageInner<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.title)?;
        f.write_char('\n')?;
        f.write_str(self.location)?;
        f.write_char('\n')?;
        f.write_char('\n')?;
        f.write_str(self.datetime)?;
        f.write_char('\n')?;
        f.write_char('\n')?;
        writeln!(f, "Отримувач: {}", self.reciever)?;
        writeln!(f, "Відправник: {}", self.sender)?;
        f.write_char('\n')?;
        write!(f, "{}", self.message)?;
        if let Some(comment) = self.comment && !comment.is_empty() {
            f.write_char('\n')?;
            write!(f, "Коментар: {}", comment)?;
        }
        Ok(())
    }
}

#[derive(Deserialize, Default)]
struct MessageOuter<'a> {
    #[serde(rename = "Key")]
    _freq: &'a str,
    #[serde(rename = "Value")]
    message: MessageInner<'a>,
}

#[derive(Deserialize, Debug,Default)]
#[serde(transparent)]
struct MessageGroup(Vec<IndividualMessage>);

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
#[serde(from = "Option<&str>")]
struct Name<'a>(&'a str);

impl<'a> Deref for Name<'a> {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a> From<Option<&'a str>> for Name<'a> {
    fn from(value: Option<&'a str>) -> Self {
        match value {
            Some(name) => if name.is_empty() {
                Self("НВ")
            } else {
                Self(name)
            },
            None => Self("НВ")
        }
    }
}
