use std::borrow::Cow;
use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::ops::Add;
use std::str::from_utf8;
use std::sync::Arc;

use axum::{http, Router, routing::get};
use axum::extract::{Path, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use chrono::{DateTime, FixedOffset};
use clap::Parser;
use lazy_static::lazy_static;
use maud::{DOCTYPE, html, Markup, PreEscaped};
use rust_embed::RustEmbed;
use tower_http::trace::TraceLayer;
use tracing_subscriber;

#[derive(RustEmbed)]
#[folder = "resources/"]
struct Asset;

lazy_static! {
    static ref NORMALIZE_CSS: PreEscaped<String> = PreEscaped{0: from_utf8(Asset::get("normalize.css").unwrap().data.as_ref()).unwrap().to_owned()};
    static ref MILLIGRAM_CSS: PreEscaped<String> = PreEscaped{0: from_utf8(Asset::get("milligram.css").unwrap().data.as_ref()).unwrap().to_owned()};
}

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
    date: DateTime<FixedOffset>,
    description: Option<String>,
    pre_rendered: Cow<'static, str>,
    assets: HashMap<String, Cow<'static, [u8]>>
}

struct SharedState {
    posts: Vec<Post>,
    post_index: HashMap<String, usize>,
    pre_rendered_index: Cow<'static, str>,
    pre_rendered_not_found: Cow<'static, str>,
}

const CONTENT_FILE_NAME: &str = "content.md";
const HTML_CONTENT_TYPE: &str = "text/html; charset=utf-8";

