//! The bounded line splitter (SDD 3.10.1): bytes in, lines out, memory bounded by the cap.
//!
//! Contract: the caller drains `next_line` until it returns `None` before the next `push`.
//! Then `buffered()` never exceeds the cap plus one pushed chunk, and every `Ok` line is at
//! most `cap` bytes before lossy UTF-8 conversion.

/// A line (complete or still growing) is longer than the cap.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LineTooLong;

#[derive(Debug)]
pub struct LineSplitter {
    buf: Vec<u8>,
    /// Start of the unconsumed bytes in `buf`. Lines are consumed by moving this index, so
    /// one `next_line` call never moves the rest of the buffer (EF-01: bare-newline floods).
    start: usize,
    /// Everything in `buf[start..scanned]` is known to hold no `\n`: a partial line is scanned
    /// once, not once per pushed chunk.
    scanned: usize,
    cap: usize,
    failed: bool,
}

impl LineSplitter {
    pub fn new(cap: usize) -> LineSplitter {
        LineSplitter {
            buf: Vec::new(),
            start: 0,
            scanned: 0,
            cap,
            failed: false,
        }
    }

    /// Appends bytes. Once a line has been found over the cap the splitter stays failed and
    /// drops everything it is given. The consumed prefix is reclaimed here, once per push.
    pub fn push(&mut self, bytes: &[u8]) {
        if self.failed {
            return;
        }
        if self.start > 0 {
            self.buf.drain(..self.start);
            self.scanned = self.scanned.saturating_sub(self.start);
            self.start = 0;
        }
        self.buf.extend_from_slice(bytes);
    }

    /// The next complete line without its `\n` (and a trailing `\r`), as lossy UTF-8.
    /// `Some(Err)` once an over-long line was seen, and on every later call. `None`: more
    /// bytes are needed (and the partial line held is within the cap).
    pub fn next_line(&mut self) -> Option<Result<String, LineTooLong>> {
        if self.failed {
            return Some(Err(LineTooLong));
        }
        let from = self.scanned.max(self.start);
        let found = self
            .buf
            .get(from..)
            .and_then(|tail| tail.iter().position(|b| *b == b'\n'))
            .map(|off| from + off);
        let Some(pos) = found else {
            self.scanned = self.buf.len();
            if self.buf.len() - self.start > self.cap {
                self.fail();
                return Some(Err(LineTooLong));
            }
            return None;
        };
        if pos - self.start > self.cap {
            self.fail();
            return Some(Err(LineTooLong));
        }
        let mut end = pos;
        if end > self.start && self.buf.get(end - 1) == Some(&b'\r') {
            end -= 1;
        }
        let text = self
            .buf
            .get(self.start..end)
            .map(|b| String::from_utf8_lossy(b).into_owned())
            .unwrap_or_default();
        self.start = pos + 1;
        self.scanned = self.start;
        Some(Ok(text))
    }

    fn fail(&mut self) {
        self.failed = true;
        self.buf.clear();
        self.start = 0;
        self.scanned = 0;
    }

    /// Bytes currently held.
    pub fn buffered(&self) -> usize {
        self.buf.len() - self.start
    }
}
