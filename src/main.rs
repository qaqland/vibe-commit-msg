use std::path::PathBuf;

use anyhow::Result;

use vibe_commit_msg::config::Config;
use vibe_commit_msg::llm;
use vibe_commit_msg::tool::{self, cache};

pub enum Mode {
    Commit(Option<PathBuf>),
    English(String),
    Help,
    Skip,
    Summary,
    Tool(String, String),
}

const USAGE: &str = "\
usage: vibe-commit-msg [flags]

LLM-powered commit message generator from staged changes.

  (no flag)  Generate commit message
  -s         Update project memory + style caches
  -e <text>  Translate to English commit text
  -t <tool>  Run a single tool for debugging
  -h         Show this help";

fn parse_args() -> Result<Mode, lexopt::Error> {
    use lexopt::prelude::*;

    let mut mode = Mode::Commit(None);
    let mut free = Vec::new();

    let mut parser = lexopt::Parser::from_env();
    while let Some(arg) = parser.next()? {
        match arg {
            Short('h') | Long("help") => {
                mode = Mode::Help;
            }
            Short('s') => {
                mode = Mode::Summary;
            }
            Short('e') => {
                mode = Mode::English(String::new());
            }
            Short('t') => {
                let val = parser.value()?.into_string()?;
                mode = Mode::Tool(val, String::new());
            }
            Value(val) => {
                free.push(val);
                free.extend(parser.raw_args()?);
            }
            _ => return Err(arg.unexpected()),
        }
    }

    if let Mode::English(ref mut input) = mode {
        *input = free
            .iter()
            .map(|x| x.to_string_lossy())
            .collect::<Vec<_>>()
            .join(" ")
    }

    if let Mode::Tool(_, ref mut json) = mode {
        if let Some(val) = free.first() {
            *json = val.to_string_lossy().into_owned();
        }
    }

    if !matches!(mode, Mode::Commit(_)) {
        return Ok(mode);
    }

    let Some(editpath) = free.first() else {
        return Ok(Mode::Commit(None));
    };

    if free.len() == 1 && editpath.to_string_lossy().ends_with("COMMIT_EDITMSG") {
        return Ok(Mode::Commit(Some(editpath.into())));
    }

    Ok(Mode::Skip)
}

async fn refresh_caches(config: &Config) -> Result<()> {
    eprintln!("refreshing project summary cache");
    llm::summarize(config).await?;
    eprintln!("refreshing style cache");
    llm::style(config).await?;
    Ok(())
}

async fn run() -> Result<()> {
    let mode = parse_args()?;
    let config = Config::load()?;
    match mode {
        Mode::Commit(editmsg_path) => {
            tool::staged_hash()?;
            tool::ensure_cache_dir();

            if cache::is_stale("style.md") {
                eprintln!("refreshing stale style cache");
                refresh_caches(&config).await?;
            } else if cache::is_stale("project.md") {
                eprintln!("refreshing stale project cache");
                refresh_caches(&config).await?;
            }

            let template = editmsg_path.as_ref().and_then(|p| {
                let content = std::fs::read_to_string(p).ok()?;
                let filtered: String = content
                    .lines()
                    .filter(|line| !line.starts_with('#'))
                    .collect::<Vec<_>>()
                    .join("\n");
                let trimmed = filtered.trim_end().to_string();
                if trimmed.is_empty() {
                    None
                } else {
                    Some(trimmed)
                }
            });

            let summary = {
                eprintln!("analyzing staged changes");
                llm::commit(&config).await?
            };

            if summary == "[No staged changes]" {
                eprintln!("no staged changes");
                return Ok(());
            }

            let style_cache = cache::read_cache_file("style.md")
                .unwrap_or_else(|| "[No style cache found]".to_string());

            let output = {
                eprintln!("generating commit message");
                llm::message(&config, &summary, &style_cache, template.as_deref()).await?
            };

            if let Some(ref path) = editmsg_path {
                std::fs::write(path, &output)?;
            } else {
                println!("{}", output);
            }
        }
        Mode::English(input) => {
            eprintln!("translating");
            let output = llm::translator(&config, &input).await?;
            println!("{}", output);
        }
        Mode::Help => {
            println!("{}", USAGE);
            std::process::exit(0);
        }
        Mode::Skip => {}
        Mode::Summary => {
            tool::staged_hash()?;
            tool::ensure_cache_dir();
            refresh_caches(&config).await?;
        }
        Mode::Tool(name, json) => {
            tool::staged_hash()?;
            let output = tool::run_tool(&name, &json).await?;
            println!("{}", output);
        }
    }
    Ok(())
}

#[tokio::main]
async fn main() {
    if let Err(err) = run().await {
        eprintln!("error: {}", err);
        if std::env::var_os("VIBE_DEBUG").is_some() {
            let mut source = err.source();
            while let Some(cause) = source {
                eprintln!("caused by: {}", cause);
                source = cause.source();
            }
        }
        std::process::exit(1);
    }
}
