use anyhow::{Context, Result};
use flate2::write::GzEncoder;
use flate2::Compression;
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::session::{Session, SessionMessage};

#[derive(Debug, Serialize, Deserialize)]
pub struct ExportedSession {
    pub version: String,
    pub exported_at: String,
    pub exported_by: String,
    pub session: SessionData,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SessionData {
    pub id: String,
    pub project_path: String,
    pub messages: Vec<SessionMessage>,
    pub summary: String,
    pub git_branch: Option<String>,
}

impl ExportedSession {
    pub fn from_session(session: &Session) -> Result<Self> {
        let hostname = hostname::get()
            .ok()
            .and_then(|h| h.into_string().ok())
            .unwrap_or_else(|| "unknown".to_string());

        let username = std::env::var("USER").unwrap_or_else(|_| "unknown".to_string());

        Ok(ExportedSession {
            version: "1.0.0".to_string(),
            exported_at: chrono::Utc::now().to_rfc3339(),
            exported_by: format!("{}@{}", username, hostname),
            session: SessionData {
                id: session.id.clone(),
                project_path: session.project_path.clone(),
                messages: session.messages.clone(),
                summary: session.summary.clone(),
                git_branch: session.git_branch.clone(),
            },
        })
    }

    /// Export session to a compressed .mcc file
    pub fn export_to_file(&self, output_path: &Path) -> Result<()> {
        let json = serde_json::to_string_pretty(&self)?;

        let file = File::create(output_path)
            .context(format!("Failed to create file: {:?}", output_path))?;

        let mut encoder = GzEncoder::new(file, Compression::default());
        encoder
            .write_all(json.as_bytes())
            .context("Failed to write compressed data")?;
        encoder
            .finish()
            .context("Failed to finish compression")?;

        Ok(())
    }
}

/// Export a session to an .mcc file
pub fn export_session(session: &Session, output_dir: Option<&Path>) -> Result<PathBuf> {
    let exported = ExportedSession::from_session(session)?;

    // Determine output directory
    let output_dir = output_dir
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| {
            let home = std::env::var("HOME").unwrap();
            PathBuf::from(home).join(".mcc/exports")
        });

    // Create output directory if it doesn't exist
    fs::create_dir_all(&output_dir)?;

    // Generate filename
    let timestamp = chrono::Utc::now().format("%Y%m%d-%H%M%S");
    let safe_summary = session
        .summary
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == ' ' || *c == '-')
        .take(30)
        .collect::<String>()
        .replace(' ', "-")
        .to_lowercase();

    let filename = format!("{}-{}.json.gz", timestamp, safe_summary);
    let output_path = output_dir.join(filename);

    exported.export_to_file(&output_path)?;

    Ok(output_path)
}
