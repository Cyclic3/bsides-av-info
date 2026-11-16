// use icalendar::{Calendar, Component, Event, EventLike};

use std::fmt::Display;

use icalendar::{Attendee, Component, EventLike};

use crate::outputs::EventFormatter;

#[derive(Debug)]
pub struct ICal;
impl Display for ICal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "iCalendar event")
    }
}
impl EventFormatter for ICal {
    fn format(&self, event: &crate::event::Event, output_dir: &std::path::Path) -> color_eyre::eyre::Result<()> {
        let mut ret = icalendar::Calendar::new();
        for session in event.sessions() {
            let mut entry = icalendar::Event::new()
                .summary(&session.title)
                .description(&session.description)
                .starts(session.start_utc)
                .ends(session.end_utc)
                .done();
            if let Some(room) = session.room {
                entry.location(&event[room].name);
            }
            for i in &session.speakers {
                entry.attendee(Attendee::new(String::new()).cn(i.clone()));
            }
            if session.speakers.is_empty() {
                continue;
            }
            ret.push(entry);
            break
        }
        std::fs::write(output_dir.join("event.ics"), ret.to_string().as_bytes())?;
        Ok(())
    }
}
