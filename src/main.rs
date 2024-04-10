use std::borrow::Cow;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hasher;
use std::net::{IpAddr, SocketAddr};
use std::ops::Add;
use std::str::from_utf8;
use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::{http, routing::get, Router};
use clap::{crate_version, Parser};
use deflate::deflate_bytes;
use hyper::Request;
use lazy_static::lazy_static;
use maud::{html, Markup, PreEscaped, DOCTYPE};
use opentelemetry::trace::{
    Link, SamplingDecision, SamplingResult, SpanKind, TraceContextExt, TraceId, TraceState,
};
use opentelemetry::{Context, KeyValue};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::trace::ShouldSample;
use rust_embed::RustEmbed;
use time::format_description::FormatItem;
use time::macros::{format_description, time};
use time::{Date, OffsetDateTime, PrimitiveDateTime};
use tower_http::trace::{MakeSpan, TraceLayer};
use tracing::Span;
use tracing_subscriber;
use tracing_subscriber::layer::SubscriberExt;

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

    #[arg(long)]
    external_url_prefix: Option<String>,
}

#[derive(Clone, Debug)]
struct Item {
    content: Cow<'static, [u8]>,
    compressed: Cow<'static, [u8]>,
    content_type: HeaderValue,
    etag: String,
    children: HashMap<String, Cow<'static, Item>>,
}

struct Post {
    path: String,
    title: String,
    date: PrimitiveDateTime,
    pre_rendered: Cow<'static, [u8]>,
    assets: HashMap<String, Cow<'static, [u8]>>,
}

struct SharedState {
    root: Cow<'static, Item>,
    not_found: Cow<'static, Item>,
}

const CONTENT_FILE_NAME: &str = "content.md";
const HTML_CONTENT_TYPE: &str = "text/html; charset=utf-8";
const PLAIN_CONTENT_TYPE: &str = "text/plain; charset=utf-8";
const CRATE_VERSION: &str = crate_version!();
const CACHE_CONTROL: &str = "max-age=300";

const POST_DATE_FORMAT: &[FormatItem] =
    format_description!("[day padding:none] [month repr:long] [year]");
const RFC3339_DATE_FORMAT: &[FormatItem] =
    format_description!("[year]-[month]-[day]T[hour]:[minute]:[second]Z");
const RFC2822_DATE_FORMAT: &[FormatItem] = format_description!(
    "[weekday repr:short], [day] [month repr:short] [year] [hour]:[minute]:[second] GMT"
);
const FOOTER_DATE_FORMAT: &[FormatItem] = RFC3339_DATE_FORMAT;
const ENCODED_FAVICON: &str = "data:image/svg+xml,%3Csvg version='1.0' xmlns='http://www.w3.org/2000/svg' xmlns:xlink='http://www.w3.org/1999/xlink' viewBox='0 0 64 64' enable-background='new 0 0 64 64' xml:space='preserve'%3E%3Cg%3E%3Cg%3E%3Cpolygon fill='%23F9EBB2' points='46,3.414 46,14 56.586,14 '/%3E%3Cpath fill='%23F9EBB2' d='M45,16c-0.553,0-1-0.447-1-1V2H8C6.896,2,6,2.896,6,4v56c0,1.104,0.896,2,2,2h48c1.104,0,2-0.896,2-2V16 H45z'/%3E%3C/g%3E%3Cpath fill='%23394240' d='M14,26c0,0.553,0.447,1,1,1h34c0.553,0,1-0.447,1-1s-0.447-1-1-1H15C14.447,25,14,25.447,14,26z'/%3E%3Cpath fill='%23394240' d='M49,37H15c-0.553,0-1,0.447-1,1s0.447,1,1,1h34c0.553,0,1-0.447,1-1S49.553,37,49,37z'/%3E%3Cpath fill='%23394240' d='M49,43H15c-0.553,0-1,0.447-1,1s0.447,1,1,1h34c0.553,0,1-0.447,1-1S49.553,43,49,43z'/%3E%3Cpath fill='%23394240' d='M49,49H15c-0.553,0-1,0.447-1,1s0.447,1,1,1h34c0.553,0,1-0.447,1-1S49.553,49,49,49z'/%3E%3Cpath fill='%23394240' d='M49,31H15c-0.553,0-1,0.447-1,1s0.447,1,1,1h34c0.553,0,1-0.447,1-1S49.553,31,49,31z'/%3E%3Cpath fill='%23394240' d='M15,20h16c0.553,0,1-0.447,1-1s-0.447-1-1-1H15c-0.553,0-1,0.447-1,1S14.447,20,15,20z'/%3E%3Cpath fill='%23394240' d='M59.706,14.292L45.708,0.294C45.527,0.112,45.277,0,45,0H8C5.789,0,4,1.789,4,4v56c0,2.211,1.789,4,4,4h48 c2.211,0,4-1.789,4-4V15C60,14.723,59.888,14.473,59.706,14.292z M46,3.414L56.586,14H46V3.414z M58,60c0,1.104-0.896,2-2,2H8 c-1.104,0-2-0.896-2-2V4c0-1.104,0.896-2,2-2h36v13c0,0.553,0.447,1,1,1h13V60z'/%3E%3Cpolygon opacity='0.15' fill='%23231F20' points='46,3.414 56.586,14 46,14 '/%3E%3C/g%3E%3C/svg%3E";

