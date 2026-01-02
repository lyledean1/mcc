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
                return Ok(());
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
                    Ok(session_file) => {
                        println!("✓ Session imported!");
                        println!("\nOpen Claude Code and run /resume to continue the session.");
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
                println!("  mcc                       Browse all sessions (TUI)");
                println!("  mcc preview <file>        Preview session details");
                println!("\nOther:");
                println!("  mcc help                  Show this help");
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
