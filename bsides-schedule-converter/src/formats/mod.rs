use color_eyre::{Result, eyre::eyre};
use serde::{Deserialize, Serialize};

pub mod pretalx;
pub mod sessionize;

use pretalx::Pretalx;
use sessionize::Sessionize;

#[derive(Debug, PartialEq, Eq, Deserialize, Serialize)]
pub enum Format {
    Pretalx(Pretalx),
    SessionizeHtml(Sessionize),
}

impl TryFrom<Format> for crate::Event {
    type Error = color_eyre::Report;
    fn try_from(this: Format) -> Result<Self> {
        match this {
            Format::Pretalx(sched) => sched.try_into(),
            Format::SessionizeHtml(sched) => sched.try_into(),
        }
    }
}

impl Format {
    pub fn parse_from_str(input: &str) -> Result<Self> {
        if let Ok(pretalx) = serde_json::from_str(input) {
            Ok(Self::Pretalx(pretalx))
        } else if let Ok(sessionize) = input.parse() {
            Ok(Self::SessionizeHtml(sessionize))
        } else {
            Err(eyre!("Unable to detect schedule type"))
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    const BRISTOL: &str = include_str!("../../test-data/bristol-2026.json");
    const BELFAST: &str = include_str!("../../test-data/belfast.html");

    #[test]
    fn test_pretalx() {
        let format = Format::parse_from_str(BRISTOL).unwrap();
        insta::assert_yaml_snapshot!(format);
    }

    #[test]
    fn test_sessionize() {
        let format = Format::parse_from_str(BELFAST).unwrap();
        insta::assert_yaml_snapshot!(format);
    }
}
