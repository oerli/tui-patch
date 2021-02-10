use std::io::prelude::*;
use std::net::{TcpStream};
use ssh2::{Session, Error, ErrorCode};

use serde::Deserialize;

use crate::logfile::{LogFile, LogSeverity};

#[derive(Debug, Deserialize)]
pub struct Config {
    pub targets: Vec<Target>,
    pub tasks: Vec<Task>,
}

#[derive(Debug, Deserialize)]
pub struct Target {
    pub host: String,
    port: u16,
    user: String,
    pub tasks: Vec<Task>,
}

#[derive(Debug, Deserialize)]
pub struct Task {
    command: String,
    expected_result: i32,
    stop_on_error: bool,
}

#[derive(Debug)]
pub enum State {
    Failed,
    Warning,
    Ok,
}

impl Target {
    pub fn connect(&self, output: &mut LogFile) -> Result<Session, Error> {
        // Open SSH Session to Address
        match TcpStream::connect(format!("{}:{}", self.host, self.port).as_str()) {
            Ok(tcp) => {
                match Session::new() {
                    Ok(mut session) => {
                        session.set_timeout(150000);
                        session.set_tcp_stream(tcp);
                        session.handshake()?;
                        session.userauth_agent(&self.user)?;
                        Ok(session)
                    },
                    Err(e) => {
                        output.write(LogSeverity::Failed, &format!("Connection Error: {}", e)).unwrap();
                        return Err(Error::new(ErrorCode::Session(-26), "Connection Error"))
                    }
                }
            },
            Err(e) => {
                output.write(LogSeverity::Failed, &format!("Connection Error: {}", e)).unwrap();
                return Err(Error::new(ErrorCode::Session(-9), "Connection Error"))
            }
        }
    }
}

impl Task {
    pub fn run(&self, session: &Session, output: &mut LogFile) -> Result<State, Error> {
        // Run command in session
        let mut channel = session.channel_session()?;
        channel.exec(&self.command)?;
        
        let mut buffer = String::new();
        channel.read_to_string(&mut buffer).unwrap();
        
        channel.wait_close()?;

        output.write(LogSeverity::Info, &buffer).unwrap();

        match channel.exit_status() {
            Ok(r) => {
                if self.expected_result == r {
                    return Ok(State::Ok);
                } else {
                    if self.stop_on_error == true {
                        output.write(LogSeverity::Failed, &format!("expected result {} but recieved {}", self.expected_result, r)).unwrap();
                        return Ok(State::Failed);
                    } else {
                        output.write(LogSeverity::Warning, &format!("expected result {} but recieved {}", self.expected_result, r)).unwrap();
                        return Ok(State::Warning);
                    }
                }
            },
            Err(e) => {
                output.write(LogSeverity::Error, &e.to_string()).unwrap();
                return Err(e);
            }
        }
    }
}