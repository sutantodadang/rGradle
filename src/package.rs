use crate::config::{Config, SourceSet};
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use walkdir::WalkDir;

fn copy_resources(source_set: &SourceSet, target_dir: &Path) -> io::Result<()> {
    if let Some(resource_dirs) = &source_set.resources {
        for resource_dir in resource_dirs {
            if !Path::new(resource_dir).exists() {
                continue;
            }
            for entry in WalkDir::new(resource_dir).into_iter().filter_map(|e| e.ok()) {
                if entry.path().is_file() {
                    let rel_path = entry.path().strip_prefix(resource_dir).unwrap();
                    let target = target_dir.join(rel_path);
                    if let Some(parent) = target.parent() {
                        fs::create_dir_all(parent)?;
                    }
                    fs::copy(entry.path(), target)?;
                }
            }
        }
    }
    Ok(())
}

pub fn package_project(config: &Config, uber: bool) -> io::Result<()> {
    // First, ensure the project is built
    if !crate::build::build_project(config) {
        return Err(io::Error::new(io::ErrorKind::Other, "Build failed"));
    }

    let jar_name = format!("{}-{}.jar", config.project.name, config.project.version);
    let temp_dir = PathBuf::from("build").join("temp_jar");
    fs::create_dir_all(&temp_dir)?;

    // Copy main classes and resources
    if let Some(main) = &config.main {
        let main_output = main.output.as_deref().unwrap_or("build/classes/java/main");
        for entry in WalkDir::new(main_output).into_iter().filter_map(|e| e.ok()) {
            if entry.path().is_file() {
                let rel_path = entry.path().strip_prefix(main_output).unwrap();
                let target = temp_dir.join(rel_path);
                if let Some(parent) = target.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::copy(entry.path(), target)?;
            }
        }
        copy_resources(main, &temp_dir)?;
    }

    // Create META-INF/MANIFEST.MF
    let manifest_dir = temp_dir.join("META-INF");
    fs::create_dir_all(&manifest_dir)?;
    let manifest_path = manifest_dir.join("MANIFEST.MF");

    let mut manifest = File::create(&manifest_path)?;
    writeln!(manifest, "Manifest-Version: 1.0")?;
    writeln!(manifest, "Main-Class: {}", config.project.main_class)?;
    writeln!(manifest)?;  // Required empty line at end of manifest

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
                    fs::copy(e.path(), &target)?;
                    Ok(format!("lib/{}", jar_name))
                })
                .collect::<io::Result<_>>()?
        } else {
            vec![]
        };

        // Write Class-Path to manifest if we have dependencies
        if !deps.is_empty() {
            write!(manifest, "Class-Path:")?;
            for (i, dep) in deps.iter().enumerate() {
                if i > 0 && i % 3 == 0 {
                    // Start a new continuation line every 3 entries
                    writeln!(manifest)?;
                    write!(manifest, " ")?;
                } else if i > 0 {
                    write!(manifest, " ")?;
                }
                write!(manifest, " {}", dep)?;
            }
            writeln!(manifest)?;
        }
    }

    // Create the JAR
    println!("Creating JAR: {}", jar_name);
    let mut cmd = Command::new("jar");
    cmd.current_dir(&temp_dir)
        .arg("cfm")
        .arg(&jar_name)
        .arg("META-INF/MANIFEST.MF")
        .arg(".");

    let status = cmd.status()?;
    if !status.success() {
        return Err(io::Error::new(io::ErrorKind::Other, "jar command failed"));
    }

    // Move JAR to project root and clean up
    let source = temp_dir.join(&jar_name);
    let target = Path::new(&jar_name);
    if target.exists() {
        fs::remove_file(target)?;
    }
    fs::rename(source, target)?;
    fs::remove_dir_all(&temp_dir)?;

    println!("✓ Created {}", jar_name);
    Ok(())
}
