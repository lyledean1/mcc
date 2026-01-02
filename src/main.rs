mod cloud;
mod export;
mod import;
mod session;

use anyhow::{Context, Result};
use session::find_all_sessions;
use std::path::PathBuf;

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();

    // If no arguments, show help
    if args.len() == 1 {
        show_help();
        return Ok(());
    }

    // Handle CLI commands
    match args[1].as_str() {
            "export" => {
                // mcc export - always exports to ./mcc-export.json.gz
                let current_dir = std::env::current_dir()?;
                let current_path = current_dir.to_str().context("Invalid current directory path")?;

                let sessions = find_all_sessions()?;
                let current_session = sessions.iter()
                    .filter(|s| s.project_path == current_path)
                    .max_by_key(|s| s.last_modified);

                match current_session {
                    Some(session) => {
                        let output_path = current_dir.join("mcc-export.json.gz");

                        // Export
                        let exported = export::ExportedSession::from_session(session)?;
                        exported.export_to_file(&output_path)?;

                        println!("✓ Session exported to ./mcc-export.json.gz");
                        println!("\nShare with teammate:");
                        println!("  1. Send mcc-export.json.gz via Slack/email");
                        println!("  2. They drop it in their project folder");
                        println!("  3. They run: mcc import");
                    }
                    None => {
                        eprintln!("✗ No Claude Code session found for current directory");
                        eprintln!("  Current: {}", current_path);
                        eprintln!("\nMake sure you've used Claude Code in this directory first.");
                        std::process::exit(1);
                    }
                }
                Ok(())
            }
            "import" => {
                // mcc import - looks for ./mcc-export.json.gz
                let current_dir = std::env::current_dir()?;
                let file_path = current_dir.join("mcc-export.json.gz");

                if !file_path.exists() {
                    eprintln!("✗ File not found: ./mcc-export.json.gz");
                    eprintln!("\nMake sure you have mcc-export.json.gz in the current directory.");
                    std::process::exit(1);
                }

                let target_path = current_dir.to_str().map(|s| s.to_string());

                match import::import_session(&file_path, target_path) {
                    Ok(_session_file) => {
                        println!("✓ Session imported!");
                        println!("\nOpen Claude Code and run /resume to continue the session.");
                    }
                    Err(e) => {
                        eprintln!("✗ Import failed: {}", e);
                        std::process::exit(1);
                    }
                }
                Ok(())
            }
            "preview" => {
                if args.len() < 3 {
                    eprintln!("Usage: mcc preview <file.json.gz>");
                    std::process::exit(1);
                }
                let file_path = PathBuf::from(&args[2]);

                match import::preview_session(&file_path) {
                    Ok(session) => {
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
                    }
                    Err(e) => {
                        eprintln!("✗ Preview failed: {}", e);
                        std::process::exit(1);
                    }
                }
                Ok(())
            }
            "config" => {
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
            "share" => {
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
                        Ok(gcs_path) => {
                            println!("✓ Session uploaded!");
                            println!("  GCS path: {}", gcs_path);
                            println!("\nShare with your team:");
                            println!("  mcc fetch {}", gcs_path);
                        }
                        Err(e) => {
                            eprintln!("✗ Upload failed: {}", e);
                            std::process::exit(1);
                        }
                    }
                    Ok(())
                }
                #[cfg(not(feature = "gcs"))]
                {
                    eprintln!("✗ GCS support not enabled");
                    eprintln!("Rebuild with: cargo build --release --features gcs");
                    std::process::exit(1);
                }
            }
            "fetch" => {
                #[cfg(feature = "gcs")]
                {
                    if args.len() < 3 {
                        eprintln!("Usage: mcc fetch <gs://bucket/file.json.gz> [target-path]");
                        std::process::exit(1);
                    }
                    let gcs_path = &args[2];
                    let target_path = args.get(3).map(|s| s.to_string()).or_else(|| {
                        // Default to current directory
                        std::env::current_dir()
                            .ok()
                            .and_then(|p| p.to_str().map(|s| s.to_string()))
                    });

                    // Download to temp file
                    let home = std::env::var("HOME")?;
                    let temp_file = PathBuf::from(home)
                        .join(".mcc/temp")
                        .join("downloaded-session.json.gz");
                    std::fs::create_dir_all(temp_file.parent().unwrap())?;

                    let runtime = tokio::runtime::Runtime::new()?;
                    if let Err(e) = runtime.block_on(cloud::download_session(gcs_path, &temp_file)) {
                        eprintln!("✗ Download failed: {}", e);
                        std::process::exit(1);
                    }

                    // Import the downloaded session
                    match import::import_session(&temp_file, target_path) {
                        Ok(session_file) => {
                            println!("✓ Session fetched and imported!");
                            println!("  File: {}", session_file.display());
                            println!("\nYou can now open Claude Code and use /resume to load this session.");
                        }
                        Err(e) => {
                            eprintln!("✗ Import failed: {}", e);
                            std::process::exit(1);
                        }
                    }
                    Ok(())
                }
                #[cfg(not(feature = "gcs"))]
                {
                    eprintln!("✗ GCS support not enabled");
                    eprintln!("Rebuild with: cargo build --release --features gcs");
                    std::process::exit(1);
                }
            }
            "help" | "-h" | "--help" => {
                show_help();
                Ok(())
            }
            _ => {
                eprintln!("Unknown command: {}", args[1]);
                eprintln!("Run 'mcc help' for usage information.");
                std::process::exit(1);
            }
        }
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
