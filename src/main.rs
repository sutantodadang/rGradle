mod build;
mod cli;
mod config;
mod fetch;
mod pom;
mod run;

use clap::Parser;
use cli::{Cli, Commands};
use config::{Config, load_config};
use std::fs;
use std::io::Write;
use std::path::Path;

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init => {
            println!("Initializing new rGradle project...");

            // Default configuration file
            let config = r#"
[project]
name = "MyJavaApp"
version = "0.1.0"
main_class = "com.example.Main"
source_dir = "example/main/java/com/example"
output_dir = "build"

[dependencies]
#example "junit:junit" = "4.13.2"
"#;

            let mut file = fs::File::create("rgradle.toml").expect("Failed to create rgradle.toml");
            file.write_all(config.trim_start().as_bytes())
                .expect("Failed to write rgradle.toml");

            let cfg = load_config();

            let src_dir = Path::new(cfg.project.source_dir.as_deref().unwrap_or("src/main/java"));

            let src_path = cfg.project.source_dir.clone().unwrap_or_default();

            let src_split = src_path.split("main").collect::<Vec<_>>();

            let src_split = src_split[0].to_string() + "main";

            let src_split = src_split + "/resources";

            let res_dir = Path::new(src_split.as_str());

            fs::create_dir_all(src_dir).expect("Failed to create src directory");
            fs::create_dir_all(res_dir).expect("Failed to create resources directory");

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
            let target_dir = Path::new(cfg.project.output_dir.as_deref().unwrap_or("build"));
            if target_dir.exists() {
                fs::remove_dir_all(target_dir).expect("Failed to delete target directory");
                println!("Deleted target directory.");
            } else {
                println!("Nothing to clean.");
            }
        }

        Commands::Run => {
            println!("Running Java application...");
            let cfg = load_config();
            run::run_project(&cfg);
        }
    }
}
