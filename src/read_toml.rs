use std::fs;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct Config {
    pub public: ServerDefinition,
    pub ezorg: ServerDefinition,
    pub domain: ServerDefinition,
}

#[derive(Deserialize)]
pub struct ServerDefinition {
    pub servers: String,
}

pub fn read_toml() -> Config {
    let file_contents = fs::read_to_string(&"./servers.toml").unwrap();
    toml::from_str(&file_contents).unwrap()
}