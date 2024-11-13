use std::fs::read_to_string;
use std::process::exit;

use toml;
use serde_derive::Deserialize;

#[derive(Deserialize, Clone)]
pub struct Config {
    pub server: Server,
}

#[derive(Deserialize, Clone)]
pub struct Server {
    pub name: String,

    pub address_v4: String,
    pub port: u16,
}

impl Config {
    pub fn read(path: &str) -> Self {
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