lazy_static! {
    static ref START_TIME: std::time::Instant = std::time::Instant::now();
}

fn collect_posts(external_url_prefix: &String) -> Vec<Post> {
    let mut options = pulldown_cmark::Options::empty();
    options.insert(pulldown_cmark::Options::ENABLE_STRIKETHROUGH);
    options.insert(pulldown_cmark::Options::ENABLE_TABLES);
    options.insert(pulldown_cmark::Options::ENABLE_FOOTNOTES);

    let title_re = regex::Regex::new(r#"<meta x-title="(.+)"/?>"#).unwrap();

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

            let format = format_description!("[year][month][day]");
            let parsed_date = path
                .split("-")
                .take(1)
                .last()
                .map(|c| Date::parse(c, &format).unwrap())
                .unwrap_or(OffsetDateTime::now_utc().date());
            let parsed_date_time = PrimitiveDateTime::new(parsed_date, time!(0:00));

            let parser = pulldown_cmark::Parser::new_ext(raw_content, options);
            let mut html_output = String::new();
            pulldown_cmark::html::push_html(&mut html_output, parser);
            let tree: Markup = PreEscaped { 0: html_output };

            let content = pre_render_post(
                &parsed_title,
                &parsed_date_time,
                &tree,
                &external_url_prefix,
                &path,
            );

            let mut assets = HashMap::new();

            let prefix = x
                .rsplitn(2, "/")
                .skip(1)
                .last()
                .unwrap()
                .to_string()
                .add("/");

            Asset::iter()
                .filter(|a| a.contains(&prefix))
                .filter(|a| !a.contains(CONTENT_FILE_NAME))
                .for_each(|a| {
                    assets.insert(
                        a.strip_prefix(&prefix).unwrap().to_string(),
                        Asset::get(a.as_ref()).unwrap().data,
                    );
                });

            Post {
                path,
                title: parsed_title,
                date: parsed_date_time,
                pre_rendered: content,
                assets,
            }
        })
        .collect::<Vec<Post>>()
}

fn build_shared_state(mut posts: Vec<Post>, external_url_prefix: &String) -> SharedState {
    posts.reverse();
    tracing::info!("Building shared state from {} posts", posts.len());

    let root_content = pre_render_index(&posts, external_url_prefix);
    let mut root: Cow<'static, Item> = Cow::Owned(Item {
        content: root_content.clone(),
        compressed: Cow::from(deflate_bytes(root_content.as_ref())),
        content_type: HeaderValue::from_str(HTML_CONTENT_TYPE).unwrap(),
        etag: make_hash("", "").to_string(),
        children: HashMap::new(),
    });

    let url_image_data = Asset::get("url-image.jpg").unwrap().data;
    let url_image_item = Cow::Owned(Item {
        content: url_image_data.clone(),
        compressed: Cow::from(deflate_bytes(url_image_data.as_ref())),
        content_type: HeaderValue::from_str("image/jpeg").unwrap(),
        etag: make_hash("url-image.jpg", "").to_string(),
        children: HashMap::new(),
    });
    root.to_mut()
        .children
        .insert("url-image.jpg".to_string(), url_image_item);

    for x in &posts {
        let mut post_item: Cow<'static, Item> = Cow::Owned(Item {
            content: x.pre_rendered.clone(),
            compressed: Cow::from(deflate_bytes(x.pre_rendered.as_ref())),
            content_type: HeaderValue::from_str(HTML_CONTENT_TYPE).unwrap(),
            etag: make_hash(x.title.as_str(), "").to_string(),
            children: HashMap::new(),
        });

        for y in x.assets.clone() {
            let asset_item = Cow::Owned(Item {
                content: y.1.clone(),
                compressed: Cow::from(deflate_bytes(y.1.as_ref())),
                content_type: HeaderValue::from_str(
                    mime_guess::from_path(y.0.as_str())
                        .first_or_text_plain()
                        .to_string()
                        .as_str(),
                )
                .unwrap(),
                etag: make_hash(x.title.as_str(), y.0.as_str()).to_string(),
                children: HashMap::new(),
            });
            post_item.to_mut().children.insert(y.0.clone(), asset_item);
        }

        root.to_mut().children.insert(x.path.clone(), post_item);
    }

    {
        let robots_content = Cow::from(
            "User-agent: *\nAllow: /\nDisallow: /livez\nDisallow: /readyz\nDisallow: /metricz\n"
                .as_bytes()
                .to_owned(),
        );
        let robots = Cow::Owned(Item {
            content: robots_content.clone(),
            compressed: Cow::from(deflate_bytes(robots_content.as_ref())),
            content_type: HeaderValue::from_str(PLAIN_CONTENT_TYPE).unwrap(),
            etag: make_hash_of_bytes(robots_content.clone()).to_string(),
            children: HashMap::new(),
        });
        root.to_mut()
            .children
            .insert("robots.txt".to_string(), robots);
    }

    {
        let rss_content = pre_render_rss(&posts, &external_url_prefix);
        let rss = Cow::Owned(Item {
            content: rss_content.clone(),
            compressed: Cow::from(deflate_bytes(rss_content.as_ref())),
            content_type: HeaderValue::from_str("text/xml").unwrap(),
            etag: make_hash_of_bytes(rss_content.clone()).to_string(),
            children: HashMap::new(),
        });
        root.to_mut().children.insert("rss.xml".to_string(), rss);

        let feed = Cow::Owned(Item {
            content: rss_content.clone(),
            compressed: Cow::from(deflate_bytes(rss_content.as_ref())),
            content_type: HeaderValue::from_str("text/xml").unwrap(),
            etag: make_hash_of_bytes(rss_content.clone()).to_string(),
            children: HashMap::new(),
        });
        root.to_mut().children.insert("feed.xml".to_string(), feed);
    }

    let not_found_content = pre_render_not_found();
    let not_found = Cow::Owned(Item {
        content: not_found_content.clone(),
        compressed: Cow::from(deflate_bytes(not_found_content.as_ref())),
        content_type: HeaderValue::from_str(HTML_CONTENT_TYPE).unwrap(),
        etag: make_hash("", "").to_string(),
        children: HashMap::new(),
    });

    SharedState { root, not_found }
}

