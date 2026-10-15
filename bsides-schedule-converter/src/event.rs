//! You should fill an Event from your input (which won't care about order much), run Event::index,
//! and then use the EventIndex for your output (which will be nicely time ordered and indexable).
//!
//! The Event type implements Serialize and Deserialize to keep the tests working

use std::{collections::BinaryHeap, ops::Index};

use chrono::{DateTime, Utc};
use chrono_tz::Tz;
use indexmap::IndexMap;
use itertools::Itertools;
use serde::{Deserialize, Serialize};

use crate::tz::TzOrOffset;

/// Wrapper to ensure that we got the track index from a sane source
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[derive(Serialize, Deserialize)]
#[serde(transparent)]
#[repr(transparent)]
pub struct TrackId(pub usize);

/// Wrapper to ensure that we got the room index from a sane source
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[derive(Serialize, Deserialize)]
#[serde(transparent)]
#[repr(transparent)]
pub struct RoomId(pub usize);

/// Wrapper to ensure that we got the session index from a sane source
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[derive(Serialize, Deserialize)]
#[serde(transparent)]
#[repr(transparent)]
pub struct SessionId(usize);

// Both of these are currently single elements. This does mean that we get a (currently) pointless
// double allocation, but means that adding extra stuff, such as accessibility/AV per-room info,
// or track descriptions, or whatever, can be done really easily by just adding a field.

#[derive(Clone, Debug, PartialEq, Eq)]
#[derive(Serialize, Deserialize)]
pub struct TrackInfo {
    pub name: String
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[derive(Serialize, Deserialize)]
pub struct RoomInfo {
    pub name: String
}

/// Reference the tracks without having to do a dynamic allocation each time (yay!
///
/// XXX: supports a *maximum* of 64 tracks. Why do you have more than this? Please genuinely tell me.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[derive(Serialize, Deserialize)]
pub struct TrackSet(u64);
impl TrackSet {
    /// Create an empty trackset
    pub const fn new() -> Self { Self(0) }
    pub const fn add(&mut self, track: TrackId) {
        self.0 |= 1 << track.0
    }
    pub const fn remove(&mut self, track: TrackId) {
        self.0 &= !(1 << track.0)
    }
    pub const fn contains(&self, track: TrackId) -> bool {
        self.0 & (1<<track.0) != 0
    }
    pub const fn is_empty(&self) -> bool {
        self.0 == 0
    }
    pub fn iter(&self) -> impl DoubleEndedIterator<Item=TrackId> {
        (0..u64::BITS).map(|i| TrackId(i as usize)).filter(|i| self.contains(*i))
    }
}
impl FromIterator<TrackId> for TrackSet {
    fn from_iter<T: IntoIterator<Item = TrackId>>(iter: T) -> Self {
        let mut ret = Self::new();
        for i in iter {
            ret.add(i);
        }
        ret
    }
}

/// All of the information on a session that could be relevant to any output
#[derive(Clone, Debug)]
#[derive(Serialize, Deserialize)]
pub struct Session {
    pub title: String,
    pub description: String,
    pub speakers: Vec<String>,
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    /// The tracks that this session is in
    pub tracks: TrackSet,
    /// The physical location of this session. If this is none, just use the tracks with commas between them
    pub room: Option<RoomId>,
}
impl Session {
    pub fn clashes_with(&self, other: &Self) -> bool {
        self.end >= other.start && self.start <= other.end
    }
    pub fn runs_during(&self, time: chrono::DateTime<Utc>) -> std::cmp::Ordering {
        if time < self.start {
            std::cmp::Ordering::Less
        }
        else if time > self.end {
            std::cmp::Ordering::Greater
        }
        else {
            std::cmp::Ordering::Equal
        }
    }
}
// Two sessions are equal if they happen in the same tracks, at the same time
impl PartialEq for Session {
    fn eq(&self, other: &Self) -> bool {
        self.start == other.start && self.end == other.end && self.tracks == other.tracks
    }
}
// impl Eq for Session {}

// Check to see if a session runs before another
impl PartialOrd for Session {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        if self.end < other.start {
            Some(std::cmp::Ordering::Less)
        }
        else if self.start > other.end {
            Some(std::cmp::Ordering::Greater)
        }
        else if self.eq(other) {
            Some(std::cmp::Ordering::Equal)
        }
        else {
            None
        }
    }
}

