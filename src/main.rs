mod cloud;
mod export;
mod import;
mod session;

use anyhow::{Context, Result};
use session::find_all_sessions;
use std::path::PathBuf;

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();

    if args.len() == 1 {
        show_help();
        return Ok(());
    }

    match args[1].as_str() {
        "export" => cmd_export(),
        "import" => cmd_import(),
        "preview" => cmd_preview(&args),
        "config" => cmd_config(&args),
        "share" => cmd_share(&args),
        "fetch" => cmd_fetch(&args),
        "help" | "-h" | "--help" => cmd_help(),
        _ => cmd_unknown(&args[1]),
    }
}

fn cmd_export() -> Result<()> {
    let current_dir = std::env::current_dir()?;
    let current_path = current_dir.to_str().context("Invalid current directory path")?;

    let sessions = find_all_sessions()?;
    let current_session = sessions.iter()
        .filter(|s| s.project_path == current_path)
        .max_by_key(|s| s.last_modified);

    match current_session {
        Some(session) => export_session_success(session, &current_dir),
        None => export_session_not_found(current_path),
    }
}

fn export_session_success(session: &session::Session, current_dir: &std::path::Path) -> Result<()> {
    let output_path = current_dir.join("mcc-export.json.gz");
    let exported = export::ExportedSession::from_session(session)?;
    exported.export_to_file(&output_path)?;

    println!("✓ Session exported to ./mcc-export.json.gz");
    println!("\nShare with teammate:");
    println!("  1. Send mcc-export.json.gz via Slack/email");
    println!("  2. They drop it in their project folder");
    println!("  3. They run: mcc import");
    Ok(())
}

fn export_session_not_found(current_path: &str) -> Result<()> {
    eprintln!("✗ No Claude Code session found for current directory");
    eprintln!("  Current: {}", current_path);
    eprintln!("\nMake sure you've used Claude Code in this directory first.");
    std::process::exit(1);
}

fn cmd_import() -> Result<()> {
    let current_dir = std::env::current_dir()?;
    let file_path = current_dir.join("mcc-export.json.gz");

    if !file_path.exists() {
        eprintln!("✗ File not found: ./mcc-export.json.gz");
        eprintln!("\nMake sure you have mcc-export.json.gz in the current directory.");
        std::process::exit(1);
    }

    let target_path = current_dir.to_str().map(|s| s.to_string());

    match import::import_session(&file_path, target_path) {
        Ok(_session_file) => import_session_success(),
        Err(e) => import_session_failed(e),
    }
}

fn import_session_success() -> Result<()> {
    println!("✓ Session imported!");
    println!("\nOpen Claude Code and run /resume to continue the session.");
    Ok(())
}

fn import_session_failed(e: anyhow::Error) -> Result<()> {
    eprintln!("✗ Import failed: {}", e);
    std::process::exit(1);
}

fn cmd_preview(args: &[String]) -> Result<()> {
    if args.len() < 3 {
        eprintln!("Usage: mcc preview <file.json.gz>");
        std::process::exit(1);
    }
    let file_path = PathBuf::from(&args[2]);

    match import::preview_session(&file_path) {
        Ok(session) => preview_session_success(&session),
        Err(e) => preview_session_failed(e),
    }
}

fn preview_session_success(session: &export::ExportedSession) -> Result<()> {
    println!("Session Preview:");
    println!("  Version: {}", session.version);
    println!("  Exported by: {}", session.exported_by);
    println!("  Exported at: {}", session.exported_at);
    println!("  Project: {}", session.session.project_path);
    println!("  Summary: {}", session.session.summary);
    println!("  Messages: {}", session.session.messages.len());
    if let Some(branch) = &session.session.git_branch {
        println!("  Git branch: {}", branch);
    }
    Ok(())
}

fn preview_session_failed(e: anyhow::Error) -> Result<()> {
    eprintln!("✗ Preview failed: {}", e);
    std::process::exit(1);
}

fn cmd_config(args: &[String]) -> Result<()> {
    if args.len() < 4 || args[2] != "set-bucket" {
        eprintln!("Usage: mcc config set-bucket <gs://bucket-name>");
        std::process::exit(1);
    }
    let bucket = &args[3];
    if let Err(e) = cloud::configure_bucket(bucket) {
        eprintln!("✗ Config failed: {}", e);
        std::process::exit(1);
    }
    Ok(())
}