fn pre_render_head() -> PreEscaped<String> {
    let css1 = from_utf8(Asset::get("normalize.css").unwrap().data.as_ref())
        .unwrap()
        .to_owned();
    let css2 = from_utf8(Asset::get("milligram.css").unwrap().data.as_ref())
        .unwrap()
        .to_owned();
    let tree = html! {
        link rel="shortcut icon" href=(ENCODED_FAVICON) type="image/svg+xml";
        link rel="me" href="https://hachyderm.io/@benmeier_";
        link rel="alternate" href="/feed.xml" type="application/rss+xml" title="RSS feed";
        meta charset="utf-8";
        meta name="author" content="Ben Meier";
        meta name="keywords" content="golang, rust, distributed systems, programming, security";
        meta name="viewport" content="width=device-width, initial-scale=1.0";
        style nonce="123456789" {
            (css1)
            (css2)
            "pre code { display: block; white-space: pre-wrap; } "
            "ul { list-style: circle outside; } "
            "ul li { margin-left: 1em; } "
            ".index-nav-ul { margin: 0; list-style: circle outside; } "
            "body { background-color: #fdfae9; }"
            ".footnote-definition { margin-bottom: 2em; }"
            ".footnote-definition p { display: inline; }"
            "header.row { justify-content: space-between; }"
            "header.row section.column { max-width: fit-content; }"
        }
    };
    tree.clone()
}

fn pre_render_footer() -> PreEscaped<String> {
    let now = OffsetDateTime::now_utc();
    let pid = std::process::id();
    let name = clap::crate_name!();
    html! {
        footer.row {
            section.column {
                hr {}
                p {
                    "© Ben Meier " (now.year())
                    br;
                    small {
                        "This blog is a single Rust binary with all assets embedded and pre-rendered. "
                        "If you're interested in how it's built, have a look at the code on "
                        a href="https://github.com/astromechza/binary-blog" {
                            "Github"
                        }
                        "."
                    }
                    br;
                    small {
                        "name=" (name) " version=" (CRATE_VERSION)
                        " pid=" (pid) " start-time=" (now.format(&FOOTER_DATE_FORMAT).unwrap().to_string())
                    }
                }
            }
        }
    }
}

fn pre_render_index(posts: &Vec<Post>, external_url_prefix: &String) -> Cow<'static, [u8]> {
    let tree = html! {
        (DOCTYPE)
        html lang="en" {
            head {
                title { "Ben's Blog" }
                meta name="description" content="Technical blog of Ben Meier";
                meta property="og:type" content="website";
                meta property="og:title" content="Ben's Blog";
                meta property="og:description" content="Technical blog of Ben Meier";
                meta property="og:url" content={ (external_url_prefix) "/" };
                meta property="og:image" content={ (external_url_prefix) "/url-image.jpg" };
                (pre_render_head())
            }
            body {
                div.container {
                    header.row {
                        section class="column" {
                            h1 { "Ben Meier" }
                        }
                        section class="column" {
                            "Mastodon: "
                            a href="https://hachyderm.io/@benmeier_" {
                                "@benmeier_@hachyderm.io"
                            }
                            " | Github: "
                            a href="https://github.com/astromechza" {
                                "astromechza"
                            }
                            " | rss: "
                            a href="/feed.xml" target="_blank" {
                                "feed.xml"
                            }
                            " | "
                            a href="/" {
                                "All Posts"
                            }
                        }
                    }
                    main.row {
                        section.column {
                            small {
                                p {
                                    "I'm a software engineer working mostly on distributed systems with an interest in security, networking, correctness, and chaos. "
                                    "All opinions expressed here are my own. "
                                    "This blog contains a wide range of content accrued over time and from multiple previous attempts at technical blogging over the course of my career. "
                                    "I intentionally don't go back and improve or rewrite old posts, so please take old content with a pinch of salt, and I apologise for any broken links!"
                                }
                            }
                            hr {}
                            nav {
                                (PreEscaped("<ul class=\"index-nav-ul\">"))
                                @let mut last_year = 0;
                                @for x in posts.iter() {
                                    @if x.date.year() != last_year {
                                        (PreEscaped("</ul>"))
                                        h2 {
                                            ({
                                                last_year = x.date.year();
                                                last_year
                                            })
                                        }
                                        (PreEscaped("<ul class=\"index-nav-ul\">"))
                                    }
                                    li {
                                        p {
                                            a href={ (x.path) "/" } {
                                                time datetime=(x.date.format(&RFC3339_DATE_FORMAT).unwrap().to_string()) { (x.date.format(&POST_DATE_FORMAT).unwrap().to_string()) }
                                                (": ") (x.title)
                                            }
                                        }
                                    }
                                }
                                (PreEscaped("</ul>"))
                            }
                        }
                    }
                    (pre_render_footer())
                }
            }
        }
    };
    Cow::from(tree.into_string().as_bytes().to_owned()).to_owned()
}

