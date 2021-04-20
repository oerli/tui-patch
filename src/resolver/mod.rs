pub mod phpipam;

use std::error::Error;
use reqwest::Url;

pub trait Resolver {
    fn new(url: Url) -> Result<Self, Box<dyn Error>> where Self: Sized;
    fn get(&self, hostname: &str) -> Result<String, Box<dyn Error>>;
}