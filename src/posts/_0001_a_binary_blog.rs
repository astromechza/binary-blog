use crate::posts::Post;
use chrono::DateTime;

pub(crate) fn build() -> Post {
    return Post {
        title: "A binary blog".to_string(),
        date: DateTime::parse_from_rfc3339("2023-04-01T00:00:00Z").unwrap(),
        raw_markdown: r#"
This is my example blog post.

Look it supports **markdown**.

Amazing!
    "#,
    };
}
