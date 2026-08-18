use serde::Serialize;
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct SessionId(pub String);

impl From<String> for SessionId {
    fn from(s: String) -> Self {
        SessionId(s)
    }
}

impl From<&str> for SessionId {
    fn from(s: &str) -> Self {
        SessionId(s.to_string())
    }
}

impl AsRef<str> for SessionId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for SessionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct OutputChunk {
    pub seq: u64,
    pub output: String,
    pub ts: u64,
}

impl OutputChunk {
    pub fn new(seq: u64, output: String) -> Self {
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        Self { seq, output, ts }
    }
}

// ============================================================================
// Output batching
// ============================================================================

/// Coalescing policy for terminal output before it crosses the IPC boundary.
///
/// Both the SSH I/O task and the local PTY reader accumulate raw output and
/// only emit a Tauri event once this policy says the batch is worth flushing.
/// Without batching, sustained output (`yes`, `find /`, large logs) would
/// produce one IPC event per OS read, drowning the WebView event loop in
/// serialization + dispatch work.
#[derive(Debug, Clone, Copy)]
pub struct OutputBatchPolicy {
    size_threshold: usize,
    time_threshold: Duration,
}

impl OutputBatchPolicy {
    pub const fn new(size_threshold: usize, time_threshold_ms: u64) -> Self {
        Self {
            size_threshold,
            time_threshold: Duration::from_millis(time_threshold_ms),
        }
    }

    /// Whether the accumulated output should be flushed and emitted now.
    ///
    /// * `pending_len` — bytes currently buffered (including any carried
    ///   incomplete UTF-8 tail).
    /// * `elapsed` — time since the last flush.
    /// * `urgent` — forces an immediate flush (e.g. right after user input so
    ///   the echo is not delayed by the batching window).
    ///
    /// Strict comparisons mirror the original SSH loop semantics (`>`), so
    /// refactoring the SSH path to use this policy is behavior-preserving.
    pub fn should_flush(&self, pending_len: usize, elapsed: Duration, urgent: bool) -> bool {
        pending_len > 0
            && (urgent
                || pending_len > self.size_threshold
                || elapsed > self.time_threshold)
    }
}

// ============================================================================
// Incremental UTF-8 boundary handling
// ============================================================================

/// Number of trailing bytes of `bytes` that form an incomplete UTF-8
/// sequence (0..=3). A multi-byte character can be split across two reads;
/// those trailing bytes must be preserved and prepended to the next chunk so
/// the character decodes intact instead of becoming `�` (U+FFFD).
pub fn utf8_incomplete_tail(bytes: &[u8]) -> usize {
    let len = bytes.len();
    if len == 0 {
        return 0;
    }

    // Scan back over continuation bytes (at most 3) to find the candidate
    // lead byte of the final sequence.
    let mut lead = len;
    let mut lookback = 0usize;
    while lookback < 3 && lead > 0 && bytes[lead - 1] & 0xC0 == 0x80 {
        lead -= 1;
        lookback += 1;
    }
    // Walked past the buffer start without finding a lead byte: the tail is
    // corrupt (only continuations) — nothing meaningful to carry.
    if lead == 0 {
        return 0;
    }

    let b = bytes[lead - 1];
    let seq_len = match b {
        0x00..=0x7F => 1,
        0xC0..=0xDF => 2,
        0xE0..=0xEF => 3,
        0xF0..=0xF7 => 4,
        // Continuation or invalid lead — nothing to carry.
        _ => 1,
    };
    let have = len - (lead - 1); // bytes from the lead byte to the end
    if have < seq_len {
        have
    } else {
        0
    }
}

