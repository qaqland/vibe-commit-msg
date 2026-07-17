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

use std::time::Duration;

use anyhow::Result;
use rig::completion::{CompletionError, Prompt, PromptError};

use crate::tool::{self, Cache, Diff, Glob, Grep, List, Log, Read, Stat, Subagent};

const DEFAULT_MAX_TURNS: usize = 100;

const MAX_ATTEMPTS: usize = 4;
const INITIAL_BACKOFF: Duration = Duration::from_secs(2);
const MAX_BACKOFF: Duration = Duration::from_secs(30);

fn is_retryable(err: &PromptError) -> bool {
    let PromptError::CompletionError(err) = err else {
        return false;
    };
    match err {
        CompletionError::HttpError(err) => match err {
            rig::http_client::Error::InvalidStatusCode(status)
            | rig::http_client::Error::InvalidStatusCodeWithMessage(status, _) => {
                // 4xx means a bad request or bad credentials; retrying won't fix
                // that, except for 408 (timeout) and 429 (rate limit).
                let code = status.as_u16();
                code == 408 || code == 429 || !status.is_client_error()
            }
            _ => true, // connection reset, DNS, TLS, ...
        },
        CompletionError::JsonError(_) | CompletionError::ResponseError(_) => true,
        CompletionError::ProviderError(msg) => {
            let msg = msg.to_lowercase();
            !(msg.contains("401")
                || msg.contains("403")
                || msg.contains("invalid_api_key")
                || msg.contains("unauthorized")
                || msg.contains("authentication"))
        }
        _ => false,
    }
}

fn short_error(err: &PromptError) -> String {
    let msg = err.to_string();
    let first = msg.lines().next().unwrap_or(&msg);
    first.chars().take(160).collect()
}

pub(crate) async fn prompt_with_retry<F, Fut>(step: &str, mut send: F) -> Result<String>
where
    F: FnMut() -> Fut,
    Fut: std::future::IntoFuture<Output = Result<String, PromptError>>,
{
    let mut backoff = INITIAL_BACKOFF;
    for attempt in 1..=MAX_ATTEMPTS {
        match send().await {
            Ok(text) => return Ok(text.trim().to_string()),
            Err(err) => {
                if attempt == MAX_ATTEMPTS || !is_retryable(&err) {
                    return Err(err.into());
                }
                eprintln!(
                    "  ! {} failed (attempt {attempt}/{MAX_ATTEMPTS}): {}",
                    step,
                    short_error(&err)
                );
                eprintln!("    retrying in {}s", backoff.as_secs());
                tokio::time::sleep(backoff).await;
                backoff = (backoff * 2).min(MAX_BACKOFF);
            }
        }
    }
    unreachable!()
}

pub async fn translator(input: &str) -> Result<String> {
    let preamble = prompt_description!("translator");
    let agent = tool::agent().preamble(preamble).build();
    prompt_with_retry("translate", || agent.prompt(input)).await
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
    prompt_with_retry("summarize", || {
        agent.prompt("Update the project memory cache. Start by reading existing cache files, then investigate the repository and write updates.")
    })
    .await
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
    prompt_with_retry("style analysis", || {
        agent.prompt("Analyze this repository's commit message style conventions. Check existing cache files first, then examine documented conventions and git history. Write the results to the style cache.")
    })
    .await
}

pub async fn commit() -> Result<String> {
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
        .tool(Subagent)
        .build();
    prompt_with_retry("change analysis", || {
        agent.prompt("Analyze the staged changes and produce a structured change summary. Review the project memory cache first, then examine the diff and stat.")
    })
    .await
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
    prompt_with_retry("message generation", || agent.prompt(&prompt_text)).await
}
