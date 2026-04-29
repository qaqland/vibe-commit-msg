use rig::completion::ToolDefinition;
use rig::tool::Tool;
use rig::tool::ToolError;
use serde::{Deserialize, Serialize};

use super::{paginate_output, tool_bail};

const MAX_BYTES: usize = 50 * 1024;

pub struct Diff;

#[derive(Debug, Deserialize, Serialize)]
pub struct DiffArgs {
    pub path: Option<String>,
}

impl Tool for Diff {
    const NAME: &'static str = "diff";

    type Args = DiffArgs;
    type Output = String;
    type Error = ToolError;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: super::tool_description!("diff").to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "The repository-absolute path to a file to show the diff for. If omitted, returns diffs for all changed files."
                    },
                },
                "required": [],
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        if let Some(ref path) = args.path {
            if !path.starts_with('/') {
                tool_bail!("path must start with '/': {}", path);
            }
        }

        eprint!("{} ", Self::NAME);

        let pathspec = args
            .path
            .as_deref()
            .map(|p| p.trim_start_matches('/'))
            .unwrap_or(".");

        let output = super::run_git(["diff", "--cached", "--no-color", "--", pathspec]);

        if !output.status.success() {
            tool_bail!("{}", output.stderr);
        }

        if output.stdout.is_empty() {
            return Ok("[No changes found]".to_string());
        }

        let lines: Vec<String> = output.stdout.lines().map(|s| s.to_string()).collect();
        let res = paginate_output(lines, 1, usize::MAX, MAX_BYTES)?;
        let mut out = res.items;
        if res.more {
            out.push(format!("[Output capped at {} bytes]", MAX_BYTES));
        }

        Ok(out.join("\n"))
    }
}
