use pest::{Parser, iterators::Pair};
use pest_derive::Parser;
use presage::proto::{BodyRange, body_range::Style};
use unicode_segmentation::UnicodeSegmentation;

#[derive(Parser)]
#[grammar = "message/format.pest"]
struct MarkdownParser;

fn parse_into_rules<'a>(s: &'a str) -> Result<impl Iterator<Item = Pair<'a, Rule>>, pest::error::Error<Rule>> {
    let res = MarkdownParser::parse(Rule::formatted_text, s)?;
    Ok(
        res.into_iter().flat_map(|r| r.into_inner())
    )
}

fn construct_ranges<'a>(pairs: impl Iterator<Item = Pair<'a, Rule>>) -> (String, Vec<BodyRange>) {
    let mut message_parts = Vec::new();
    let mut ranges = Vec::new();
    let mut message_length = 0u32;
    let mut bold: Option<u32> = None;
    let mut italic: Option<u32> = None;
    let mut strikethrough: Option<u32> = None;

    for rule in pairs {
        match rule.as_rule() {
            Rule::text => {
                let text = rule.as_str();
                message_length += text.graphemes(true).count() as u32;
                message_parts.push(text);
            },
            Rule::bold => {
                match bold {
                    Some(start) => ranges.push(
                        BodyRange {
                            start: Some(start),
                            length: Some(message_length - start),
                            associated_value: Some(presage::proto::body_range::AssociatedValue::Style(Style::Bold as i32))
                        }
                    ),
                    None => {
                        bold = Some(message_length);
                    }
                }
            },
            Rule::italic => {
                match italic {
                    Some(start) => ranges.push(
                        BodyRange {
                            start: Some(start),
                            length: Some(message_length - start),
                            associated_value: Some(presage::proto::body_range::AssociatedValue::Style(Style::Italic as i32))
                        }
                    ),
                    None => {
                        italic = Some(message_length);
                    }
                }
            },
            Rule::strikethrough => {
                match strikethrough {
                    Some(start) => ranges.push(
                        BodyRange {
                            start: Some(start),
                            length: Some(message_length - start),
                            associated_value: Some(presage::proto::body_range::AssociatedValue::Style(Style::Strikethrough as i32))
                        }
                    ),
                    None => {
                        strikethrough = Some(message_length);
                    }
                }
            },
            _ => continue
        }
    }

    (
        message_parts.join(""),
        ranges
    )
}

pub fn parse_message_with_format(message: &str) -> Result<(String, Vec<BodyRange>), pest::error::Error<Rule>> {
    Ok(
        construct_ranges(
            parse_into_rules(message)?
        )
    )
}

pub fn parse_message_with_whatsapp_format(message: &str) -> Result<String, pest::error::Error<Rule>> {
    let mut buf = String::with_capacity(message.len());
    for rule in parse_into_rules(message)? {
        match rule.as_rule() {
            Rule::text => buf.push_str(rule.as_str()),
            Rule::bold => buf.push('*'),
            Rule::italic => buf.push('_'),
            Rule::strikethrough => buf.push('~'),
            _ => continue,
        }
    }
    Ok(buf)
}

#[cfg(test)]
mod test {
    use crate::message::format::parse_message_with_format;

    #[test]
    fn test() {
        let s1 = "Regular text with some **bold statement** and ~~strikethrough~~*italic* touch";

        let (message, ranges) = parse_message_with_format(s1).unwrap();
        // println!("{message}\n{ranges:#?}");
        for range in ranges.iter() {
            println!("{:?}: {}", range.associated_value, &message[range.start.unwrap() as usize .. (range.start.unwrap() + range.length.unwrap()) as usize]);
        }
    }
}
