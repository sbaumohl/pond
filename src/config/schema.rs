use std::{collections::HashSet, fs};

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Keybind {
    pub key: String,
    pub command: String,
}

#[derive(Debug, Deserialize, Default)]
pub struct Configuration {
    pub ip: i64,
    pub keybinds: Vec<Keybind>,
}

impl Configuration {
    pub fn from_file(filepath: String) -> Self {
        let contents = fs::read_to_string(filepath).unwrap();

        match toml::from_str::<Configuration>(&contents) {
            Ok(config) => config,
            Err(_) => {
                println!("Error parsing config, defaulting...");
                Configuration::default()
            }
        }
    }
}
