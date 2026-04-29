macro_rules! prompt_description {
    ($name:literal) => {
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/docs/prompt/",
            $name,
            ".md"
        ))
    };
}

use anyhow::Result;
use rig::client::CompletionClient;
use rig::completion::Prompt;
use rig::providers::openai;

use crate::config::Config;
use crate::tool::{Cache, Diff, Glob, Grep, List, Log, Read, Stat};

pub async fn translator(config: &Config, input: &str) -> Result<String> {
    let client = openai::CompletionsClient::builder()
        .api_key(&config.auth_key)
        .base_url(&config.base_url)
        .build()?;

    let preamble = prompt_description!("translator");

    let agent = client.agent(&config.model_id).preamble(preamble).build();

    let response = agent.prompt(input).await?;
    Ok(response.trim().to_string())
}

pub async fn summarize(config: &Config) -> Result<String> {
    let client = openai::CompletionsClient::builder()
        .api_key(&config.auth_key)
        .base_url(&config.base_url)
        .build()?;

    let preamble = prompt_description!("summarize");

    let agent = client
        .agent(&config.model_id)
        .preamble(preamble)
        .default_max_turns(20)
        .tool(List)
        .tool(Glob)
        .tool(Grep)
        .tool(Read)
        .tool(Diff)
        .tool(Stat)
        .tool(Log)
        .tool(Cache)
        .build();

    let response = agent
        .prompt("Explore the staged snapshot and update the project memory cache. Start by listing existing cache files, then examine the repository.")
        .await?;
    Ok(response.trim().to_string())
}

pub async fn style(config: &Config) -> Result<String> {
    let client = openai::CompletionsClient::builder()
        .api_key(&config.auth_key)
        .base_url(&config.base_url)
        .build()?;

    let preamble = prompt_description!("style");

    let agent = client
        .agent(&config.model_id)
        .preamble(preamble)
        .default_max_turns(15)
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

pub async fn commit(config: &Config) -> Result<String> {
    let client = openai::CompletionsClient::builder()
        .api_key(&config.auth_key)
        .base_url(&config.base_url)
        .build()?;

    let preamble = prompt_description!("commit");

    let agent = client
        .agent(&config.model_id)
        .preamble(preamble)
        .default_max_turns(15)
        .tool(Cache)
        .tool(Diff)
        .tool(Stat)
        .tool(Read)
        .tool(Grep)
        .tool(Glob)
        .build();

    let response = agent
        .prompt("Analyze the staged changes and produce a structured change summary. Review the project memory cache first, then examine the diff and stat.")
        .await?;
    Ok(response.trim().to_string())
}

pub async fn message(config: &Config, summary: &str, style_cache: &str) -> Result<String> {
    let client = openai::CompletionsClient::builder()
        .api_key(&config.auth_key)
        .base_url(&config.base_url)
        .build()?;

    let preamble = prompt_description!("message");

    let prompt_text = format!(
        "## Style Rules\n\n{}\n\n## Change Summary\n\n{}",
        style_cache, summary
    );

    let agent = client.agent(&config.model_id).preamble(preamble).build();

    let response = agent.prompt(prompt_text).await?;
    Ok(response.trim().to_string())
}
