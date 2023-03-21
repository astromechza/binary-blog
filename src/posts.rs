use chrono::{DateTime, FixedOffset};

#[derive(Debug, Eq, PartialEq)]
pub(crate) struct Post {
    pub(crate) title: String,
    pub(crate) date: DateTime<FixedOffset>,
    pub(crate) raw_markdown: &'static str,
}

pub(crate) enum Tag {}

pub mod _0001_a_binary_blog;
