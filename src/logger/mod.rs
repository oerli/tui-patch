use std::io;
use std::error::Error;

use std::io::prelude::*;
use std::fs;
use std::fs::File;
use std::path::Path;

use chrono::{Utc, Datelike, Timelike};

use log::{Record, Level, LevelFilter, Metadata};

use std::cell::RefCell;

thread_local! {
    static LOGGER: RefCell<Option<File>> = RefCell::new(None);
}

pub fn init(directory_name: &str, file_prefix: &str) -> Result<(), Box<dyn Error>> {
    let now = Utc::now();
    let file_name = format!("{}/{}_{:04}-{:02}-{:02}_{:02}-{:02}-{:02}.log", directory_name, file_prefix, now.year(), now.month(), now.day(), now.hour(), now.minute(), now.second());

    if Path::new(&directory_name).exists() == false {
        match fs::create_dir_all(&directory_name) {
            Ok(_) => (),
            Err(_) => return Err(Box::new(io::Error::new(io::ErrorKind::PermissionDenied, "could not create log directory")))?,
        }
    }

    LOGGER.with(|rc| {
            rc.replace(File::create(&file_name).ok());
    });

    let logger = FileLogger::new();

    // set log level for the threads
    let _ = log::set_boxed_logger(Box::new(logger)).map(|()| log::set_max_level(LevelFilter::max()));

    Ok(())
}

pub struct FileLogger {
}

impl FileLogger {
    pub fn new() -> Self {
        FileLogger{}
    }
}

impl log::Log for FileLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Info
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            LOGGER.with(|rc| {
                match record.level() {
                    Level::Error => rc.borrow_mut().as_mut().unwrap().write(format!("\n[{}] 💣 {}: {}", Utc::now(), record.level(), record.args()).as_bytes()).unwrap(),
                    Level::Warn => rc.borrow_mut().as_mut().unwrap().write(format!("\n[{}] 🚧 {}: {}", Utc::now(), record.level(), record.args()).as_bytes()).unwrap(),
                    Level::Info => rc.borrow_mut().as_mut().unwrap().write(format!("\n[{}] 🏁 {}: {}", Utc::now(), record.level(), record.args()).as_bytes()).unwrap(),
                    Level::Debug => rc.borrow_mut().as_mut().unwrap().write(format!("\n[{}] 🐜  {}: {}", Utc::now(), record.level(), record.args()).as_bytes()).unwrap(),
                    Level::Trace => rc.borrow_mut().as_mut().unwrap().write(format!("\n[{}] 🐜  {}: {}", Utc::now(), record.level(), record.args()).as_bytes()).unwrap(),
                };  
            });
        }
    }

    fn flush(&self) {
        LOGGER.with(|rc| {
            rc.borrow_mut().as_mut().unwrap().flush().unwrap();
        });
    }
}