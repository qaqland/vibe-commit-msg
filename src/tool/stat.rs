use rig::completion::ToolDefinition;
use rig::tool::Tool;
use rig::tool::ToolError;
use serde::{Deserialize, Serialize};

use super::{paginate_output, tool_bail};

pub fn show(_args: &str) -> String {
    "staged".into()
}

const MAX_BYTES: usize = 50 * 1024;
const MAX_ENTRIES: usize = 200;

pub struct Stat;

#[derive(Debug, Deserialize, Serialize)]
pub struct StatArgs {}

impl Tool for Stat {
    const NAME: &'static str = "stat";

    type Args = StatArgs;
    type Output = String;
    type Error = ToolError;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: super::tool_description!("stat").to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        }
    }

    async fn call(&self, _args: Self::Args) -> Result<Self::Output, Self::Error> {
        let output = super::run_git(["diff", "--cached", "--numstat"]);

        if !output.status.success() {
            tool_bail!("{}", output.stderr);
        }

        let lines: Vec<String> = output
            .stdout
            .lines()
            .filter(|line| !line.is_empty())
            .map(|line| {
                let mut parts = line.splitn(3, '\t');
                let added = parts.next().unwrap_or("");
                let deleted = parts.next().unwrap_or("");
                let path = parts.next().unwrap_or("");

                let added_fmt = if added == "-" {
                    "++".to_string()
                } else {
                    format!("+{}", added)
                };
                let deleted_fmt = if deleted == "-" {
                    "--".to_string()
                } else {
                    format!("-{}", deleted)
                };

                format!("{}\t{}\t/{}", added_fmt, deleted_fmt, path)
            })
            .collect();

        if lines.is_empty() {
            return Ok("[No changes found]".to_string());
        }

        let res = paginate_output(lines, 1, MAX_ENTRIES, MAX_BYTES)?;
        let mut out = res.items;
        if res.more {
            let reason = if res.cut_by_bytes {
                format!("Output capped at {} bytes", MAX_BYTES)
            } else {
                format!("Output capped at {} entries", MAX_ENTRIES)
            };
            out.push(format!("[{}]", reason));
        }

        Ok(out.join("\n"))
    }
}
