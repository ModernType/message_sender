use std::{
    fmt::{Display, Write},
    ops::Deref,
};

use derive_more::Display;
use serde::Deserialize;

#[derive(Deserialize, Display, Debug)]
#[serde(from = "MessageOuter", bound = "'de: 'a")]
pub struct Message<'a>(MessageInner<'a>);

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

#[derive(Deserialize, Debug)]
pub struct MessageInner<'a> {
    message: MessageGroup,
    comment: Option<&'a str>,
    #[serde(rename = "rUser")]
    reciever: Name<'a>,
    #[serde(rename = "tUser")]
    sender: Name<'a>,
    datetime: &'a str,
    frequency: &'a str,
    location: &'a str,
    title: &'a str,
    #[serde(rename = "source")]
    _source: &'a str,
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
        if let Some(comment) = self.comment {
            f.write_str(comment)?;
        }
        Ok(())
    }
}

#[derive(Deserialize)]
struct MessageOuter<'a> {
    #[serde(rename = "Key")]
    _freq: &'a str,
    #[serde(rename = "Value")]
    message: MessageInner<'a>,
}

#[derive(Deserialize, Debug)]
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

#[derive(Deserialize, Display, Debug)]
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

#[derive(Debug, Deserialize, Display)]
#[serde(from = "String")]
struct CleanedMessage(String);

impl From<String> for CleanedMessage {
    fn from(value: String) -> Self {
        Self(value.trim().to_owned())
    }
}

#[derive(Deserialize, Display, Debug)]
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
