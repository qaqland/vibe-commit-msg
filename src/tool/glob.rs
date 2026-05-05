use rig::completion::ToolDefinition;
use rig::tool::Tool;
use rig::tool::ToolError;
use serde::{Deserialize, Serialize};

use super::{paginate_output, tool_bail};

const RESULT_LIMIT: usize = 100;
const MAX_BYTES: usize = 50 * 1024;

pub struct Glob;

#[derive(Debug, Deserialize, Serialize)]
pub struct GlobArgs {
    pub pattern: String,
}

impl Tool for Glob {
    const NAME: &'static str = "glob";

    type Args = GlobArgs;
    type Output = String;
    type Error = ToolError;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: super::tool_description!("glob").to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "required": ["pattern"],
                "properties": {
                    "pattern": {
                        "type": "string",
                        "description": "The glob pattern to match files against",
                    },
                },
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        if args.pattern.is_empty() {
            tool_bail!("pattern is required");
        }

        let git_args: Vec<&str> = vec![
            "--glob-pathspecs",
            "ls-files",
            "--cached",
            "--full-name",
            "--format",
            "/%(path)",
            "--",
            args.pattern.trim_start_matches('/'),
        ];

        let output = super::run_git(git_args);

        if !output.status.success() {
            tool_bail!("{}", output.stderr);
        }

        let mut files: Vec<String> = output
            .stdout
            .lines()
            .filter(|line| !line.is_empty())
            .map(|s| s.to_string())
            .collect();

        if files.is_empty() {
            return Ok("[No files found]".to_string());
        }

        files.sort();

        let res = paginate_output(files, 1, RESULT_LIMIT, MAX_BYTES)?;
        let mut out = res.items;
        if res.more {
            let reason = if res.cut_by_bytes {
                format!("Output capped at {} bytes", MAX_BYTES)
            } else {
                format!("Output capped at {} entries", RESULT_LIMIT)
            };
            out.push(format!(
                "[{}. Consider using a more specific pattern]",
                reason
            ));
        }

        Ok(out.join("\n"))
    }
}
