use crate::config::Config;
use std::process::Command;

pub fn test_project(config: &Config) {
    let test_dir = config
        .project
        .test_dir
        .clone()
        .unwrap_or_else(|| "example/test/java".to_string());
    let test_output_dir = config
        .project
        .test_output_dir
        .clone()
        .unwrap_or_else(|| "build/test-classes".to_string());
    let main_output_dir = config
        .project
        .output_dir
        .clone()
        .unwrap_or_else(|| "build".to_string());

    // Create test output directory
    std::fs::create_dir_all(&test_output_dir).expect("Failed to create test output directory");

    // Find all test files
    let test_files = walkdir::WalkDir::new(&test_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().to_string_lossy().ends_with("Test.java"))
        .map(|e| e.path().to_owned())
        .collect::<Vec<_>>();

    if test_files.is_empty() {
        println!("No test files found.");
        return;
    }

    println!("Compiling tests...");

    // Collect all JARs (including JUnit) in .rgradle/cache for test classpath
    let cache_dir = ".rgradle/cache";
    let sep = if cfg!(windows) { ";" } else { ":" };
    let jar_classpath = std::fs::read_dir(cache_dir)
        .ok()
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| p.extension().map_or(false, |ext| ext == "jar"))
                .map(|p| p.to_string_lossy().to_string())
                .collect::<Vec<_>>()
        })
        .filter(|v| !v.is_empty())
        .map(|v| v.join(sep))
        .unwrap_or_default();

    // Compile tests with main classes in classpath
    let mut cmd = Command::new("javac");
    cmd.arg("-d").arg(&test_output_dir);

    // Include both test dependencies and main classes in classpath
    let compile_classpath = format!("{}{}{}", main_output_dir, sep, jar_classpath);

    cmd.arg("-cp").arg(&compile_classpath);
    cmd.args(&test_files);

    let status = cmd.status().expect("Failed to compile tests");

    if !status.success() {
        eprintln!("✗ Test compilation failed.");
        return;
    }

    println!("Running tests...");

    // Run tests with JUnit
    let mut cmd = Command::new("java");

    // Build complete classpath for test execution
    let test_classpath = format!(
        "{}{}{}{}{}",
        test_output_dir, sep, main_output_dir, sep, jar_classpath
    );

    cmd.arg("-cp")
        .arg(&test_classpath)
        .arg("org.junit.runner.JUnitCore");

    // Add test class names with full package names
    let test_classes = test_files
        .iter()
        .filter_map(|path| {
            let rel_path = path.strip_prefix(&test_dir).ok()?;
            let class_name = rel_path
                .with_extension("")
                .to_string_lossy()
                .replace('\\', ".")
                .replace('/', ".");

            println!("Adding test class: {}", class_name);
            Some(class_name)
        })
        .collect::<Vec<_>>();

    println!("Running JUnit with classpath: {}", test_classpath);
    println!("Test classes: {:?}", test_classes);

    let status = cmd.status().expect("Failed to run tests");

    if status.success() {
        println!("✓ All tests passed.");
    } else {
        eprintln!("✗ Some tests failed.");
    }
}
