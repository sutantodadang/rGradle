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
            println!("Initializing new rrrGradle project...");

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
"junit:junit" = "4.13.2"
"org.hamcrest:hamcrest-core" = "1.3"
"#;

            let mut file =
                fs::File::create("rrrgradle.toml").expect("Failed to create rrrgradle.toml");
            file.write_all(config.trim_start().as_bytes())
                .expect("Failed to write rrrgradle.toml");

            let cfg = load_config();            // Create main source and resource directories
            if let Some(main) = &cfg.main {
                if let Some(java_dirs) = &main.java {
                    for dir in java_dirs {
                        fs::create_dir_all(dir).expect("Failed to create main java directory");
                        
                        // Create main class if specified
                        if let main_class= &cfg.project.main_class {
                            let class_parts: Vec<&str> = main_class.split('.').collect();
                            if !class_parts.is_empty() {
                                let class_name = class_parts.last().unwrap();
                                let package_path = &class_parts[..class_parts.len()-1].join("/");
                                let package_dir = format!("{}/{}", dir, package_path);
                                fs::create_dir_all(&package_dir).expect("Failed to create package directory");
                                
                                let class_path = format!("{}/{}.java", package_dir, class_name);
                                let mut class_file = fs::File::create(&class_path)
                                    .expect("Failed to create main class file");
                                
                                // Write sample main class
                                let class_content = format!(r#"package {};

public class {} {{
    public static void main(String[] args) {{
        System.out.println("Hello from rrrGradle!");
    }}
}}"#, class_parts[..class_parts.len()-1].join("."), class_name);
                                
                                class_file.write_all(class_content.as_bytes())
                                    .expect("Failed to write main class content");
                                    
                                println!("Created main class at: {}", class_path);
                            }
                        }
                    }
                }
                if let Some(resource_dirs) = &main.resources {
                    for dir in resource_dirs {
                        fs::create_dir_all(dir).expect("Failed to create main resources directory");
                    }
                }
            }            // Create test source and resource directories
            if let Some(test) = &cfg.test {
                if let Some(java_dirs) = &test.java {
                    for dir in java_dirs {
                        fs::create_dir_all(dir).expect("Failed to create test java directory");
                        
                        // Create corresponding test class for the main class
                        if let main_class = &cfg.project.main_class {
                            let class_parts: Vec<&str> = main_class.split('.').collect();
                            if !class_parts.is_empty() {
                                let class_name = class_parts.last().unwrap();
                                let package_path = &class_parts[..class_parts.len()-1].join("/");
                                let test_package_dir = format!("{}/{}", dir, package_path);
                                fs::create_dir_all(&test_package_dir).expect("Failed to create test package directory");
                                
                                let test_class_path = format!("{}/{}Test.java", test_package_dir, class_name);
                                let mut test_class_file = fs::File::create(&test_class_path)
                                    .expect("Failed to create test class file");
                                
                                // Write sample test class with JUnit
                                let test_content = format!(r#"package {};

import org.junit.Test;
import static org.junit.Assert.*;

public class {}Test {{
    @Test
    public void testSampleFunction() {{
        // TODO: Add your test cases here
        assertTrue("Default test case", true);
    }}
}}"#, class_parts[..class_parts.len()-1].join("."), class_name);
                                
                                test_class_file.write_all(test_content.as_bytes())
                                    .expect("Failed to write test class content");
                                    
                                println!("Created test class at: {}", test_class_path);
                            }
                        }
                    }
                }
                if let Some(resource_dirs) = &test.resources {
                    for dir in resource_dirs {
                        fs::create_dir_all(dir).expect("Failed to create test resources directory");
                    }
                }
            }

            println!("Project structure created.");
            println!("Edit `rrrgradle.toml` to define your dependencies.");
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
