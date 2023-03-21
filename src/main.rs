use clap::crate_name;
use clap::crate_version;
use clap::Parser;
use convert_case::{Case, Casing};
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Method, Request, Response, Server, StatusCode};
use lazy_static::lazy_static;
use regex::Regex;
use std::collections::HashMap;
use std::convert::Infallible;
use std::net::{IpAddr, SocketAddr};
use maud::{html, Markup, DOCTYPE};

pub mod posts;

#[derive(Parser, Debug)]
#[command(version)]
struct Cli {
    #[arg(long)]
    bind_address: Option<IpAddr>,

    #[arg(long)]
    bind_port: Option<u16>,
}

macro_rules! banner_format {
    () => {
        "        \
        _   ,_,   _\n       \
       / `'=) (='` \\\n      \
      /.-.-.\\ /.-.-.\\\n      \
      `      \"      `\n\
{}: {} https://github.com/astromechza/binary-blog\n\
args: {:?}"
    };
}

lazy_static! {
    static ref PAGE_RE: Regex = Regex::new("^/page/[^/]+$").unwrap();
    static ref PAGE_ASSET_RE: Regex = Regex::new("^/page/[^/]+/[^/]+$").unwrap();
    static ref POSTS_BY_TITLE: HashMap<String, posts::Post> = {
        let mut m = HashMap::new();
        let b = posts::_0001_a_binary_blog::build();
        m.insert(b.title.to_case(Case::Snake), b);
        m
    };
}

fn check_method(_req: Request<Body>, allowed: &Method) -> Option<Response<Body>> {
    if _req.method() == allowed {
        return None;
    }
    Some(Response::builder().status(405).body(Body::empty()).unwrap())
}

fn generate(title: String, inner: Markup) -> Markup {
    html! {
        (DOCTYPE)
        html {
            head {
                meta charset="utf-8";
                title { (title) }
            }
            body {
                { (inner) }
            }
        }
    }
}

async fn handle_blog_index(_req: Request<Body>) -> Result<Response<Body>, Infallible> {
    if let Some(cm) = check_method(_req, &Method::GET) {
        return Ok(cm);
    }

    let mut doc: Markup = generate("Index".into(), html! {
    });

    Ok(Response::builder()
        .body(doc.into_string().into())
        .unwrap())
}

async fn blog_service(_req: Request<Body>) -> Result<Response<Body>, Infallible> {
    let mut resp = Response::default();
    match _req.uri().path() {
        "/" => handle_blog_index(_req).await,
        x if PAGE_RE.is_match(x) => {
            *resp.body_mut() = "here's a blog".into();
            Ok(resp)
        }
        x if PAGE_ASSET_RE.is_match(x) => {
            *resp.body_mut() = "here's a blog asset".into();
            Ok(resp)
        }
        _ => Ok(Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::empty())
            .unwrap()),
    }
}

#[tokio::main]
async fn main() {
    let args = Cli::parse();
    println!(banner_format!(), crate_name!(), crate_version!(), args);
    //
    // let post = posts::_0001_a_binary_blog::build();
    //
    // let mut options = Options::empty();
    // options.insert(Options::ENABLE_STRIKETHROUGH);
    // let parser = CmarkParser::new_ext(post.raw_markdown, options);
    //
    // let mut html_output = String::new();
    // html::push_html(&mut html_output, parser);
    //
    // println!("{}", html_output);

    let addr = SocketAddr::from((
        args.bind_address.unwrap_or([127, 0, 0, 1].into()),
        args.bind_port.unwrap_or(8080),
    ));

    let make_svc = make_service_fn(|_conn| async {
        // service_fn converts our function into a `Service`
        Ok::<_, Infallible>(service_fn(blog_service))
    });

    let server = Server::bind(&addr).serve(make_svc);

    println!("listening on {}...", server.local_addr());

    // Run this server for... forever!
    if let Err(e) = server.await {
        eprintln!("server error: {}", e);
    }
}
