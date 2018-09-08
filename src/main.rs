extern crate reqwest;
extern crate serde;
extern crate serde_json;
#[macro_use]
extern crate serde_derive;
extern crate env_logger;
#[macro_use]
extern crate failure;
#[macro_use]
extern crate structopt;
extern crate rustyline;

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

pub fn cut_file(
    input_file: &str,
    output_file: &str,
    start: Option<&String>,
    end: Option<&String>,
    verbose: bool,
) -> Result<bool> {
    use std::process::Command;

    if start.is_none() && end.is_none() {
        if verbose {
            println!("No start or end time specified, not calling ffmpeg");
        }
        return Ok(false);
    }

    let mut args = vec!["-y", "-i", input_file];
    if let Some(start) = start {
        args.push("-ss");
        args.push(start)
    }
    if let Some(end) = end {
        args.push("-to");
        args.push(end);
    }

    args.push("-c");
    args.push("copy");
    args.push(output_file);
    if verbose {
        println!("Calling `ffmpeg {}`", args.join(" "));
    }

    Command::new("ffmpeg").args(args).output()?;
    Ok(true)
}

pub fn get_ticket(client: &Client) -> Result<GfycatInfo> {
    use std::collections::HashMap;

    let mut body = HashMap::new();
    body.insert("noMd5", "false");
    let data: GfycatInfo = client
        .post("https://api.gfycat.com/v1/gfycats")
        .json(&body)
        .send()?
        .error_for_status()?
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
    #[structopt(
        short = "s",
        long = "start",
        help = "time at which to start the video"
    )]
    start_time: Option<String>,

    #[structopt(
        short = "e",
        long = "end",
        help = "time at which to end the video"
    )]
    end_time: Option<String>,

    #[structopt(short = "v", long = "verbose", help = "verbose output")]
    verbose: bool,

    #[structopt(parse(from_os_str))]
    input_file: PathBuf,
}

impl Args {
    pub fn new() -> Result<Self> {
        use std::env::args;
        use structopt::StructOpt;

        Args::from_iter_safe(args()).or_else(|_| Args::from_cli())
    }

    pub fn from_cli() -> Result<Self> {
        use rustyline::Editor;

        fn read_time(rl: &mut Editor<()>, prompt: &str) -> Option<String> {
            let input = rl.readline(prompt).ok()?;
            if input.trim().len() == 0 {
                None
            } else {
                Some(input)
            }
        }

        let mut rl = Editor::<()>::new();

        println!("gfycat_uploader");
        let input_file = rl.readline("Enter path to input file: ")?;
        let input_file = PathBuf::from(input_file);
        let start_time = read_time(&mut rl, "Enter start time (leave blank for 00:00): ");
        let end_time = read_time(&mut rl, "Enter end time (leave blank for end of video): ");

        Ok(Args {
            start_time,
            end_time,
            input_file,
            verbose: false,
        })
    }
}

fn main() -> Result<()> {
    use std::time::Duration;
    use std::{fs, thread};

    let Args {
        input_file,
        start_time,
        end_time,
        verbose,
    } = Args::new()?;
    let output_file = "out.mp4";

    if !input_file.is_file() {
        bail!("Input file {} does not exist!", input_file.display());
    }

    let input_file_str = input_file.display().to_string();

    let did_cut = cut_file(
        &input_file_str,
        output_file,
        start_time.as_ref(),
        end_time.as_ref(),
        verbose,
    )?;
    if !did_cut {
        if verbose {
            println!("Copying {} to {}", input_file.display(), output_file);
        }
        fs::copy(input_file, output_file)?;
    }

    let client = Client::new();
    let ticket = get_ticket(&client)?;
    println!("Starting upload to https://gfycat.com/{}", ticket.gfy_name);
    upload_video(&client, &ticket.gfy_name, output_file)?;
    println!("Upload finished. Waiting for encoding to finish.");
    loop {
        let progress = get_progress(&client, &ticket.gfy_name)?;
        if verbose {
            println!("Upload progress response {:?}", progress);
        }
        if let Some(task) = progress.task {
            if task == "complete" {
                println!(
                    "Encoding finished! Finished gfycat at: https://gfycat.com/{}",
                    ticket.gfy_name
                );
                break;
            }
        }
        if verbose {
            println!("Waiting for 5 seconds to make next request");
        }
        thread::sleep(Duration::from_secs(5));
    }
    Ok(())
}
