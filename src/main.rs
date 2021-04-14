use std::io::prelude::*;

use structopt::StructOpt;
use std::path::PathBuf;

use rpassword;

use std::thread;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};

use std::sync::Arc;

use std::fs::{File};

mod logfile;
use logfile::{LogFile, LogSeverity};

mod config;
use config::{Config, State};

mod bitwarden;
use bitwarden::Bitwarden;

mod authentication;
use authentication::Authenticator;

// TODO:
// - update while waiting
// - add timeout to yaml
// - check ssh agent running
// - ssh key added
// - limit processes
// - add dependecies of server
// - alternative file structure with just a list of hosts/tasks

#[derive(Debug, StructOpt)]
#[structopt(name = "tui-patch", about = "Run SSH commands from a YAML script file in parallel.")]
struct Opt {
    #[structopt(parse(from_os_str), help = "YAML script file, for format details see in examples folder.")]
    config: PathBuf,

    #[structopt(default_value = "./log", short, long, help = "Specify the log output directory, the directory will be created if it does not exist. Each logfile will be created with hostname and timestamp.")]
    log: String,

    #[structopt(short, long, help = "Pass your Bitwarden master password to unlock the vault. Specify '-' to get prompt to enter hidden password. Setup bitwarden-cli before use (bw login).")]
    bitwarden: Option<String>,
}

fn main() {
    // read parameters
    let args = Opt::from_args();

    // check if bitwarden password should be read by user input
    let bitwarden_secret = match args.bitwarden {
        Some(secret) => { match secret == "-" {
            true => rpassword::read_password_from_tty(Some("Bitwarden Master Password: ")).ok(),
            false => Some(secret)
        }},
        None => None
    };

    // open config file
    let mut config_file = match File::open(args.config) {
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

    // load bitwarden
    let bitwarden = match &bitwarden_secret {
        // exit if password is wrong
        Some(path) => Arc::new(Some(Bitwarden::new(path).unwrap())),
        None => Arc::new(None)
    };

    // create multithreaded progress bar
    let multi_progress = MultiProgress::new();
    let style = ProgressStyle::default_bar().template("[{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}").progress_chars("##-");

    let log_directory: Arc<String> = Arc::new(args.log);

    for target in config.targets {
        // add progress bar for thread
        let count = target.tasks.len();
        let progress = multi_progress.add(ProgressBar::new(count as u64));
        progress.set_style(style.clone());

        // create a read only copy for each thread
        let authenticator = bitwarden.clone();
        
        // copy path for logs
        let log = log_directory.clone();

        let _ = thread::spawn(move || {
            progress.set_message(target.host.as_str());
            
            // create a logfile
            let mut log_file = LogFile::new(&*log, &target.host);
            let mut worst_sate = State::Ok;

            match target.connect(&mut log_file, &*authenticator) {
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
                // unreachable
                State::Failed => {}
            }
        });
    }

    // wait for threads to be finished
    multi_progress.join().unwrap();
}