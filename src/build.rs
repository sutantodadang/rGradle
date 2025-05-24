use crate::config::Config;
use std::fs;
use std::process::Command;

pub fn build_project(config: &Config) {
    let source_dir = config.project.source_dir.clone().unwrap_or_default();
    let target_dir = config.project.output_dir.clone().unwrap_or_default();

    fs::create_dir_all(&target_dir).expect("Failed to create target/classes");

    // Find all .java files recursively
    let java_files = walkdir::WalkDir::new(&source_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "java"))
        .map(|e| e.path().to_owned())
        .collect::<Vec<_>>();

    if java_files.is_empty() {
        println!("No Java files found to compile.");
        return;
    }

    // Incremental build: only recompile if .java is newer than .class or .class is missing
    let mut to_compile = Vec::new();
    for java_file in &java_files {
        // Get the full path relative to source root (example/main/java)
        let source_root = std::path::Path::new(&source_dir)
            .parent()
            .and_then(|p| p.parent())
            .unwrap_or(std::path::Path::new(&source_dir));
        let rel_path = java_file.strip_prefix(source_root).unwrap_or(java_file);

        // Change extension to .class while maintaining full package path
        let class_file = {
            let mut p = std::path::PathBuf::from(&target_dir);
            p.push(rel_path);
            p.set_extension("class");
            p
        };

        // Create parent directories for the class file if they don't exist
        if let Some(parent) = class_file.parent() {
            fs::create_dir_all(parent).expect("Failed to create output directory structure");
        }

        let needs_recompile = match (std::fs::metadata(java_file), std::fs::metadata(&class_file)) {
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
                println!("Class file missing, needs compilation");
                true
            }
            _ => {
                println!("Error reading file metadata, defaulting to rebuild");
                true
            }
        };

        if needs_recompile {
            to_compile.push(java_file);
        }
    }

    if to_compile.is_empty() {
        println!("✓ Nothing to compile (incremental build up-to-date).");
        return;
    }

    // Collect all JARs in .rgradle/cache for classpath
    let cache_dir = ".rgradle/cache";
    let sep = if cfg!(windows) { ";" } else { ":" };
    let classpath = std::fs::read_dir(cache_dir)
        .ok()
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| p.extension().map_or(false, |ext| ext == "jar"))
                .map(|p| p.to_string_lossy().to_string())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
        .join(sep);

    let mut cmd = Command::new("javac");
    cmd.arg("-d").arg(&target_dir);
    if !classpath.is_empty() {
        cmd.arg("-cp").arg(classpath);
    }
    cmd.args(&to_compile);

    let status = cmd.status().expect("Failed to run javac");

    if status.success() {
        println!(
            "✓ Build successful ({} file(s) compiled).",
            to_compile.len()
        );
    } else {
        eprintln!("✗ Build failed.");
    }
}
