use color_eyre::Result;
use std::{fmt::{Display, Write as FmtWrite}, path::Path};

use crate::{event::IndexedSession, outputs::EventFormatter};

/// TODO: actually do this intelligently
const TITLE_WRAP: usize = 35;

#[derive(Debug)]
pub struct ShowFile;
impl Display for ShowFile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Show File")
    }
}
impl EventFormatter for ShowFile {
    fn format(&self, event: &crate::event::Event, output_path: &Path) -> Result<()> {
        for (name, sessions) in super::standard_grouping(event) {
            let mut data = String::new();
            for talk in sessions {
                format_talk(&mut data, talk)?;
            }
            std::fs::write(output_path.join(name), data.as_bytes())?;
        }
        Ok(())
    }
}

fn format_talk(out: &mut impl FmtWrite, talk: IndexedSession<'_>) -> Result<()> {
    let start = talk.start();
    // TODO: no way do we end up straddling midnight?
    let end = talk.end();
    let times = format_args!(
        "{}-{}",
        start.format("%H:%M"),
        end.format("%H:%M"),
    );

    let title_lines = wrap_title(&talk.title);

    for line in title_lines {
        writeln!(out, "{times}\t{line}")?;
    }
    for speaker_name in &talk.speakers {
        writeln!(out, "{times}\t - {speaker_name}")?;
    }
    writeln!(out)?;
    Ok(())
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


#[cfg(test)]
mod test {
    use super::ShowFile;
    use crate::{event::test::{load_belfast, load_bristol}, outputs::{show_file::format_talk, test::test_formatter}, test_data_path};

    #[test]
    fn test_bristol() {
        test_formatter(&ShowFile, &load_bristol(), &test_data_path!("show-file"), |_| true);
    }


    #[test]
    fn test_belfast() {
        let event = load_belfast();

        dbg!(&event);

        let mut out = String::new();
        for talk in event.filter().track_name("Track 1 - Exhibition Centre").sessions() {
            format_talk(&mut out, talk).unwrap();
        }

        assert_eq!(
            out,
            "\
09:45-10:25\tMorning Keynote

10:25-11:10\tWhat the Real AI Attacks of the Last
10:25-11:10\tTwo Years Actually Taught Us
10:25-11:10\t - Liana Anca Tomescu

11:30-12:15\t45,724 Networks, One Flipper Zero:
11:30-12:15\tMapping the Wireless Threat Landscape
11:30-12:15\tof Northern Ireland
11:30-12:15\t - M0r3al

12:15-13:00\tHacking Browsers: The Easy Way
12:15-13:00\t - Robbe Van Roey / PinkDraconian

13:45-14:30\tReverse Engineering an OAuth Supply
13:45-14:30\tChain Attack
13:45-14:30\t - art

14:30-15:15\tNo CVV, No 3DS, No Problem: Chaining
14:30-15:15\tTrust Failures Across a Live Aviation
14:30-15:15\tPayment Stack
14:30-15:15\t - Esat Berk Kandemir

15:40-16:25\tChaining the Unchainable: Finding and
15:40-16:25\tExploiting Logic Flaws in Modern Web
15:40-16:25\tArchitectures
15:40-16:25\t - Raj Dhruv

16:25-17:10\tLeaks on the Livewire: How a Few Blocked
16:25-17:10\tRequests Led to a Shocking Discovery
16:25-17:10\t - Daniel Johnston

17:10-17:50\tEvening Keynote
17:10-17:50\t - Aleksandra Aytova

"
        );
    }
}
