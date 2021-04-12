use std::error::Error;
use std::io;
use std::collections::HashMap;

use std::process::Command;
use std::str::from_utf8;

use serde::Deserialize;
use serde_json::Value;

use crate::authentication::Authenticator;

#[derive(Debug, Deserialize)]
struct Item {
    name: Option<String>,
    login: Option<LoginItem>,

    #[serde(flatten)]
    extra: HashMap<String, Value>
}

#[derive(Debug, Deserialize)]
struct LoginItem {
    username: Option<String>,
    password: Option<String>,

    #[serde(flatten)]
    extra: HashMap<String, Value>
}

pub struct Bitwarden {
    secrets: Vec<Item>
}

impl Authenticator for Bitwarden {
    // read bitwarden output, bitwarden cli must logged in before
    fn new(master_password: &str) -> Result<Self, Box<dyn Error>> {
        let output = Command::new("bw")
            .arg("unlock")
            .arg(master_password)
            .arg("--raw")
            .output()
            .expect("bitwarden cli command bw not found!");
            
        if !output.status.success() {
            return Err(Box::new(io::Error::new(io::ErrorKind::Other,from_utf8(&output.stderr)?)));
        }

        let json = Command::new("bw")
            .arg("list")
            .arg("items")
            .arg("--session")
            .arg(from_utf8(&output.stdout).unwrap())
            .arg("--raw")
            .output()
            .expect("bitwarden cli command bw not found!");

        
        Ok(Bitwarden {
            secrets: serde_json::from_slice(&json.stdout)?
        })
    }

    // get first matching password
    fn get(&self, name: &str, username: &str) -> Result<&str, Box<dyn Error>> {
        // TODO: fix checks with ? synatx if possible
        for item in &self.secrets {
            match &item.name {
                Some(item_name) => {
                    // check if hostname appears in bitwarden name
                    if item_name.contains(name) {
                        match &item.login {
                            Some(item_login) => {
                                match &item_login.username {
                                    Some(item_username) => {
                                        // check if username appears in bitwarden username
                                        if item_username.contains(&username) {
                                            match &item_login.password {
                                                // password found
                                                Some(item_password) => return Ok(item_password),
                                                None => ()
                                            }
                                        }
                                    },
                                    None => ()
                                }
                            },
                            None => ()
                        }
                    }
                },
                None => ()
            }
        }

        Err(Box::new(io::Error::new(io::ErrorKind::NotFound, "Password in Bitwarden not found!")))
    }
}