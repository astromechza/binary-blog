use chrono::{DateTime, FixedOffset};

#[derive(Debug)]
pub(crate) struct Post {
    title: String,
    date: DateTime<FixedOffset>,
    published: bool,
    pub(crate) raw_markdown: &'static str,
}

pub(crate) enum Tag {
}

pub mod _0001_a_binary_blog;