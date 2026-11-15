//! You should fill an Event from your input (which won't care about order much), run Event::index,
//! and then use the EventIndex for your output (which will be nicely time ordered and indexable).
//!
//! The Event type implements Serialize and Deserialize to keep the tests working

use std::{collections::BinaryHeap, fmt::Debug, ops::{Deref, Index, Range, RangeBounds}};

use chrono::{DateTime, Days, NaiveDate, NaiveTime, TimeDelta, TimeZone, Utc};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

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

/// Iterates over the bits in a trackset
pub struct TrackSetIterator {
    remaining_tracks: u64,
    pos: usize
}
impl Iterator for TrackSetIterator {
    type Item = TrackId;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.remaining_tracks == 0 {
                return None
            }
            if self.remaining_tracks & 1 != 0 {
                let ret = Some(TrackId(self.pos));
                self.remaining_tracks >>= 1;
                self.pos += 1;
                return ret
            }
            else {
                self.remaining_tracks >>= 1;
                self.pos += 1;
            }
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.len();
        (len, Some(len))
    }
}
impl ExactSizeIterator for TrackSetIterator {
    fn len(&self) -> usize {
        self.remaining_tracks.count_ones() as usize
    }
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
    pub const fn intersection(&self, other: TrackSet) -> Self {
        Self(self.0 & other.0)
    }
    pub const fn is_empty(&self) -> bool {
        self.0 == 0
    }
    pub fn iter(&self) -> TrackSetIterator {
        TrackSetIterator { remaining_tracks: self.0, pos: 0 }
    }
    pub fn len(&self) -> usize {
        self.0.count_ones() as usize
    }
}
impl IntoIterator for TrackSet {
    type Item = TrackId;

