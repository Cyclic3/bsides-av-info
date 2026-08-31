use chrono::NaiveTime;
use color_eyre::Result;
use serde::Serialize;
use std::io::Write;

const TITLE_WRAP: usize = 35;

#[derive(Debug, Default, Serialize)]
pub struct ShowFile {
    pub sessions: Vec<Session>,
}

#[derive(Debug, Serialize)]
pub struct Session {
    pub title: String,
    pub description: String,
    pub speakers: Vec<String>,
    pub is_break: bool,
    pub start: NaiveTime,
    pub end: NaiveTime,
}

impl ShowFile {
    pub fn make_track<W>(&self, mut writer: W) -> Result<()>
    where
        W: Write,
    {
        for talk in self.sessions.iter().filter(|session| !session.is_break) {
            talk.write_file(&mut writer)?;
        }
        Ok(())
    }
}

impl Session {
    fn write_file<W>(&self, mut writer: W) -> Result<()>
    where
        W: Write,
    {
        let times = format_args!(
            "{}-{}",
            self.start.format("%H:%M"),
            self.end.format("%H:%M"),
        );

        let title_lines = wrap_title(&self.title);

        for line in title_lines {
            writeln!(&mut writer, "{times}\t{line}")?;
        }
        for speaker_name in &self.speakers {
            writeln!(&mut writer, "{times}\t - {speaker_name}")?;
        }
        writeln!(&mut writer)?;

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
