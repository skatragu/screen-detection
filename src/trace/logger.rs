use std::{fs::OpenOptions, io::Write, sync::Mutex};

use crate::trace::trace::TraceEvent;

pub struct TraceLogger {
    file: Option<Mutex<std::fs::File>>,
}

impl TraceLogger {
    pub fn new(path: &str) -> Self {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path);

        match file {
            Ok(f) => Self {
                file: Some(Mutex::new(f)),
            },
            Err(e) => {
                eprintln!("Warning: could not open trace file '{}': {}", path, e);
                Self { file: None }
            }
        }
    }

    pub fn log(&self, event: &TraceEvent) {
        let file_mutex = match &self.file {
            Some(f) => f,
            None => return, // tracing disabled
        };

        let json = match serde_json::to_string(event) {
            Ok(j) => j,
            Err(e) => {
                eprintln!("Warning: failed to serialize trace event: {}", e);
                return;
            }
        };

        let mut file = match file_mutex.lock() {
            Ok(f) => f,
            Err(e) => {
                eprintln!("Warning: trace logger lock poisoned: {}", e);
                return;
            }
        };

        if let Err(e) = writeln!(file, "{}", json) {
            eprintln!("Warning: failed to write trace event: {}", e);
        }
    }
}
