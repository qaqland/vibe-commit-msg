use std::{
    io::Read as _,
    process::{Command, Stdio},
    thread,
};

use rig::tool::Tool;
use rig::{completion::ToolDefinition, tool::ToolError};
use serde::Deserialize;

use super::{Entry, git_find, paginate_output, tool_bail};

pub fn show(args: &str) -> String {
    let Ok(parsed) = serde_json::from_str::<ReadArgs>(args) else {
        return "(?)".into();
    };
    let mut s = parsed.path;
    let offset = parsed.offset.unwrap_or(1);
    let limit = parsed.limit.unwrap_or(DEFAULT_READ_LIMIT);
    if offset != 1 || limit != DEFAULT_READ_LIMIT {
        s.push_str(&format!(" ({}-{})", offset, offset + limit - 1));
    }
    s
}

const DEFAULT_READ_LIMIT: usize = 2000;
const MAX_BYTES: usize = 50 * 1024;

pub struct Read;

#[derive(Deserialize)]
pub struct ReadArgs {
    path: String,
    offset: Option<usize>,
    limit: Option<usize>,
}

impl Tool for Read {
    const NAME: &'static str = "read";

    type Error = ToolError;
    type Args = ReadArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "read".to_string(),
            description: super::tool_description!("read").to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "The repository-absolute path to the file or directory to read"
                    },
                    "offset": {
                        "type": "integer",
                        "description": "The 1-indexed line number or directory entry number to start reading from"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "The maximum number of lines or directory entries to read (defaults to 2000)"
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

        let offset = args.offset.unwrap_or(1);
        if offset == 0 {
            tool_bail!("Offset must be greater than or equal to 1");
        }

        let limit = args.limit.unwrap_or(DEFAULT_READ_LIMIT);
        if limit == 0 {
            tool_bail!("Limit must be greater than or equal to 1");
        }

        let entry = git_find(&args.path)?;

        match entry {
            Entry::Commit { hash, .. } => {
                let mut out = Vec::new();
                out.push(hash);
                out.push(format!("[submodule]"));
                Ok(out.join("\n"))
            }
            Entry::Tree { .. } => {
                tool_bail!("Path is a directory. Use list tool instead.");
            }
            Entry::Blob { hash, .. } => {
                let lines = git_read(&hash)?;
                if lines.is_empty() {
                    return Ok("[Empty file]".to_string());
                }
                let res = paginate_output(lines, offset, limit, MAX_BYTES)?;
                let mut out = res.items;
                if res.more {
                    let shown = out.len();
                    let last = offset + shown.saturating_sub(1);
                    let next = offset + shown;
                    if res.cut_by_bytes {
                        out.push(format!(
                            "[Output capped at {} bytes; showing {offset}-{last} of {}. Use offset={next} to continue]",
                            MAX_BYTES, res.total
                        ));
                    } else {
                        out.push(format!(
                            "[Showing {offset}-{last} of {}. Use offset={next} to continue]",
                            res.total
                        ));
                    }
                }
                Ok(out.join("\n"))
            }
        }
    }
}

fn git_read(hash: &str) -> Result<Vec<String>, ToolError> {
    let mut child = Command::new("git")
        .env("LANG", "C.UTF-8")
        .arg("cat-file")
        .arg("blob")
        .arg(hash)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to execute git");

    let mut stderr = child.stderr.take().expect("failed to capture stderr");

    let stderr_handle = thread::spawn(move || -> String {
        let mut buf = Vec::new();
        stderr.read_to_end(&mut buf).unwrap_or_default();
        String::from_utf8_lossy(&buf).into_owned()
    });

    let mut stdout = child.stdout.take().expect("failed to capture stdout");

    let mut head = vec![0u8; 1024];
    let n = stdout.read(&mut head).expect("failed to read head");
    head.truncate(n);

    if !is_utf8_text(&head) {
        let _ = child.kill();
        let _ = child.wait();
        tool_bail!("File is not valid UTF-8 text");
    }

    let mut rest = Vec::new();
    stdout.read_to_end(&mut rest).expect("failed to read body");

    let status = child.wait().expect("command wasn't running");

    let stderr_text = stderr_handle.join().expect("handle stderr thread");
    if !status.success() {
        tool_bail!("{}", stderr_text);
    }

    let mut full = head;
    full.extend_from_slice(&rest);

    let lines: Vec<String> = String::from_utf8_lossy(&full)
        .lines()
        .enumerate()
        .map(|(index, line)| format!("{}: {}", index + 1, line))
        .collect();
    Ok(lines)
}

fn is_utf8_text(head: &[u8]) -> bool {
    if head.is_empty() {
        return true;
    }
    if head.contains(&0) {
        return false;
    }

    let bytes = head.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(head);

    for skip in 0..4 {
        let end = bytes.len().saturating_sub(skip);
        if std::str::from_utf8(&bytes[..end]).is_ok() {
            return true;
        }
    }

    let lossy = String::from_utf8_lossy(bytes);
    let replacement_count = lossy.chars().filter(|&c| c == '\u{FFFD}').count();
    let total = lossy.chars().count();
    if total == 0 {
        return true;
    }

    replacement_count as f64 / total as f64 <= 0.05
}
