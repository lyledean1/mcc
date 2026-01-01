# MCC - Multi-Claude Code

Share Claude Code sessions between developers for collaborative debugging and pair programming.

## What is MCC?

MCC (Multi-Claude Code) is a session sharing tool for [Claude Code](https://claude.ai/code). It allows you to export your entire Claude Code conversation — including all messages, tool calls, file edits, and context — into a portable file that colleagues can import and continue from exactly where you left off.

Think of it as "multiplayer mode" for Claude Code debugging sessions.

## Use Case

**Scenario:** You're debugging a production issue and need to hand off to a teammate.

**Without MCC:**
- Copy/paste conversation snippets
- Explain what you tried
- They start from scratch

**With MCC:**
- Export your session in seconds
- Share one file via Slack
- They resume with full context: your conversation, file changes, debugging steps, everything

## Installation

```bash
cd mcc
cargo build --release
cargo install --path .
```

Or run directly:
```bash
cargo run --release
```

## Quick Start

### Export a Session

```bash
# Launch the TUI
mcc

# Navigate with j/k or arrow keys
# Press 'e' on a session to export
# File saved to ~/.mcc/exports/
```

### Import a Session

```bash
# Import to a specific project directory
mcc import session.json.gz /path/to/your/project

# Or import to current directory
cd /path/to/your/project
mcc import session.json.gz
```

### Preview Before Import

```bash
mcc preview session.json.gz
```

## How It Works

MCC reads Claude Code's session files from `~/.claude/projects/` and packages them into compressed `.json.gz` files containing:

- **Full conversation history** - Every message and response
- **Tool call results** - File reads, edits, bash commands
- **File snapshots** - State of modified files
- **Git context** - Branch, commit info
- **Working directory** - Where the session was running
- **Session metadata** - When exported, by whom

When imported, MCC:
1. Decompresses the session
2. Rewrites file paths to match your local environment
3. Creates a new session file in `~/.claude/projects/`
4. Registers it in `~/.claude.json`
5. Makes it available via `/resume` in Claude Code

## Usage

### TUI Mode (default)

```bash
mcc
```

**Controls:**
- `j`/`k` or `↓`/`↑` - Navigate sessions
- `e` - Export selected session
- `r` - Reload sessions list
- `q` - Quit

### CLI Commands

```bash
# Import a session
mcc import <file.json.gz> [target-directory]

# Preview session details
mcc preview <file.json.gz>

# Show help
mcc help
```

## Workflow Example

### Developer A (Exporting)

```bash
# Working on a bug in /Users/alice/myapp
cd /Users/alice/myapp
# ... uses Claude Code to debug ...

# Export the session
mcc
# Press 'e' on the current session
# -> Exported to: ~/.mcc/exports/20260101-143022-debugging-auth-bug.json.gz

# Share via Slack, email, etc.
```

### Developer B (Importing)

```bash
# Receives session.json.gz file
cd /Users/bob/myapp

# Import the session
mcc import ~/Downloads/20260101-143022-debugging-auth-bug.json.gz

# Open Claude Code
claude

# Resume the session
/resume
# -> Select the imported session
# -> Continue debugging from where Alice left off!
```

## File Format

Exported files are gzipped JSON with this structure:

```json
{
  "version": "1.0.0",
  "exported_at": "2026-01-01T14:30:22Z",
  "exported_by": "alice@macbook.local",
  "session": {
    "id": "3ec1525f-8ab7-498f-a99c-b24231359f36",
    "project_path": "/Users/alice/myapp",
    "summary": "debugging authentication bug...",
    "git_branch": "hotfix/auth",
    "messages": [...]
  }
}
```

View the contents:
```bash
gunzip -c session.json.gz | jq '.'
```

## Configuration

Exports are saved to `~/.mcc/exports/` by default.

Sessions are read from `~/.claude/projects/` (Claude Code's default location).

## Tips

- **Export often** - Sessions are snapshots, export at key debugging milestones
- **Add context** - The first user message becomes the session summary, make it descriptive
- **Check paths** - Imported sessions work best when directory structures match
- **Git branches** - Session includes branch info to help sync environments

## Troubleshooting

**Session doesn't appear in `/resume`:**
- Make sure you imported to the correct directory
- Restart Claude Code to refresh session list
- Check that `~/.claude.json` lists the session ID

**Paths don't match:**
- MCC rewrites `cwd` fields automatically
- File paths in messages reference original locations
- Works best when project structure is similar

**Exported file too large:**
- Long sessions create large exports
- Consider exporting smaller debugging segments
- Compressed files are typically 10-50KB for normal sessions

## Architecture

```
mcc/
├── src/
│   ├── main.rs      # TUI and CLI entry point
│   ├── session.rs   # Session discovery and parsing
│   ├── export.rs    # Export to .json.gz
│   └── import.rs    # Import and path rewriting
└── README.md
```

## License

MIT

## Contributing

This is a personal tool, but issues and PRs welcome!

## Acknowledgments

Built for [Claude Code](https://claude.ai/code) - Anthropic's official CLI for Claude.
