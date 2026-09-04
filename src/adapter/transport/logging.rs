use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::path::PathBuf;
use std::sync::mpsc::{SyncSender, sync_channel};

const QUEUE_CAPACITY: usize = 32;
const FILE_LIMIT: u64 = 8 * 1024 * 1024;

struct Record {
    ts: u64,
    direction: &'static str,
    connection_id: usize,
    line: Vec<u8>,
}

/// Diagnostic only: queue overflow drops records, never protocol responses.
pub(super) struct WireLog(SyncSender<Record>);

impl WireLog {
    pub fn open(path: Option<&str>) -> Option<Self> {
        let path = path?;
        let path = if path == "auto" {
            PathBuf::from(format!(
                "/tmp/tetris-ai-adapter-{}.jsonl",
                super::now_unix_ms()
            ))
        } else {
            PathBuf::from(path)
        };
        let (sender, receiver) = sync_channel::<Record>(QUEUE_CAPACITY);
        std::thread::Builder::new()
            .name("adapter-log".into())
            .spawn(move || {
                let Ok(mut log) = RollingLog::open(path, FILE_LIMIT) else {
                    return;
                };
                for record in receiver {
                    let value = serde_json::json!({
                        "ts_ms": record.ts,
                        "direction": record.direction,
                        "connection_id": record.connection_id,
                        "line": String::from_utf8_lossy(&record.line),
                    });
                    if let Ok(mut bytes) = serde_json::to_vec(&value) {
                        bytes.push(b'\n');
                        if log.write(&bytes).is_err() {
                            break;
                        }
                    }
                }
            })
            .ok()?;
        Some(Self(sender))
    }

    pub fn record(&self, direction: &'static str, connection_id: usize, line: &[u8]) {
        let _ = self.0.try_send(Record {
            ts: super::now_unix_ms(),
            direction,
            connection_id,
            line: line.to_vec(),
        });
    }
}

struct RollingLog {
    path: PathBuf,
    file: File,
    size: u64,
    limit: u64,
}
impl RollingLog {
    fn open(path: PathBuf, limit: u64) -> io::Result<Self> {
        let file = OpenOptions::new().create(true).append(true).open(&path)?;
        let size = file.metadata()?.len();
        Ok(Self {
            path,
            file,
            size,
            limit,
        })
    }
    fn write(&mut self, bytes: &[u8]) -> io::Result<()> {
        if bytes.len() as u64 > self.limit {
            return Ok(());
        }
        if self.size.saturating_add(bytes.len() as u64) > self.limit {
            let mut backup = self.path.as_os_str().to_os_string();
            backup.push(".previous");
            std::fs::rename(&self.path, PathBuf::from(backup))?;
            self.file = OpenOptions::new()
                .create(true)
                .truncate(true)
                .write(true)
                .open(&self.path)?;
            self.size = 0;
        }
        self.file.write_all(bytes)?;
        self.size += bytes.len() as u64;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rotation_keeps_only_current_and_previous_bounded_files() {
        let path = std::env::temp_dir().join(format!("gpui-log-test-{}.jsonl", std::process::id()));
        let mut log = RollingLog::open(path.clone(), 8).unwrap();
        for _ in 0..5 {
            log.write(b"1234\n").unwrap();
        }
        let backup = PathBuf::from(format!("{}.previous", path.display()));
        assert_eq!(std::fs::read(&path).unwrap(), b"1234\n");
        assert_eq!(std::fs::read(&backup).unwrap(), b"1234\n");
        drop(log);
        std::fs::remove_file(path).unwrap();
        std::fs::remove_file(backup).unwrap();
    }
}
