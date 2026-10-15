use std::fmt::Display;

use chrono::{NaiveDate, NaiveTime, TimeDelta, Utc};
use color_eyre::{Result, eyre::OptionExt};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::{event::{Event, TrackInfo, TrackSet}, inputs::EventParser};

pub struct Pretalx;
impl Display for Pretalx {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Pretalx")
    }
}
impl EventParser for Pretalx {
    fn parse_from_str(&self, data: &str) -> Result<Event> {
        // Parse the json
        let PretalxJson { schedule: Schedule { conference } } = serde_json::from_str(data)?;
        let time_zone = conference.time_zone_name.parse()?;

        // Pump it
        let mut ret = Event::new(time_zone);
        for day in conference.days.into_iter() {
            // For some reason tracks are stored in rooms, which have slightly different pluralisation >:(
            for (track_name, sessions) in day.rooms {
                let track_id = ret.get_or_create_track(track_name, |name| TrackInfo { name });
                for talk in sessions {
                    let speakers = talk.persons
                        .into_iter()
                        .map(|person| {
                            if !person.public_name.is_empty() {
                                person.public_name
                            }
                            else {
                                println!(
                                    "Warning: no public name specified, falling back to \
                                    regular name for `{}`",
                                    person.name,
                                );
                                person.name
                            }
                        })
                        .collect();


                    let start =
                        day.date
                            .and_time(talk.start)
                            .and_local_timezone(ret.time_zone)
                            .single().expect("Got an ambiguous start time")
                            .with_timezone(&Utc);
                    let (hours, minutes) = talk
                        .duration
                        .split_once(':')
                        .ok_or_eyre("Malformed duration")?;
                    let hours = hours.parse::<i64>()?;
                    let minutes = minutes.parse::<i64>()?;
                    let duration = TimeDelta::minutes(hours * 60 + minutes);

                    ret.add_session(crate::event::Session {
                        title: talk.title,
                        description: talk.r#abstract,
                        room: None,
                        speakers,
                        start,
                        end: start + duration,
                        tracks: TrackSet::from_iter([track_id]),
                    });
                }
            }
        }

        Ok(ret)
    }
}


// impl TryFrom<Vec<Session>> for crate::show_file::ShowFile {
//     type Error = color_eyre::Report;
//     fn try_from(talks: Vec<Session>) -> Result<Self> {
//         Ok(Self {
//             sessions: talks
//                 .into_iter()
//                 .map(TryInto::try_into)
//                 .collect::<Result<_>>()?,
//         })
//     }
// }

// impl TryFrom<Session> for crate::show_file::Session {
//     type Error = color_eyre::Report;
//     fn try_from(talk: Session) -> Result<Self> {
//         let start = talk.start;
//         let (hours, minutes) = talk
//             .duration
//             .split_once(':')
//             .ok_or_eyre("Malformed duration")?;
//         let hours = hours.parse::<i64>()?;
//         let minutes = minutes.parse::<i64>()?;
//         let duration = TimeDelta::minutes(hours * 60 + minutes);
//         let end = start + duration;
//         // let times =
//         //     format_args!("{}-{}", start.format("%H:%M"), end.format("%H:%M"));

//         let mut speakers = Vec::new();
//         for person in &talk.persons {
//             let name = if !person.public_name.is_empty() {
//                 &person.public_name
//             } else {
//                 println!(
//                     "Warning: no public name specified, falling back to \
//                     regular name for `{}`",
//                     person.name,
//                 );
//                 &person.name
//             };
//             speakers.push(name.clone());
//         }

//         Ok(Self {
//             title: talk.title,
//             description: talk.r#abstract + "\n" + &talk.description,
//             speakers,
//             is_break: false,
//             start,
//             end,
//         })
//     }
// }

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct PretalxJson {
    pub schedule: Schedule,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct Schedule {
    pub conference: Conference,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct Conference {
    pub title: String,
    pub tracks: Vec<Track>,
    pub days: Vec<Day>,
    pub time_zone_name: String
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct Track {
    pub name: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct Day {
    pub index: i64,
    pub rooms: IndexMap<String, Vec<Session>>,
    pub date: NaiveDate
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct Session {
    pub title: String,
    pub r#abstract: String,
    pub description: String,
    pub start: NaiveTime,
    pub duration: String,
    #[serde(default)]
    pub persons: Vec<Person>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct Person {
    pub name: String,
    pub public_name: String,
}

// #[cfg(test)]
// mod test {
//     use super::*;
//     use std::io::Cursor;

//     const BRISTOL: &str = include_str!("../../test-data/bristol-2026.json");

//     #[test]
//     fn test_deserialise() {
//         let _ = serde_json::from_str::<Pretalx>(BRISTOL).unwrap();
//     }

//     #[test]
//     fn test_show_file() {
//         let mut bristol = serde_json::from_str::<Pretalx>(BRISTOL).unwrap();

//         let mut file = Vec::<u8>::new();
//         let mut cur = Cursor::new(&mut file);

//         let t1 = bristol.schedule.conference.days[0]
//             .rooms
//             .swap_remove("Track 1")
//             .unwrap();
//         let show = crate::show_file::ShowFile::try_from(t1).unwrap();
//         show.make_track(&mut cur).unwrap();

//         let out = String::from_utf8(file).unwrap();
//         assert_eq!(
//             out,
//             "\
// 08:55-09:05\tOpening Notes Friday

// 09:05-09:45\tKeynote - Managing The Minefield of
// 09:05-09:45\tManagement Cyber Explosions - Understanding
// 09:05-09:45\twhat to say and when to say it
// 09:05-09:45\t - Peter Jones

// 10:00-10:40\tFake It Till You Detect It: Live Deepfakes,
// 10:00-10:40\tDetection Gaps, and the Human Risk
// 10:00-10:40\tManagement Response
// 10:00-10:40\t - James R. McQuiggan, CISSP

// 11:00-11:40\tThe Next Internal Network: Why Your
// 11:00-11:40\tOld Playbook Doesn't Work
// 11:00-11:40\t - Dumi Masimini

// 14:00-14:40\tDomain Protection - How We Protect
// 14:00-14:40\tOur Customers
// 14:00-14:40\t - Martin Clarke

// 15:00-15:40\tThe biggest digital sovereignty risk
// 15:00-15:40\tisn’t what you think...
// 15:00-15:40\t - Matt Johnson

// 16:00-16:40\tGetting started in embedded device
// 16:00-16:40\tresearch
// 16:00-16:40\t - Andy Monaghan

// 17:00-17:20\tHow Efficiency Can Lead to Disruption:
// 17:00-17:20\tThe Evolution of Phishing Emails
// 17:00-17:20\t - Becky Stacey

// "
//         );
//     }
// }
