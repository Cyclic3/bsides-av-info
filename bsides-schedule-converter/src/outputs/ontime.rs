use chrono::{NaiveTime, TimeDelta};
use color_eyre::Result;
use rust_xlsxwriter::Workbook;
use serde::{Deserialize, Serialize};
use std::{fmt::Display, io::Write};

use crate::{event::IndexedSession, outputs::EventFormatter};

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

const BREAK_COLOUR: &str = "yellow";
const TALK_COLOUR: &str = "green";

const TIME_WARN: TimeDelta = TimeDelta::minutes(5);
const TIME_DANGER: TimeDelta = TimeDelta::minutes(1);
// Time just before midnight for generating end-of-day cues
const MIDNIGHT: NaiveTime = NaiveTime::from_hms_opt(23, 59, 59).unwrap();

pub struct Ontime;
impl Display for Ontime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Ontime")
    }
}
impl EventFormatter for Ontime {
    fn format(&self, event: &crate::event::Event, output_dir: &std::path::Path) -> Result<()> {
        for (name, sessions) in super::standard_grouping(event) {
            let csv_file = std::fs::File::create(output_dir.join(format!("{name}-ontime.csv")))?;
            // Adding here consumes name, which mayyy be faster (but is very unlikely to be slower)
            let xlsx_file = std::fs::File::create(output_dir.join(name + "-ontime.xlsx"))?;
            make_ontime_export(csv_file,  xlsx_file, sessions)?;
        }
        Ok(())
    }
}

