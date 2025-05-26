use crate::config::{Config, SourceSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use walkdir::WalkDir;

fn find_java_files(dirs: &[String]) -> Vec<PathBuf> {
    let mut java_files = Vec::new();
    for dir in dirs {
        if !Path::new(dir).exists() {
            continue;
        }
        for entry in WalkDir::new(dir).into_iter().filter_map(|e| e.ok()) {
            if entry.path().extension().map_or(false, |ext| ext == "java") {
                java_files.push(entry.path().to_owned());
            }
        }
    }
    java_files
}

fn copy_resources(source_set: &SourceSet, output_dir: &Path) {
    if let Some(resource_dirs) = &source_set.resources {
        for dir in resource_dirs {
            if !Path::new(dir).exists() {
                continue;
            }
            for entry in WalkDir::new(dir).into_iter().filter_map(|e| e.ok()) {
                if entry.path().is_file() {
                    if let Ok(rel_path) = entry.path().strip_prefix(dir) {
                        let target = output_dir.join(rel_path);
                        if let Some(parent) = target.parent() {
                            let _ = fs::create_dir_all(parent);
                        }
                        let _ = fs::copy(entry.path(), target);
                    }
                }
            }
        }
    }
}

fn build_classpath(cache_dir: &str, main_output: Option<&str>, is_test: bool) -> String {
    let sep = if cfg!(windows) { ";" } else { ":" };
    let mut cp_entries = Vec::new();

    // Add dependency JARs
    if let Ok(entries) = fs::read_dir(cache_dir) {
        for entry in entries.filter_map(|e| e.ok()) {
            if entry.path().extension().map_or(false, |ext| ext == "jar") {
                cp_entries.push(entry.path().to_string_lossy().to_string());
            }
        }
    }

    // For test compilation, add main classes to classpath
    if is_test {
        if let Some(main_out) = main_output {
            cp_entries.push(main_out.to_string());
        }
    }

    cp_entries.join(sep)
}

fn compile_source_set(
    source_set: &SourceSet,
    cache_dir: &str,
    is_test: bool,
    main_output: Option<&str>,
) -> bool {
    // Get output directory
    let output_dir = source_set.output.as_deref().unwrap_or(if is_test {
        "build/classes/java/test"
    } else {
        "build/classes/java/main"
    });

    // Create output directory
    fs::create_dir_all(output_dir).expect("Failed to create output directory");

    // Find all .java files
    let all_java_files = if let Some(java_dirs) = &source_set.java {
        let mut files = Vec::new();
        for dir in java_dirs {
            // Use the entire java directory as the source root
            let source_root = Path::new(dir);
            for entry in WalkDir::new(dir).into_iter().filter_map(|e| e.ok()) {
                if entry.path().extension().map_or(false, |ext| ext == "java") {
                    files.push((entry.path().to_owned(), source_root.to_owned()));
                }
            }
        }
        files
    } else {
        Vec::new()
    };

    if all_java_files.is_empty() {
        println!(
            "No Java files found to compile in {} source set.",
            if is_test { "test" } else { "main" }
        );
        return true;
    }

    // Incremental build: only recompile if .java is newer than .class or .class is missing
    let mut to_compile = Vec::new();
    for (java_file, source_root) in &all_java_files {
        // Get the relative path from the source root to preserve package structure
        let rel_path = java_file.strip_prefix(source_root).unwrap_or(java_file);

        // Change extension to .class while maintaining full package path
        let class_file = {
            let mut p = PathBuf::from(output_dir);
            p.push(rel_path);
            p.set_extension("class");
            p
        };

        // Create parent directories for the class file if they don't exist
        if let Some(parent) = class_file.parent() {
            fs::create_dir_all(parent).expect("Failed to create output directory structure");
        }

        let needs_recompile = match (fs::metadata(java_file), fs::metadata(&class_file)) {
            (Ok(java_meta), Ok(class_meta)) => {
                let java_mtime = java_meta
                    .modified()
                    .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
                let class_mtime = class_meta
                    .modified()
                    .unwrap_or(std::time::SystemTime::UNIX_EPOCH);

                java_mtime > class_mtime
            }
            (Ok(_), Err(_)) => {
                println!("Class file missing: {}", class_file.display());
                true
            }
            _ => {
                println!("Error reading file metadata, defaulting to rebuild");
                true
            }
        };

        if needs_recompile {
            to_compile.push(java_file.clone());
        }
    }

    if to_compile.is_empty() {
        println!("✓ Nothing to compile (incremental build up-to-date).");
        // Still copy resources in case they changed
        copy_resources(source_set, Path::new(output_dir));
        return true;
    }

    println!(
        "Compiling {} {} source file(s)...",
        to_compile.len(),
        if is_test { "test" } else { "main" }
    );

    // Build classpath
    let classpath = build_classpath(cache_dir, main_output, is_test);

    // Compile Java files
    let mut cmd = Command::new("javac");
    cmd.arg("-d").arg(output_dir);

    if !classpath.is_empty() {
        cmd.arg("-cp").arg(classpath);
    }

    cmd.args(&to_compile);

    let status = cmd.status().expect("Failed to run javac");

    if status.success() {
        // Copy resources after successful compilation
        copy_resources(source_set, Path::new(output_dir));
        println!(
            "✓ {} compilation successful ({} file(s) compiled)",
            if is_test { "Test" } else { "Main" },
            to_compile.len()
        );
        true
    } else {
        eprintln!(
            "✗ {} compilation failed",
            if is_test { "Test" } else { "Main" }
        );
        false
    }
}

pub fn build_project(config: &Config) -> bool {
    let cache_dir = ".rgradle/cache";

    // Compile main source set
    let main_success = if let Some(main) = &config.main {
        compile_source_set(main, cache_dir, false, None)
    } else {
        true
    };

    // Compile test source set if main compilation succeeded
    if main_success {
        if let Some(test) = &config.test {
            let main_output = config.main.as_ref().and_then(|m| m.output.as_deref());
            compile_source_set(test, cache_dir, true, main_output)
        } else {
            true
        }
    } else {
        false
    }
}
