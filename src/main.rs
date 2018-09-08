extern crate reqwest;
extern crate serde;
extern crate serde_json;
#[macro_use]
extern crate serde_derive;
extern crate env_logger;
extern crate failure;
#[macro_use]
extern crate structopt;

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;
use std::time::{Duration, Instant};

use reqwest::header::ContentType;
use reqwest::Client;

type Result<T> = std::result::Result<T, failure::Error>;

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
        "-y", "-i", in_file, "-ss", start, "-to", end, "-c", "copy", out_file,
    ];
    println!("Cutting file {} into output {}", in_file, out_file);
    Command::new("ffmpeg").args(args).output()?;
    Ok(())
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
    use reqwest::multipart::Form;

    let form = Form::new()
        .text("key", gfy_name.to_string())
        .file("file", path)?;

    client
        .post("https://filedrop.gfycat.com")
        .multipart(form)
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

#[derive(Clone, Debug, StructOpt)]
pub struct Args {
    #[structopt(parse(from_os_str))]
    in_file: PathBuf,

    #[structopt(parse(from_os_str))]
    out_file: PathBuf,

    start: String,

    end: String,
}

fn main() -> Result<()> {
    use structopt::StructOpt;

    let Args {
        in_file,
        out_file,
        start,
        end,
    } = Args::from_args();

    if !in_file.is_file() {
        println!("Input file {} does not exist!", in_file.display());
    }

    let in_file_str = in_file.display().to_string();
    let out_file_str = out_file.display().to_string();
    cut_file(&in_file_str, &out_file_str, &start, &end)?;
    let client = reqwest::Client::new();
    let ticket = get_ticket(&client)?;
    println!("Starting upload to https://gfycat.com/{}", ticket.gfy_name);
    upload_video(&client, &ticket.gfy_name, &out_file_str)?;
    println!("Upload finished. Waiting for encoding to finish.");
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
