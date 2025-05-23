use crate::config::Config;
use std::fs;
use std::process::Command;

pub fn run_project(config: &Config) {
    let main_class = &config.project.main_class;
    let target_dir = config.project.output_dir.clone().unwrap_or_default();
    let cache_dir = ".rgradle/cache";

    // Collect all JARs in .rgradle/cache for classpath
    let mut classpath = target_dir;
    let jars = fs::read_dir(cache_dir)
        .ok()
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| p.extension().map_or(false, |ext| ext == "jar"))
                .map(|p| p.to_string_lossy().to_string())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if !jars.is_empty() {
        classpath = format!("{};{}", classpath, jars.join(";"));
    }

    let status = Command::new("java")
        .arg("-cp")
        .arg(classpath)
        .arg(main_class)
        .status()
        .expect("Failed to run java");

    if status.success() {
        println!("✓ Run successful.");
    } else {
        eprintln!("✗ Run failed.");
    }
}
