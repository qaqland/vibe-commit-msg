use std::sync::{Mutex, OnceLock};

use rig::completion::ToolDefinition;
use rig::tool::Tool;
use rig::tool::ToolError;
use serde::{Deserialize, Serialize};

use super::tool_bail;

const STEPS: [&str; 7] = [
    "Step 1 — Load context",
    "Step 2 — Survey the diff",
    "Step 3 — Classify and group",
    "Step 4 — Contextual deep-dive",
    "Step 5 — Subagent exploration",
    "Step 6 — Verify the summary",
    "Step 7 — Write the summary",
];

static PROGRESS: OnceLock<Mutex<[StepState; 7]>> = OnceLock::new();

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StepState {
    Pending,
    InProgress,
    Completed,
}

impl StepState {
    fn from_str(s: &str) -> Option<Self> {
        match s {
            "pending" => Some(Self::Pending),
            "in_progress" => Some(Self::InProgress),
            "completed" => Some(Self::Completed),
            _ => None,
        }
    }

    fn label(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::InProgress => "in_progress",
            Self::Completed => "completed",
        }
    }
}

pub fn reset_progress() {
    let mut state = progress_state().lock().expect("progress state");
    state.fill(StepState::Pending);
}

fn progress_state() -> &'static Mutex<[StepState; 7]> {
    PROGRESS.get_or_init(|| Mutex::new(std::array::from_fn(|_| StepState::Pending)))
}

pub fn show(args: &str) -> Option<String> {
    let parsed = serde_json::from_str::<ProgressArgs>(args).ok()?;
    if StepState::from_str(&parsed.status) != Some(StepState::Completed) {
        return None;
    }
    let idx = canonical_step(&parsed.step)?;
    let note = parsed
        .note
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("");
    Some(format!("{}/{} {}", idx + 1, STEPS.len(), note))
}

pub struct Progress;

#[derive(Debug, Deserialize, Serialize)]
pub struct ProgressArgs {
    pub step: String,
    pub status: String,
    pub note: Option<String>,
}

impl Tool for Progress {
    const NAME: &'static str = "progress";

    type Args = ProgressArgs;
    type Output = String;
    type Error = ToolError;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: super::tool_description!("progress").to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "step": {
                        "type": "string",
                        "description": "One exact commit analysis step, e.g. 'Step 1 — Load context'."
                    },
                    "status": {
                        "type": "string",
                        "enum": ["pending", "in_progress", "completed"],
                        "description": "The step status. A step must be marked in_progress before completed."
                    },
                    "note": {
                        "type": "string",
                        "description": "Required when status is completed; summarize the evidence or result from that step."
                    }
                },
                "required": ["step", "status"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let Some(idx) = canonical_step(&args.step) else {
            tool_bail!("Unknown progress step: {}", args.step);
        };

        let Some(next) = StepState::from_str(&args.status) else {
            tool_bail!("Invalid progress status: {}", args.status);
        };

        let note = args.note.map(|note| note.trim().to_string());
        if next == StepState::Completed && note.as_deref().unwrap_or_default().is_empty() {
            tool_bail!("note is required when status is completed");
        }

        let mut state = progress_state().lock().expect("progress state");
        let current = state[idx];

        match (current, next) {
            (StepState::Pending, StepState::Completed) => {
                tool_bail!("{} must be marked in_progress before completed", STEPS[idx]);
            }
            (StepState::Completed, StepState::InProgress | StepState::Pending) => {
                tool_bail!("{} is already completed", STEPS[idx]);
            }
            _ => {}
        }

        state[idx] = next;

        let done = state.iter().filter(|s| **s == StepState::Completed).count();
        let note_part = note
            .as_deref()
            .filter(|s| !s.is_empty())
            .map(|n| format!(" ({})", n))
            .unwrap_or_default();
        Ok(format!(
            "{} {}{}. {}/7 done.",
            STEPS[idx],
            next.label(),
            note_part,
            done
        ))
    }
}

fn canonical_step(input: &str) -> Option<usize> {
    let trimmed = input.trim();
    STEPS.iter().position(|step| {
        trimmed == *step
            || trimmed.starts_with(
                step.split(' ')
                    .take(2)
                    .collect::<Vec<_>>()
                    .join(" ")
                    .as_str(),
            )
    })
}