/// Flush `pending` as a decoded UTF-8 string, preserving an incomplete
/// trailing sequence in `carry` so it is prepended to the next chunk.
///
/// Any leftover bytes carried from a previous flush are prepended first, then
/// the incomplete tail (if any) is split off and kept for the next flush. The
/// complete part is taken OUT of `pending` — `pending` must end up empty, or
/// every subsequent flush would re-emit the same accumulated content (the
/// SSH I/O loop would repeatedly re-send the welcome banner / MOTD and make
/// every echoed keystroke appear doubled).
pub fn flush_utf8(pending: &mut Vec<u8>, carry: &mut Vec<u8>) -> String {
    if !carry.is_empty() {
        let mut tail = std::mem::take(carry);
        tail.append(pending);
        *pending = tail;
    }
    let incomplete = utf8_incomplete_tail(pending);
    let split = pending.len() - incomplete;
    // `pending.split_off(split)` leaves the COMPLETE part in `pending` and
    // returns the incomplete tail — carry the tail and then take the complete
    // part out so `pending` is drained and never re-emitted.
    *carry = pending.split_off(split);
    let complete = std::mem::take(pending);
    String::from_utf8_lossy(&complete).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn batch_policy_flushes_when_size_threshold_exceeded() {
        let policy = OutputBatchPolicy::new(1024, 5);
        // Empty pending never flushes.
        assert!(!policy.should_flush(0, Duration::from_millis(100), false));
        // At the threshold (strictly greater, matching legacy semantics).
        assert!(!policy.should_flush(1024, Duration::from_millis(0), false));
        assert!(policy.should_flush(1025, Duration::from_millis(0), false));
    }

    #[test]
    fn batch_policy_flushes_on_time_threshold() {
        let policy = OutputBatchPolicy::new(1024, 5);
        assert!(!policy.should_flush(10, Duration::from_millis(5), false));
        assert!(policy.should_flush(10, Duration::from_millis(6), false));
    }

    #[test]
    fn batch_policy_urgent_flush_ignores_thresholds() {
        let policy = OutputBatchPolicy::new(1024, 5);
        // Urgent flushes even a single byte with zero elapsed time.
        assert!(policy.should_flush(1, Duration::from_millis(0), true));
    }

    #[test]
    fn incomplete_tail_detects_ascii() {
        assert_eq!(utf8_incomplete_tail(b"hello"), 0);
        assert_eq!(utf8_incomplete_tail(b""), 0);
    }

    #[test]
    fn incomplete_tail_detects_partial_multibyte() {
        // 0xE4 0xB8 is the first two bytes of 中 (U+4E2D, 3-byte).
        assert_eq!(utf8_incomplete_tail(&[0xE4, 0xB8]), 2);
        // Complete 中 (3 bytes) — nothing to carry.
        assert_eq!(utf8_incomplete_tail(&[0xE4, 0xB8, 0xAD]), 0);
        // One byte of a 3-byte char.
        assert_eq!(utf8_incomplete_tail(&[0xE4]), 1);
        // Partial 4-byte char: lead + two continuations.
        assert_eq!(utf8_incomplete_tail(&[0xF0, 0x9F, 0x98]), 3);
    }

    #[test]
    fn flush_utf8_joins_carried_bytes() {
        let mut pending: Vec<u8> = vec![0xE4, 0xB8]; // first 2 bytes of 中
        let mut carry: Vec<u8> = Vec::new();

        // Simulate the next read arriving with the final byte of 中 then "!"
        pending.extend_from_slice(&[0xAD, b'!']);
        let out = flush_utf8(&mut pending, &mut carry);
        assert_eq!(out, "中!");
        assert!(carry.is_empty());
        assert!(pending.is_empty());
    }

    #[test]
    fn flush_utf8_keeps_incomplete_tail_for_next_chunk() {
        let mut pending: Vec<u8> = b"ok \xE4\xB8".to_vec(); // 中 split after 2 bytes
        let mut carry: Vec<u8> = Vec::new();
        let out = flush_utf8(&mut pending, &mut carry);
        assert_eq!(out, "ok ");
        assert_eq!(carry, &[0xE4, 0xB8]);

        // Next flush prepends the carried bytes.
        let final_byte = b"\xAD";
        assert!(!final_byte.is_empty());
        let mut pending2: Vec<u8> = vec![0xAD];
        let out2 = flush_utf8(&mut pending2, &mut carry);
        assert_eq!(out2, "中");
        assert!(carry.is_empty());
    }

    #[test]
    fn flush_utf8_rejects_nothing_on_plain_ascii() {
        let mut pending: Vec<u8> = b"plain text".to_vec();
        let mut carry: Vec<u8> = Vec::new();
        assert_eq!(flush_utf8(&mut pending, &mut carry), "plain text");
        assert!(carry.is_empty());
    }

    #[test]
    fn flush_utf8_drains_pending_to_prevent_reemission() {
        // Regression for the SSH banner-repeat / doubled-echo bug: the flush
        // must REMOVE the complete bytes from `pending`, otherwise the SSH I/O
        // loop (20ms read timeout, fast idle spin) re-emits the same
        // accumulated content on every iteration — the welcome banner kept
        // scrolling and every echoed keystroke appeared twice.
        let mut pending: Vec<u8> = b"Welcome banner content".to_vec();
        let mut carry: Vec<u8> = Vec::new();

        let first = flush_utf8(&mut pending, &mut carry);
        assert_eq!(first, "Welcome banner content");
        assert!(pending.is_empty(), "pending must be drained after a flush");
        assert!(carry.is_empty());

        // Repeated flushes with no new data must not re-emit anything.
        assert!(flush_utf8(&mut pending, &mut carry).is_empty());
        assert!(pending.is_empty());
    }
}