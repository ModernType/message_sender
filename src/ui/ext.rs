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