use iced::{Element, widget::{Column, Row, Stack}};

pub trait PushMaybe<'a, Message> {
    fn push_maybe(self, child: Option<impl Into<Element<'a, Message>>>) -> Self;
}

macro_rules! impl_push_maybe {
    ($t:ident) => {
        impl<'a, Message> PushMaybe<'a, Message> for $t<'a, Message>{
            fn push_maybe(self, child: Option<impl Into<Element<'a, Message>>>) -> Self {
                match child {
                    Some(child) => self.push(child),
                    None => self
                }
            }
        }
    };
}

impl_push_maybe!(Column);
impl_push_maybe!(Row);
impl_push_maybe!(Stack);

pub trait ColorExt {
    fn lighter(self, amount: f32) -> Self;
    fn darker(self, amount: f32) -> Self;
}

impl ColorExt for iced::Color {
    fn lighter(self, amount: f32) -> Self {
        iced::Color {
            r: (self.r + self.r * amount).min(1.0),
            g: (self.g + self.g * amount).min(1.0),
            b: (self.b + self.b * amount).min(1.0),
            a: self.a,
        }
    }

    fn darker(self, amount: f32) -> Self {
        iced::Color {
            r: (self.r - self.r * amount).max(0.0),
            g: (self.g - self.g * amount).max(0.0),
            b: (self.b - self.b * amount).max(0.0),
            a: self.a,
        }
    }
}