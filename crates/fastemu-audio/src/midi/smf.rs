//! Standard MIDI file parsing.
//!
//! Format 0 and 1 files (and format 2, played as if it were 1), bare or
//! in a RIFF RMID wrapper. Tracks are merged into one list of channel
//! messages timed in microseconds through the tempo map. Running status,
//! tempo and time signature changes are honoured; system exclusive
//! messages are skipped. Like WAV, phone MIDI files often carry wrong
//! chunk lengths, so the parser reads what is there.

use crate::error::AudioError;
use crate::format::Format;

/// The tempo a file plays at until it sets one: 120 beats per minute.
const DEFAULT_TEMPO_US: u32 = 500_000;
/// A file with more events than this is refused rather than held in
/// memory; game music has a few thousand. The cap also bounds the work of
/// a seek, which sends the controllers before the new position again, and
/// of each pass of a loop, which sends every event.
const MAX_EVENTS: usize = 1 << 16;

/// A channel message at a time.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Event {
    pub time_us: u64,
    pub status: u8,
    pub data1: u8,
    pub data2: u8,
}

/// A time signature change, kept for controls that report it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct TimeSignature {
    pub time_us: u64,
    pub numerator: u8,
    /// The denominator is 2 to this power: 2 for quarter notes.
    pub denominator_power: u8,
}

/// A parsed MIDI file.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Sequence {
    /// Channel messages, in the order they play.
    pub events: Vec<Event>,
    /// From the start to the end of the longest track, trailing rests
    /// included, so loops keep their rhythm.
    pub duration_us: u64,
    pub time_signatures: Vec<TimeSignature>,
    /// How many tempo changes the file has.
    pub tempo_changes: usize,
}

fn malformed(reason: impl Into<String>) -> AudioError {
    AudioError::Malformed {
        format: Format::Midi,
        reason: reason.into(),
    }
}

/// A message before tempo is applied: its tick, and its place in the
/// file to break ties.
#[derive(Clone, Copy)]
struct Raw {
    tick: u64,
    kind: RawKind,
}

#[derive(Clone, Copy)]
enum RawKind {
    Channel([u8; 3]),
    Tempo(u32),
    TimeSignature(u8, u8),
}

/// Parses a whole MIDI file.
pub(crate) fn parse(data: &[u8]) -> Result<Sequence, AudioError> {
    let data = unwrap_rmid(data);
    let start = find(data, *b"MThd").ok_or_else(|| malformed("no MThd header"))?;
    let header_len = read_u32(data, start + 4).ok_or_else(|| malformed("short header"))? as usize;
    let header = data
        .get(start + 8..start + 14)
        .ok_or_else(|| malformed("short header"))?;
    let division = u16::from_be_bytes([header[4], header[5]]);
    let mut raw = Vec::new();
    let mut end_tick = 0u64;
    let mut tracks = 0;
    let mut at = start + 8 + header_len.max(6);
    while at + 8 <= data.len() {
        if &data[at..at + 4] != b"MTrk" {
            if is_chunk_id(&data[at..at + 4]) {
                // Some other chunk: skip it whole.
                let len = read_u32(data, at + 4).unwrap_or(0) as usize;
                at = at.saturating_add(8).saturating_add(len);
                continue;
            }
            // Lost: the last track's length was wrong. Look for the next.
            match find(&data[at..], *b"MTrk") {
                Some(offset) => at += offset,
                None => break,
            }
            continue;
        }
        let len = read_u32(data, at + 4).unwrap_or(0) as usize;
        let body_start = at + 8;
        let declared_end = body_start.saturating_add(len).min(data.len());
        // Trust the declared length when another chunk starts there.
        // Otherwise it is wrong: read the track up to its end-of-track
        // event and carry on from there.
        let trusted = declared_end == data.len()
            || data
                .get(declared_end..declared_end + 4)
                .is_some_and(is_chunk_id);
        let body = if trusted {
            &data[body_start..declared_end]
        } else {
            &data[body_start..]
        };
        let track = parse_track(body, &mut raw)?;
        end_tick = end_tick.max(track.end_tick);
        tracks += 1;
        at = if trusted {
            declared_end
        } else {
            body_start + track.consumed
        };
    }
    if tracks == 0 {
        return Err(malformed("no MTrk chunks"));
    }
    // Merge the tracks: a stable sort keeps each track's own order and
    // puts earlier tracks first at equal ticks.
    raw.sort_by_key(|r| r.tick);
    Ok(apply_tempo(&raw, division, end_tick))
}

fn unwrap_rmid(data: &[u8]) -> &[u8] {
    if data.len() >= 12
        && &data[..4] == b"RIFF"
        && &data[8..12] == b"RMID"
        && let Some(at) = find(&data[12..], *b"data")
    {
        return data.get(12 + at + 8..).unwrap_or_default();
    }
    data
}

