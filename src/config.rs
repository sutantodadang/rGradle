use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
pub struct SourceSet {
    pub java: Option<Vec<String>>,      // Java source directories
    pub resources: Option<Vec<String>>, // Resource directories
    pub output: Option<String>,         // Output directory for this source set
}

#[derive(Debug, Deserialize)]
pub struct Project {
    pub name: String,
    pub version: String,
    pub main_class: String,

    // New fields for directory configuration
    pub source_dir: Option<String>,   // Source directory
    pub resource_dir: Option<String>, // Resource directory
    pub output_dir: Option<String>,   // Output directory
}

#[derive(Debug, Deserialize)]
pub struct Config {
    pub project: Project,
    pub main: Option<SourceSet>, // Main source set
    pub test: Option<SourceSet>, // Test source set
    pub dependencies: Option<HashMap<String, String>>,
    pub test_dependencies: Option<HashMap<String, String>>,
}

impl Default for SourceSet {
    fn default() -> Self {
        Self {
            java: Some(vec![]),
            resources: Some(vec![]),
            output: None,
        }
    }
}

pub fn load_config() -> Config {
    let content = std::fs::read_to_string("rrrgradle.toml").expect("Cannot read rrrgradle.toml");
    let mut config: Config = toml::from_str(&content).expect("Failed to parse rrrgradle.toml");

    // Set defaults if not specified
    if config.main.is_none() {
        config.main = Some(SourceSet {
            java: Some(vec!["src/main/java".to_string()]),
            resources: Some(vec!["src/main/resources".to_string()]),
            output: Some("build/classes/java/main".to_string()),
        });
    }

    if config.test.is_none() {
        config.test = Some(SourceSet {
            java: Some(vec!["src/test/java".to_string()]),
            resources: Some(vec!["src/test/resources".to_string()]),
            output: Some("build/classes/java/test".to_string()),
        });
    }

    config
}