fn collect_posts() -> Vec<Post> {
    let mut options = pulldown_cmark::Options::empty();
    options.insert(pulldown_cmark::Options::ENABLE_STRIKETHROUGH);

    let title_re = regex::Regex::new(r#"<meta x-title="(.+)"/?>"#).unwrap();
    let date_re = regex::Regex::new(r#"<meta x-date="(.+)"/?>"#).unwrap();
    let description_r = regex::Regex::new(r#"<meta x-description="(.+)"/?>"#).unwrap();

    Asset::iter()
        .filter(|x| x.ends_with(CONTENT_FILE_NAME))
        .map(|x| {
            let path = x
                .split("/")
                .take_while(|y| !(*y == CONTENT_FILE_NAME))
                .last()
                .unwrap()
                .to_string();

            let raw_bytes = Asset::get(&x).unwrap();
            let raw_content = from_utf8(raw_bytes.data.as_ref()).unwrap();

            let parsed_title = title_re
                .captures(raw_content)
                .map(|c| c.get(1).unwrap().as_str().to_owned())
                .unwrap_or("unknown".to_string());

            let parsed_date = date_re
                .captures(raw_content)
                .map(|c| c.get(1).unwrap().as_str())
                .map(|c| chrono::DateTime::parse_from_rfc3339(c).unwrap())
                .unwrap_or(chrono::DateTime::default());

            let parsed_description = description_r
                .captures(raw_content)
                .map(|c| c.get(1).unwrap().as_str().to_owned());

            let parser = pulldown_cmark::Parser::new_ext(raw_content, options);
            let mut html_output = String::new();
            pulldown_cmark::html::push_html(&mut html_output, parser);
            let tree: Markup = PreEscaped { 0: html_output };

            let content = pre_render_post(&parsed_title, &parsed_date, &parsed_description, &tree);

            let mut assets = HashMap::new();

            let prefix = x.rsplitn(2, "/")
                .skip(1)
                .last()
                .unwrap()
                .to_string()
                .add("/");

            Asset::iter()
                .filter(|a| a.contains(&prefix))
                .filter(|a| !a.contains(CONTENT_FILE_NAME))
                .for_each(|a| {
                    assets.insert(a.strip_prefix(&prefix).unwrap().to_string(), Asset::get(a.as_ref()).unwrap().data);
                });

            Post {
                path,
                title: parsed_title,
                date: parsed_date,
                description: parsed_description,
                pre_rendered: content,
                assets,
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

    let index_page = pre_render_index(&posts);

    SharedState {
        posts,
        post_index,
        pre_rendered_index: index_page,
        pre_rendered_not_found: pre_render_not_found(),
    }
}

fn pre_render_head(title: &String) -> PreEscaped<String> {
    html! {
        head {
            title { (title) }
            link rel="shortcut icon" href="data:image/x-icon;," type="image/x-icon";
            meta name="author" content="Ben Meier";
            meta name="author" content="astromechza";
            meta name="description" content="Technical blog of Ben Meier";
            meta name="keywords" content="golang, rust, distributed systems, programming, security";
            meta name="viewport" content="width=device-width, initial-scale=1.0";

            style {
                (NORMALIZE_CSS.0)
                (MILLIGRAM_CSS.0)
            }
        }
    }.clone()
}

fn pre_render_footer() -> PreEscaped<String> {
    let now = chrono::Utc::now().format("%Y").to_string();
    html! {
        footer.row {
            section.column {
                hr {}
                p {
                    "© Ben Meier " (now)
                    br;
                    small {
                        "This blog is a single Rust binary with all assets embedded and pre-rendered. "
                        "If you're interested in how it's built, have a look at the code on my Github."
                    }
                }
            }
        }
    }
}

fn pre_render_index(posts: &Vec<Post>) -> Cow<'static, str> {
    html! {
        (DOCTYPE)
        html lang="en" {
            (pre_render_head(&"Technical blog of Ben Meier".to_string()))
            body {
                div.container style="margin-top: 2em" {
                    header.row {
                        section.column {
                            h1 { "Technical blog of Ben Meier" }
                            "I'm a software engineer working mostly on distributed systems with an interest in security, networking, correctness, and chaos. "
                            br;
                            "All opinions expressed here are my own. "
                            br;
                            br;
                            ul style="display: inline-flex; margin: 0" {
                                li style="margin-right: 1em;" {
                                    "Mastodon: "
                                    a href="https://hachyderm.io/@benmeier_" {
                                        "@benmeier_@hachyderm.io"
                                    }
                                }
                                li {
                                    "Github: "
                                    a href="https://github.com/astromechza" {
                                        "astromechza"
                                    }
                                }
                            }
                            hr {}
                        }
                    }
                    main.row {
                        section.column {
                            h2 {
                                "Posts"
                            }
                            nav {
                                ul style="margin: 0" {
                                    @for x in posts.iter() {
                                        li {
                                            p style="display: inline-grid;" {
                                                a href={ (x.path) "/" } {
                                                    time { (x.date.format("%e %B %Y").to_string()) }
                                                    (": ") (x.title)
                                                }
                                                @if x.description.is_some() {
                                                    br;
                                                    (x.description.clone().unwrap())
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    (pre_render_footer())
                }
            }
        }
    }.into_string().into()
}

fn pre_render_post(title: &String, time: &DateTime<FixedOffset>, description: &Option<String>, content: &PreEscaped<String>) -> Cow<'static, str> {
    html! {
        (DOCTYPE)
        html lang="en" {
            (pre_render_head(title))
            body {
                div.container style="margin-top: 2em" {
                    header.row {
                        section.column {
                            h1 { (title) }
                            p {
                                (time.format("%e %B %Y").to_string())
                                @match description {
                                    Some(d) => {
                                        br;
                                        (d)
                                    },
                                    _ => {}
                                }
                            }
                            a href="../" {
                                "< Back to the index"
                            }
                            hr {}
                        }
                    }
                    main.row {
                        section.column {
                            article {
                                (content)
                            }
                        }
                    }
                    (pre_render_footer())
                }
            }
        }
    }.into_string().clone().into()
}

fn pre_render_not_found() -> Cow<'static, str> {
    html! {
        (DOCTYPE)
        html lang="en" {
            (pre_render_head(&"Not found".to_string()))
            body {
                div.container style="margin-top: 2em" {
                    header.row {
                        section.column {
                            h1 { ("Post not found") }
                            a href="../" {
                                "< Back to the index"
                            }
                            hr {}
                        }
                    }
                    main.row {
                        section.column {
                            p {
                                "Looks like the incoming link is wrong, corrupted, or the post has been removed. Sorry about that."
                            }
                        }
                    }
                    (pre_render_footer())
                }
            }
        }
    }.into_string().into()
}

async fn list_posts(state: State<Arc<SharedState>>) -> (HeaderMap<HeaderValue>, Cow<'static, str>) {
    let mut headers = HeaderMap::new();
    headers.insert(http::header::CONTENT_TYPE, HeaderValue::from_str(HTML_CONTENT_TYPE).unwrap());
    (headers, state.pre_rendered_index.clone())
}

async fn view_post(Path(post_key): Path<String>, headers: HeaderMap, state: State<Arc<SharedState>>) -> Response {
    state.post_index.get(post_key.as_str())
        .map(|i| state.posts.get(*i).unwrap())
        .map(|p| {
            let mut headers = HeaderMap::new();
            headers.insert(http::header::CONTENT_TYPE, HeaderValue::from_str(HTML_CONTENT_TYPE).unwrap());
            (StatusCode::OK, headers, p.pre_rendered.clone()).into_response()
        })
        .unwrap_or_else(|| { gen_not_found(state, headers) })
}

fn gen_not_found(state: State<Arc<SharedState>>, headers: HeaderMap) -> Response {
    let provide_html = headers.get("accept")
        .map(|v| v.to_str().unwrap().contains("text/html"))
        .unwrap_or(false);

    if provide_html {
        let mut headers = HeaderMap::new();
        headers.insert(http::header::CONTENT_TYPE, HeaderValue::from_str(HTML_CONTENT_TYPE).unwrap());
        (StatusCode::NOT_FOUND, headers, state.pre_rendered_not_found.clone()).into_response()
    } else {
        StatusCode::NOT_FOUND.into_response()
    }
}

async fn not_found(state: State<Arc<SharedState>>, headers: HeaderMap) -> Response {
    gen_not_found(state, headers)
}

async fn view_asset(Path(key): Path<(String, String)>, state: State<Arc<SharedState>>, headers: HeaderMap) -> Response {
    state.post_index.get(key.0.as_str())
        .map(|i| state.posts.get(*i).unwrap())
        .filter(|p| p.assets.contains_key(key.1.as_str()))
        .map(|p| p.assets.get(key.1.as_str()).unwrap())
        .map(|a| {
            let mut resp = a.to_owned().into_response();
            if let Some(val) = mime_guess::from_path(key.1.as_str()).first_raw() {
                resp.headers_mut().insert(http::header::CONTENT_TYPE, HeaderValue::from_str(val).unwrap());
            }
            resp
        })
        .unwrap_or_else(|| { gen_not_found(state, headers) })
}

fn setup_router() -> Router {
    let state = Arc::new(build_shared_state(collect_posts()));
    Router::new()
        .route("/", get(list_posts))
        .route("/:post_key/:asset_key", get(view_asset))
        .route("/:post_key/", get(view_post))
        .fallback(not_found)
        .with_state(state)
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(
                    tower_http::trace::DefaultMakeSpan::new().level(tracing::Level::INFO),
                )
                .on_response(
                    tower_http::trace::DefaultOnResponse::new().level(tracing::Level::INFO),
                ),
        )
}

#[tokio::main]
async fn main() {
    let args = Cli::parse();
    tracing_subscriber::fmt::init();
    let addr = SocketAddr::from((
        args.bind_address.unwrap_or([127, 0, 0, 1].into()),
        args.bind_port.unwrap_or(8080),
    ));

    let app = setup_router().into_make_service();
    let svr = axum::Server::bind(&addr).serve(app);

    tracing::info!("server is listening on {}...", svr.local_addr());
    if let Err(err) = svr.await {
        tracing::error!("server error: {}", err);
    }
}

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use axum::http::{HeaderValue, Method, Request, StatusCode};
    use axum::http::header::{ACCEPT, CONTENT_LENGTH, CONTENT_TYPE};
    use crate::{Asset, CONTENT_FILE_NAME, setup_router};
    use tower::ServiceExt; // for `oneshot` and `ready`
    use test_case::test_case;

    #[tokio::test]
    async fn test_index() {
        let app = setup_router();
        let resp = app.oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(resp.headers().get(CONTENT_TYPE).unwrap(), "text/html; charset=utf-8");
        let length: u32 = resp.headers().get(CONTENT_LENGTH).unwrap().to_str().unwrap().parse().unwrap();
        assert!(length > 1);
    }

    #[test_case("/a" ; "plain/a")]
    #[test_case("/a/" ; "plain/a/")]
    #[test_case("/a/b" ; "plain/a/b")]
    #[test_case("/a/b/" ; "plain/a/b/")]
    #[test_case("/a/b/c" ; "plain/a/b/c")]
    #[tokio::test]
    async fn test_plain_404(uri: &str) {
        let app = setup_router();
        let resp = app.oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap()).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
        assert_eq!(resp.headers().get(CONTENT_LENGTH).unwrap(), "0");
        assert!(resp.headers().get(CONTENT_TYPE).is_none());
    }

    #[test_case("/a" ; "html/a")]
    #[test_case("/a/" ; "html/a/")]
    #[test_case("/a/b" ; "html/a/b")]
    #[test_case("/a/b/" ; "html/a/b/")]
    #[test_case("/a/b/c" ; "html/a/b/c")]
    #[tokio::test]
    async fn test_html_404(uri: &str) {
        let app = setup_router();
        let mut req = Request::builder().uri(uri);
        req.headers_mut().unwrap().insert(ACCEPT, HeaderValue::from_str("text/html").unwrap());
        let resp = app.oneshot(req.body(Body::empty()).unwrap()).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
        assert_eq!(resp.headers().get(CONTENT_TYPE).unwrap(), "text/html; charset=utf-8");
        let length: u32 = resp.headers().get(CONTENT_LENGTH).unwrap().to_str().unwrap().parse().unwrap();
        assert!(length > 1);
    }

    #[test_case("/a", 404 ; "post/a")]
    #[test_case("/a/", 405 ; "post/a/")]
    #[test_case("/a/b", 405 ; "post/a/b")]
    #[test_case("/a/b/", 404 ; "post/a/b/")]
    #[test_case("/a/b/c", 404 ; "post/a/b/c")]
    #[tokio::test]
    async fn test_post(uri: &str, code: u16) {
        let app = setup_router();
        let resp = app.oneshot(Request::builder().method(Method::POST).uri(uri).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(resp.status().as_u16(), code);
    }

    #[tokio::test]
    async fn test_posts() {

        let blogs: Vec<String> = Asset::iter()
            .filter(|p| p.contains(CONTENT_FILE_NAME))
            .map(|p| p.rsplitn(3, "/").skip(1).take(1).last().unwrap().to_owned().to_string())
            .collect();

        for x in blogs {
            println!("checking {}", x);
            let app = setup_router();
            let resp = app.oneshot(Request::builder().uri(format!("/{}/", x)).body(Body::empty()).unwrap())
                .await
                .unwrap();
            assert_eq!(resp.status(), StatusCode::OK);
        }
    }

}