fn find(data: &[u8], needle: [u8; 4]) -> Option<usize> {
    data.windows(4).position(|w| w == needle)
}

fn is_chunk_id(id: &[u8]) -> bool {
    id.len() == 4 && id.iter().all(u8::is_ascii_alphanumeric)
}

fn read_u32(data: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_be_bytes(data.get(at..at + 4)?.try_into().ok()?))
}

struct TrackEnd {
    end_tick: u64,
    /// Bytes read up to and including the end-of-track event.
    consumed: usize,
}

struct Reader<'a> {
    data: &'a [u8],
    at: usize,
}

impl Reader<'_> {
    fn byte(&mut self) -> Option<u8> {
        let byte = *self.data.get(self.at)?;
        self.at += 1;
        Some(byte)
    }

    /// A variable-length quantity: up to four bytes of seven bits.
    fn vlq(&mut self) -> Option<u64> {
        let mut value = 0u64;
        for _ in 0..4 {
            let byte = self.byte()?;
            value = (value << 7) | u64::from(byte & 0x7F);
            if byte & 0x80 == 0 {
                return Some(value);
            }
        }
        Some(value)
    }

    fn skip(&mut self, count: u64) -> Option<&[u8]> {
        let count = usize::try_from(count).ok()?;
        let start = self.at;
        let end = start.checked_add(count)?;
        let bytes = self.data.get(start..end)?;
        self.at = end;
        Some(bytes)
    }
}

fn parse_track(data: &[u8], raw: &mut Vec<Raw>) -> Result<TrackEnd, AudioError> {
    let mut reader = Reader { data, at: 0 };
    let mut tick = 0u64;
    let mut running = None;
    while let Some(delta) = reader.vlq() {
        tick = tick.saturating_add(delta);
        let Some(first) = reader.byte() else { break };
        let kind = match first {
            0xFF => {
                let Some(kind) = reader.byte() else { break };
                let Some(len) = reader.vlq() else { break };
                let Some(body) = reader.skip(len) else { break };
                match (kind, body) {
                    (0x2F, _) => {
                        return Ok(TrackEnd {
                            end_tick: tick,
                            consumed: reader.at,
                        });
                    }
                    (0x51, &[a, b, c, ..]) => {
                        let tempo = u32::from_be_bytes([0, a, b, c]);
                        // A tempo of zero would make time stand still.
                        Some(RawKind::Tempo(tempo.max(1)))
                    }
                    (0x58, &[numerator, power, ..]) => {
                        Some(RawKind::TimeSignature(numerator, power))
                    }
                    _ => None,
                }
            }
            0xF0 | 0xF7 => {
                let Some(len) = reader.vlq() else { break };
                if reader.skip(len).is_none() {
                    break;
                }
                None
            }
            // System common and real-time bytes do not belong in files;
            // skip them with their data.
            0xF1 | 0xF3 => {
                reader.byte();
                None
            }
            0xF2 => {
                reader.skip(2);
                None
            }
            0xF4..=0xFE => None,
            status @ 0x80..=0xEF => {
                running = Some(status);
                let Some(data1) = reader.byte() else { break };
                channel_message(status, data1, &mut reader)
            }
            data1 => match running {
                Some(status) => channel_message(status, data1, &mut reader),
                // A data byte with no status to run: skip it.
                None => None,
            },
        };
        if let Some(kind) = kind {
            if raw.len() >= MAX_EVENTS {
                return Err(AudioError::TooLarge(format!(
                    "a MIDI file of more than {MAX_EVENTS} events"
                )));
            }
            raw.push(Raw { tick, kind });
        }
    }
    // No end-of-track event: the track ends at its last event.
    Ok(TrackEnd {
        end_tick: tick,
        consumed: data.len(),
    })
}

fn channel_message(status: u8, data1: u8, reader: &mut Reader<'_>) -> Option<RawKind> {
    let data2 = match status & 0xF0 {
        0xC0 | 0xD0 => 0,
        _ => reader.byte()?,
    };
    Some(RawKind::Channel([status, data1 & 0x7F, data2 & 0x7F]))
}

/// Times every message through the tempo map.
fn apply_tempo(raw: &[Raw], division: u16, end_tick: u64) -> Sequence {
    let mut timer = Timer::new(division);
    let mut sequence = Sequence::default();
    for r in raw {
        let time_us = timer.time_us(r.tick);
        match r.kind {
            RawKind::Channel([status, data1, data2]) => sequence.events.push(Event {
                time_us,
                status,
                data1,
                data2,
            }),
            RawKind::Tempo(tempo) => {
                timer.set_tempo(r.tick, tempo);
                sequence.tempo_changes += 1;
            }
            RawKind::TimeSignature(numerator, denominator_power) => {
                sequence.time_signatures.push(TimeSignature {
                    time_us,
                    numerator,
                    denominator_power,
                });
            }
        }
    }
    let last_tick = raw.last().map_or(0, |r| r.tick).max(end_tick);
    sequence.duration_us = timer.time_us(last_tick);
    sequence
}

