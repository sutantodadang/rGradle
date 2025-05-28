use crate::config::Config;
use std::path::Path;
use std::process::Command;
use walkdir::WalkDir;

fn find_test_classes(dir: &str) -> Vec<String> {
    let mut test_classes = Vec::new();

    for entry in WalkDir::new(dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .file_name()
                .map_or(false, |n| n.to_string_lossy().ends_with("Test.java"))
        })
    {
        // Convert path/to/com/example/Test.java to com.example.Test
        if let Some(rel_path) = entry.path().strip_prefix(dir).ok() {
            if let Some(path_str) = rel_path.to_str() {
                let class_name = path_str
                    .trim_end_matches(".java")
                    .replace('\\', ".")
                    .replace('/', ".");
                test_classes.push(class_name);
            }
        }
    }

    test_classes
}

pub fn test_project(config: &Config) {
    // Get test output directory from config
    let test_output = config
        .test
        .as_ref()
        .and_then(|t| t.output.as_deref())
        .unwrap_or("build/classes/java/test");

    // Get main output directory for test classpath
    let main_output = config
        .main
        .as_ref()
        .and_then(|m| m.output.as_deref())
        .unwrap_or("build/classes/java/main");

    // Find test classes
    let mut test_classes = Vec::new();
    for entry in WalkDir::new(test_output).into_iter().filter_map(|e| e.ok()) {
        if entry.path().extension().map_or(false, |ext| ext == "class")
            && entry.path().to_string_lossy().contains("Test")
        {
            // Convert file path to Java class name (com.example.MainTest)
            if let Ok(rel_path) = entry.path().strip_prefix(test_output) {
                let class_path = rel_path.with_extension("");
                let class_name = class_path
                    .to_string_lossy()
                    .replace('\\', ".")
                    .replace('/', ".");
                test_classes.push(class_name.to_string());
            }
        }
    }

    if test_classes.is_empty() {
        println!("No test classes found.");
        return;
    }

    println!("Running tests...");

    // Build classpath: main classes + test classes + all dependency JARs
    let sep = if cfg!(windows) { ";" } else { ":" };
    let mut cp_parts = vec![test_output.to_string(), main_output.to_string()];

    // Add all JARs from cache
    if let Ok(entries) = std::fs::read_dir(".rrrgradle/cache") {
        for entry in entries.filter_map(|e| e.ok()) {
            if entry.path().extension().map_or(false, |ext| ext == "jar") {
                cp_parts.push(entry.path().to_string_lossy().to_string());
            }
        }
    }

    let classpath = cp_parts.join(sep);

    // Run tests using JUnit
    let mut cmd = Command::new("java");
    cmd.arg("-cp")
        .arg(&classpath)
        .arg("org.junit.runner.JUnitCore")
        .args(&test_classes);

    match cmd.status() {
        Ok(status) if status.success() => {
            println!("✓ All tests passed.");
        }
        _ => {
            eprintln!("✗ Some tests failed.");
            std::process::exit(1);
        }
    }
}
