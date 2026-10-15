#![cfg_attr(not(test), warn(clippy::unwrap_used))]

use clap::Parser;
use color_eyre::Result;

use crate::{event::{Event, EventIndex}, inputs::DEFAULT_PARSERS, outputs::DEFAULT_FORMATTERS};

mod tz;
mod event;
mod inputs;
mod outputs;
// mod wordlist;

#[cfg(test)]
#[macro_use(assert_eq)]
extern crate pretty_assertions;

#[derive(Parser)]
struct Args {
    location: String,
    output_dir: std::path::PathBuf
}

// impl Event {
//     fn iter_sessions(&self) -> impl Iterator<Item = &show_file::Session> {
//         self.tracks.values().flat_map(|track| track.sessions.iter())
//     }

//     fn make_show_files(&self) -> Result<()> {
//         for (track_name, track) in &self.tracks {
//             let mut file = std::fs::File::create(track_name)?;
//             track.make_track(&mut file)?;
//             println!("Track file written to `{track_name}`");

//             let ontime_name_csv = format!("{track_name}-ontime.csv");
//             let ontime_name_xlsx = format!("{track_name}-ontime.xlsx");
//             let mut csv_file = std::fs::File::create(&ontime_name_csv)?;
//             let mut xlsx_file = std::fs::File::create(&ontime_name_xlsx)?;
//             ontime::make_ontime_export(&mut csv_file, &mut xlsx_file, track)?;
//             println!(
//                 "Ontime file written to `{ontime_name_csv}` and `{ontime_name_xlsx}`"
//             );
//         }
//         Ok(())
//     }
// }

pub fn main_inner(args: Args) -> Result<()> {
    println!("game");
    std::fs::create_dir_all(&args.output_dir)?;
    color_eyre::install()?;
    let event = Event::parse(&args.location, DEFAULT_PARSERS)?;
    println!("{event:?}");
    println!("Indexing");
    let event: EventIndex = event.into();
    for formatter in DEFAULT_FORMATTERS {
        formatter.format(event.clone(), &args.output_dir);
    }
    Ok(())
}

fn main() -> Result<()> {
    main_inner(Args::parse())



    // let sched = std::fs::read_to_string(&args.location)?;
    // println!("Loaded file `{}`", args.location.display());
    // let schedule = formats::Format::parse_from_str(&sched)?;
    // let event = Event::try_from(schedule)?;
    // println!("Loaded event with {} tracks", event.tracks.len());
    // event.make_show_files()?;

    // let wordlist_file = "wordlist.txt";
    // let file = std::fs::File::create(wordlist_file)?;
    // wordlist::make_wordlist_file(file, &event)?;
    // println!("Wordlist written to `{wordlist_file}`");
}
