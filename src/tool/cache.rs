use rig::completion::ToolDefinition;
use rig::tool::Tool;
use rig::tool::ToolError;
use serde::{Deserialize, Serialize};

use super::tool_bail;

pub fn show(args: &str) -> Option<String> {
    let parsed = serde_json::from_str::<CacheArgs>(args).ok()?;
    Some(match (parsed.path.as_deref(), parsed.content.as_deref()) {
        (None, None) => "list".into(),
        (Some(path), None) => format!("load {}", path),
        (Some(path), Some(_)) => format!("save {}", path),
        (None, Some(_)) => "save ?".into(),
    })
}

const MAX_FILE_SIZE: usize = 8 * 1024;

pub const STALENESS_SECS: u64 = 7 * 24 * 60 * 60;

pub struct Cache;

#[derive(Debug, Deserialize, Serialize)]
pub struct CacheArgs {
    pub path: Option<String>,
    pub content: Option<String>,
}

fn valid_filename(path: &str) -> bool {
    if path.contains("..") || path.contains('/') || path.is_empty() {
        return false;
    }
    path.chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.')
}

fn extract_front_matter(content: &str) -> Option<&str> {
    let rest = content.strip_prefix("---")?;
    rest.find("\n---").map(|i| &rest[..i])
}

fn parse_front_matter(content: &str) -> Vec<(String, String)> {
    let yaml_str = match extract_front_matter(content) {
        Some(s) => s,
        None => return vec![],
    };
    let value: serde_yaml::Value = match serde_yaml::from_str(yaml_str) {
        Ok(v) => v,
        Err(_) => return vec![],
    };
    let mapping = match value {
        serde_yaml::Value::Mapping(m) => m,
        _ => return vec![],
    };
    let mut result = Vec::new();
    for (k, v) in &mapping {
        let k_str = match k.as_str() {
            Some(s) => s.to_string(),
            None => continue,
        };
        let v_str = match v {
            serde_yaml::Value::String(s) => s.clone(),
            serde_yaml::Value::Number(n) => n.to_string(),
            serde_yaml::Value::Bool(b) => b.to_string(),
            _ => continue,
        };
        result.push((k_str, v_str));
    }
    result
}

fn validate_front_matter(content: &str) -> Result<(), String> {
    let yaml_str = extract_front_matter(content)
        .ok_or_else(|| "File must start with --- ... --- YAML front matter".to_string())?;
    serde_yaml::from_str::<serde_yaml::Value>(yaml_str)
        .map_err(|e| format!("Front matter YAML error: {}", e))?;
    Ok(())
}

impl Tool for Cache {
    const NAME: &'static str = "cache";

