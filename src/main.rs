use anyhow::Result;

use vibe_commit_msg::config::Config;
use vibe_commit_msg::llm;
use vibe_commit_msg::tool::{self, cache};

pub enum Mode {
    Commit,
    English(String),
    Help,
    Summary,
    Tool(String, String),
}

const USAGE: &str = "\
vibe-commit-msg [flags]

Generate commit messages with AI from staged changes.

  (no flag)  Generate commit message
  -s         Update project memory + style caches
  -e <text>  Translate to English commit text
  -t <tool>  Run a single tool for debugging
  -h         Show this help";

fn parse_args() -> Result<Mode, lexopt::Error> {
    use lexopt::prelude::*;

    let mut mode = Mode::Commit;
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

    Ok(mode)
}

async fn refresh_caches(config: &Config) -> Result<()> {
    eprintln!("Refreshing project summary cache...");
    llm::summarize(config).await?;
    eprintln!("Refreshing style cache...");
    llm::style(config).await?;
    Ok(())
}

async fn run() -> Result<()> {
    let mode = parse_args()?;
    let config = Config::load()?;
    match mode {
        Mode::Commit => {
            tool::staged_hash()?;
            tool::ensure_cache_dir();

            if cache::is_stale("style.md") {
                eprintln!("Style cache is stale, refreshing...");
                refresh_caches(&config).await?;
            } else if cache::is_stale("project.md") {
                eprintln!("Project cache is stale, refreshing...");
                refresh_caches(&config).await?;
            }

            let summary = llm::commit(&config).await?;

            if summary == "[No staged changes]" {
                println!("{}", summary);
                return Ok(());
            }

            let style_cache = cache::read_cache_file("style.md")
                .unwrap_or_else(|| "[No style cache found]".to_string());

            let output = llm::message(&config, &summary, &style_cache).await?;
            println!("{}", output);
        }
        Mode::English(input) => {
            let output = llm::translator(&config, &input).await?;
            println!("{}", output);
        }
        Mode::Help => {
            println!("{}", USAGE);
            std::process::exit(0);
        }
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
