use std::{fs::OpenOptions, io::Write, sync::Mutex};

use crate::trace::trace::TraceEvent;

pub struct TraceLogger {
    file: Mutex<std::fs::File>,
}

impl TraceLogger {
    pub fn new(path: &str) -> Self {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .expect("failed to open trace file");

        Self {
            file: Mutex::new(file),
        }
    }

    pub fn log(&self, event: &TraceEvent) {
        let json = serde_json::to_string(event).unwrap();
        let mut file = self.file.lock().unwrap();
        writeln!(file, "{}", json).unwrap();
    }
}
