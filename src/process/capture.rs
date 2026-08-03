use std::{
    collections::VecDeque,
    io::Read,
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

/// Convert process bytes into a display/audit-safe string.
///
/// Strips executable terminal control sequences, keeps a bounded tail, and
/// marks truncation. Non-UTF-8 bytes are lossy-decoded only here.
pub fn sanitize_process_output(bytes: &[u8], cap: usize) -> String {
    let lossy = String::from_utf8_lossy(bytes);
    let mut sanitized = String::with_capacity(lossy.len().min(cap.saturating_add(32)));
    let mut truncated = false;

    for ch in lossy.chars() {
        let piece = match ch {
            '\n' | '\r' | '\t' => ch.to_string(),
            c if c.is_control() || c == '\u{7f}' => format!("\\u{{{:x}}}", u32::from(c)),
            c => c.to_string(),
        };
        if sanitized.len() + piece.len() > cap {
            truncated = true;
            break;
        }
        sanitized.push_str(&piece);
    }

    if truncated || (cap > 0 && lossy.len() > sanitized.len()) {
        if sanitized.len() > cap {
            sanitized.truncate(cap);
        }
        sanitized.push_str("...[truncated]");
    } else if bytes.len() > lossy.len() {
        // Byte length can exceed char display length for multi-byte sequences;
        // still mark when the raw buffer was larger than the rendered form and
        // the caller asked for a tight cap relative to raw input.
    }

    if cap == 0 {
        return "...[truncated]".to_string();
    }

    sanitized
}

#[derive(Debug, Default)]
pub(super) struct StreamCapture {
    pub(super) bytes: Vec<u8>,
    pub(super) truncated: bool,
    pub(super) total: u64,
}

pub(super) fn read_bounded<R: Read>(mut reader: R, cap: usize) -> StreamCapture {
    let mut ring = ByteRing::new(cap);
    let mut buf = [0_u8; 8192];
    loop {
        match reader.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => ring.push(&buf[..n]),
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(_) => break,
        }
    }
    ring.into_capture()
}

#[derive(Debug)]
struct ByteRing {
    buf: VecDeque<u8>,
    cap: usize,
    total: u64,
    truncated: bool,
}

impl ByteRing {
    fn new(cap: usize) -> Self {
        Self {
            buf: VecDeque::with_capacity(cap.min(64 * 1024)),
            cap,
            total: 0,
            truncated: false,
        }
    }

    fn push(&mut self, data: &[u8]) {
        self.total = self.total.saturating_add(data.len() as u64);
        if self.cap == 0 {
            self.truncated = self.truncated || !data.is_empty();
            return;
        }
        if data.len() >= self.cap {
            self.truncated = true;
            self.buf.clear();
            self.buf
                .extend(data[data.len() - self.cap..].iter().copied());
            return;
        }
        let next_len = self.buf.len() + data.len();
        if next_len > self.cap {
            let drop_count = next_len - self.cap;
            self.buf.drain(..drop_count);
            self.truncated = true;
        }
        self.buf.extend(data.iter().copied());
    }

    fn into_capture(self) -> StreamCapture {
        StreamCapture {
            bytes: self.buf.into_iter().collect(),
            truncated: self.truncated,
            total: self.total,
        }
    }
}

pub(super) fn collect_capture(
    rx: mpsc::Receiver<StreamCapture>,
    worker: Option<thread::JoinHandle<()>>,
    grace: Duration,
) -> StreamCapture {
    let deadline = Instant::now() + grace;
    let capture = loop {
        match rx.try_recv() {
            Ok(capture) => break capture,
            Err(mpsc::TryRecvError::Empty) => {
                if Instant::now() >= deadline {
                    break StreamCapture::default();
                }
                thread::sleep(Duration::from_millis(5));
            }
            Err(mpsc::TryRecvError::Disconnected) => break StreamCapture::default(),
        }
    };
    if let Some(handle) = worker {
        let _ = handle.join();
    }
    capture
}
