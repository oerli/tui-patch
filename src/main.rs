use std::env;

use std::io::prelude::*;

use std::thread;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};

use std::fs::{File};

mod logfile;
use logfile::{LogFile, LogSeverity};

mod config;
use config::{Config, State};

// TODO:
// - check for ssh key loaded and corresponds with username
// - update while waiting
// - timeout for ssh connection
// - fix to catch all errors
// - add timeout to yaml
// - check ssh agent running
// - ssh key added
// - todo limit processes

fn main() {
    // read parameters
    let args: Vec<String> = env::args().collect();
    
    // open config file
    let mut config_file = match File::open(&args[1]) {
        Ok(file) => file,
        Err(error) => panic!("{}", error)
    };

    // read config file
    let mut raw_config = String::new();
    match config_file.read_to_string(&mut raw_config) {
        Ok(string) => string,
        Err(error) => panic!("{}", error)
    };
    
    let config: Config = serde_yaml::from_str(&raw_config).unwrap();

    // // create multithreaded progress bar
    let multi_progress = MultiProgress::new();
    let style = ProgressStyle::default_bar().template("[{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}").progress_chars("##-");


    for target in config.targets {
        // add progress bar for thread
        let count = target.tasks.len();
        let progress = multi_progress.add(ProgressBar::new(count as u64));
        progress.set_style(style.clone());

        let _ = thread::spawn(move || {
            progress.set_message(target.host.as_str());

            // create a logfile
            let mut log_file = LogFile::new("./log", &target.host);
            let mut worst_sate = State::Ok;

            match target.connect(&mut log_file) {
                Ok(c) => {
                    for task in target.tasks {
                        match task.run(&c, &mut log_file) {
                            Ok(r) => {
                                match r {
                                    State::Ok => {
                                        progress.inc(1);
                                    },
                                    State::Warning => {
                                        worst_sate = State::Warning;
                                        progress.set_style(ProgressStyle::default_bar().template("[{elapsed_precise}] {bar:40.magenta/red} {pos:>7}/{len:7} {msg}").progress_chars("##-"));
                                        progress.set_message(&format!("{}: warning.", &target.host));
                                        progress.inc(1);

                                    },
                                    State::Failed => {
                                        worst_sate = State::Failed;
                                        progress.set_style(ProgressStyle::default_bar().template("[{elapsed_precise}] {bar:40.magenta/red} {pos:>7}/{len:7} {msg}").progress_chars("##-"));
                                        progress.set_message(&format!("{}: command failed.", &target.host));
                                        progress.finish_at_current_pos();
                                        return
                                    },
                                }
                            },
                            Err(e) => {
                                progress.set_style(ProgressStyle::default_bar().template("[{elapsed_precise}] {bar:40.red/red} {pos:>7}/{len:7} {msg}").progress_chars("XX-"));
                                progress.finish_at_current_pos();
                                log_file.write(LogSeverity::Error, &e.to_string()).unwrap();
                                return
                            }
                        }
                    }
                },
                Err(e) => {
                    progress.set_style(ProgressStyle::default_bar().template("[{elapsed_precise}] {bar:40.red/red} {pos:>7}/{len:7} {msg}").progress_chars("XX-"));
                    progress.set_message(&format!("{}: connection failed.", &target.host));
                    progress.finish_at_current_pos();
                    log_file.write(LogSeverity::Error, &e.to_string()).unwrap();
                    return
                }
            }

            // execution finished for a target
            match worst_sate {
                State::Ok => {
                    progress.set_style(ProgressStyle::default_bar().template("[{elapsed_precise}] {bar:40.green/green} {pos:>7}/{len:7} {msg}").progress_chars("##-"));
                    progress.finish_with_message(&format!("{}: done.", &target.host));
                },
                State::Warning => {
                    progress.finish_with_message(&format!("{}: done with warnings.", &target.host));
                },
                State::Failed => {},
            }
        });
    }

    // wait for threads to be finished
    multi_progress.join().unwrap();
}