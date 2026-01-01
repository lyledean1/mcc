mod cloud;
mod export;
mod import;
mod session;

use anyhow::{Context, Result};
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame, Terminal,
};
use std::io;

use session::{find_all_sessions, Session};
use std::path::PathBuf;

struct App {
    sessions: Vec<Session>,
    selected: usize,
    message: Option<String>,
}

impl App {
    fn new() -> Result<Self> {
        let sessions = find_all_sessions()?;
        Ok(App {
            sessions,
            selected: 0,
            message: None,
        })
    }

    fn reload_sessions(&mut self) -> Result<()> {
        self.sessions = find_all_sessions()?;
        if self.selected >= self.sessions.len() && self.sessions.len() > 0 {
            self.selected = self.sessions.len() - 1;
        }
        Ok(())
    }

    fn select_next(&mut self) {
        if !self.sessions.is_empty() {
            self.selected = (self.selected + 1) % self.sessions.len();
        }
    }

    fn select_prev(&mut self) {
        if !self.sessions.is_empty() {
            self.selected = if self.selected == 0 {
                self.sessions.len() - 1
            } else {
                self.selected - 1
            };
        }
    }

    fn export_selected(&mut self) -> Result<()> {
        if let Some(session) = self.sessions.get(self.selected) {
            let output_path = export::export_session(session, None)?;
            self.message = Some(format!(
                "Exported to: {}",
                output_path.display()
            ));
        }
        Ok(())
    }
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();

    // Handle CLI commands
    if args.len() > 1 {
        match args[1].as_str() {
            "export" => {
                // mcc export [name]
                let custom_name = args.get(2).map(|s| s.as_str());

                // Find the session for the current directory
                let current_dir = std::env::current_dir()?;
                let current_path = current_dir.to_str().context("Invalid current directory path")?;

                let sessions = find_all_sessions()?;
                let current_session = sessions.iter()
                    .filter(|s| s.project_path == current_path)
                    .max_by_key(|s| s.last_modified);

                match current_session {
                    Some(session) => {
                        let home = std::env::var("HOME")?;
                        let export_dir = PathBuf::from(home).join(".mcc/exports");
                        std::fs::create_dir_all(&export_dir)?;

                        // Generate filename with custom name or summary
                        let timestamp = chrono::Utc::now().format("%Y%m%d-%H%M%S");
                        let name = custom_name.unwrap_or(&session.summary);
                        let safe_name = name
                            .chars()
                            .filter(|c| c.is_alphanumeric() || *c == ' ' || *c == '-')
                            .take(30)
                            .collect::<String>()
                            .replace(' ', "-")
                            .to_lowercase();

                        let filename = if let Some(_) = custom_name {
                            format!("{}.json.gz", safe_name)
                        } else {
                            format!("{}-{}.json.gz", timestamp, safe_name)
                        };

                        let output_path = export_dir.join(&filename);

                        // Export
                        let exported = export::ExportedSession::from_session(session)?;
                        exported.export_to_file(&output_path)?;

                        println!("✓ Session exported!");
                        println!("  Name: {}", filename.trim_end_matches(".json.gz"));
                        println!("  File: {}", output_path.display());
                        println!("\nShare with your team:");
                        println!("  mcc import {}", filename.trim_end_matches(".json.gz"));
                        #[cfg(feature = "gcs")]
                        {
                            let config = cloud::CloudConfig::load()?;
                            if config.enabled {
                                println!("  mcc share {}", output_path.display());
                            }
                        }
                    }
                    None => {
                        eprintln!("✗ No Claude Code session found for current directory");
                        eprintln!("  Current: {}", current_path);
                        eprintln!("\nMake sure you've used Claude Code in this directory first.");
                        std::process::exit(1);
                    }
                }
                return Ok(());
            }
            "import" => {
                if args.len() < 3 {
                    eprintln!("Usage: mcc import <name-or-file> [target-project-path]");
                    std::process::exit(1);
                }

                // Check if it's a name or a full path
                let input = &args[2];
                let file_path = if input.contains('/') || input.ends_with(".json.gz") {
                    // It's a path
                    PathBuf::from(input)
                } else {
                    // It's a name - look in ~/.mcc/exports
                    let home = std::env::var("HOME")?;
                    let exports_dir = PathBuf::from(home).join(".mcc/exports");

                    // Try with .json.gz extension
                    let with_ext = format!("{}.json.gz", input);
                    let candidate = exports_dir.join(&with_ext);

                    if candidate.exists() {
                        candidate
                    } else {
                        // Maybe they included the extension
                        let candidate = exports_dir.join(input);
                        if candidate.exists() {
                            candidate
                        } else {
                            eprintln!("✗ Session not found: {}", input);
                            eprintln!("  Looked in: {}", exports_dir.display());
                            eprintln!("\nAvailable sessions:");
                            if let Ok(entries) = std::fs::read_dir(&exports_dir) {
                                for entry in entries.flatten() {
                                    if let Some(name) = entry.file_name().to_str() {
                                        if name.ends_with(".json.gz") {
                                            println!("  - {}", name.trim_end_matches(".json.gz"));
                                        }
                                    }
                                }
                            }
                            std::process::exit(1);
                        }
                    }
                };

                let target_path = args.get(3).map(|s| s.to_string()).or_else(|| {
                    // Default to current directory
                    std::env::current_dir()
                        .ok()
                        .and_then(|p| p.to_str().map(|s| s.to_string()))
                });

                match import::import_session(&file_path, target_path) {
                    Ok(session_file) => {
                        println!("✓ Session imported successfully!");
                        println!("  File: {}", session_file.display());
                        println!("\nYou can now open Claude Code and use /resume to load this session.");
                    }
                    Err(e) => {
                        eprintln!("✗ Import failed: {}", e);
                        std::process::exit(1);
                    }
                }
                return Ok(());
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
                return Ok(());
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
                return Ok(());
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
                }
                #[cfg(not(feature = "gcs"))]
                {
                    eprintln!("✗ GCS support not enabled");
                    eprintln!("Rebuild with: cargo build --release --features gcs");
                    std::process::exit(1);
                }
                return Ok(());
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
                }
                #[cfg(not(feature = "gcs"))]
                {
                    eprintln!("✗ GCS support not enabled");
                    eprintln!("Rebuild with: cargo build --release --features gcs");
                    std::process::exit(1);
                }
                return Ok(());
            }
            "help" | "-h" | "--help" => {
                println!("MCC - Multi-Claude Code");
                println!("\nQuick Start:");
                println!("  mcc export [name]                      Export current directory's session");
                println!("  mcc import <name> [path]               Import a session (defaults to current dir)");
                println!("\nAdvanced:");
                println!("  mcc                                    Launch TUI browser");
                println!("  mcc preview <file.json.gz>             Preview session details");
                println!("\nCloud Storage (requires --features gcs):");
                println!("  mcc config set-bucket <gs://bucket>    Configure GCS bucket");
                println!("  mcc share <file.json.gz>               Upload to GCS");
                println!("  mcc fetch <gs://bucket/file> [path]    Download and import from GCS");
                println!("\nExamples:");
                println!("  cd /my/project");
                println!("  mcc export auth-bug-fix                Export with custom name");
                println!("  mcc import auth-bug-fix                Import to current directory");
                println!("\nOther:");
                println!("  mcc help                               Show this help");
                return Ok(());
            }
            _ => {
                eprintln!("Unknown command: {}", args[1]);
                eprintln!("Run 'mcc help' for usage information.");
                std::process::exit(1);
            }
        }
    }

    // No args - launch TUI
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app
    let mut app = App::new()?;

    // Run the app
    let result = run_app(&mut terminal, &mut app);

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    if let Err(e) = result {
        println!("Error: {:?}", e);
    }

    Ok(())
}

fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> Result<()> {
    loop {
        terminal.draw(|f| ui(f, app))?;

        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                match key.code {
                    KeyCode::Char('q') => return Ok(()),
                    KeyCode::Char('j') | KeyCode::Down => app.select_next(),
                    KeyCode::Char('k') | KeyCode::Up => app.select_prev(),
                    KeyCode::Char('e') => {
                        if let Err(e) = app.export_selected() {
                            app.message = Some(format!("Export failed: {}", e));
                        }
                    }
                    KeyCode::Char('r') => {
                        if let Err(e) = app.reload_sessions() {
                            app.message = Some(format!("Reload failed: {}", e));
                        } else {
                            app.message = Some("Sessions reloaded".to_string());
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}

fn ui(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(f.area());

    // Title
    let title = Paragraph::new("MCC - Multi-Claude Code")
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(title, chunks[0]);

    // Sessions list
    let items: Vec<ListItem> = app
        .sessions
        .iter()
        .enumerate()
        .map(|(i, session)| {
            let project_name = session
                .project_path
                .split('/')
                .last()
                .unwrap_or(&session.project_path);

            let content = vec![
                Line::from(vec![
                    Span::styled(
                        format!("{} ", project_name),
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        format!("({})", session.time_ago()),
                        Style::default().fg(Color::Gray),
                    ),
                ]),
                Line::from(vec![
                    Span::raw("  "),
                    Span::styled(&session.summary, Style::default().fg(Color::White)),
                ]),
                Line::from(vec![
                    Span::raw("  "),
                    Span::styled(
                        format!(
                            "{} messages",
                            session.message_count(),
                        ),
                        Style::default().fg(Color::DarkGray),
                    ),
                    Span::raw(" • "),
                    Span::styled(
                        session
                            .git_branch
                            .as_deref()
                            .unwrap_or("no branch"),
                        Style::default().fg(Color::Green),
                    ),
                ]),
            ];

            let style = if i == app.selected {
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            ListItem::new(content).style(style)
        })
        .collect();

    let sessions_list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!("Sessions ({})", app.sessions.len())),
    );
    f.render_widget(sessions_list, chunks[1]);

    // Footer
    let footer_text = if let Some(msg) = &app.message {
        msg.clone()
    } else {
        "[e]xport [i]mport [r]eload [q]uit".to_string()
    };

    let footer = Paragraph::new(footer_text)
        .style(Style::default().fg(Color::Cyan))
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(footer, chunks[2]);
}
