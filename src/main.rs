use clap::Parser;
use clap::crate_name;
use clap::crate_version;
use pulldown_cmark::{Parser as CmarkParser, Options, html};

pub mod posts;

#[derive(Parser,Debug)]
#[command(version)]
struct Cli {
}

macro_rules! banner_format { () => { "        \
        _   ,_,   _\n       \
       / `'=) (='` \\\n      \
      /.-.-.\\ /.-.-.\\\n      \
      `      \"      `\n\
{}: {} https://github.com/astromechza/binary-blog\n\
args: {:?}" }; }

const BLOG_1: &str = r#####"
# This is my example post

foo far fee

## blah
"#####;

fn main() {
    let args = Cli::parse();
    println!(banner_format!(), crate_name!(), crate_version!(), args);

    let post = posts::_0001_a_binary_blog::build();

    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    let parser = CmarkParser::new_ext(post.raw_markdown, options);

    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);

    println!("{}", html_output);
}
