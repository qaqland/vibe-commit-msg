use std::process::Command;

use anyhow::{Result, anyhow, bail};

// Advertising space.
const DEFAULT_BASE_URL: &str = "https://api.openai.com/v1";
const DEFAULT_MODEL_ID: &str = "gpt-5.2";

#[derive(Clone, Debug)]
pub struct Config {
    pub base_url: String,
    pub model_id: String,
    pub auth_key: String,
}

impl Config {
    pub fn load() -> Result<Self> {
        let base_url =
            read_git_config("vibe.base-url")?.unwrap_or_else(|| DEFAULT_BASE_URL.to_owned());
        let model_id =
            read_git_config("vibe.model-id")?.unwrap_or_else(|| DEFAULT_MODEL_ID.to_owned());

        let auth_key =
            read_git_config("vibe.auth-key")?.ok_or_else(|| anyhow!("vibe.auth-key is not set"))?;

        Ok(Self {
            base_url,
            model_id,
            auth_key,
        })
    }
}

fn read_git_config(key: &str) -> Result<Option<String>> {
    let output = Command::new("git")
        .arg("config")
        .arg("--get")
        .arg(key)
        .env("LANG", "C.UTF-8")
        .output()?;

    if output.status.code() == Some(1) {
        // #define CONFIG_INVALID_KEY 1
        return Ok(None);
    }

    let stdout = String::from_utf8_lossy(&output.stdout)
        .trim_end()
        .to_string();
    let stderr = String::from_utf8_lossy(&output.stderr)
        .trim_end()
        .to_string();

    if !output.status.success() {
        bail!("git config: {}", stderr);
    }
    if stdout.is_empty() {
        Ok(None)
    } else {
        Ok(Some(stdout))
    }
}
