extern crate reqwest;
extern crate serde;
extern crate serde_json;
#[macro_use]
extern crate serde_derive;
extern crate env_logger;
extern crate failure;
#[macro_use]
extern crate structopt;

use std::path::PathBuf;

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

pub fn cut_file(input_file: &str, output_file: &str, start: &str, end: &str) -> Result<()> {
    use std::process::Command;

    let args = &[
        "-y",
        "-i",
        input_file,
        "-ss",
        start,
        "-to",
        end,
        "-c",
        "copy",
        output_file,
    ];
    println!("Cutting file '{}' from {} to {}", input_file, start, end);
    Command::new("ffmpeg").args(args).output()?;
    Ok(())
}

pub fn get_ticket(client: &Client) -> Result<GfycatInfo> {
    use std::collections::HashMap;

    let mut body = HashMap::new();
    body.insert("noMd5", "false");
    let data: GfycatInfo = client
        .post("https://api.gfycat.com/v1/gfycats")
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
        .send()?
        .error_for_status()?;
    Ok(())
}

pub fn get_progress(client: &Client, gfy_name: &str) -> Result<GfycatProgress> {
    let url = format!(
        "https://api.gfycat.com/v1/gfycats/fetch/status/{}",
        gfy_name
    );
    let data: GfycatProgress = client.get(&url).send()?.error_for_status()?.json()?;
    Ok(data)
}

#[derive(Clone, Debug, StructOpt)]
pub struct Args {
    #[structopt(parse(from_os_str))]
    input_file: PathBuf,

    start: String,

    end: String,
}

fn main() -> Result<()> {
    use std::thread;
    use std::time::Duration;
    use structopt::StructOpt;

    let Args {
        input_file,
        start,
        end,
    } = Args::from_args();
    let output_file = "out.mp4";

    if !input_file.is_file() {
        println!("Input file {} does not exist!", input_file.display());
    }

    let input_file_str = input_file.display().to_string();
    cut_file(&input_file_str, output_file, &start, &end)?;
    let client = Client::new();
    let ticket = get_ticket(&client)?;
    println!("Starting upload to https://gfycat.com/{}", ticket.gfy_name);
    upload_video(&client, &ticket.gfy_name, output_file)?;
    println!("Upload finished. Waiting for encoding to finish.");
    loop {
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

        thread::sleep(Duration::from_secs(5));
    }
    Ok(())
}