    type IntoIter = TrackSetIterator;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
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
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[derive(Serialize, Deserialize)]
pub struct Session {
    pub title: String,
    pub description: String,
    pub speakers: Vec<String>,
    pub start_utc: DateTime<Utc>,
    pub end_utc: DateTime<Utc>,
    /// The tracks that this session is in
    pub tracks: TrackSet,
    /// The physical location of this session. If this is none, just use the tracks with commas between them
    pub room: Option<RoomId>,
}
impl Session {
    pub fn clashes_with(&self, other: &Self) -> bool {
        self.end_utc >= other.start_utc && self.start_utc <= other.end_utc
    }
    pub fn runs_during(&self, time: chrono::DateTime<Utc>) -> std::cmp::Ordering {
        if time < self.start_utc {
            std::cmp::Ordering::Less
        }
        else if time > self.end_utc {
            std::cmp::Ordering::Greater
        }
        else {
            std::cmp::Ordering::Equal
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
        self.0.start_utc == other.0.start_utc
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
        self.0.start_utc.cmp(&other.0.start_utc)
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
#[derive(Default, Serialize, Deserialize)]
pub struct EventBuilder {
    // Name indexed tracks
    tracks: IndexMap<String, TrackInfo>,
    // Name indexed rooms
    rooms: IndexMap<String, RoomInfo>,
    sessions: BinaryHeap<TimeOrderedSession>,
}
impl EventBuilder {
    pub fn new() -> Self {
        Default::default()
    }

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
    pub fn finish<T>(self, time_zone: T) -> Event where TzOrOffset: From<T> {
        Event::new(self, time_zone.into())
    }
}
impl Index<TrackId> for EventBuilder {
    type Output = TrackInfo;

    fn index(&self, index: TrackId) -> &Self::Output {
        // This index must be valid unless it came from another event
        &self.tracks[index.0]
    }
}
impl Index<RoomId> for EventBuilder {
    type Output = RoomInfo;

    fn index(&self, index: RoomId) -> &Self::Output {
        // This index must be valid unless it came from another event
        &self.rooms[index.0]
    }
}
// Makes writing some tests a lot easier if we can just EventIndex::filter().sessions().collect() into a new event
impl<'a> FromIterator<IndexedSession<'a>> for EventBuilder {
    fn from_iter<T: IntoIterator<Item = IndexedSession<'a>>>(iter: T) -> Self {
        let mut ret = EventBuilder::new();
        for i in iter {
            let room = i.room.map(|parent_room_id| {
                let parent_room = &i.event[parent_room_id];
                ret.get_or_create_room(&parent_room.name, |_| parent_room.info.clone())
            });
            let tracks = i.tracks.into_iter().map(|parent_track_id| {
                let parent_track = &i.event[parent_track_id];
                ret.get_or_create_track(&parent_track.name, |_| parent_track.info.clone())
            }).collect();
            // I'm explicitly listing this so that I don't miss anything when transfering ids
            ret.add_session(Session {
                title: i.title.clone(),
                description: i.description.clone(),
                speakers: i.speakers.clone(),
                start_utc: i.start_utc,
                end_utc: i.end_utc,
                tracks,
                room
            });
        }
        ret
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RoomIndex {
    pub info: RoomInfo,
    pub sessions: Vec<SessionId>
}
impl Deref for RoomIndex {
    type Target = RoomInfo;

    fn deref(&self) -> &Self::Target {
        &self.info
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TrackIndex {
    pub info: TrackInfo,
    pub sessions: Vec<SessionId>
}
impl Deref for TrackIndex {
    type Target = TrackInfo;

    fn deref(&self) -> &Self::Target {
        &self.info
    }
}

#[derive(Clone, Copy)]
pub struct IndexedSession<'a> {
    id: SessionId,
    event: &'a Event,
    session: &'a Session,
}
impl<'a> IndexedSession<'a> {
    #[inline(always)]
    pub fn event(&self) -> &'a Event {
        self.event
    }
    #[inline(always)]
    pub fn id(&self) -> SessionId {
        self.id
    }
    /// Get the start time in the event-local time zone
    pub fn start(&self) -> DateTime<TzOrOffset> {
        self.session.start_utc.with_timezone(&self.event.time_zone)
    }
    /// Get the end time in the event-local time zone
    pub fn end(&self) -> DateTime<TzOrOffset> {
        self.session.end_utc.with_timezone(&self.event.time_zone)
    }
    /// Get the index of the day in the event
    pub fn day(&self) -> u64 {
        let ret = (self.start() - self.event.start()).num_days();
        // This cannot be negative, as we came from a sorted list :(
        debug_assert!(ret >= 0, "Got an indexed session before the start of the event");
        ret as u64
    }
    /// Get a sequence of `day{day + 1}-{track_name}` that this session is in, skipping empty components
    pub fn maybe_day_track_name(&self) -> impl ExactSizeIterator<Item=String> {
        if self.event().single_day() {
            if self.event().tracks().len() == 1 {
                either::Left(either::Left(std::iter::once(Default::default())))
            }
            else {
                either::Left(either::Right(self.tracks.iter().map(move |track_id| self.event()[track_id].info.name.clone())))
            }
        }
        else {
            if self.event().tracks().len() == 1 {
                either::Right(either::Left(std::iter::once(format!("day{}", self.day() + 1))))
            }
            else {
                either::Right(either::Right(self.tracks.iter().map(|track_id| format!("day{}-{}", self.day() + 1, self.event()[track_id].info.name))))
            }
        }
    }
}
impl Debug for IndexedSession<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("IndexedSession").field(&self.session).finish()
    }
}
impl<'a> Deref for IndexedSession<'a> {
    type Target = &'a Session;

    fn deref(&self) -> &Self::Target {
        &self.session
    }
}
impl PartialEq for IndexedSession<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.session == other.session
    }
}
impl Eq for IndexedSession<'_> {}

///
/// Every collection of sessions and session ids in this object remain sorted, both by ids and time.
/// As well as being a nice property, it means that [`Event::filter`] is O(log(n)) for any of the exposed methods!
///
/// It also exposes a `FromIterator` method, which (reasonably) efficiently allows the following workflow:
/// ```rust
///     let bristol = crate::event::test::load_bristol();
///     let bristol_without_opener: crate::event::Event =
///         bristol.sessions()
///         .filter(|i| !i.title.starts_with("Opening Notes "))
///         .collect();
/// ```
#[serde_as]
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Event {
    pub time_zone: TzOrOffset,
    // Sessions sorted by time
    sessions: Vec<Session>,
    tracks: IndexMap<String, TrackIndex>,
    rooms: IndexMap<String, RoomIndex>,

    // These'll be referenced a lot, so we might as well precompute them
    #[serde_as(as = "crate::tz::Rfc9557DateTime")]
    start: DateTime<TzOrOffset>,
    #[serde_as(as = "crate::tz::Rfc9557DateTime")]
    end: DateTime<TzOrOffset>,
    last_day: NaiveDate
}
impl Event {
    /// Get the session with the given id
    //
    // Since they should have got the SessionId from us, any out of bounds access is a serious logic error,
    // so we don't give them a chance to recover
    pub fn session<'a>(&'a self, id: SessionId) -> IndexedSession<'a> {
        IndexedSession { event: self, session: &self.sessions[id.0], id }
    }
    /// Return the sessions by id
    pub fn sessions<'a>(&'a self) -> impl ExactSizeIterator<Item = IndexedSession<'a>> + DoubleEndedIterator {
        self.sessions.iter().enumerate().map(|(id, session)| IndexedSession { id: SessionId(id), event: self, session })
    }
    /// Returns the tracks in TrackId sorted order
    pub fn tracks(&self) -> impl ExactSizeIterator<Item=(TrackId, &TrackIndex)> + DoubleEndedIterator {
        self.tracks.iter().enumerate().map(|(idx, (_, i))| (TrackId(idx), i))
    }
    /// Returns the tracks in RoomId sorted order
    pub fn rooms(&self) -> impl ExactSizeIterator<Item=(RoomId, &RoomIndex)> + DoubleEndedIterator {
        self.rooms.iter().enumerate().map(|(idx, (_, i))| (RoomId(idx), i))
    }
    pub fn get_track_by_name(&self, name: &str) -> Option<TrackId> {
        self.tracks.get_index_of(name).map(TrackId)
    }
    pub fn get_room_by_name(&self, name: &str) -> Option<RoomId> {
        self.rooms.get_index_of(name).map(RoomId)
    }
    /// The start time of the first event, in the event-local time zone
    pub fn start(&self) -> DateTime<TzOrOffset> { self.start }
    /// The end time of the last event, in the event-local time zone
    pub fn end(&self) -> DateTime<TzOrOffset> { self.end }
    /// The first day of the event
    pub fn first_day(&self) -> NaiveDate { self.start.date_naive() }
    /// The last day of the event (checks for midnight)
    pub fn last_day(&self) -> NaiveDate { self.last_day }
    /// A more efficiently way than `self.days.len()` to check if there is only a single day in the event
    pub fn single_day(&self) -> bool { self.start.date_naive() == self.last_day }
    /// Iterate over each day the event runs
    pub fn days(&self) -> impl ExactSizeIterator<Item=NaiveDate> + DoubleEndedIterator {
        let first = self.start().date_naive();
        let mut last_dt = self.end();
        // Handle event ending on midnight: we don't get an extra day smh
        if last_dt.time() == NaiveTime::MIN {
            last_dt -= TimeDelta::seconds(1);
        }
        let last = last_dt.date_naive();
        let n_days_minus_1 = (last - first).num_days();
        // If your event is running for more than 10 million years, then you can make a PR
        assert!(n_days_minus_1 <= (u32::MAX as i64 - 1), "Event duration {first} to {last} is more than (2^32 - 2) days (>10 million years!)");
        // Has to be a u32 exclusive range so that we get a free exact size iterator impl
        (0u32..n_days_minus_1 as u32 + 1)
        .map(move |i| first + TimeDelta::days(i as i64))
    }

    /// Efficiently filter sessions by track/room/time or some combination thereof. The worst-case for any exposed method is O(log n)
    pub fn filter<'a>(&'a self) -> Filtered<'a, ()> {
        Filtered { event: self, id_range: 0..self.sessions.len(), session_ids: () }
    }

    pub fn new(event: EventBuilder, time_zone: TzOrOffset) -> Self {
        // Check that we actually have a session in our event
        if event.sessions.is_empty() {
            panic!("Asked to index an empty event");
        }

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

        let start =  sessions.first().unwrap().start_utc.with_timezone(&time_zone);
        let end = sessions.last().unwrap().end_utc.with_timezone(&time_zone);
        // Move back one second.
        //
        // If the entire event is zero seconds long and starts at midnight, then this will break.
        // Send a PR if you can actually justify why you would care
        let last_day = (end - TimeDelta::seconds(1)).date_naive();
        Self {
            start,
            end,
            last_day,
            time_zone,
            sessions,
            tracks,
            rooms,
        }
    }
}
// Now that it's all sorted, we can do efficient comparison
//
// If an event has the same rooms, tracks and events, then they're equal
impl PartialEq for Event {
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
            a.1.sessions.iter().zip(&b.1.sessions)
            .all(|(our_session, their_session)| self.session(*our_session) == self.session(*their_session))
        })
    }
}
impl Eq for Event {}
impl Index<TrackId> for Event {
    type Output = TrackIndex;