#[repr(transparent)]
#[derive(Debug, Clone)]
#[derive(Serialize, Deserialize)]
#[serde(transparent)]
struct TimeOrderedSession(Session);
impl PartialEq for TimeOrderedSession {
    fn eq(&self, other: &Self) -> bool {
        self.0.start == other.0.start
    }
}
impl Eq for TimeOrderedSession {}
impl Ord for TimeOrderedSession {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Revese the order because rust uses max heaps for some reason
        //
        // This will be stable AS LONG AS you don't put two things on the same tracks with the same start time with the same title
        //
        // obviously
        self.0.start.cmp(&other.0.start)
        .then_with(|| self.0.tracks.0.cmp(&other.0.tracks.0))
        // We could remove this, for some nebulous perf gain and a lack of stability when someone screws up tracks
        .then_with(|| self.0.title.cmp(&other.0.title))
    }
}
impl PartialOrd for TimeOrderedSession {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// Put all of your events in here
//
// This has a hard invariant: we must make sure that no index changes, so ONLY APPEND!!!
#[derive(Debug, Clone)]
#[derive(Serialize, Deserialize)]
pub struct Event {
    pub time_zone: TzOrOffset,
    // Name indexed tracks
    tracks: IndexMap<String, TrackInfo>,
    // Name indexed rooms
    rooms: IndexMap<String, RoomInfo>,
    sessions: BinaryHeap<TimeOrderedSession>,
}
impl Event {
    pub fn new(time_zone: TzOrOffset) -> Self {
        Self {
            time_zone,
            tracks: Default::default(),
            rooms: Default::default(),
            sessions: Default::default(),
        }
    }

    // pub fn add_room(&mut self, name: String) -> Result<RoomId> {
    //     let ret = RoomId(self.rooms.len());
    //     if !self.rooms.insert(name) {
    //         // TODO: maybe gracefully handle this
    //         panic!("")
    //     }
    //     ret
    // }
    pub fn get_room_by_name(&self, name: &str) -> Option<RoomId> {
        self.rooms.get_index_of(name).map(RoomId)
    }
    pub fn get_track_by_name(&self, name: &str) -> Option<TrackId> {
        self.tracks.get_index_of(name).map(TrackId)
    }
    pub fn get_or_create_room<Name: AsRef<str>>(&mut self, name: Name, create: impl FnOnce(Name) -> RoomInfo) -> RoomId {
        self.get_room_by_name(name.as_ref()).unwrap_or_else(|| {
            let ret = RoomId(self.rooms.len());
            let room = create(name);
            if self.rooms.insert(room.name.clone(), room).is_some() {
                panic!("Provided room name collided with created room name");
            }
            ret
        })
    }
    pub fn get_or_create_track<Name: AsRef<str>>(&mut self, name: Name, create: impl FnOnce(Name) -> TrackInfo) -> TrackId {
        self.get_track_by_name(name.as_ref()).unwrap_or_else(|| {
            let ret = TrackId(self.tracks.len());
            let track = create(name);
            if self.tracks.insert(track.name.clone(), track).is_some() {
                panic!("Provided track name collided with created track name");
            }
            ret
        })
    }
    // We don't return a "SessionIdx", because sessions do not need to refer to each other
    // and we don't want to have to juggle a nice "SessionIdx" with an evil "OptionSessionIdx"
    pub fn add_session(&mut self, session: Session) {
        self.sessions.push(TimeOrderedSession(session));
    }

    // We take ownership to make sure that noone invalidates our index
    pub fn finish(self) -> EventIndex {
        EventIndex::new(self)
    }
}
impl Index<TrackId> for Event {
    type Output = TrackInfo;

    fn index(&self, index: TrackId) -> &Self::Output {
        // This index must be valid unless it came from another event
        &self.tracks[index.0]
    }
}
impl Index<RoomId> for Event {
    type Output = RoomInfo;