pub fn make_ontime_export<'a, W: Write + Send>(
    csv_writer: W,
    xlsx_writer: W,
    sessions: impl Iterator<Item=IndexedSession<'a>>,
) -> Result<()> {
    let mut csv_writer = csv::WriterBuilder::new()
        .has_headers(true)
        .from_writer(csv_writer);

    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();
    worksheet.deserialize_headers::<OntimeExport>(1, 1)?;

    let mut cue_number = 0;

    let sessions = sessions
        .map(Some)
        .chain(std::iter::once(None))
        .collect::<Vec<_>>();
    for [talk1, talk2] in sessions.array_windows() {
        // Talk

        #[expect(clippy::unwrap_used)]
        let talk1 = talk1.unwrap();

        let data = OntimeExport::talk(talk1, &mut cue_number);
        csv_writer.serialize(&data)?;
        worksheet.serialize(&data)?;

        let intermission_start = talk1.end().time();
        let (intermission_title, intermission_end) =
            if let Some(talk2) = talk2 {
                if talk1.end_utc == talk2.start_utc {
                    continue;
                }
                ("Intermission".into(),  talk2.start().time())
            } else {
                // make an end of day
                ("End of day".into(), MIDNIGHT)
            };

        let data = OntimeExport::intermission(intermission_start, intermission_end, intermission_title, &mut cue_number);
        csv_writer.serialize(&data)?;
        worksheet.serialize(&data)?;
    }

    csv_writer.flush()?;
    workbook.save_to_writer(xlsx_writer)?;
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

#[derive(Copy, Clone)]
enum SessionType {
    Talk,
    Break,
}

impl SessionType {
    // link_start is TRUE for TALK sessions
    const fn link_start(self) -> bool {
        matches!(self, Self::Talk)
    }

    fn colour(self) -> String {
        match self {
            Self::Talk => TALK_COLOUR.into(),
            Self::Break => BREAK_COLOUR.into(),
        }
    }

    // count_to_end is FALSE for TALK sessions
    const fn count_to_end(self) -> bool {
        matches!(self, Self::Break)
    }

    // time_ty is CountDown for talks and Clock for breaks
    const fn time_ty(self) -> TimerType {
        match self {
            Self::Talk => TimerType::CountDown,
            Self::Break => TimerType::Clock,
        }
    }
}

impl OntimeExport {
    fn new(start: NaiveTime, end: NaiveTime, ty: SessionType, title: String, cue_number: &mut i32) -> Self {
        let duration = format_duration(end - start);
        *cue_number += 1;
        Self {
            start: start.to_string(),
            link_start: ty.link_start(),
            end: end.to_string(),
            duration,
            cue: cue_number.to_string(),
            title,
            skip: false,
            colour: ty.colour(),
            end_action: EndAction::None,
            time_ty: ty.time_ty(),
            count_to_end: ty.count_to_end(),
            time_warning: format_duration(TIME_WARN),
            time_danger: format_duration(TIME_DANGER)
        }
    }
    fn intermission(start: NaiveTime, end: NaiveTime, title: String, cue_number: &mut i32) -> Self {
        Self::new(start, end, SessionType::Break, title, cue_number)
    }
    fn talk(session: IndexedSession, cue_number: &mut i32) -> Self {
        Self::new(session.start().time(), session.end().time(), SessionType::Talk, session.title.clone(), cue_number)
    }
}

fn format_duration(duration: TimeDelta) -> String {
    format!(
        "{:02}:{:02}:{:02}",
        duration.num_hours(),
        duration.num_minutes() % 60,
        duration.num_seconds() % 60,
    )
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
    use crate::{event::test::load_bristol, outputs::test::test_formatter, test_data_path};
    use std::{ffi::OsStr, io::Cursor};

    #[test]
    fn test_duration_fmt() {
        let mut duration = TimeDelta::seconds(0);
        assert_eq!(format_duration(duration), "00:00:00");
        duration += TimeDelta::seconds(15);
        assert_eq!(format_duration(duration), "00:00:15");
        duration += TimeDelta::minutes(2);
        assert_eq!(format_duration(duration), "00:02:15");
        duration += TimeDelta::minutes(2);
        assert_eq!(format_duration(duration), "00:04:15");
        duration += TimeDelta::minutes(59);
        assert_eq!(format_duration(duration), "01:03:15");
    }

    #[test]
    fn test_show_file() {
        let bristol = load_bristol();

        let mut csv_file = Vec::<u8>::new();
        let mut csv_cur = Cursor::new(&mut csv_file);
        let mut xlsx_file = Vec::<u8>::new();
        let mut xlsx_cur = Cursor::new(&mut xlsx_file);

        let t1 = bristol.filter()
            .start_day(0)
            .track_name("Track 1")
            .sessions();

        make_ontime_export(&mut csv_cur, &mut xlsx_cur, t1).unwrap();

        let out = String::from_utf8(csv_file).unwrap();
        assert_eq!(
            out,
            r#"Time Start,Link start,Time End,Duration,Cue,Title,Skip,Colour,End action,Timer type,Count to end,Warning time,Danger time
08:55:00,true,09:05:00,00:10:00,1,Opening Notes Friday,false,green,none,count-down,false,00:05:00,00:01:00
09:05:00,true,09:45:00,00:40:00,2,Keynote - Managing The Minefield of Management Cyber Explosions - Understanding what to say and when to say it,false,green,none,count-down,false,00:05:00,00:01:00
09:45:00,false,10:00:00,00:15:00,3,Intermission,false,yellow,none,clock,true,00:05:00,00:01:00
10:00:00,true,10:40:00,00:40:00,4,"Fake It Till You Detect It: Live Deepfakes, Detection Gaps, and the Human Risk Management Response",false,green,none,count-down,false,00:05:00,00:01:00
10:40:00,false,11:00:00,00:20:00,5,Intermission,false,yellow,none,clock,true,00:05:00,00:01:00
11:00:00,true,11:40:00,00:40:00,6,The Next Internal Network: Why Your Old Playbook Doesn't Work,false,green,none,count-down,false,00:05:00,00:01:00
11:40:00,false,14:00:00,02:20:00,7,Intermission,false,yellow,none,clock,true,00:05:00,00:01:00
14:00:00,true,14:40:00,00:40:00,8,Domain Protection - How We Protect Our Customers,false,green,none,count-down,false,00:05:00,00:01:00
14:40:00,false,15:00:00,00:20:00,9,Intermission,false,yellow,none,clock,true,00:05:00,00:01:00
15:00:00,true,15:40:00,00:40:00,10,The biggest digital sovereignty risk isn’t what you think...,false,green,none,count-down,false,00:05:00,00:01:00
15:40:00,false,16:00:00,00:20:00,11,Intermission,false,yellow,none,clock,true,00:05:00,00:01:00
16:00:00,true,16:40:00,00:40:00,12,Getting started in embedded device research,false,green,none,count-down,false,00:05:00,00:01:00
16:40:00,false,17:00:00,00:20:00,13,Intermission,false,yellow,none,clock,true,00:05:00,00:01:00
17:00:00,true,17:20:00,00:20:00,14,How Efficiency Can Lead to Disruption: The Evolution of Phishing Emails,false,green,none,count-down,false,00:05:00,00:01:00
17:20:00,false,23:59:59,06:39:59,15,End of day,false,yellow,none,clock,true,00:05:00,00:01:00
"#,
        );
    }

    #[test]
    fn test_bristol() {
        // FIXME: xlsx files do not seem to be fully stable, so we can't compare ;-;
        test_formatter(&Ontime, &load_bristol(), &test_data_path!("ontime"), |p| p.extension() != Some(OsStr::new("xlsx")));
    }

    // TODO: actually implement this
//     #[test]
//     fn test_show_file_with_breaks() {
//         let belfast = load_belfast();
//             serde_json::from_str::<SessionizeJson>(BELFAST_JSON).unwrap();

//         let mut csv_file = Vec::<u8>::new();
//         let mut csv_cur = Cursor::new(&mut csv_file);
//         let mut xlsx_file = Vec::<u8>::new();
//         let mut xlsx_cur = Cursor::new(&mut xlsx_file);

//         let event = crate::EventBuilder::try_from(belfast).unwrap();
//         make_ontime_export(
//             &mut csv_cur,
//             &mut xlsx_cur,
//             event.tracks.get("Track 1 - Exhibition Centre").unwrap(),
//         )
//         .unwrap();

//         let out = String::from_utf8(csv_file).unwrap();
//         assert_eq!(
//             out,
//             r#"Time Start,Link start,Time End,Duration,Cue,Title,Skip,Colour,End action,Timer type,Count to end,Warning time,Danger time
// 08:30:00,false,09:30:00,01:00:00,1,Registration (Grand Ballroom Entrance),false,yellow,none,clock,true,00:05:00,00:01:00
// 09:30:00,false,09:45:00,00:15:00,2,Opening remarks,false,yellow,none,clock,true,00:05:00,00:01:00
// 09:45:00,true,10:25:00,00:40:00,3,Morning Keynote,false,green,none,count-down,false,00:05:00,00:01:00
// 10:25:00,true,11:10:00,00:45:00,4,What the Real AI Attacks of the Last Two Years Actually Taught Us,false,green,none,count-down,false,00:05:00,00:01:00
// 11:10:00,false,11:30:00,00:20:00,5,Morning break,false,yellow,none,clock,true,00:05:00,00:01:00
// 11:30:00,true,12:15:00,00:45:00,6,"45,724 Networks, One Flipper Zero: Mapping the Wireless Threat Landscape of Northern Ireland",false,green,none,count-down,false,00:05:00,00:01:00
// 12:15:00,true,13:00:00,00:45:00,7,Hacking Browsers: The Easy Way,false,green,none,count-down,false,00:05:00,00:01:00
// 13:00:00,false,13:45:00,00:45:00,8,Lunch,false,yellow,none,clock,true,00:05:00,00:01:00
// 13:45:00,true,14:30:00,00:45:00,9,Reverse Engineering an OAuth Supply Chain Attack,false,green,none,count-down,false,00:05:00,00:01:00
// 14:30:00,true,15:15:00,00:45:00,10,"No CVV, No 3DS, No Problem: Chaining Trust Failures Across a Live Aviation Payment Stack",false,green,none,count-down,false,00:05:00,00:01:00
// 15:15:00,false,15:40:00,00:25:00,11,Afternoon break,false,yellow,none,clock,true,00:05:00,00:01:00
// 15:40:00,true,16:25:00,00:45:00,12,Chaining the Unchainable: Finding and Exploiting Logic Flaws in Modern Web Architectures,false,green,none,count-down,false,00:05:00,00:01:00
// 16:25:00,true,17:10:00,00:45:00,13,Leaks on the Livewire: How a Few Blocked Requests Led to a Shocking Discovery,false,green,none,count-down,false,00:05:00,00:01:00
// 17:10:00,true,17:50:00,00:40:00,14,Evening Keynote,false,green,none,count-down,false,00:05:00,00:01:00
// 17:50:00,false,18:10:00,00:20:00,15,Closing Remarks,false,yellow,none,clock,true,00:05:00,00:01:00
// 18:10:00,false,18:30:00,00:20:00,16,Intermission,false,yellow,none,clock,true,00:05:00,00:01:00
// 18:30:00,false,23:00:00,04:30:00,17,"After Party (Haymarket, wristbands required)",false,yellow,none,clock,true,00:05:00,00:01:00
// 23:00:00,false,23:59:59,00:59:59,18,End of day,false,yellow,none,clock,true,00:05:00,00:01:00
// "#,
//         );
//     }
}
