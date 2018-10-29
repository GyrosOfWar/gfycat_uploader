use error::Result;
use reqwest::Client;

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