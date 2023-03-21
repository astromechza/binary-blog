use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::str::from_utf8;
use std::sync::Arc;

use axum::{Error, response::Html, Router, routing::get};
use axum::extract::{Path, State};
use axum::http::{HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use chrono::FixedOffset;
use clap::Parser;
use maud::{DOCTYPE, html, Markup, PreEscaped};
use rust_embed::RustEmbed;
use tower_http::trace::TraceLayer;
use tracing_subscriber;

#[derive(RustEmbed)]
#[folder = "resources/"]
struct Asset;

#[derive(Parser, Debug)]
#[command(version)]
struct Cli {
    #[arg(long)]
    bind_address: Option<IpAddr>,

    #[arg(long)]
    bind_port: Option<u16>,
}

struct Post {
    path: String,
    title: String,
    date: chrono::DateTime<FixedOffset>,
    content: Markup,
}

struct SharedState {
    posts: Vec<Post>,
    post_index: HashMap<String, usize>,
}

fn collect_posts() -> Vec<Post> {
    let mut options = pulldown_cmark::Options::empty();
    options.insert(pulldown_cmark::Options::ENABLE_STRIKETHROUGH);

    let title_re = regex::Regex::new(r#"<meta x-title="(.+)"/?>"#).unwrap();
    let date_re = regex::Regex::new(r#"<meta x-date="(.+)"/?>"#).unwrap();

    Asset::iter()
        .filter(|x| x.ends_with("content.md"))
        .map(|x| {
            let path = x
                .split("/")
                .take_while(|y| !(*y == "content.md"))
                .last()
                .unwrap();

            let raw_bytes = Asset::get(&x).unwrap();
            let raw_content = from_utf8(raw_bytes.data.as_ref()).unwrap();

            let parsed_title = title_re
                .captures(raw_content)
                .map(|c| c.get(1).unwrap().as_str())
                .unwrap_or("unknown");

            let parsed_date = date_re
                .captures(raw_content)
                .map(|c| c.get(1).unwrap().as_str())
                .map(|c| chrono::DateTime::parse_from_rfc3339(c).unwrap())
                .unwrap_or(chrono::DateTime::default());

            // TODO: find a way to parse/scan the content and populate the date / title fields

            let parser = pulldown_cmark::Parser::new_ext(raw_content, options);
            let mut html_output = String::new();
            pulldown_cmark::html::push_html(&mut html_output, parser);
            let tree: Markup = PreEscaped { 0: html_output };

            Post {
                path: path.to_string(),
                title: parsed_title.to_string(),
                date: parsed_date,
                content: tree,
            }
        })
        .collect()
}

fn build_shared_state(posts: Vec<Post>) -> SharedState {
    let mut post_index = HashMap::new();
    let mut i = 0;
    for x in &posts {
        post_index.insert(x.path.clone(), i);
        i = i + 1;
    }

    SharedState { posts, post_index }
}

async fn list_posts(state: State<Arc<SharedState>>) -> Html<String> {
    Html(
        html! {
            (DOCTYPE)
            html {
                head {
                    title { ("Index") }
                }
                body {
                    h1 { "Hello, World!" }
                    ul {
                        @for x in state.0.posts.iter() {
                            li {
                                a href={ "posts/" (x.path) } {
                                    (x.title) (x.date.to_string())
                                }
                            }
                        }
                    }
                }
            }
        }
            .into_string(),
    )
}

async fn view_post(Path(post_key): Path<String>, state: State<Arc<SharedState>>) -> Result<Html<String>, StatusCode> {
    state.post_index.get(post_key.as_str())
        .map(|i| state.posts.get(*i).unwrap())
        .map(|p| {
            Ok(Html(html! {
                html {
                    (p.content)
                }
            }.into_string()))
        })
        .unwrap_or(Err(StatusCode::NOT_FOUND))
}

async fn view_asset(Path(key): Path<(String, String)>) -> Response {
    let mut path: String = "posts/".to_owned();
    path.push_str(key.0.as_str());
    path.push('/');
    path.push_str(key.1.as_str());
    Asset::get(path.as_str())
        .map(|a| {
            let guess = mime_guess::from_path(path);

            let mut resp = a.data.into_response();
            if !guess.is_empty() {
                let val = HeaderValue::from_str(guess.first_raw().unwrap()).unwrap();
                resp.headers_mut().insert("content-type", val);
            }
            resp
        })
        .unwrap_or_else(|| {
            StatusCode::NOT_FOUND.into_response()
        })
}

#[tokio::main]
async fn main() {
    let args = Cli::parse();
    tracing_subscriber::fmt::init();

    let state = Arc::new(build_shared_state(collect_posts()));

    let app = Router::new()
        .route("/", get(list_posts))
        .route("/posts/:post_key", get(view_post))
        .route("/posts/:post_key/:asset_key", get(view_asset))
        .with_state(state)
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(
                    tower_http::trace::DefaultMakeSpan::new().level(tracing::Level::INFO),
                )
                .on_response(
                    tower_http::trace::DefaultOnResponse::new().level(tracing::Level::INFO),
                ),
        );
    let addr = SocketAddr::from((
        args.bind_address.unwrap_or([127, 0, 0, 1].into()),
        args.bind_port.unwrap_or(8080),
    ));
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
