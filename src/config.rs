use std::io::prelude::*;
use std::net::{TcpStream};
use std::error::Error;

use ssh2::{Session, ErrorCode, ExtendedData};

use serde::Deserialize;

use crate::logfile::{LogFile, LogSeverity};
use crate::authentication::Authenticator;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub targets: Vec<Target>,
    pub tasks: Vec<Task>,
}

#[derive(Debug, Deserialize)]
pub struct Target {
    pub host: String,
    port: Option<u16>,
    user: String,
    password: Option<String>,
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
    pub fn connect(&self, output: &mut LogFile, authenticator: &Option<impl Authenticator>) -> Result<Session, Box<dyn Error>> {
        // Open SSH Session to Address
        match TcpStream::connect(format!("{}:{}", self.host, self.port.unwrap_or(22u16)).as_str()) {
            Ok(tcp) => {
                match Session::new() {
                    Ok(mut session) => {
                        session.set_timeout(150000);
                        session.set_tcp_stream(tcp);
                        session.handshake()?;
                        match &self.password {
                            Some(password) => {
                                match password.as_str() {
                                    "bitwarden" => {
                                        match authenticator {
                                            Some(a) => session.userauth_password(&self.user, a.get(&self.host, &self.user)?)?,
                                            None => return Err(Box::new(ssh2::Error::new(ssh2::ErrorCode::Session(-18), "Authentication method not available")))
                                        }
                                    },
                                    _ => session.userauth_password(&self.user, password)?,
                                }
                            },
                            None => session.userauth_agent(&self.user)?,
                        }
                        
                        Ok(session)
                    },
                    Err(e) => {
                        output.write(LogSeverity::Failed, &format!("Connection Error: {}", e)).unwrap();
                        return Err(Box::new(ssh2::Error::new(ssh2::ErrorCode::Session(-26), "Connection Error")))
                    }
                }
            },
            Err(e) => {
                output.write(LogSeverity::Failed, &format!("Connection Error: {}", e)).unwrap();
                return Err(Box::new(ssh2::Error::new(ErrorCode::Session(-9), "Connection Error")))
            }
        }
    }
}

impl Task {
    pub fn run(&self, session: &Session, output: &mut LogFile) -> Result<State, Box<dyn Error>> {
        // Run command in session
        let mut channel = session.channel_session()?;
        
        // Add stderr stream to normal output
        channel.handle_extended_data(ExtendedData::Merge)?;

        channel.exec(&self.command)?;
        
        let mut buffer = String::new();

        // catch session timeout
        match channel.read_to_string(&mut buffer) {
            Err(_e) => {
                return Err(Box::new(ssh2::Error::new(ErrorCode::Session(-23), "Data Read Error/Timeout")))
            },
            _ => ()
        }

        channel.wait_close()?;

        // catch session timeout
        match output.write(LogSeverity::Info, &buffer) {
            Err(_e) => {
                return Err(Box::new(ssh2::Error::new(ErrorCode::Session(-16), "Data Write Error/Timeout")))
                },
            _ => ()
        }

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
                return Err(Box::new(e));
            }
        }
    }
}