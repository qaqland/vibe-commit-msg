use rig::completion::Prompt;
use rig::completion::ToolDefinition;
use rig::tool::Tool;
use rig::tool::ToolError;
use serde::Deserialize;

use super::{Glob, Grep, List, Read, agent, tool_bail};

pub fn show(args: &str) -> String {
    let Ok(parsed) = serde_json::from_str::<SubagentArgs>(args) else {
        return "(?)".into();
    };
    let level = match parsed.thoroughness.as_deref() {
        Some("quick") => "quick",
        Some("thorough") => "thorough",
        _ => "medium",
    };
    let task = if parsed.task.chars().count() > 60 {
        let end = parsed
            .task
            .char_indices()
            .nth(60)
            .map(|(i, _)| i)
            .unwrap_or(parsed.task.len());
        format!("{}...", &parsed.task[..end])
    } else {
        parsed.task
    };
    format!("{} \"{}\"", level, task)
}

const EXPLORE_PREAMBLE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/docs/prompt/explore.txt"
));

const MAX_TURNS_QUICK: usize = 5;
const MAX_TURNS_MEDIUM: usize = 10;
const MAX_TURNS_THOROUGH: usize = 20;

pub struct Subagent;

#[derive(Deserialize)]
pub struct SubagentArgs {
    pub task: String,
    pub thoroughness: Option<String>,
}

impl Tool for Subagent {
    const NAME: &'static str = "subagent";

    type Args = SubagentArgs;
    type Output = String;
    type Error = ToolError;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: super::tool_description!("subagent").to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "required": ["task"],
                "properties": {
                    "task": {
                        "type": "string",
                        "description": "The question or investigation to perform"
                    },
                    "thoroughness": {
                        "type": "string",
                        "description": "Exploration depth: quick (≤5 calls), medium (≤10), thorough (≤20). Defaults to medium."
                    }
                }
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let (max_turns, level) = match args.thoroughness.as_deref() {
            Some("quick") => (MAX_TURNS_QUICK, "quick"),
            Some("thorough") => (MAX_TURNS_THOROUGH, "thorough"),
            _ => (MAX_TURNS_MEDIUM, "medium"),
        };

        let preamble = EXPLORE_PREAMBLE.replace("{level}", level);

        let agent = agent()
            .preamble(&preamble)
            .default_max_turns(max_turns)
            .tool(Read)
            .tool(List)
            .tool(Grep)
            .tool(Glob)
            .build();

        match agent.prompt(&args.task).await {
            Ok(response) => Ok(response.trim().to_string()),
            Err(e) => tool_bail!("subagent failed: {}", e),
        }
    }
}