    fn index(&self, index: TrackId) -> &Self::Output {
        // This index must be valid unless it came from another event
        &self.tracks[index.0]
    }
}
impl Index<RoomId> for Event {
    type Output = RoomIndex;

    fn index(&self, index: RoomId) -> &Self::Output {
        // This index must be valid unless it came from another event
        &self.rooms[index.0]
    }
}
impl<'a> FromIterator<IndexedSession<'a>> for Event {
    fn from_iter<T: IntoIterator<Item = IndexedSession<'a>>>(iter: T) -> Self {
        let mut iter = iter.into_iter().peekable();
        let parent = iter.peek().expect("Cannot create an EventIndex from an empty session iterator: need timezone").event;
        let tz = parent.time_zone;
        Event::new(iter.collect(), tz)
    }
}

pub struct Filtered<'a, Inner> {
    pub event: &'a Event,
    id_range: Range<usize>,
    session_ids: Inner
}
impl<'a, Inner> Filtered<'a, Inner> {
    pub fn start_time<Tz: TimeZone>(self, time_range: impl RangeBounds<DateTime<Tz>>) -> Self {
        // TODO: don't actually need SortedSessionIdContainer any more!
        // A bsearch on all sessions is not really much slower than just the track/room we want!
        let ranged_sessions = &self.event.sessions[self.id_range.start..self.id_range.end];
        let start_offset =
            match time_range.start_bound() {
                std::ops::Bound::Included(start) => {
                   ranged_sessions.partition_point(|i| i.start_utc < *start)
                },
                std::ops::Bound::Excluded(start) => {
                    ranged_sessions.partition_point(|i| i.start_utc < *start)
                },
                std::ops::Bound::Unbounded => 0,
            };
        // We can further shrink the range, as the end is after the start (hopefully!)
        let new_len =
            match time_range.end_bound() {
                std::ops::Bound::Included(end) => {
                    ranged_sessions[start_offset..].partition_point(|i| i.start_utc <= *end)
                },
                std::ops::Bound::Excluded(end) => {
                    ranged_sessions[start_offset..].partition_point(|i| i.start_utc < *end)
                },
                std::ops::Bound::Unbounded => self.id_range.end,
            };
        Self {
            id_range: self.id_range.start + start_offset .. self.id_range.start + start_offset + new_len,
            .. self
        }
    }

