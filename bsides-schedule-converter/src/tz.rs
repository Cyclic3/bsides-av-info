//! Some formats (*cough* sessionize *cough*) give offsets instead of proper time zones, so we have to cover both cases

use std::{fmt::Display, str::FromStr};

use chrono::{FixedOffset, NaiveDateTime, Offset, TimeZone};
use chrono_tz::{Tz, TzOffset};

use color_eyre::{eyre::eyre, Result};
use itertools::Itertools;
use serde_with::{DeserializeFromStr, SerializeDisplay};


#[derive(Clone, Copy, Debug)]
pub enum TzOrOffsetOffset {
    TimeZone(TzOffset),
    FixedOffset(FixedOffset)
}
impl Offset for TzOrOffsetOffset {
    fn fix(&self) -> FixedOffset {
        match self {
            TzOrOffsetOffset::TimeZone(tz_offset) => tz_offset.fix(),
            TzOrOffsetOffset::FixedOffset(fixed_offset) => *fixed_offset,
        }
    }
}
impl Display for TzOrOffsetOffset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TzOrOffsetOffset::TimeZone(tz_offset) => tz_offset.fmt(f),
            TzOrOffsetOffset::FixedOffset(fixed_offset) => fixed_offset.fmt(f),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[derive(SerializeDisplay, DeserializeFromStr)]
pub enum TzOrOffset {
    TimeZone(Tz),
    FixedOffset(FixedOffset)
}
impl TimeZone for TzOrOffset {
    type Offset = TzOrOffsetOffset;

    fn from_offset(offset: &Self::Offset) -> Self {
        match offset {
            TzOrOffsetOffset::TimeZone(tz_offset) => Self::TimeZone(Tz::from_offset(tz_offset)),
            TzOrOffsetOffset::FixedOffset(fixed_offset) => Self::FixedOffset(*fixed_offset),
        }
    }

    fn offset_from_local_date(&self, local: &chrono::prelude::NaiveDate) -> chrono::MappedLocalTime<Self::Offset> {
        match self {
            TzOrOffset::TimeZone(tz) => tz.offset_from_local_date(local).map(Self::Offset::TimeZone),
            TzOrOffset::FixedOffset(fixed_offset) => fixed_offset.offset_from_local_date(local).map(Self::Offset::FixedOffset),
        }
    }

    fn offset_from_local_datetime(&self, local: &NaiveDateTime) -> chrono::MappedLocalTime<Self::Offset> {
        match self {
            TzOrOffset::TimeZone(tz) => tz.offset_from_local_datetime(local).map(Self::Offset::TimeZone),
            TzOrOffset::FixedOffset(fixed_offset) => fixed_offset.offset_from_local_datetime(local).map(Self::Offset::FixedOffset),
        }
    }

    fn offset_from_utc_date(&self, utc: &chrono::prelude::NaiveDate) -> Self::Offset {
        match self {
            TzOrOffset::TimeZone(tz) => Self::Offset::TimeZone(tz.offset_from_utc_date(utc)),
            TzOrOffset::FixedOffset(fixed_offset) => Self::Offset::FixedOffset(fixed_offset.offset_from_utc_date(utc)),
        }
    }

    fn offset_from_utc_datetime(&self, utc: &NaiveDateTime) -> Self::Offset {
        match self {
            TzOrOffset::TimeZone(tz) => Self::Offset::TimeZone(tz.offset_from_utc_datetime(utc)),
            TzOrOffset::FixedOffset(fixed_offset) => Self::Offset::FixedOffset(fixed_offset.offset_from_utc_datetime(utc)),
        }
    }
}
impl FromStr for TzOrOffset {
    type Err = color_eyre::eyre::Error;

    fn from_str(s: &str) -> Result<Self> {
        if let Ok(tz) = Tz::from_str(s) {
            return Ok(Self::TimeZone(tz))
        }
        let trimmed = s.strip_prefix("UTC").unwrap_or(s);
        let sign =
            match trimmed.chars().next() {
                Some('+') => 1i32,
                Some('-') => -1i32,
                Some(other) => return Err(eyre!("Invalid UTC offset sign {other:?}")),
                None => return Ok(Self::TimeZone(chrono_tz::UTC))
            };
        // chrono::FixedOffset::from_str doesn't allow UTC+01, it needs minutes for some reason
        let time: u32 = trimmed[1..].splitn(3, ":")
            .map(u32::from_str)
            .process_results(|i|
                i.enumerate()
                .map(|(idx, value)| 60u32.pow(2 - idx as u32) * value)
                .sum()
            )?;
        chrono::FixedOffset::east_opt(time as i32 * sign)
        .map(Self::FixedOffset)
        .ok_or_else(|| eyre!("Value was neither a valid time zone nor a valid UTC offset"))
    }
}
impl Display for TzOrOffset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TzOrOffset::TimeZone(tz) => tz.fmt(f),
            TzOrOffset::FixedOffset(fixed_offset) => write!(f, "UTC{fixed_offset}"),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_deserialise_tz() {
        assert_eq!("Europe/London".parse::<TzOrOffset>().expect("Could not parse time zone"), TzOrOffset::TimeZone(Tz::Europe__London))
    }


    #[test]
    fn test_deserialise_offset() {
        assert_eq!("UTC+01:00".parse::<TzOrOffset>().expect("Could not parse offset"), TzOrOffset::FixedOffset(FixedOffset::east_opt(3600).unwrap()));
        assert_eq!("UTC+01".parse::<TzOrOffset>().expect("Could not parse offset"), TzOrOffset::FixedOffset(FixedOffset::east_opt(3600).unwrap()));
        assert_eq!("UTC-03".parse::<TzOrOffset>().expect("Could not parse offset"), TzOrOffset::FixedOffset(FixedOffset::west_opt(3600 * 3).unwrap()));
        assert_eq!("UTC-03:15".parse::<TzOrOffset>().expect("Could not parse offset"), TzOrOffset::FixedOffset(FixedOffset::west_opt(3600* 3 + 15 * 60).unwrap()));
    }

    #[test]
    fn test_serialise_tz() {
        assert_eq!(TzOrOffset::TimeZone(Tz::Europe__London).to_string(), "Europe/London")
    }

    #[test]
    fn test_serialise_offset() {
        assert_eq!(TzOrOffset::FixedOffset(FixedOffset::east_opt(3600).unwrap()).to_string(), "UTC+01:00")
    }
}
