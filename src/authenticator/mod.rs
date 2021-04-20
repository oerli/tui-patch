pub mod bitwarden;

use std::error::Error;

pub trait Authenticator {
    fn new(master_password: &str) -> Result<Self, Box<dyn Error>> where Self: Sized;
    fn get(&self, hostname: &str, user: &str) -> Result<&str, Box<dyn Error>>;
}