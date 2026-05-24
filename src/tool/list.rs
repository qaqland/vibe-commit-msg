use rig::tool::Tool;
use rig::{completion::ToolDefinition, tool::ToolError};
use serde::Deserialize;

use super::{Entry, paginate_output, tool_bail};

pub fn show(args: &str) -> String {
    let Ok(parsed) = serde_json::from_str::<ListArgs>(args) else {
        return "(?)".into();
    };
    parsed.path
}

const MAX_BYTES: usize = 50 * 1024;
const MAX_ENTRIES: usize = 200;

pub struct List;

#[derive(Deserialize)]
pub struct ListArgs {
    path: String,
}

impl Tool for List {
    const NAME: &'static str = "list";

    type Error = ToolError;
    type Args = ListArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "list".to_string(),
            description: super::tool_description!("list").to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "The repository-absolute path to the directory to list"
                    }
                },
                "required": ["path"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        if !args.path.starts_with('/') {
            tool_bail!("Path must start with '/'");
        }

        let entry = super::git_find(&args.path)?;

        match entry {
            Entry::Commit { hash, .. } => {
                let mut out = Vec::new();
                out.push(hash);
                out.push("[submodule]".to_string());
                Ok(out.join("\n"))
            }
            Entry::Blob { .. } => {
                tool_bail!("Path is a file. Use read tool instead.");
            }
            Entry::Tree { path } => {
                let items = git_list(&path)?;
                if items.is_empty() {
                    return Ok("[Empty directory]".to_string());
                }
                let res = paginate_output(items, 1, MAX_ENTRIES, MAX_BYTES)?;
                let mut out = res.items;
                if res.more {
                    let reason = if res.cut_by_bytes {
                        format!("Output capped at {} bytes", MAX_BYTES)
                    } else {
                        format!("Output capped at {} entries", MAX_ENTRIES)
                    };
                    out.push(format!("[{}. Consider using glob 'pattern']", reason));
                }
                Ok(out.join("\n"))
            }
        }
    }
}

fn git_list(path: &str) -> Result<Vec<String>, ToolError> {
    let filepath = path.trim_start_matches('/');

    let tree = super::STAGED_HASH.get().expect("git staged-tree");
    let output = super::run_git([
        "ls-tree",
        "--full-tree",
        "--format",
        "%(objecttype)%x09%(objectname)%x09/%(path)",
        tree,
        "--",
        filepath,
    ]);
    if !output.status.success() {
        tool_bail!("{}", output.stderr);
    }
    let mut items: Vec<String> = output
        .stdout
        .lines()
        .filter_map(super::parse_tree_line)
        .map(Entry::path)
        .collect();
    items.sort();
    Ok(items)
}
