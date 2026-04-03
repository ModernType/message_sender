use std::str::FromStr;

use derive_more::Display;
use pest::Parser;
use pest_derive::Parser;
use serde::{Deserialize, Serialize};

use crate::message::OperatorMessage;


#[derive(Parser)]
#[grammar = "message/compose.pest"]
struct FormattingParser;

#[derive(Debug, Clone, Serialize, Deserialize, Display)]
pub enum FormatPart {
    #[display("%частота%")]
    Freq,
    #[display("%текст%")]
    Text,
    #[display("%хто%")]
    Who,
    #[display("%кому%")]
    Whom,
    #[display("%заголовок%")]
    Title,
    #[display("%привʼязка%")]
    Location,
    #[display("%дата%")]
    Date,
    #[display("%джерело%")]
    Source,
    #[display("%коментар%")]
    Comment,
    #[display("{_0}")]
    Literal(String),
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Formatting {
    parts: Vec<FormatPart>,
}

impl std::fmt::Display for Formatting {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for part in self.parts.iter() {
            write!(f, "{part}")?;
        }
        Ok(())
    }
}

impl FromIterator<FormatPart> for Formatting {
    fn from_iter<T: IntoIterator<Item = FormatPart>>(iter: T) -> Self {
        Self {
            parts: iter.into_iter().collect(),
        }
    }
}

impl FromStr for Formatting {
    type Err = pest::error::Error<Rule>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let pairs = FormattingParser::parse(Rule::format, s)?;
        Ok(
            pairs.filter_map(
                |pair| {
                    match pair.as_rule() {
                        Rule::freq => Some(FormatPart::Freq),
                        Rule::text => Some(FormatPart::Text),
                        Rule::who => Some(FormatPart::Who),
                        Rule::whom => Some(FormatPart::Whom),
                        Rule::title => Some(FormatPart::Title),
                        Rule::location => Some(FormatPart::Location),
                        Rule::date => Some(FormatPart::Date),
                        Rule::source => Some(FormatPart::Source),
                        Rule::comment => Some(FormatPart::Comment),
                        Rule::literal => Some(FormatPart::Literal(pair.as_str().to_owned())),
                        _ => None
                    }
                }
            )
            .collect()
        )
    }
}

impl<T: AsRef<str>> From<T> for Formatting {
    fn from(value: T) -> Self {
        value.as_ref().parse().unwrap()
    }
}

impl Formatting {
    pub fn parse(s: &str) -> Self {
        s.parse().unwrap()
    }

    pub fn format_message(&self, message: &OperatorMessage) -> String {
        let mut res = String::new();
        for part in self.parts.iter() {
            match part {
                FormatPart::Freq => res.push_str(&message.frequency),
                FormatPart::Text => res.push_str(&message.message.to_string()),
                FormatPart::Who => res.push_str(&message.sender),
                FormatPart::Whom => res.push_str(&message.reciever),
                FormatPart::Title => res.push_str(&message.title),
                FormatPart::Location => res.push_str(&message.location),
                FormatPart::Date => res.push_str(&message.datetime),
                FormatPart::Source => res.push_str(&message.source),
                FormatPart::Comment => res.push_str(message.comment.as_deref().unwrap_or("")),
                FormatPart::Literal(s) => res.push_str(s),
            }
        }

        res
    }
}
