use chrono::NaiveTime;
use color_eyre::Result;
use serde::{Deserialize, Serialize};
use std::io::Write;

/*
Start 	Excel time | string 	00:00:00
Link start 	boolean 	false
End 	Excel time | string 	00:00:00
Duration 	Excel time | string 	00:00:00
Cue 	string 	“”
Title 	string 	“”
Skip 	boolean 	false
Note 	string 	“”
Colour 	string (# hex colour or named css colour) 	“”
End action 	none load-next play-next 	none
Timer type* 	count-down count-up clock block skip group group-end milestone none 	count-down
Count to end* 	boolean 	false
Time warning 	Excel time | string 	00:02:00
Time danger 	Excel time | string 	00:01:00
*/

pub fn make_ontime_export<W: Write>(
    writer: W,
    track: &crate::show_file::ShowFile,
) -> Result<()> {
    let time_warning = "00:05:00".parse::<NaiveTime>()?;
    let time_danger = "00:01:00".parse::<NaiveTime>()?;

    let mut writer = csv::WriterBuilder::new()
        .has_headers(true)
        .from_writer(writer);

    let mut intermission = OntimeExport {
        start: String::new(),
        link_start: false,
        end: String::new(),
        duration: String::new(),
        cue: "Intermission".into(),
        title: "Intermission".into(),
        skip: false,
        note: String::new(),
        colour: "yellow".into(),
        end_action: EndAction::None,
        time_ty: TimerType::Clock,
        count_to_end: true,
        time_warning: time_warning.to_string(),
        time_danger: time_danger.to_string(),
    };

    let sessions = track
        .sessions
        .iter()
        .map(Some)
        .chain(std::iter::once(None))
        .collect::<Vec<_>>();
    for [talk1, talk2] in sessions.array_windows() {
        #[expect(clippy::unwrap_used)]
        let talk1 = talk1.unwrap();
        let duration = talk1.end - talk1.start;
        let duration = format!(
            "{:02}:{:02}:{:02}",
            duration.num_hours(),
            duration.num_minutes() % 60,
            duration.num_seconds() % 60,
        );
        let data = OntimeExport {
            start: talk1.start.to_string(),
            link_start: false,
            end: talk1.end.to_string(),
            duration,
            cue: talk1.title.clone(),
            title: talk1.title.clone(),
            skip: false,
            note: String::new(),
            colour: "green".into(),
            end_action: EndAction::None,
            time_ty: TimerType::CountDown,
            count_to_end: true,
            time_warning: time_warning.to_string(),
            time_danger: time_danger.to_string(),
        };

        writer.serialize(&data)?;

        let duration = if let Some(talk2) = talk2 {
            // make an intermission
            intermission.start = talk1.end.to_string();
            intermission.end = talk2.start.to_string();

            talk1.end - talk2.start
        } else {
            // make an end of day
            intermission.start = talk1.end.to_string();
            let end = "23:59".parse::<NaiveTime>()?;
            intermission.end = end.to_string();

            intermission.cue = "End".into();
            intermission.title = "End of day".into();

            end - talk1.start
        };

        let duration = format!(
            "{:02}:{:02}:{:02}",
            duration.num_hours(),
            duration.num_minutes() % 60,
            duration.num_seconds() % 60,
        );
        intermission.duration = duration.to_string();
        writer.serialize(&intermission)?;
    }

    writer.flush()?;
    Ok(())
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
struct OntimeExport {
    #[serde(rename = "Time Start")]
    start: String,
    #[serde(rename = "Link start")]
    link_start: bool,
    #[serde(rename = "Time End")]
    end: String,
    duration: String,
    cue: String,
    title: String,
    skip: bool,
    note: String,
    colour: String,
    #[serde(rename = "End action")]
    end_action: EndAction,
    #[serde(rename = "Timer type")]
    time_ty: TimerType,
    #[serde(rename = "Count to end")]
    count_to_end: bool,
    #[serde(rename = "Warning time")]
    time_warning: String,
    #[serde(rename = "Danger time")]
    time_danger: String,
}

#[derive(
    Copy, Clone, Debug, Default, PartialEq, Eq, Deserialize, Serialize,
)]
#[serde(rename_all = "kebab-case")]
pub enum EndAction {
    #[default]
    None,
    LoadNext,
    PlayNext,
}

