use rig::completion::ToolDefinition;
use rig::tool::Tool;
use rig::tool::ToolError;
use serde::{Deserialize, Serialize};

use super::{paginate_output, tool_bail};

const RESULT_LIMIT: usize = 100;
const MAX_BYTES: usize = 50 * 1024;

pub struct Grep;

#[derive(Debug, Deserialize, Serialize)]
pub struct GrepArgs {
    pub pattern: String,
    pub path: Option<String>,
}

struct Match {
    filepath: String,
    line: usize,
    text: String,
}

fn parse_grep_line(line: &str) -> Option<Match> {
    let (_tree, rest) = line.split_once(':')?;
    let mut parts = rest.splitn(3, '\0');
    let filepath = parts.next()?.to_string();
    let line = parts.next()?.parse::<usize>().ok()?;
    let text = parts.next()?.to_string();
    if line == 0 {
        return None;
    }
    Some(Match {
        filepath,
        line,
        text,
    })
}

impl Tool for Grep {
    const NAME: &'static str = "grep";
    type Args = GrepArgs;
    type Output = String;
    type Error = ToolError;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: super::tool_description!("grep").to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "required": ["pattern"],
                "properties": {
                    "pattern": {
                        "type": "string",
                        "description": "The regex pattern to search for in file contents",
                    },
                    "path": {
                        "type": "string",
                        "description": "The directory to search in. Defaults to the repository root. Paths start from '/', where '/' refers to the git repository root.",
                    },
                },
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        if args.pattern.is_empty() {
            tool_bail!("pattern is required");
        }

        let tree = super::STAGED_HASH.get().expect("git staged-tree");

        let pathspec = args
            .path
            .as_deref()
            .map(|p| p.trim_start_matches('/'))
            .filter(|p| !p.is_empty())
            .unwrap_or(".")
            .to_string();

        let git_args: Vec<&str> = vec![
            "grep",
            "--threads",
            "1",
            "--perl-regexp",
            "--full-name",
            "--line-number",
            "--no-color",
            "--null", // NUL-separated fields for safe parsing
            "-e",
            &args.pattern,
            tree,
            "--",
            &pathspec,
        ];

        let output = super::run_git(git_args);

        match output.status.code() {
            Some(0) => {}
            Some(1) => {
                return Ok("[No matches found]".to_string());
            }
            _ => {
                tool_bail!("{}", output.stderr);
            }
        }

        let matches: Vec<_> = output.stdout.lines().filter_map(parse_grep_line).collect();
        if matches.is_empty() {
            return Ok("[No matches found]".to_string());
        }

        let mut lines: Vec<String> = Vec::new();
        let mut prev_file: Option<&str> = None;

        for m in &matches {
            if prev_file != Some(m.filepath.as_str()) {
                if prev_file.is_some() {
                    lines.push(String::new());
                }
                lines.push(format!("/{}", m.filepath));
                prev_file = Some(&m.filepath);
            }
            lines.push(format!("{}:{}", m.line, m.text));
        }

        let res = paginate_output(lines, 1, RESULT_LIMIT, MAX_BYTES)?;
        let mut out = res.items;

        if res.more && out.last().map(|s| s.starts_with('/')).unwrap_or(false) {
            out.pop();
        }

        if res.more {
            let reason = if res.cut_by_bytes {
                format!("Output capped at {} bytes", MAX_BYTES)
            } else {
                format!("Output capped at {} matches", RESULT_LIMIT)
            };
            out.push(format!(
                "[{}. Consider using a more specific path or pattern]",
                reason
            ));
        }

        Ok(out.join("\n"))
    }
}