    /// 0-indexed day of the event
    pub fn start_day(self, day: u64) -> Self {
        let first = self.event.first_day().checked_add_days(Days::new(day)).expect("Day overflow");
        let last = self.event.first_day().checked_add_days(Days::new(day + 1)).expect("Day overflow");
        self.start_date(first..last)
    }
    pub fn start_date(self, date: impl RangeBounds<NaiveDate>) -> Self {
        let tz = self.event.time_zone;
        let start = date.start_bound().map(|i| i.and_time(NaiveTime::MIN).and_local_timezone(tz).earliest().expect("Could not resolve start time to event-local timezone").to_utc());
        let end = date.end_bound().map(|i| i.and_time(NaiveTime::MIN).and_local_timezone(tz).earliest().expect("Could not resolve start time to event-local timezone").to_utc());
        self.start_time((start, end))
    }

}

// We can only efficiently filter for one of tracks or rooms
impl<'a> Filtered<'a, ()> {
    /// Returns the sessions in SessionId and time sorted order
    pub fn sessions(&self) -> impl ExactSizeIterator<Item = IndexedSession<'a>> + DoubleEndedIterator {
        self.id_range.clone().into_iter().map(|i| self.event.session(SessionId(i)))
    }

    pub fn track(self, id: TrackId) -> Filtered<'a, &'a [SessionId]> {
        Filtered { event: self.event, id_range: self.id_range, session_ids: &self.event[id].sessions }
    }
    pub fn track_name(self, name: &str) -> Filtered<'a, &'a [SessionId]> {
        let id = self.event.get_track_by_name(name).expect("Track name not found");
        self.track(id)
    }

    pub fn room(self, id: RoomId) -> Filtered<'a, &'a [SessionId]> {
        Filtered { event: self.event, id_range: self.id_range, session_ids: &self.event[id].sessions }
    }
    pub fn room_name(self, name: &str) -> Filtered<'a, &'a [SessionId]> {
        let id = self.event.get_room_by_name(name).expect("Room name not found");
        self.room(id)
    }
}
impl<'a> Filtered<'a, &'a[SessionId]> {
    /// Returns the sessions in SessionId and time sorted order
    pub fn sessions(&self) -> impl ExactSizeIterator<Item = IndexedSession<'a>> + DoubleEndedIterator {
        let start = self.session_ids.partition_point(|i| i.0 < self.id_range.start);
        // TODO: we could drop exact size iterator here if we cared enough,
        // but when you consider extra allocations and whatnot, we (I) don't
        let len = self.session_ids[start..].partition_point(|i| i.0 < self.id_range.end);
        self.session_ids[start..start + len].iter().map(|id| self.event.session(*id))
    }
}


