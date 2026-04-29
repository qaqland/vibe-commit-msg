use rig::completion::ToolDefinition;
use rig::tool::Tool;
use rig::tool::ToolError;
use serde::{Deserialize, Serialize};

use super::{paginate_output, tool_bail};

const MAX_BYTES: usize = 50 * 1024;
const DEFAULT_LIMIT: usize = 20;
const MAX_LIMIT: usize = 100;

pub struct Log;

#[derive(Debug, Deserialize, Serialize)]
pub struct LogArgs {
    pub author: Option<String>,
    pub path: Option<String>,
    pub offset: Option<usize>,
    pub limit: Option<usize>,
}

impl Tool for Log {
    const NAME: &'static str = "log";

    type Args = LogArgs;
    type Output = String;
    type Error = ToolError;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: super::tool_description!("log").to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "author": {
                        "type": "string",
                        "description": "Filter commits by author name or email (regex pattern)"
                    },
                    "path": {
                        "type": "string",
                        "description": "Filter commits that touched files under this path. Paths start from '/'."
                    },
                    "offset": {
                        "type": "integer",
                        "description": "1-indexed offset to skip commits before starting (default 1)"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Maximum number of commits to return (default 20, max 100)"
                    }
                },
                "required": []
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        if let Some(ref path) = args.path {
            if !path.starts_with('/') {
                tool_bail!("path must start with '/': {}", path);
            }
        }

        let offset = args.offset.unwrap_or(1);
        if offset == 0 {
            tool_bail!("Offset must be greater than or equal to 1");
        }
        let skip = offset.saturating_sub(1);

        let limit = args.limit.unwrap_or(DEFAULT_LIMIT);
        if limit < 1 || limit > MAX_LIMIT {
            tool_bail!("limit must be between 1 and {}", MAX_LIMIT);
        }

        eprint!("{} ", Self::NAME);

        let author = args.author.as_deref().unwrap_or(".");
        let skip_str = skip.to_string();
        let limit_str = limit.to_string();
        let path_arg = match args.path.as_deref() {
            Some("/") | None => ".",
            Some(p) => p.trim_start_matches('/'),
        };

        let git_args: Vec<&str> = vec![
            "log",
            "--date=short",
            "--format=%H%n%an <%ae>%n%ad%n%s%n%b%x00",
            "--author",
            author,
            "--skip",
            &skip_str,
            "-n",
            &limit_str,
            "--",
            path_arg,
        ];

        let output = super::run_git(git_args);

        if !output.status.success() {
            tool_bail!("{}", output.stderr);
        }

        if output.stdout.is_empty() {
            return Ok("[No commits found]".to_string());
        }

        let mut entries: Vec<String> = Vec::new();
        for raw in output.stdout.split('\0').filter(|s| !s.is_empty()) {
            let mut lines = raw.lines();
            let hash = lines.next().unwrap_or("");
            let author = lines.next().unwrap_or("");
            let date = lines.next().unwrap_or("");
            let subject = lines.next().unwrap_or("");
            let body = lines.collect::<Vec<_>>().join("\n");

            let mut entry = format!("{}  {}  {}", hash, author, date);
            entry.push('\n');
            entry.push_str(subject);
            if !body.is_empty() {
                entry.push('\n');
                entry.push_str(&body);
            }
            entries.push(entry);
        }

        if entries.is_empty() {
            return Ok("[No commits found]".to_string());
        }

        let res = paginate_output(entries, 1, usize::MAX, MAX_BYTES)?;
        let shown = res.items.len();
        let mut out = res.items;
        if res.more {
            let next_offset = offset + shown;
            out.push(format!(
                "[Output capped at {} bytes; showing {}-{} of {}. Use offset={} to continue]",
                MAX_BYTES,
                offset,
                offset + shown - 1,
                res.total,
                next_offset
            ));
        }

        Ok(out.join("\n\n---\n\n"))
    }
}
