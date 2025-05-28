use crate::config::Config;
use std::process::Command;

pub fn run_project(config: &Config) {
    // Get main output directory
    let main_output = config
        .main
        .as_ref()
        .and_then(|m| m.output.as_deref())
        .unwrap_or("build/classes/java/main");

    // Build classpath: main classes + all dependency JARs
    let sep = if cfg!(windows) { ";" } else { ":" };
    let mut cp_parts = vec![main_output.to_string()];

    // Add all JARs from cache
    if let Ok(entries) = std::fs::read_dir(".rrrgradle/cache") {
        for entry in entries.filter_map(|e| e.ok()) {
            if entry.path().extension().map_or(false, |ext| ext == "jar") {
                cp_parts.push(entry.path().to_string_lossy().to_string());
            }
        }
    }

    let classpath = cp_parts.join(sep);

    // Run the main class
    let mut cmd = Command::new("java");
    cmd.arg("-cp")
        .arg(&classpath)
        .arg(&config.project.main_class);

    match cmd.status() {
        Ok(status) if status.success() => {
            println!("✓ Application finished successfully.");
        }
        _ => {
            eprintln!("✗ Application failed to run.");
            std::process::exit(1);
        }
    }
}