// FIXME: add tests
#[cfg(test)]
pub mod test {
    use itertools::Itertools;

use crate::inputs::{EventParser, pretalx::Pretalx};
    use super::*;
    // This is the test case that we're going to run non-parsers against. We should probably add more later...
    //
    // It has multiple tracks and multiple days, so should be enough to mess something up if you weren't careful
    //
    // Also, this means that all formatter tests will fail if the pretalx formatter dies, which isn't great
    pub fn load_bristol() -> Event {
        Pretalx.parse_from_str(include_str!("../test-data/pretalx/bristol-2026.json"))
            .expect("THIS IS A DIFFERENT ISSUE! Sessionize parser is buggy")
    }

    #[test]
    fn test_index() {
        insta::assert_yaml_snapshot!(load_bristol())
    }

    #[test]
    fn test_time_track_filter() {
        use crate::tz::Rfc9557DateTime;
        let bristol = load_bristol();
        let first_time: Rfc9557DateTime = "2026-08-21T10:00:00+01:00".parse().expect("Could not parse first time");
        let last_time: Rfc9557DateTime = "2026-08-21T12:00:00+01:00".parse().expect("Could not parse last time");
        let res =
            bristol.filter()
            .start_time(DateTime::from(first_time)..DateTime::from(last_time))
            .track_name("Track 1")
            .sessions()
            .collect_vec();
        println!("Sessions: {res:#?}");
        assert_eq!(res.len(), 2, "Filtered sessions had wrong length");
        assert!(res[0].title.starts_with("Fake It Till You Detect It:"), "First session incorrect");
        assert!(res[1].title.starts_with("The Next Internal Network:"), "Second session incorrect");
    }

