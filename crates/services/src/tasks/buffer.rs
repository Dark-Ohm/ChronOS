//! Ring buffer for task log lines.

use std::collections::VecDeque;

/// Default max lines kept (~`cargo build` cold can emit tens of thousands).
pub const DEFAULT_LOG_CAP: usize = 8_000;

/// Which pipe a line came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamKind {
    Stdout,
    Stderr,
    /// Synthetic system line (spawn failed, cancelled, truncated notice).
    System,
}

/// One log line with stream origin.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogLine {
    pub stream: StreamKind,
    pub text: String,
}

/// Bounded FIFO of log lines; oldest drop when over cap.
#[derive(Debug, Clone)]
pub struct LogBuffer {
    lines: VecDeque<LogLine>,
    cap: usize,
    /// How many lines were dropped from the front.
    dropped: usize,
}

impl LogBuffer {
    pub fn new(cap: usize) -> Self {
        Self {
            lines: VecDeque::new(),
            cap: cap.max(1),
            dropped: 0,
        }
    }

    pub fn push(&mut self, line: LogLine) {
        while self.lines.len() >= self.cap {
            self.lines.pop_front();
            self.dropped = self.dropped.saturating_add(1);
        }
        self.lines.push_back(line);
    }

    pub fn push_str(&mut self, stream: StreamKind, text: impl Into<String>) {
        self.push(LogLine {
            stream,
            text: text.into(),
        });
    }

    pub fn clear(&mut self) {
        self.lines.clear();
        self.dropped = 0;
    }

    pub fn lines(&self) -> &VecDeque<LogLine> {
        &self.lines
    }

    pub fn len(&self) -> usize {
        self.lines.len()
    }

    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    pub fn dropped(&self) -> usize {
        self.dropped
    }

    pub fn is_truncated(&self) -> bool {
        self.dropped > 0
    }

    pub fn cap(&self) -> usize {
        self.cap
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn respects_cap_and_marks_dropped() {
        let mut buf = LogBuffer::new(3);
        for i in 0..5 {
            buf.push_str(StreamKind::Stdout, format!("L{i}"));
        }
        assert_eq!(buf.len(), 3);
        assert_eq!(buf.dropped(), 2);
        assert!(buf.is_truncated());
        let texts: Vec<_> = buf.lines().iter().map(|l| l.text.as_str()).collect();
        assert_eq!(texts, ["L2", "L3", "L4"]);
    }

    #[test]
    fn clear_resets_dropped() {
        let mut buf = LogBuffer::new(2);
        buf.push_str(StreamKind::Stderr, "a");
        buf.push_str(StreamKind::Stderr, "b");
        buf.push_str(StreamKind::Stderr, "c");
        assert!(buf.is_truncated());
        buf.clear();
        assert_eq!(buf.dropped(), 0);
        assert!(buf.is_empty());
    }

    #[test]
    fn stream_kinds_preserved() {
        let mut buf = LogBuffer::new(10);
        buf.push_str(StreamKind::Stdout, "out");
        buf.push_str(StreamKind::Stderr, "err");
        assert_eq!(buf.lines()[0].stream, StreamKind::Stdout);
        assert_eq!(buf.lines()[1].stream, StreamKind::Stderr);
    }
}
