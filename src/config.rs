use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
pub struct Project {
    pub name: String,
    pub version: String,
    pub main_class: String,
    pub source_dir: Option<String>,
    pub output_dir: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Config {
    pub project: Project,
    pub dependencies: Option<HashMap<String, String>>,
}

pub fn load_config() -> Config {
    let content = std::fs::read_to_string("rgradle.toml").expect("Cannot read rgradle.toml");

    toml::from_str(&content).expect("Failed to parse rgradle.toml")
}
