use std::collections::HashMap;
use tokio::process::Command;

pub async fn get_cli_versions() -> HashMap<String, String> {
    let mut versions = HashMap::new();

    for cli in &["claude", "codex", "cursor", "gemini"] {
        if let Ok(output) = Command::new(cli).arg("--version").output().await {
            if output.status.success() {
                let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
                versions.insert(cli.to_string(), version);
            }
        }
    }

    versions
}

pub async fn update_cli(
    provider: &str,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    match provider {
        "claude" => {
            // Try claude update first, fall back to npm
            let status = Command::new("claude")
                .arg("update")
                .status()
                .await;

            match status {
                Ok(s) if s.success() => {}
                _ => {
                    let npm_status = Command::new("npm")
                        .args(["update", "-g", "@anthropic-ai/claude-code"])
                        .status()
                        .await?;

                    if !npm_status.success() {
                        return Err("Failed to update claude CLI".into());
                    }
                }
            }
        }
        "codex" => {
            let status = Command::new("npm")
                .args(["update", "-g", "@openai/codex"])
                .status()
                .await?;

            if !status.success() {
                return Err("Failed to update codex CLI".into());
            }
        }
        _ => {
            return Err(format!("Unknown provider for update: {}", provider).into());
        }
    }

    // Get new version
    let output = Command::new(provider)
        .arg("--version")
        .output()
        .await?;

    let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(version)
}
