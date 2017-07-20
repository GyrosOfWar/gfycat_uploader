#[macro_use]
extern crate clap;
extern crate hyper;
extern crate hyper_native_tls;
#[macro_use]
extern crate error_chain;
extern crate serde;
extern crate serde_json;
#[macro_use]
extern crate serde_derive;
extern crate multipart;
extern crate env_logger;

use std::process::Command;
use std::collections::HashMap;
use std::time::{Instant, Duration};

use hyper::Client;
use hyper::header::ContentType;
use hyper::net::HttpsConnector;
use hyper_native_tls::NativeTlsClient;
use multipart::client::lazy::Multipart;


use errors::Result;

mod errors {
    use std::io;
    use std::error::Error as StdError;

    use multipart::client::lazy::LazyError;

    error_chain! {
        foreign_links {
            Io(io::Error);
            Hyper(::hyper::Error);
            Json(::serde_json::Error);
        }
    }

    impl<'a, E> From<LazyError<'a, E>> for Error
    where
        E: StdError,
    {
        fn from(err: LazyError<'a, E>) -> Error {
            Error::from(format!("{}", err))
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GfycatInfo {
    #[serde(rename = "gfyname")]
    pub gfy_name: String,
    pub secret: String,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GfycatProgress {
    pub task: Option<String>,
    #[serde(rename = "gfyname")]
    pub gfy_name: Option<String>,
    pub time: Option<i32>,
}

pub fn cut_file(in_file: &str, out_file: &str, start: &str, end: &str) -> Result<()> {
    let args = &[
        "-y",
        "-i",
        in_file,
        "-ss",
        start,
        "-to",
        end,
        "-c",
        "copy",
        out_file,
    ];
    println!("Cutting file {} into output {}", in_file, out_file);
    Command::new("ffmpeg").args(args).output()?;
    Ok(())
}

pub fn get_ticket(client: &Client) -> Result<GfycatInfo> {
    let mut body = HashMap::new();
    body.insert("noMd5", "false");
    let body = serde_json::to_string(&body)?;
    let response = client
        .post("https://api.gfycat.com/v1/gfycats")
        .header(ContentType::json())
        .body(&body)
        .send()?;
    let data = serde_json::from_reader(response)?;
    Ok(data)
}

pub fn upload_video(client: &Client, gfy_name: &str, path: &str) -> Result<()> {
    Multipart::new()
        .add_file("file", path)
        .add_text("key", gfy_name)
        .client_request(client, "https://filedrop.gfycat.com")?;

    Ok(())
}

pub fn get_progress(client: &Client, gfy_name: &str) -> Result<GfycatProgress> {
    let url = format!(
        "https://api.gfycat.com/v1/gfycats/fetch/status/{}",
        gfy_name
    );
    let response = client.get(&url).send()?;
    let data = serde_json::from_reader(response)?;
    Ok(data)
}

fn run() -> Result<()> {
    let matches = clap_app!(gfycat_uploader => 
        (version: crate_version!())
        (author: crate_authors!())
        (about: "Uploads files to gfycat")
        (@arg IN_FILE: +required "Input video file")
        (@arg OUT_FILE: +required "Output video file")
        (@arg START: +required "Start time")
        (@arg END: +required "End time")
    ).get_matches();
    let in_file = matches.value_of("IN_FILE").unwrap();
    let out_file = matches.value_of("OUT_FILE").unwrap().to_string();
    let start = matches.value_of("START").unwrap();
    let end = matches.value_of("END").unwrap();

    cut_file(in_file, &out_file, start, end)?;
    let client = {
        let ssl = NativeTlsClient::new().unwrap();
        let connector = HttpsConnector::new(ssl);
        Client::with_connector(connector)
    };

    let ticket = get_ticket(&client)?;
    println!("Starting upload to https://gfycat.com/{}", ticket.gfy_name);
    upload_video(&client, &ticket.gfy_name, &out_file)?;
    println!("Upload finished. Waiting for encode.");
    let mut last = Instant::now();
    loop {
        if last.elapsed() > Duration::from_secs(5) {
            last = Instant::now();
            let progress = get_progress(&client, &ticket.gfy_name)?;
            if let Some(task) = progress.task {
                if task == "complete" {
                    println!(
                        "Encoding finished! Finished gfycat at: https://gfycat.com/{}",
                        ticket.gfy_name
                    );
                    break;
                }
            }
        }
    }
    Ok(())
}

fn main() {
    if let Err(e) = run() {
        println!("Error: {}", e);
    }
}