/// Turns ticks into microseconds, one tempo segment at a time. Ticks
/// must be asked for in order.
struct Timer {
    /// Ticks per quarter note, or `None` for SMPTE timing.
    ticks_per_quarter: Option<f64>,
    us_per_tick: f64,
    segment_tick: u64,
    segment_us: f64,
}

impl Timer {
    fn new(division: u16) -> Self {
        if division & 0x8000 != 0 {
            // SMPTE: frames per second (negative) and ticks per frame;
            // tempo changes do not apply.
            let fps = match -i32::from((division >> 8) as i8) {
                29 => 29.97,
                fps if fps > 0 => f64::from(fps),
                _ => 25.0,
            };
            let ticks_per_frame = f64::from((division & 0xFF).max(1));
            return Self {
                ticks_per_quarter: None,
                us_per_tick: 1_000_000.0 / (fps * ticks_per_frame),
                segment_tick: 0,
                segment_us: 0.0,
            };
        }
        // A division of zero is broken; 96 is a common real value.
        let ticks_per_quarter = f64::from(if division == 0 { 96 } else { division });
        Self {
            ticks_per_quarter: Some(ticks_per_quarter),
            us_per_tick: f64::from(DEFAULT_TEMPO_US) / ticks_per_quarter,
            segment_tick: 0,
            segment_us: 0.0,
        }
    }

    fn time_us(&self, tick: u64) -> u64 {
        let ticks = tick.saturating_sub(self.segment_tick) as f64;
        (self.segment_us + ticks * self.us_per_tick).round() as u64
    }

