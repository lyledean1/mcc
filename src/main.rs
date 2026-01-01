mod export;
mod import;
mod session;

use anyhow::Result;
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
            "import" => {
                if args.len() < 3 {
                    eprintln!("Usage: mcc import <file.json.gz> [target-project-path]");
                    std::process::exit(1);
                }
                let file_path = PathBuf::from(&args[2]);
                let target_path = args.get(3).map(|s| s.to_string());

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
            "help" | "-h" | "--help" => {
                println!("MCC - Multi-Claude Code");
                println!("\nUsage:");
                println!("  mcc                                    Launch TUI");
                println!("  mcc import <file.json.gz> [path]      Import a session");
                println!("  mcc preview <file.json.gz>            Preview session details");
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