#[cfg_attr(not(feature = "gcs"), allow(unused_variables))]
fn cmd_share(args: &[String]) -> Result<()> {
    #[cfg(feature = "gcs")]
    {
        if args.len() < 3 {
            eprintln!("Usage: mcc share <file.json.gz>");
            std::process::exit(1);
        }
        let file_path = PathBuf::from(&args[2]);
        let config = cloud::CloudConfig::load()?;

        if !config.enabled {
            eprintln!("✗ GCS not configured. Run: mcc config set-bucket gs://your-bucket");
            std::process::exit(1);
        }

        let runtime = tokio::runtime::Runtime::new()?;
        match runtime.block_on(cloud::upload_session(&file_path, &config.bucket)) {
            Ok(gcs_path) => share_upload_success(&gcs_path),
            Err(e) => share_upload_failed(e),
        }
    }
    #[cfg(not(feature = "gcs"))]
    {
        gcs_not_enabled()
    }
}

#[allow(dead_code)]
fn share_upload_success(gcs_path: &str) -> Result<()> {
    println!("✓ Session uploaded!");
    println!("  GCS path: {}", gcs_path);
    println!("\nShare with your team:");
    println!("  mcc fetch {}", gcs_path);
    Ok(())
}

#[allow(dead_code)]
fn share_upload_failed(e: anyhow::Error) -> Result<()> {
    eprintln!("✗ Upload failed: {}", e);
    std::process::exit(1);
}

#[cfg_attr(not(feature = "gcs"), allow(unused_variables))]
fn cmd_fetch(args: &[String]) -> Result<()> {
    #[cfg(feature = "gcs")]
    {
        if args.len() < 3 {
            eprintln!("Usage: mcc fetch <gs://bucket/file.json.gz> [target-path]");
            std::process::exit(1);
        }
        let gcs_path = &args[2];
        let target_path = args.get(3).map(|s| s.to_string()).or_else(|| {
            std::env::current_dir()
                .ok()
                .and_then(|p| p.to_str().map(|s| s.to_string()))
        });

        let home = std::env::var("HOME")?;
        let temp_file = PathBuf::from(home)
            .join(".mcc/temp")
            .join("downloaded-session.json.gz");
        std::fs::create_dir_all(temp_file.parent().context("Invalid temp file path")?)?;

        let runtime = tokio::runtime::Runtime::new()?;
        if let Err(e) = runtime.block_on(cloud::download_session(gcs_path, &temp_file)) {
            eprintln!("✗ Download failed: {}", e);
            std::process::exit(1);
        }

        match import::import_session(&temp_file, target_path) {
            Ok(session_file) => fetch_import_success(&session_file),
            Err(e) => fetch_import_failed(e),
        }
    }
    #[cfg(not(feature = "gcs"))]
    {
        gcs_not_enabled()
    }
}

#[allow(dead_code)]
fn fetch_import_success(session_file: &std::path::Path) -> Result<()> {
    println!("✓ Session fetched and imported!");
    println!("  File: {}", session_file.display());
    println!("\nYou can now open Claude Code and use /resume to load this session.");
    Ok(())
}

#[allow(dead_code)]
fn fetch_import_failed(e: anyhow::Error) -> Result<()> {
    eprintln!("✗ Import failed: {}", e);
    std::process::exit(1);
}

fn gcs_not_enabled() -> Result<()> {
    eprintln!("✗ GCS support not enabled");
    eprintln!("Rebuild with: cargo build --release --features gcs");
    std::process::exit(1);
}

fn cmd_help() -> Result<()> {
    show_help();
    Ok(())
}

fn cmd_unknown(command: &str) -> Result<()> {
    eprintln!("Unknown command: {}", command);
    eprintln!("Run 'mcc help' for usage information.");
    std::process::exit(1);
}

fn show_help() {
    println!("MCC - Multi-Claude Code");
    println!("\nUsage:");
    println!("  mcc export        Export session to ./mcc-export.json.gz");
    println!("  mcc import        Import session from ./mcc-export.json.gz");
    println!("\nWorkflow:");
    println!("  1. cd /my/project && mcc export");
    println!("  2. Send mcc-export.json.gz to teammate via Slack");
    println!("  3. Teammate drops file in their project folder");
    println!("  4. cd /my/project && mcc import");
    println!("  5. claude -> /resume");
    println!("\nAdvanced:");
    println!("  mcc preview <file>        Preview session details");
    println!("\nOther:");
    println!("  mcc help                  Show this help");
}
