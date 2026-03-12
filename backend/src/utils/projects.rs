use serde::Serialize;
use std::path::Path;

#[derive(Debug, Serialize)]
pub struct ProjectInfo {
    pub name: String,
    pub path: String,
    pub has_git: bool,
    pub last_modified: Option<String>,
}

/// Discover projects in common locations
pub async fn discover_projects(base_dirs: &[&str]) -> Vec<ProjectInfo> {
    let mut projects = Vec::new();

    for base_dir in base_dirs {
        let path = Path::new(base_dir);
        if !path.exists() {
            continue;
        }

        if let Ok(entries) = std::fs::read_dir(path) {
            for entry in entries.flatten() {
                let entry_path = entry.path();
                if !entry_path.is_dir() {
                    continue;
                }

                // Skip hidden directories
                if entry
                    .file_name()
                    .to_str()
                    .map(|s| s.starts_with('.'))
                    .unwrap_or(true)
                {
                    continue;
                }

                let has_git = entry_path.join(".git").exists();
                let name = entry
                    .file_name()
                    .to_str()
                    .unwrap_or("unknown")
                    .to_string();

                let last_modified = entry
                    .metadata()
                    .ok()
                    .and_then(|m| m.modified().ok())
                    .map(|t| {
                        chrono::DateTime::<chrono::Utc>::from(t)
                            .format("%Y-%m-%dT%H:%M:%SZ")
                            .to_string()
                    });

                projects.push(ProjectInfo {
                    name,
                    path: entry_path.to_string_lossy().to_string(),
                    has_git,
                    last_modified,
                });
            }
        }
    }

    // Sort by last modified (newest first)
    projects.sort_by(|a, b| b.last_modified.cmp(&a.last_modified));
    projects
}

/// Create a workspace directory
pub fn create_workspace(path: &str, init_git: bool) -> Result<String, std::io::Error> {
    std::fs::create_dir_all(path)?;

    if init_git {
        std::process::Command::new("git")
            .arg("init")
            .current_dir(path)
            .output()?;
    }

    Ok(path.to_string())
}
