use quick_xml::Reader;
use quick_xml::events::Event;
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;

#[derive(Debug, Clone)]
pub struct PomDependency {
    pub group_id: String,
    pub artifact_id: String,
    pub version: String,
    pub scope: Option<String>,
    pub optional: bool,
}

#[derive(Debug, Clone)]
pub struct PomModel {
    pub group_id: Option<String>,
    pub artifact_id: String,
    pub version: Option<String>,
    pub properties: HashMap<String, String>,
    pub dependencies: Vec<PomDependency>,
    pub parent: Option<ParentPom>,
}

#[derive(Debug, Clone)]
pub struct ParentPom {
    pub group_id: String,
    pub artifact_id: String,
    pub version: String,
}

/// Resolves placeholders like ${...} using a properties map
fn resolve_placeholders(s: &str, props: &HashMap<String, String>) -> String {
    let mut result = s.to_string();
    let mut changed = true;

    while changed {
        changed = false;
        let mut replaced = result.clone();
        for (k, v) in props {
            let pattern = format!("${{{}}}", k);
            if replaced.contains(&pattern) {
                replaced = replaced.replace(&pattern, v);
                changed = true;
            }
        }
        result = replaced;
    }

    result
}

pub fn parse_pom_model(path: &str) -> PomModel {
    let file = File::open(path).expect("Failed to open POM file");
    let file = BufReader::new(file);
    let mut reader = Reader::from_reader(file);
    reader.trim_text(true);

    let mut buf = Vec::new();
    let mut current_tag = String::new();

    let mut dependencies = Vec::new();
    let mut properties = HashMap::new();
    let mut parent = ParentPom {
        group_id: String::new(),
        artifact_id: String::new(),
        version: String::new(),
    };

    let mut current_dep = PomDependency {
        group_id: String::new(),
        artifact_id: String::new(),
        version: String::new(),
        scope: None,
        optional: false,
    };

    let mut artifact_id = String::new();
    let mut group_id = None;
    let mut version = None;

    let mut in_dependency = false;
    let mut in_properties = false;
    let mut in_parent = false;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let tag = String::from_utf8_lossy(e.name().as_ref()).to_string();
                match tag.as_str() {
                    "dependency" => {
                        in_dependency = true;
                        current_dep = PomDependency {
                            group_id: String::new(),
                            artifact_id: String::new(),
                            version: String::new(),
                            scope: None,
                            optional: false,
                        };
                    }
                    "properties" => in_properties = true,
                    "parent" => in_parent = true,
                    _ => current_tag = tag,
                }
            }

            Ok(Event::End(ref e)) => {
                let tag = String::from_utf8_lossy(e.name().as_ref()).to_string();
                match tag.as_str() {
                    "dependency" => {
                        if !current_dep.group_id.is_empty()
                            && !current_dep.artifact_id.is_empty()
                            && !current_dep.version.is_empty()
                        {
                            dependencies.push(current_dep.clone());
                        }
                        in_dependency = false;
                    }
                    "properties" => in_properties = false,
                    "parent" => in_parent = false,
                    _ => {}
                }
                current_tag.clear();
            }

            Ok(Event::Text(e)) => {
                let value = e.unescape().unwrap_or_default().to_string();
                if in_properties {
                    properties.insert(current_tag.clone(), value);
                } else if in_parent {
                    match current_tag.as_str() {
                        "groupId" => parent.group_id = value,
                        "artifactId" => parent.artifact_id = value,
                        "version" => parent.version = value,
                        _ => {}
                    }
                } else if in_dependency {
                    match current_tag.as_str() {
                        "groupId" => {
                            current_dep.group_id = resolve_placeholders(&value, &properties)
                        }
                        "artifactId" => current_dep.artifact_id = value,
                        "version" => {
                            current_dep.version = resolve_placeholders(&value, &properties)
                        }
                        "scope" => current_dep.scope = Some(value),
                        "optional" => current_dep.optional = value.to_lowercase() == "true",
                        _ => {}
                    }
                } else {
                    match current_tag.as_str() {
                        "artifactId" => artifact_id = value,
                        "groupId" => group_id = Some(value),
                        "version" => version = Some(value),
                        _ => {}
                    }
                }
            }

            Ok(Event::Eof) => break,
            Err(e) => {
                eprintln!("Error reading POM: {:?}", e);
                break;
            }
            _ => {}
        }

        buf.clear();
    }

    // Final fallback jika tidak didefinisikan secara eksplisit
    let resolved_group_id = group_id.clone().or_else(|| {
        if !parent.group_id.is_empty() {
            Some(parent.group_id.clone())
        } else {
            None
        }
    });

    let resolved_version = version.clone().or_else(|| {
        if !parent.version.is_empty() {
            Some(parent.version.clone())
        } else {
            None
        }
    });

    PomModel {
        group_id: resolved_group_id,
        artifact_id,
        version: resolved_version,
        properties,
        dependencies,
        parent: if parent.group_id.is_empty() {
            None
        } else {
            Some(parent)
        },
    }
}
