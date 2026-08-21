use chrono::NaiveTime;
use color_eyre::{Result, eyre::OptionExt};
use indexmap::IndexMap;
use scraper::{ElementRef, Html, Selector};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct Sessionize {
    sessions: Vec<Session>,
}

#[derive(Debug, PartialEq, Eq, Deserialize, Serialize)]
struct Session {
    title: String,
    speakers: Vec<String>,
    room: String,
    start: NaiveTime,
    end: NaiveTime,
}

impl TryFrom<Sessionize> for crate::Event {
    type Error = color_eyre::Report;
    fn try_from(this: Sessionize) -> Result<Self> {
        let mut tracks = IndexMap::<_, crate::show_file::ShowFile>::new();

        for session in this.sessions {
            let entry = tracks.entry(session.room.clone()).or_default();
            entry.sessions.push(crate::show_file::Session {
                title: session.title,
                speakers: session.speakers,
                start: session.start,
                end: session.end,
            });
        }

        Ok(Self { tracks })
    }
}

impl FromStr for Sessionize {
    type Err = color_eyre::Report;
    fn from_str(input: &str) -> Result<Self> {
        let fragment = Html::parse_fragment(input);

        #[expect(clippy::unwrap_used)]
        let card_selector = Selector::parse(r#"li.sz-session"#).unwrap();

        let mut sessions = Vec::new();

        for card in fragment.select(&card_selector) {
            match parse_card(&card) {
                Ok(session) => {
                    sessions.push(session);
                }
                Err(err) => {
                    println!("Warn: {err}");
                }
            }
        }

        Ok(Self { sessions })
    }
}

fn parse_card(card: &ElementRef<'_>) -> Result<Session> {
    #[expect(clippy::unwrap_used)]
    let room_selector = Selector::parse(r#"div.sz-session__room"#).unwrap();
    #[expect(clippy::unwrap_used)]
    let time_selector = Selector::parse(r#"div.sz-session__time"#).unwrap();
    #[expect(clippy::unwrap_used)]
    let title_selector = Selector::parse(r#"h3.sz-session__title"#).unwrap();
    #[expect(clippy::unwrap_used)]
    let speaker_selector =
        Selector::parse(r#"ul.sz-session__speakers"#).unwrap();

    let room = card
        .select(&room_selector)
        .next()
        .ok_or_eyre("Card missing room definition")?
        .text()
        .next()
        .ok_or_eyre("Room definition missing text")?
        .trim()
        .to_string();
    let time = card
        .select(&time_selector)
        .next()
        .ok_or_eyre("Card missing time definition")?
        .text()
        .next()
        .ok_or_eyre("Time definition missing text")?
        .trim()
        .to_string();
    let title = card
        .select(&title_selector)
        .next()
        .ok_or_eyre("Card missing title definition")?
        .text()
        .next()
        .ok_or_eyre("Title definition missing text")?
        .trim()
        .to_string();
    let speakers = card
        .select(&speaker_selector)
        .flat_map(|ele| ele.text())
        .filter(|s| s.len() > 1)
        .map(|s| s.trim().to_string())
        .collect::<Vec<_>>();

    let mut parts = time.split_whitespace();
    parts.next().ok_or_eyre("Malformed time")?;
    let start = parts.next().ok_or_eyre("Malformed time")?.parse()?;
    parts.next().ok_or_eyre("Malformed time")?;
    let end = parts.next().ok_or_eyre("Malformed time")?.parse()?;

    Ok(Session {
        title,
        speakers,
        room,
        start,
        end,
    })
}

#[cfg(test)]
mod test {
    use super::*;
    use pretty_assertions::assert_eq;
    use std::io::Cursor;

    const BELFAST: &str = include_str!("../../test-data/belfast.html");

    #[test]
    fn test_html() {
        let sessions: Sessionize = BELFAST.parse().unwrap();
        insta::assert_yaml_snapshot!(sessions);
    }

    #[test]
    fn test_show_file() {
        let sessions: Sessionize = BELFAST.parse().unwrap();
        let event = crate::Event::try_from(sessions).unwrap();
        insta::assert_yaml_snapshot!(event);

        let mut file = Vec::<u8>::new();
        let mut cur = Cursor::new(&mut file);

        dbg!(&event);

        let t1 = event.tracks.get("Track 1 - Exhibition Centre").unwrap();
        t1.make_track(&mut cur).unwrap();

        let out = String::from_utf8(file).unwrap();
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
