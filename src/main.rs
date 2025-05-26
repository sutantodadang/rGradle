mod build;
mod cli;
mod config;
mod fetch;
mod package;
mod pom;
mod run;
mod test;

use clap::Parser;
use cli::{Cli, Commands};
use config::load_config;
use std::fs;
use std::io::Write;
use std::path::Path;

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init => {
            println!("Initializing new rGradle project...");

            let config = r#"
[project]
name = "MyJavaApp"
version = "0.1.0"
main_class = "com.example.Main"

[main]
java = ["src/main/java"]
resources = ["src/main/resources"]
output = "build/classes/java/main"

[test]
java = ["src/test/java"]
resources = ["src/test/resources"]
output = "build/classes/java/test"

[dependencies]
# Main dependencies go here
# example: "org.slf4j:slf4j-api" = "2.0.9"

[test_dependencies]
# example: "junit:junit" = "4.13.2"
"#;

            let mut file = fs::File::create("rgradle.toml").expect("Failed to create rgradle.toml");
            file.write_all(config.trim_start().as_bytes())
                .expect("Failed to write rgradle.toml");

            let cfg = load_config();

            // Create main source and resource directories
            if let Some(main) = &cfg.main {
                if let Some(java_dirs) = &main.java {
                    for dir in java_dirs {
                        fs::create_dir_all(dir).expect("Failed to create main java directory");
                    }
                }
                if let Some(resource_dirs) = &main.resources {
                    for dir in resource_dirs {
                        fs::create_dir_all(dir).expect("Failed to create main resources directory");
                    }
                }
            }

            // Create test source and resource directories
            if let Some(test) = &cfg.test {
                if let Some(java_dirs) = &test.java {
                    for dir in java_dirs {
                        fs::create_dir_all(dir).expect("Failed to create test java directory");
                    }
                }
                if let Some(resource_dirs) = &test.resources {
                    for dir in resource_dirs {
                        fs::create_dir_all(dir).expect("Failed to create test resources directory");
                    }
                }
            }

            println!("Project structure created.");
            println!("Edit `rgradle.toml` to define your dependencies.");
        }

        Commands::Fetch => {
            println!("Fetching dependencies...");
            let cfg = load_config();
            fetch::fetch_dependencies(&cfg).await;
        }

        Commands::Build => {
            println!("Building project...");
            let cfg = load_config();
            build::build_project(&cfg);
        }

        Commands::Clean => {
            println!("Cleaning build directory...");
            let cfg = load_config();
            let build_dir = Path::new("build");
            if build_dir.exists() {
                fs::remove_dir_all(build_dir).expect("Failed to delete build directory");
                println!("Deleted build directory.");
            } else {
                println!("Nothing to clean.");
            }
        }

        Commands::Run => {
            println!("Running Java application...");
            let cfg = load_config();
            run::run_project(&cfg);
        }

        Commands::Test => {
            println!("Running tests...");
            let cfg = load_config();
            test::test_project(&cfg);
        }

        Commands::Package { uber } => {
            println!("Packaging project{}", if uber { " (uber JAR)" } else { "" });
            let cfg = load_config();
            if let Err(e) = package::package_project(&cfg, uber) {
                eprintln!("✗ Packaging failed: {}", e);
            }
        }
    }
}
