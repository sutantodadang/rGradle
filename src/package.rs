use crate::config::Config;
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use walkdir::WalkDir;

pub fn package_project(config: &Config, uber: bool) -> io::Result<()> {
    // First, ensure the project is built
    crate::build::build_project(config);

    let output_dir = config.project.output_dir.as_deref().unwrap_or("build");
    let jar_name = format!("{}-{}.jar", config.project.name, config.project.version);

    // Create a temporary directory for packaging
    let temp_dir = Path::new(output_dir).join("temp_jar");
    fs::create_dir_all(&temp_dir)?;

    // Copy class files to temp directory, preserving package structure
    let class_dir = Path::new(output_dir);
    for entry in WalkDir::new(class_dir).into_iter().filter_map(|e| e.ok()) {
        if entry.path().extension().map_or(false, |ext| ext == "class") {
            let rel_path = entry.path().strip_prefix(class_dir).unwrap();
            let target = temp_dir.join(rel_path);
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(entry.path(), target)?;
        }
    }

    // Create META-INF/MANIFEST.MF in temp directory
    let manifest_dir = temp_dir.join("META-INF");
    fs::create_dir_all(&manifest_dir)?;
    let manifest_path = manifest_dir.join("MANIFEST.MF");

    let mut manifest = File::create(&manifest_path)?;
    writeln!(manifest, "Manifest-Version: 1.0")?;
    writeln!(manifest, "Main-Class: {}", config.project.main_class)?;

    if uber {
        // Create lib directory for dependencies
        let lib_dir = temp_dir.join("lib");
        fs::create_dir_all(&lib_dir)?;

        // Copy all dependency JARs to lib/
        let deps: Vec<String> = if let Ok(entries) = fs::read_dir(".rgradle/cache") {
            entries
                .filter_map(|e| e.ok())
                .filter(|e| e.path().extension().map_or(false, |ext| ext == "jar"))
                .map(|e| {
                    let jar_name = e.path().file_name().unwrap().to_string_lossy().to_string();
                    let target = lib_dir.join(&jar_name);
                    fs::copy(e.path(), &target).unwrap_or_else(|_| 0);
                    format!("lib/{}", jar_name)
                })
                .collect()
        } else {
            vec![]
        };

        if !deps.is_empty() {
            // Write Class-Path with continuation lines (max 72 chars per line)
            writeln!(manifest, "Class-Path: {}", deps[0])?;
            let mut current_line = String::new();
            for dep in deps.iter().skip(1) {
                if current_line.len() + dep.len() + 1 > 70 {
                    // 70 to account for space
                    writeln!(manifest, " {}", current_line.trim())?;
                    current_line.clear();
                }
                if !current_line.is_empty() {
                    current_line.push(' ');
                }
                current_line.push_str(dep);
            }
            if !current_line.is_empty() {
                writeln!(manifest, " {}", current_line.trim())?;
            }
        }
    }
    writeln!(manifest)?; // Empty line at end of manifest

    // Create the JAR from the temp directory
    let mut cmd = Command::new("jar");
    cmd.current_dir(&temp_dir) // Run from temp directory
        .arg("cfm") // Create JAR with manifest
        .arg(&jar_name) // Output JAR name
        .arg("META-INF/MANIFEST.MF") // Manifest file
        .arg("."); // All contents of current directory

    println!("Creating JAR: {}", jar_name);
    let status = cmd.status()?;

    if status.success() {
        // Move JAR to project root
        let source = temp_dir.join(&jar_name);
        let target = Path::new(&jar_name);
        if source.exists() {
            if target.exists() {
                fs::remove_file(target)?;
            }
            fs::rename(source, target)?;
            // Clean up temp directory
            fs::remove_dir_all(temp_dir)?;
            println!("✓ Created {}", jar_name);
            Ok(())
        } else {
            Err(io::Error::new(io::ErrorKind::Other, "JAR file not created"))
        }
    } else {
        Err(io::Error::new(io::ErrorKind::Other, "jar command failed"))
    }
}
