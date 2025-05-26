use crate::config::Config;
use crate::pom::parse_pom_model;
use futures::stream::{FuturesUnordered, StreamExt};
use num_cpus;
use std::collections::{HashSet, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs as async_fs;
use tokio::io::AsyncWriteExt;

// Function to get the optimal number of concurrent downloads
// Since downloading is I/O bound, we can use more threads than CPU cores
fn get_max_concurrent_downloads() -> usize {
    num_cpus::get() * 4
}

/// Download file from `url` and save to `path`, unless it already exists.
async fn fetch_file_async(url: &str, path: &Path, is_test: bool) {
    if path.exists() {
        eprintln!(
            "✔️  Cached: {} ({})",
            path.display(),
            if is_test { "test" } else { "main" }
        );
        return;
    }

    let mut response = match reqwest::get(url).await {
        Ok(resp) if resp.status().is_success() => resp,
        Ok(resp) if resp.status().as_u16() == 404 => {
            eprintln!("[WARN] 404 Not Found, skipping: {url}");
            return;
        }
        _ => {
            eprintln!("⚠️  Failed to fetch: {url}");
            return;
        }
    };

    let total_size = response.content_length();
    let mut file = match async_fs::File::create(path).await {
        Ok(f) => f,
        Err(_) => {
            eprintln!("⚠️  Failed to create file: {}", path.display());
            return;
        }
    };
    let mut downloaded: u64 = 0;
    use std::cmp::min;
    use std::io::Write;
    use std::io::stdout;

    while let Ok(Some(chunk)) = response.chunk().await {
        if let Err(_) = file.write_all(&chunk).await {
            eprintln!("⚠️  Failed to write to file: {}", path.display());
            return;
        }
        downloaded += chunk.len() as u64;
        if let Some(total) = total_size {
            let percent = min(100, (downloaded * 100 / total) as u64);
            print!(
                "\rDownloading: {} [{:3}%] ({})",
                path.file_name().unwrap().to_string_lossy(),
                percent,
                if is_test { "test" } else { "main" }
            );
            let _ = stdout().flush();
        }
    }
    if total_size.is_some() {
        println!(
            "\rDownloading: {} [100%] ({})",
            path.file_name().unwrap().to_string_lossy(),
            if is_test { "test" } else { "main" }
        );
    }
}

/// Download JAR and POM for a given dependency (group:artifact), then parse transitive dependencies (async, iterative).
async fn fetch_jar_and_pom_async(
    root_dep: String,
    root_version: String,
    is_test: bool,
    cache_dir: PathBuf,
    visited: Arc<tokio::sync::Mutex<HashSet<String>>>,
    pool: Arc<tokio::sync::Semaphore>,
) {
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

        let (group_id, artifact_id) = if let Some(idx) = dep.find(':') {
            (&dep[..idx], &dep[idx + 1..])
        } else {
            ("?", "?")
        };

        let (base_url, jar_name, pom_name) = match dep_to_url(&dep, &version) {
            Some(t) => t,
            None => {
                eprintln!(
                    "[DEBUG] Invalid dep_to_url for: {}:{}:{}",
                    group_id, artifact_id, version
                );
                continue;
            }
        };

        let jar_url = format!("{base_url}/{}", jar_name);
        let pom_url = format!("{base_url}/{}", pom_name);

        let jar_path = cache_dir.join(&jar_name);
        let pom_path = cache_dir.join(&pom_name);

        println!(
            "→ Downloading {}:{} ({})",
            dep,
            version,
            if is_test { "test" } else { "main" }
        );

        let _permit = pool.acquire().await.unwrap();
        let f1 = fetch_file_async(&jar_url, &jar_path, is_test);
        let f2 = fetch_file_async(&pom_url, &pom_path, is_test);
        futures::future::join(f1, f2).await;

        if pom_path.exists() {
            let model = parse_pom_model(pom_path.to_str().unwrap());
            let deps: Vec<_> = model
                .dependencies
                .into_iter()
                .filter(|dep| dep.scope.as_deref() != Some("test") && !dep.optional)
                .map(|dep| {
                    (
                        dep.group_id.clone(),
                        dep.artifact_id.clone(),
                        dep.version.clone(),
                    )
                })
                .collect();
            for (group_id, artifact_id, version) in deps {
                let sub = format!("{}:{}", group_id, artifact_id);
                stack.push_back((sub, version));
            }
        }
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

/// Entry point: fetches dependencies listed in config (async, parallel).
pub async fn fetch_dependencies(config: &Config) {
    let cache_dir = Path::new(".rgradle/cache/").to_path_buf();
    fs::create_dir_all(&cache_dir).expect("Failed to create cache dir");

    let visited = Arc::new(tokio::sync::Mutex::new(HashSet::new()));
    let pool = Arc::new(tokio::sync::Semaphore::new(get_max_concurrent_downloads())); // max 8 concurrent downloads

    let mut futs = FuturesUnordered::new();

    // Fetch main dependencies
    if let Some(deps) = &config.dependencies {
        for (dep, version) in deps {
            let visited = visited.clone();
            let cache_dir = cache_dir.clone();
            let pool = pool.clone();
            futs.push(tokio::spawn(fetch_jar_and_pom_async(
                dep.clone(),
                version.clone(),
                false,
                cache_dir,
                visited,
                pool,
            )));
        }
    }

    // Fetch test dependencies
    if let Some(test_deps) = &config.test_dependencies {
        println!("Fetching test dependencies...");
        for (dep, version) in test_deps {
            let visited = visited.clone();
            let cache_dir = cache_dir.clone();
            let pool = pool.clone();
            futs.push(tokio::spawn(fetch_jar_and_pom_async(
                dep.clone(),
                version.clone(),
                true,
                cache_dir,
                visited,
                pool,
            )));
        }
    }

    while futs.next().await.is_some() {}

    println!("✓ Dependency resolution complete.");
}
