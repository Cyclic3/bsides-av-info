use chrono::{DateTime, NaiveTime, TimeDelta};
use color_eyre::Result;
use serde::Serialize;
use std::{io::Write, path::Path};

use crate::{event::TrackIndex, outputs::EventFormatter};

/// TODO: actually do this intelligently
pub type EventTimeZone = chrono::offset::Local;

const TITLE_WRAP: usize = 35;

pub struct ShowFile;
impl EventFormatter for ShowFile {
    fn format(&self, event: crate::event::EventIndex, output_path: &Path) -> Result<()> {
        for (_track_id, track) in event.tracks() {
            let track_name = &track.info.name;
            let mut file = std::fs::File::create(output_path.join(std::path::Path::new(track_name)))?;
            for talk_id in track.sessions.iter().copied() {
                let talk = &event[talk_id];
                let start = talk.start.with_timezone(&event.time_zone);
                let end = talk.start.with_timezone(&event.time_zone);
                let times = format_args!(
                    "{}-{}",
                    start.format("%H:%M"),
                    end.format("%H:%M"),
                );

                let title_lines = wrap_title(&talk.title);

                for line in title_lines {
                    writeln!(&mut file, "{times}\t{line}")?;
                }
                for speaker_name in &talk.speakers {
                    writeln!(&mut file, "{times}\t - {speaker_name}")?;
                }
                writeln!(&mut file)?;
            }
            println!("Track file written to `{track_name}`");
        }
        Ok(())
    }
}

// Wrap the title at the first space after 30 chars
fn wrap_title(title: &str) -> Vec<String> {
    let mut title_lines = Vec::new();
    let mut counter = 0;
    let mut line = String::new();
    for chr in title.chars() {
        counter += 1;
        if counter >= TITLE_WRAP && chr == ' ' {
            title_lines.push(std::mem::take(&mut line));
            counter = 0;
        } else {
            line.push(chr);
        }
    }
    if line.len() > 1 {
        title_lines.push(line);
    }
    title_lines
}