    type Args = CacheArgs;
    type Output = String;
    type Error = ToolError;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: super::tool_description!("cache").to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Cache file name (e.g. 'project.md'). If omitted, lists all cache files with their front matter. If provided without content, reads the file. If provided with content, writes the file."
                    },
                    "content": {
                        "type": "string",
                        "description": "Full markdown content to write (with front matter). Only valid when path is also provided."
                    }
                },
                "required": []
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let dir = super::cache_dir();

        match (args.path.as_deref(), args.content.as_deref()) {
            (None, None) => {
                let _ = std::fs::create_dir_all(&dir);

                let names = match std::fs::read_dir(&dir) {
                    Ok(read) => {
                        let mut names: Vec<String> = read
                            .filter_map(|e| e.ok())
                            .filter_map(|e| {
                                let name = e.file_name().to_string_lossy().into_owned();
                                if name.ends_with(".md") {
                                    Some(name)
                                } else {
                                    None
                                }
                            })
                            .collect();
                        names.sort();
                        names
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                        return Ok("[No cache files]".to_string());
                    }
                    Err(e) => {
                        tool_bail!("Failed to list cache directory: {}", e);
                    }
                };

                if names.is_empty() {
                    return Ok("[No cache files]".to_string());
                }

                let mut out = Vec::new();
                for name in &names {
                    let filepath = dir.join(name);
                    let content = match std::fs::read_to_string(&filepath) {
                        Ok(c) => c,
                        Err(e) => {
                            out.push(format!("{}\n  [read error: {}]", name, e));
                            continue;
                        }
                    };
                    let fm = parse_front_matter(&content);
                    if fm.is_empty() {
                        out.push(format!("{}\n  (no front matter)", name));
                    } else {
                        out.push(name.clone());
                        for (k, v) in fm {
                            out.push(format!("  {}: {}", k, v));
                        }
                    }
                }
                Ok(out.join("\n"))
            }

            (Some(path), None) => {
                if !valid_filename(path) {
                    tool_bail!("Invalid file name: {}", path);
                }
                let filepath = dir.join(path);
                match std::fs::read_to_string(&filepath) {
                    Ok(content) => Ok(content),
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                        Ok("[File not found]".to_string())
                    }
                    Err(e) => {
                        tool_bail!("Failed to read {}: {}", path, e);
                    }
                }
            }

            (Some(path), Some(content)) => {
                if !valid_filename(path) {
                    tool_bail!("Invalid file name: {}", path);
                }
                if content.len() > MAX_FILE_SIZE {
                    tool_bail!(
                        "Content too large ({} bytes, max {})",
                        content.len(),
                        MAX_FILE_SIZE
                    );
                }
                if let Err(e) = validate_front_matter(content) {
                    tool_bail!("{}", e);
                }
                let content = inject_updated_at(&content);
                if let Err(e) = std::fs::create_dir_all(&dir) {
                    tool_bail!("Failed to create cache directory: {}", e);
                }
                let filepath = dir.join(path);
                if let Err(e) = std::fs::write(&filepath, &content) {
                    tool_bail!("Failed to write {}: {}", path, e);
                }
                Ok(format!("[Written {} ({} bytes)]", path, content.len()))
            }

            (None, Some(_)) => {
                tool_bail!("content requires path");
            }
        }
    }
}

fn inject_updated_at(content: &str) -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let Some(rest) = content.strip_prefix("---") else {
        return content.to_string();
    };
    let Some(i) = rest.find("\n---") else {
        return content.to_string();
    };

    let fm = &rest[..i];
    let after = &rest[i + 4..];

    let parsed: serde_yaml::Value = match serde_yaml::from_str(fm) {
        Ok(v) => v,
        Err(_) => return content.to_string(),
    };

    let serde_yaml::Value::Mapping(mut mapping) = parsed else {
        return content.to_string();
    };

    mapping.insert(
        serde_yaml::Value::String("updated_at".into()),
        serde_yaml::Value::Number(now.into()),
    );

    let fm_out = match serde_yaml::to_string(&mapping) {
        Ok(s) => s,
        Err(_) => return content.to_string(),
    };

    let mut out = String::with_capacity(content.len());
    out.push_str("---\n");
    out.push_str(&fm_out);
    out.push_str("---");
    out.push_str(after);
    out
}

pub fn is_stale(name: &str) -> bool {
    let dir = super::cache_dir();
    let filepath = dir.join(name);

    let content = match std::fs::read_to_string(&filepath) {
        Ok(c) => c,
        Err(_) => return true,
    };

    let Some(yaml_str) = extract_front_matter(&content) else {
        return true;
    };

    let Ok(value) = serde_yaml::from_str::<serde_yaml::Value>(yaml_str) else {
        return true;
    };

    let ts = value
        .get("updated_at")
        .and_then(|v| v.as_i64())
        .or_else(|| {
            value
                .get("updated_at")
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse::<i64>().ok())
        });

    let Some(ts) = ts else {
        return true;
    };

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;

    now - ts > STALENESS_SECS as i64
}

pub fn read_cache_file(name: &str) -> Option<String> {
    let dir = super::cache_dir();
    let filepath = dir.join(name);
    std::fs::read_to_string(filepath).ok()
}
