use clap::crate_name;
use clap::crate_version;
use clap::Parser;
use convert_case::{Case, Casing};
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Method, Request, Response, Server, StatusCode};
use lazy_static::lazy_static;
use pulldown_cmark::{html, Options, Parser as CmarkParser};
use regex::Regex;
use std::collections::HashMap;
use std::convert::Infallible;
use std::net::{IpAddr, SocketAddr};

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

async fn blog_service(_req: Request<Body>) -> Result<Response<Body>, Infallible> {
    let mut resp = Response::default();
    match (_req.method(), _req.uri().path()) {
        // blog index page
        (&Method::GET, "/") => {
            // TODO - update
            *resp.body_mut() = format!("here's a list of {} blogs", POSTS_BY_TITLE.len()).into();
            Ok(resp)
        }
        (&Method::GET, x) if PAGE_RE.is_match(x) => {
            *resp.body_mut() = "here's a blog".into();
            Ok(resp)
        }
        (&Method::GET, x) if PAGE_ASSET_RE.is_match(x) => {
            *resp.body_mut() = "here's a blog asset".into();
            Ok(resp)
        }
        _ => {
            *resp.status_mut() = StatusCode::NOT_FOUND;
            Ok(resp)
        }
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
