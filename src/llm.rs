macro_rules! prompt_description {
    ($name:literal) => {
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/docs/prompt/",
            $name,
            ".txt"
        ))
    };
}

use anyhow::Result;
use rig::completion::Prompt;

use crate::tool::{self, Cache, Diff, Glob, Grep, List, Log, Progress, Read, Stat, Subagent};

const DEFAULT_MAX_TURNS: usize = 100;

pub async fn translator(input: &str) -> Result<String> {
    let preamble = prompt_description!("translator");
    let agent = tool::agent().preamble(preamble).build();
    let response = agent.prompt(input).await?;
    Ok(response.trim().to_string())
}

pub async fn summarize() -> Result<String> {
    let preamble = prompt_description!("summarize");
    let agent = tool::agent()
        .preamble(preamble)
        .default_max_turns(DEFAULT_MAX_TURNS)
        .tool(List)
        .tool(Glob)
        .tool(Grep)
        .tool(Read)
        .tool(Cache)
        .tool(Subagent)
        .build();
    let response = agent
        .prompt("Update the project memory cache. Start by reading existing cache files, then investigate the repository and write updates.")
        .await?;
    Ok(response.trim().to_string())
}

pub async fn style() -> Result<String> {
    let preamble = prompt_description!("style");
    let agent = tool::agent()
        .preamble(preamble)
        .default_max_turns(DEFAULT_MAX_TURNS)
        .tool(Glob)
        .tool(Grep)
        .tool(Read)
        .tool(Log)
        .tool(Cache)
        .build();
    let response = agent
        .prompt("Analyze this repository's commit message style conventions. Check existing cache files first, then examine documented conventions and git history. Write the results to the style cache.")
        .await?;
    Ok(response.trim().to_string())
}

pub async fn commit() -> Result<String> {
    tool::progress::reset_progress();
    let preamble = prompt_description!("commit");
    let agent = tool::agent()
        .preamble(preamble)
        .default_max_turns(DEFAULT_MAX_TURNS)
        .tool(Cache)
        .tool(Diff)
        .tool(Stat)
        .tool(Read)
        .tool(Grep)
        .tool(Glob)
        .tool(Progress)
        .tool(Subagent)
        .build();
    let response = agent
        .prompt("Analyze the staged changes and produce a structured change summary. Review the project memory cache first, then examine the diff and stat.")
        .await?;
    Ok(response.trim().to_string())
}

pub async fn message(summary: &str, style_cache: &str, template: Option<&str>) -> Result<String> {
    let preamble = prompt_description!("message");
    let mut prompt_text = format!(
        "## Style Rules\n\n{}\n\n## Change Summary\n\n{}",
        style_cache, summary
    );
    if let Some(tmpl) = template {
        prompt_text.push_str(&format!("\n\n## Reference Template\n\n{}", tmpl));
    }
    let agent = tool::agent().preamble(preamble).build();
    let response = agent.prompt(prompt_text).await?;
    Ok(response.trim().to_string())
}
