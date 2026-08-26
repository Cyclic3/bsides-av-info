#![cfg_attr(not(test), warn(clippy::unwrap_used))]

use clap::Parser;
use color_eyre::Result;
use indexmap::IndexMap;
use serde::Serialize;
use std::path::PathBuf;

mod formats;
mod ontime;
mod show_file;

#[cfg(test)]
#[macro_use(assert_eq)]
extern crate pretty_assertions;

#[derive(Parser)]
struct Args {
    #[clap(long)]
    file: PathBuf,
}

fn main() -> Result<()> {
    color_eyre::install()?;

    println!("game");
    let args = Args::parse();

    let sched = std::fs::read_to_string(&args.file)?;
    println!("Loaded file `{}`", args.file.display());
    let schedule = formats::Format::parse_from_str(&sched)?;
    let event = Event::try_from(schedule)?;
    println!("Loaded event with {} tracks", event.tracks.len());
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

            let ontime_name_csv = format!("{track_name}-ontime.csv");
            let ontime_name_xlsx = format!("{track_name}-ontime.xlsx");
            let mut csv_file = std::fs::File::create(&ontime_name_csv)?;
            let mut xlsx_file = std::fs::File::create(&ontime_name_xlsx)?;
            ontime::make_ontime_export(&mut csv_file, &mut xlsx_file, track)?;
            println!(
                "Ontime file written to `{ontime_name_csv}` and `{ontime_name_xlsx}`"
            );
        }
        Ok(())
    }
}
