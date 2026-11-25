//! The log lines a running game wrote, kept for its log panel.

use std::collections::VecDeque;

use fastemu_core::{LogLevel, LogLine};

/// The last lines of one game's log, oldest first, with a count of the
/// errors and warnings nobody has looked at yet. It holds at most a number
/// of lines and a number of bytes of text, so a game that logs long lines
/// cannot fill the computer's memory with them.
#[derive(Debug)]
pub struct LogBuffer {
    lines: VecDeque<LogLine>,
    capacity: usize,
    /// The bytes of text the lines hold.
    bytes: usize,
    max_bytes: usize,
    unseen_problems: usize,
    dropped: u64,
}

impl LogBuffer {
    /// How many lines a game keeps; older ones make way.
    pub const DEFAULT_CAPACITY: usize = 2000;

    /// How many bytes of text a game's lines may take together; older
    /// lines make way. The log of a game cuts each line at 4096 bytes, so
    /// this holds at least 256 of them.
    pub const DEFAULT_MAX_BYTES: usize = 1 << 20;

    pub fn new(capacity: usize) -> Self {
        Self::with_max_bytes(capacity, Self::DEFAULT_MAX_BYTES)
    }

    /// A buffer of at most `capacity` lines and `max_bytes` bytes of text.
    pub fn with_max_bytes(capacity: usize, max_bytes: usize) -> Self {
        Self {
            lines: VecDeque::with_capacity(capacity.min(256)),
            capacity: capacity.max(1),
            bytes: 0,
            max_bytes,
            unseen_problems: 0,
            dropped: 0,
        }
    }

    /// Appends lines, dropping the oldest beyond the capacity and beyond
    /// the byte budget. A line longer than the whole budget is dropped on
    /// its own, and the lines before it stay.
    pub fn extend(&mut self, lines: impl IntoIterator<Item = LogLine>) {
        for line in lines {
            if matches!(line.level, LogLevel::Err | LogLevel::Warn) {
                self.unseen_problems += 1;
            }
            if line.text.len() > self.max_bytes {
                // It would push out every line before it and then itself.
                self.dropped += 1;
                continue;
            }
            self.bytes += line.text.len();
            self.lines.push_back(line);
            while self.lines.len() > self.capacity || self.bytes > self.max_bytes {
                let Some(oldest) = self.lines.pop_front() else {
                    break;
                };
                self.bytes -= oldest.text.len();
                self.dropped += 1;
            }
        }
    }

    pub fn len(&self) -> usize {
        self.lines.len()
    }

    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    pub fn get(&self, index: usize) -> Option<&LogLine> {
        self.lines.get(index)
    }

    /// How many lines fell off the front since the game started.
    pub fn dropped(&self) -> u64 {
        self.dropped
    }

    /// Errors and warnings written since the panel last showed this log.
    pub fn unseen_problems(&self) -> usize {
        self.unseen_problems
    }

    pub fn mark_seen(&mut self) {
        self.unseen_problems = 0;
    }

    pub fn clear(&mut self) {
        self.lines.clear();
        self.bytes = 0;
        self.unseen_problems = 0;
    }
}

impl Default for LogBuffer {
    fn default() -> Self {
        Self::new(Self::DEFAULT_CAPACITY)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(level: LogLevel, text: &str) -> LogLine {
        LogLine {
            level,
            text: text.into(),
        }
    }

    #[test]
    fn keeps_the_newest_lines() {
        let mut log = LogBuffer::new(2);
        log.extend([
            line(LogLevel::Out, "a"),
            line(LogLevel::Out, "b"),
            line(LogLevel::Out, "c"),
        ]);
        assert_eq!(log.len(), 2);
        assert_eq!(log.get(0).map(|l| l.text.as_str()), Some("b"));
        assert_eq!(log.dropped(), 1);
    }

    #[test]
    fn two_thousand_lines_of_4_kib_stay_under_the_byte_budget() {
        let mut log = LogBuffer::default();
        let text = "x".repeat(4096);
        for i in 0..2000 {
            log.extend([line(LogLevel::Out, &format!("{i:04}{text}"))]);
        }
        let bytes: usize = (0..log.len()).map(|i| log.get(i).unwrap().text.len()).sum();
        assert!(bytes <= LogBuffer::DEFAULT_MAX_BYTES, "{bytes} bytes");
        assert!(log.len() >= 200, "{} lines", log.len());
        assert_eq!(log.dropped() as usize, 2000 - log.len());
        // The newest are the ones kept.
        let last = log.get(log.len() - 1).unwrap();
        assert!(last.text.starts_with("1999"));
        assert!(log.get(0).unwrap().text.as_str() > "0000");
    }

    #[test]
    fn a_line_over_the_whole_budget_is_not_kept() {
        let mut log = LogBuffer::with_max_bytes(10, 100);
        log.extend([
            line(LogLevel::Out, "small"),
            line(LogLevel::Err, &"y".repeat(101)),
            line(LogLevel::Out, "after"),
        ]);
        // The long line is the only one that goes.
        assert_eq!(log.len(), 2);
        assert_eq!(log.get(0).map(|l| l.text.as_str()), Some("small"));
        assert_eq!(log.get(1).map(|l| l.text.as_str()), Some("after"));
        assert_eq!(log.dropped(), 1);
        // It still counts as a problem nobody has seen.
        assert_eq!(log.unseen_problems(), 1);
        log.clear();
        log.extend([line(LogLevel::Out, &"z".repeat(100))]);
        assert_eq!(log.len(), 1, "the budget starts again after a clear");
    }

    #[test]
    fn counts_unseen_problems() {
        let mut log = LogBuffer::default();
        log.extend([
            line(LogLevel::Err, "boom"),
            line(LogLevel::Info, "ok"),
            line(LogLevel::Warn, "missing api"),
        ]);
        assert_eq!(log.unseen_problems(), 2);
        log.mark_seen();
        assert_eq!(log.unseen_problems(), 0);
        log.clear();
        assert!(log.is_empty());
    }
}
