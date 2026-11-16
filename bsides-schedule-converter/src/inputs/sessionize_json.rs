use std::{collections::BTreeMap, fmt::Display};

use chrono::{DateTime, Utc};
use color_eyre::{Result, eyre::{OptionExt, eyre}};
use itertools::Itertools;
use serde::{Deserialize, Serialize};

use crate::{event::{EventBuilder, RoomInfo, TrackInfo}, inputs::EventParser, tz::TzOrOffset};

pub struct SessionizeJson;
impl Display for SessionizeJson {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Sessionize JSON object")
    }
}

impl EventParser for SessionizeJson {
    fn parse_from_str(&self, data: &str) -> Result<crate::event::Event> {
        let obj = serde_json::from_str::<SessionizeJsonObject>(data)?;

        let mut event = EventBuilder::new();

        let mut track_lookup = BTreeMap::new();

        for room in obj.rooms {
            track_lookup.insert(room.id, (event.create_track(TrackInfo{name: room.name.clone()}), event.create_room(RoomInfo{name: room.name})));
        }
        let mut speaker_lookup = BTreeMap::new();
        for speaker in obj.speakers {
            speaker_lookup.insert(speaker.id, speaker.full_name);
        }

        for session in obj.sessions {
            let (track, room) =
                // Opening remarks and such like
                if session.is_service_session {
                    (event.get_or_create_track("Service Session", |_| TrackInfo { name: "Service Session".into() }), None)
                }
                else if let Some((track_id, room_id)) = track_lookup.get(&session.room_id).copied() {
                    (track_id, Some(room_id))
                }
                else {
                    return Err(eyre!("Invalid room id"));
                };

            event.add_session(crate::event::Session {
                title: session.title,
                description: session.description.unwrap_or_default(),
                speakers: session.speakers.iter().map(|id| speaker_lookup.get(id).cloned().ok_or_eyre("Invalid speaker id")).try_collect()?,
                start_utc: session.starts_at,
                end_utc: session.ends_at,
                tracks: std::iter::once(track).collect(),
                room
            });
        }

        println!("WARNING: Sessionize JSON doesn't indicate timezone, assuming everywhere is London. Please use a different format if this isn't close enough!");

        Ok(event.finish(TzOrOffset::TimeZone(chrono_tz::Europe::London)))
    }
}

#[derive(Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionizeJsonObject {
    sessions: Vec<Session>,
    speakers: Vec<Speaker>,
    categories: Vec<Category>,
    rooms: Vec<Room>,
}

#[derive(Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::pedantic)]
pub struct Session {
    id: String,
    title: String,
    description: Option<String>,
    starts_at: DateTime<Utc>,
    ends_at: DateTime<Utc>,
    is_service_session: bool,
    is_plenum_session: bool,
    speakers: Vec<String>,
    category_items: Vec<i64>,
    question_answers: Vec<()>,
    room_id: i64,
    status: Option<String>,
    is_informed: bool,
    is_confirmed: bool,
}

#[derive(Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::pedantic)]
pub struct Speaker {
    id: String,
    first_name: String,
    last_name: String,
    bio: String,
    tag_line: String,
    profile_picture: String,
    is_top_speaker: bool,
    sessions: Vec<i64>,
    full_name: String,
}

#[derive(Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Category {
    id: i64,
    title: String,
    items: Vec<CategoryItem>,
    sort: i64,
    r#type: String,
}

#[derive(Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoryItem {
    id: i64,
    name: String,
    sort: i64,
}

#[derive(Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Room {
    id: i64,
    name: String,
    sort: i64,
}

#[cfg(test)]
mod test {
    use super::*;

    const BELFAST_JSON: &str = include_str!("../../test-data/sessionize/belfast.json");

    #[test]
    fn test_parse() {
        let event = SessionizeJson.parse_from_str(BELFAST_JSON).unwrap();
        insta::assert_yaml_snapshot!(event);
    }

    #[test]
    fn test_session() {
        const JSON: &str = r#"
    {
      "id": "1291414",
      "title": "Morning Keynote",
      "description": "The morning keynote session",
      "startsAt": "2026-09-10T08:45:00Z",
      "endsAt": "2026-09-10T09:25:00Z",
      "isServiceSession": false,
      "isPlenumSession": true,
      "speakers": [],
      "categoryItems": [],
      "questionAnswers": [],
      "roomId": 76855,
      "liveUrl": null,
      "recordingUrl": null,
      "status": "Accepted",
      "isInformed": true,
      "isConfirmed": false
    }
"#;
        let parsed = serde_json::from_str::<Session>(JSON).unwrap();
        insta::assert_yaml_snapshot!(parsed);
    }

    #[test]
    fn test_speaker() {
        const JSON: &str = r#"
    {
      "id": "902f3fb5-ae5d-4e22-aa02-ee8c2361838e",
      "firstName": "",
      "lastName": "",
      "bio": "Adams has worked as an engineering lead in the...",
      "tagLine": "Principal Engineer, AI @ Workday",
      "profilePicture": "https://cdn.sessionize.com/image/--.png",
      "isTopSpeaker": false,
      "links": [],
      "sessions": [
        1258496
      ],
      "fullName": "Adam",
      "categoryItems": [],
      "questionAnswers": []
    }
"#;
        let parsed = serde_json::from_str::<Speaker>(JSON).unwrap();
        insta::assert_yaml_snapshot!(parsed);
    }

    #[test]
    fn test_category() {
        const JSON: &str = r#"
    {
      "id": 117963,
      "title": "Session format",
      "items": [
        {
          "id": 425309,
          "name": "Lightning talk",
          "sort": 1
        },
        {
          "id": 425310,
          "name": "Session",
          "sort": 2
        },
        {
          "id": 425311,
          "name": "Workshop",
          "sort": 3
        }
      ],
      "sort": 0,
      "type": "session"
    }
"#;
        let parsed = serde_json::from_str::<Category>(JSON).unwrap();
        insta::assert_yaml_snapshot!(parsed);
    }

    #[test]
    fn test_roomy() {
        const JSON: &str = r#"
    {
      "id": 76855,
      "name": "Track 1 - Exhibition Centre",
      "sort": 0
    }
"#;
        let parsed = serde_json::from_str::<Room>(JSON).unwrap();
        insta::assert_yaml_snapshot!(parsed);
    }
}