    #[test]
    fn test_double_time_track_filter() {
        use crate::tz::Rfc9557DateTime;
        let bristol = load_bristol();
        let first_time: Rfc9557DateTime = "2026-08-21T10:00:00+01:00".parse().expect("Could not parse first time");
        let last_time: Rfc9557DateTime = "2026-08-21T12:00:00+01:00".parse().expect("Could not parse last time");
        let new_last_time: Rfc9557DateTime = "2026-08-21T11:00:00+01:00".parse().expect("Could not parse last time");
        let res =
            bristol.filter()
            .start_time(DateTime::from(first_time)..DateTime::from(last_time))
            .track_name("Track 1")
            .start_time(DateTime::from(first_time)..DateTime::from(new_last_time))
            .sessions()
            .collect_vec();
        println!("Sessions: {res:#?}");
        assert_eq!(res.len(), 1, "Filtered sessions had wrong length");
        assert!(res[0].title.starts_with("Fake It Till You Detect It:"), "First session incorrect");
    }

    #[test]
    fn test_event_collect() {
        let bristol = crate::event::test::load_bristol();
        let bristol_without_opener: crate::event::Event =
            bristol.sessions()
            .filter(|i| !i.title.starts_with("Opening Notes "))
            .collect();
        assert_eq!(bristol.days().len(), bristol_without_opener.days().len());
        assert_eq!(bristol.sessions().len(), bristol_without_opener.sessions().len() + 2);
        assert_eq!(
            bristol.rooms.values().filter(|&i| !i.sessions.is_empty()).cloned().map(|i| i.info).sorted_by_cached_key(|i| i.name.clone()).collect_vec(),
            bristol_without_opener.rooms.values().cloned().map(|i| i.info).sorted_by_cached_key(|i| i.name.clone()).collect_vec(),
            "Room mismatch"
        );
        assert_eq!(
            bristol.tracks.values().filter(|&i| !i.sessions.is_empty()).cloned().map(|i| i.info).sorted_by_cached_key(|i| i.name.clone()).collect_vec(),
            bristol_without_opener.tracks.values().cloned().map(|i| i.info).sorted_by_cached_key(|i| i.name.clone()).collect_vec(),
            "Track mismatch"
        );
    }

    #[test]
    fn test_track_loop() {
        let expected = [2, 3, 7];
        let ts: TrackSet = expected.into_iter().map(TrackId).collect();
        assert_eq!(ts.0, expected.into_iter().map(|i| 1 << i).fold(0, |acc, i| acc | i), "Bit set implementation wonky");
        let res: Vec<_> = ts.into_iter().map(|i| i.0).collect();
        assert_eq!(expected.as_slice(), res, "IntoIter impl wonky");
    }

    // This broke once, so regression test!
    #[test]
    fn test_single_track_loop() {
        let expected = [0];
        let ts: TrackSet = expected.into_iter().map(TrackId).collect();
        assert_eq!(ts.0, expected.into_iter().map(|i| 1 << i).fold(0, |acc, i| acc | i), "Bit set implementation wonky");
        let res: Vec<_> = ts.into_iter().map(|i| i.0).collect();
        assert_eq!(expected.as_slice(), res, "IntoIter impl wonky");
    }
}
