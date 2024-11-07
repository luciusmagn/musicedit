use clap::Parser;
use anyhow::{Result, Context};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[derive(Parser)]
#[command(
    author = "Lukáš Hozda (LHO) - TPOT Superman",
    version,
    about = "Make a music edit from a list of pictures and an audio track"
)]
struct Args {
    /// Path to the audio file
    #[arg(short, long)]
    audio: PathBuf,
    /// Directory containing images
    #[arg(short, long)]
    images: PathBuf,
    /// Fade duration in seconds
    #[arg(short, long, default_value_t = 2.0)]
    fade: f64,
    /// Output filename
    #[arg(short, long, default_value = "output.mp4")]
    output: PathBuf,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let images: Vec<_> = fs::read_dir(&args.images)?
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            if path
                .extension()?
                .to_str()?
                .matches(|c| "jpg|jpeg|png".contains(c))
                .count()
                > 0
            {
                Some(path.to_str()?.to_owned())
            } else {
                None
            }
        })
        .collect();

    let output = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-show_entries",
            "format=duration",
            "-of",
            "default=noprint_wrappers=1:nokey=1",
            args.audio.to_str().context("no str")?,
        ])
        .output()?;

    let duration: f64 =
        String::from_utf8(output.stdout)?.trim().parse()?;
    let pic_time =
        (duration - 2.0 * args.fade) / images.len() as f64;

    let mut filter = String::new();
    for i in 0..images.len() {
        filter.push_str(&format!(
            "[{}:v]loop=loop=-1:size=1,scale=w=1920:h=1080:force_original_aspect_ratio=decrease,pad=1920:1080:(ow-iw)/2:(oh-ih)/2,setsar=1,format=yuv420p,trim=duration={}[v{}];",
            i, pic_time, i
        ));
    }

    for i in 0..images.len() {
        let fade_str = if i == 0 {
            format!("fade=t=in:st=0:d={}", args.fade)
        } else if i == images.len() - 1 {
            format!(
                "fade=t=out:st={}:d={}",
                pic_time - args.fade,
                args.fade
            )
        } else {
            "null".to_string()
        };

        filter.push_str(&format!(
            "[v{}]{}[v{}out];",
            i, fade_str, i
        ));
    }

    filter.push_str(&format!(
        "{}",
        images
            .iter()
            .enumerate()
            .map(|(i, _)| format!("[v{}out]", i))
            .collect::<Vec<_>>()
            .join("")
    ));
    filter.push_str(&format!(
        "concat=n={}:v=1:a=0[outv]", // Added [outv] label here
        images.len()
    ));

    let input_args: Vec<_> =
        images.iter().flat_map(|img| vec!["-i", img]).collect();

    let audio_index = images.len();
    Command::new("ffmpeg")
        .args(input_args)
        .args(["-i", args.audio.to_str().context("no str")?])
        .args(["-filter_complex", &filter])
        .args([
            "-map",
            "[outv]",
            "-map",
            &format!("{}:a", audio_index),
            "-c:a",
            "copy",
        ])
        .arg("-shortest")
        .arg(args.output)
        .status()?;

    Ok(())
}
