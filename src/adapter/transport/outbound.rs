use std::collections::VecDeque;
use std::io::{self, Write};

const MAX_RESPONSE_BYTES: usize = 262_144;
const WRITE_BYTES_PER_POLL: usize = 65_536;
const WRITE_CALLS_PER_POLL: usize = 64;

#[derive(Debug)]
struct Frame {
    bytes: Vec<u8>,
    offset: usize,
    reliable: bool,
}

impl Frame {
    fn new(payload: &[u8], reliable: bool) -> Self {
        let mut bytes = Vec::with_capacity(payload.len() + 1);
        bytes.extend_from_slice(payload);
        bytes.push(b'\n');
        Self {
            bytes,
            offset: 0,
            reliable,
        }
    }
}

/// Priority applies only between frames. Started frames cannot be replaced or interrupted.
#[derive(Debug, Default)]
pub(super) struct Outbound {
    current: Option<Frame>,
    responses: VecDeque<Frame>,
    observation: Option<Frame>,
    response_bytes: usize,
}

impl Outbound {
    pub fn response(&mut self, payload: &[u8]) -> bool {
        let size = payload.len() + 1;
        if size > MAX_RESPONSE_BYTES.saturating_sub(self.response_bytes) {
            return false;
        }
        self.response_bytes += size;
        self.responses.push_back(Frame::new(payload, true));
        true
    }

    pub fn observation(&mut self, payload: &[u8]) {
        self.observation = Some(Frame::new(payload, false));
    }

    pub fn flush(&mut self, writer: &mut impl Write) -> io::Result<()> {
        let mut budget = WRITE_BYTES_PER_POLL;
        for _ in 0..WRITE_CALLS_PER_POLL {
            if budget == 0 {
                break;
            }
            if self.current.is_none() {
                self.current = self
                    .responses
                    .pop_front()
                    .or_else(|| self.observation.take());
            }
            let Some(frame) = self.current.as_mut() else {
                break;
            };
            let end = frame.bytes.len().min(frame.offset + budget);
            match writer.write(&frame.bytes[frame.offset..end]) {
                Ok(0) => return Err(io::ErrorKind::WriteZero.into()),
                Ok(n) => {
                    budget -= n;
                    frame.offset += n;
                    if frame.reliable {
                        self.response_bytes -= n;
                    }
                    if frame.offset == frame.bytes.len() {
                        self.current = None;
                    }
                }
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                    if frame.offset == 0 {
                        let frame = self.current.take().unwrap();
                        if frame.reliable {
                            self.responses.push_front(frame);
                        } else {
                            self.observation = Some(frame);
                        }
                    }
                    break;
                }
                Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(e) => return Err(e),
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct ShortWriter {
        bytes: Vec<u8>,
        remaining: usize,
    }
    impl Write for ShortWriter {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            if self.remaining == 0 {
                return Err(io::ErrorKind::WouldBlock.into());
            }
            let n = self.remaining.min(bytes.len());
            self.bytes.extend_from_slice(&bytes[..n]);
            self.remaining -= n;
            Ok(n)
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn ack_waits_for_started_observation_and_latest_pending_snapshot_wins() {
        let mut queue = Outbound::default();
        let mut writer = ShortWriter {
            remaining: 8,
            ..Default::default()
        };
        queue.observation(br#"{"type":"observation","seq":1}"#);
        queue.flush(&mut writer).unwrap();
        assert_eq!(writer.bytes.len(), 8);
        assert!(queue.response(br#"{"type":"ack","seq":2}"#));
        queue.observation(br#"{"type":"observation","seq":3}"#);
        queue.observation(br#"{"type":"observation","seq":4}"#);
        queue.flush(&mut writer).unwrap();
        writer.remaining = usize::MAX;
        queue.flush(&mut writer).unwrap();
        let frames: Vec<serde_json::Value> = writer
            .bytes
            .split(|b| *b == b'\n')
            .filter(|line| !line.is_empty())
            .map(|line| serde_json::from_slice(line).unwrap())
            .collect();
        assert_eq!(
            frames
                .iter()
                .map(|v| v["seq"].as_u64().unwrap())
                .collect::<Vec<_>>(),
            [1, 2, 4]
        );
        assert_eq!(queue.response_bytes, 0);
    }

    #[test]
    fn unsent_observation_is_replaceable_and_does_not_preempt_response() {
        let mut queue = Outbound::default();
        let mut writer = ShortWriter::default();
        queue.observation(b"old");
        queue.flush(&mut writer).unwrap();
        queue.observation(b"new");
        assert!(queue.response(b"ack"));
        writer.remaining = usize::MAX;
        queue.flush(&mut writer).unwrap();
        assert_eq!(writer.bytes, b"ack\nnew\n");
    }

    #[test]
    fn response_capacity_and_each_flush_are_bounded() {
        let mut queue = Outbound::default();
        assert!(queue.response(&vec![b'x'; MAX_RESPONSE_BYTES - 1]));
        assert!(!queue.response(b"overflow"));
        let mut writer = ShortWriter {
            remaining: usize::MAX,
            ..Default::default()
        };
        queue.flush(&mut writer).unwrap();
        assert_eq!(writer.bytes.len(), WRITE_BYTES_PER_POLL);
    }
}
