# MCC - Multi-Claude Code

**It's 1am. Production is down. You've been debugging with Claude for 2 hours and found something. Your teammate has a different theory.**

Instead of copy-pasting chat history or explaining what you tried, hand off your entire Claude Code session. They see every message, every file change, every debugging step. They pick up exactly where you left off.

```bash
# You at 1am
cd /my/project
mcc export
# Creates: ./mcc-export.json.gz

# Send mcc-export.json.gz via Slack to teammate

# Teammate at 1:02am (drops file in their project folder)
cd /my/project
mcc import
claude
/resume
# Sees your full 2-hour debugging session, continues from there
```

## Install

```bash
cargo install --path .
```

For cloud storage support: `cargo install --path . --features gcs`

## Usage

```bash
# Export current session (creates ./mcc-export.json.gz)
mcc export

# Import a session (reads ./mcc-export.json.gz)
mcc import
```

## What Gets Shared

- Full conversation history
- All file changes and tool calls
- Git branch and working directory context

## Cloud Sharing (Advanced)

Want to skip the file transfer? Set up GCS for automatic sharing:

```bash
mcc config set-bucket gs://my-team-sessions
mcc share ~/.mcc/exports/my-fix.json.gz
# Teammate runs: mcc fetch gs://my-team-sessions/my-fix.json.gz
```

See [GCS_SETUP.md](GCS_SETUP.md) for setup. **But start with local files first - it's simpler.**

## Files

- Export creates: `./mcc-export.json.gz` (in current directory)
- Sessions stored in: `~/.claude/projects/`

## Tips

- Export at debugging milestones
- Project structure should match between teammates
- Session includes git branch context

## License

MIT
