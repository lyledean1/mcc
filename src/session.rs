use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SessionMessage {
    #[serde(rename = "type")]
    pub msg_type: String,
    #[serde(flatten)]
    pub data: serde_json::Value,
}

#[derive(Debug, Clone)]
pub struct Session {
    pub id: String,
    pub project_path: String,
    #[allow(dead_code)]
    pub file_path: PathBuf,
    pub messages: Vec<SessionMessage>,
    pub last_modified: u64,
    pub summary: String,
    pub git_branch: Option<String>,
}

impl Session {
    /// Load a session from a .jsonl file
    pub fn load(file_path: PathBuf, project_path: String) -> Result<Self> {
        let content = fs::read_to_string(&file_path)
            .context("Failed to read session file")?;

        let messages: Vec<SessionMessage> = content
            .lines()
            .filter_map(|line| serde_json::from_str(line).ok())
            .collect();

        // Extract session ID from filename
        let id = file_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        // Get last modified time
        let metadata = fs::metadata(&file_path)?;
        let last_modified = metadata
            .modified()?
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs();

        // Extract git branch and summary from messages
        let mut git_branch = None;
        let mut summary = String::from("No messages");

        for msg in &messages {
            if let Some(branch) = msg.data.get("gitBranch").and_then(|v| v.as_str()) {
                git_branch = Some(branch.to_string());
            }

            // Try to get first user message as summary
            if msg.msg_type == "user"
                && let Some(message) = msg.data.get("message")
                && let Some(content) = message.get("content").and_then(|v| v.as_str())
            {
                summary = content.chars().take(60).collect();
                if content.len() > 60 {
                    summary.push_str("...");
                }
                break;
            }
        }

        Ok(Session {
            id,
            project_path,
            file_path,
            messages,
            last_modified,
            summary,
            git_branch,
        })
    }

    /// Get the number of messages in this session
    #[allow(dead_code)]
    pub fn message_count(&self) -> usize {
        self.messages.len()
    }

    /// Get formatted time ago
    #[allow(dead_code)]
    pub fn time_ago(&self) -> String {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let diff = now.saturating_sub(self.last_modified);

        if diff < 60 {
            format!("{}s ago", diff)
        } else if diff < 3600 {
            format!("{}m ago", diff / 60)
        } else if diff < 86400 {
            format!("{}h ago", diff / 3600)
        } else {
            format!("{}d ago", diff / 86400)
        }
    }
}

/// Find all Claude Code sessions
pub fn find_all_sessions() -> Result<Vec<Session>> {
    let home = std::env::var("HOME")?;
    let projects_dir = PathBuf::from(home).join(".claude/projects");

    if !projects_dir.exists() {
        return Ok(Vec::new());
    }

    let mut sessions = Vec::new();

    for project_entry in fs::read_dir(&projects_dir)? {
        let project_entry = project_entry?;
        let project_path = project_entry.path();

        if !project_path.is_dir() {
            continue;
        }

        // Decode project name from directory
        let project_name = project_path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .replace("-", "/");

        // Find all .jsonl files in this project directory
        for session_entry in fs::read_dir(&project_path)? {
            let session_entry = session_entry?;
            let session_path = session_entry.path();

            if session_path.extension().and_then(|s| s.to_str()) == Some("jsonl")
                && let Ok(session) = Session::load(session_path, project_name.clone())
            {
                sessions.push(session);
            }
        }
    }

    // Sort by last modified (most recent first)
    sessions.sort_by(|a, b| b.last_modified.cmp(&a.last_modified));

    Ok(sessions)
}
