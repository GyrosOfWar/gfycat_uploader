#[macro_use]
extern crate clap;
extern crate reqwest;
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
use std::io::Read;
use std::thread;
use std::time::{Instant, Duration};

use multipart::client::lazy::Multipart;
use reqwest::Client;
use reqwest::mime::{Mime, SubLevel, TopLevel, Value, Attr};
use reqwest::header::ContentType;

use errors::Result;

mod errors {
    use std::io;
    use std::error::Error as StdError;

    use multipart::client::lazy::LazyError;
    use reqwest;

    error_chain! {
        foreign_links {
            Io(io::Error);
            Reqwest(reqwest::Error);
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

fn content_type(bound: &str) -> ContentType {
    ContentType(multipart_mime(bound))
}

fn multipart_mime(bound: &str) -> Mime {
    Mime(
        TopLevel::Multipart,
        SubLevel::Ext("form-data".into()),
        vec![(Attr::Ext("boundary".into()), Value::Ext(bound.into()))],
    )
}

pub fn get_ticket(client: &Client) -> Result<GfycatInfo> {
    let mut body = HashMap::new();
    body.insert("noMd5", "false");
    let data: GfycatInfo = client
        .post("https://api.gfycat.com/v1/gfycats")
        .header(ContentType::json())
        .json(&body)
        .send()?
        .json()?;
    Ok(data)
}

pub fn upload_video(client: &Client, gfy_name: &str, path: &str) -> Result<()> {
    let mut multipart = Multipart::new()
        .add_file("file", path)
        .add_text("key", gfy_name)
        .prepare()?;
    let mut buf = vec![];
    multipart.read_to_end(&mut buf)?;
    let content_type = content_type(multipart.boundary());

    client
        .post("https://filedrop.gfycat.com")
        .header(content_type)
        .body(buf)
        .send()?;
    Ok(())
}

pub fn get_progress(client: &Client, gfy_name: &str) -> Result<GfycatProgress> {
    let url = format!(
        "https://api.gfycat.com/v1/gfycats/fetch/status/{}",
        gfy_name
    );
    let data: GfycatProgress = client.get(&url).send()?.json()?;
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
    let client = reqwest::Client::new()?;
    let ticket = get_ticket(&client)?;
    let gfy_name = ticket.gfy_name.clone();
    println!("Starting upload to https://gfycat.com/{}", ticket.gfy_name);
    let thread = thread::spawn(move || {
        if let Err(e) = upload_video(&client, &ticket.gfy_name, &out_file) {
            println!("Failed to upload: {}", e);
        }
        println!("Upload finished, waiting for encode.");
    });

    let client = Client::new()?;
    let mut last = Instant::now();
    loop {
        if last.elapsed() > Duration::from_secs(5) {
            last = Instant::now();
            let progress = get_progress(&client, &gfy_name)?;
            if let Some(task) = progress.task {
                if task == "complete" {
                    println!(
                        "Encoding finished. Finished gfycat at: https://gfycat.com/{}",
                        gfy_name
                    );
                    break;
                }
                if task == "encoding" {
                    println!("\rWaiting for encode..");
                }
            }
        }
    }

    let _ = thread.join();
    Ok(())
}

fn main() {
    if let Err(e) = run() {
        println!("Error: {}", e);
    }
}
