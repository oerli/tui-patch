use super::Resolver;

use std::error::Error;
use std::io;

use reqwest::{blocking::Client, Url};
use http::Method;
use std::collections::HashMap;

use serde_json::Value;
use serde::Deserialize;

pub struct PhpIpam {
    url: reqwest::Url,
    client: reqwest::blocking::Client,
}

#[derive(Debug, Deserialize)]
struct PhpIpamResponse {
    code: u16,
    data: Option<Vec<HashMap<String, Value>>>,
    message: Option<String>,
    success: bool,
    time: f32,
}

impl Resolver for PhpIpam {
    fn new(base_url: Url) -> Result<Self, Box<dyn Error>> {
        let client = Client::new();

        // test connection/get permission request, only api code is implemented
        client.request(Method::OPTIONS, format!("{}/api/{}/", base_url, base_url.username())).header("token", base_url.password().ok_or("no password specified")?).send()?;
        
        Ok(PhpIpam {
            url: base_url,
            client: client,
        })
    }

    fn get(&self, hostname: &str) -> Result<String, Box<dyn Error>> {
        let phpipam_item: PhpIpamResponse = self.client.get(format!("{}/api/{}/addresses/search_hostname/{}/", self.url, self.url.username(), hostname)).header("token", self.url.password().ok_or("no password specified")?).send()?.json()?;

        match phpipam_item.success {
            true => {
                match phpipam_item.data {
                    Some(v) => {
                        if v.len() == 1 {
                            match v.get(0).unwrap().get("ip") {
                                Some(Value::String(s)) => {
                                    return Ok(s.to_string());
                                },
                                _ => return Err(Box::new(io::Error::new(io::ErrorKind::NotFound, "no ip address found"))),
                            }
                        } else {
                            return Err(Box::new(io::Error::new(io::ErrorKind::InvalidData, "no data or too many results received")));
                        }
                    },
                    None => return Err(Box::new(io::Error::new(io::ErrorKind::NotFound, "no data received"))),
                }
                
            },
            false => {
                match phpipam_item.message {
                    Some(s) => {
                        return Err(Box::new(io::Error::new(io::ErrorKind::NotFound, s)));
                    },
                    _ => return Err(Box::new(io::Error::new(io::ErrorKind::NotFound, "phpipam data error"))),
                }
                
            }
        }
    }
}