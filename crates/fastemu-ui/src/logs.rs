//! The log lines a running game wrote, kept for its log panel.

use std::collections::VecDeque;

use fastemu_core::{LogLevel, LogLine};

/// The last lines of one game's log, oldest first, with a count of the
/// errors and warnings nobody has looked at yet.
#[derive(Debug)]
pub struct LogBuffer {
    lines: VecDeque<LogLine>,
    capacity: usize,
    unseen_problems: usize,
    dropped: u64,
}

impl LogBuffer {
    /// How many lines a game keeps; older ones make way.
    pub const DEFAULT_CAPACITY: usize = 2000;

    pub fn new(capacity: usize) -> Self {
        Self {
            lines: VecDeque::with_capacity(capacity.min(256)),
            capacity: capacity.max(1),
            unseen_problems: 0,
            dropped: 0,
        }
    }

    /// Appends lines, dropping the oldest beyond the capacity.
    pub fn extend(&mut self, lines: impl IntoIterator<Item = LogLine>) {
        for line in lines {
            if matches!(line.level, LogLevel::Err | LogLevel::Warn) {
                self.unseen_problems += 1;
            }
            if self.lines.len() == self.capacity {
                self.lines.pop_front();
                self.dropped += 1;
            }
            self.lines.push_back(line);
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
