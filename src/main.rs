use anyhow::{anyhow, Result};
use clap::{AppSettings, Clap};
use reqwest::{Url, header, Client, Response};
use std::str::FromStr;
use std::collections::HashMap;
use colored::*;
use mime::{Mime, APPLICATION_JSON};

// 定义 HTTPie 的 CLI 的主入口，它包含若干个子命令
// 下面 /// 的注释是文档，clap 会将其作为 CLI 的帮助

/// A naive httpie implementation with Rust, can you imagine how easy it is?
#[derive(Clap, Debug)]
#[clap(version = "1.0", author = "Bryan")]
#[clap(setting = AppSettings::ColoredHelp)]
struct Opts {
    #[clap(subcommand)]
    subcmd: SubCommand,
}

// 子命令分别对应不同的 HTTP 方法，目前只支持 get / post
#[derive(Clap, Debug)]
enum SubCommand {
    Get(Get),
    Post(Post),
    // 我们暂且不支持其它 HTTP 方法
}

// get 子命令

/// feed get with an url and we will retrieve the response for you
#[derive(Clap, Debug)]
struct Get {
    /// HTTP 请求的 URL
    #[clap(parse(try_from_str = parse_url))]
    url: String,
}

// post 子命令。需要输入一个 URL，和若干个可选的 key=value，用于提供 json body

/// feed post with an url and optional key=value pairs. We will post the data
/// as JSON, and retrieve the response for you
#[derive(Clap, Debug)]
struct Post {
    /// HTTP 请求的 URL
    #[clap(parse(try_from_str = parse_url))]
    url: String,
    /// HTTP 请求的 body
    #[clap(parse(try_from_str = parse_kv_pair))]
    body: Vec<KvPair>,
}

fn parse_url(s: &str) -> Result<String> {
    // 这里我们仅仅检查一下 URL 是否合法
    let _url: Url = s.parse()?;
    Ok(s.into())
}

#[derive(Debug)]
struct KvPair {
    k: String,
    v: String,
}

impl FromStr for KvPair {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut split = s.split("=");
        let err = || anyhow!("Failed to parse {}", s);
        Ok(Self {
            k: split.next().ok_or_else(err)?.to_string(),
            v: split.next().ok_or_else(err)?.to_string(),
        })
    }
}

fn parse_kv_pair(s: &str) -> Result<KvPair> {
    Ok(s.parse()?)
}


async fn get(client: Client, args: &Get) -> Result<()> {
    let resp = client.get(&args.url).send().await?;

    print_response(resp).await?;

    Ok(())
}

async fn post(client: Client, args: &Post) -> Result<()> {
    let body = {
        let mut body = HashMap::with_capacity(args.body.len());

        for pair in args.body.iter() {
            body.insert(&pair.k, &pair.v);
        }

        body
    };

    let resp = client.post(&args.url).json(&body).send().await?;

    print_response(resp).await?;

    Ok(())
}

fn print_response_line(resp: &Response) {
    println!("{}", (format!("{:?} {}", resp.version(), resp.status())).blue());
}

fn print_response_headers(resp: &Response) {
    let headers = resp.headers();

    for (name, value) in headers {
        println!("{}: {:?}", name.to_string().green(), value);
    }
}

fn print_body(m: Option<Mime>, body: &str) {
    if matches!(m, Some(v) if v == APPLICATION_JSON) {
        let j_text = jsonxf::pretty_print(body);
        if let Ok(j_text) = j_text {
            println!("{}", j_text.cyan());
            return;
        }
    }

    println!("{}", body);
}

async fn print_response(resp: Response) -> Result<()> {
    print_response_line(&resp);
    print_response_headers(&resp);

    let ct = get_content_type(&resp);
    let body = resp.text().await?;
    print_body(ct, &body);

    Ok(())
}

fn get_content_type(resp: &Response) -> Option<Mime> {
    resp.headers().get(header::CONTENT_TYPE).map(|v| v.to_str().unwrap().parse().unwrap())
}

#[tokio::main]
async fn main() -> Result<()> {
    let opts: Opts = Opts::parse();
    // println!("{:?}", opts);

    let client = Client::new();

    match opts.subcmd {
        SubCommand::Get(ref args) => get(client, args).await?,
        SubCommand::Post(ref args) => post(client, args).await?,
    };

    Ok(())
}

