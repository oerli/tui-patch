use std::io;
use std::io::prelude::*;
use std::fs;
use std::fs::File;
use std::path::Path;


use chrono::{Utc, Datelike, Timelike};


pub enum LogSeverity {
    Error,
    Failed,
    Warning,
    Info,
    // Debug,
}

pub struct LogFile {
    file: Option<File>,
}

impl LogFile {
    pub fn new(directory_name: &str, file_name: &str) -> LogFile {
        let now = Utc::now();
        let file_name = format!("{}/{}_{:04}-{:02}-{:02}_{:02}-{:02}-{:02}.log", directory_name, file_name, now.year(), now.month(), now.day(), now.hour(), now.minute(), now.second());

        if Path::new(&directory_name).exists() == false {
            match fs::create_dir_all(&directory_name) {
                Ok(_) => (),
                Err(_) => {
                    return LogFile {file: None}
                }
            }
        }

        match File::create(&file_name) {
            Ok(f) => return LogFile {file: Some(f)},
            Err(_) => {
                return LogFile {file: None}
            }
        }
    }

    pub fn write(&mut self, severity: LogSeverity, buffer: &str) -> io::Result<()> {
        if buffer.is_empty() {
            return Ok(());
        }

        if self.file.is_some() {

            match severity {
                LogSeverity::Error => {
                    self.file.as_ref().unwrap().write(format!("\n[{}] 💣 ERROR: ", Utc::now()).as_bytes())?;
                    self.file.as_ref().unwrap().write_all(buffer.as_bytes())?;
                },
                LogSeverity::Failed => {
                    self.file.as_ref().unwrap().write(format!("\n[{}] 🚨 FAILED: ", Utc::now()).as_bytes())?;
                    self.file.as_ref().unwrap().write_all(buffer.as_bytes())?;
                },
                LogSeverity::Warning => {
                    self.file.as_ref().unwrap().write(format!("\n[{}] 🚧 WARNING: ", Utc::now()).as_bytes())?;
                    self.file.as_ref().unwrap().write_all(buffer.as_bytes())?;
                },
                LogSeverity::Info => {
                    self.file.as_ref().unwrap().write(format!("\n[{}] 🏁 INFO: ", Utc::now()).as_bytes())?;
                    self.file.as_ref().unwrap().write_all(buffer.as_bytes())?;
                },
                // LogSeverity::Debug => {
                //     self.file.as_ref().unwrap().write(format!("\n[{}] 🐜 DEBUG: ", Utc::now()).as_bytes())?;
                //     self.file.as_ref().unwrap().write_all(buffer.as_bytes())?;
                // },
            }
            return Ok(());
        } else {
            return Err(io::Error::new(io::ErrorKind::NotFound, "No Logfile loaded"));
        }
    }
}
