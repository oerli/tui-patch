use std::io::prelude::*;
use std::net::{TcpStream};
use std::error::Error;

use ssh2::{Session, ErrorCode, ExtendedData};

use serde::Deserialize;
use std::collections::HashMap;

use log::{error, warn, info};

use crate::authenticator::Authenticator;

use crate::resolver::Resolver;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub targets: Vec<Target>,

    #[serde(flatten)]
    tasks: HashMap<String, Vec<Task>>,
}

#[derive(Debug, Deserialize)]
pub struct Target {
    pub host: String,
    ip: Option<String>,
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
    pub fn connect(&self, authenticator: &Option<impl Authenticator>, resolver: &Option<impl Resolver>) -> Result<Session, Box<dyn Error>> {
        // try resolver
        let address = match resolver {
            // always fall back to dns name
            Some(resolver) => {
                match resolver.get(&self.host) {
                    Ok(ip) => {
                        info!("found ip address: {}", ip);
                        ip
                    },
                    Err(e) => {
                        error!("unable to resolve ip: {}", e);
                        self.host.to_string()
                    }
                }
            },
            None => self.host.to_string(),
        };

        // Open SSH Session to Address
        match TcpStream::connect(format!("{}:{}", address, self.port.unwrap_or(22u16)).as_str()) {
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
                                            None => return Err(Box::new(ssh2::Error::new(ssh2::ErrorCode::Session(-18), "authentication method not available")))
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
                        error!("Connection Error: {}", e);
                        return Err(Box::new(ssh2::Error::new(ssh2::ErrorCode::Session(-26), "Connection Error")))
                    }
                }
            },
            Err(e) => {
                error!("Connection Error: {} {}", format!("{}:{}", address, self.port.unwrap_or(22u16)).as_str(), e);
                return Err(Box::new(ssh2::Error::new(ErrorCode::Session(-9), "Connection Error")))
            }
        }
    }
}

impl Task {
    pub fn run(&self, session: &Session) -> Result<State, Box<dyn Error>> {
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

        // write output to logfile
        info!("{}", buffer);

        match channel.exit_status() {
            Ok(r) => {
                if self.expected_result == r {
                    return Ok(State::Ok);
                } else {
                    if self.stop_on_error == true {
                        error!("expected result {} but recieved {}", self.expected_result, r);
                        return Ok(State::Failed);
                    } else {
                        warn!("expected result {} but recieved {}", self.expected_result, r);
                        return Ok(State::Warning);
                    }
                }
            },
            Err(e) => {
                error!("{}", e);
                return Err(Box::new(e));
            }
        }
    }
}