    /// Plays at `tempo_us` per quarter note from `tick` on.
    fn set_tempo(&mut self, tick: u64, tempo_us: u32) {
        let Some(ticks_per_quarter) = self.ticks_per_quarter else {
            return;
        };
        self.segment_us += tick.saturating_sub(self.segment_tick) as f64 * self.us_per_tick;
        self.segment_tick = tick;
        self.us_per_tick = f64::from(tempo_us) / ticks_per_quarter;
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// Builds a MIDI file from tracks of raw event bytes (delta times
    /// included).
    pub(crate) fn smf(format: u16, division: u16, tracks: &[&[u8]]) -> Vec<u8> {
        let mut out = b"MThd\0\0\0\x06".to_vec();
        out.extend_from_slice(&format.to_be_bytes());
        out.extend_from_slice(&(tracks.len() as u16).to_be_bytes());
        out.extend_from_slice(&division.to_be_bytes());
        for track in tracks {
            out.extend_from_slice(b"MTrk");
            out.extend_from_slice(&(track.len() as u32).to_be_bytes());
            out.extend_from_slice(track);
        }
        out
    }

    const END: [u8; 4] = [0x00, 0xFF, 0x2F, 0x00];

    fn track(events: &[u8]) -> Vec<u8> {
        let mut t = events.to_vec();
        t.extend_from_slice(&END);
        t
    }

    #[test]
    fn reads_a_format_0_file_with_running_status() {
        // 96 ticks per quarter at 120 bpm: a quarter is 500 ms.
        let t = track(&[
            0x00, 0x90, 60, 100, // note on
            0x60, 60, 0, // running status: note on, velocity 0
            0x00, 64, 90, // running status again
            0x60, 0x80, 64, 0,
        ]);
        let seq = parse(&smf(0, 96, &[&t])).unwrap();
        let times: Vec<u64> = seq.events.iter().map(|e| e.time_us).collect();
        assert_eq!(times, vec![0, 500_000, 500_000, 1_000_000]);
        assert_eq!(seq.events[1].status, 0x90);
        assert_eq!(seq.events[1].data2, 0);
        assert_eq!(seq.events[2].data1, 64);
        assert_eq!(seq.duration_us, 1_000_000);
    }

    #[test]
    fn merges_format_1_tracks_and_follows_tempo_changes() {
        // The conductor track: 120 bpm, then 60 bpm from tick 96, and a
        // 3/4 time signature.
        let conductor = track(&[
            0x00, 0xFF, 0x58, 0x04, 3, 2, 24, 8, //
            0x60, 0xFF, 0x51, 0x03, 0x0F, 0x42, 0x40,
        ]);
        let notes = track(&[
            0x00, 0x91, 60, 100, //
            0x81, 0x40, 0x81, 60, 0, // delta 192 ticks
        ]);
        let seq = parse(&smf(1, 96, &[&conductor, &notes])).unwrap();
        assert_eq!(seq.events.len(), 2);
        // 96 ticks at 500 ms per quarter, then 96 at 1 s per quarter.
        assert_eq!(seq.events[1].time_us, 1_500_000);
        assert_eq!(seq.tempo_changes, 1);
        assert_eq!(seq.time_signatures.len(), 1);
        assert_eq!(seq.time_signatures[0].numerator, 3);
        assert_eq!(seq.time_signatures[0].denominator_power, 2);
    }

    #[test]
    fn skips_sysex_and_keeps_trailing_rests() {
        let t = [
            0x00, 0xF0, 0x03, 0x7E, 0x09, 0xF7, // GM on
            0x00, 0xC0, 5, // program change: one data byte
            0x00, 0x90, 60, 100, //
            0x60, 0x80, 60, 0, //
            // The track ends two quarters after the last note.
            0x81, 0x40, 0xFF, 0x2F, 0x00,
        ];
        let seq = parse(&smf(0, 96, &[&t])).unwrap();
        assert_eq!(seq.events.len(), 3);
        assert_eq!(seq.events[0].status, 0xC0);
        assert_eq!(seq.duration_us, 1_500_000);
    }

    #[test]
    fn tolerates_wrong_lengths_and_wrappers() {
        let t = track(&[0x00, 0x90, 60, 100, 0x60, 0x80, 60, 0]);
        // A track length far past the end of the file.
        let mut file = smf(0, 96, &[&t]);
        file[18..22].copy_from_slice(&0x00FF_FFFFu32.to_be_bytes());
        assert_eq!(parse(&file).unwrap().events.len(), 2);
        // A track length too short: the parser reads on to the end of
        // track and finds the next track.
        let mut file = smf(1, 96, &[&t, &t]);
        file[18..22].copy_from_slice(&4u32.to_be_bytes());
        assert_eq!(parse(&file).unwrap().events.len(), 4);
        // A RIFF RMID wrapper.
        let inner = smf(0, 96, &[&t]);
        let mut rmid = b"RIFF\0\0\0\0RMIDdata".to_vec();
        rmid.extend_from_slice(&(inner.len() as u32).to_le_bytes());
        rmid.extend_from_slice(&inner);
        assert_eq!(parse(&rmid).unwrap().events.len(), 2);
        // A missing end of track.
        let bare = [0x00, 0x90, 60, 100, 0x60, 0x80, 60, 0];
        assert_eq!(parse(&smf(0, 96, &[&bare])).unwrap().duration_us, 500_000);
    }

    #[test]
    fn smpte_timing_ignores_tempo() {
        // 25 frames per second, 40 ticks per frame: 1 ms per tick.
        let division = (((-25i8) as u8 as u16) << 8) | 0x28;
        let t = track(&[
            0x00, 0xFF, 0x51, 0x03, 0x0F, 0x42, 0x40, //
            0x00, 0x90, 60, 100, //
            0x87, 0x68, 0x80, 60, 0, // 1000 ticks
        ]);
        let seq = parse(&smf(0, division, &[&t])).unwrap();
        assert_eq!(seq.events[1].time_us, 1_000_000);
    }

    /// A format 0 file of `notes` events: note ons at tick 0 on channel 1.
    fn of_events(notes: usize) -> Vec<u8> {
        let mut t = Vec::with_capacity(notes * 4 + 4);
        for i in 0..notes {
            t.extend_from_slice(&[0x00, 0x90, (i % 128) as u8, 100]);
        }
        t.extend_from_slice(&END);
        smf(0, 96, &[&t])
    }

    #[test]
    fn a_file_may_hold_65536_events_and_no_more() {
        let most = parse(&of_events(MAX_EVENTS)).unwrap();
        assert_eq!(most.events.len(), 65_536);
        let refused = parse(&of_events(MAX_EVENTS + 1)).unwrap_err();
        assert_eq!(
            refused,
            AudioError::TooLarge("a MIDI file of more than 65536 events".into())
        );
        // A file of 200,000 events is refused as soon as the cap is passed,
        // without reading the rest.
        assert!(matches!(
            parse(&of_events(200_000)),
            Err(AudioError::TooLarge(_))
        ));
    }

    #[test]
    fn tempo_changes_count_as_events() {
        // Nothing but tempo changes: each is an event of the file too.
        let mut t = Vec::new();
        for _ in 0..=MAX_EVENTS {
            t.extend_from_slice(&[0x00, 0xFF, 0x51, 0x03, 0x07, 0xA1, 0x20]);
        }
        t.extend_from_slice(&END);
        assert!(matches!(
            parse(&smf(0, 96, &[&t])),
            Err(AudioError::TooLarge(_))
        ));
    }

    #[test]
    fn refuses_files_without_tracks_or_header() {
        assert!(matches!(parse(b"hello"), Err(AudioError::Malformed { .. })));
        assert!(matches!(
            parse(&smf(0, 96, &[])),
            Err(AudioError::Malformed { .. })
        ));
    }
}