fn pre_render_rss(posts: &Vec<Post>, external_url_prefix: &String) -> Cow<'static, [u8]> {
    let tree = html! {
        (PreEscaped("<?xml version=\"1.0\" encoding=\"UTF-8\" ?>"))
        rss version="2.0" xmlns:atom="http://www.w3.org/2005/Atom" {
            channel {
                title { "Ben Meier" }
                link { (external_url_prefix) "/" }
                (PreEscaped("<atom:link rel=\"self\" href=\""))
                (PreEscaped(external_url_prefix))
                (PreEscaped("/rss.xml\" />"))
                language { "en" }
                description { "I'm a software engineer working mostly on distributed systems with an interest in security, networking, correctness, and chaos." }
                @for x in posts.iter() {
                    item {
                        title { (x.title) }
                        link { (external_url_prefix) "/" (x.path) "/" }
                        guid { (external_url_prefix) "/" (x.path) "/" }
                        pubDate { (x.date.format(&RFC2822_DATE_FORMAT).unwrap().to_string()) }
                        category { "IT/Technical" }
                    }
                }
            }
        }
    };
    Cow::from(tree.into_string().as_bytes().to_owned()).to_owned()
}

fn pre_render_post(
    title: &String,
    time: &PrimitiveDateTime,
    content: &PreEscaped<String>,
    external_url_prefix: &String,
    path: &String,
) -> Cow<'static, [u8]> {
    let tree = html! {
        (DOCTYPE)
        html lang="en" {
            head {
                title { (title) }
                meta name="description" content=(title);
                meta property="og:type" content="article";
                meta property="og:title" content=(title);
                meta property="og:description" content={ (time.format(&POST_DATE_FORMAT).unwrap().to_string()) " - " (title) };
                meta property="og:url" content={ (external_url_prefix) "/" (path) "/" };
                meta property="og:image" content={ (external_url_prefix) "/url-image.jpg" };
                meta property="article:author" content="Ben Meier";
                meta property="article:published_time" content=(time.format(&RFC3339_DATE_FORMAT).unwrap().to_string());
                (pre_render_head())
            }
            body {
                div.container {
                    header.row {
                        section class="column" {
                            h1 { (title) }
                        }
                        section class="column" {
                            a href="/" {
                                "All Posts"
                            }
                        }
                    }
                    main.row {
                        section.column {
                            small {
                                "Ben Meier - "
                                time datetime=(time.format(&RFC3339_DATE_FORMAT).unwrap().to_string()) { (time.format(&POST_DATE_FORMAT).unwrap().to_string()) }
                            }
                            hr {}
                            article {
                                (content)
                            }
                        }
                    }
                    (pre_render_footer())
                }
            }
        }
    };
    Cow::from(tree.into_string().as_bytes().to_owned()).to_owned()
}

