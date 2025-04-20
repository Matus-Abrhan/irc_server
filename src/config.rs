use std::fs::read_to_string;
use std::net::IpAddr;
use std::process::exit;
use once_cell::sync::Lazy;
use std::sync::{Arc, Mutex};

use toml;
use serde_derive::Deserialize;

pub static CONFIG: Lazy<Arc<Mutex<Config>>> = Lazy::new(|| {
    Arc::new(Mutex::new(Config::new("config.toml")))
});


#[derive(Deserialize, Clone)]
pub struct Config {
    pub server: Server,
}

#[derive(Deserialize, Clone)]
pub struct Server {
    pub name: String,
    pub password: String,

    pub address_v4: IpAddr,
    pub port: u16,
}

impl Config {
    pub fn new(path: &str) -> Self {
        let contents  = match read_to_string(path) {
            Ok(c) => c,
            Err(_) => {
                eprintln!("Could not read file \"{}\"", path);
                exit(1);
            }
        };

        let config: Config = match toml::from_str(&contents) {
            Ok(c) => c,
            Err(_) => {
                eprintln!("Could not load data from file \"{}\"", path);
                exit(1);
            }
        };

        return config;
    }
}
