use crate::config::Config;
use crate::pom::{PomDependency, parse_pom_model};
use futures::stream::FuturesUnordered;
use futures::stream::StreamExt;
use std::collections::{HashSet, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs as async_fs;
use tokio::io::AsyncWriteExt;
use tokio::sync::Semaphore;

/// Download file from `url` and save to `path`, unless it already exists.
async fn fetch_file_async(url: &str, path: &Path) {
    if path.exists() {
        eprintln!("✔️  Cached: {}", path.display());
        return;
    }

    let response = match reqwest::get(url).await {
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

    let bytes = match response.bytes().await {
        Ok(b) => b,
        Err(_) => {
            eprintln!("⚠️  Failed to read response body: {url}");
            return;
        }
    };

    if let Ok(mut file) = async_fs::File::create(path).await {
        let _ = file.write_all(&bytes).await;
    }
}

/// Download JAR and POM for a given dependency (group:artifact), then parse transitive dependencies (async, iterative).
async fn fetch_jar_and_pom_async(
    root_dep: String,
    root_version: String,
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

        // Debug print to trace dependency resolution
        let (group_id, artifact_id) = if let Some(idx) = dep.find(':') {
            (&dep[..idx], &dep[idx + 1..])
        } else {
            ("?", "?")
        };
        // println!(
        //     "[DEBUG] Will fetch group_id='{}', artifact_id='{}', version='{}'",
        //     group_id, artifact_id, version
        // );

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
        // println!("[DEBUG] URL: {}/{}", base_url, jar_name);

        let jar_url = format!("{base_url}/{}", jar_name);
        let pom_url = format!("{base_url}/{}", pom_name);

        let jar_path = cache_dir.join(&jar_name);
        let pom_path = cache_dir.join(&pom_name);

        println!("→ Downloading {dep}:{version}");

        // Debug prints for all URL components
        // println!("[DEBUG] base_url: {}", base_url);
        // println!("[DEBUG] jar_name: {}", jar_name);
        // println!("[DEBUG] pom_name: {}", pom_name);
        // println!("[DEBUG] jar_url: {}", jar_url);
        // println!("[DEBUG] pom_url: {}", pom_url);

        let _permit = pool.acquire().await.unwrap();
        let f1 = fetch_file_async(&jar_url, &jar_path);
        let f2 = fetch_file_async(&pom_url, &pom_path);
        futures::future::join(f1, f2).await;

        if pom_path.exists() {
            let model = parse_pom_model(pom_path.to_str().unwrap());
            // Only follow essential dependencies: skip test and optional dependencies
            // (Plugin dependencies are not handled in this tool)
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
    let (group, artifact) = if dep.contains(':') {
        let parts: Vec<&str> = dep.split(':').collect();
        if parts.len() != 2 {
            return None;
        }
        (parts[0], parts[1])
    } else {
        return None;
    };

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
    let pool = Arc::new(tokio::sync::Semaphore::new(Semaphore::MAX_PERMITS));

    let mut futs = FuturesUnordered::new();
    for (dep, version) in deps {
        let visited = visited.clone();
        let cache_dir = cache_dir.clone();
        let pool = pool.clone();
        futs.push(tokio::spawn(fetch_jar_and_pom_async(
            dep.clone(),
            version.clone(),
            cache_dir,
            visited,
            pool,
        )));
    }

    while futs.next().await.is_some() {}

    println!("✓ Dependency resolution complete.");
}
