# MCC - Multiplayer Claude Code

**Share Claude Code sessions with your team.**

It's 1am. Production is down. You've been debugging with Claude for 2 hours and found something crucial. Your teammate has a different theory and wants to help.

Instead of copy-pasting chat history or explaining what you tried, hand off your entire Claude Code session. They see every message, every file change, every debugging step. They pick up exactly where you left off.

## How It Works

```bash
# You at 1am
cd /my/project
mcc export
# ✓ Session exported to ./mcc-export.json.gz

# Send mcc-export.json.gz to teammate via Slack/email

# Teammate at 1:02am (saves file in their project folder)
cd /my/project
mcc import
# ✓ Session imported!

claude
/resume
# They now see your full 2-hour debugging session and continue from there
```

**That's it.** Export creates a file. Import loads it. No setup, no config, no cloud accounts.

## Install

### From Pre-built Binaries (Recommended)

Download the latest release for your platform from [GitHub Releases](https://github.com/lyledean1/mcc/releases):

```bash
# Linux (x86_64)
curl -L https://github.com/lyledean1/mcc/releases/latest/download/mcc-linux-amd64.tar.gz | tar xz
sudo mv mcc /usr/local/bin/

# Linux (ARM64)
curl -L https://github.com/lyledean1/mcc/releases/latest/download/mcc-linux-arm64.tar.gz | tar xz
sudo mv mcc /usr/local/bin/

# macOS (Intel)
curl -L https://github.com/lyledean1/mcc/releases/latest/download/mcc-macos-amd64.tar.gz | tar xz
sudo mv mcc /usr/local/bin/

# macOS (Apple Silicon)
curl -L https://github.com/lyledean1/mcc/releases/latest/download/mcc-macos-arm64.tar.gz | tar xz
sudo mv mcc /usr/local/bin/
```

### From Source

```bash
cargo install --path .
```

## Commands

```bash
mcc export    # Export current session → ./mcc-export.json.gz
mcc import    # Import session from ./mcc-export.json.gz
mcc preview <file>  # Preview session details without importing
mcc help      # Show help
```

## What Gets Shared

Your exported session includes:

- **Full conversation history** - Every message, question, and response
- **All file changes** - Every edit Claude made
- **Complete context** - Git branch, working directory, tool calls
- **Session metadata** - Who exported, when, from which machine

The session file is compressed and typically small (a few hundred KB for most sessions).

## How to Share

### 1. Via Slack/Chat
```bash
mcc export
# Attach ./mcc-export.json.gz to Slack message
```

### 2. Via Email
```bash
mcc export
# Attach ./mcc-export.json.gz to email
```

### 3. Via Shared Drive
```bash
mcc export
cp mcc-export.json.gz /path/to/shared/drive/
```

Your teammate just needs to save the file in their project directory and run `mcc import`.

## Preview Before Importing

Want to see what's in a session before importing?

```bash
mcc preview mcc-export.json.gz
# Session Preview:
#   Version: 1.0.0
#   Exported by: alice@laptop
#   Exported at: 2026-01-02T12:34:56Z
#   Project: /Users/alice/projects/myapp
#   Summary: Fix production database timeout issue
#   Messages: 47
#   Git branch: hotfix/db-timeout
```

## Requirements

- Both you and your teammate need Claude Code installed
- Both should have the project cloned locally
- Project paths can differ (MCC handles this automatically)

## Tips

- **Export at milestones** - After fixing a bug, before switching tasks
- **Name your exports** - Rename `mcc-export.json.gz` to `db-fix.json.gz` before sharing
- **Project structure** - Should generally match between teammates (same repo)
- **Git branches** - Session includes branch info, but you can resume on any branch

## Technical Details

- **Export location**: `./mcc-export.json.gz` (current directory)
- **Session storage**: `~/.claude/projects/` (Claude Code's session directory)
- **Format**: Compressed JSON (gzip)
- **Session rewriting**: Import automatically rewrites paths to match teammate's environment

## License

MIT
