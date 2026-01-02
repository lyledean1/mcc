use anyhow::{Context, Result};
use flate2::read::GzDecoder;
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};

use crate::export::ExportedSession;

/// Import a session from an .mcc file
pub fn import_session(mcc_file: &Path, target_project_path: Option<String>) -> Result<PathBuf> {
    // Read and decompress the .mcc file
    let file = File::open(mcc_file).context("Failed to open .mcc file")?;
    let mut decoder = GzDecoder::new(file);
    let mut json = String::new();
    decoder
        .read_to_string(&mut json)
        .context("Failed to decompress .mcc file")?;

    let exported: ExportedSession =
        serde_json::from_str(&json).context("Failed to parse .mcc file")?;

    // Determine target project path
    let project_path = if let Some(path) = target_project_path {
        path
    } else {
        // Ask user or use current directory
        std::env::current_dir()?
            .to_str()
            .context("Invalid current directory")?
            .to_string()
    };

    // Create Claude projects directory structure
    let home = std::env::var("HOME")?;
    let encoded_path = project_path.replace("/", "-");
    let session_dir = PathBuf::from(&home)
        .join(".claude/projects")
        .join(&encoded_path);

    fs::create_dir_all(&session_dir)?;

    // Generate new session file
    let session_file = session_dir.join(format!("{}.jsonl", exported.session.id));

    // Write session messages as JSONL, rewriting paths
    let original_path = &exported.session.project_path;
    let mut output = String::new();

    for message in &exported.session.messages {
        let mut msg = message.clone();

        // Rewrite the cwd field in the data if it exists
        if let Some(cwd) = msg.data.get("cwd").and_then(|v| v.as_str())
            && cwd == original_path
            && let Some(obj) = msg.data.as_object_mut()
        {
            obj.insert("cwd".to_string(), serde_json::json!(project_path));
        }

        output.push_str(&serde_json::to_string(&msg)?);
        output.push('\n');
    }

    fs::write(&session_file, output)?;

    // Update ~/.claude.json to register the session
    update_claude_config(&project_path, &exported.session.id)?;

    Ok(session_file)
}

/// Update ~/.claude.json to set lastSessionId for the project
fn update_claude_config(project_path: &str, session_id: &str) -> Result<()> {
    let home = std::env::var("HOME")?;
    let config_path = PathBuf::from(home).join(".claude.json");

    let content = fs::read_to_string(&config_path)?;
    let mut config: serde_json::Value = serde_json::from_str(&content)?;

    // Set lastSessionId for this project
    if let Some(projects) = config.get_mut("projects")
        && let Some(project) = projects.get_mut(project_path)
    {
        project["lastSessionId"] = serde_json::json!(session_id);
    }

    fs::write(&config_path, serde_json::to_string_pretty(&config)?)?;
    Ok(())
}

/// Preview an .mcc file without importing
pub fn preview_session(mcc_file: &Path) -> Result<ExportedSession> {
    let file = File::open(mcc_file).context("Failed to open .mcc file")?;
    let mut decoder = GzDecoder::new(file);
    let mut json = String::new();
    decoder
        .read_to_string(&mut json)
        .context("Failed to decompress .mcc file")?;

    serde_json::from_str(&json).context("Failed to parse .mcc file")
}
