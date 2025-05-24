use crate::config::Config;
use crate::pom::parse_pom_model;
use futures::stream::{FuturesUnordered, StreamExt};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::collections::{HashSet, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs as async_fs;
use tokio::io::AsyncWriteExt;
use num_cpus;

// Function to get the optimal number of concurrent downloads
// Since downloading is I/O bound, we can use more threads than CPU cores
fn get_max_concurrent_downloads() -> usize {
    num_cpus::get() * 4
}

#[derive(Debug)]
struct DownloadError {
    url: String,
    error: String,
}

async fn fetch_file_async(url: &str, path: &Path, pb: ProgressBar) -> Result<(), DownloadError> {
    if path.exists() {
        pb.set_style(ProgressStyle::default_bar()
            .template("{msg}")
            .unwrap());
        pb.set_message(format!("✔️  Cached: {}", path.display()));
        pb.abandon_with_message(format!("✔️  Cached: {}", path.display()));
        return Ok(());
    }

    let response = reqwest::get(url).await.map_err(|e| DownloadError {
        url: url.to_string(),
        error: e.to_string(),
    })?;

    if response.status().is_success() {
        let total_size = response.content_length().unwrap_or(0);
        pb.set_length(total_size);
        pb.set_style(ProgressStyle::default_bar()
            .template("{msg}\n{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({eta})")
            .unwrap()
            .progress_chars("#>-"));
        
        let mut file = async_fs::File::create(path).await.map_err(|e| DownloadError {
            url: url.to_string(),
            error: format!("Failed to create file: {}", e),
        })?;

        let bytes = response.bytes().await.map_err(|e| DownloadError {
            url: url.to_string(),
            error: format!("Download error: {}", e),
        })?;
        
        file.write_all(&bytes).await.map_err(|e| DownloadError {
            url: url.to_string(),
            error: format!("Write error: {}", e),
        })?;
        
        pb.set_position(bytes.len() as u64);
        pb.finish_with_message(format!("✔️  Downloaded: {}", path.display()));
        Ok(())
    } else if response.status().as_u16() == 404 {
        pb.finish_with_message(format!("⚠️  Not found: {url}"));
        Err(DownloadError {
            url: url.to_string(),
            error: "404 Not Found".to_string(),
        })
    } else {
        pb.finish_with_message(format!("❌ Failed: {url}"));
        Err(DownloadError {
            url: url.to_string(),
            error: format!("HTTP {}", response.status()),
        })
    }
}

async fn fetch_jar_and_pom_async(
    root_dep: String,
    root_version: String,
    cache_dir: PathBuf,
    visited: Arc<tokio::sync::Mutex<HashSet<String>>>,
    pool: Arc<tokio::sync::Semaphore>,
    multi_progress: Arc<MultiProgress>,
) -> Result<(), Vec<DownloadError>> {
    let mut errors = Vec::new();
    let mut stack = VecDeque::new();
    stack.push_back((root_dep, root_version));

    while let Some((dep, version)) = stack.pop_front() {
        let key = format!("{dep}:{version}");
        let already_fetched = {
            let mut v = visited.lock().await;
            !v.insert(key.clone())
        };
        if already_fetched {
            continue;
        }

        let (base_url, jar_name, pom_name) = match dep_to_url(&dep, &version) {
            Some(t) => t,
            None => {
                errors.push(DownloadError {
                    url: format!("{}:{}", dep, version),
                    error: "Invalid dependency format".to_string(),
                });
                continue;
            }
        };

        let jar_url = format!("{base_url}/{}", jar_name);
        let pom_url = format!("{base_url}/{}", pom_name);
        let jar_path = cache_dir.join(&jar_name);
        let pom_path = cache_dir.join(&pom_name);

        // Create parent directories if they don't exist
        if let Some(parent) = jar_path.parent() {
            if let Err(e) = fs::create_dir_all(parent) {
                errors.push(DownloadError {
                    url: jar_url.clone(),
                    error: format!("Failed to create directories: {}", e),
                });
                continue;
            }
        }

        let _permit = pool.acquire().await.unwrap();
        let pb_jar = multi_progress.add(ProgressBar::new(0));
        let pb_pom = multi_progress.add(ProgressBar::new(0));
        
        pb_jar.set_message(format!("JAR: {}", jar_name));
        pb_pom.set_message(format!("POM: {}", pom_name));

        let (jar_result, pom_result) = futures::join!(
            fetch_file_async(&jar_url, &jar_path, pb_jar.clone()),
            fetch_file_async(&pom_url, &pom_path, pb_pom.clone())
        );

        if let Err(e) = jar_result {
            errors.push(e);
        }

        if let Err(e) = pom_result {
            errors.push(e);
        } else if pom_path.exists() {
            let model = parse_pom_model(pom_path.to_str().unwrap());
            // Only follow essential dependencies
            let deps: Vec<_> = model.dependencies
                .into_iter()
                .filter(|dep| dep.scope.as_deref() != Some("test") && !dep.optional)
                .map(|dep| (
                    format!("{}:{}", dep.group_id, dep.artifact_id),
                    dep.version
                ))
                .collect();

            for (sub_dep, sub_version) in deps {
                stack.push_back((sub_dep, sub_version));
            }
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

// Entry point: fetches dependencies listed in config (async, parallel).
pub async fn fetch_dependencies(config: &Config) {
    let deps = match config.dependencies.as_ref() {
        Some(d) if !d.is_empty() => d,
        _ => {
            println!("No dependencies to fetch.");
            return;
        }
    };

    let cache_dir = Path::new(".rgradle/cache/").to_path_buf();
    fs::create_dir_all(&cache_dir).expect("Failed to create cache dir");

    let visited = Arc::new(tokio::sync::Mutex::new(HashSet::new()));
    let pool = Arc::new(tokio::sync::Semaphore::new(get_max_concurrent_downloads()));
    let multi_progress = Arc::new(MultiProgress::new());

    let mut futs = FuturesUnordered::new();
    for (dep, version) in deps {
        let visited = visited.clone();
        let cache_dir = cache_dir.clone();
        let pool = pool.clone();
        let mp = multi_progress.clone();
        
        futs.push(tokio::spawn(fetch_jar_and_pom_async(
            dep.clone(),
            version.clone(),
            cache_dir,
            visited,
            pool,
            mp,
        )));
    }

    let mut error_count = 0;
    while let Some(result) = futs.next().await {
        if let Ok(Err(errors)) = result {
            error_count += errors.len();
            for error in errors {
                eprintln!("❌ Error fetching {}: {}", error.url, error.error);
            }
        }
    }

    if error_count > 0 {
        eprintln!("✗ Dependency resolution completed with {} errors.", error_count);
    } else {
        println!("✓ Dependency resolution complete.");
    }
}

/// Converts "group:artifact" into (base_url, jar_name, pom_name)
fn dep_to_url(dep: &str, version: &str) -> Option<(String, String, String)> {
    let parts: Vec<&str> = dep.split(':').collect();
    if parts.len() != 2 {
        return None;
    }
    let (group, artifact) = (parts[0], parts[1]);

    let path = group.replace('.', "/");
    let jar_name = format!("{artifact}-{version}.jar");
    let pom_name = format!("{artifact}-{version}.pom");
    let base_url = format!(
        "https://repo1.maven.org/maven2/{}/{}/{}",
        path, artifact, version
    );

    Some((base_url, jar_name, pom_name))
}
