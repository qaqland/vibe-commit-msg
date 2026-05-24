use std::{process::Command, sync::OnceLock};

use anyhow::{Result, bail};

pub mod cache;
pub mod diff;
pub mod glob;
pub mod grep;
pub mod list;
pub mod log;
pub mod read;
pub mod stat;
pub mod subagent;

pub use cache::Cache;
pub use diff::Diff;
pub use glob::Glob;
pub use grep::Grep;
pub use list::List;
pub use log::Log;
pub use read::Read;
pub use stat::Stat;
pub use subagent::Subagent;

static STAGED_HASH: OnceLock<String> = OnceLock::new();
static CONFIG: OnceLock<crate::config::Config> = OnceLock::new();

pub fn set_config(config: crate::config::Config) {
    CONFIG.set(config).expect("config already set");
}

pub fn staged_hash() -> Result<()> {
    if STAGED_HASH.get().is_some() {
        return Ok(());
    }

    let output = Command::new("git")
        .env("LANG", "C.UTF-8")
        .arg("rev-parse")
        .arg("--show-toplevel")
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8(output.stderr)?.trim().to_string();
        bail!("git rev-parse --show-toplevel failed: {}", stderr);
    }

    let root = String::from_utf8(output.stdout)?.trim().to_string();
    std::env::set_current_dir(&root)?;

    let output = Command::new("git")
        .env("LANG", "C.UTF-8")
        .arg("write-tree")
        .output()?;

    let stdout = String::from_utf8(output.stdout)?.trim().to_string();
    let stderr = String::from_utf8(output.stderr)?.trim().to_string();

    if !output.status.success() {
        bail!("git write-tree {}", stderr);
    }

    if stdout.is_empty() {
        bail!("git write-tree empty output");
    }

    let _ = STAGED_HASH.set(stdout);
    Ok(())
}

//

#[derive(Debug)]
pub struct GitOutput {
    pub status: std::process::ExitStatus,
    pub stdout: String,
    pub stderr: String,
}

pub fn run_git<I, S>(args: I) -> GitOutput
where
    I: IntoIterator<Item = S>,
    S: AsRef<std::ffi::OsStr>,
{
    let output = Command::new("git")
        .env("LANG", "C.UTF-8")
        .args(args)
        .output()
        .unwrap_or_else(|err| {
            // it's not the first time
            eprintln!("failed to execute git: {err}");
            std::process::exit(1);
        });

    let result = GitOutput {
        status: output.status,
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    };

    result
}

macro_rules! tool_bail {
    ($($arg:tt)*) => {
        return Err(rig::tool::ToolError::ToolCallError(
            format!($($arg)*).into(),
        ))
    };
}

pub(crate) use tool_bail;

macro_rules! tool_description {
    ($name:literal) => {
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/docs/tool/",
            $name,
            ".txt"
        ))
    };
}
pub(crate) use tool_description;

pub(crate) fn cache_dir() -> std::path::PathBuf {
    std::env::current_dir()
        .expect("not in a git repo")
        .join(".git")
        .join("vibe")
}

pub fn ensure_cache_dir() {
    let _ = std::fs::create_dir_all(cache_dir());
}

pub(crate) struct PaginateResult {
    pub items: Vec<String>,
    pub total: usize,
    pub cut_by_bytes: bool,
    pub more: bool,
}

pub(crate) fn paginate_output<I, S>(
    items: I,
    offset: usize,
    limit: usize,
    max_bytes: usize,
) -> Result<PaginateResult, rig::tool::ToolError>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let start = offset.saturating_sub(1);

    let mut collected = Vec::new();
    let mut bytes = 0;
    let mut count = 0;
    let mut cut_by_bytes = false;
    let mut more = false;

    for item in items {
        count += 1;
        if count <= start {
            continue;
        }

        if cut_by_bytes || collected.len() >= limit {
            more = true;
            continue;
        }

        let item = item.into();
        let size = item.len() + usize::from(!collected.is_empty());
        if bytes + size > max_bytes {
            cut_by_bytes = true;
            more = true;
            continue;
        }

        collected.push(item);
        bytes += size;
    }

    if count < offset && !(count == 0 && offset == 1) {
        tool_bail!("Offset {offset} is out of range ({count} items available)");
    }

    Ok(PaginateResult {
        items: collected,
        total: count,
        cut_by_bytes,
        more,
    })
}

// Shared git helpers for read / list tools

pub(crate) enum Entry {
    Blob { path: String, hash: String },
    Tree { path: String },
    Commit { path: String, hash: String },
}

impl Entry {
    pub fn path(self) -> String {
        match self {
            Entry::Blob { path, .. } => path,
            Entry::Tree { path } => path,
            Entry::Commit { path, .. } => path,
        }
    }
}

pub(crate) fn parse_tree_line(line: &str) -> Option<Entry> {
    let mut parts = line.splitn(3, '\t');
    let kind = parts.next()?;
    let hash = parts.next()?.to_string();
    let path = parts.next()?.to_string();

    Some(match kind {
        "blob" => Entry::Blob { path, hash },
        "tree" => Entry::Tree {
            path: format!("{}/", path),
        },
        "commit" => Entry::Commit { path, hash },
        _ => return None,
    })
}

pub(crate) fn git_find(path: &str) -> Result<Entry, rig::tool::ToolError> {
    let filepath = path.trim_matches('/');
    if filepath.is_empty() {
        return Ok(Entry::Tree { path: "/.".into() });
    }

    let tree = STAGED_HASH.get().expect("git staged-tree");
    let output = run_git([
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
    let entry = output.stdout.lines().next().and_then(parse_tree_line);
    let Some(one) = entry else {
        tool_bail!("File not found")
    };
    Ok(one)
}

use rig::agent::{PromptHook, ToolCallHookAction};
use rig::client::CompletionClient;
use rig::providers::openai;
use rig::tool::ToolSet;

fn tool_set() -> ToolSet {
    let mut toolset = ToolSet::default();
    toolset.add_tool(cache::Cache);
    toolset.add_tool(log::Log);
    toolset.add_tool(read::Read);
    toolset.add_tool(list::List);
    toolset.add_tool(glob::Glob);
    toolset.add_tool(grep::Grep);
    toolset.add_tool(stat::Stat);
    toolset.add_tool(diff::Diff);
    toolset.add_tool(subagent::Subagent);
    toolset
}

pub async fn run_tool(name: &str, json: &str) -> Result<String> {
    let raw = tool_set().call(name, json.to_string()).await?;
    Ok(raw)
}

//

#[derive(Clone)]
pub struct ToolLog;

impl PromptHook<openai::CompletionModel> for ToolLog {
    async fn on_tool_call(
        &self,
        name: &str,
        _: Option<String>,
        _: &str,
        args: &str,
    ) -> ToolCallHookAction {
        let summary = match name {
            "read" => read::show(args),
            "list" => list::show(args),
            "grep" => grep::show(args),
            "glob" => glob::show(args),
            "diff" => diff::show(args),
            "stat" => stat::show(args),
            "log" => log::show(args),
            "cache" => cache::show(args),
            "subagent" => subagent::show(args),
            _ => "(?)".into(),
        };
        eprintln!("{:>8} {}", name, summary);
        ToolCallHookAction::cont()
    }
}

pub fn agent() -> rig::agent::AgentBuilder<openai::CompletionModel, ToolLog> {
    let config = CONFIG.get().expect("config not initialized");
    let client = openai::CompletionsClient::builder()
        .api_key(&config.auth_key)
        .base_url(&config.base_url)
        .build()
        .expect("failed to build LLM client");
    client.agent(&config.model_id).hook(ToolLog)
}