    fn index(&self, index: RoomId) -> &Self::Output {
        // This index must be valid unless it came from another event
        &self.rooms[index.0]
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RoomIndex {
    pub info: RoomInfo,
    pub sessions: Vec<SessionId>
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TrackIndex {
    pub info: TrackInfo,
    pub sessions: Vec<SessionId>
}

/// Sorted events, that you can use something like index[track_id].sessions.map(EventIndex.index).binary_search_with(|i| i.runs_during(time))
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EventIndex {
    pub time_zone: TzOrOffset,
    // Sessions sorted by time
    sessions: Vec<Session>,
    tracks: IndexMap<String, TrackIndex>,
    rooms: IndexMap<String, RoomIndex>,
}
impl EventIndex {
    /// Returns the tracks in SessionId and time sorted order
    pub fn sessions(&self) -> impl ExactSizeIterator<Item=(SessionId, &Session)> + DoubleEndedIterator {
        self.sessions.iter().enumerate().map(|(idx, i)| (SessionId(idx), i))
    }
    /// Returns the tracks in TrackId sorted order
    pub fn tracks(&self) -> impl ExactSizeIterator<Item=(TrackId, &TrackIndex)> + DoubleEndedIterator {
        self.tracks.iter().enumerate().map(|(idx, (_, i))| (TrackId(idx), i))
    }
    pub fn rooms(&self) -> impl ExactSizeIterator<Item=(RoomId, &RoomIndex)> + DoubleEndedIterator {
        self.rooms.iter().enumerate().map(|(idx, (_, i))| (RoomId(idx), i))
    }

    pub fn new(mut event: Event) -> Self {
        // Take all of the events and sort them by time
        //
        // This (should!) optimise to a in_place_collect, so no-reallocations
        //
        // Once we get https://github.com/rust-lang/rust/issues/59278, we won't even need the intermediate vec
        let sessions: Vec<Session> = event.sessions.into_sorted_vec().into_iter().map(|i| i.0).collect();
        // We now need this index to _NEVER CHANGE_ or our cool indexes all become useless
        //
        // We could use Arc to avoid this, but it seems a shame to do that when we already have everything in a vec

        let mut tracks: IndexMap<String, TrackIndex> = event.tracks.into_iter().map(|i| (i.0, TrackIndex{info: i.1, sessions: vec![]})).collect();
        let mut rooms: IndexMap<String, RoomIndex> = event.rooms.into_iter().map(|i| (i.0, RoomIndex{info: i.1, sessions: vec![]})).collect();

        // Iterate through the events in time order, adding to the various event indexes in order
        for (idx, session) in sessions.iter().enumerate() {
            let session_id = SessionId(idx);
            for track in session.tracks.iter() {
                tracks[track.0].sessions.push(session_id);
            }
            if let Some(room) = session.room {
                rooms[room.0].sessions.push(session_id);
            }
        }

        Self {
            time_zone: event.time_zone,
            sessions,
            tracks,
            rooms
        }
    }
}
impl From<Event> for EventIndex {
    fn from(event: Event) -> Self {
        Self::new(event)
    }
}
// Now that it's all sorted, we can do efficient comparison
//
// If an event has the same rooms, tracks and events, then they're equal
impl PartialEq for EventIndex {
    fn eq(&self, other: &Self) -> bool {
        // Easy check first
        self.time_zone == other.time_zone &&
        // Check all the room info is the same.
        //
        // We're going to assume that both these and the tracks are in the same order, unless someone wants to make a separate id system
        self.rooms.iter().zip(&other.rooms).all(|(a, b)| a.1.info == b.1.info) &&
        // ASSUMING that there are no clashes, the sessions in a track must be in the same order, so we can actually do a direct comparison
        //
        // Asymptotically, building an intermediate translation table for events may be faster, but that's dumb so dumb comparison it is
        self.tracks.iter().zip(&other.tracks).all(|(a, b)| {
            a.1.info == b.1.info &&
            a.1.sessions.iter().zip(&b.1.sessions).all(|(our_session, their_session)| self[*our_session] == other[*their_session])
        })
    }
}
impl Eq for EventIndex {}

impl Index<TrackId> for EventIndex {
    type Output = TrackInfo;

    fn index(&self, index: TrackId) -> &Self::Output {
        // This index must be valid unless it came from another event
        &self.tracks[index.0].info
    }
}
impl Index<RoomId> for EventIndex {
    type Output = RoomInfo;

    fn index(&self, index: RoomId) -> &Self::Output {
        // This index must be valid unless it came from another event
        &self.rooms[index.0].info
    }
}
impl Index<SessionId> for EventIndex {
    type Output = Session;

    fn index(&self, index: SessionId) -> &Self::Output {
        // This index must be valid unless it came from another event
        &self.sessions[index.0]
    }
}

// FIXME: add tests
#[cfg(test)]
pub mod test {
    use crate::inputs::{EventParser, sessionize::Sessionize};

    use super::*;
    // This is the test case that we're going to run non-parsers against. We should probably add more later...
    //
    // Also, this means that all formatter tests will fail if the sessionize formatter dies, which isn't great
    pub fn load_belfast() -> Event {
        Sessionize.parse_from_str(include_str!("../test-data/sessionize/belfast.html"))
            .expect("THIS IS A DIFFERENT ISSUE! Sessionize parser is buggy")
    }

    #[test]
    fn test_index() {
        insta::assert_yaml_snapshot!(load_belfast().finish())
    }
}