#[derive(
    Copy, Clone, Debug, Default, PartialEq, Eq, Deserialize, Serialize,
)]
#[serde(rename_all = "kebab-case")]
pub enum TimerType {
    #[default]
    CountDown,
    CountUp,
    Clock,
    Block,
    Skip,
    Group,
    GroupEnd,
    Milestone,
    None,
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::formats::pretalx::Pretalx;
    use std::io::Cursor;

    const BRISTOL: &str = include_str!("../test-data/bristol-2026.json");

    #[test]
    fn test_deserialise() {
        let _ = serde_json::from_str::<Pretalx>(BRISTOL).unwrap();
    }

    #[test]
    fn test_show_file() {
        let mut bristol = serde_json::from_str::<Pretalx>(BRISTOL).unwrap();

        let mut file = Vec::<u8>::new();
        let mut cur = Cursor::new(&mut file);

        let t1 = bristol.schedule.conference.days[0]
            .rooms
            .swap_remove("Track 1")
            .unwrap();
        let show = crate::show_file::ShowFile::try_from(t1).unwrap();

        make_ontime_export(&mut cur, &show).unwrap();

        let out = String::from_utf8(file).unwrap();
        assert_eq!(
            out,
            r#"Time Start,Link start,Time End,Duration,Cue,Title,Skip,Note,Colour,End action,Timer type,Count to end,Warning time,Danger time
08:55:00,false,09:05:00,00:10:00,Opening Notes Friday,Opening Notes Friday,false,,green,none,count-down,true,00:05:00,00:01:00
09:05:00,false,09:05:00,00:00:00,Intermission,Intermission,false,,yellow,none,clock,true,00:05:00,00:01:00
09:05:00,false,09:45:00,00:40:00,Keynote - Managing The Minefield of Management Cyber Explosions - Understanding what to say and when to say it,Keynote - Managing The Minefield of Management Cyber Explosions - Understanding what to say and when to say it,false,,green,none,count-down,true,00:05:00,00:01:00
09:45:00,false,10:00:00,00:-15:00,Intermission,Intermission,false,,yellow,none,clock,true,00:05:00,00:01:00
10:00:00,false,10:40:00,00:40:00,"Fake It Till You Detect It: Live Deepfakes, Detection Gaps, and the Human Risk Management Response","Fake It Till You Detect It: Live Deepfakes, Detection Gaps, and the Human Risk Management Response",false,,green,none,count-down,true,00:05:00,00:01:00
10:40:00,false,11:00:00,00:-20:00,Intermission,Intermission,false,,yellow,none,clock,true,00:05:00,00:01:00
11:00:00,false,11:40:00,00:40:00,The Next Internal Network: Why Your Old Playbook Doesn't Work,The Next Internal Network: Why Your Old Playbook Doesn't Work,false,,green,none,count-down,true,00:05:00,00:01:00
11:40:00,false,14:00:00,-2:-20:00,Intermission,Intermission,false,,yellow,none,clock,true,00:05:00,00:01:00
14:00:00,false,14:40:00,00:40:00,Domain Protection - How We Protect Our Customers,Domain Protection - How We Protect Our Customers,false,,green,none,count-down,true,00:05:00,00:01:00
14:40:00,false,15:00:00,00:-20:00,Intermission,Intermission,false,,yellow,none,clock,true,00:05:00,00:01:00
15:00:00,false,15:40:00,00:40:00,The biggest digital sovereignty risk isn’t what you think...,The biggest digital sovereignty risk isn’t what you think...,false,,green,none,count-down,true,00:05:00,00:01:00
15:40:00,false,16:00:00,00:-20:00,Intermission,Intermission,false,,yellow,none,clock,true,00:05:00,00:01:00
16:00:00,false,16:40:00,00:40:00,Getting started in embedded device research,Getting started in embedded device research,false,,green,none,count-down,true,00:05:00,00:01:00
16:40:00,false,17:00:00,00:-20:00,Intermission,Intermission,false,,yellow,none,clock,true,00:05:00,00:01:00
17:00:00,false,17:20:00,00:20:00,How Efficiency Can Lead to Disruption: The Evolution of Phishing Emails,How Efficiency Can Lead to Disruption: The Evolution of Phishing Emails,false,,green,none,count-down,true,00:05:00,00:01:00
17:20:00,false,23:59:00,06:59:00,End,End of day,false,,yellow,none,clock,true,00:05:00,00:01:00
"#,
        );
    }
}