fn pre_render_not_found() -> Cow<'static, [u8]> {
    let tree = html! {
        (DOCTYPE)
        html lang="en" {
            head {
                title { "404 - Not Found" }
                (pre_render_head())
            }
            body {
                div.container {
                    header.row {
                        section class="column" {
                            h1 { "Not found" }
                        }
                        section class="column" {
                            a href="/" {
                                "All Posts"
                            }
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
    };
    Cow::from(tree.into_string().as_bytes().to_owned()).to_owned()
}

fn make_hash_of_bytes(x: Cow<'static, [u8]>) -> u64 {
    let mut hasher = DefaultHasher::new();
    hasher.write(CRATE_VERSION.as_bytes());
    hasher.write(x.as_ref());
    hasher.finish()
}

fn make_hash(x: &str, y: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    hasher.write(CRATE_VERSION.as_bytes());
    hasher.write(x.as_bytes());
    hasher.write(y.as_bytes());
    hasher.finish()
}

fn check_etag_and_return(
    etag: String,
    req_headers: &HeaderMap,
    resp_headers: &HeaderMap,
) -> Option<Response> {
    if req_headers
        .get(http::header::IF_NONE_MATCH)
        .map(|v| v.to_str().unwrap().eq(etag.as_str()))
        .unwrap_or(false)
    {
        return Some((StatusCode::NOT_MODIFIED, resp_headers.clone()).into_response());
    }
    None
}

fn gen_not_found(state: State<Arc<SharedState>>, req_headers: HeaderMap) -> Response {
    let provide_html = req_headers
        .get("accept")
        .map(|v| v.to_str().unwrap().contains("text/html"))
        .unwrap_or(false);

    if provide_html {
        let mut headers = HeaderMap::new();
        headers.insert(
            http::header::CONTENT_TYPE,
            state.not_found.content_type.clone(),
        );
        headers.insert(
            http::header::CACHE_CONTROL,
            HeaderValue::from_str(CACHE_CONTROL).unwrap(),
        );
        (
            StatusCode::NOT_FOUND,
            headers,
            state.not_found.content.clone(),
        )
            .into_response()
    } else {
        StatusCode::NOT_FOUND.into_response()
    }
}

async fn not_found(state: State<Arc<SharedState>>, headers: HeaderMap) -> Response {
    gen_not_found(state, headers)
}

async fn view_root_item(
    state: State<Arc<SharedState>>,
    req_headers: HeaderMap,
    req: Request<axum::body::Body>,
) -> Response {
    view_nested_item(
        Path(("".to_string(), "".to_string())),
        state,
        req_headers,
        req,
    )
    .await
}

async fn view_item(
    Path(key): Path<String>,
    state: State<Arc<SharedState>>,
    req_headers: HeaderMap,
    req: Request<axum::body::Body>,
) -> Response {
    view_nested_item(Path((key, "".to_string())), state, req_headers, req).await
}

async fn view_nested_item(
    Path(key): Path<(String, String)>,
    state: State<Arc<SharedState>>,
    req_headers: HeaderMap,
    req: Request<axum::body::Body>,
) -> Response {
    let mut x = state.root.clone();
    if !key.0.is_empty() {
        if let Some(y) = x.children.get(key.0.as_str()) {
            x = y.clone();
            if !key.1.is_empty() {
                if let Some(z) = x.children.get(key.1.as_str()) {
                    x = z.clone()
                } else {
                    return gen_not_found(state, req_headers);
                }
            }

            // if we are loading a root item that doesn't end in slash and is html, lets redirect to the slash path.
            if !req.uri().path().ends_with('/')
                && x.content_type
                    .to_str()
                    .map(|s| s.eq(HTML_CONTENT_TYPE))
                    .unwrap_or_default()
            {
                let mut headers = HeaderMap::new();
                let mut newpath = req.uri().path().to_owned();
                newpath.push('/');
                headers.insert(
                    http::header::LOCATION,
                    HeaderValue::from_str(newpath.as_str()).unwrap(),
                );
                headers.insert(
                    http::header::CACHE_CONTROL,
                    HeaderValue::from_str(CACHE_CONTROL).unwrap(),
                );
                return (StatusCode::TEMPORARY_REDIRECT, headers.clone()).into_response();
            }
        } else {
            return gen_not_found(state, req_headers);
        }
    }

    let mut headers = HeaderMap::new();
    headers.insert(http::header::CONTENT_TYPE, x.content_type.clone());
    headers.insert(
        http::header::ETAG,
        HeaderValue::from_str(x.etag.as_str()).unwrap(),
    );
    headers.insert(
        http::header::CACHE_CONTROL,
        HeaderValue::from_str(CACHE_CONTROL).unwrap(),
    );
    headers.insert(
        http::header::X_FRAME_OPTIONS,
        HeaderValue::from_str("DENY").unwrap(),
    );
    headers.insert(
        http::header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_str(
            "default-src 'none'; style-src 'nonce-123456789'; img-src 'self' data: https:",
        )
        .unwrap(),
    );
    headers.insert(
        http::header::REFERRER_POLICY,
        HeaderValue::from_str("origin-when-cross-origin").unwrap(),
    );
    headers.insert(
        http::header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_str("nosniff").unwrap(),
    );

    if let Some(not_modified) = check_etag_and_return(x.etag.clone(), &req_headers, &headers) {
        return not_modified;
    }

    let ae_header = req_headers.get(http::header::ACCEPT_ENCODING);
    if ae_header.is_some() && ae_header.unwrap().to_str().unwrap().contains("deflate") {
        headers.insert(
            http::header::CONTENT_ENCODING,
            HeaderValue::from_str("deflate").unwrap(),
        );
        return (StatusCode::OK, headers, x.compressed.clone()).into_response();
    }

    (StatusCode::OK, headers, x.content.clone()).into_response()
}

async fn healthcheck() -> Response {
    (StatusCode::NO_CONTENT).into_response()
}

#[derive(Default, Clone)]
struct HttpTraceLayerHooks;

impl<B> MakeSpan<B> for HttpTraceLayerHooks {
    fn make_span(&mut self, req: &Request<B>) -> Span {
        // see https://github.com/open-telemetry/semantic-conventions/blob/main/docs/http/http-spans.md
        let span = tracing::info_span!(
            // span name here will be ignored by open telemetry and replaced with otel.name
            "request",
            // the span name
            otel.name = tracing::field::Empty,
            otel.kind = "server",
            otel.status_code = tracing::field::Empty,
            // common attributes
            http.response.status_code = tracing::field::Empty,
            http.request.body.size = tracing::field::Empty,
            http.response.body.size = tracing::field::Empty,
            http.request.method = req.method().as_str(),
            network.protocol.name = "http",
            network.protocol.version = format!("{:?}", req.version()).strip_prefix("HTTP/"),
            user_agent.original = tracing::field::Empty,
            // http request and response headers
            http.request.header.x_forwarded_for = tracing::field::Empty,
            http.request.header.cf_ipcountry = tracing::field::Empty,
            http.request.header.referer = tracing::field::Empty,
            // http server
            http.route = tracing::field::Empty,
            server.address = tracing::field::Empty,
            server.port = tracing::field::Empty,
            url.path = req.uri().path(),
            url.query = tracing::field::Empty,
            url.scheme = tracing::field::Empty,
            // set on the failure hook
            "error.type" = tracing::field::Empty,
            error = tracing::field::Empty,
        );

        req.uri().query().map(|v| span.record("url.query", v));
        req.uri()
            .scheme()
            .map(|v| span.record("url.scheme", v.as_str()));
        req.uri().host().map(|v| span.record("server.address", v));
        req.uri().port_u16().map(|v| span.record("server.port", v));

        req.headers()
            .get(http::header::CONTENT_LENGTH)
            .map(|v| v.to_str().map(|v| span.record("http.request.body.size", v)));
        req.headers()
            .get(http::header::USER_AGENT)
            .map(|v| v.to_str().map(|v| span.record("user_agent.original", v)));
        req.headers().get("X-Forwarded-For").map(|v| {
            v.to_str()
                .map(|v| span.record("http.request.header.x_forwarded_for", v))
        });
        req.headers().get("CF-IPCountry").map(|v| {
            v.to_str()
                .map(|v| span.record("http.request.header.cf_ipcountry", v))
        });
        req.headers().get("Referer").map(|v| {
            v.to_str()
                .map(|v| span.record("http.request.header.referer", v))
        });

        if let Some(path) = req.extensions().get::<axum::extract::MatchedPath>() {
            span.record("otel.name", format!("{} {}", req.method(), path.as_str()));
            span.record("http.route", path.as_str());
        } else {
            span.record("otel.name", format!("{} -", req.method()));
        };

        span
    }
}

impl<B> tower_http::trace::OnRequest<B> for HttpTraceLayerHooks {
    fn on_request(&mut self, _: &Request<B>, _: &Span) {
        tracing::event!(tracing::Level::DEBUG, "start processing request");
    }
}

impl<B> tower_http::trace::OnResponse<B> for HttpTraceLayerHooks {
    fn on_response(self, response: &Response<B>, _: std::time::Duration, span: &Span) {
        if let Some(size) = response.headers().get(http::header::CONTENT_LENGTH) {
            span.record("http.response.body.size", size.to_str().unwrap());
        }
        span.record("http.response.status_code", response.status().as_u16());

        // Server errors are handled by the OnFailure hook
        if !response.status().is_server_error() {
            tracing::event!(tracing::Level::INFO, "finished processing request");
        }
    }
}

impl<FailureClass> tower_http::trace::OnFailure<FailureClass> for HttpTraceLayerHooks
where
    FailureClass: std::fmt::Display,
{
    fn on_failure(&mut self, error: FailureClass, _: std::time::Duration, _: &Span) {
        tracing::event!(
            tracing::Level::ERROR,
            error = error.to_string(),
            "error.type" = "_OTHER",
            "response failed"
        );
    }
}

fn setup_router(external_url_prefix: String) -> Router {
    let state = Arc::new(build_shared_state(
        collect_posts(&external_url_prefix),
        &external_url_prefix,
    ));
    let trace_layer = TraceLayer::new_for_http()
        .make_span_with(HttpTraceLayerHooks::default())
        .on_request(HttpTraceLayerHooks::default())
        .on_response(HttpTraceLayerHooks::default())
        .on_failure(HttpTraceLayerHooks::default());
    Router::new()
        .route("/", get(view_root_item))
        .route("/livez", get(healthcheck))
        .route("/readyz", get(healthcheck))
        .route("/:a", get(view_item))
        .route("/:a/", get(view_item))
        .route("/:a/:b", get(view_nested_item))
        .fallback(not_found)
        .with_state(state)
        .layer(trace_layer)
}

#[derive(Debug, Clone)]
struct CustomSampler {}

impl ShouldSample for CustomSampler {
    fn should_sample(
        &self,
        parent_context: Option<&Context>,
        _trace_id: TraceId,
        _name: &str,
        _span_kind: &SpanKind,
        attributes: &[KeyValue],
        _links: &[Link],
    ) -> SamplingResult {
        let mut decision = SamplingDecision::RecordAndSample;
        for a in attributes {
            if a.key.as_str().eq("http.route")
                && (a.value.as_str().eq("/livez") || a.value.as_str().eq("/readyz"))
            {
                decision = SamplingDecision::Drop;
                break;
            }
        }
        SamplingResult {
            decision,
            // No extra attributes ever set by the SDK samplers.
            attributes: Vec::new(),
            // all sampler in SDK will not modify trace state.
            trace_state: match parent_context {
                Some(ctx) => ctx.span().span_context().trace_state().clone(),
                None => TraceState::default(),
            },
        }
    }
}

#[tokio::main]
async fn main() {
    let args = Cli::parse();

    let honeycomb_key_path =
        std::env::var("HONEYCOMB_KEY_PATH").unwrap_or_else(|_| "honeycomb.key".to_string());
    match std::fs::read_to_string(honeycomb_key_path) {
        Ok(api_key) => {
            let otlp_endpoint = "https://api.honeycomb.io";
            let otlp_headers =
                HashMap::from([("x-honeycomb-team".into(), api_key.trim().to_string())]);
            let otlp_service_name = args
                .external_url_prefix
                .clone()
                .unwrap_or("unknown".to_string())
                .replace("://", "-")
                .replace(&['.', '/'], "-");

            let tracer = opentelemetry_otlp::new_pipeline()
                .tracing()
                .with_exporter(
                    opentelemetry_otlp::new_exporter()
                        .http()
                        .with_endpoint(otlp_endpoint)
                        .with_http_client(reqwest::Client::default())
                        .with_headers(otlp_headers)
                        .with_timeout(std::time::Duration::from_secs(5)),
                ) // Replace with runtime::Tokio if using async main
                .with_trace_config(
                    opentelemetry_sdk::trace::config()
                        .with_sampler(CustomSampler {})
                        .with_resource(opentelemetry_sdk::Resource::new(vec![
                            opentelemetry::KeyValue::new("service.name", otlp_service_name),
                        ])),
                )
                .install_batch(opentelemetry_sdk::runtime::Tokio)
                .unwrap();

            let subscriber = tracing_subscriber::registry::Registry::default()
                .with(tracing_subscriber::filter::LevelFilter::INFO) // filter out low-level debug tracing (eg tokio executor)
                .with(tracing_subscriber::fmt::Layer::default()) // log to stdout
                .with(tracing_opentelemetry::layer().with_tracer(tracer)); // traces can go to open telemetry
            tracing::subscriber::set_global_default(subscriber)
                .expect("failed to set tracing subscriber");
            tracing::info!("set up honeycomb tracing");
        }
        Err(e) => {
            tracing_subscriber::fmt::init();
            tracing::info!("couldn't read honeycomb key file: {}", e.to_string());
        }
    }

    let addr = SocketAddr::from((
        args.bind_address.unwrap_or([0, 0, 0, 0, 0, 0, 0, 0].into()),
        args.bind_port.unwrap_or(8080),
    ));

    let app = setup_router(args.external_url_prefix.clone().unwrap_or("".to_string()))
        .into_make_service();
    let svr = axum::Server::bind(&addr).serve(app);

    tracing::info!("server is listening on http://{}...", svr.local_addr());
    if let Err(err) = svr.await {
        tracing::error!("server error: {}", err);
    }
}

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use axum::http::header::{ACCEPT, CACHE_CONTROL, CONTENT_LENGTH, CONTENT_TYPE, ETAG};
    use axum::http::{HeaderValue, Method, Request, StatusCode};
    use hyper::header::{ACCEPT_ENCODING, CONTENT_ENCODING, LOCATION};
    // for `oneshot` and `ready`
    use test_case::test_case;
    use tower::ServiceExt;

    use crate::{setup_router, Asset, CONTENT_FILE_NAME};

    #[tokio::test]
    async fn test_index() {
        let app = setup_router("http://example".to_string());
        let resp = app
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(
            resp.headers().get(CONTENT_TYPE).unwrap(),
            "text/html; charset=utf-8"
        );
        let length: u32 = resp
            .headers()
            .get(CONTENT_LENGTH)
            .unwrap()
            .to_str()
            .unwrap()
            .parse()
            .unwrap();
        assert!(length > 1);
        assert!(resp.headers().get(ETAG).is_some());
        assert!(resp.headers().get(CONTENT_ENCODING).is_none());
        assert_eq!(resp.headers().get(CACHE_CONTROL).unwrap(), "max-age=300");
    }

    #[tokio::test]
    async fn test_redirect_slash() {
        let app = setup_router("http://example".to_string());
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/20230706-binary-blog")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::TEMPORARY_REDIRECT);
        assert_eq!(
            resp.headers().get(LOCATION).unwrap(),
            "/20230706-binary-blog/"
        );
        let length: u32 = resp
            .headers()
            .get(CONTENT_LENGTH)
            .unwrap()
            .to_str()
            .unwrap()
            .parse()
            .unwrap();
        assert_eq!(length, 0);
        assert_eq!(resp.headers().get(CACHE_CONTROL).unwrap(), "max-age=300");
    }

    #[tokio::test]
    async fn test_index_gzipped() {
        let app = setup_router("http://example".to_string());
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/")
                    .header(
                        ACCEPT_ENCODING,
                        HeaderValue::from_str("deflate,gzip").unwrap(),
                    )
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(resp.headers().get(CONTENT_ENCODING).unwrap(), "deflate");
    }

    #[tokio::test]
    async fn test_livez() {
        let app = setup_router("http://example".to_string());
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/livez")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NO_CONTENT);
        assert_eq!(resp.headers().get(CONTENT_LENGTH).unwrap(), "0");
        assert!(resp.headers().get(CONTENT_TYPE).is_none());
    }

    #[tokio::test]
    async fn test_readyz() {
        let app = setup_router("http://example".to_string());
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/readyz")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NO_CONTENT);
        assert_eq!(resp.headers().get(CONTENT_LENGTH).unwrap(), "0");
        assert!(resp.headers().get(CONTENT_TYPE).is_none());
    }

    #[tokio::test]
    async fn test_robots() {
        let app = setup_router("http://example".to_string());
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/robots.txt")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(resp.headers().get(CONTENT_LENGTH).unwrap(), "77");
        assert_eq!(
            resp.headers().get(CONTENT_TYPE).unwrap(),
            "text/plain; charset=utf-8"
        );
        assert!(resp.headers().get(ETAG).is_some());
        assert_eq!(resp.headers().get(CACHE_CONTROL).unwrap(), "max-age=300");
    }

    #[tokio::test]
    async fn test_rss() {
        let app = setup_router("http://example".to_string());
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/rss.xml")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(resp.headers().get(CONTENT_TYPE).unwrap(), "text/xml");
        assert!(resp.headers().get(ETAG).is_some());
        assert_eq!(resp.headers().get(CACHE_CONTROL).unwrap(), "max-age=300");
    }

    #[tokio::test]
    async fn test_feed() {
        let app = setup_router("http://example".to_string());
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/feed.xml")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(resp.headers().get(CONTENT_TYPE).unwrap(), "text/xml");
        assert!(resp.headers().get(ETAG).is_some());
        assert_eq!(resp.headers().get(CACHE_CONTROL).unwrap(), "max-age=300");
    }

    #[test_case("/a"; "plain/a")]
    #[test_case("/a/"; "plain/a/")]
    #[test_case("/a/b"; "plain/a/b")]
    #[test_case("/a/b/"; "plain/a/b/")]
    #[test_case("/a/b/c"; "plain/a/b/c")]
    #[tokio::test]
    async fn test_plain_404(uri: &str) {
        let app = setup_router("http://example".to_string());
        let resp = app
            .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
        assert_eq!(resp.headers().get(CONTENT_LENGTH).unwrap(), "0");
        assert!(resp.headers().get(CONTENT_TYPE).is_none());
    }

    #[test_case("/a"; "html/a")]
    #[test_case("/a/"; "html/a/")]
    #[test_case("/a/b"; "html/a/b")]
    #[test_case("/a/b/"; "html/a/b/")]
    #[test_case("/a/b/c"; "html/a/b/c")]
    #[tokio::test]
    async fn test_html_404(uri: &str) {
        let app = setup_router("http://example".to_string());
        let mut req = Request::builder().uri(uri);
        req.headers_mut()
            .unwrap()
            .insert(ACCEPT, HeaderValue::from_str("text/html").unwrap());
        let resp = app.oneshot(req.body(Body::empty()).unwrap()).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
        assert_eq!(
            resp.headers().get(CONTENT_TYPE).unwrap(),
            "text/html; charset=utf-8"
        );
        let length: u32 = resp
            .headers()
            .get(CONTENT_LENGTH)
            .unwrap()
            .to_str()
            .unwrap()
            .parse()
            .unwrap();
        assert!(length > 1);
    }

    #[test_case("/", 405; "post/")]
    #[test_case("/a", 405; "post/a")]
    #[test_case("/a/", 405; "post/a/")]
    #[test_case("/a/b", 405; "post/a/b")]
    #[test_case("/a/b/", 404; "post/a/b/")]
    #[test_case("/a/b/c", 404; "post/a/b/c")]
    #[tokio::test]
    async fn test_post(uri: &str, code: u16) {
        let app = setup_router("http://example".to_string());
        let resp = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri(uri)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status().as_u16(), code);
    }

    #[tokio::test]
    async fn test_image_assets_are_ok() {
        let blogs: Vec<String> = Asset::iter()
            .filter(|p| p.contains(CONTENT_FILE_NAME))
            .map(|p| {
                p.rsplitn(3, "/")
                    .skip(1)
                    .take(1)
                    .last()
                    .unwrap()
                    .to_owned()
                    .to_string()
            })
            .collect();

        let link_re = regex::Regex::new(r#"src=".+?""#).unwrap();

        for x in blogs {
            println!("checking {}", x);
            let app = setup_router("http://example".to_string());
            let resp = app
                .oneshot(
                    Request::builder()
                        .uri(format!("/{}/", x))
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(resp.status(), StatusCode::OK);
            assert!(resp.headers().get(ETAG).is_some());
            assert_eq!(resp.headers().get(CACHE_CONTROL).unwrap(), "max-age=300");

            let body = hyper::body::to_bytes(resp.into_body()).await.unwrap();
            let body_str = String::from_utf8_lossy(body.as_ref());
            let links: Vec<&str> = link_re
                .find_iter(body_str.as_ref())
                .filter(|m| !m.as_str().contains("://"))
                .map(|m| m.as_str().split("\"").skip(1).take(1).last().unwrap())
                .collect();

            for y in links {
                println!("checking {}", y);
                let app2 = setup_router("http://example".to_string());
                let resp2 = app2
                    .oneshot(
                        Request::builder()
                            .uri(format!("/{}/{}", x, y))
                            .body(Body::empty())
                            .unwrap(),
                    )
                    .await
                    .unwrap();
                assert_eq!(resp2.status(), StatusCode::OK);
                assert!(resp2.headers().get(ETAG).is_some());
                assert_eq!(resp2.headers().get(CACHE_CONTROL).unwrap(), "max-age=300");
            }
        }
    }
}
