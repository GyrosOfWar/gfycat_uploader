use error::Result;
use upload::{cut_file, get_ticket, upload_video, get_progress};
use std::path::PathBuf;
use reqwest::Client;

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

fn cli_main() -> Result<()> {
    use std::time::Duration;
    use std::{fs, thread};
    use structopt::StructOpt;

    let Args {
        input_file,
        start_time,
        end_time,
        verbose,
    } = Args::from_args();
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
