#![cfg_attr(not(test), warn(clippy::unwrap_used))]

use clap::Parser;
use color_eyre::Result;
use indexmap::IndexMap;
use serde::Serialize;
use std::path::PathBuf;

mod formats;
mod show_file;

#[derive(Parser)]
struct Args {
    #[clap(long)]
    file: PathBuf,
}

fn main() -> Result<()> {
    color_eyre::install()?;

    let args = Args::parse();

    let sched = std::fs::read_to_string(&args.file)?;
    let schedule = formats::Format::parse_from_str(&sched)?;
    let event = Event::try_from(schedule)?;
    event.make_show_files()?;

    Ok(())
}

#[derive(Debug, Serialize)]
struct Event {
    tracks: IndexMap<String, show_file::ShowFile>,
}

impl Event {
    fn make_show_files(&self) -> Result<()> {
        for (track_name, track) in &self.tracks {
            let mut file = std::fs::File::create(track_name)?;
            track.make_track(&mut file)?;
            println!("Track file written to `{track_name}`");
        }
        Ok(())
    }
}
